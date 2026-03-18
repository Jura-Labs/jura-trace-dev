"""Tests for the CLIP-based AI image detection service."""

import io
import sys
from unittest.mock import patch

import numpy as np
import pytest
from PIL import Image

from app.models.schemas import ClipDetectionResponse


# ── Test image helpers ────────────────────────────────────────────────


def _make_solid_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a solid-colour image."""
    img = Image.new("RGB", size, (128, 128, 128))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_noisy_photo(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create an image with random noise simulating a real photograph."""
    rng = np.random.default_rng(42)
    arr = rng.integers(50, 200, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_gradient_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a gradient image."""
    w, h = size
    arr = np.zeros((h, w, 3), dtype=np.uint8)
    for i in range(h):
        arr[i, :, 0] = int(i * 255 / h)
        arr[i, :, 1] = int((h - i) * 255 / h)
        arr[i, :, 2] = 128
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


# ── Graceful degradation tests (always run) ──────────────────────────


class TestClipDetectorGracefulDegradation:
    """Tests that run regardless of whether open_clip is installed."""

    def test_unavailable_response_when_import_fails(self):
        """When open_clip is not importable, should return model_available=False."""
        # Reset module-level state so _ensure_model retries
        import app.services.clip_detector as mod

        original_model = mod._model
        original_attempted = mod._model_load_attempted

        try:
            mod._model = None
            mod._model_load_attempted = False

            with patch.dict(sys.modules, {"open_clip": None}):
                # Force re-evaluation of import
                mod._model_load_attempted = False
                result = mod.perform_clip_detection(_make_noisy_photo())

            assert isinstance(result, ClipDetectionResponse)
            assert result.model_available is False
            assert result.score == 0.0
            assert result.verdict_level == "inconclusive"
            assert "not available" in result.summary.lower()
        finally:
            mod._model = original_model
            mod._model_load_attempted = original_attempted

    def test_unavailable_response_schema_valid(self):
        """The unavailable response should be a valid ClipDetectionResponse."""
        from app.services.clip_detector import _unavailable_response

        result = _unavailable_response()
        assert isinstance(result, ClipDetectionResponse)
        assert result.model_available is False
        assert result.score == 0.0
        assert result.confidence == "low"
        assert isinstance(result.class_probabilities, dict)
        assert isinstance(result.model_name, str)
        assert isinstance(result.summary, str)

    def test_invalid_image_raises_when_model_available(self):
        """Invalid image data should raise ValueError if model is available."""
        import app.services.clip_detector as mod

        if mod._ensure_model():
            with pytest.raises(ValueError, match="Cannot decode image"):
                mod.perform_clip_detection(b"not an image")
        else:
            # Model not available — should return unavailable response
            result = mod.perform_clip_detection(b"not an image")
            assert result.model_available is False

    def test_response_fields_present(self):
        """All expected fields should be present in the response."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        assert hasattr(result, "score")
        assert hasattr(result, "suspicious")
        assert hasattr(result, "verdict_level")
        assert hasattr(result, "confidence")
        assert hasattr(result, "class_probabilities")
        assert hasattr(result, "model_name")
        assert hasattr(result, "model_available")
        assert hasattr(result, "summary")


# ── Tests that require open_clip (skipped if not installed) ──────────


class TestClipDetectorWithModel:
    """Tests that require the CLIP model to be loaded."""

    @pytest.fixture(autouse=True)
    def _require_open_clip(self):
        """Skip all tests in this class if open_clip is not available."""
        pytest.importorskip("open_clip")
        pytest.importorskip("torch")
        from app.services.clip_detector import _ensure_model

        if not _ensure_model():
            pytest.skip("CLIP model could not be loaded")

    def test_solid_image_produces_result(self):
        """A solid-colour image should return a valid result with model_available=True."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_solid_image())
        assert result.model_available is True
        assert 0.0 <= result.score <= 1.0
        assert result.verdict_level in ("authentic", "inconclusive", "synthetic")

    def test_noisy_image_produces_result(self):
        """A noisy image should return a valid result."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        assert result.model_available is True
        assert 0.0 <= result.score <= 1.0

    def test_gradient_image_produces_result(self):
        """A gradient image should return a valid result."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_gradient_image())
        assert result.model_available is True
        assert 0.0 <= result.score <= 1.0

    def test_score_bounded(self):
        """Score should always be between 0.0 and 1.0."""
        from app.services.clip_detector import perform_clip_detection

        for img_fn in (_make_solid_image, _make_noisy_photo, _make_gradient_image):
            result = perform_clip_detection(img_fn())
            assert 0.0 <= result.score <= 1.0, f"Score {result.score} out of bounds"

    def test_class_probabilities_present(self):
        """Class probabilities should contain all expected keys."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        expected_keys = {"photograph", "real_scene", "ai_generated", "synthetic", "manipulated"}
        assert set(result.class_probabilities.keys()) == expected_keys

    def test_class_probabilities_sum_near_one(self):
        """Class probabilities should sum to approximately 1.0."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        total = sum(result.class_probabilities.values())
        assert abs(total - 1.0) < 0.01, f"Probabilities sum to {total}, expected ~1.0"

    def test_confidence_valid(self):
        """Confidence should be one of the expected values."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        assert result.confidence in ("low", "medium", "high")

    def test_verdict_level_valid(self):
        """verdict_level should be one of the three allowed values."""
        from app.services.clip_detector import perform_clip_detection

        for img_fn in (_make_solid_image, _make_noisy_photo, _make_gradient_image):
            result = perform_clip_detection(img_fn())
            assert result.verdict_level in ("authentic", "inconclusive", "synthetic"), (
                f"Unexpected verdict_level: {result.verdict_level} (score={result.score})"
            )

    def test_verdict_consistent_with_score(self):
        """verdict_level boundaries should match score thresholds."""
        from app.services.clip_detector import perform_clip_detection

        for img_fn in (_make_solid_image, _make_noisy_photo, _make_gradient_image):
            result = perform_clip_detection(img_fn())
            if result.score > 0.60:
                assert result.verdict_level == "synthetic"
            elif result.score < 0.35:
                assert result.verdict_level == "authentic"
            else:
                assert result.verdict_level == "inconclusive"

    def test_suspicious_consistent_with_score(self):
        """suspicious flag should be True when score > 0.5."""
        from app.services.clip_detector import perform_clip_detection

        for img_fn in (_make_solid_image, _make_noisy_photo, _make_gradient_image):
            result = perform_clip_detection(img_fn())
            assert result.suspicious == (result.score > 0.5)

    def test_summary_not_empty(self):
        """Summary should always be a non-empty string."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        assert isinstance(result.summary, str)
        assert len(result.summary) > 0

    def test_model_name_set(self):
        """model_name should identify the CLIP model used."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        assert "ViT-B-32" in result.model_name

    def test_jpeg_input_works(self):
        """JPEG input should be handled correctly."""
        from app.services.clip_detector import perform_clip_detection

        img = Image.new("RGB", (200, 200), (100, 150, 200))
        buf = io.BytesIO()
        img.save(buf, format="JPEG", quality=85)
        result = perform_clip_detection(buf.getvalue())
        assert result.model_available is True
        assert 0.0 <= result.score <= 1.0

    def test_small_image_handled(self):
        """A small image should be handled gracefully."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_solid_image(size=(32, 32)))
        assert result.model_available is True
        assert 0.0 <= result.score <= 1.0
