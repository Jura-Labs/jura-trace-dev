"""Tests for the frequency domain visualisation service."""

import base64
import io

import pytest
from PIL import Image

from app.services.frequency_visualisation import perform_frequency_visualisation


def _make_jpeg(size: tuple[int, int] = (128, 128)) -> bytes:
    """Create a synthetic JPEG image in memory."""
    img = Image.new("RGB", size, (100, 150, 200))
    from PIL import ImageDraw

    draw = ImageDraw.Draw(img)
    draw.rectangle([20, 20, 60, 60], fill=(255, 0, 0))
    draw.rectangle([70, 70, 110, 110], fill=(0, 0, 255))
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


def _make_png(size: tuple[int, int] = (128, 128)) -> bytes:
    """Create a synthetic PNG image in memory."""
    img = Image.new("RGB", size, (50, 100, 150))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestPerformFrequencyVisualisation:
    def test_output_has_all_fields(self):
        """Result should contain all four expected fields."""
        result = perform_frequency_visualisation(_make_jpeg())
        assert "fft_magnitude_base64" in result
        assert "dct_heatmap_base64" in result
        assert "has_jpeg_grid" in result
        assert "dominant_frequency" in result

    def test_fft_magnitude_is_valid_png(self):
        """FFT magnitude should decode to a valid PNG image."""
        result = perform_frequency_visualisation(_make_jpeg())
        raw = base64.b64decode(result["fft_magnitude_base64"])
        img = Image.open(io.BytesIO(raw))
        assert img.format == "PNG"

    def test_dct_heatmap_is_valid_png(self):
        """DCT heatmap should decode to a valid PNG image."""
        result = perform_frequency_visualisation(_make_jpeg())
        raw = base64.b64decode(result["dct_heatmap_base64"])
        img = Image.open(io.BytesIO(raw))
        assert img.format == "PNG"

    def test_dominant_frequency_is_positive_float(self):
        """dominant_frequency should be a positive float."""
        result = perform_frequency_visualisation(_make_jpeg())
        assert isinstance(result["dominant_frequency"], float)
        assert result["dominant_frequency"] >= 0.0

    def test_has_jpeg_grid_is_bool(self):
        """has_jpeg_grid should be a boolean."""
        result = perform_frequency_visualisation(_make_jpeg())
        assert isinstance(result["has_jpeg_grid"], bool)

    def test_invalid_input_raises_valueerror(self):
        """Invalid image bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Could not decode image"):
            perform_frequency_visualisation(b"not an image at all")

    def test_empty_input_raises_valueerror(self):
        """Empty bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Could not decode image"):
            perform_frequency_visualisation(b"")

    def test_png_input_works(self):
        """Service should handle PNG input as well as JPEG."""
        result = perform_frequency_visualisation(_make_png())
        assert "fft_magnitude_base64" in result
        assert result["dominant_frequency"] >= 0.0
