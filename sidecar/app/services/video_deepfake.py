"""
Jura Trace Sidecar — Video deepfake analysis.

Analyses video files for AI-generated or manipulated frames by running
the image deepfake detector on evenly-spaced frames extracted via FFmpeg.
Per-frame scores are aggregated into a video-level verdict.

Near-duplicate frames are detected via mean absolute pixel difference and
skipped to avoid redundant analysis on static or near-static video segments.
"""

import base64
import io
import logging

import numpy as np
from PIL import Image

from app.models.schemas import (
    FrameDeepfakeResult,
    VideoDeepfakeResponse,
)
from app.services.deepfake import perform_deepfake_detection_with_features
from app.services.video_frames import perform_frame_extraction

logger = logging.getLogger(__name__)

# Frame counts by analysis mode
FRAME_COUNTS = {"standard": 6, "deep": 20, "archival": 40}

# Default similarity threshold for near-duplicate frame skipping.
# Two frames with normalised MAD similarity >= this value are considered
# near-duplicates and the second is skipped.
DEFAULT_SIMILARITY_THRESHOLD = 0.95


def _decode_frame_to_array(frame_b64: str) -> np.ndarray:
    """Decode a base64 JPEG frame to a numpy array (RGB, uint8)."""
    frame_bytes = base64.b64decode(frame_b64)
    img = Image.open(io.BytesIO(frame_bytes)).convert("RGB")
    return np.asarray(img, dtype=np.uint8)


def _frame_similarity(arr_a: np.ndarray, arr_b: np.ndarray) -> float:
    """
    Compute normalised similarity between two frames.

    Uses mean absolute difference (MAD) of pixel values, normalised to [0, 1].
    Returns 1.0 for identical frames and 0.0 for maximally different frames.

    Both arrays are resized to the same small resolution (64x64) before
    comparison so the metric is fast regardless of input resolution.
    """
    # Resize to 64x64 for speed
    target_size = (64, 64)
    a_small = np.asarray(Image.fromarray(arr_a).resize(target_size), dtype=np.float32)
    b_small = np.asarray(Image.fromarray(arr_b).resize(target_size), dtype=np.float32)

    mad = np.mean(np.abs(a_small - b_small))
    # Normalise: max possible MAD is 255.0
    return 1.0 - float(mad / 255.0)


def deduplicate_frames(
    frames_b64: list[str],
    similarity_threshold: float = DEFAULT_SIMILARITY_THRESHOLD,
) -> tuple[list[str], list[int], int]:
    """
    Remove near-duplicate consecutive frames from a list of base64 JPEG frames.

    Args:
        frames_b64: List of base64-encoded JPEG frame strings.
        similarity_threshold: Frames with similarity >= this value are
            considered near-duplicates.  Range [0.0, 1.0].

    Returns:
        Tuple of (kept_frames_b64, kept_original_indices, skipped_count).
    """
    if not frames_b64:
        return [], [], 0

    kept: list[str] = [frames_b64[0]]
    kept_indices: list[int] = [0]
    prev_arr = _decode_frame_to_array(frames_b64[0])
    skipped = 0

    for i in range(1, len(frames_b64)):
        curr_arr = _decode_frame_to_array(frames_b64[i])
        sim = _frame_similarity(prev_arr, curr_arr)

        if sim >= similarity_threshold:
            skipped += 1
            logger.debug(
                "Skipping frame %d (similarity %.4f >= %.4f threshold)",
                i, sim, similarity_threshold,
            )
        else:
            kept.append(frames_b64[i])
            kept_indices.append(i)
            prev_arr = curr_arr

    return kept, kept_indices, skipped


