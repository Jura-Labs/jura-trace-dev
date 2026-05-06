# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the noise visualisation service."""

import base64
import io

import pytest
from PIL import Image

from app.services.noise_visualisation import perform_noise_visualisation


def _make_jpeg(size: tuple[int, int] = (128, 128)) -> bytes:
    """Create a synthetic JPEG image in memory."""
    img = Image.new("RGB", size, (100, 150, 200))
    # Add some variation so noise is non-trivial
    from PIL import ImageDraw
    draw = ImageDraw.Draw(img)
    draw.rectangle([20, 20, 60, 60], fill=(255, 0, 0))
    draw.rectangle([70, 70, 110, 110], fill=(0, 0, 255))
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


class TestPerformNoiseVisualisation:
    def test_output_has_all_fields(self):
        """Result should contain all four expected fields."""
        result = perform_noise_visualisation(_make_jpeg())
        assert "noise_residual_base64" in result
        assert "variance_heatmap_base64" in result
        assert "noise_std" in result
        assert "noise_mean" in result

    def test_base64_strings_are_valid(self):
        """Base64 strings should decode without error."""
        result = perform_noise_visualisation(_make_jpeg())
        noise_bytes = base64.b64decode(result["noise_residual_base64"])
        assert len(noise_bytes) > 0
        variance_bytes = base64.b64decode(result["variance_heatmap_base64"])
        assert len(variance_bytes) > 0

    def test_noise_residual_is_valid_png(self):
        """Noise residual should be a decodable PNG image."""
        result = perform_noise_visualisation(_make_jpeg())
        img = Image.open(io.BytesIO(base64.b64decode(result["noise_residual_base64"])))
        assert img.format == "PNG"

    def test_variance_heatmap_is_valid_png(self):
        """Variance heatmap should be a decodable PNG image."""
        result = perform_noise_visualisation(_make_jpeg())
        img = Image.open(io.BytesIO(base64.b64decode(result["variance_heatmap_base64"])))
        assert img.format == "PNG"

    def test_noise_std_is_non_negative_float(self):
        """noise_std should be a non-negative float."""
        result = perform_noise_visualisation(_make_jpeg())
        assert isinstance(result["noise_std"], float)
        assert result["noise_std"] >= 0.0

    def test_noise_mean_is_non_negative_float(self):
        """noise_mean should be a non-negative float."""
        result = perform_noise_visualisation(_make_jpeg())
        assert isinstance(result["noise_mean"], float)
        assert result["noise_mean"] >= 0.0

    def test_invalid_image_raises_valueerror(self):
        """Invalid image bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Could not decode image"):
            perform_noise_visualisation(b"not an image at all")

    def test_png_input_works(self):
        """Service should handle PNG input as well as JPEG."""
        img = Image.new("RGB", (100, 100), (50, 100, 150))
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        result = perform_noise_visualisation(buf.getvalue())
        assert "noise_residual_base64" in result
        assert result["noise_std"] >= 0.0
