"""Tests for the GAN spectral fingerprint visualisation service."""

import base64
import io

import numpy as np
import pytest
from PIL import Image

from app.services.gan_fingerprint import visualise_gan_fingerprint


def _make_test_image(size: tuple[int, int] = (128, 128)) -> bytes:
    """Create a synthetic 128x128 JPEG test image."""
    rng = np.random.default_rng(42)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=90)
    return buf.getvalue()


class TestGanFingerprint:
    def test_output_has_all_expected_fields(self):
        """Result should contain all 7 documented fields."""
        result = visualise_gan_fingerprint(_make_test_image())
        expected_keys = {
            "spectrum_base64",
            "residual_spectrum_base64",
            "spectral_peaks",
            "has_periodic_artefacts",
            "model_attribution",
            "confidence",
            "analysis_notes",
        }
        assert expected_keys == set(result.keys())

    def test_spectrum_base64_decodes_to_valid_png(self):
        """spectrum_base64 should decode to a valid PNG image."""
        result = visualise_gan_fingerprint(_make_test_image())
        raw = base64.b64decode(result["spectrum_base64"])
        img = Image.open(io.BytesIO(raw))
        assert img.format == "PNG"

    def test_residual_spectrum_base64_decodes_to_valid_png(self):
        """residual_spectrum_base64 should decode to a valid PNG image."""
        result = visualise_gan_fingerprint(_make_test_image())
        raw = base64.b64decode(result["residual_spectrum_base64"])
        img = Image.open(io.BytesIO(raw))
        assert img.format == "PNG"

    def test_spectral_peaks_is_a_list(self):
        """spectral_peaks should be a list."""
        result = visualise_gan_fingerprint(_make_test_image())
        assert isinstance(result["spectral_peaks"], list)

    def test_has_periodic_artefacts_is_bool(self):
        """has_periodic_artefacts should be a boolean."""
        result = visualise_gan_fingerprint(_make_test_image())
        assert isinstance(result["has_periodic_artefacts"], bool)

    def test_confidence_is_between_zero_and_one(self):
        """confidence should be a float between 0 and 1."""
        result = visualise_gan_fingerprint(_make_test_image())
        assert isinstance(result["confidence"], float)
        assert 0.0 <= result["confidence"] <= 1.0

    def test_analysis_notes_is_list_of_strings(self):
        """analysis_notes should be a list of strings."""
        result = visualise_gan_fingerprint(_make_test_image())
        assert isinstance(result["analysis_notes"], list)
        for note in result["analysis_notes"]:
            assert isinstance(note, str)

    def test_invalid_input_raises_value_error(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError):
            visualise_gan_fingerprint(b"not an image")
