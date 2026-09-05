# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the Shadow Consistency service."""

import io

import cv2
import numpy as np
from PIL import Image

from app.services.shadow_consistency import (
    _weighted_circular_mean,
    perform_shadow_consistency,
)


def _make_gradient_jpeg(
    size: tuple[int, int] = (256, 256),
) -> bytes:
    """Create a JPEG with a uniform horizontal gradient (consistent lighting)."""
    w, h = size
    # Horizontal gradient — bright on left, dark on right
    gradient = np.tile(np.linspace(255, 0, w, dtype=np.uint8), (h, 1))
    img = np.stack([gradient, gradient, gradient], axis=-1)
    _, buf = cv2.imencode(".jpg", img)
    return buf.tobytes()


def _make_solid_jpeg(
    size: tuple[int, int] = (256, 256),
    colour: tuple[int, int, int] = (128, 128, 128),
) -> bytes:
    """Create a solid-colour JPEG in memory."""
    img = Image.new("RGB", size, colour)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=95)
    return buf.getvalue()


def _make_scene_jpeg() -> bytes:
    """Create a JPEG with distinct foreground objects (high contrast)."""
    img = np.zeros((256, 256, 3), dtype=np.uint8)
    # Dark background
    img[:, :] = [30, 30, 30]
    # Bright rectangle (foreground object)
    img[50:150, 50:150] = [220, 220, 220]
    # Another bright rectangle
    img[100:200, 150:250] = [200, 200, 200]
    _, buf = cv2.imencode(".jpg", img)
    return buf.tobytes()


class TestShadowConsistency:
    def test_shadow_consistency_uniform_image(self):
        """A solid-colour image may lack foreground components."""
        result = perform_shadow_consistency(_make_solid_jpeg())
        # May return neutral if Otsu can't segment
        assert result["score"] >= 0.0
        assert isinstance(result["summary"], str)

    def test_shadow_consistency_returns_regions(self):
        """A scene with objects should return region data."""
        result = perform_shadow_consistency(_make_scene_jpeg())
        assert isinstance(result["regions"], list)
        assert result["total_regions"] >= 0
        if result["total_regions"] > 0:
            region = result["regions"][0]
            assert "x" in region
            assert "y" in region
            assert "width" in region
            assert "height" in region
            assert "gradient_angle_mean" in region
            assert "deviation_from_global" in region
            assert "inconsistent" in region

    def test_shadow_consistency_global_direction(self):
        """A gradient image should produce a meaningful global direction."""
        result = perform_shadow_consistency(_make_gradient_jpeg())
        # The global light direction should be a valid angle
        assert -180.0 <= result["global_light_direction"] <= 180.0

    def test_weighted_circular_mean(self):
        """Test the circular mean helper with known values."""
        # All angles at 0 degrees should give ~0
        angles = np.zeros(100)
        weights = np.ones(100)
        mean = _weighted_circular_mean(angles, weights)
        assert abs(mean) < 1.0

        # All angles at 90 degrees should give ~90
        angles_90 = np.full(100, 90.0)
        mean_90 = _weighted_circular_mean(angles_90, weights)
        assert abs(mean_90 - 90.0) < 1.0

        # Opposite angles should cancel depending on weights
        angles_mixed = np.array([0.0, 180.0])
        weights_equal = np.array([1.0, 1.0])
        # Result is undefined direction but should not raise
        _weighted_circular_mean(angles_mixed, weights_equal)

    def test_invalid_image_returns_neutral(self):
        """Invalid image data should return a neutral result."""
        result = perform_shadow_consistency(b"not an image")
        assert result["score"] == 0.0
        assert not result["suspicious"]
        assert "Could not decode" in result["summary"]

    def test_score_bounded(self):
        """Score should always be between 0.0 and 1.0."""
        result = perform_shadow_consistency(_make_scene_jpeg())
        assert 0.0 <= result["score"] <= 1.0
