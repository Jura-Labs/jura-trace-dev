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
        """When onnxruntime is not importable, should return model_available=False.

        JTV-143 (3 May 2026): switched the import name from open_clip to
        onnxruntime since CLIP is now loaded via ONNX runtime.
        """
        import app.services.clip_detector as mod

        original_session = mod._vision_session
        original_attempted = mod._model_load_attempted

        try:
            mod._vision_session = None
            mod._model_load_attempted = False

            with patch.dict(sys.modules, {"onnxruntime": None}):
                mod._model_load_attempted = False
                result = mod.perform_clip_detection(_make_noisy_photo())

            assert isinstance(result, ClipDetectionResponse)
            assert result.model_available is False
            assert result.score == 0.0
            assert result.verdict_level == "inconclusive"
            assert "not available" in result.summary.lower()
        finally:
            mod._vision_session = original_session
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

    def test_verdict_thresholds_field_present(self):
        """Every ClipDetectionResponse populates verdict_thresholds (JTV-97)."""
        from app.services.clip_detector import perform_clip_detection

        result = perform_clip_detection(_make_noisy_photo())
        assert result.verdict_thresholds is not None
        assert result.verdict_thresholds.synthetic_min > result.verdict_thresholds.authentic_max
        assert result.verdict_thresholds.model_version.startswith("univfd-probe-")
        # Threshold basis must reference the AUC figure for the trained probe.
        assert "AUC" in result.verdict_thresholds.threshold_basis

    def test_verdict_thresholds_match_module_constants(self):
        """Wire values must equal the module source-of-truth constants."""
        from app.services.clip_detector import (
            _AUTHENTIC_THRESHOLD,
            _MODEL_VERSION,
            _SYNTHETIC_THRESHOLD,
            perform_clip_detection,
        )

        result = perform_clip_detection(_make_noisy_photo())
        assert result.verdict_thresholds.synthetic_min == _SYNTHETIC_THRESHOLD
        assert result.verdict_thresholds.authentic_max == _AUTHENTIC_THRESHOLD
        assert result.verdict_thresholds.model_version == _MODEL_VERSION

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


class TestClipModelEviction:
    """Tests for the CLIP model eviction (unload + status) API."""

    def test_unload_returns_correct_schema(self):
        """unload_clip_model() should return the expected dict keys."""
        from app.services.clip_detector import unload_clip_model

        result = unload_clip_model()
        assert "unloaded" in result
        assert "previously_loaded" in result
        assert result["unloaded"] is True

    def test_unload_when_not_loaded_reports_not_previously_loaded(self):
        """Unloading a never-loaded session should report previously_loaded=False.

        JTV-143: globals renamed `_model` → `_vision_session` after the ONNX
        backend swap.
        """
        import app.services.clip_detector as mod
        from app.services.clip_detector import unload_clip_model

        original_session = mod._vision_session
        original_attempted = mod._model_load_attempted
        mod._vision_session = None
        mod._model_load_attempted = False
        try:
            result = unload_clip_model()
            assert result["previously_loaded"] is False
            assert result["unloaded"] is True
        finally:
            mod._vision_session = original_session
            mod._model_load_attempted = original_attempted

    def test_unload_clears_globals(self):
        """After unload_clip_model(), session globals should all be None."""
        import app.services.clip_detector as mod
        from app.services.clip_detector import unload_clip_model

        original_vision = mod._vision_session
        original_text = mod._text_session
        original_cache = mod._text_prompt_cache
        original_attempted = mod._model_load_attempted
        try:
            unload_clip_model()
            assert mod._vision_session is None
            assert mod._text_session is None
            assert mod._text_prompt_cache is None
            assert mod._model_load_attempted is False
        finally:
            mod._vision_session = original_vision
            mod._text_session = original_text
            mod._text_prompt_cache = original_cache
            mod._model_load_attempted = original_attempted

    def test_get_last_used_ts_initially_zero(self):
        """get_last_used_ts() should return 0.0 before any detection has run."""
        import app.services.clip_detector as mod
        from app.services.clip_detector import get_last_used_ts

        original_ts = mod._last_used_ts
        mod._last_used_ts = 0.0
        try:
            assert get_last_used_ts() == 0.0
        finally:
            mod._last_used_ts = original_ts

    def test_get_last_used_ts_reflects_set_value(self):
        """get_last_used_ts() should return the value set by the caller."""
        import time

        import app.services.clip_detector as mod
        from app.services.clip_detector import get_last_used_ts

        original_ts = mod._last_used_ts
        sentinel = time.time() - 42.0
        mod._last_used_ts = sentinel
        try:
            assert get_last_used_ts() == pytest.approx(sentinel)
        finally:
            mod._last_used_ts = original_ts

    @pytest.fixture
    def client(self):
        from fastapi import FastAPI
        from fastapi.testclient import TestClient

        from app.api.forensics import router

        app = FastAPI()
        app.include_router(router, prefix="/forensics")
        return TestClient(app)

    def test_unload_endpoint_returns_200(self, client):
        """POST /forensics/unload-clip should return HTTP 200."""
        response = client.post("/forensics/unload-clip")
        assert response.status_code == 200
        data = response.json()
        assert data["unloaded"] is True
        assert "previously_loaded" in data

    def test_clip_status_endpoint_returns_200(self, client):
        """GET /forensics/clip-status should return HTTP 200 with expected keys."""
        response = client.get("/forensics/clip-status")
        assert response.status_code == 200
        data = response.json()
        assert "loaded" in data
        assert "last_used_ts" in data
        assert "idle_seconds" in data
        assert isinstance(data["loaded"], bool)

    def test_clip_status_loaded_false_after_unload(self, client):
        """After unload, clip-status should report loaded=False."""
        client.post("/forensics/unload-clip")
        response = client.get("/forensics/clip-status")
        assert response.status_code == 200
        data = response.json()
        assert data["loaded"] is False
