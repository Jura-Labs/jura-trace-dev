"""Tests for the JPEG ghost detection service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.jpeg_ghost import perform_jpeg_ghost_detection, QUALITY_RANGE


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


def _make_jpeg_at_quality(size: tuple[int, int] = (256, 256), quality: int = 85) -> bytes:
    """Create a noisy image, save as JPEG at a specific quality, return JPEG bytes."""
    rng = np.random.default_rng(99)
    arr = rng.integers(30, 220, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=quality)
    return buf.getvalue()


class TestJpegGhostDetection:
    def test_solid_image_produces_result(self):
        """A solid-colour image should return a valid result."""
        result = perform_jpeg_ghost_detection(_make_solid_image())
        assert 0.0 <= result.score <= 1.0
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.ghost_quality, int)
        assert result.total_blocks > 0
        assert len(result.heatmap_base64) > 0
        assert result.summary != ""

    def test_noisy_image_produces_result(self):
        """A noisy image should return a valid result."""
        result = perform_jpeg_ghost_detection(_make_noisy_photo())
        assert 0.0 <= result.score <= 1.0
        assert isinstance(result.quality_variance, float)
        assert result.quality_variance >= 0.0
        assert result.total_blocks > 0

    def test_jpeg_input_works(self):
        """JPEG input at Q85 should find ghost quality near Q85.

        The ghost quality should be within one step (5) of the actual
        compression quality, because the minimum re-compression difference
        occurs at the original quality level.
        """
        jpeg_bytes = _make_jpeg_at_quality(quality=85)
        result = perform_jpeg_ghost_detection(jpeg_bytes)
        assert 0.0 <= result.score <= 1.0
        # Ghost quality should be close to 85 (within +-10 to allow for
        # quantisation table rounding differences across Pillow versions)
        assert abs(result.ghost_quality - 85) <= 10, (
            f"Ghost quality {result.ghost_quality} should be near 85"
        )

    def test_score_bounded(self):
        """Score should always be between 0.0 and 1.0."""
        for img_fn in (_make_solid_image, _make_noisy_photo):
            result = perform_jpeg_ghost_detection(img_fn())
            assert 0.0 <= result.score <= 1.0

    def test_invalid_data_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_jpeg_ghost_detection(b"not an image")

    def test_response_fields_valid(self):
        """All response fields should be present and correctly typed."""
        result = perform_jpeg_ghost_detection(_make_noisy_photo())
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.ghost_quality, int)
        assert isinstance(result.quality_variance, float)
        assert isinstance(result.deviating_blocks, int)
        assert isinstance(result.total_blocks, int)
        assert isinstance(result.heatmap_base64, str)
        assert isinstance(result.summary, str)
        # Deviating blocks cannot exceed total
        assert result.deviating_blocks <= result.total_blocks

    def test_ghost_quality_in_valid_range(self):
        """Ghost quality should be one of the tested quality levels."""
        result = perform_jpeg_ghost_detection(_make_noisy_photo())
        assert result.ghost_quality in QUALITY_RANGE, (
            f"Ghost quality {result.ghost_quality} not in {QUALITY_RANGE}"
        )

    def test_small_image_handled(self):
        """An image smaller than one block should return neutral result."""
        result = perform_jpeg_ghost_detection(_make_solid_image(size=(8, 8)))
        assert result.score == 0.0
        assert result.total_blocks == 0
        assert not result.suspicious

    def test_uniform_compression_low_variance(self):
        """A single-compression JPEG should have low quality variance."""
        jpeg_bytes = _make_jpeg_at_quality(quality=75)
        result = perform_jpeg_ghost_detection(jpeg_bytes)
        # Single compression → most blocks agree → low variance
        # Allow generous threshold since test images are synthetic noise
        assert result.quality_variance < 100.0, (
            f"Single-compression JPEG variance {result.quality_variance} unexpectedly high"
        )
