# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Content-type heuristic classifier.

Purpose
-------
Authentic screenshots and scanned documents share many signal-level features
with AI-generated images (no camera EXIF, clean re-encoded output, no
demosaicing noise, smooth UI gradients). On a 2026-03-28 audit of the
pilot corpus, UnivFD v9 produced a 100% false-positive rate on screenshots
and the GBM deepfake classifier produced 65% FPs. This module classifies
an image into one of five content-type buckets so the verify pipeline can
suppress the AI-detection stage for bucket types where it is not reliable.

Design principles
-----------------
- **Fast**: pure heuristics, no ML model load, <500 ms target on 4K images.
- **Conservative**: when confidence is low, return `photograph` so the AI
  detection stage runs. A false negative here (photo misclassified as
  screenshot) would suppress legitimate deepfake detection — much worse
  than a false positive (screenshot classified as photograph) which just
  means the user sees the existing ensemble output.
- **Stateless**: no model files, no corpus. Safe for PyInstaller bundling.

Heuristics
----------
1. Hard screenshot signals: absent camera EXIF + matching device resolution
   + PNG container + screenshot-specific ICC profile ("Display P3",
   "sRGB IEC61966-2.1" paired with zero EXIF) + known software fingerprint.
2. Soft signals: axis-aligned 1-pixel Canny edges, <1000 unique colours in
   a 1 MP+ image, large solid-colour regions, anomalously low Laplacian
   noise despite sharp edges.
3. Document signals: >80% near-black/near-white pixels, A4/Letter aspect
   ratios, high text-like edge regularity.
4. Artwork signals: rich XMP creator tool fingerprint (Photoshop, Procreate,
   Krita, Affinity), high saturation variance, no camera EXIF.
