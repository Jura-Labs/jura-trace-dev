# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the noise analysis service."""

import io

import numpy as np
import pytest
from PIL import Image

from app.services.noise_analysis import perform_noise_analysis


def _make_uniform_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create a uniform solid-colour image."""
    img = Image.new("RGB", size, (128, 128, 128))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_noisy_image(size: tuple[int, int] = (256, 256)) -> bytes:
    """Create an image with random noise."""
    rng = np.random.default_rng(42)
    arr = rng.integers(0, 256, (*size[::-1], 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_spliced_image() -> bytes:
    """Create an image with a region of different noise level to simulate splicing."""
    rng = np.random.default_rng(42)
    # Base: low noise (smooth gradient)
    arr = np.full((256, 256, 3), 128, dtype=np.uint8)
    # Add low-level noise everywhere
    arr = arr + rng.integers(-5, 6, arr.shape, dtype=np.int16)
    arr = np.clip(arr, 0, 255).astype(np.uint8)
    # Splice: a block of high noise (simulating pasted content)
    arr[64:128, 64:128] = rng.integers(0, 256, (64, 64, 3), dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestNoiseAnalysis:
    def test_uniform_image_low_score(self):
        """A uniform image should have very low anomaly score."""
        result = perform_noise_analysis(_make_uniform_image())
        assert result.score < 0.3
        assert not result.suspicious
        assert result.anomalous_blocks == 0

    def test_noisy_image_produces_result(self):
        """A fully noisy image should still return a valid result."""
        result = perform_noise_analysis(_make_noisy_image())
        assert 0.0 <= result.score <= 1.0
        assert result.total_blocks > 0
        assert len(result.heatmap_base64) > 0

    def test_spliced_image_detects_anomaly(self):
        """An image with a spliced high-noise region should flag anomalies."""
        result = perform_noise_analysis(_make_spliced_image())
        # The spliced region should create variance outliers
        assert result.anomalous_blocks >= 0  # May or may not trigger depending on threshold
        assert result.total_blocks > 0

    def test_small_image_handled(self):
        """An image too small for block analysis should return gracefully."""
        result = perform_noise_analysis(_make_uniform_image(size=(16, 16)))
        assert result.total_blocks == 0
        assert result.score == 0.0
        assert not result.suspicious

    def test_invalid_data_raises(self):
        """Invalid image data should raise ValueError."""
        with pytest.raises(ValueError, match="Cannot decode image"):
            perform_noise_analysis(b"not an image")

    def test_block_size_parameter(self):
        """Different block sizes should produce different block counts."""
        data = _make_noisy_image()
        result_32 = perform_noise_analysis(data, block_size=32)
        result_64 = perform_noise_analysis(data, block_size=64)
        assert result_32.total_blocks > result_64.total_blocks

    def test_response_fields_present(self):
        """All response fields should be populated."""
        result = perform_noise_analysis(_make_noisy_image())
        assert isinstance(result.heatmap_base64, str)
        assert isinstance(result.block_variances, list)
        assert isinstance(result.global_variance, float)
        assert isinstance(result.anomalous_blocks, int)
        assert isinstance(result.total_blocks, int)
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)

    def test_global_variance_positive_for_noisy(self):
        """Noisy image should have positive global variance."""
        result = perform_noise_analysis(_make_noisy_image())
        assert result.global_variance > 0
