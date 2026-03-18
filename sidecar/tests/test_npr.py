"""Tests for the Neighbouring Pixel Relationship (NPR) detection service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.npr import perform_npr_analysis


# ── Image factories ───────────────────────────────────────────────────────────


def _make_solid_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a solid-colour image (zero pixel differences)."""
    img = Image.new("RGB", size, (128, 128, 128))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_noisy_image(size: tuple[int, int] = (256, 256), seed: int = 42) -> bytes:
    """Create an image with random pixel noise (high inter-pixel variation)."""
    rng = np.random.default_rng(seed)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_gradient_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a smooth gradient image."""
    w, h = size
    arr = np.zeros((h, w, 3), dtype=np.uint8)
    for i in range(h):
        arr[i, :, 0] = int(i * 255 / h)
        arr[i, :, 1] = int((h - i) * 255 / h)
        arr[i, :, 2] = 128
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_jpeg_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create an image saved as JPEG."""
    rng = np.random.default_rng(99)
    arr = rng.integers(50, 200, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


# ── Test classes ──────────────────────────────────────────────────────────────


class TestNprSolidImage:
    """NPR analysis on a solid-colour image (degenerate case)."""

    def test_solid_image_produces_result(self):
        """A solid-colour image should return a valid NprResponse."""
        result = perform_npr_analysis(_make_solid_image())
        assert result is not None

    def test_score_bounded(self):
        """Score should be within [0.0, 1.0] for a solid image."""
        result = perform_npr_analysis(_make_solid_image())
        assert 0.0 <= result.score <= 1.0

    def test_response_fields_valid(self):
        """All required fields should be present and correctly typed."""
        result = perform_npr_analysis(_make_solid_image())
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.hv_correlation, float)
        assert isinstance(result.diff_variance_ratio, float)
        assert isinstance(result.hf_energy_ratio, float)
        assert isinstance(result.heatmap_base64, str)
        assert isinstance(result.summary, str)
        assert result.summary != ""

    def test_heatmap_is_non_empty_base64(self):
        """Heatmap field should be a non-empty base64 string."""
        result = perform_npr_analysis(_make_solid_image())
        assert len(result.heatmap_base64) > 0

    def test_hv_correlation_bounded(self):
        """H-V correlation should be in [-1.0, 1.0]."""
        result = perform_npr_analysis(_make_solid_image())
        assert -1.0 <= result.hv_correlation <= 1.0

    def test_hf_energy_ratio_bounded(self):
        """HF energy ratio should be non-negative."""
        result = perform_npr_analysis(_make_solid_image())
        assert result.hf_energy_ratio >= 0.0

    def test_diff_variance_ratio_non_negative(self):
        """Difference variance ratio should be non-negative."""
        result = perform_npr_analysis(_make_solid_image())
        assert result.diff_variance_ratio >= 0.0

    def test_suspicious_is_bool(self):
        """suspicious field should be a boolean."""
        result = perform_npr_analysis(_make_solid_image())
        assert isinstance(result.suspicious, bool)

    def test_suspicious_consistent_with_score(self):
        """suspicious should be True iff score > 0.5."""
        result = perform_npr_analysis(_make_solid_image())
        assert result.suspicious == (result.score > 0.5)


