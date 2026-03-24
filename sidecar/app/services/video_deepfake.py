"""
Jura Trace Sidecar — Video deepfake analysis.

Analyses video files for AI-generated or manipulated frames by running
the image deepfake detector on evenly-spaced frames extracted via FFmpeg.
Per-frame scores are aggregated into a video-level verdict.
"""

import base64
import logging

import numpy as np

from app.models.schemas import (
    DeepfakeSignal,
    FrameDeepfakeResult,
    VideoDeepfakeResponse,
)
from app.services.deepfake import perform_deepfake_detection_with_features
from app.services.video_frames import perform_frame_extraction

logger = logging.getLogger(__name__)

# Frame counts by analysis mode
FRAME_COUNTS = {"standard": 6, "deep": 20, "archival": 40}

# Default similarity threshold for frame deduplication (SSIM-like metric)
DEDUP_SIMILARITY_THRESHOLD = 0.95


def deduplicate_frames(
    frames_b64: list[str],
    threshold: float = DEDUP_SIMILARITY_THRESHOLD,
    buffer_size: int = 5,
) -> list[str]:
    """
    Remove near-duplicate frames using a rolling buffer.

    Compares each candidate frame against the last `buffer_size` kept frames.
    A frame is skipped if its similarity to ANY frame in the buffer exceeds
    the threshold.

    Args:
        frames_b64: List of base64-encoded JPEG frame strings.
        threshold: Similarity threshold in [0, 1]. Frames above this are skipped.
        buffer_size: Number of recent kept frames to compare against.

    Returns:
        Filtered list of base64-encoded frame strings with duplicates removed.
    """
    if not frames_b64 or buffer_size < 1:
        return list(frames_b64)

    from PIL import Image
    import io

    def _decode_frame(b64_str: str) -> np.ndarray:
        """Decode a base64 JPEG string to a grayscale numpy array."""
        raw = base64.b64decode(b64_str)
        img = Image.open(io.BytesIO(raw)).convert("L")
        # Resize to a small standard size for fast comparison
        img = img.resize((64, 64), Image.BILINEAR)
        return np.asarray(img, dtype=np.float64)

    def _similarity(a: np.ndarray, b: np.ndarray) -> float:
        """Compute normalised correlation similarity in [0, 1]."""
        a_flat = a.ravel()
        b_flat = b.ravel()
        a_mean = a_flat - a_flat.mean()
        b_mean = b_flat - b_flat.mean()
        norm_a = np.linalg.norm(a_mean)
        norm_b = np.linalg.norm(b_mean)
        if norm_a < 1e-10 or norm_b < 1e-10:
            # Constant images — treat as identical if both constant
            return 1.0 if norm_a < 1e-10 and norm_b < 1e-10 else 0.0
        return float(np.dot(a_mean, b_mean) / (norm_a * norm_b))

    kept_b64: list[str] = []
    # Rolling buffer of decoded arrays for the last `buffer_size` kept frames
    buffer: list[np.ndarray] = []

    for frame_b64 in frames_b64:
        try:
            candidate = _decode_frame(frame_b64)
        except Exception:
            # If we can't decode, keep the frame to avoid data loss
            kept_b64.append(frame_b64)
            continue

        # Check against all frames in the rolling buffer
        is_duplicate = False
        for buf_frame in buffer:
            if _similarity(candidate, buf_frame) > threshold:
                is_duplicate = True
                break

        if not is_duplicate:
            kept_b64.append(frame_b64)
            buffer.append(candidate)
            # Maintain rolling buffer size
            if len(buffer) > buffer_size:
                buffer.pop(0)

    logger.debug(
        "Frame deduplication: %d -> %d frames (buffer_size=%d, threshold=%.2f)",
        len(frames_b64), len(kept_b64), buffer_size, threshold,
    )
    return kept_b64


def perform_video_deepfake_analysis(
    video_bytes: bytes,
    mode: str = "standard",
) -> VideoDeepfakeResponse:
    """
    Analyse video for AI-generated or manipulated frames.

    Extracts N evenly-spaced frames from the video, runs each through the
    image deepfake detector, and aggregates per-frame scores into a
    video-level verdict.

    Args:
        video_bytes: Raw bytes of the input video file.
        mode: Analysis mode — "standard" (6 frames), "deep" (20), "archival" (40).

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
            temporal_available=False,
            mode=mode,
            duration=frames_response.duration,
            success=False,
            message=frames_response.message or "Frame extraction failed",
        )

    # Step 1b: Deduplicate frames (rolling buffer catches cyclical repeats)
    raw_frames = frames_response.frames
    deduped_frames = deduplicate_frames(raw_frames)
    if len(deduped_frames) < len(raw_frames):
        logger.info(
            "Deduplicated %d -> %d frames",
            len(raw_frames), len(deduped_frames),
        )

    # Step 2: Run deepfake detection on each frame
    frame_results: list[FrameDeepfakeResult] = []
    feature_dicts: list[dict[str, float]] = []
    duration = frames_response.duration or 0.0
    num_frames = len(deduped_frames)

    for i, frame_b64 in enumerate(deduped_frames):
        try:
            frame_bytes = base64.b64decode(frame_b64)
            response, features = perform_deepfake_detection_with_features(
                frame_bytes, mime_type="image/jpeg",
            )
            # Compute approximate timestamp
            timestamp = duration * (i + 1) / (num_frames + 1) if duration > 0 else 0.0

            frame_results.append(FrameDeepfakeResult(
                frame_index=i,
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
            logger.warning("Frame %d deepfake analysis failed: %s", i, exc)
            # Include a failed frame result with neutral score
            frame_results.append(FrameDeepfakeResult(
                frame_index=i,
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
    if temporal_available and temporal_score > 0.4:
        summary += " Frame-to-frame drift detected in forensic features."

    return VideoDeepfakeResponse(
        frame_results=frame_results,
        aggregate_score=video_score,
        aggregate_verdict=verdict,
        aggregate_confidence=confidence,
        frames_analysed=len(frame_results),
        frames_requested=count,
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
    import json
    import os
    import subprocess
    import tempfile

    with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as f:
        f.write(video_bytes)
        tmp_path = f.name

    try:
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
        os.unlink(tmp_path)
