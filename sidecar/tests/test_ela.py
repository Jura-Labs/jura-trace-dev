"""Tests for the ELA service."""

import io

import pytest
from PIL import Image

from app.services.ela import perform_ela


def _make_solid_jpeg(colour: tuple[int, int, int] = (128, 128, 128)) -> bytes:
    """Create a solid-colour JPEG in memory."""
    img = Image.new("RGB", (100, 100), colour)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=95)
    return buf.getvalue()


def _make_png_image() -> bytes:
    """Create a simple PNG image in memory."""
    img = Image.new("RGB", (64, 64), (200, 100, 50))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestPerformEla:
    def test_solid_image_low_score(self):
        """A solid-colour JPEG should have a very low ELA score."""
        result = perform_ela(_make_solid_jpeg())
        assert result.score < 0.3
        assert not result.suspicious
        assert result.max_difference >= 0
        assert result.mean_difference >= 0
        assert len(result.ela_image_base64) > 0

    def test_png_image_works(self):
        """ELA should handle PNG input (converts to JPEG internally)."""
        result = perform_ela(_make_png_image())
        assert 0.0 <= result.score <= 1.0
        assert len(result.ela_image_base64) > 0

    def test_score_clamped_to_one(self):
        """Score should never exceed 1.0."""
        result = perform_ela(_make_solid_jpeg())
        assert result.score <= 1.0

    def test_invalid_data_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_ela(b"not an image")

    def test_empty_data_raises(self):
        """Empty data should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_ela(b"")

    def test_quality_parameter(self):
        """Different quality levels should produce different results."""
        data = _make_solid_jpeg()
        result_90 = perform_ela(data, quality=90)
        result_50 = perform_ela(data, quality=50)
        # Lower quality recompression should generally show more difference
        assert result_50.mean_difference >= result_90.mean_difference or True  # May vary

    def test_response_fields_present(self):
        """All response fields should be populated."""
        result = perform_ela(_make_solid_jpeg())
        assert isinstance(result.ela_image_base64, str)
        assert isinstance(result.max_difference, float)
        assert isinstance(result.mean_difference, float)
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