class TestNprNoisyImage:
    """NPR analysis on a noisy image (closer to real photograph)."""

    def test_noisy_image_produces_result(self):
        """A noisy image should return a valid NprResponse."""
        result = perform_npr_analysis(_make_noisy_image())
        assert result is not None

    def test_score_bounded(self):
        """Score should be within [0.0, 1.0] for a noisy image."""
        result = perform_npr_analysis(_make_noisy_image())
        assert 0.0 <= result.score <= 1.0

    def test_response_fields_valid(self):
        """All required fields should be present and correctly typed."""
        result = perform_npr_analysis(_make_noisy_image())
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.hv_correlation, float)
        assert isinstance(result.diff_variance_ratio, float)
        assert isinstance(result.hf_energy_ratio, float)
        assert isinstance(result.heatmap_base64, str)
        assert isinstance(result.summary, str)
        assert result.summary != ""

    def test_noisy_image_higher_hf_than_solid(self):
        """A noisy image should have higher HF energy ratio than a solid image."""
        noisy_result = perform_npr_analysis(_make_noisy_image())
        solid_result = perform_npr_analysis(_make_solid_image())
        # Noisy images have more high-frequency content in their difference maps
        assert noisy_result.hf_energy_ratio >= solid_result.hf_energy_ratio

    def test_different_seeds_produce_different_results(self):
        """Different random images should produce different NPR results."""
        result1 = perform_npr_analysis(_make_noisy_image(seed=1))
        result2 = perform_npr_analysis(_make_noisy_image(seed=999))
        # Scores need not be different, but at least one metric should differ
        metrics_differ = (
            result1.score != result2.score
            or result1.hv_correlation != result2.hv_correlation
            or result1.hf_energy_ratio != result2.hf_energy_ratio
        )
        assert metrics_differ


class TestNprScoreBounded:
    """Score-bounded invariant tests across multiple image types."""

    def test_score_bounded(self):
        """Score should be in [0.0, 1.0] for all test images."""
        for img_fn in (_make_solid_image, _make_noisy_image, _make_gradient_image):
            result = perform_npr_analysis(img_fn())
            assert 0.0 <= result.score <= 1.0, (
                f"{img_fn.__name__}: score {result.score} out of bounds"
            )

    def test_score_bounded_jpeg(self):
        """JPEG input should produce a bounded score."""
        result = perform_npr_analysis(_make_jpeg_image())
        assert 0.0 <= result.score <= 1.0

    def test_small_image_score_bounded(self):
        """Small images should produce a bounded score."""
        result = perform_npr_analysis(_make_noisy_image(size=(32, 32)))
        assert 0.0 <= result.score <= 1.0

    def test_large_image_score_bounded(self):
        """Large images (resized internally) should produce a bounded score."""
        result = perform_npr_analysis(_make_noisy_image(size=(1024, 768)))
        assert 0.0 <= result.score <= 1.0


class TestNprInvalidData:
    """Error handling tests."""

    def test_invalid_data_raises(self):
        """Invalid image bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_npr_analysis(b"not an image")

    def test_empty_bytes_raises(self):
        """Empty bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_npr_analysis(b"")

    def test_truncated_jpeg_raises(self):
        """Truncated JPEG data should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_npr_analysis(b"\xff\xd8\xff\xe0" + b"\x00" * 10)


class TestNprResponseFieldsValid:
    """Comprehensive field-validity tests."""

    def test_response_fields_valid(self):
        """All NprResponse fields should be valid on a noisy image."""
        result = perform_npr_analysis(_make_noisy_image())

        # Type checks
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.hv_correlation, float)
        assert isinstance(result.diff_variance_ratio, float)
        assert isinstance(result.hf_energy_ratio, float)
        assert isinstance(result.heatmap_base64, str)
        assert isinstance(result.summary, str)

        # Range checks
        assert 0.0 <= result.score <= 1.0
        assert -1.0 <= result.hv_correlation <= 1.0
        assert result.diff_variance_ratio >= 0.0
        assert result.hf_energy_ratio >= 0.0
        assert len(result.heatmap_base64) > 0
        assert len(result.summary) > 0

    def test_schema_round_trip(self):
        """NprResponse should serialise and deserialise without loss."""
        from app.models.schemas import NprResponse

        result = perform_npr_analysis(_make_noisy_image())
        dumped = result.model_dump()
        restored = NprResponse(**dumped)

        assert restored.score == result.score
        assert restored.suspicious == result.suspicious
        assert restored.hv_correlation == result.hv_correlation
        assert restored.heatmap_base64 == result.heatmap_base64