"""

from __future__ import annotations

import io
from typing import Any

import cv2  # type: ignore[import-untyped]
import numpy as np
from PIL import Image, ImageCms  # type: ignore[import-untyped]

from app.models.content_type import ContentTypeResult


# ── Known device/screen resolutions ────────────────────────────────────────
# Populated from public device spec sheets. We match both orientations
# (portrait/landscape) by checking (w, h) and (h, w) against the set.
_SCREEN_RESOLUTIONS: set[tuple[int, int]] = {
    # Desktop / laptop monitors
    (1280, 720), (1366, 768), (1440, 900), (1536, 864), (1600, 900),
    (1680, 1050), (1920, 1080), (1920, 1200), (2048, 1152), (2048, 1280),
    (2304, 1440), (2560, 1440), (2560, 1600), (2880, 1800), (3024, 1964),
    (3072, 1920), (3200, 1800), (3440, 1440), (3456, 2234), (3840, 2160),
    (3840, 2400), (5120, 2160), (5120, 2880), (6016, 3384), (6144, 3456),
    # iPhone logical and native
    (750, 1334), (828, 1792), (1080, 1920), (1125, 2436), (1170, 2532),
    (1179, 2556), (1242, 2208), (1242, 2688), (1284, 2778), (1290, 2796),
    (1320, 2868),
    # Android flagships
    (1080, 2340), (1080, 2400), (1440, 2560), (1440, 2960), (1440, 3088),
    (1440, 3120), (1440, 3200), (1440, 3216), (1080, 2280),
    # iPad
    (1620, 2160), (1640, 2360), (1668, 2224), (1668, 2388), (2048, 2732),
    # Common cropped/half sizes
    (640, 1136), (375, 812), (390, 844), (393, 852), (430, 932),
}


# Image-editing / digital-art software strings that appear in EXIF Software
# or XMP CreatorTool fields. Their presence — combined with no camera Make/
# Model — is a strong artwork signal.
_ART_SOFTWARE_FINGERPRINTS: tuple[str, ...] = (
    "photoshop", "procreate", "krita", "affinity photo", "affinity designer",
    "gimp", "clip studio", "illustrator", "corel painter", "sketchbook",
    "paint tool sai", "medibang", "autodesk sketchbook", "figma", "sketch",
)


# Screenshot-tool software fingerprints. Seen in PNG tEXt / iTXt chunks or
# EXIF Software field.
_SCREENSHOT_SOFTWARE_FINGERPRINTS: tuple[str, ...] = (
    "screenshot", "snipping tool", "snipaste", "shottr", "cleanshot",
    "lightshot", "greenshot", "sharex", "gyazo",
)


def _decode_image(image_bytes: bytes) -> Image.Image | None:
    """Decode bytes → PIL Image, returning None on failure."""
    try:
        img = Image.open(io.BytesIO(image_bytes))
        img.load()
        return img
    except Exception:
        return None


def _extract_exif_fields(img: Image.Image) -> dict[str, str]:
    """Pull a handful of EXIF fields as lower-case strings."""
    out: dict[str, str] = {}
    try:
        exif = img.getexif()
        if not exif:
            return out
        # Tag IDs
        MAKE = 0x010F
        MODEL = 0x0110
        SOFTWARE = 0x0131
        DATETIME_ORIGINAL = 0x9003
        USER_COMMENT = 0x9286
        for tag_id, key in (
            (MAKE, "make"),
            (MODEL, "model"),
            (SOFTWARE, "software"),
            (DATETIME_ORIGINAL, "datetime_original"),
            (USER_COMMENT, "user_comment"),
        ):
            val = exif.get(tag_id)
            if val is None:
                continue
            if isinstance(val, bytes):
                try:
                    val = val.decode("utf-8", errors="ignore")
                except Exception:
                    val = ""
            out[key] = str(val).strip().lower()
    except Exception:
        return {}
    return out


def _icc_profile_name(img: Image.Image) -> str:
    """Best-effort ICC profile description — lower-cased or empty."""
    try:
        icc_bytes = img.info.get("icc_profile")
        if not icc_bytes:
            return ""
        profile = ImageCms.ImageCmsProfile(io.BytesIO(icc_bytes))
        desc = ImageCms.getProfileDescription(profile) or ""
        return desc.strip().lower()
    except Exception:
        return ""


def _png_text_chunks(img: Image.Image) -> dict[str, str]:
    """Pull PNG tEXt/iTXt metadata (lowercased keys + values)."""
    out: dict[str, str] = {}
    try:
        if img.format != "PNG":
            return out
        for k, v in (img.info or {}).items():
            if isinstance(v, (str, bytes)):
                val = v.decode("utf-8", errors="ignore") if isinstance(v, bytes) else v
                out[str(k).lower()] = val.strip().lower()
    except Exception:
        return {}
    return out


def _matches_device_resolution(size: tuple[int, int]) -> bool:
    w, h = size
    return (w, h) in _SCREEN_RESOLUTIONS or (h, w) in _SCREEN_RESOLUTIONS


def _count_unique_colours(arr: np.ndarray, cap: int = 20_000) -> int:
    """Count unique RGB colours in the image, early-exiting above `cap`.

    Down-samples to at most 512×512 first for speed — that preserves the
    screenshot vs photograph distinction (photos have >>20 000 unique
    colours even at 512 resolution; UI renders typically <1 000).
    """
    if arr.ndim != 3:
        return 0
    h, w = arr.shape[:2]
    if max(h, w) > 512:
        scale = 512 / max(h, w)
        new_w, new_h = max(int(w * scale), 1), max(int(h * scale), 1)
        arr = cv2.resize(arr, (new_w, new_h), interpolation=cv2.INTER_AREA)
    flat = arr.reshape(-1, arr.shape[2])
    # Pack RGB into a single uint32 for fast unique()
    packed = (
        flat[:, 0].astype(np.uint32) << 16
        | flat[:, 1].astype(np.uint32) << 8
        | flat[:, 2].astype(np.uint32)
    )
    unique = np.unique(packed)
    return int(min(unique.size, cap))


def _axis_aligned_edge_ratio(grey: np.ndarray) -> float:
    """Fraction of Canny edge pixels that lie on axis-aligned 1-pixel runs.

    Screenshots render UI with perfectly axis-aligned 1-pixel lines (window
    borders, dividers, text baselines). Photographs, because of lens blur
    and sensor demosaicing, do not — their edges are gradient ramps wider
    than one pixel and rarely lie on a single row/column.
    """
    if grey.size == 0:
        return 0.0
    g8 = grey.astype(np.uint8) if grey.dtype != np.uint8 else grey
    edges = cv2.Canny(g8, 80, 200)
    total = int(edges.sum() // 255)
    if total < 50:
        return 0.0
    # A pixel is "axis-aligned 1-pixel" if it has ≥3 edge neighbours along
    # a single row or column direction but few along the diagonals.
    horiz = cv2.filter2D(edges, -1, np.array([[1, 1, 1]], dtype=np.int16) // 1)
    vert = cv2.filter2D(edges, -1, np.array([[1], [1], [1]], dtype=np.int16) // 1)
    aligned_mask = ((horiz >= 3 * 255) | (vert >= 3 * 255)) & (edges > 0)
    aligned = int(aligned_mask.sum() // 255) if aligned_mask.sum() > 0 else 0
    return aligned / max(total, 1)


def _monochrome_ratio(grey: np.ndarray) -> float:
    """Fraction of pixels that are near-black (<20) or near-white (>235)."""
    if grey.size == 0:
        return 0.0
    dark = grey < 20
    light = grey > 235
    return float((dark | light).sum()) / grey.size


def _aspect_matches_paper(size: tuple[int, int], tol: float = 0.03) -> bool:
    """True if aspect ratio is close to A4 (1.414) or US Letter (1.294)."""
    w, h = size
    if w == 0 or h == 0:
        return False
    ratio = max(w, h) / min(w, h)
    return abs(ratio - 1.414) < tol or abs(ratio - 1.294) < tol


def _saturation_variance(arr: np.ndarray) -> float:
    """Variance of HSV saturation channel — high for rich digital art."""
    if arr.ndim != 3:
        return 0.0
    hsv = cv2.cvtColor(arr, cv2.COLOR_RGB2HSV)
    return float(np.var(hsv[:, :, 1]))


# ── Main classification ────────────────────────────────────────────────────
def classify_content(image_bytes: bytes) -> ContentTypeResult:
    """Classify an image's content type via fast heuristics.

    See module docstring for design intent and signal catalogue.

    Args:
        image_bytes: raw bytes of a PNG/JPEG/HEIF/TIFF/WebP image.

    Returns:
        ContentTypeResult with category, confidence, ai_detection_suitable
        flag, signal dict, and a one-line human-readable reasoning string.

        On decode failure returns `unknown` with ai_detection_suitable=True
        (fail-open — let the downstream pipeline decide).
    """
    img = _decode_image(image_bytes)
    if img is None:
        return ContentTypeResult(
            category="unknown",
            confidence=0.0,
            ai_detection_suitable=True,
            signals={"decode_failed": True},
            reasoning="Image could not be decoded; defaulting to photograph pipeline.",
        )

    # ── Metadata ────────────────────────────────────────────────────────
    exif = _extract_exif_fields(img)
    icc = _icc_profile_name(img)
    png_text = _png_text_chunks(img)
    fmt = (img.format or "").upper()
    size = img.size
    w, h = size
    megapixels = (w * h) / 1_000_000.0

    has_camera_make_model = bool(exif.get("make") and exif.get("model"))
    has_datetime_original = bool(exif.get("datetime_original"))
    software = exif.get("software", "") + " " + png_text.get("software", "")
    software = software.strip()

    matches_device_res = _matches_device_resolution(size)

    is_png = fmt == "PNG"
    is_jpeg = fmt in ("JPEG", "JPG")

    art_software = any(fp in software for fp in _ART_SOFTWARE_FINGERPRINTS)
    screenshot_software = any(
        fp in software for fp in _SCREENSHOT_SOFTWARE_FINGERPRINTS
    )

    # ── Pixel analysis ──────────────────────────────────────────────────
    rgb = img.convert("RGB")
    arr = np.asarray(rgb)
    grey = cv2.cvtColor(arr, cv2.COLOR_RGB2GRAY)

    # Unique colour count (expensive — capped at 20k, downsampled)
    unique_colours = _count_unique_colours(arr)

    axis_edge_ratio = _axis_aligned_edge_ratio(grey)
    mono_ratio = _monochrome_ratio(grey)
    paper_aspect = _aspect_matches_paper(size)
    sat_var = _saturation_variance(arr)

    signals: dict[str, Any] = {
        "format": fmt,
        "size": [w, h],
        "megapixels": round(megapixels, 2),
        "has_camera_exif": has_camera_make_model,
        "has_datetime_original": has_datetime_original,
        "exif_make": exif.get("make", ""),
        "exif_model": exif.get("model", ""),
        "exif_software": exif.get("software", ""),
        "icc_profile": icc,
        "matches_device_resolution": matches_device_res,
        "art_software_fingerprint": art_software,
        "screenshot_software_fingerprint": screenshot_software,
        "unique_colours": unique_colours,
        "axis_aligned_edge_ratio": round(axis_edge_ratio, 3),
        "monochrome_ratio": round(mono_ratio, 3),
        "paper_aspect_ratio": paper_aspect,
        "saturation_variance": round(sat_var, 1),
    }

    # ── Scoring ─────────────────────────────────────────────────────────
    # We compute independent scores per candidate category, then pick the
    # highest — with the conservative rule that the winning score must
    # exceed 0.55 (otherwise we fall back to `photograph`).

    # Screenshot score
    ss_score = 0.0
    if not has_camera_make_model:
        ss_score += 0.20
    if matches_device_res:
        ss_score += 0.35
    if is_png:
        ss_score += 0.10
    if screenshot_software:
        ss_score += 0.40
    if megapixels >= 0.5 and unique_colours < 1000:
        ss_score += 0.25
    elif megapixels >= 0.5 and unique_colours < 5000:
        ss_score += 0.10
    if axis_edge_ratio > 0.15:
        ss_score += 0.15
    if icc in ("display p3", "srgb iec61966-2.1") and not has_camera_make_model and is_png:
        ss_score += 0.10
    ss_score = min(ss_score, 1.0)

    # Document score
    doc_score = 0.0
    if mono_ratio > 0.80:
        doc_score += 0.50
    elif mono_ratio > 0.60:
        doc_score += 0.25
    if paper_aspect:
        doc_score += 0.20
    if axis_edge_ratio > 0.20 and mono_ratio > 0.40:
        doc_score += 0.20
    if not has_camera_make_model and mono_ratio > 0.50:
        doc_score += 0.10
    doc_score = min(doc_score, 1.0)

    # Artwork score
    art_score = 0.0
    if art_software:
        art_score += 0.55
    if not has_camera_make_model and sat_var > 4000 and megapixels > 0.3:
        art_score += 0.15
    if is_png and not has_camera_make_model and unique_colours > 10_000 and axis_edge_ratio < 0.05:
        art_score += 0.10
    art_score = min(art_score, 1.0)

    # Photograph score — conservative default. Camera EXIF is near-definitive.
    photo_score = 0.0
    if has_camera_make_model:
        photo_score += 0.70
    if has_datetime_original:
        photo_score += 0.10
    if is_jpeg and not is_png:
        photo_score += 0.10
    if unique_colours >= 10_000 and axis_edge_ratio < 0.05:
        photo_score += 0.15
    photo_score = min(photo_score, 1.0)

    # Decide
    candidates = {
        "screenshot": ss_score,
        "document": doc_score,
        "artwork": art_score,
        "photograph": photo_score,
    }
    top_category, top_score = max(candidates.items(), key=lambda kv: kv[1])

    # Camera EXIF trumps everything except the artwork-software fingerprint
    # (which is how camera RAWs processed in Lightroom can still carry a
    # Make/Model — those remain photographs). A "photograph" with a real
    # Make/Model tag is near-impossible to fake without deliberate EXIF
    # injection, and injection is handled by a separate detector upstream.
    if has_camera_make_model and not art_software:
        top_category = "photograph"
        top_score = max(top_score, photo_score, 0.85)

    # Conservative floor: require ≥0.55 confidence to suppress AI detection.
    # Below that, fall back to photograph so the AI detection stage still runs.
    if top_category != "photograph" and top_score < 0.55:
        reasoning = (
            f"Top candidate '{top_category}' at {top_score:.2f} is below the "
            "0.55 conservative floor; defaulting to photograph so AI detection runs."
        )
        return ContentTypeResult(
            category="photograph",
            confidence=max(photo_score, 0.50),
            ai_detection_suitable=True,
            signals=signals,
            reasoning=reasoning,
        )

    # Build reasoning string
    reasoning_parts: list[str] = []
    if top_category == "screenshot":
        if matches_device_res:
            reasoning_parts.append(f"resolution {w}×{h} matches a known device")
        if screenshot_software:
            reasoning_parts.append("screenshot tool software fingerprint present")
        if megapixels >= 0.5 and unique_colours < 1000:
            reasoning_parts.append(f"only {unique_colours} unique colours in {megapixels:.1f} MP")
        if axis_edge_ratio > 0.15:
            reasoning_parts.append(f"axis-aligned edges {axis_edge_ratio:.0%}")
        if not has_camera_make_model:
            reasoning_parts.append("no camera EXIF")
    elif top_category == "document":
        reasoning_parts.append(f"{mono_ratio:.0%} near-black/white pixels")
        if paper_aspect:
            reasoning_parts.append("paper aspect ratio")
    elif top_category == "artwork":
        if art_software:
            reasoning_parts.append(f"digital-art software fingerprint ({exif.get('software','')})")
        if sat_var > 4000:
            reasoning_parts.append(f"high saturation variance ({sat_var:.0f})")
    else:  # photograph
        if has_camera_make_model:
            reasoning_parts.append(f"camera EXIF {exif.get('make','')} {exif.get('model','')}")
        else:
            reasoning_parts.append("no dispositive screenshot/document/artwork signals")

    reasoning = f"{top_category}: " + "; ".join(reasoning_parts) if reasoning_parts else top_category

    return ContentTypeResult(
        category=top_category,  # type: ignore[arg-type]
        confidence=round(float(top_score), 3),
        ai_detection_suitable=top_category in ("photograph", "artwork"),
        signals=signals,
        reasoning=reasoning,
    )
