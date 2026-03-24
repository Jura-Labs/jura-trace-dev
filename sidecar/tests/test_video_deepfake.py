"""Tests for the video deepfake analysis service."""

import io
from unittest.mock import patch, MagicMock

import numpy as np
import pytest
from PIL import Image

from app.models.schemas import (
    DeepfakeSignal,
    FrameDeepfakeResult,
    VideoDeepfakeResponse,
    VideoFramesResponse,
)
from app.services.video_deepfake import perform_video_deepfake_analysis


def _make_test_image() -> bytes:
    """Create a small test image as JPEG bytes."""
    rng = np.random.default_rng(42)
    arr = rng.integers(50, 200, (64, 64, 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG")
    return buf.getvalue()


def _make_test_image_with_seed(seed: int) -> bytes:
    """Create a test image with a specific random seed for reproducible content."""
    rng = np.random.default_rng(seed)
    arr = rng.integers(50, 200, (64, 64, 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG")
    return buf.getvalue()


def _make_b64_frame(seed: int) -> str:
    """Create a base64-encoded test frame with a specific seed."""
    import base64
    return base64.b64encode(_make_test_image_with_seed(seed)).decode("utf-8")


# Pre-compute distinct frames for reuse (avoids JPEG non-determinism issues)
_PRECOMPUTED_FRAMES: dict[int, str] = {}


def _get_b64_frame(seed: int) -> str:
    """Get a cached base64-encoded test frame for a given seed."""
    if seed not in _PRECOMPUTED_FRAMES:
        _PRECOMPUTED_FRAMES[seed] = _make_b64_frame(seed)
    return _PRECOMPUTED_FRAMES[seed]


def _make_mock_frames_response(count: int = 6, duration: float = 10.0) -> VideoFramesResponse:
    """Create a mock VideoFramesResponse with distinct base64 test frames."""
    import base64
    # Use different seeds so frames are distinct and survive deduplication
    frames = [base64.b64encode(_make_test_image_with_seed(seed)).decode("utf-8")
              for seed in range(count)]
    return VideoFramesResponse(
        frames=frames,
        count=count,
        duration=duration,
        success=True,
        message=f"Extracted {count} frames from {duration}s video",
    )


def _make_mock_frames_failure(msg: str = "FFmpeg not installed") -> VideoFramesResponse:
    """Create a failed VideoFramesResponse."""
    return VideoFramesResponse(
        frames=[],
        count=0,
        duration=None,
        success=False,
        message=msg,
    )


class TestVideoDeepfakeAnalysis:
    """Tests for perform_video_deepfake_analysis."""

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_returns_valid_response(self, mock_extract):
        """Service returns a valid VideoDeepfakeResponse for standard mode."""
        mock_extract.return_value = _make_mock_frames_response(count=6)
        result = perform_video_deepfake_analysis(b"fake-video-data", mode="standard")

        assert isinstance(result, VideoDeepfakeResponse)
        assert result.success is True
        assert result.frames_analysed == 6
        assert result.frames_requested == 6
        assert result.mode == "standard"
        assert len(result.frame_results) == 6
        assert result.aggregate_verdict in ("authentic", "inconclusive", "synthetic")
        assert result.aggregate_confidence in ("low", "medium", "high")

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_handles_missing_ffmpeg(self, mock_extract):
        """Service returns success=False gracefully when FFmpeg is absent."""
        mock_extract.return_value = _make_mock_frames_failure(
            "FFmpeg is not installed. Install from ffmpeg.org."
        )
        result = perform_video_deepfake_analysis(b"fake-video-data", mode="standard")

        assert result.success is False
        assert result.frames_analysed == 0
        assert "FFmpeg" in result.message

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_handles_empty_data(self, mock_extract):
        """Service handles empty frame extraction result."""
        mock_extract.return_value = _make_mock_frames_failure("Frame extraction failed")
        result = perform_video_deepfake_analysis(b"", mode="standard")

        assert result.success is False
        assert result.frames_analysed == 0
        assert len(result.frame_results) == 0

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_score_bounded(self, mock_extract):
        """Aggregate score must be in [0.0, 1.0]."""
        mock_extract.return_value = _make_mock_frames_response(count=6)
        result = perform_video_deepfake_analysis(b"fake-video-data", mode="standard")

        assert 0.0 <= result.aggregate_score <= 1.0
        for fr in result.frame_results:
            assert 0.0 <= fr.score <= 1.0

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_frame_scores_present(self, mock_extract):
        """Each frame has a score and verdict."""
        mock_extract.return_value = _make_mock_frames_response(count=4)
        result = perform_video_deepfake_analysis(b"fake-video-data", mode="standard")

        assert len(result.frame_results) == 4
        for fr in result.frame_results:
            assert isinstance(fr.score, float)
            assert fr.verdict_level in ("authentic", "inconclusive", "synthetic")
            assert isinstance(fr.frame_index, int)

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_temporal_signals_absent_with_few_frames(self, mock_extract):
        """Temporal signals are skipped when fewer than 3 frames."""
        mock_extract.return_value = _make_mock_frames_response(count=2)
        result = perform_video_deepfake_analysis(b"fake-video-data", mode="standard")

        # With only 2 frames, temporal signals should not be available
        assert result.temporal_available is False
        assert result.temporal_noise_drift is None

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_temporal_signals_present_with_enough_frames(self, mock_extract):
        """Temporal signals are computed when >= 3 frames are available."""
        mock_extract.return_value = _make_mock_frames_response(count=6)
        result = perform_video_deepfake_analysis(b"fake-video-data", mode="standard")

        assert result.temporal_available is True
        # Drift values should be computed (all frames identical so drift should be low)
        if result.temporal_noise_drift is not None:
            assert 0.0 <= result.temporal_noise_drift <= 1.0

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_verdict_assignment(self, mock_extract):
        """Verdict is assigned based on aggregate score thresholds."""
        mock_extract.return_value = _make_mock_frames_response(count=6)
        result = perform_video_deepfake_analysis(b"fake-video-data", mode="standard")

        if result.aggregate_score >= 0.60:
            assert result.aggregate_verdict == "synthetic"
        elif result.aggregate_score >= 0.35:
            assert result.aggregate_verdict == "inconclusive"
        else:
            assert result.aggregate_verdict == "authentic"


class TestVideoDeepfakeSchemas:
    """Tests for schema imports and validation."""

    def test_frame_deepfake_result_schema(self):
        """FrameDeepfakeResult is importable and validates."""
        fr = FrameDeepfakeResult(
            frame_index=0,
            timestamp=1.5,
            score=0.42,
            suspicious=False,
            verdict_level="inconclusive",
            signals=[],
        )
        assert fr.frame_index == 0
        assert fr.score == 0.42

    def test_video_deepfake_response_schema(self):
        """VideoDeepfakeResponse is importable and validates."""
        resp = VideoDeepfakeResponse(
            frame_results=[],
            aggregate_score=0.35,
            aggregate_verdict="authentic",
            aggregate_confidence="high",
            frames_analysed=6,
            frames_requested=6,
            temporal_available=False,
            mode="standard",
            success=True,
            message="Analysis complete",
        )
        assert resp.success is True
        assert resp.aggregate_score == 0.35


class TestDeepfakeWithFeatures:
    """Tests for the refactored perform_deepfake_detection_with_features."""

    def test_returns_features_dict(self):
        """perform_deepfake_detection_with_features returns response + features."""
        from app.services.deepfake import perform_deepfake_detection_with_features

        image_bytes = _make_test_image()
        response, features = perform_deepfake_detection_with_features(image_bytes)

        assert hasattr(response, "score")
        assert isinstance(features, dict)
        assert "noise_std" in features
        assert "spectral_decay_beta" in features
        assert "glcm_contrast_mean" in features

    def test_features_are_numeric(self):
        """Feature dict values are numeric."""
        from app.services.deepfake import perform_deepfake_detection_with_features

        image_bytes = _make_test_image()
        _response, features = perform_deepfake_detection_with_features(image_bytes)

        for key in ("noise_std", "spectral_decay_beta", "glcm_contrast_mean"):
            val = features[key]
            assert isinstance(val, (int, float)), f"{key} should be numeric, got {type(val)}"


class TestDeduplicateFrames:
    """Tests for the deduplicate_frames rolling buffer implementation."""

    def test_import(self):
        """deduplicate_frames is importable."""
        from app.services.video_deepfake import deduplicate_frames
        assert callable(deduplicate_frames)

    def test_empty_list(self):
        """Empty input returns empty output."""
        from app.services.video_deepfake import deduplicate_frames
        assert deduplicate_frames([]) == []

    def test_single_frame(self):
        """Single frame is always kept."""
        from app.services.video_deepfake import deduplicate_frames
        frames = [_make_b64_frame(1)]
        result = deduplicate_frames(frames)
        assert len(result) == 1

    def test_all_unique_frames_kept(self):
        """Distinct frames are all kept."""
        from app.services.video_deepfake import deduplicate_frames
        frames = [_make_b64_frame(seed) for seed in range(6)]
        result = deduplicate_frames(frames)
        assert len(result) == 6

    def test_identical_frames_deduplicated(self):
        """Identical frames are reduced to one."""
        from app.services.video_deepfake import deduplicate_frames
        frame = _make_b64_frame(42)
        frames = [frame] * 8
        result = deduplicate_frames(frames)
        assert len(result) == 1

    def test_adjacent_duplicates_removed(self):
        """Adjacent duplicate pairs are collapsed."""
        from app.services.video_deepfake import deduplicate_frames
        a = _make_b64_frame(1)
        b = _make_b64_frame(2)
        c = _make_b64_frame(3)
        frames = [a, a, b, b, c, c]
        result = deduplicate_frames(frames)
        assert len(result) == 3

    def test_cyclical_frames_deduplicated_with_rolling_buffer(self):
        """Cyclical video: frames 5-8 repeat frames 1-4 and are caught by rolling buffer.

        With a single-prev comparison, frames 5-8 would NOT be caught because
        they differ from the immediately preceding kept frame. The rolling
        buffer (size >= 4) catches them.
        """
        from app.services.video_deepfake import deduplicate_frames
        # Create 4 distinct frames (cached so repeats are identical)
        f1 = _get_b64_frame(10)
        f2 = _get_b64_frame(20)
        f3 = _get_b64_frame(30)
        f4 = _get_b64_frame(40)
        # Cyclical: frames repeat
        frames = [f1, f2, f3, f4, f1, f2, f3, f4]
        result = deduplicate_frames(frames, buffer_size=5)
        # Only the first 4 unique frames should be kept
        assert len(result) == 4

    def test_buffer_size_limits_lookback(self):
        """With buffer_size=2, only the 2 most recent kept frames are checked.

        A frame identical to one kept 3+ positions ago will NOT be caught.
        """
        from app.services.video_deepfake import deduplicate_frames
        # Create 4 distinct frames + repeat of frame 1
        f1 = _get_b64_frame(10)
        f2 = _get_b64_frame(20)
        f3 = _get_b64_frame(30)
        f4 = _get_b64_frame(40)
        # f1 repeated at position 4 — with buffer_size=2, buffer holds [f3, f4]
        # so f1 is NOT in the buffer and will be kept again
        frames = [f1, f2, f3, f4, f1]
        result = deduplicate_frames(frames, buffer_size=2)
        # f1 at position 4 is NOT caught because buffer only has f3, f4
        assert len(result) == 5

    def test_buffer_size_catches_recent_duplicates(self):
        """With buffer_size=2, recent duplicates are still caught."""
        from app.services.video_deepfake import deduplicate_frames
        f1 = _get_b64_frame(10)
        f2 = _get_b64_frame(20)
        # f2 repeated immediately after — buffer has [f1, f2], so f2 is caught
        frames = [f1, f2, f2]
        result = deduplicate_frames(frames, buffer_size=2)
        assert len(result) == 2

    def test_default_buffer_size(self):
        """Default buffer_size=5 catches duplicates within 5-frame window."""
        from app.services.video_deepfake import deduplicate_frames
        # 5 distinct frames, then repeat frame 1 — buffer holds all 5
        frames = [_get_b64_frame(i) for i in range(5)]
        frames.append(_get_b64_frame(0))  # repeat of first
        result = deduplicate_frames(frames)
        assert len(result) == 5

    def test_threshold_controls_sensitivity(self):
        """Lower threshold deduplicates more aggressively."""
        from app.services.video_deepfake import deduplicate_frames
        frames = [_make_b64_frame(seed) for seed in range(4)]
        # With threshold=1.0, nothing should be considered similar (must be > 1.0)
        result_high = deduplicate_frames(frames, threshold=1.0)
        assert len(result_high) == 4
        # With threshold=-1.0, everything is "similar" except the first
        result_low = deduplicate_frames(frames, threshold=-1.0)
        assert len(result_low) == 1

    def test_performance_40_frames(self):
        """Deduplication of 40 frames with buffer_size=5 completes quickly."""
        import time
        from app.services.video_deepfake import deduplicate_frames
        # Use 3 unique seeds cycling over 40 frames — buffer_size=5 catches all repeats
        frames = [_get_b64_frame(i % 3) for i in range(40)]
        start = time.monotonic()
        result = deduplicate_frames(frames, buffer_size=5)
        elapsed_ms = (time.monotonic() - start) * 1000
        # Must complete in < 50ms
        assert elapsed_ms < 50, f"Deduplication took {elapsed_ms:.1f}ms, expected < 50ms"
        # Only 3 unique frames should survive (3 seeds, buffer holds all 3)
        assert len(result) == 3
