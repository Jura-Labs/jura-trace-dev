"""Tests for the deepfake / AI-generated image detection service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.deepfake import (
    perform_deepfake_detection,
    detect_sd_watermark,
    _extract_patch_spectral_features,
    _extract_multiscale_gradient_features,
)


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
        assert len(result.signals) == 13  # 13 signals in the ensemble

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
        """All 13 ensemble signals should be present."""
        result = perform_deepfake_detection(_make_noisy_photo())
        assert len(result.signals) == 13
        names = {s.name for s in result.signals}
        expected_names = {
            "noise_residual",
            "noise_smoothed_kurtosis",
            "prnu_asymmetry",
            "noise_consistency",
            "frequency_energy",
            "spectral_decay",
            "texture_consistency",
            "patch_spectral_variance",
            "channel_correlation",
            "color_gamut",
            "sharpness_consistency",
            "multiscale_gradient",
            "benford_divergence",
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

    def test_patch_spectral_features_valid(self):
        """Patch spectral CV should be a non-negative float."""
        rng = np.random.default_rng(42)
        arr = rng.integers(50, 200, (256, 256), dtype=np.uint8)
        features = _extract_patch_spectral_features(arr)
        assert "patch_spectral_cv" in features
        assert features["patch_spectral_cv"] >= 0.0

    def test_patch_spectral_small_image(self):
        """Small image should return default patch spectral CV."""
        arr = np.zeros((32, 32), dtype=np.uint8)
        features = _extract_patch_spectral_features(arr)
        assert features["patch_spectral_cv"] == 1.0

    def test_multiscale_gradient_features_valid(self):
        """Multi-scale gradient ratio should be a non-negative float."""
        rng = np.random.default_rng(42)
        arr = rng.integers(50, 200, (256, 256), dtype=np.uint8)
        features = _extract_multiscale_gradient_features(arr)
        assert "multiscale_gradient_ratio" in features
        assert features["multiscale_gradient_ratio"] >= 0.0

    def test_multiscale_gradient_small_image(self):
        """Small image should return default gradient ratio."""
        arr = np.zeros((8, 8), dtype=np.uint8)
        features = _extract_multiscale_gradient_features(arr)
        assert features["multiscale_gradient_ratio"] == 0.4

    def test_noise_consistency_signal_present(self):
        """The noise_consistency signal should be present and valid."""
        result = perform_deepfake_detection(_make_noisy_photo())
        noise_signals = [s for s in result.signals if s.name == "noise_consistency"]
        assert len(noise_signals) == 1
        assert noise_signals[0].weight == 1.5
        assert isinstance(noise_signals[0].triggered, bool)

    def test_watermarks_field_present(self):
        """DeepfakeResponse should always include the watermarks field."""
        result = perform_deepfake_detection(_make_noisy_photo())
        assert hasattr(result, "watermarks")
        assert isinstance(result.watermarks, list)

    def test_no_watermark_on_synthetic_image(self):
        """Synthetic test images should have no detected watermarks."""
        result = perform_deepfake_detection(_make_noisy_photo())
        detected = [w for w in result.watermarks if w.detected]
        assert len(detected) == 0

    def test_watermarks_small_image_skipped(self):
        """Images below 256x256 should not produce watermark detections."""
        result = perform_deepfake_detection(_make_solid_image(size=(128, 128)))
        detected = [w for w in result.watermarks if w.detected]
        assert len(detected) == 0


class TestWatermarkDetection:
    """Tests for the SD/SDXL/Flux invisible watermark detector."""

    def test_no_watermark_on_random_image(self):
        """A random image should not trigger watermark detection."""
        detections = detect_sd_watermark(_make_noisy_photo(size=(512, 512)))
        detected = [d for d in detections if d.detected]
        assert len(detected) == 0

    def test_small_image_returns_empty(self):
        """Images below minimum size should return empty list."""
        detections = detect_sd_watermark(_make_solid_image(size=(128, 128)))
        assert detections == []

    def test_invalid_bytes_returns_empty(self):
        """Invalid image bytes should return empty list, not raise."""
        detections = detect_sd_watermark(b"not an image")
        assert detections == []

    def test_detection_fields_valid(self):
        """Each detection should have all required fields with valid values."""
        detections = detect_sd_watermark(_make_noisy_photo(size=(512, 512)))
        for d in detections:
            assert isinstance(d.type, str) and d.type
            assert isinstance(d.detected, bool)
            assert isinstance(d.confidence, float)
            assert 0.0 <= d.confidence <= 1.0
            assert isinstance(d.details, str) and d.details
