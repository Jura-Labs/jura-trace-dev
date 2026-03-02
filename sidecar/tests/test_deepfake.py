"""Tests for the deepfake / AI-generated image detection service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.deepfake import perform_deepfake_detection


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
    """Create a gradient image with natural-looking variation."""
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


class TestDeepfakeDetection:
    def test_solid_image_produces_result(self):
        """A solid-colour image should return a valid result."""
        result = perform_deepfake_detection(_make_solid_image())
        assert 0.0 <= result.score <= 1.0
        assert isinstance(result.suspicious, bool)
        assert len(result.signals) > 0
        assert len(result.heatmap_base64) > 0
        assert result.summary != ""

    def test_noisy_image_produces_result(self):
        """A noisy image should return a valid result."""
        result = perform_deepfake_detection(_make_noisy_photo())
        assert 0.0 <= result.score <= 1.0
        assert len(result.signals) == 8  # 8 signals in the ensemble

    def test_gradient_image_valid(self):
        """A gradient image should return a valid result."""
        result = perform_deepfake_detection(_make_gradient_image())
        assert 0.0 <= result.score <= 1.0

    def test_small_image_handled(self):
        """A small image should be handled gracefully."""
        result = perform_deepfake_detection(_make_solid_image(size=(32, 32)))
        assert 0.0 <= result.score <= 1.0
        assert isinstance(result.signals, list)

    def test_invalid_data_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_deepfake_detection(b"not an image")

    def test_score_bounded(self):
        """Score should always be between 0.0 and 1.0."""
        for img_fn in (_make_solid_image, _make_noisy_photo, _make_gradient_image):
            result = perform_deepfake_detection(img_fn())
            assert 0.0 <= result.score <= 1.0

    def test_signals_populated(self):
        """All 8 ensemble signals should be present."""
        result = perform_deepfake_detection(_make_noisy_photo())
        assert len(result.signals) == 8
        names = {s.name for s in result.signals}
        expected_names = {
            "noise_residual",
            "noise_prnu",
            "noise_consistency",
            "frequency_energy",
            "spectral_decay",
            "texture_consistency",
            "color_gamut",
            "sharpness_consistency",
        }
        assert names == expected_names

    def test_confidence_valid(self):
        """Confidence should be one of the expected values."""
        result = perform_deepfake_detection(_make_noisy_photo())
        assert result.confidence in ("low", "medium", "high")

    def test_signal_fields_complete(self):
        """Each signal should have all required fields."""
        result = perform_deepfake_detection(_make_noisy_photo())
        for signal in result.signals:
            assert isinstance(signal.name, str) and signal.name
            assert isinstance(signal.description, str) and signal.description
            assert isinstance(signal.weight, float) and signal.weight > 0
            assert isinstance(signal.triggered, bool)

    def test_jpeg_input_works(self):
        """JPEG input should be handled correctly."""
        img = Image.new("RGB", (200, 200), (100, 150, 200))
        buf = io.BytesIO()
        img.save(buf, format="JPEG", quality=85)
        result = perform_deepfake_detection(buf.getvalue())
        assert 0.0 <= result.score <= 1.0