def perform_video_deepfake_analysis(
    video_bytes: bytes,
    mode: str = "standard",
    similarity_threshold: float = DEFAULT_SIMILARITY_THRESHOLD,
) -> VideoDeepfakeResponse:
    """
    Analyse video for AI-generated or manipulated frames.

    Extracts N evenly-spaced frames from the video, removes near-duplicate
    consecutive frames (adaptive sampling), runs each remaining frame through
    the image deepfake detector, and aggregates per-frame scores into a
    video-level verdict.

    Args:
        video_bytes: Raw bytes of the input video file.
        mode: Analysis mode — "standard" (6 frames), "deep" (20), "archival" (40).
        similarity_threshold: Frames with normalised MAD similarity >= this
            value are considered near-duplicates and skipped.  Range [0.0, 1.0].

    Returns:
        VideoDeepfakeResponse with per-frame and aggregate results.
    """
    count = FRAME_COUNTS.get(mode, 6)

    # Step 1: Extract frames
    frames_response = perform_frame_extraction(video_bytes, count=min(count, 12))
    # video_frames.py caps at 12 — for deep/archival we re-extract with higher count
    if count > 12:
        frames_response = _extract_many_frames(video_bytes, count)

    if not frames_response.success or not frames_response.frames:
        return VideoDeepfakeResponse(
            frame_results=[],
            aggregate_score=0.0,
            aggregate_verdict="inconclusive",
            aggregate_confidence="low",
            frames_analysed=0,
            frames_requested=count,
            frames_skipped=0,
            temporal_available=False,
            mode=mode,
            duration=frames_response.duration,
            success=False,
            message=frames_response.message or "Frame extraction failed",
        )

    # Step 1b: Adaptive frame sampling — skip near-duplicate frames
    raw_frames = frames_response.frames
    kept_frames, kept_indices, frames_skipped = deduplicate_frames(
        raw_frames, similarity_threshold=similarity_threshold,
    )
    if frames_skipped > 0:
        logger.info(
            "Adaptive sampling: skipped %d/%d near-duplicate frames (threshold=%.2f)",
            frames_skipped, len(raw_frames), similarity_threshold,
        )

    # Step 2: Run deepfake detection on each kept frame
    frame_results: list[FrameDeepfakeResult] = []
    feature_dicts: list[dict[str, float]] = []
    duration = frames_response.duration or 0.0
    total_extracted = len(raw_frames)

    for kept_idx, frame_b64 in zip(kept_indices, kept_frames):
        try:
            frame_bytes = base64.b64decode(frame_b64)
            response, features = perform_deepfake_detection_with_features(
                frame_bytes, mime_type="image/jpeg",
            )
            # Compute approximate timestamp using original frame index
            timestamp = (
                duration * (kept_idx + 1) / (total_extracted + 1)
                if duration > 0 else 0.0
            )

            frame_results.append(FrameDeepfakeResult(
                frame_index=kept_idx,
                timestamp=round(timestamp, 2),
                score=response.score,
                suspicious=response.suspicious,
                verdict_level=response.verdict_level,
                signals=response.signals,
                classifier_score=response.classifier_score,
                classifier_available=response.classifier_available,
                heatmap_base64="",  # Omit per-frame heatmaps to reduce payload
            ))
            feature_dicts.append(features)
        except Exception as exc:
            logger.warning("Frame %d deepfake analysis failed: %s", kept_idx, exc)
            # Include a failed frame result with neutral score
            frame_results.append(FrameDeepfakeResult(
                frame_index=kept_idx,
                timestamp=0.0,
                score=0.5,
                suspicious=False,
                verdict_level="inconclusive",
                signals=[],
            ))

    if not frame_results:
        return VideoDeepfakeResponse(
            frame_results=[],
            aggregate_score=0.0,
            aggregate_verdict="inconclusive",
            aggregate_confidence="low",
            frames_analysed=0,
            frames_requested=count,
            frames_skipped=frames_skipped,
            temporal_available=False,
            mode=mode,
            duration=duration,
            success=False,
            message="No frames could be analysed",
        )

    # Step 3: Compute temporal consistency (requires >= 3 frames with features)
    temporal_available = len(feature_dicts) >= 3
    temporal_noise_drift = None
    temporal_spectral_drift = None
    temporal_lbp_drift = None
    temporal_score = 0.0

    if temporal_available:
        temporal_noise_drift = _compute_drift(feature_dicts, "noise_std")
        temporal_spectral_drift = _compute_drift(feature_dicts, "spectral_decay_beta")
        temporal_lbp_drift = _compute_drift(feature_dicts, "glcm_contrast_mean")

        # Temporal inconsistency score: mean of normalised drifts
        drifts = [
            d for d in [temporal_noise_drift, temporal_spectral_drift, temporal_lbp_drift]
            if d is not None
        ]
        temporal_score = float(np.mean(drifts)) if drifts else 0.0
        # Clamp to [0, 1]
        temporal_score = max(0.0, min(1.0, temporal_score))

    # Step 4: Aggregate video score
    scores = [fr.score for fr in frame_results]
    mean_score = float(np.mean(scores))
    max_score = float(np.max(scores))

    video_score = 0.5 * mean_score + 0.3 * max_score + 0.2 * temporal_score
    video_score = max(0.0, min(1.0, round(video_score, 4)))

    # Step 5: Verdict
    if video_score >= 0.60:
        verdict = "synthetic"
    elif video_score >= 0.35:
        verdict = "inconclusive"
    else:
        verdict = "authentic"

    # Confidence based on score extremity and frame count
    if video_score > 0.70 or video_score < 0.20:
        confidence = "high"
    elif video_score > 0.55 or video_score < 0.30:
        confidence = "medium"
    else:
        confidence = "low"

    # Summary
    triggered_frames = sum(1 for fr in frame_results if fr.suspicious)
    summary = (
        f"Analysed {len(frame_results)} frames in {mode} mode. "
        f"{triggered_frames} frame{'s' if triggered_frames != 1 else ''} flagged as suspicious."
    )
    if frames_skipped > 0:
        summary += f" {frames_skipped} near-duplicate frame{'s' if frames_skipped != 1 else ''} skipped."
    if temporal_available and temporal_score > 0.4:
        summary += " Frame-to-frame drift detected in forensic features."

    return VideoDeepfakeResponse(
        frame_results=frame_results,
        aggregate_score=video_score,
        aggregate_verdict=verdict,
        aggregate_confidence=confidence,
        frames_analysed=len(frame_results),
        frames_requested=count,
        frames_skipped=frames_skipped,
        temporal_available=temporal_available,
        temporal_noise_drift=round(temporal_noise_drift, 4) if temporal_noise_drift is not None else None,
        temporal_spectral_drift=round(temporal_spectral_drift, 4) if temporal_spectral_drift is not None else None,
        temporal_lbp_drift=round(temporal_lbp_drift, 4) if temporal_lbp_drift is not None else None,
        mode=mode,
        duration=duration,
        success=True,
        message=summary,
    )


