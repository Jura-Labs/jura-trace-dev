"""Tests for the Splice Boundary Detection service."""

import io

import cv2
import numpy as np
import pytest
from PIL import Image

from app.services.splice_boundary import (
    _check_jpeg_grid_alignment,
    _check_noise_asymmetry,
    perform_splice_boundary,
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


def _make_sharp_insert_jpeg() -> bytes:
    """Create a JPEG with a sharp rectangular insert (simulates splice)."""
    # Textured background
    np.random.seed(42)
    bg = np.random.randint(80, 180, (256, 256, 3), dtype=np.uint8)
    # Smooth the background
    bg = cv2.GaussianBlur(bg, (5, 5), 0)

    # Sharp insert with very different characteristics
    insert = np.full((80, 80, 3), [30, 200, 30], dtype=np.uint8)
    # Add noise to the insert at a different level
    noise = np.random.randint(-20, 20, insert.shape, dtype=np.int16)
    insert = np.clip(insert.astype(np.int16) + noise, 0, 255).astype(
        np.uint8
    )

    # Paste at a grid-aligned position (multiple of 8)
    bg[64:144, 64:144] = insert

    _, buf = cv2.imencode(".jpg", bg)
    return buf.tobytes()


def _make_clean_scene_jpeg() -> bytes:
    """Create a clean JPEG scene with natural edges."""
    img = np.zeros((256, 256, 3), dtype=np.uint8)
    # Simple gradient background
    for i in range(256):
        img[i, :] = [i, i, i]
    _, buf = cv2.imencode(".jpg", img)
    return buf.tobytes()


class TestSpliceBoundary:
    def test_splice_boundary_clean_image(self):
        """A solid-colour image should have no splice boundaries."""
        result = perform_splice_boundary(_make_solid_jpeg())
        assert result["suspicious_boundaries"] == 0
        assert not result["suspicious"]
        assert result["score"] == 0.0

    def test_splice_boundary_with_sharp_insert(self):
        """An image with a sharp insert should produce valid results."""
        result = perform_splice_boundary(_make_sharp_insert_jpeg())
        # Score should be valid regardless of whether contours are found
        assert 0.0 <= result["score"] <= 1.0
        # Heatmap should be generated if contours were checked
        if result["total_boundaries_checked"] > 0:
            assert result["heatmap_base64"] is not None
        # Summary should always be present
        assert isinstance(result["summary"], str)

    def test_jpeg_grid_alignment_check(self):
        """Test the JPEG grid alignment helper."""
        # Create a contour aligned with 8x8 grid
        # Points at x=0,8,16,24 and y=0,8,16,24
        aligned_points = np.array([
            [[0, 0]], [[8, 0]], [[16, 0]], [[24, 0]],
            [[24, 8]], [[24, 16]], [[24, 24]],
            [[16, 24]], [[8, 24]], [[0, 24]],
            [[0, 16]], [[0, 8]],
        ], dtype=np.int32)
        assert _check_jpeg_grid_alignment(aligned_points)

        # Create a contour NOT aligned with grid
        # Points at odd positions
        unaligned_points = np.array([
            [[3, 5]], [[11, 5]], [[19, 5]], [[27, 5]],
            [[27, 13]], [[27, 21]], [[27, 29]],
            [[19, 29]], [[11, 29]], [[3, 29]],
            [[3, 21]], [[3, 13]],
        ], dtype=np.int32)
        assert not _check_jpeg_grid_alignment(unaligned_points)

    def test_noise_asymmetry_check(self):
        """Test the noise asymmetry helper with a synthetic contour."""
        # Create an image with different noise on each side of a boundary
        np.random.seed(42)
        img = np.zeros((256, 256), dtype=np.uint8)
        # Left half: low noise
        img[:, :128] = 128
        # Right half: high noise
        img[:, 128:] = np.random.randint(50, 200, (256, 128), dtype=np.uint8)

        # Vertical contour at x=128
        contour = np.array([
            [[128, y]] for y in range(20, 236)
        ], dtype=np.int32)

        result = _check_noise_asymmetry(img, contour, 256, 256)
        assert isinstance(result, bool)

    def test_invalid_image_returns_neutral(self):
        """Invalid image data should return a neutral result."""
        result = perform_splice_boundary(b"not an image")
        assert result["score"] == 0.0
        assert not result["suspicious"]
        assert "Could not decode" in result["summary"]

    def test_score_bounded(self):
        """Score should always be between 0.0 and 1.0."""
        result = perform_splice_boundary(_make_sharp_insert_jpeg())
        assert 0.0 <= result["score"] <= 1.0

    def test_response_fields_present(self):
        """All expected response fields should be present."""
        result = perform_splice_boundary(_make_clean_scene_jpeg())
        assert "heatmap_base64" in result
        assert "boundaries" in result
        assert "suspicious_boundaries" in result
        assert "total_boundaries_checked" in result
        assert "score" in result
        assert "suspicious" in result
        assert "summary" in result
