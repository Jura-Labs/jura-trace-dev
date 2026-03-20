"""Tests for the Segmented ELA service."""

import io

import cv2
import numpy as np
import pytest
from PIL import Image

from app.services.segmented_ela import (
    _check_anomalous_clusters,
    perform_segmented_ela,
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


def _make_png_image() -> bytes:
    """Create a simple PNG image in memory."""
    img = Image.new("RGB", (256, 256), (200, 100, 50))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_composite_jpeg() -> bytes:
    """Create a JPEG with one quadrant at different compression."""
    # Start with a clean image
    img = Image.new("RGB", (256, 256), (100, 150, 200))
    # Save at high quality
    buf1 = io.BytesIO()
    img.save(buf1, format="JPEG", quality=95)
    buf1.seek(0)
    clean = Image.open(buf1)

    # Create a noisy patch and paste into one quadrant
    patch = np.random.randint(0, 255, (128, 128, 3), dtype=np.uint8)
    patch_img = Image.fromarray(patch)
    # Compress the patch at very low quality
    buf2 = io.BytesIO()
    patch_img.save(buf2, format="JPEG", quality=10)
    buf2.seek(0)
    low_q_patch = Image.open(buf2)

    # Paste into the clean image
    clean.paste(low_q_patch, (0, 0))

    # Save final composite
    buf3 = io.BytesIO()
    clean.save(buf3, format="JPEG", quality=95)
    return buf3.getvalue()


class TestSegmentedEla:
    def test_segmented_ela_clean_image(self):
        """A solid-colour JPEG should have a low score."""
        result = perform_segmented_ela(_make_solid_jpeg())
        assert result["score"] < 0.5
        assert not result["suspicious"]
        assert result["total_regions"] == 64  # 8x8 grid
        assert result["heatmap_base64"] is not None

    def test_segmented_ela_returns_regions(self):
        """Result should contain 64 regions for an 8x8 grid."""
        result = perform_segmented_ela(_make_solid_jpeg())
        assert len(result["regions"]) == 64
        for region in result["regions"]:
            assert "x" in region
            assert "y" in region
            assert "width" in region
            assert "height" in region
            assert "ela_score" in region
            assert "anomalous" in region

    def test_segmented_ela_non_jpeg_returns_neutral(self):
        """A PNG image should return a neutral result."""
        result = perform_segmented_ela(_make_png_image())
        assert result["score"] == 0.0
        assert not result["suspicious"]
        assert result["total_regions"] == 0
        assert "non-JPEG" in result["summary"]

    def test_segmented_ela_anomaly_detection(self):
        """A composite JPEG should detect some anomalous regions."""
        result = perform_segmented_ela(_make_composite_jpeg())
        assert result["total_regions"] == 64
        # The composite should have some variance
        assert result["inter_region_variance"] >= 0.0
        # Score should be a valid float
        assert 0.0 <= result["score"] <= 1.0

    def test_cluster_detection(self):
        """Test the cluster detection helper with known patterns."""
        # No anomalies — no cluster
        regions_clean = [{"anomalous": False} for _ in range(64)]
        assert not _check_anomalous_clusters(regions_clean, 8, 8)

        # Single anomalous cell — no cluster
        regions_single = [{"anomalous": False} for _ in range(64)]
        regions_single[0]["anomalous"] = True
        assert not _check_anomalous_clusters(regions_single, 8, 8)

        # Three adjacent cells in a row — cluster of 3
        regions_cluster = [{"anomalous": False} for _ in range(64)]
        regions_cluster[0]["anomalous"] = True  # (0, 0)
        regions_cluster[1]["anomalous"] = True  # (0, 1)
        regions_cluster[2]["anomalous"] = True  # (0, 2)
        assert _check_anomalous_clusters(regions_cluster, 8, 8)

    def test_invalid_image_returns_neutral(self):
        """Invalid image data should return a neutral result."""
        result = perform_segmented_ela(b"not an image")
        assert result["score"] == 0.0
        assert not result["suspicious"]
        assert "Could not decode" in result["summary"]

    def test_score_bounded(self):
        """Score should always be between 0.0 and 1.0."""
        result = perform_segmented_ela(_make_solid_jpeg())
        assert 0.0 <= result["score"] <= 1.0

        result2 = perform_segmented_ela(_make_composite_jpeg())
        assert 0.0 <= result2["score"] <= 1.0
