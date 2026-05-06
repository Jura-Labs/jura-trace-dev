# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the ROI forensic analysis service."""

import base64
import io

import numpy as np
import pytest
from PIL import Image

from app.services.roi_analysis import analyse_roi


def _make_test_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a simple test image with some texture."""
    rng = np.random.default_rng(42)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestRoiAnalysis:
    def test_output_has_expected_fields(self):
        """Result should contain all documented fields."""
        result = analyse_roi(_make_test_image(), x=10, y=10, width=100, height=100)
        expected_keys = {
            "noise_std",
            "noise_mean",
            "ela_mean",
            "frequency_energy",
            "texture_complexity",
            "noise_residual_base64",
            "roi",
        }
        assert expected_keys == set(result.keys())

    def test_noise_std_non_negative(self):
        """noise_std should be non-negative."""
        result = analyse_roi(_make_test_image(), x=0, y=0, width=128, height=128)
        assert result["noise_std"] >= 0.0

    def test_ela_mean_non_negative(self):
        """ela_mean should be non-negative."""
        result = analyse_roi(_make_test_image(), x=0, y=0, width=128, height=128)
        assert result["ela_mean"] >= 0.0

    def test_frequency_energy_range(self):
        """frequency_energy should be between 0 and 1."""
        result = analyse_roi(_make_test_image(), x=0, y=0, width=128, height=128)
        assert 0.0 <= result["frequency_energy"] <= 1.0

    def test_roi_clamping(self):
        """ROI exceeding image bounds should be clamped, not error."""
        data = _make_test_image(size=(100, 100))
        result = analyse_roi(data, x=80, y=80, width=200, height=200)
        # The returned ROI should be clamped within image dimensions
        assert result["roi"]["x"] >= 0
        assert result["roi"]["y"] >= 0
        assert result["roi"]["x"] + result["roi"]["width"] <= 100
        assert result["roi"]["y"] + result["roi"]["height"] <= 100

    def test_noise_residual_is_valid_png(self):
        """The noise residual should be a decodable base64 PNG."""
        result = analyse_roi(_make_test_image(), x=10, y=10, width=100, height=100)
        raw = base64.b64decode(result["noise_residual_base64"])
        img = Image.open(io.BytesIO(raw))
        assert img.format == "PNG"

    def test_invalid_input_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError):
            analyse_roi(b"not an image", x=0, y=0, width=10, height=10)
