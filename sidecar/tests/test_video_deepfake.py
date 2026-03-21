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


def _make_mock_frames_response(count: int = 6, duration: float = 10.0) -> VideoFramesResponse:
    """Create a mock VideoFramesResponse with base64 test frames."""
    import base64
    frame_b64 = base64.b64encode(_make_test_image()).decode("utf-8")
    return VideoFramesResponse(
        frames=[frame_b64] * count,
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
