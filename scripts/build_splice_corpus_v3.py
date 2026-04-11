#!/usr/bin/env python3
"""
Jura Trace — Synthetic Splice Corpus Builder v3 (adversarial extensions)

v3 extends v2 with four adversarial splice categories designed to stress-test
JPEG Ghost detection at its known failure boundaries:

1. Q-delta-near-threshold (Δ=5 or Δ=10):
   bg ∈ {80,85}, fg ∈ {75,80} so Δ ∈ {5,10}. These sit at or just below the
   detector's QUALITY_DEVIATION_THRESHOLD = 10 and are the hardest case for
   JPEG Ghost — low differential signal, high FN risk.

2. Platform re-encoding passes:
   After building a normal v2-style splice, apply a second JPEG save at Q=75
   (Twitter) or Q=85 (WhatsApp). Simulates social-media forwarding, which
   applies a uniform quantisation pass that flattens the differential ghost
   signal. Tests whether the detector survives realistic pipeline decay.

3. Alpha-blended edges:
   Instead of a hard rectangular paste, feather the paste region's edges with
   a 4–8 pixel Gaussian alpha gradient. Tests whether the detector handles
   realistic compositing rather than hard-edge rectangles.

4. Chroma subsampling mismatch:
   Paste a 4:4:4-encoded foreground region (subsampling=0) into a 4:2:0
   background (subsampling=2). Tests whether blocking-pattern features detect
   the subsampling-mode boundary.

v3 also retains all v2 baseline categories (small/medium/large/copy-move) and
authentic controls scaled to hit a 300-image total (150 spliced / 150 authentic).

Reuses v2's shared helpers (ijg_qtables, save_jpeg_with_quality,
reencode_at_quality) by direct import — do not modify v2.

Licence: uses only COCO (CC-BY 4.0) and Flickr30k (non-commercial cleared
under Option C corpus strategy). CASIA v1/v2 and other research-only datasets
are NOT used — Jura Trace is a commercial product.

Output: models/splice_calibration_v3/
  spliced/      — synthetic forgeries (baseline + adversarial)
  authentic/    — control images
  labels.json   — ground-truth with splice_subtype field

Usage:
    python scripts/build_splice_corpus_v3.py                # full 300
    python scripts/build_splice_corpus_v3.py --count 60     # pilot run
    python scripts/build_splice_corpus_v3.py --corpus /path/to/authentic/
"""

import argparse
import io
import json
import random
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

# ---------------------------------------------------------------------------
# Import shared helpers from v2
# ---------------------------------------------------------------------------

_V2_PATH = Path(__file__).parent / "build_splice_corpus_v2.py"
if not _V2_PATH.exists():
    print(f"ERROR: v2 script not found at {_V2_PATH}", file=sys.stderr)
    sys.exit(1)

# Import by exec to avoid requiring __init__.py
import importlib.util as _ilu
_spec = _ilu.spec_from_file_location("build_splice_corpus_v2", _V2_PATH)
_v2 = _ilu.module_from_spec(_spec)
_spec.loader.exec_module(_v2)

ijg_qtables = _v2.ijg_qtables
save_jpeg_with_quality = _v2.save_jpeg_with_quality
reencode_at_quality = _v2.reencode_at_quality
load_cc_by_images = _v2.load_cc_by_images
is_usable = _v2.is_usable
CC_BY_DIRS = _v2.CC_BY_DIRS
MIN_DIMENSION = _v2.MIN_DIMENSION
JPEG_BLOCK = _v2.JPEG_BLOCK

# ---------------------------------------------------------------------------
# v3 constants
# ---------------------------------------------------------------------------

# v2 baseline qualities (kept for baseline splice categories)
BG_SOURCE_QUALITIES = [80, 85, 90]
FG_SOURCE_QUALITIES = [50, 55, 60]
FINAL_SAVE_QUALITIES = [92, 95, 98]

# Adversarial: near-threshold — Δ ∈ {5, 10}
# bg ∈ {80,85}, fg ∈ {75,80}. Δ = bg − fg ∈ {0,5,10}.
# We enforce Δ ≥ 5 by pairing: (80,75)→Δ5, (85,75)→Δ10, (85,80)→Δ5.
NEAR_THRESH_PAIRS = [(80, 75), (85, 75), (85, 80)]

