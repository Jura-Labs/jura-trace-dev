"""Tests for the video deepfake analysis service."""

import base64
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
from app.services.video_deepfake import (
    deduplicate_frames,
    perform_video_deepfake_analysis,
    _frame_similarity,
)


def _make_test_image() -> bytes:
    """Create a small test image as JPEG bytes."""
    rng = np.random.default_rng(42)
    arr = rng.integers(50, 200, (64, 64, 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG")
    return buf.getvalue()


def _make_distinct_test_image(seed: int = 42) -> bytes:
    """Create a small test image as JPEG bytes with a specific random seed."""
    rng = np.random.default_rng(seed)
    arr = rng.integers(0, 255, (64, 64, 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG")
    return buf.getvalue()


def _make_mock_frames_response(count: int = 6, duration: float = 10.0) -> VideoFramesResponse:
    """Create a mock VideoFramesResponse with distinct base64 test frames."""
    frames = []
    for i in range(count):
        frame_bytes = _make_distinct_test_image(seed=1000 + i)
        frames.append(base64.b64encode(frame_bytes).decode("utf-8"))
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


def _make_test_image_b64(seed: int = 42, low: int = 50, high: int = 200) -> str:
    """Create a small test image as a base64 JPEG string."""
    rng = np.random.default_rng(seed)
    arr = rng.integers(low, high, (64, 64, 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG")
    return base64.b64encode(buf.getvalue()).decode("utf-8")


class TestFrameDeduplication:
    """Tests for the adaptive frame sampling / deduplication logic."""

    def test_identical_frames_are_skipped(self):
        """All-identical frames should keep only the first."""
        frame = _make_test_image_b64(seed=42)
        frames = [frame] * 6

        kept, indices, skipped = deduplicate_frames(frames)

        assert skipped == 5
        assert len(kept) == 1
        assert indices == [0]

    def test_distinct_frames_are_all_kept(self):
        """Visually distinct frames should not be skipped."""
        # Use very different pixel ranges to ensure distinct content
        frames = [
            _make_test_image_b64(seed=1, low=0, high=50),
            _make_test_image_b64(seed=2, low=100, high=200),
            _make_test_image_b64(seed=3, low=200, high=255),
            _make_test_image_b64(seed=4, low=0, high=80),
        ]

        kept, indices, skipped = deduplicate_frames(frames)

        assert skipped == 0
        assert len(kept) == 4
        assert indices == [0, 1, 2, 3]

    def test_empty_input(self):
        """Empty frame list returns empty results."""
        kept, indices, skipped = deduplicate_frames([])

        assert kept == []
        assert indices == []
        assert skipped == 0

    def test_single_frame(self):
        """Single frame is always kept."""
        frames = [_make_test_image_b64(seed=10)]

        kept, indices, skipped = deduplicate_frames(frames)

        assert len(kept) == 1
        assert skipped == 0

    def test_custom_threshold(self):
        """A threshold of 1.0 means only exact pixel matches are skipped."""
        frame = _make_test_image_b64(seed=42)
        frames = [frame] * 4

        # With threshold 1.0, JPEG re-encoding makes frames not *exactly*
        # identical at pixel level after decode, so some may pass.
        # With threshold 0.5 (very loose), even somewhat different frames
        # would be considered duplicates.
        kept_strict, _, skipped_strict = deduplicate_frames(frames, similarity_threshold=1.0)
        kept_loose, _, skipped_loose = deduplicate_frames(frames, similarity_threshold=0.5)

        # Loose threshold should skip at least as many as strict
        assert skipped_loose >= skipped_strict

    def test_frame_similarity_identical(self):
        """Identical arrays should have similarity 1.0."""
        rng = np.random.default_rng(99)
        arr = rng.integers(0, 255, (64, 64, 3), dtype=np.uint8)

        sim = _frame_similarity(arr, arr)
        assert sim == 1.0

    def test_frame_similarity_opposite(self):
        """Black vs white frames should have very low similarity."""
        black = np.zeros((64, 64, 3), dtype=np.uint8)
        white = np.full((64, 64, 3), 255, dtype=np.uint8)

        sim = _frame_similarity(black, white)
        assert sim < 0.05  # Nearly 0

    def test_frames_skipped_in_response(self):
        """VideoDeepfakeResponse includes frames_skipped count."""
        resp = VideoDeepfakeResponse(
            frame_results=[],
            aggregate_score=0.0,
            aggregate_verdict="authentic",
            aggregate_confidence="high",
            frames_analysed=4,
            frames_requested=6,
            frames_skipped=2,
            temporal_available=False,
            mode="standard",
            success=True,
            message="Test",
        )
        assert resp.frames_skipped == 2

    def test_frames_skipped_default_zero(self):
        """frames_skipped defaults to 0 when not provided."""
        resp = VideoDeepfakeResponse(
            frame_results=[],
            aggregate_score=0.0,
            aggregate_verdict="authentic",
            aggregate_confidence="high",
            frames_analysed=6,
            frames_requested=6,
            temporal_available=False,
            mode="standard",
            success=True,
            message="Test",
        )
        assert resp.frames_skipped == 0

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_integration_identical_frames_skipped(self, mock_extract):
        """End-to-end: identical frames produce frames_skipped > 0."""
        frame_b64 = _make_test_image_b64(seed=42)
        mock_extract.return_value = VideoFramesResponse(
            frames=[frame_b64] * 6,
            count=6,
            duration=10.0,
            success=True,
            message="Extracted 6 frames",
        )

        result = perform_video_deepfake_analysis(b"fake-video", mode="standard")

        assert result.success is True
        assert result.frames_skipped == 5
        assert result.frames_analysed == 1
        assert result.frames_requested == 6
        assert "5 near-duplicate frames skipped" in result.message

    @patch("app.services.video_deepfake.perform_frame_extraction")
    def test_integration_distinct_frames_none_skipped(self, mock_extract):
        """End-to-end: distinct frames produce frames_skipped == 0."""
        frames = [
            _make_test_image_b64(seed=1, low=0, high=50),
            _make_test_image_b64(seed=2, low=100, high=200),
            _make_test_image_b64(seed=3, low=200, high=255),
        ]
        mock_extract.return_value = VideoFramesResponse(
            frames=frames,
            count=3,
            duration=10.0,
            success=True,
            message="Extracted 3 frames",
        )

        result = perform_video_deepfake_analysis(b"fake-video", mode="standard")

        assert result.success is True
        assert result.frames_skipped == 0
        assert result.frames_analysed == 3
