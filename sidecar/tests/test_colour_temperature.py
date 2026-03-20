"""Tests for the Colour Temperature service."""

import io

import cv2
import numpy as np
import pytest
from PIL import Image

from app.services.colour_temperature import (
    _check_colour_clusters,
    perform_colour_temperature,
)


def _make_solid_jpeg(
    size: tuple[int, int] = (256, 256),
    colour: tuple[int, int, int] = (128, 128, 128),
) -> bytes:
    """Create a solid-colour JPEG in memory."""
    img = Image.new("RGB", size, colour)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=95)
    return buf.getvalue()


def _make_warm_patch_jpeg() -> bytes:
    """Create a JPEG with a warm-coloured patch in one quadrant."""
    # Cool blue-grey background
    img = np.full((256, 256, 3), [180, 180, 200], dtype=np.uint8)
    # Warm orange patch in top-left quadrant
    img[0:128, 0:128] = [40, 100, 240]  # BGR: warm orange
    _, buf = cv2.imencode(".jpg", img)
    return buf.tobytes()


def _make_uniform_jpeg() -> bytes:
    """Create a uniform mid-tone JPEG."""
    img = np.full((256, 256, 3), [128, 128, 128], dtype=np.uint8)
    _, buf = cv2.imencode(".jpg", img)
    return buf.tobytes()


class TestColourTemperature:
    def test_colour_temperature_uniform_image(self):
        """A uniform image should have no anomalous regions."""
        result = perform_colour_temperature(_make_uniform_jpeg())
        assert result["score"] == 0.0
        assert not result["suspicious"]
        assert result["anomalous_regions"] == 0
        assert result["total_regions"] == 16  # 4x4 grid

    def test_colour_temperature_returns_regions(self):
        """Result should contain 16 regions for a 4x4 grid."""
        result = perform_colour_temperature(_make_solid_jpeg())
        assert len(result["regions"]) == 16
        for region in result["regions"]:
            assert "x" in region
            assert "y" in region
            assert "width" in region
            assert "height" in region
            assert "mean_a" in region
            assert "mean_b" in region
            assert "deviation_from_global" in region
            assert "anomalous" in region

    def test_colour_temperature_anomaly_with_warm_patch(self):
        """An image with a warm patch should detect colour anomalies."""
        result = perform_colour_temperature(_make_warm_patch_jpeg())
        assert result["total_regions"] == 16
        # The warm patch should create some deviation
        assert result["heatmap_base64"] is not None
        # Score should be valid
        assert 0.0 <= result["score"] <= 1.0
        # At least some regions should show deviation
        max_dev = max(r["deviation_from_global"] for r in result["regions"])
        assert max_dev > 0.0

    def test_colour_cluster_detection(self):
        """Test the cluster detection helper with known patterns."""
        # No anomalies — no cluster
        regions_clean = [
            {"anomalous": False, "width": 64, "height": 64}
            for _ in range(16)
        ]
        assert not _check_colour_clusters(regions_clean, 4, 4, 256, 256)

        # Four adjacent anomalous regions (top-left 2x2 block)
        # In 4x4 grid: indices 0,1,4,5 are (0,0),(0,1),(1,0),(1,1)
        regions_cluster = [
            {"anomalous": False, "width": 64, "height": 64}
            for _ in range(16)
        ]
        regions_cluster[0]["anomalous"] = True
        regions_cluster[1]["anomalous"] = True
        regions_cluster[4]["anomalous"] = True
        regions_cluster[5]["anomalous"] = True
        # 4 cells x 64x64 = 16384 pixels, image = 256x256 = 65536
        # 16384/65536 = 25% > 10%
        assert _check_colour_clusters(regions_cluster, 4, 4, 256, 256)

    def test_invalid_image_returns_neutral(self):
        """Invalid image data should return a neutral result."""
        result = perform_colour_temperature(b"not an image")
        assert result["score"] == 0.0
        assert not result["suspicious"]
        assert "Could not decode" in result["summary"]

    def test_global_mean_values(self):
        """Global mean A and B should be present and valid."""
        result = perform_colour_temperature(_make_solid_jpeg())
        assert isinstance(result["global_mean_a"], float)
        assert isinstance(result["global_mean_b"], float)

    def test_score_bounded(self):
        """Score should always be between 0.0 and 1.0."""
        result = perform_colour_temperature(_make_warm_patch_jpeg())
        assert 0.0 <= result["score"] <= 1.0
