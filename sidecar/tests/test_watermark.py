# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the invisible watermark embed/extract service."""

import io

import cv2
import numpy as np
import pytest
from PIL import Image

from app.services.watermark import perform_watermark_embed, perform_watermark_extract


def _make_colour_image(size: tuple[int, int] = (512, 512)) -> bytes:
    """Create a colour image with varied content suitable for watermarking."""
    h, w = size
    rng = np.random.default_rng(42)
    arr = np.zeros((h, w, 3), dtype=np.uint8)
    for i in range(h):
        for j in range(w):
            arr[i, j] = [
                int(50 + 150 * (i / h)),
                int(100 + 80 * np.sin(j / w * 3.14)),
                int(80 + 100 * (j / w)),
            ]
    noise = rng.normal(0, 5, arr.shape).astype(np.int16)
    arr = np.clip(arr.astype(np.int16) + noise, 0, 255).astype(np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_small_image(size: tuple[int, int] = (128, 128)) -> bytes:
    """Create an image below the 256x256 minimum."""
    img = Image.new("RGB", size, (100, 150, 200))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestWatermarkEmbed:
    """Tests for watermark embedding."""

    def test_embed_returns_base64_image(self):
        """Successful embed should return a base64-encoded watermarked image."""
        result = perform_watermark_embed(_make_colour_image(), "TestPayload")
        assert result["success"] is True
        assert result["watermarked_image_base64"] is not None
        assert len(result["watermarked_image_base64"]) > 0
        assert result["algorithm"] == "dwtDctSvd"
        assert result["payload_length"] == len("TestPayload".encode("utf-8"))

    def test_embed_small_image_returns_error(self):
        """Images below 256x256 should return an error, not raise."""
        result = perform_watermark_embed(_make_small_image(), "Test")
        assert result["success"] is False
        assert "too small" in result["message"].lower()
        assert result["watermarked_image_base64"] is None

    def test_embed_invalid_image_returns_error(self):
        """Invalid image bytes should return an error, not raise."""
        result = perform_watermark_embed(b"not an image", "Test")
        assert result["success"] is False
        assert "could not decode" in result["message"].lower()

    def test_embed_respects_strength_parameter(self):
        """Strength parameter should be echoed back in the result."""
        for strength in ("low", "medium", "high"):
            result = perform_watermark_embed(
                _make_colour_image(), "Test", strength=strength
            )
            assert result["success"] is True
            assert result["strength"] == strength

    def test_embed_invalid_strength_defaults_to_medium(self):
        """Invalid strength should default to medium."""
        result = perform_watermark_embed(
            _make_colour_image(), "Test", strength="invalid"
        )
        assert result["success"] is True
        assert result["strength"] == "medium"

    def test_embed_truncates_long_payload(self):
        """Payloads longer than 64 bytes should be truncated."""
        long_payload = "A" * 100
        result = perform_watermark_embed(_make_colour_image(), long_payload)
        assert result["success"] is True
        assert result["payload_length"] == 64

    def test_embed_output_is_valid_png(self):
        """The watermarked image should be a valid PNG."""
        import base64

        result = perform_watermark_embed(_make_colour_image(), "Test")
        assert result["success"] is True
        img_bytes = base64.b64decode(result["watermarked_image_base64"])
        # PNG magic bytes
        assert img_bytes[:4] == b"\x89PNG"


class TestWatermarkExtract:
    """Tests for watermark extraction."""

    def test_extract_returns_result(self):
        """Extraction should return a valid result dict."""
        result = perform_watermark_extract(_make_colour_image())
        assert result["success"] is True
        assert result["algorithm"] == "dwtDctSvd"
        assert isinstance(result["has_watermark"], bool)
        assert isinstance(result["confidence"], float)
        assert 0.0 <= result["confidence"] <= 1.0

    def test_extract_invalid_image_returns_error(self):
        """Invalid image bytes should return an error, not raise."""
        result = perform_watermark_extract(b"not an image")
        assert result["success"] is False
        assert "could not decode" in result["message"].lower()


class TestWatermarkRoundtrip:
    """Tests for embed-then-extract round-trip."""

    def test_embed_extract_roundtrip(self):
        """Embedding then extracting should recover the original payload."""
        import base64

        payload = "JuralabsCIC"
        payload_len = len(payload.encode("utf-8"))

        # Embed
        embed_result = perform_watermark_embed(_make_colour_image(), payload)
        assert embed_result["success"] is True

        # Decode the watermarked image back to bytes
        watermarked_bytes = base64.b64decode(
            embed_result["watermarked_image_base64"]
        )

        # Extract
        extract_result = perform_watermark_extract(
            watermarked_bytes, payload_length=payload_len
        )
        assert extract_result["success"] is True
        assert extract_result["has_watermark"] is True
        assert extract_result["extracted_payload"] == payload

    def test_roundtrip_with_short_payload(self):
        """Round-trip should work with a short payload."""
        import base64

        payload = "Hi"
        payload_len = len(payload.encode("utf-8"))

        embed_result = perform_watermark_embed(
            _make_colour_image(), payload
        )
        assert embed_result["success"] is True

        watermarked_bytes = base64.b64decode(
            embed_result["watermarked_image_base64"]
        )

        extract_result = perform_watermark_extract(
            watermarked_bytes, payload_length=payload_len
        )
        assert extract_result["success"] is True
        assert extract_result["has_watermark"] is True
        assert extract_result["extracted_payload"] == payload
