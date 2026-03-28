"""Tests for the diffusion artefact detection service."""

import base64
import io

import numpy as np
import pytest
from PIL import Image

from app.services.diffusion_artefacts import detect_diffusion_artefacts


def _make_test_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a simple test image."""
    rng = np.random.default_rng(42)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_smooth_image(size: tuple[int, int] = (512, 512)) -> bytes:
    """Create a very smooth image (low texture variance)."""
    arr = np.full((*size[::-1], 3), 128, dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestDiffusionArtefacts:
    def test_output_has_expected_fields(self):
        """Result should contain all 6 documented fields."""
        result = detect_diffusion_artefacts(_make_test_image())
        expected_keys = {
            "texture_smoothness_score",
            "texture_smoothness_map_base64",
            "vae_banding_score",
            "resolution_match",
            "resolution_note",
            "overall_diffusion_score",
        }
        assert expected_keys == set(result.keys())

    def test_texture_smoothness_score_range(self):
        """texture_smoothness_score should be between 0 and 1."""
        result = detect_diffusion_artefacts(_make_test_image())
        assert 0.0 <= result["texture_smoothness_score"] <= 1.0

    def test_vae_banding_score_range(self):
        """vae_banding_score should be between 0 and 1."""
        result = detect_diffusion_artefacts(_make_test_image())
        assert 0.0 <= result["vae_banding_score"] <= 1.0

    def test_overall_diffusion_score_range(self):
        """overall_diffusion_score should be between 0 and 1."""
        result = detect_diffusion_artefacts(_make_test_image())
        assert 0.0 <= result["overall_diffusion_score"] <= 1.0

    def test_resolution_match_is_bool(self):
        """resolution_match should be a boolean."""
        result = detect_diffusion_artefacts(_make_test_image())
        assert isinstance(result["resolution_match"], bool)

    def test_smoothness_map_is_valid_png(self):
        """The smoothness heatmap should be a decodable base64 PNG."""
        result = detect_diffusion_artefacts(_make_test_image())
        raw = base64.b64decode(result["texture_smoothness_map_base64"])
        img = Image.open(io.BytesIO(raw))
        assert img.format == "PNG"

    def test_invalid_input_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError):
            detect_diffusion_artefacts(b"not an image")