def _compute_drift(
    feature_dicts: list[dict[str, float]],
    key: str,
) -> float | None:
    """
    Compute the coefficient of variation for a feature across frames.

    Returns a value in [0, 1] where higher means more inconsistency.
    Returns None if the feature is missing from all frames.
    """
    values = []
    for fd in feature_dicts:
        v = fd.get(key)
        if v is not None and not (isinstance(v, float) and (np.isnan(v) or np.isinf(v))):
            values.append(v)

    if len(values) < 2:
        return None

    arr = np.array(values, dtype=np.float64)
    mean_val = np.mean(arr)
    std_val = np.std(arr)

    if abs(mean_val) < 1e-10:
        # Mean near zero — use raw std as drift measure, clamped
        return float(min(1.0, std_val))

    # Coefficient of variation, clamped to [0, 1]
    cv = float(std_val / abs(mean_val))
    return min(1.0, cv)


def _extract_many_frames(video_bytes: bytes, count: int):
    """
    Extract more than 12 frames by calling perform_frame_extraction in batches.

    For deep/archival modes that need >12 frames, we call the extraction
    with count=12 (the max) and accept fewer frames. The frame extraction
    logic already samples evenly — for higher counts we accept what FFmpeg
    can provide within the 12-frame limit and note the shortfall.
    """
    # The video_frames module caps at 12 — for higher counts, we modify
    # the call to allow more frames by calling the internal logic directly.
    import os
    import tempfile

    tmp_path: str | None = None
    try:
        with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as f:
            f.write(video_bytes)
            tmp_path = f.name

        from app.services.video_frames import _get_duration, _extract_frame_at
        from app.models.schemas import VideoFramesResponse

        duration = _get_duration(tmp_path)
        if duration is None or duration <= 0:
            return VideoFramesResponse(
                frames=[], count=0, duration=None,
                success=False, message="Could not determine video duration",
            )

        timestamps = [duration * i / (count + 1) for i in range(1, count + 1)]
        frames: list[str] = []
        for ts in timestamps:
            frame_b64 = _extract_frame_at(tmp_path, ts)
            if frame_b64 is not None:
                frames.append(frame_b64)

        return VideoFramesResponse(
            frames=frames,
            count=len(frames),
            duration=duration,
            success=True,
            message=f"Extracted {len(frames)} frames from {duration:.1f}s video",
        )
    except FileNotFoundError:
        from app.models.schemas import VideoFramesResponse
        return VideoFramesResponse(
            frames=[], count=0, duration=None,
            success=False, message="FFmpeg is not installed. Install from ffmpeg.org.",
        )
    except Exception as e:
        from app.models.schemas import VideoFramesResponse
        return VideoFramesResponse(
            frames=[], count=0, duration=None,
            success=False, message=f"Frame extraction failed: {e}",
        )
    finally:
        if tmp_path is not None:
            try:
                os.unlink(tmp_path)
            except FileNotFoundError:
                pass
