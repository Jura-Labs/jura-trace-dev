"""Tests for the JPEG quantisation grid visualisation service."""

import base64
import io

import pytest
from PIL import Image

from app.services.jpeg_grid import perform_jpeg_grid_visualisation


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


class TestPerformJpegGridVisualisation:
    def test_output_has_all_fields(self):
        """Result should contain all three expected fields."""
        result = perform_jpeg_grid_visualisation(_make_jpeg())
        assert "grid_artefact_base64" in result
        assert "q_table" in result
        assert "grid_consistency" in result

    def test_grid_artefact_is_valid_png(self):
        """Grid artefact heatmap should decode to a valid PNG image."""
        result = perform_jpeg_grid_visualisation(_make_jpeg())
        raw = base64.b64decode(result["grid_artefact_base64"])
        img = Image.open(io.BytesIO(raw))
        assert img.format == "PNG"

    def test_grid_consistency_in_range(self):
        """grid_consistency should be between 0 and 1."""
        result = perform_jpeg_grid_visualisation(_make_jpeg())
        assert isinstance(result["grid_consistency"], float)
        assert 0.0 <= result["grid_consistency"] <= 1.0

    def test_q_table_present_for_jpeg(self):
        """JPEG input should produce a non-None Q-table."""
        result = perform_jpeg_grid_visualisation(_make_jpeg())
        assert result["q_table"] is not None
        assert len(result["q_table"]) == 8
        assert len(result["q_table"][0]) == 8

    def test_q_table_none_for_png(self):
        """PNG input should have q_table=None (no JPEG Q-table)."""
        result = perform_jpeg_grid_visualisation(_make_png())
        assert result["q_table"] is None

    def test_invalid_input_raises_valueerror(self):
        """Invalid image bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Could not decode image"):
            perform_jpeg_grid_visualisation(b"not an image at all")

    def test_empty_input_raises_valueerror(self):
        """Empty bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Could not decode image"):
            perform_jpeg_grid_visualisation(b"")
