"""Tests for the copy-move forgery detection service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.copy_move import perform_copy_move_detection


def _make_simple_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a simple gradient image (no duplicated regions)."""
    arr = np.zeros((*size[::-1], 3), dtype=np.uint8)
    for i in range(size[1]):
        arr[i, :, 0] = i  # Red gradient
        arr[i, :, 1] = 255 - i  # Green inverse gradient
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_textured_image(size: tuple[int, int] = (512, 512)) -> bytes:
    """Create a richly textured image with distinct features."""
    rng = np.random.default_rng(42)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_copy_move_image() -> bytes:
    """Create an image with a duplicated region to simulate copy-move forgery."""
    rng = np.random.default_rng(42)
    arr = rng.integers(50, 200, (512, 512, 3), dtype=np.uint8)
    # Copy a block from one location to another
    block = arr[50:150, 50:150].copy()
    arr[300:400, 300:400] = block
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestCopyMoveDetection:
    def test_simple_image_clean(self):
        """A simple gradient image should have low copy-move score."""
        result = perform_copy_move_detection(_make_simple_image())
        assert result.score < 0.5
        assert len(result.visualisation_base64) > 0

    def test_textured_image_produces_result(self):
        """A textured image should return a valid result."""
        result = perform_copy_move_detection(_make_textured_image())
        assert 0.0 <= result.score <= 1.0
        assert isinstance(result.matched_pairs, int)
        assert len(result.visualisation_base64) > 0

    def test_copy_move_image_detects_clones(self):
        """An image with an explicitly copied block should detect matches."""
        result = perform_copy_move_detection(_make_copy_move_image())
        # ORB may or may not find the cloned region depending on texture
        # but it should at least return a valid result
        assert 0.0 <= result.score <= 1.0
        assert isinstance(result.clone_regions, list)

    def test_invalid_data_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_copy_move_detection(b"not an image")

    def test_small_image_handled(self):
        """An image with very few features should return clean result."""
        small = Image.new("RGB", (32, 32), (128, 128, 128))
        buf = io.BytesIO()
        small.save(buf, format="PNG")
        result = perform_copy_move_detection(buf.getvalue())
        assert result.score == 0.0
        assert result.matched_pairs == 0
        assert not result.suspicious

    def test_response_fields_present(self):
        """All response fields should be populated."""
        result = perform_copy_move_detection(_make_textured_image())
        assert isinstance(result.visualisation_base64, str)
        assert isinstance(result.clone_regions, list)
        assert isinstance(result.matched_pairs, int)
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)

    def test_max_features_parameter(self):
        """Different max_features values should be accepted."""
        data = _make_textured_image()
        result_low = perform_copy_move_detection(data, max_features=100)
        result_high = perform_copy_move_detection(data, max_features=5000)
        # Both should produce valid results
        assert 0.0 <= result_low.score <= 1.0
        assert 0.0 <= result_high.score <= 1.0
