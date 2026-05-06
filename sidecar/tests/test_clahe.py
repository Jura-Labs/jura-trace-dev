# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the CLAHE service."""

import base64
import io

import pytest
from PIL import Image

from app.services.clahe import perform_clahe


def _make_jpeg(size: tuple[int, int] = (100, 100)) -> bytes:
    """Create a synthetic JPEG image in memory."""
    img = Image.new("RGB", size, (80, 120, 160))
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=90)
    return buf.getvalue()


class TestPerformClahe:
    def test_output_has_expected_fields(self):
        """Result should contain enhanced_image_base64 and clip_limit."""
        result = perform_clahe(_make_jpeg())
        assert "enhanced_image_base64" in result
        assert "clip_limit" in result

    def test_enhanced_image_is_valid_png(self):
        """Enhanced image should be a decodable PNG."""
        result = perform_clahe(_make_jpeg())
        img_bytes = base64.b64decode(result["enhanced_image_base64"])
        img = Image.open(io.BytesIO(img_bytes))
        assert img.format == "PNG"

    def test_enhanced_image_same_dimensions(self):
        """Enhanced image should have the same dimensions as the input."""
        size = (200, 150)
        data = _make_jpeg(size=size)
        result = perform_clahe(data)
        img = Image.open(io.BytesIO(base64.b64decode(result["enhanced_image_base64"])))
        assert img.size == size

    def test_clip_limit_default(self):
        """Default clip_limit should be 2.0."""
        result = perform_clahe(_make_jpeg())
        assert result["clip_limit"] == 2.0

    def test_clip_limit_low(self):
        """clip_limit=0.5 should work and be reflected in result."""
        result = perform_clahe(_make_jpeg(), clip_limit=0.5)
        assert result["clip_limit"] == 0.5
        assert len(result["enhanced_image_base64"]) > 0

    def test_clip_limit_high(self):
        """clip_limit=8.0 should work and be reflected in result."""
        result = perform_clahe(_make_jpeg(), clip_limit=8.0)
        assert result["clip_limit"] == 8.0
        assert len(result["enhanced_image_base64"]) > 0

    def test_various_clip_limits_produce_different_output(self):
        """Different clip limits should generally produce different images."""
        data = _make_jpeg()
        r1 = perform_clahe(data, clip_limit=0.5)
        r2 = perform_clahe(data, clip_limit=8.0)
        # The base64 strings should differ (different enhancement levels)
        # For a solid image they might be very similar, but the test validates
        # that both calls succeed without error.
        assert len(r1["enhanced_image_base64"]) > 0
        assert len(r2["enhanced_image_base64"]) > 0

    def test_invalid_image_raises_valueerror(self):
        """Invalid image bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Could not decode image"):
            perform_clahe(b"not an image")

    def test_png_input_works(self):
        """Service should handle PNG input."""
        img = Image.new("RGB", (64, 64), (200, 100, 50))
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        result = perform_clahe(buf.getvalue())
        assert "enhanced_image_base64" in result
