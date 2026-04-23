#!/usr/bin/env python3
"""
Jura Trace — Synthetic Splice Corpus Builder v2 (commercial-licence-clean)

v2 improvements over scripts/build_splice_corpus.py (v1):

1. Explicit IJG standard quantisation tables via PIL's `qtables=` kwarg
   instead of PIL's approximate `quality=` parameter. Eliminates the
   Q-estimation round-trip and guarantees deterministic per-image Q values
   matching what JPEG Ghost's own re-save loop produces.

2. High final save quality (Q_final ∈ {92, 95, 98}). v1 re-saved the
   composite at Q ∈ {60, 70, 80, 90}, which applied a harsh uniform
   quantisation pass that wiped the differential source-region ghost
   patterns JPEG Ghost is designed to detect. v2 keeps the final pass
   light so source Q footprints survive in both background and pasted
   regions.

3. Wider Q-delta between background and foreground sources. v1 used a
   min delta of 20; v2 uses 25+ and draws from disjoint ranges
   (bg ∈ {80,85,90}, fg ∈ {50,55,60}) so every splice has Δ ≥ 20 with
   most at Δ ≥ 25-40.

4. 8-pixel-aligned paste coordinates. JPEG encodes on an 8×8 DCT block
   grid; pasting on non-aligned coordinates distributes the pasted
   region's DCT signature across 4 blocks, diluting ghost signal. v2
   rounds dst_x, dst_y, paste_w, paste_h to multiples of 8.

5. Explicit re-encoding of bg and fg sources through a known-Q JPEG pass
   before compositing. This establishes deterministic source-side
   quantisation that ghost analysis can detect, regardless of what Q the
   original USB corpus file was encoded at.

Licence: uses only COCO (CC-BY 4.0), Open Images (CC-BY 2.0), and
Flickr30k (non-commercial cleared under Option C corpus strategy — see
docs/decisions/option-c-corpus-strategy.md). CASIA v1/v2 and other
research-only datasets are explicitly rejected — Jura Trace is a
commercial product and requires commercial-use-cleared training data.

Output: models/splice_calibration_150_v2/
  spliced/      — synthetic forgeries with preserved JPEG ghost patterns
  authentic/    — control images (multi-compression + quality variants)
  labels.json   — ground-truth metadata with true (not estimated) Q values

Usage:
    python scripts/build_splice_corpus_v2.py --corpus /path/to/authentic/
    python scripts/build_splice_corpus_v2.py --count 30  # pilot run
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
from PIL import Image

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# CC-BY-compatible source subdirectories on the USB corpus.
# Flickr30k: non-commercial cleared under Option C.
CC_BY_DIRS = ["coco", "coco_extra", "coco_extra2", "coco_train", "flickr30k"]

MIN_DIMENSION = 256

# Final save qualities — HIGH so source Q footprints survive.
# v1 used {60,70,80,90} which wiped ghost patterns.
FINAL_SAVE_QUALITIES = [92, 95, 98]

# Source-side re-encoding qualities. Disjoint ranges guarantee a clean
# Q-delta on every splice.
BG_SOURCE_QUALITIES = [80, 85, 90]
FG_SOURCE_QUALITIES = [50, 55, 60]

SPLICE_SMALL_MAX = 0.10
SPLICE_MEDIUM_MIN = 0.10
SPLICE_MEDIUM_MAX = 0.30
SPLICE_LARGE_MIN = 0.30

# JPEG DCT block size — paste coordinates must align to this.
JPEG_BLOCK = 8


# ---------------------------------------------------------------------------
# IJG standard quantisation tables
# ---------------------------------------------------------------------------

# Standard JPEG luminance quantisation table at Q=50 (IJG / ITU T.81 Annex K).
_STD_LUM_Q50 = np.array([
    16, 11, 10, 16,  24,  40,  51,  61,
    12, 12, 14, 19,  26,  58,  60,  55,
    14, 13, 16, 24,  40,  57,  69,  56,
    14, 17, 22, 29,  51,  87,  80,  62,
    18, 22, 37, 56,  68, 109, 103,  77,
    24, 35, 55, 64,  81, 104, 113,  92,
    49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103,  99,
], dtype=np.int32)

# Standard JPEG chrominance quantisation table at Q=50.
_STD_CHROM_Q50 = np.array([
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
], dtype=np.int32)


def ijg_qtables(quality: int) -> list[list[int]]:
    """
    Compute IJG-standard luminance and chrominance quantisation tables
    for the given integer quality ∈ [1, 100]. Matches libjpeg-turbo's
    jpeg_set_quality() scaling.

    Returns a list of two 64-element lists suitable for passing to
    PIL's Image.save(qtables=...).
    """
    q = max(1, min(100, int(quality)))
    if q < 50:
        scale = 5000 // q
    else:
        scale = 200 - 2 * q

    def _scale(table: np.ndarray) -> list[int]:
        scaled = (table.astype(np.int64) * scale + 50) // 100
        # Clamp to [1, 255] — libjpeg-turbo's baseline behaviour.
        scaled = np.clip(scaled, 1, 255)
        return scaled.tolist()

    return [_scale(_STD_LUM_Q50), _scale(_STD_CHROM_Q50)]


def save_jpeg_with_quality(
    img: Image.Image,
    output: Path | io.BytesIO,
    quality: int,
) -> None:
    """
    Save a PIL image as JPEG using explicit IJG-standard quant tables.
    Deterministic — two calls with the same quality produce byte-identical
    DCT coefficients (modulo Huffman table choices).
    """
    qtables = ijg_qtables(quality)
    if isinstance(output, Path):
        with open(output, "wb") as f:
            img.save(f, format="JPEG", qtables=qtables, subsampling=2, optimize=False)
    else:
        img.save(output, format="JPEG", qtables=qtables, subsampling=2, optimize=False)


def reencode_at_quality(img: Image.Image, quality: int) -> Image.Image:
    """Round-trip an RGB image through a JPEG at the given quality."""
    buf = io.BytesIO()
    save_jpeg_with_quality(img, buf, quality)
    buf.seek(0)
    return Image.open(buf).convert("RGB")


# ---------------------------------------------------------------------------
# Corpus loading
# ---------------------------------------------------------------------------

def load_cc_by_images(corpus_dir: Path, rng: random.Random) -> list[Path]:
    images: list[Path] = []
    missing: list[str] = []
    for subdir in CC_BY_DIRS:
        d = corpus_dir / subdir
        if not d.exists():
            missing.append(subdir)
            continue
        found = sorted(d.glob("*.jpg")) + sorted(d.glob("*.jpeg")) + sorted(d.glob("*.JPG"))
        images.extend(found)
    if missing:
        print(f"  WARNING: CC-BY subdirs not found: {missing}", file=sys.stderr)
    rng.shuffle(images)
    if len(images) < 200:
        print(
            f"  ERROR: Only {len(images)} CC-BY images found in {corpus_dir}. "
            "Expected ≥200.",
            file=sys.stderr,
        )
        sys.exit(1)
    print(f"  CC-BY source pool: {len(images)} images from {len(CC_BY_DIRS) - len(missing)} dirs")
    return images


def is_usable(img_path: Path) -> bool:
    try:
        with Image.open(img_path) as img:
            w, h = img.size
            return w >= MIN_DIMENSION and h >= MIN_DIMENSION
    except Exception:
        return False


# ---------------------------------------------------------------------------
# Splice generation
# ---------------------------------------------------------------------------

def _align8(n: int) -> int:
    """Round down to nearest multiple of 8 (JPEG DCT block size)."""
    return (n // JPEG_BLOCK) * JPEG_BLOCK


def generate_splice(
    bg_path: Path,
    fg_path: Path,
    size_category: str,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    """
    Generate a single spliced image with deterministic source quantisation.

    Procedure:
    1. Load bg and fg as RGB.
    2. Re-encode bg at a random Q from BG_SOURCE_QUALITIES via explicit
       qtables. This establishes the background's ghost signature at Q_bg.
    3. Crop a rectangle from fg (8-aligned dimensions).
    4. Re-encode the fg crop at a random Q from FG_SOURCE_QUALITIES.
       This establishes the foreground's ghost signature at Q_fg.
    5. Paste onto the re-encoded background at 8-aligned coordinates.
    6. Save the composite at a HIGH Q_final ∈ {92, 95, 98} so neither
       source footprint is wiped by the final quantisation pass.
    """
    try:
        bg_orig = Image.open(bg_path).convert("RGB")
        fg_orig = Image.open(fg_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open error): {exc}", file=sys.stderr)
        return None

    q_bg = rng.choice(BG_SOURCE_QUALITIES)
    q_fg = rng.choice(FG_SOURCE_QUALITIES)
    q_final = rng.choice(FINAL_SAVE_QUALITIES)

    # Step 2 — re-encode bg at Q_bg.
    bg = reencode_at_quality(bg_orig, q_bg)
    bg_w, bg_h = bg.size
    bg_area = bg_w * bg_h

    # Target paste area by category.
    if size_category == "small":
        target_fraction = rng.uniform(0.02, SPLICE_SMALL_MAX)
    elif size_category == "medium":
        target_fraction = rng.uniform(SPLICE_MEDIUM_MIN, SPLICE_MEDIUM_MAX)
    else:
        target_fraction = rng.uniform(SPLICE_LARGE_MIN, 0.60)

    target_area = int(bg_area * target_fraction)
    aspect = rng.uniform(0.5, 2.0)
    paste_h = max(JPEG_BLOCK * 4, int((target_area / aspect) ** 0.5))
    paste_w = max(JPEG_BLOCK * 4, int(paste_h * aspect))

    # Step 3 — crop fg and force 8-aligned dimensions.
    fg_w, fg_h = fg_orig.size
    paste_w = _align8(min(paste_w, bg_w, fg_w))
    paste_h = _align8(min(paste_h, bg_h, fg_h))
    if paste_w < JPEG_BLOCK * 4 or paste_h < JPEG_BLOCK * 4:
        return None

    src_x = _align8(rng.randint(0, max(0, fg_w - paste_w)))
    src_y = _align8(rng.randint(0, max(0, fg_h - paste_h)))
    fg_crop = fg_orig.crop((src_x, src_y, src_x + paste_w, src_y + paste_h))

    # Step 4 — re-encode fg crop at Q_fg.
    fg_crop = reencode_at_quality(fg_crop, q_fg)

    # Step 5 — 8-aligned paste location in bg.
    dst_x = _align8(rng.randint(0, max(0, bg_w - paste_w)))
    dst_y = _align8(rng.randint(0, max(0, bg_h - paste_h)))

    composite = bg.copy()
    composite.paste(fg_crop, (dst_x, dst_y))

    # Step 6 — save at HIGH final Q so ghost patterns survive.
    save_jpeg_with_quality(composite, output_path, q_final)

    actual_area_fraction = round((paste_w * paste_h) / bg_area, 4)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": size_category,
        "background_source": str(bg_path),
        "foreground_source": str(fg_path),
        "q_bg_true": q_bg,
        "q_fg_true": q_fg,
        "q_final": q_final,
        "q_delta": abs(q_bg - q_fg),
        "bbox": [dst_x, dst_y, dst_x + paste_w, dst_y + paste_h],
        "paste_area_fraction": actual_area_fraction,
        "bg_size": [bg_w, bg_h],
        "aligned_8px": True,
    }


def generate_copy_move(
    src_path: Path,
    rng: random.Random,
    output_path: Path,
) -> dict | None:
    """Same-source region paste. Q-delta = 0 — JPEG Ghost should score near 0."""
    try:
        img_orig = Image.open(src_path).convert("RGB")
    except Exception as exc:
        print(f"    skip (open error): {exc}", file=sys.stderr)
        return None

    q = rng.choice(BG_SOURCE_QUALITIES)
    q_final = rng.choice(FINAL_SAVE_QUALITIES)
    img = reencode_at_quality(img_orig, q)
    w, h = img.size
    area = w * h

    target_fraction = rng.uniform(0.10, 0.30)
    target_area = int(area * target_fraction)
    aspect = rng.uniform(0.5, 2.0)
    reg_h = _align8(max(JPEG_BLOCK * 4, int((target_area / aspect) ** 0.5)))
    reg_w = _align8(max(JPEG_BLOCK * 4, int(reg_h * aspect)))
    reg_h = _align8(min(reg_h, h))
    reg_w = _align8(min(reg_w, w))
    if reg_w < JPEG_BLOCK * 4 or reg_h < JPEG_BLOCK * 4:
        return None

    src_x = _align8(rng.randint(0, max(0, w - reg_w)))
    src_y = _align8(rng.randint(0, max(0, h - reg_h)))
    region = img.crop((src_x, src_y, src_x + reg_w, src_y + reg_h))

    for _ in range(20):
        dst_x = _align8(rng.randint(0, max(0, w - reg_w)))
        dst_y = _align8(rng.randint(0, max(0, h - reg_h)))
        if abs(dst_x - src_x) > reg_w or abs(dst_y - src_y) > reg_h:
            break
    else:
        dst_x = _align8((src_x + w // 2) % max(1, w - reg_w))
        dst_y = _align8((src_y + h // 2) % max(1, h - reg_h))

    composite = img.copy()
    composite.paste(region, (dst_x, dst_y))
    save_jpeg_with_quality(composite, output_path, q_final)

    actual_fraction = round((reg_w * reg_h) / area, 4)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": "copy_move",
        "background_source": str(src_path),
        "foreground_source": str(src_path),
        "q_bg_true": q,
        "q_fg_true": q,
        "q_final": q_final,
        "q_delta": 0,
        "bbox": [dst_x, dst_y, dst_x + reg_w, dst_y + reg_h],
        "paste_area_fraction": actual_fraction,
        "bg_size": [w, h],
        "aligned_8px": True,
    }


# ---------------------------------------------------------------------------
# Authentic controls
# ---------------------------------------------------------------------------

def save_authentic_plain(
    src_path: Path,
    output_path: Path,
    q_save: int | None = None,
) -> dict:
    if q_save is None:
        shutil.copy2(src_path, output_path)
        category = "plain"
        q_actual = None
    else:
        img = Image.open(src_path).convert("RGB")
        save_jpeg_with_quality(img, output_path, q_save)
        q_actual = q_save
        if q_save >= 90:
            category = "high_quality"
        elif q_save <= 50:
            category = "heavy_compression"
        else:
            category = "plain"
    return {
        "filename": output_path.name,
        "label": "authentic",
        "control_type": category,
        "source": str(src_path),
        "quality": q_actual,
    }


def save_authentic_multicompressed(
    src_path: Path,
    output_path: Path,
    rng: random.Random,
) -> dict:
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
        "source": str(src_path),
        "compression_rounds": n_rounds,
        "quality_chain": round_qs,
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def build_corpus(corpus_dir: Path, output_dir: Path, count: int, seed: int) -> None:
    rng = random.Random(seed)
    scale = count / 150.0

    n_small    = max(1, round(25 * scale))
    n_medium   = max(1, round(20 * scale))
    n_large    = max(1, round(15 * scale))
    n_copymove = max(1, round(15 * scale))
    n_spliced  = n_small + n_medium + n_large + n_copymove

    n_multicomp = max(1, round(20 * scale))
    n_high_q    = max(1, round(10 * scale))
    n_heavy_q   = max(1, round(10 * scale))
    n_plain     = max(1, round(35 * scale))
    n_authentic = n_multicomp + n_high_q + n_heavy_q + n_plain

    total = n_spliced + n_authentic

    print(f"\nCorpus plan v2 ({total} images):")
    print(f"  Spliced:   {n_spliced} (small={n_small}, medium={n_medium}, large={n_large}, copy-move={n_copymove})")
    print(f"  Authentic: {n_authentic} (multi-comp={n_multicomp}, hi-Q={n_high_q}, lo-Q={n_heavy_q}, plain={n_plain})")
    print(f"  Final save qualities: {FINAL_SAVE_QUALITIES} (HIGH — preserves source Q footprints)")
    print(f"  BG source qualities:  {BG_SOURCE_QUALITIES}")
    print(f"  FG source qualities:  {FG_SOURCE_QUALITIES}")

    images = load_cc_by_images(corpus_dir, rng)
    print("  Filtering source pool for usability...")
    usable = [p for p in images if is_usable(p)]
    print(f"  Usable: {len(usable)} / {len(images)}")
    if len(usable) < total * 2:
        print(f"  WARNING: usable pool ({len(usable)}) < 2× corpus size ({total}).", file=sys.stderr)

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

    def make_splice(size_cat: str, idx: int, total_cat: int) -> None:
        bg_path, fg_path = draw(2)
        name = f"splice_{size_cat}_{idx:03d}.jpg"
        out_path = spliced_dir / name
        print(f"  [{idx+1}/{total_cat}] splice/{size_cat} {name} ...", end=" ", flush=True)
        label = generate_splice(bg_path, fg_path, size_cat, rng, out_path)
        if label:
            labels.append(label)
            print(f"ok (Δ={label['q_delta']}, q_final={label['q_final']})")
        else:
            errors.append(name)
            print("FAILED")

    print(f"\nGenerating {n_small} small-region splices ...")
    for i in range(n_small):
        make_splice("small", i, n_small)
    print(f"\nGenerating {n_medium} medium-region splices ...")
    for i in range(n_medium):
        make_splice("medium", i, n_medium)
    print(f"\nGenerating {n_large} large-region splices ...")
    for i in range(n_large):
        make_splice("large", i, n_large)

    print(f"\nGenerating {n_copymove} copy-move images ...")
    for i in range(n_copymove):
        src_path = draw()[0]
        name = f"splice_copy_move_{i:03d}.jpg"
        out_path = spliced_dir / name
        print(f"  [{i+1}/{n_copymove}] splice/copy_move {name} ...", end=" ", flush=True)
        label = generate_copy_move(src_path, rng, out_path)
        if label:
            labels.append(label)
            print("ok")
        else:
            errors.append(name)
            print("FAILED")

    print(f"\nGenerating {n_multicomp} multi-compression authentic controls ...")
    for i in range(n_multicomp):
        src_path = draw()[0]
        name = f"auth_multicomp_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_multicomp}] auth/multi-comp {name} ...", end=" ", flush=True)
        labels.append(save_authentic_multicompressed(src_path, out_path, rng))
        print("ok")

    print(f"\nGenerating {n_high_q} high-quality authentic controls (Q≥90) ...")
    for i in range(n_high_q):
        src_path = draw()[0]
        q = rng.choice([90, 92, 95])
        name = f"auth_highq_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_high_q}] auth/high-Q {name} (Q={q}) ...", end=" ", flush=True)
        labels.append(save_authentic_plain(src_path, out_path, q_save=q))
        print("ok")

    print(f"\nGenerating {n_heavy_q} heavily-compressed authentic controls (Q≤50) ...")
    for i in range(n_heavy_q):
        src_path = draw()[0]
        q = rng.choice([40, 45, 50])
        name = f"auth_heavyq_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_heavy_q}] auth/heavy-Q {name} (Q={q}) ...", end=" ", flush=True)
        labels.append(save_authentic_plain(src_path, out_path, q_save=q))
        print("ok")

    print(f"\nGenerating {n_plain} plain baseline authentic controls ...")
    for i in range(n_plain):
        src_path = draw()[0]
        name = f"auth_plain_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_plain}] auth/plain {name} ...", end=" ", flush=True)
        labels.append(save_authentic_plain(src_path, out_path))
        print("ok")

    summary = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "generator_version": "v2",
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
            "subsampling": "4:2:0",
        },
        "corpus_breakdown": {
            "splice_small": n_small,
            "splice_medium": n_medium,
            "splice_large": n_large,
            "splice_copy_move": n_copymove,
            "auth_multi_compression": n_multicomp,
            "auth_high_quality": n_high_q,
            "auth_heavy_compression": n_heavy_q,
            "auth_plain": n_plain,
        },
        "licence_note": (
            "All source images are from COCO (CC-BY 4.0) and Flickr30k "
            "(non-commercial cleared under Option C). Generated composites "
            "are derivative works under CC-BY. CASIA v1/v2 and other "
            "research-only datasets are NOT used — Jura Trace is a "
            "commercial product."
        ),
        "images": labels,
    }

    labels_path = output_dir / "labels.json"
    labels_path.write_text(json.dumps(summary, indent=2))
    print(f"\nLabels written: {labels_path}")
    if errors:
        print(f"\nWARNING: {len(errors)} images failed: {errors}", file=sys.stderr)

    n_spliced_actual = sum(1 for l in labels if l["label"] == "spliced")
    n_auth_actual = sum(1 for l in labels if l["label"] == "authentic")
    print(f"\nCorpus v2 complete: {len(labels)} images ({n_spliced_actual} spliced, {n_auth_actual} authentic)")
    print(f"Output: {output_dir}")


def main() -> None:
    default_corpus = "/Volumes/MAC SSD/Training Data/corpus/training/authentic"
    default_output = Path(__file__).parent.parent / "models" / "splice_calibration_150_v2"

    parser = argparse.ArgumentParser(
        description="Build synthetic splice test corpus v2 (S28-FU9 follow-up)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument("--corpus", type=Path, default=default_corpus)
    parser.add_argument("--output", type=Path, default=default_output)
    parser.add_argument("--count", type=int, default=150)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    if args.count < 8:
        print("Error: --count must be at least 8.", file=sys.stderr)
        sys.exit(1)
    if not args.corpus.exists():
        print(f"Error: corpus directory not found: {args.corpus}", file=sys.stderr)
        sys.exit(1)

    print("Jura Trace — Synthetic Splice Corpus Builder v2")
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