# Platform re-encoding qualities: Twitter ≈ Q=75, WhatsApp ≈ Q=85.
PLATFORM_QUALITIES = [75, 85]

# Alpha feather width range (pixels)
ALPHA_FEATHER_MIN = 4
ALPHA_FEATHER_MAX = 8

SPLICE_SMALL_MAX = 0.10
SPLICE_MEDIUM_MIN = 0.10
SPLICE_MEDIUM_MAX = 0.30
SPLICE_LARGE_MIN = 0.30


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _align8(n: int) -> int:
    return (n // JPEG_BLOCK) * JPEG_BLOCK


def _crop_and_reencode(
    img_orig: Image.Image,
    paste_w: int,
    paste_h: int,
    q: int,
    rng: random.Random,
    subsampling: int = 2,
) -> tuple[Image.Image, int, int]:
    """Crop a (paste_w × paste_h) region from img_orig and re-encode at quality q.
    Returns (crop, src_x, src_y)."""
    w, h = img_orig.size
    src_x = _align8(rng.randint(0, max(0, w - paste_w)))
    src_y = _align8(rng.randint(0, max(0, h - paste_h)))
    crop = img_orig.crop((src_x, src_y, src_x + paste_w, src_y + paste_h))
    buf = io.BytesIO()
    qtables = ijg_qtables(q)
    crop.save(buf, format="JPEG", qtables=qtables, subsampling=subsampling, optimize=False)
    buf.seek(0)
    return Image.open(buf).convert("RGB"), src_x, src_y


def _target_paste_dims(
    bg_w: int,
    bg_h: int,
    fg_w: int,
    fg_h: int,
    size_category: str,
    rng: random.Random,
) -> tuple[int, int]:
    """Return (paste_w, paste_h) aligned to 8px, or (0,0) if too small."""
    bg_area = bg_w * bg_h
    if size_category == "small":
        frac = rng.uniform(0.02, SPLICE_SMALL_MAX)
    elif size_category == "medium":
        frac = rng.uniform(SPLICE_MEDIUM_MIN, SPLICE_MEDIUM_MAX)
    else:
        frac = rng.uniform(SPLICE_LARGE_MIN, 0.60)
    target_area = int(bg_area * frac)
    aspect = rng.uniform(0.5, 2.0)
    ph = max(JPEG_BLOCK * 4, int((target_area / aspect) ** 0.5))
    pw = max(JPEG_BLOCK * 4, int(ph * aspect))
    pw = _align8(min(pw, bg_w, fg_w))
    ph = _align8(min(ph, bg_h, fg_h))
    if pw < JPEG_BLOCK * 4 or ph < JPEG_BLOCK * 4:
        return 0, 0
    return pw, ph


# ---------------------------------------------------------------------------
# Baseline splice generators (v2 logic, relabelled with splice_subtype)
# ---------------------------------------------------------------------------

def generate_baseline_splice(
    bg_path: Path,
    fg_path: Path,
    size_category: str,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    try:
        bg_orig = Image.open(bg_path).convert("RGB")
        fg_orig = Image.open(fg_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open): {exc}", file=sys.stderr)
        return None

    q_bg = rng.choice(BG_SOURCE_QUALITIES)
    q_fg = rng.choice(FG_SOURCE_QUALITIES)
    q_final = rng.choice(FINAL_SAVE_QUALITIES)

    bg = reencode_at_quality(bg_orig, q_bg)
    bg_w, bg_h = bg.size

    pw, ph = _target_paste_dims(bg_w, bg_h, *fg_orig.size, size_category, rng)
    if pw == 0:
        return None

    fg_crop, _, _ = _crop_and_reencode(fg_orig, pw, ph, q_fg, rng)
    dst_x = _align8(rng.randint(0, max(0, bg_w - pw)))
    dst_y = _align8(rng.randint(0, max(0, bg_h - ph)))

    composite = bg.copy()
    composite.paste(fg_crop, (dst_x, dst_y))
    save_jpeg_with_quality(composite, output_path, q_final)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": size_category,
        "splice_subtype": "baseline",
        "background_source": str(bg_path),
        "foreground_source": str(fg_path),
        "q_bg_true": q_bg,
        "q_fg_true": q_fg,
        "q_final": q_final,
        "q_delta": abs(q_bg - q_fg),
        "bbox": [dst_x, dst_y, dst_x + pw, dst_y + ph],
        "paste_area_fraction": round((pw * ph) / (bg_w * bg_h), 4),
        "bg_size": [bg_w, bg_h],
        "aligned_8px": True,
    }


def generate_copy_move(
    src_path: Path,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    try:
        img_orig = Image.open(src_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open): {exc}", file=sys.stderr)
        return None

    q = rng.choice(BG_SOURCE_QUALITIES)
    q_final = rng.choice(FINAL_SAVE_QUALITIES)
    img = reencode_at_quality(img_orig, q)
    w, h = img.size

    frac = rng.uniform(0.10, 0.30)
    area = w * h
    aspect = rng.uniform(0.5, 2.0)
    rh = _align8(max(JPEG_BLOCK * 4, int((area * frac / aspect) ** 0.5)))
    rw = _align8(max(JPEG_BLOCK * 4, int(rh * aspect)))
    rw = _align8(min(rw, w))
    rh = _align8(min(rh, h))
    if rw < JPEG_BLOCK * 4 or rh < JPEG_BLOCK * 4:
        return None

    src_x = _align8(rng.randint(0, max(0, w - rw)))
    src_y = _align8(rng.randint(0, max(0, h - rh)))
    region = img.crop((src_x, src_y, src_x + rw, src_y + rh))

    for _ in range(20):
        dx = _align8(rng.randint(0, max(0, w - rw)))
        dy = _align8(rng.randint(0, max(0, h - rh)))
        if abs(dx - src_x) > rw or abs(dy - src_y) > rh:
            break
    else:
        dx = _align8((src_x + w // 2) % max(1, w - rw))
        dy = _align8((src_y + h // 2) % max(1, h - rh))

    composite = img.copy()
    composite.paste(region, (dx, dy))
    save_jpeg_with_quality(composite, output_path, q_final)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": "copy_move",
        "splice_subtype": "baseline",
        "background_source": str(src_path),
        "foreground_source": str(src_path),
        "q_bg_true": q,
        "q_fg_true": q,
        "q_final": q_final,
        "q_delta": 0,
        "bbox": [dx, dy, dx + rw, dy + rh],
        "paste_area_fraction": round((rw * rh) / area, 4),
        "bg_size": [w, h],
        "aligned_8px": True,
    }


# ---------------------------------------------------------------------------
# Adversarial category 1: near-threshold (Δ=5 or Δ=10)
# ---------------------------------------------------------------------------

def generate_near_threshold(
    bg_path: Path,
    fg_path: Path,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    """Q-delta ∈ {5,10} — sits at/below the detector's deviation threshold."""
    try:
        bg_orig = Image.open(bg_path).convert("RGB")
        fg_orig = Image.open(fg_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open): {exc}", file=sys.stderr)
        return None

    q_bg, q_fg = rng.choice(NEAR_THRESH_PAIRS)
    q_final = rng.choice(FINAL_SAVE_QUALITIES)

    bg = reencode_at_quality(bg_orig, q_bg)
    bg_w, bg_h = bg.size
    fg_w, fg_h = fg_orig.size

    # Use medium-sized paste region for this adversarial category
    pw, ph = _target_paste_dims(bg_w, bg_h, fg_w, fg_h, "medium", rng)
    if pw == 0:
        return None

    fg_crop, _, _ = _crop_and_reencode(fg_orig, pw, ph, q_fg, rng)
    dst_x = _align8(rng.randint(0, max(0, bg_w - pw)))
    dst_y = _align8(rng.randint(0, max(0, bg_h - ph)))

    composite = bg.copy()
    composite.paste(fg_crop, (dst_x, dst_y))
    save_jpeg_with_quality(composite, output_path, q_final)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": "medium",
        "splice_subtype": "near_threshold",
        "background_source": str(bg_path),
        "foreground_source": str(fg_path),
        "q_bg_true": q_bg,
        "q_fg_true": q_fg,
        "q_final": q_final,
        "q_delta": abs(q_bg - q_fg),
        "bbox": [dst_x, dst_y, dst_x + pw, dst_y + ph],
        "paste_area_fraction": round((pw * ph) / (bg_w * bg_h), 4),
        "bg_size": [bg_w, bg_h],
        "aligned_8px": True,
    }


# ---------------------------------------------------------------------------
# Adversarial category 2: platform re-encoding
# ---------------------------------------------------------------------------

def generate_platform_reencode(
    bg_path: Path,
    fg_path: Path,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    """Build a normal splice then apply a second uniform save at Q=75 or Q=85."""
    try:
        bg_orig = Image.open(bg_path).convert("RGB")
        fg_orig = Image.open(fg_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open): {exc}", file=sys.stderr)
        return None

    q_bg = rng.choice(BG_SOURCE_QUALITIES)
    q_fg = rng.choice(FG_SOURCE_QUALITIES)
    q_intermediate = rng.choice(FINAL_SAVE_QUALITIES)
    q_platform = rng.choice(PLATFORM_QUALITIES)

    bg = reencode_at_quality(bg_orig, q_bg)
    bg_w, bg_h = bg.size
    fg_w, fg_h = fg_orig.size

    pw, ph = _target_paste_dims(bg_w, bg_h, fg_w, fg_h, "medium", rng)
    if pw == 0:
        return None

    fg_crop, _, _ = _crop_and_reencode(fg_orig, pw, ph, q_fg, rng)
    dst_x = _align8(rng.randint(0, max(0, bg_w - pw)))
    dst_y = _align8(rng.randint(0, max(0, bg_h - ph)))

    composite = bg.copy()
    composite.paste(fg_crop, (dst_x, dst_y))

    # Intermediate save at high Q to establish splice
    buf = io.BytesIO()
    save_jpeg_with_quality(composite, buf, q_intermediate)
    buf.seek(0)
    intermediate = Image.open(buf).convert("RGB")

    # Platform re-encode pass (Twitter Q=75 or WhatsApp Q=85)
    save_jpeg_with_quality(intermediate, output_path, q_platform)

    platform_name = "twitter" if q_platform == 75 else "whatsapp"
    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": "medium",
        "splice_subtype": "platform_reencode",
        "platform": platform_name,
        "background_source": str(bg_path),
        "foreground_source": str(fg_path),
        "q_bg_true": q_bg,
        "q_fg_true": q_fg,
        "q_intermediate": q_intermediate,
        "q_final": q_platform,
        "q_delta": abs(q_bg - q_fg),
        "bbox": [dst_x, dst_y, dst_x + pw, dst_y + ph],
        "paste_area_fraction": round((pw * ph) / (bg_w * bg_h), 4),
        "bg_size": [bg_w, bg_h],
        "aligned_8px": True,
    }


# ---------------------------------------------------------------------------
# Adversarial category 3: alpha-blended edges
# ---------------------------------------------------------------------------

def generate_alpha_blended(
    bg_path: Path,
    fg_path: Path,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    """Paste with a feathered edge gradient instead of a hard rectangle."""
    try:
        bg_orig = Image.open(bg_path).convert("RGB")
        fg_orig = Image.open(fg_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open): {exc}", file=sys.stderr)
        return None

    q_bg = rng.choice(BG_SOURCE_QUALITIES)
    q_fg = rng.choice(FG_SOURCE_QUALITIES)
    q_final = rng.choice(FINAL_SAVE_QUALITIES)
    feather = rng.randint(ALPHA_FEATHER_MIN, ALPHA_FEATHER_MAX)

    bg = reencode_at_quality(bg_orig, q_bg)
    bg_w, bg_h = bg.size
    fg_w, fg_h = fg_orig.size

    pw, ph = _target_paste_dims(bg_w, bg_h, fg_w, fg_h, "medium", rng)
    if pw == 0:
        return None

    fg_crop, _, _ = _crop_and_reencode(fg_orig, pw, ph, q_fg, rng)
    dst_x = _align8(rng.randint(0, max(0, bg_w - pw)))
    dst_y = _align8(rng.randint(0, max(0, bg_h - ph)))

    # Build a feathered alpha mask: solid centre, gradient edges
    mask_arr = np.ones((ph, pw), dtype=np.float32)
    for i in range(feather):
        alpha = (i + 1) / (feather + 1)
        if i < ph:
            mask_arr[i, :] = np.minimum(mask_arr[i, :], alpha)
            mask_arr[ph - 1 - i, :] = np.minimum(mask_arr[ph - 1 - i, :], alpha)
        if i < pw:
            mask_arr[:, i] = np.minimum(mask_arr[:, i], alpha)
            mask_arr[:, pw - 1 - i] = np.minimum(mask_arr[:, pw - 1 - i], alpha)
    mask_pil = Image.fromarray((mask_arr * 255).astype(np.uint8))

    # Composite: bg + fg blend via mask
    bg_rgba = bg.copy().convert("RGBA")
    fg_rgba = fg_crop.convert("RGBA")
    # Paste fg over bg using the feather mask as alpha
    composite_rgba = bg_rgba.copy()
    bg_crop = bg.crop((dst_x, dst_y, dst_x + pw, dst_y + ph))
    blended = Image.composite(fg_rgba, bg_crop.convert("RGBA"), mask_pil)
    composite_rgba.paste(blended, (dst_x, dst_y))
    composite = composite_rgba.convert("RGB")

    save_jpeg_with_quality(composite, output_path, q_final)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": "medium",
        "splice_subtype": "alpha_blended",
        "background_source": str(bg_path),
        "foreground_source": str(fg_path),
        "q_bg_true": q_bg,
        "q_fg_true": q_fg,
        "q_final": q_final,
        "q_delta": abs(q_bg - q_fg),
        "feather_px": feather,
        "bbox": [dst_x, dst_y, dst_x + pw, dst_y + ph],
        "paste_area_fraction": round((pw * ph) / (bg_w * bg_h), 4),
        "bg_size": [bg_w, bg_h],
        "aligned_8px": True,
    }


# ---------------------------------------------------------------------------
# Adversarial category 4: chroma subsampling mismatch
# ---------------------------------------------------------------------------

def _reencode_subsampling(img: Image.Image, quality: int, subsampling: int) -> Image.Image:
    """Re-encode at specified quality and subsampling mode."""
    buf = io.BytesIO()
    qtables = ijg_qtables(quality)
    img.save(buf, format="JPEG", qtables=qtables, subsampling=subsampling, optimize=False)
    buf.seek(0)
    return Image.open(buf).convert("RGB")


def generate_chroma_mismatch(
    bg_path: Path,
    fg_path: Path,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    """Paste 4:4:4 foreground region into a 4:2:0 background."""
    try:
        bg_orig = Image.open(bg_path).convert("RGB")
        fg_orig = Image.open(fg_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open): {exc}", file=sys.stderr)
        return None

    q_bg = rng.choice(BG_SOURCE_QUALITIES)
    q_fg = rng.choice(FG_SOURCE_QUALITIES)
    q_final = rng.choice(FINAL_SAVE_QUALITIES)

    # bg: 4:2:0 (subsampling=2); fg: 4:4:4 (subsampling=0)
    bg = _reencode_subsampling(bg_orig, q_bg, subsampling=2)
    bg_w, bg_h = bg.size
    fg_w, fg_h = fg_orig.size

    pw, ph = _target_paste_dims(bg_w, bg_h, fg_w, fg_h, "medium", rng)
    if pw == 0:
        return None

    # fg crop re-encoded at 4:4:4
    src_x = _align8(rng.randint(0, max(0, fg_w - pw)))
    src_y = _align8(rng.randint(0, max(0, fg_h - ph)))
    fg_crop_orig = fg_orig.crop((src_x, src_y, src_x + pw, src_y + ph))
    fg_crop = _reencode_subsampling(fg_crop_orig, q_fg, subsampling=0)

    dst_x = _align8(rng.randint(0, max(0, bg_w - pw)))
    dst_y = _align8(rng.randint(0, max(0, bg_h - ph)))

    composite = bg.copy()
    composite.paste(fg_crop, (dst_x, dst_y))
    # Final save at 4:2:0 to match the overall image container
    save_jpeg_with_quality(composite, output_path, q_final)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": "medium",
        "splice_subtype": "chroma_mismatch",
        "background_source": str(bg_path),
        "foreground_source": str(fg_path),
        "q_bg_true": q_bg,
        "q_fg_true": q_fg,
        "q_final": q_final,
        "q_delta": abs(q_bg - q_fg),
        "bg_subsampling": "4:2:0",
        "fg_subsampling": "4:4:4",
        "bbox": [dst_x, dst_y, dst_x + pw, dst_y + ph],
        "paste_area_fraction": round((pw * ph) / (bg_w * bg_h), 4),
        "bg_size": [bg_w, bg_h],
        "aligned_8px": True,
    }


# ---------------------------------------------------------------------------
# Authentic controls (same as v2)
# ---------------------------------------------------------------------------

def save_authentic_plain(src_path: Path, output_path: Path, q_save: int | None = None) -> dict:
    if q_save is None:
        shutil.copy2(src_path, output_path)
        category = "plain"
        q_actual = None
    else:
        img = Image.open(src_path).convert("RGB")
        save_jpeg_with_quality(img, output_path, q_save)
        q_actual = q_save
        category = "high_quality" if q_save >= 90 else ("heavy_compression" if q_save <= 50 else "plain")
    return {
        "filename": output_path.name,
        "label": "authentic",
        "control_type": category,
        "splice_subtype": None,
        "source": str(src_path),
        "quality": q_actual,
    }


def save_authentic_multicompressed(src_path: Path, output_path: Path, rng: random.Random) -> dict:
    qualities = [85, 80, 75, 70, 85, 80]
    n_rounds = rng.randint(2, 3)
    round_qs = rng.choices(qualities, k=n_rounds)
    img = Image.open(src_path).convert("RGB")
    for q in round_qs:
        img = reencode_at_quality(img, q)
    save_jpeg_with_quality(img, output_path, round_qs[-1])
    return {
        "filename": output_path.name,
        "label": "authentic",
        "control_type": "multi_compression",
        "splice_subtype": None,
        "source": str(src_path),
        "compression_rounds": n_rounds,
        "quality_chain": round_qs,
    }


# ---------------------------------------------------------------------------
# Main corpus builder
# ---------------------------------------------------------------------------

def build_corpus(corpus_dir: Path, output_dir: Path, count: int, seed: int) -> None:
    rng = random.Random(seed)
    scale = count / 300.0

    # Spliced breakdown (150 at full scale)
    n_small      = max(1, round(25 * scale))
    n_medium     = max(1, round(20 * scale))
    n_large      = max(1, round(15 * scale))
    n_copymove   = max(1, round(15 * scale))
    n_near_thresh = max(1, round(20 * scale))
    n_platform   = max(1, round(20 * scale))
    n_alpha      = max(1, round(15 * scale))
    n_chroma     = max(1, round(20 * scale))
    n_spliced    = (n_small + n_medium + n_large + n_copymove
                    + n_near_thresh + n_platform + n_alpha + n_chroma)

    # Authentic breakdown (150 at full scale)
    n_multicomp  = max(1, round(40 * scale))
    n_high_q     = max(1, round(20 * scale))
    n_heavy_q    = max(1, round(20 * scale))
    n_plain      = max(1, round(70 * scale))
    n_authentic  = n_multicomp + n_high_q + n_heavy_q + n_plain

    total = n_spliced + n_authentic

    print(f"\nCorpus plan v3 ({total} images):")
    print(f"  Spliced: {n_spliced}")
    print(f"    baseline small={n_small} medium={n_medium} large={n_large} copy-move={n_copymove}")
    print(f"    adversarial near-threshold={n_near_thresh} platform-reencode={n_platform} "
          f"alpha-blended={n_alpha} chroma-mismatch={n_chroma}")
    print(f"  Authentic: {n_authentic}")
    print(f"    multi-comp={n_multicomp} hi-Q={n_high_q} lo-Q={n_heavy_q} plain={n_plain}")

    images = load_cc_by_images(corpus_dir, rng)
    print("  Filtering for usability ...")
    usable = [p for p in images if is_usable(p)]
    print(f"  Usable: {len(usable)} / {len(images)}")
    if len(usable) < total * 2:
        print(f"  WARNING: usable pool ({len(usable)}) < 2× corpus target ({total})", file=sys.stderr)

    spliced_dir = output_dir / "spliced"
    authentic_dir = output_dir / "authentic"
    spliced_dir.mkdir(parents=True, exist_ok=True)
    authentic_dir.mkdir(parents=True, exist_ok=True)

    labels: list[dict] = []
    errors: list[str] = []
    pool = list(usable)

    def draw(n: int = 1) -> list[Path]:
        drawn: list[Path] = []
        for _ in range(n):
            if not pool:
                pool.extend(usable)
                rng.shuffle(pool)
            drawn.append(pool.pop())
        return drawn

    # --- Baseline splices ---
    for cat, count_cat in [("small", n_small), ("medium", n_medium), ("large", n_large)]:
        print(f"\nGenerating {count_cat} baseline {cat} splices ...")
        for i in range(count_cat):
            bg, fg = draw(2)
            name = f"splice_{cat}_{i:03d}.jpg"
            out = spliced_dir / name
            print(f"  [{i+1}/{count_cat}] {name} ...", end=" ", flush=True)
            lbl = generate_baseline_splice(bg, fg, cat, rng, out)
            if lbl:
                labels.append(lbl)
                print(f"ok (Δ={lbl['q_delta']}, q_final={lbl['q_final']})")
            else:
                errors.append(name)
                print("FAILED")

    print(f"\nGenerating {n_copymove} copy-move splices ...")
    for i in range(n_copymove):
        src = draw()[0]
        name = f"splice_copy_move_{i:03d}.jpg"
        out = spliced_dir / name
        print(f"  [{i+1}/{n_copymove}] {name} ...", end=" ", flush=True)
        lbl = generate_copy_move(src, rng, out)
        if lbl:
            labels.append(lbl)
            print("ok")
        else:
            errors.append(name)
            print("FAILED")

    # --- Adversarial: near-threshold ---
    print(f"\nGenerating {n_near_thresh} near-threshold adversarial splices (Δ∈{{5,10}}) ...")
    for i in range(n_near_thresh):
        bg, fg = draw(2)
        name = f"splice_near_thresh_{i:03d}.jpg"
        out = spliced_dir / name
        print(f"  [{i+1}/{n_near_thresh}] {name} ...", end=" ", flush=True)
        lbl = generate_near_threshold(bg, fg, rng, out)
        if lbl:
            labels.append(lbl)
            print(f"ok (Δ={lbl['q_delta']})")
        else:
            errors.append(name)
            print("FAILED")

    # --- Adversarial: platform re-encoding ---
    print(f"\nGenerating {n_platform} platform-reencode adversarial splices ...")
    for i in range(n_platform):
        bg, fg = draw(2)
        name = f"splice_platform_{i:03d}.jpg"
        out = spliced_dir / name
        print(f"  [{i+1}/{n_platform}] {name} ...", end=" ", flush=True)
        lbl = generate_platform_reencode(bg, fg, rng, out)
        if lbl:
            labels.append(lbl)
            print(f"ok (platform={lbl.get('platform')}, q_final={lbl['q_final']})")
        else:
            errors.append(name)
            print("FAILED")

    # --- Adversarial: alpha-blended edges ---
    print(f"\nGenerating {n_alpha} alpha-blended adversarial splices ...")
    for i in range(n_alpha):
        bg, fg = draw(2)
        name = f"splice_alpha_{i:03d}.jpg"
        out = spliced_dir / name
        print(f"  [{i+1}/{n_alpha}] {name} ...", end=" ", flush=True)
        lbl = generate_alpha_blended(bg, fg, rng, out)
        if lbl:
            labels.append(lbl)
            print(f"ok (feather={lbl.get('feather_px')}px)")
        else:
            errors.append(name)
            print("FAILED")

    # --- Adversarial: chroma subsampling mismatch ---
    print(f"\nGenerating {n_chroma} chroma-mismatch adversarial splices ...")
    for i in range(n_chroma):
        bg, fg = draw(2)
        name = f"splice_chroma_{i:03d}.jpg"
        out = spliced_dir / name
        print(f"  [{i+1}/{n_chroma}] {name} ...", end=" ", flush=True)
        lbl = generate_chroma_mismatch(bg, fg, rng, out)
        if lbl:
            labels.append(lbl)
            print(f"ok (Δ={lbl['q_delta']}, bg=4:2:0, fg=4:4:4)")
        else:
            errors.append(name)
            print("FAILED")

    # --- Authentic controls ---
    print(f"\nGenerating {n_multicomp} multi-compression authentic controls ...")
    for i in range(n_multicomp):
        src = draw()[0]
        name = f"auth_multicomp_{i:03d}.jpg"
        out = authentic_dir / name
        print(f"  [{i+1}/{n_multicomp}] {name} ...", end=" ", flush=True)
        labels.append(save_authentic_multicompressed(src, out, rng))
        print("ok")

    print(f"\nGenerating {n_high_q} high-quality authentic controls (Q≥90) ...")
    for i in range(n_high_q):
        src = draw()[0]
        q = rng.choice([90, 92, 95])
        name = f"auth_highq_{i:03d}.jpg"
        out = authentic_dir / name
        print(f"  [{i+1}/{n_high_q}] {name} (Q={q}) ...", end=" ", flush=True)
        labels.append(save_authentic_plain(src, out, q_save=q))
        print("ok")

    print(f"\nGenerating {n_heavy_q} heavily-compressed authentic controls (Q≤50) ...")
    for i in range(n_heavy_q):
        src = draw()[0]
        q = rng.choice([40, 45, 50])
        name = f"auth_heavyq_{i:03d}.jpg"
        out = authentic_dir / name
        print(f"  [{i+1}/{n_heavy_q}] {name} (Q={q}) ...", end=" ", flush=True)
        labels.append(save_authentic_plain(src, out, q_save=q))
        print("ok")

    print(f"\nGenerating {n_plain} plain authentic controls ...")
    for i in range(n_plain):
        src = draw()[0]
        name = f"auth_plain_{i:03d}.jpg"
        out = authentic_dir / name
        print(f"  [{i+1}/{n_plain}] {name} ...", end=" ", flush=True)
        labels.append(save_authentic_plain(src, out))
        print("ok")

    # --- Summary JSON ---
    summary = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "generator_version": "v3",
        "seed": seed,
        "count_requested": count,
        "count_generated": len(labels),
        "errors": errors,
        "generation_method": {
            "explicit_ijg_qtables": True,
            "final_save_qualities": FINAL_SAVE_QUALITIES,
            "bg_source_qualities": BG_SOURCE_QUALITIES,
            "fg_source_qualities": FG_SOURCE_QUALITIES,
            "paste_8_aligned": True,
            "adversarial_categories": [
                "near_threshold (Δ=5,10)",
                "platform_reencode (Twitter Q=75, WhatsApp Q=85)",
                "alpha_blended (feather 4-8px)",
                "chroma_mismatch (4:4:4 fg into 4:2:0 bg)",
            ],
        },
        "corpus_breakdown": {
            "splice_small": n_small,
            "splice_medium": n_medium,
            "splice_large": n_large,
            "splice_copy_move": n_copymove,
            "splice_near_threshold": n_near_thresh,
            "splice_platform_reencode": n_platform,
            "splice_alpha_blended": n_alpha,
            "splice_chroma_mismatch": n_chroma,
            "auth_multi_compression": n_multicomp,
            "auth_high_quality": n_high_q,
            "auth_heavy_compression": n_heavy_q,
            "auth_plain": n_plain,
        },
        "licence_note": (
            "CC-BY 4.0 (COCO) + non-commercial-cleared (Flickr30k under Option C). "
            "CASIA v1/v2 and other research-only datasets are NOT used — "
            "Jura Trace is a commercial product requiring commercial-use-cleared data."
        ),
        "images": labels,
    }

    labels_path = output_dir / "labels.json"
    labels_path.write_text(json.dumps(summary, indent=2))
    print(f"\nLabels written: {labels_path}")

    if errors:
        print(f"\nWARNING: {len(errors)} images failed: {errors}", file=sys.stderr)

    n_spl = sum(1 for l in labels if l["label"] == "spliced")
    n_aut = sum(1 for l in labels if l["label"] == "authentic")
    print(f"\nCorpus v3 complete: {len(labels)} images ({n_spl} spliced, {n_aut} authentic)")
    print(f"Output: {output_dir}")


def main() -> None:
    default_corpus = "/Volumes/Samsung USB/Training Data/corpus/training/authentic"
    default_output = Path(__file__).parent.parent / "models" / "splice_calibration_v3"

    parser = argparse.ArgumentParser(
        description="Build synthetic splice test corpus v3 (adversarial extensions)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--corpus", type=Path, default=default_corpus)
    parser.add_argument("--output", type=Path, default=default_output)
    parser.add_argument("--count", type=int, default=300)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    if args.count < 10:
        print("Error: --count must be at least 10.", file=sys.stderr)
        sys.exit(1)
    if not args.corpus.exists():
        print(f"Error: corpus directory not found: {args.corpus}", file=sys.stderr)
        sys.exit(1)

    print("Jura Trace — Synthetic Splice Corpus Builder v3")
    print(f"  Corpus:  {args.corpus}")
    print(f"  Output:  {args.output}")
    print(f"  Count:   {args.count}")
    print(f"  Seed:    {args.seed}")

    start = datetime.now()
    build_corpus(args.corpus, args.output, args.count, args.seed)
    elapsed = (datetime.now() - start).total_seconds()
    print(f"\nCompleted in {elapsed:.1f}s")


if __name__ == "__main__":
    main()
