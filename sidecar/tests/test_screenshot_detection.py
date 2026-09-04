# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the screenshot pre-classifier in the deepfake detection pipeline."""

import io

import numpy as np
from PIL import Image

from app.services.deepfake import (
    is_likely_screenshot,
    perform_deepfake_detection,
)


# ── Test image generators ─────────────────────────────────────────────────


def _make_screenshot(
    size: tuple[int, int] = (1920, 1080),
    bg_colour: tuple[int, int, int] = (30, 30, 40),
    panel_colour: tuple[int, int, int] = (50, 55, 65),
) -> bytes:
    """Create a synthetic screenshot-like image with UI elements.

    Features: PNG format, no EXIF, solid-colour blocks, sharp 1px borders,
    common screen resolution, limited colour palette.
    """
    w, h = size
    arr = np.full((h, w, 3), bg_colour, dtype=np.uint8)

    # Add a "toolbar" — solid colour bar at the top
    arr[:60, :] = (45, 45, 55)
    # Sharp 1px border under toolbar
    arr[60, :] = (80, 80, 100)

    # Add a "sidebar" panel
    arr[61:, :300] = panel_colour
    # Sharp 1px border on right edge of sidebar
    arr[61:, 300] = (80, 80, 100)

    # Add some "content" panels with solid backgrounds
    arr[80:400, 320:900] = (40, 42, 52)
    arr[420:700, 320:900] = (40, 42, 52)

    # Add a few "text" lines (alternating bright/dark rows)
    for y in range(100, 380, 20):
        arr[y, 340:340 + np.random.randint(200, 500)] = (200, 200, 210)

    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_photo_jpeg(size: tuple[int, int] = (4032, 3024)) -> bytes:
    """Create a JPEG image with EXIF data simulating a real photograph.

    Features: JPEG format, camera EXIF, natural noise, varied colours,
    non-standard resolution.
    """
    w, h = size
    rng = np.random.default_rng(42)
    # Create a scene with gradients and natural noise
    base = np.zeros((h, w, 3), dtype=np.float64)
    # Sky gradient (top half)
    for y in range(h // 2):
        t = y / (h // 2)
        base[y, :] = [100 + 80 * t, 140 + 60 * t, 200 - 30 * t]
    # Ground (bottom half)
    for y in range(h // 2, h):
        t = (y - h // 2) / (h // 2)
        base[y, :] = [40 + 60 * t, 80 + 40 * t, 30 + 20 * t]
    # Add natural noise (sensor noise)
    noise = rng.normal(0, 8, (h, w, 3))
    arr = np.clip(base + noise, 0, 255).astype(np.uint8)

    img = Image.fromarray(arr)
    # Add minimal EXIF data
    from PIL.ExifTags import Base as ExifBase
    exif = img.getexif()
    exif[ExifBase.Make] = "Canon"
    exif[ExifBase.Model] = "EOS R5"

    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=92, exif=exif.tobytes())
    return buf.getvalue()


def _make_ai_generated_png(size: tuple[int, int] = (1024, 1024)) -> bytes:
    """Create a PNG simulating AI-generated image characteristics.

    Features: PNG format, no EXIF, but with varied colours, textures,
    and noise patterns typical of diffusion models — NOT flat solid blocks.
    """
    w, h = size
    rng = np.random.default_rng(99)
    # Complex scene with many colours and smooth gradients (like a painting)
    arr = np.zeros((h, w, 3), dtype=np.float64)
    # Overlapping colour blobs (simulating a complex AI scene)
    for _ in range(20):
        cx, cy = rng.integers(0, w), rng.integers(0, h)
        r = rng.integers(50, 300)
        colour = rng.integers(30, 255, 3).astype(np.float64)
        yy, xx = np.ogrid[:h, :w]
        dist = np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2)
        blend = np.clip(1.0 - dist / r, 0, 1)
        for c in range(3):
            arr[:, :, c] += blend * colour[c]
    arr = np.clip(arr, 0, 255)
    # Add subtle AI-like noise (very smooth, correlated across channels)
    noise = rng.normal(0, 3, (h, w, 1))
    noise = np.repeat(noise, 3, axis=2)  # Correlated noise (AI-like)
    arr = np.clip(arr + noise, 0, 255).astype(np.uint8)

    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_photo_saved_as_png(size: tuple[int, int] = (2400, 1600)) -> bytes:
    """Create a PNG of a photograph (no EXIF, but natural noise and textures).

    This is the critical borderline case: a photo exported as PNG (e.g., from
    Photoshop, GIMP, or a web download). It should NOT be classified as a
    screenshot because it has natural noise, varied colour palette, and no
    solid-colour UI blocks.
    """
    w, h = size
    rng = np.random.default_rng(77)
    # Natural-looking scene
    base = np.zeros((h, w, 3), dtype=np.float64)
    for y in range(h):
        for x_start in range(0, w, w // 5):
            x_end = min(x_start + w // 5, w)
            region_colour = rng.integers(20, 230, 3).astype(np.float64)
            base[y, x_start:x_end] = region_colour + rng.normal(0, 15, 3)
    # Heavy natural noise (camera-like)
    noise = rng.normal(0, 12, (h, w, 3))
    arr = np.clip(base + noise, 0, 255).astype(np.uint8)

    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


# ── Unit tests for is_likely_screenshot ───────────────────────────────────


class TestScreenshotDetection:
    """Test the screenshot pre-classifier function directly."""

    def test_screenshot_detected(self):
        """A generated screenshot image should be detected as a screenshot."""
        is_ss, confidence, signals = is_likely_screenshot(_make_screenshot())
        assert is_ss is True
        assert confidence > 0.70
        assert signals["png_format"] == 1.0
        assert signals["no_exif"] == 1.0
        assert signals["low_noise"] == 1.0
        assert signals["solid_regions"] > 0.0

    def test_photograph_not_screenshot(self):
        """A JPEG photo with EXIF should not be detected as a screenshot."""
        is_ss, confidence, signals = is_likely_screenshot(_make_photo_jpeg(size=(800, 600)))
        assert is_ss is False
        assert confidence < 0.40
        assert signals["png_format"] == 0.0
        assert signals["no_exif"] == 0.0

    def test_ai_image_not_screenshot(self):
        """An AI-generated PNG should not be classified as a screenshot.

        AI images share some screenshot signals (PNG, no EXIF) but differ
        in noise patterns, colour variety, and lack of solid-colour blocks.
        """
        is_ss, confidence, signals = is_likely_screenshot(_make_ai_generated_png())
        # Should not be confidently classified as a screenshot
        assert confidence < 0.70, (
            f"AI-generated image scored {confidence} — too close to screenshot threshold. "
            f"Signals: {signals}"
        )

    def test_screenshot_confidence_threshold(self):
        """Clear screenshots should have confidence > 0.70."""
        # Full HD screenshot
        is_ss, confidence, _ = is_likely_screenshot(_make_screenshot((1920, 1080)))
        assert confidence > 0.70

        # MacBook Retina screenshot
        is_ss, confidence, _ = is_likely_screenshot(_make_screenshot((2880, 1800)))
        assert confidence > 0.70

    def test_borderline_png_photo_not_screenshot(self):
        """A PNG-saved photo (no EXIF, natural noise) should NOT be a screenshot.

        This is the most important false positive case to avoid: photographs
        that happen to be saved as PNG (downloaded from web, exported from
        an editor) must not be gated away from the deepfake ensemble.
        """
        is_ss, confidence, signals = is_likely_screenshot(
            _make_photo_saved_as_png(size=(800, 600))
        )
        assert is_ss is False, (
            f"PNG photo misclassified as screenshot (confidence={confidence}). "
            f"Signals: {signals}"
        )

    def test_small_image_handled(self):
        """A small image should not crash the screenshot detector."""
        img = Image.new("RGB", (16, 16), (128, 128, 128))
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        is_ss, confidence, signals = is_likely_screenshot(buf.getvalue())
        # Should not crash; result is not critical
        assert isinstance(is_ss, bool)
        assert 0.0 <= confidence <= 1.0

    def test_invalid_bytes_returns_false(self):
        """Invalid image bytes should return False, not raise."""
        is_ss, confidence, signals = is_likely_screenshot(b"not an image")
        assert is_ss is False
        assert confidence == 0.0
        assert signals == {}

    def test_signals_dict_has_expected_keys(self):
        """The signal dict should contain all expected keys."""
        _, _, signals = is_likely_screenshot(_make_screenshot())
        expected_keys = {
            "png_format", "no_exif", "low_noise", "solid_regions",
            "screen_resolution", "sharp_edges", "limited_palette",
        }
        assert set(signals.keys()) == expected_keys

    def test_screen_resolution_signal_fires_for_1080p(self):
        """An image at exactly 1920x1080 should trigger the resolution signal."""
        _, _, signals = is_likely_screenshot(_make_screenshot((1920, 1080)))
        assert signals["screen_resolution"] == 1.0

    def test_screen_resolution_signal_off_for_odd_size(self):
        """An image at a non-standard resolution should not trigger."""
        _, _, signals = is_likely_screenshot(_make_screenshot((1234, 567)))
        assert signals["screen_resolution"] == 0.0


# ── Integration tests: screenshot bypass in the pipeline ──────────────────


class TestScreenshotBypassIntegration:
    """Test that the screenshot pre-classifier correctly gates the pipeline."""

    def test_screenshot_bypasses_ensemble(self):
        """A clear screenshot should bypass the ensemble and get a low score.

        The hardcoded bypass score is 0.15 (raised from 0.05 in commit
        ead664a to prevent AI images resembling screenshots from receiving
        "high confidence authentic" verdicts). The assertion upper bound
        is 0.20 to allow for any future minor tuning without breaking
        this test — the semantic guarantee is score, verdict_level,
        suspicious, and summary, not the exact numeric value.
        """
        result = perform_deepfake_detection(
            _make_screenshot(), mime_type="image/png"
        )
        assert result.score <= 0.20
        assert result.verdict_level == "authentic"
        assert result.suspicious is False
        assert "screenshot" in result.summary.lower()

    def test_photo_jpeg_runs_full_ensemble(self):
        """A JPEG photo should run the full ensemble (not be gated)."""
        result = perform_deepfake_detection(
            _make_photo_jpeg(size=(800, 600)),
            mime_type="image/jpeg",
            has_camera_exif=True,
        )
        # Should have multiple signals (the full ensemble), not just 1
        assert len(result.signals) > 1
        # Should not mention screenshot
        assert "screenshot" not in result.summary.lower()

    def test_ai_png_runs_full_ensemble(self):
        """An AI-generated PNG should run the full ensemble, not be gated."""
        result = perform_deepfake_detection(
            _make_ai_generated_png(), mime_type="image/png"
        )
        # Should have the full signal set, not just the screenshot signal
        assert len(result.signals) > 1

    def test_screenshot_result_has_valid_structure(self):
        """The screenshot bypass result should have all required fields."""
        result = perform_deepfake_detection(
            _make_screenshot(), mime_type="image/png"
        )
        assert isinstance(result.score, float)
        assert isinstance(result.suspicious, bool)
        assert result.confidence in ("high", "medium", "low")
        assert result.verdict_level in ("authentic", "inconclusive", "synthetic")
        assert isinstance(result.signals, list)
        assert len(result.signals) >= 1
        assert isinstance(result.heatmap_base64, str)
        assert len(result.heatmap_base64) > 0
        assert isinstance(result.summary, str)
        assert isinstance(result.watermarks, list)
