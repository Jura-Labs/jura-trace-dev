"""Tests for the seasonal indicator analysis service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.seasonal_indicators import analyse_seasonal_indicators


def _make_test_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a simple test image."""
    rng = np.random.default_rng(42)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_green_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a strongly green image to simulate lush vegetation."""
    arr = np.full((*size[::-1], 3), 0, dtype=np.uint8)
    arr[:, :, 1] = 180  # strong green channel
    arr[:, :, 0] = 30   # low red
    arr[:, :, 2] = 20   # low blue
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestSeasonalIndicators:
    def test_output_has_expected_fields(self):
        """Result should contain all documented fields."""
        result = analyse_seasonal_indicators(_make_test_image())
        expected_keys = {
            "greenness_index",
            "snow_coverage",
            "warmth_index",
            "estimated_season",
            "confidence",
            "indicators",
        }
        assert expected_keys == set(result.keys())

    def test_greenness_index_range(self):
        """greenness_index should be between 0 and 1."""
        result = analyse_seasonal_indicators(_make_test_image())
        assert 0.0 <= result["greenness_index"] <= 1.0

    def test_snow_coverage_range(self):
        """snow_coverage should be between 0 and 1."""
        result = analyse_seasonal_indicators(_make_test_image())
        assert 0.0 <= result["snow_coverage"] <= 1.0

    def test_estimated_season_valid(self):
        """estimated_season should be one of the expected values."""
        result = analyse_seasonal_indicators(_make_test_image())
        valid_seasons = {"spring", "summer", "autumn", "winter", "indeterminate"}
        assert result["estimated_season"] in valid_seasons

    def test_indicators_is_list_of_strings(self):
        """indicators should be a list of strings."""
        result = analyse_seasonal_indicators(_make_test_image())
        assert isinstance(result["indicators"], list)
        for item in result["indicators"]:
            assert isinstance(item, str)

    def test_green_image_detects_greenness(self):
        """A very green image should have a high greenness index."""
        result = analyse_seasonal_indicators(_make_green_image())
        assert result["greenness_index"] > 0.5

    def test_invalid_input_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError):
            analyse_seasonal_indicators(b"not an image")
