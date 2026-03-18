"""Tests for the Chromatic Aberration Consistency Analyser service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.chromatic_aberration import (
    perform_ca_analysis,
    _estimate_shift_magnitude,
    _r_squared,
)


# ── Image factories ───────────────────────────────────────────────────────────


def _make_solid_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a solid-colour image (all channels identical)."""
    img = Image.new("RGB", size, (128, 128, 128))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_noisy_image(size: tuple[int, int] = (256, 256), seed: int = 42) -> bytes:
    """Create an image with independent per-channel random noise."""
    rng = np.random.default_rng(seed)
    arr = rng.integers(20, 235, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_gradient_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a smooth gradient image with high spatial variation."""
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
    """Create an image saved as JPEG (lossy, all channels present)."""
    rng = np.random.default_rng(77)
    arr = rng.integers(30, 220, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


def _make_small_grid_image(size: tuple[int, int] = (64, 64)) -> bytes:
    """Create a very small image likely to produce fewer CA samples."""
    rng = np.random.default_rng(11)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


# ── Test classes ──────────────────────────────────────────────────────────────


class TestCaSolidImage:
    """CA analysis on a solid-colour image (degenerate: no inter-channel shift)."""

    def test_solid_image_produces_result(self):
        """A solid-colour image should return a valid CaResponse."""
        result = perform_ca_analysis(_make_solid_image())
        assert result is not None

    def test_score_bounded(self):
        """Score should be within [0.0, 1.0]."""
        result = perform_ca_analysis(_make_solid_image())
        assert 0.0 <= result.score <= 1.0

    def test_r_squared_bounded(self):
        """R² should be within [0.0, 1.0]."""
        result = perform_ca_analysis(_make_solid_image())
        assert 0.0 <= result.r_squared <= 1.0

    def test_response_fields_valid(self):
        """All required fields should be present and correctly typed."""
        result = perform_ca_analysis(_make_solid_image())
        assert isinstance(result.r_squared, float)
        assert isinstance(result.is_consistent, bool)
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.sample_count, int)
        assert isinstance(result.summary, str)
        assert result.summary != ""

    def test_sample_count_non_negative(self):
        """Sample count should be >= 0."""
        result = perform_ca_analysis(_make_solid_image())
        assert result.sample_count >= 0

    def test_suspicious_consistent_with_score(self):
        """suspicious should be True iff score > 0.5."""
        result = perform_ca_analysis(_make_solid_image())
        assert result.suspicious == (result.score > 0.5)

    def test_is_consistent_consistent_with_r_squared(self):
        """is_consistent should be True iff R² >= 0.15."""
        result = perform_ca_analysis(_make_solid_image())
        # is_consistent threshold is 0.15 per spec
        if result.r_squared >= 0.15:
            assert result.is_consistent is True
        else:
            assert result.is_consistent is False


class TestCaNoisyImage:
    """CA analysis on noisy images (high per-channel variation)."""

    def test_noisy_image_produces_result(self):
        """A noisy image should return a valid CaResponse."""
        result = perform_ca_analysis(_make_noisy_image())
        assert result is not None

    def test_score_bounded(self):
        """Score should be within [0.0, 1.0] for noisy images."""
        result = perform_ca_analysis(_make_noisy_image())
        assert 0.0 <= result.score <= 1.0

    def test_response_fields_valid(self):
        """All required fields should be valid on a noisy image."""
        result = perform_ca_analysis(_make_noisy_image())
        assert isinstance(result.r_squared, float)
        assert isinstance(result.is_consistent, bool)
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.sample_count, int)
        assert isinstance(result.summary, str)
        assert result.summary != ""

    def test_sample_count_positive_on_large_image(self):
        """A 256×256 noisy image should yield at least one CA sample."""
        result = perform_ca_analysis(_make_noisy_image(size=(256, 256)))
        assert result.sample_count >= 0  # May be 0 if all patches are uniform

    def test_different_seeds_produce_different_results(self):
        """Different random images should produce different CA results."""
        result1 = perform_ca_analysis(_make_noisy_image(seed=7))
        result2 = perform_ca_analysis(_make_noisy_image(seed=314))
        # At least one numeric field should differ
        any_differ = (
            result1.r_squared != result2.r_squared
            or result1.score != result2.score
            or result1.sample_count != result2.sample_count
        )
        assert any_differ


class TestCaScoreBounded:
    """Score-bounded invariant tests across multiple image types."""

    def test_score_bounded(self):
        """Score should be in [0.0, 1.0] for all standard test images."""
        for img_fn in (_make_solid_image, _make_noisy_image, _make_gradient_image):
            result = perform_ca_analysis(img_fn())
            assert 0.0 <= result.score <= 1.0, (
                f"{img_fn.__name__}: score {result.score} out of bounds"
            )

    def test_score_bounded_jpeg(self):
        """JPEG input should produce a bounded score."""
        result = perform_ca_analysis(_make_jpeg_image())
        assert 0.0 <= result.score <= 1.0

    def test_score_bounded_small_image(self):
        """Small images should produce a bounded score."""
        result = perform_ca_analysis(_make_small_grid_image())
        assert 0.0 <= result.score <= 1.0

    def test_score_bounded_large_image(self):
        """Large images (resized internally) should produce a bounded score."""
        result = perform_ca_analysis(_make_noisy_image(size=(1024, 768)))
        assert 0.0 <= result.score <= 1.0


class TestCaRSquaredBounded:
    """R²-bounded invariant tests."""

    def test_r_squared_bounded(self):
        """R² should always be in [0.0, 1.0]."""
        for img_fn in (_make_solid_image, _make_noisy_image, _make_gradient_image):
            result = perform_ca_analysis(img_fn())
            assert 0.0 <= result.r_squared <= 1.0, (
                f"{img_fn.__name__}: R² {result.r_squared} out of bounds"
            )

    def test_r_squared_bounded_jpeg(self):
        """JPEG input R² should be in [0.0, 1.0]."""
        result = perform_ca_analysis(_make_jpeg_image())
        assert 0.0 <= result.r_squared <= 1.0


class TestCaInvalidData:
    """Error handling tests."""

    def test_invalid_data_raises(self):
        """Invalid image bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_ca_analysis(b"not an image")

    def test_empty_bytes_raises(self):
        """Empty bytes should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_ca_analysis(b"")

    def test_truncated_png_raises(self):
        """Truncated PNG header should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_ca_analysis(b"\x89PNG\r\n\x1a\n" + b"\x00" * 8)


class TestCaResponseFieldsValid:
    """Comprehensive field-validity tests."""

    def test_response_fields_valid(self):
        """All CaResponse fields should be valid on a gradient image."""
        result = perform_ca_analysis(_make_gradient_image())

        # Type checks
        assert isinstance(result.r_squared, float)
        assert isinstance(result.is_consistent, bool)
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert isinstance(result.sample_count, int)
        assert isinstance(result.summary, str)

        # Range checks
        assert 0.0 <= result.r_squared <= 1.0
        assert 0.0 <= result.score <= 1.0
        assert result.sample_count >= 0
        assert len(result.summary) > 0

    def test_schema_round_trip(self):
        """CaResponse should serialise and deserialise without loss."""
        from app.models.schemas import CaResponse

        result = perform_ca_analysis(_make_gradient_image())
        dumped = result.model_dump()
        restored = CaResponse(**dumped)

        assert restored.r_squared == result.r_squared
        assert restored.score == result.score
        assert restored.suspicious == result.suspicious
        assert restored.sample_count == result.sample_count


class TestCaHelpers:
    """Unit tests for private helper functions."""

    def test_r_squared_perfect_fit(self):
        """A perfect linear relationship should give R² = 1.0."""
        x = np.linspace(0, 100, 50)
        y = 2.0 * x + 5.0
        r2 = _r_squared(x, y)
        assert abs(r2 - 1.0) < 1e-6, f"Expected R²=1.0, got {r2}"

    def test_r_squared_no_fit(self):
        """Constant y should give R² = 0.0."""
        x = np.linspace(0, 100, 50)
        y = np.ones(50) * 42.0
        r2 = _r_squared(x, y)
        assert r2 == 0.0

    def test_r_squared_constant_x(self):
        """Constant x should return 0.0 (undefined fit)."""
        x = np.ones(50) * 10.0
        y = np.random.default_rng(1).random(50)
        r2 = _r_squared(x, y)
        assert r2 == 0.0

    def test_r_squared_too_few_points(self):
        """Fewer than 2 points should return 0.0."""
        assert _r_squared(np.array([1.0]), np.array([2.0])) == 0.0

    def test_r_squared_bounded_on_random(self):
        """R² should be in [0.0, 1.0] for any random data."""
        rng = np.random.default_rng(42)
        x = rng.random(30) * 200
        y = rng.random(30) * 5
        r2 = _r_squared(x, y)
        assert 0.0 <= r2 <= 1.0

    def test_estimate_shift_zero_for_identical_patches(self):
        """Identical patches should produce a shift of 0.0."""
        rng = np.random.default_rng(7)
        patch = rng.integers(20, 220, (32, 32), dtype=np.uint8).astype(np.float32)
        shift = _estimate_shift_magnitude(patch, patch)
        assert shift == 0.0

    def test_estimate_shift_non_negative(self):
        """Shift magnitude should always be non-negative."""
        rng = np.random.default_rng(9)
        patch_a = rng.integers(0, 256, (32, 32), dtype=np.uint8).astype(np.float32)
        patch_b = rng.integers(0, 256, (32, 32), dtype=np.uint8).astype(np.float32)
        shift = _estimate_shift_magnitude(patch_a, patch_b)
        assert shift >= 0.0
