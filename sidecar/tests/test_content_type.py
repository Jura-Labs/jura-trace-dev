# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the content-type heuristic classifier (`app.services.content_type`)."""

from __future__ import annotations

import io
import time

import numpy as np
from PIL import Image
from PIL.ExifTags import Base as ExifBase

from app.services.content_type import classify_content


# ── Test image generators ─────────────────────────────────────────────────


def _make_screenshot(
    size: tuple[int, int] = (1920, 1080),
    bg_colour: tuple[int, int, int] = (30, 30, 40),
) -> bytes:
    """Synthetic screenshot — PNG, no EXIF, device resolution, solid UI blocks."""
    w, h = size
    arr = np.full((h, w, 3), bg_colour, dtype=np.uint8)
    arr[:60, :] = (45, 45, 55)
    arr[60, :] = (80, 80, 100)
    arr[61:, :300] = (50, 55, 65)
    arr[61:, 300] = (80, 80, 100)
    arr[80:400, 320:900] = (40, 42, 52)
    arr[420:700, 320:900] = (40, 42, 52)
    # Some crisp single-pixel "text" rows
    for y in range(100, 380, 20):
        arr[y, 340:540] = (200, 200, 210)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_photo_jpeg_with_exif(size: tuple[int, int] = (4032, 3024)) -> bytes:
    """Realistic JPEG photograph with camera Make/Model EXIF."""
    w, h = size
    rng = np.random.default_rng(42)
    base = np.zeros((h, w, 3), dtype=np.float64)
    for y in range(h // 2):
        t = y / max(h // 2, 1)
        base[y, :] = [100 + 80 * t, 140 + 60 * t, 200 - 30 * t]
    for y in range(h // 2, h):
        t = (y - h // 2) / max(h // 2, 1)
        base[y, :] = [40 + 60 * t, 80 + 40 * t, 30 + 20 * t]
    noise = rng.normal(0, 8, (h, w, 3))
    arr = np.clip(base + noise, 0, 255).astype(np.uint8)
    img = Image.fromarray(arr)
    exif = img.getexif()
    exif[ExifBase.Make] = "Canon"
    exif[ExifBase.Model] = "EOS R5"
    exif[ExifBase.DateTimeOriginal] = "2024:06:15 14:32:01"
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=92, exif=exif.tobytes())
    return buf.getvalue()


def _make_photo_png_no_exif(size: tuple[int, int] = (2400, 1600)) -> bytes:
    """Photograph re-saved as PNG, no EXIF — the conservative case.

    Must NOT be classified as a screenshot. Uses a large non-device
    resolution, natural noise, and >>10 000 unique colours.
    """
    w, h = size
    rng = np.random.default_rng(77)
    base = np.zeros((h, w, 3), dtype=np.float64)
    for y in range(h):
        t = y / h
        base[y, :] = [50 + 120 * t, 70 + 100 * t, 90 + 60 * (1 - t)]
    # Add colourful per-region variation so we get tens of thousands of colours
    region_w = w // 8
    for x0 in range(0, w, region_w):
        tint = rng.integers(-30, 30, 3)
        base[:, x0 : x0 + region_w] += tint
    noise = rng.normal(0, 12, (h, w, 3))
    arr = np.clip(base + noise, 0, 255).astype(np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_low_colour_large_png(size: tuple[int, int] = (1920, 1200)) -> bytes:
    """1 MP+ PNG with <500 unique colours — screenshot-like palette."""
    w, h = size
    arr = np.full((h, w, 3), (32, 34, 44), dtype=np.uint8)
    # Add a handful of distinct panels with unique colours
    palette = [
        (44, 46, 58),
        (56, 58, 72),
        (80, 82, 96),
        (110, 112, 130),
        (200, 200, 210),
        (240, 240, 245),
        (60, 80, 140),
        (140, 60, 80),
    ]
    for i, c in enumerate(palette):
        y0 = (i * h) // len(palette)
        y1 = ((i + 1) * h) // len(palette)
        arr[y0:y1, : w // 2] = c
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_macos_screenshot_png(size: tuple[int, int] = (2880, 1800)) -> bytes:
    """PNG at MacBook Pro Retina resolution with Display P3 ICC profile."""
    w, h = size
    # Keep the canvas small visually but at the logical resolution
    arr = np.full((h, w, 3), (245, 245, 245), dtype=np.uint8)
    arr[:80, :] = (235, 235, 235)  # menu bar
    arr[80, :] = (200, 200, 200)  # 1 px divider
    arr[100:700, 100:2000] = (255, 255, 255)  # content window

    # Minimal Display P3 ICC profile — we embed a well-known short profile
    # via PIL's `icc_profile` info key. A genuine P3 profile is ~500 bytes;
    # for the heuristic only the profile *description* matters, which the
    # service reads via ImageCms. To keep the test hermetic we instead
    # assert the screenshot is still detected via the other signals — so we
    # skip embedding a real P3 profile here (it would require shipping a
    # binary artefact into the test corpus).
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_document_scan(size: tuple[int, int] = (2480, 3508)) -> bytes:
    """A4 aspect ratio, >80% near-white with black text-like strokes."""
    w, h = size
    arr = np.full((h, w, 3), 250, dtype=np.uint8)
    # Add "text" — many thin black horizontal strokes
    for y in range(200, h - 200, 40):
        for x in range(200, w - 200, 30):
            if (x // 30) % 3:
                arr[y : y + 4, x : x + 22] = 15
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=88)
    return buf.getvalue()


def _make_artwork_png(size: tuple[int, int] = (2000, 2000)) -> bytes:
    """Rich-colour PNG with Photoshop CreatorTool-like Software tag."""
    w, h = size
    rng = np.random.default_rng(123)
    arr = np.zeros((h, w, 3), dtype=np.float64)
    for _ in range(30):
        cx, cy = rng.integers(0, w), rng.integers(0, h)
        r = rng.integers(100, 500)
        colour = rng.integers(30, 255, 3).astype(np.float64)
        yy, xx = np.ogrid[:h, :w]
        mask_blend = np.clip(1.0 - np.sqrt((xx - cx) ** 2 + (yy - cy) ** 2) / r, 0, 1)
        for c in range(3):
            arr[:, :, c] += mask_blend * colour[c]
    arr = np.clip(arr, 0, 255).astype(np.uint8)
    img = Image.fromarray(arr)
    # Tag via EXIF Software field
    exif = img.getexif()
    exif[ExifBase.Software] = "Adobe Photoshop 24.0"
    buf = io.BytesIO()
    img.save(buf, format="PNG", exif=exif.tobytes())
    return buf.getvalue()


def _make_unknown_tiny(size: tuple[int, int] = (32, 32)) -> bytes:
    """Tiny blob — ambiguous."""
    arr = np.full((*size, 3), 128, dtype=np.uint8)
    img = Image.fromarray(arr)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


# ── Classification tests ───────────────────────────────────────────────────


class TestPhotograph:
    def test_jpeg_with_camera_exif_is_photograph(self):
        r = classify_content(_make_photo_jpeg_with_exif((1600, 1200)))
        assert r.category == "photograph"
        assert r.ai_detection_suitable is True
        assert r.signals["has_camera_exif"] is True
        assert r.confidence >= 0.7

    def test_photo_png_no_exif_defaults_to_photograph(self):
        """Conservative: PNG photo without EXIF should not be flagged as screenshot."""
        r = classify_content(_make_photo_png_no_exif((2400, 1600)))
        assert r.category == "photograph"
        assert r.ai_detection_suitable is True


class TestScreenshot:
    def test_device_resolution_png_detected(self):
        r = classify_content(_make_screenshot((1920, 1080)))
        assert r.category == "screenshot"
        assert r.ai_detection_suitable is False
        assert r.signals["matches_device_resolution"] is True
        assert r.confidence >= 0.55

    def test_macbook_pro_resolution_detected(self):
        r = classify_content(_make_macos_screenshot_png((2880, 1800)))
        assert r.category == "screenshot"
        assert r.ai_detection_suitable is False

    def test_low_colour_large_png_detected_as_screenshot(self):
        r = classify_content(_make_low_colour_large_png((1920, 1200)))
        assert r.category == "screenshot"
        assert r.signals["unique_colours"] < 1000
        assert r.confidence >= 0.55


class TestDocument:
    def test_a4_mostly_white_detected_as_document(self):
        r = classify_content(_make_document_scan((1240, 1754)))
        assert r.category == "document"
        assert r.ai_detection_suitable is False
        assert r.signals["monochrome_ratio"] > 0.7


class TestArtwork:
    def test_photoshop_software_tag_detected_as_artwork(self):
        r = classify_content(_make_artwork_png((1000, 1000)))
        assert r.category == "artwork"
        # Artwork still runs AI detection (but with artwork-appropriate thresholds)
        assert r.ai_detection_suitable is True
        assert r.signals["art_software_fingerprint"] is True


class TestUnknownAndEdgeCases:
    def test_undecodable_bytes_returns_unknown_ai_suitable(self):
        r = classify_content(b"not an image at all")
        assert r.category == "unknown"
        # Fail-open — let AI detection run rather than suppressing it silently
        assert r.ai_detection_suitable is True

    def test_tiny_ambiguous_image_defaults_to_photograph(self):
        """Low confidence on all categories should fall back to photograph."""
        r = classify_content(_make_unknown_tiny((48, 48)))
        assert r.category == "photograph"
        assert r.ai_detection_suitable is True


class TestPerformance:
    def test_classification_under_500ms_on_4k(self):
        img = _make_photo_png_no_exif((3840, 2160))
        t0 = time.perf_counter()
        r = classify_content(img)
        elapsed_ms = (time.perf_counter() - t0) * 1000
        assert r.category in {
            "photograph",
            "screenshot",
            "document",
            "artwork",
            "unknown",
        }
        assert elapsed_ms < 500, (
            f"Classification took {elapsed_ms:.0f} ms (>500 ms budget)"
        )


class TestResultSchema:
    def test_result_fields_present_and_typed(self):
        r = classify_content(_make_photo_jpeg_with_exif((800, 600)))
        assert r.category in {
            "photograph",
            "screenshot",
            "document",
            "artwork",
            "unknown",
        }
        assert 0.0 <= r.confidence <= 1.0
        assert isinstance(r.ai_detection_suitable, bool)
        assert isinstance(r.signals, dict)
        assert isinstance(r.reasoning, str) and r.reasoning
