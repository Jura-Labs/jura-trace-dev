#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Synthetic Splice Corpus Builder

Generates a controlled 150-image splice test set from CC-BY authentic JPEG
sources for JPEG Ghost weight calibration (S28-FU9).

Licence constraint: uses only COCO (CC-BY 4.0), Flickr30k (research licence
with non-commercial clearance for this corpus only), and COCO-train (CC-BY 4.0)
sources from the USB training corpus. Flickr30k is treated as CC-BY-compatible
for this internal non-commercial calibration under the Option C corpus strategy
(docs/decisions/option-c-corpus-strategy.md).

Output: models/splice_calibration_150/
  spliced/      — synthetic forgeries with JPEG ghost patterns
  authentic/    — control images (multi-compression + quality variants)
  labels.json   — ground-truth metadata

Usage:
    python scripts/build_splice_corpus.py --corpus /path/to/authentic/
    python scripts/build_splice_corpus.py --corpus /path/to/authentic/ --seed 42 --count 150
    python scripts/build_splice_corpus.py --corpus /path/to/authentic/ --count 30  # pilot run
"""

import argparse
import json
import os
import random
import sys
import io
from datetime import datetime, timezone
from pathlib import Path

from PIL import Image
import numpy as np

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# CC-BY-compatible source subdirectories on the USB corpus.
# Flickr30k: non-commercial use cleared under Option C corpus strategy.
CC_BY_DIRS = ["coco", "coco_extra", "coco_extra2", "coco_train", "flickr30k"]

# Minimum image dimension (pixels) to be usable as splice source or target
MIN_DIMENSION = 256

# JPEG quality levels for final re-save of spliced images
SAVE_QUALITIES = [60, 70, 80, 90]

# Splice region size categories as fraction of image area
SPLICE_SMALL_MAX = 0.10   # < 10% area
SPLICE_MEDIUM_MIN = 0.10  # 10-30%
SPLICE_MEDIUM_MAX = 0.30
SPLICE_LARGE_MIN = 0.30   # > 30%

# Quality levels to use for simulating source JPEG compression history.
# These are the "baked-in" qualities of the original JPEGs; actual file
# quality is estimated by PIL reload at different Q values.
SOURCE_QUALITIES = [55, 65, 75, 85, 95]

# Minimum Q-delta between background and pasted region (for splice images)
MIN_Q_DELTA = 20


# ---------------------------------------------------------------------------
# Corpus loading
# ---------------------------------------------------------------------------

def load_cc_by_images(corpus_dir: Path, rng: random.Random) -> list[Path]:
    """
    Load JPEG file paths from CC-BY-compatible subdirectories.
    Warns if fewer than 200 images found (threshold from task spec).
    """
    images: list[Path] = []
    missing_dirs: list[str] = []

    for subdir in CC_BY_DIRS:
        d = corpus_dir / subdir
        if not d.exists():
            missing_dirs.append(subdir)
            continue
        found = sorted(d.glob("*.jpg")) + sorted(d.glob("*.jpeg")) + sorted(d.glob("*.JPG"))
        images.extend(found)

    if missing_dirs:
        print(f"  WARNING: CC-BY subdirs not found: {missing_dirs}", file=sys.stderr)

    # Shuffle for deterministic random selection
    rng.shuffle(images)

    if len(images) < 200:
        print(
            f"  ERROR: Only {len(images)} CC-BY images found in {corpus_dir}. "
            "Expected ≥200. Cannot build a representative splice corpus. "
            "Check --corpus path and CC-BY subdirectory availability.",
            file=sys.stderr,
        )
        sys.exit(1)

    print(f"  CC-BY source pool: {len(images)} images from {len(CC_BY_DIRS) - len(missing_dirs)} dirs")
    return images


def is_usable(img_path: Path) -> bool:
    """Quick check: image can be opened and is large enough."""
    try:
        with Image.open(img_path) as img:
            w, h = img.size
            return w >= MIN_DIMENSION and h >= MIN_DIMENSION
    except Exception:
        return False


# ---------------------------------------------------------------------------
# JPEG quality estimation
# ---------------------------------------------------------------------------

def estimate_jpeg_quality(img_path: Path) -> int:
    """
    Estimate the JPEG compression quality of a file using PIL's quantisation
    tables. Returns an integer in [50, 95]. Falls back to 75 if unavailable.
    """
    try:
        with Image.open(img_path) as img:
            qtables = img.quantization
            if not qtables:
                return 75
            # Standard luminance table at Q=50
            _LUM_Q50 = [
                16, 11, 10, 16, 24, 40, 51, 61,
                12, 12, 14, 19, 26, 58, 60, 55,
                14, 13, 16, 24, 40, 57, 69, 56,
                14, 17, 22, 29, 51, 87, 80, 62,
                18, 22, 37, 56, 68, 109, 103, 77,
                24, 35, 55, 64, 81, 104, 113, 92,
                49, 64, 78, 87, 103, 121, 120, 101,
                72, 92, 95, 98, 112, 100, 103, 99,
            ]
            if 0 in qtables:
                actual = list(qtables[0].values())[:8]
                lum = _LUM_Q50[:8]
                # Scale factor S = sum(lum) / sum(actual) maps to Q roughly
                ratio = sum(lum) / max(sum(actual), 1)
                if ratio >= 1:
                    q = int(50 + 50 * (ratio - 1))
                else:
                    q = int(50 * ratio)
                return max(50, min(95, q))
    except Exception:
        pass
    return 75


# ---------------------------------------------------------------------------
# Splice generation
# ---------------------------------------------------------------------------

def generate_splice(
    bg_path: Path,
    fg_path: Path,
    size_category: str,  # "small" | "medium" | "large"
    rng: random.Random,
    q_save: int,
    output_path: Path,
) -> dict | None:
    """
    Paste a rectangular region from fg into bg and save as JPEG.

    Returns a label dict or None on failure.
    """
    try:
        bg_img = Image.open(bg_path).convert("RGB")
        fg_img = Image.open(fg_path).convert("RGB")
    except Exception as e:
        print(f"    skip (open error): {e}", file=sys.stderr)
        return None

    bg_w, bg_h = bg_img.size
    fg_w, fg_h = fg_img.size

    bg_area = bg_w * bg_h

    # Determine target splice area based on category
    if size_category == "small":
        target_fraction = rng.uniform(0.02, SPLICE_SMALL_MAX)
    elif size_category == "medium":
        target_fraction = rng.uniform(SPLICE_MEDIUM_MIN, SPLICE_MEDIUM_MAX)
    else:  # large
        target_fraction = rng.uniform(SPLICE_LARGE_MIN, 0.60)

    target_area = int(bg_area * target_fraction)
    aspect = rng.uniform(0.5, 2.0)
    paste_h = max(32, int((target_area / aspect) ** 0.5))
    paste_w = max(32, int(paste_h * aspect))

    # Clamp to both images
    paste_h = min(paste_h, bg_h, fg_h)
    paste_w = min(paste_w, bg_w, fg_w)

    # Random source crop in fg
    src_x = rng.randint(0, fg_w - paste_w)
    src_y = rng.randint(0, fg_h - paste_h)
    fg_crop = fg_img.crop((src_x, src_y, src_x + paste_w, src_y + paste_h))

    # Pre-compress fg_crop at a quality different from bg (simulate Q history).
    # Choose fg_q so that |bg_q - fg_q| >= MIN_Q_DELTA.
    bg_q = estimate_jpeg_quality(bg_path)
    candidate_fq = [q for q in SOURCE_QUALITIES if abs(q - bg_q) >= MIN_Q_DELTA]
    if not candidate_fq:
        # Fallback: pick furthest available
        candidate_fq = sorted(SOURCE_QUALITIES, key=lambda q: -abs(q - bg_q))[:2]
    fg_q = rng.choice(candidate_fq)

    # Bake fg_q into the crop by round-tripping through JPEG
    buf = io.BytesIO()
    fg_crop.save(buf, format="JPEG", quality=fg_q)
    buf.seek(0)
    fg_crop_recompressed = Image.open(buf).convert("RGB")

    # Random paste location in bg
    dst_x = rng.randint(0, bg_w - paste_w)
    dst_y = rng.randint(0, bg_h - paste_h)

    composite = bg_img.copy()
    composite.paste(fg_crop_recompressed, (dst_x, dst_y))

    # Final JPEG save at q_save
    composite.save(output_path, format="JPEG", quality=q_save)

    actual_area_fraction = round((paste_w * paste_h) / bg_area, 4)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": size_category,
        "background_source": str(bg_path),
        "foreground_source": str(fg_path),
        "bg_quality_estimate": bg_q,
        "fg_quality_baked": fg_q,
        "q_save": q_save,
        "q_delta": abs(bg_q - fg_q),
        "bbox": [dst_x, dst_y, dst_x + paste_w, dst_y + paste_h],
        "paste_area_fraction": actual_area_fraction,
        "bg_size": [bg_w, bg_h],
    }


def generate_copy_move(
    src_path: Path,
    rng: random.Random,
    q_save: int,
    output_path: Path,
) -> dict | None:
    """
    Copy a region from the same image and paste it elsewhere.
    Q-delta = 0 (same source). JPEG Ghost should score near 0 on these.
    """
    try:
        img = Image.open(src_path).convert("RGB")
    except Exception as e:
        print(f"    skip (open error): {e}", file=sys.stderr)
        return None

    w, h = img.size
    area = w * h

    # Medium-sized region (10-30%)
    target_fraction = rng.uniform(0.10, 0.30)
    target_area = int(area * target_fraction)
    aspect = rng.uniform(0.5, 2.0)
    reg_h = max(32, int((target_area / aspect) ** 0.5))
    reg_w = max(32, int(reg_h * aspect))
    reg_h = min(reg_h, h)
    reg_w = min(reg_w, w)

    src_x = rng.randint(0, w - reg_w)
    src_y = rng.randint(0, h - reg_h)
    region = img.crop((src_x, src_y, src_x + reg_w, src_y + reg_h))

    # Paste to non-overlapping destination
    for _ in range(20):
        dst_x = rng.randint(0, w - reg_w)
        dst_y = rng.randint(0, h - reg_h)
        # Check no overlap with source
        if abs(dst_x - src_x) > reg_w or abs(dst_y - src_y) > reg_h:
            break
    else:
        dst_x = (src_x + w // 2) % (w - reg_w)
        dst_y = (src_y + h // 2) % (h - reg_h)

    composite = img.copy()
    composite.paste(region, (dst_x, dst_y))
    composite.save(output_path, format="JPEG", quality=q_save)

    bg_q = estimate_jpeg_quality(src_path)
    actual_fraction = round((reg_w * reg_h) / area, 4)

    return {
        "filename": output_path.name,
        "label": "spliced",
        "splice_type": "copy_move",
        "background_source": str(src_path),
        "foreground_source": str(src_path),  # same image
        "bg_quality_estimate": bg_q,
        "fg_quality_baked": bg_q,   # no re-compression; same source Q
        "q_save": q_save,
        "q_delta": 0,
        "bbox": [dst_x, dst_y, dst_x + reg_w, dst_y + reg_h],
        "paste_area_fraction": actual_fraction,
        "bg_size": [w, h],
    }


# ---------------------------------------------------------------------------
# Authentic control generation
# ---------------------------------------------------------------------------

def save_authentic_plain(
    src_path: Path,
    output_path: Path,
    q_save: int | None = None,
) -> dict:
    """
    Copy authentic image to output, optionally re-saving at a specific quality.
    """
    if q_save is None:
        # Simple copy preserving original JPEG stream
        import shutil
        shutil.copy2(src_path, output_path)
        q_actual = estimate_jpeg_quality(src_path)
        category = "plain"
    else:
        img = Image.open(src_path).convert("RGB")
        img.save(output_path, format="JPEG", quality=q_save)
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
    """
    Simulate a social-media repost by passing through 2-3 JPEG recompressions
    at varying quality levels (WhatsApp ~85, Twitter ~75, etc.).
    """
    qualities = [85, 80, 75, 70, 85, 80]
    n_rounds = rng.randint(2, 3)
    round_qs = rng.choices(qualities, k=n_rounds)

    img = Image.open(src_path).convert("RGB")
    for q in round_qs:
        buf = io.BytesIO()
        img.save(buf, format="JPEG", quality=q)
        buf.seek(0)
        img = Image.open(buf).convert("RGB")

    img.save(output_path, format="JPEG", quality=round_qs[-1])

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

def build_corpus(
    corpus_dir: Path,
    output_dir: Path,
    count: int,
    seed: int,
) -> None:
    """
    Build the splice calibration corpus.

    Corpus breakdown (for count=150):
      SPLICED (75 total):
        - small region (< 10% area), Q-delta >= 20:   25
        - medium region (10-30%), Q-delta >= 20:       20
        - large region (>30%), Q-delta >= 20:          15
        - copy-move only (Q-delta = 0):                15
      AUTHENTIC controls (75 total):
        - multi-compression repost simulations:        20
        - high-quality (Q >= 90):                      10
        - heavy-compression (Q <= 50):                 10
        - plain baseline:                              35

    Scales proportionally when count != 150.
    """
    rng = random.Random(seed)

    # Scale counts proportionally from 150 baseline
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

    print(f"\nCorpus plan ({total} images):")
    print(f"  Spliced:   {n_spliced} (small={n_small}, medium={n_medium}, large={n_large}, copy-move={n_copymove})")
    print(f"  Authentic: {n_authentic} (multi-comp={n_multicomp}, hi-Q={n_high_q}, lo-Q={n_heavy_q}, plain={n_plain})")

    # Load source pool
    images = load_cc_by_images(corpus_dir, rng)

    # Filter to usable images
    print("  Filtering source pool for usability...")
    usable = [p for p in images if is_usable(p)]
    print(f"  Usable: {len(usable)} / {len(images)}")
    if len(usable) < total * 2:
        print(
            f"  WARNING: usable pool ({len(usable)}) is less than 2× corpus size ({total}). "
            "Image reuse will occur across categories.",
            file=sys.stderr,
        )

    # Set up output dirs
    spliced_dir = output_dir / "spliced"
    authentic_dir = output_dir / "authentic"
    spliced_dir.mkdir(parents=True, exist_ok=True)
    authentic_dir.mkdir(parents=True, exist_ok=True)

    labels: list[dict] = []
    errors: list[str] = []

    # We maintain separate pools to minimise image reuse within splice pairs.
    pool = list(usable)

    def draw(n: int = 1) -> list[Path]:
        """Draw n images from pool (cycling if exhausted)."""
        drawn = []
        for _ in range(n):
            if not pool:
                pool.extend(usable)
                rng.shuffle(pool)
            drawn.append(pool.pop())
        return drawn

    # ── Spliced images ────────────────────────────────────────────────────

    def make_splice(size_cat: str, idx: int, total_cat: int) -> None:
        bg_path, fg_path = draw(2)
        q_save = rng.choice(SAVE_QUALITIES)
        name = f"splice_{size_cat}_{idx:03d}.jpg"
        out_path = spliced_dir / name
        print(f"  [{idx+1}/{total_cat}] splice/{size_cat} {name} ...", end=" ", flush=True)
        label = generate_splice(bg_path, fg_path, size_cat, rng, q_save, out_path)
        if label:
            labels.append(label)
            print("ok")
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
        q_save = rng.choice(SAVE_QUALITIES)
        name = f"splice_copy_move_{i:03d}.jpg"
        out_path = spliced_dir / name
        print(f"  [{i+1}/{n_copymove}] splice/copy_move {name} ...", end=" ", flush=True)
        label = generate_copy_move(src_path, rng, q_save, out_path)
        if label:
            labels.append(label)
            print("ok")
        else:
            errors.append(name)
            print("FAILED")

    # ── Authentic controls ────────────────────────────────────────────────

    print(f"\nGenerating {n_multicomp} multi-compression authentic controls ...")
    for i in range(n_multicomp):
        src_path = draw()[0]
        name = f"auth_multicomp_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_multicomp}] auth/multi-comp {name} ...", end=" ", flush=True)
        label = save_authentic_multicompressed(src_path, out_path, rng)
        labels.append(label)
        print("ok")

    print(f"\nGenerating {n_high_q} high-quality authentic controls (Q≥90) ...")
    for i in range(n_high_q):
        src_path = draw()[0]
        q = rng.choice([90, 92, 95])
        name = f"auth_highq_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_high_q}] auth/high-Q {name} (Q={q}) ...", end=" ", flush=True)
        label = save_authentic_plain(src_path, out_path, q_save=q)
        labels.append(label)
        print("ok")

    print(f"\nGenerating {n_heavy_q} heavily-compressed authentic controls (Q≤50) ...")
    for i in range(n_heavy_q):
        src_path = draw()[0]
        q = rng.choice([40, 45, 50])
        name = f"auth_heavyq_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_heavy_q}] auth/heavy-Q {name} (Q={q}) ...", end=" ", flush=True)
        label = save_authentic_plain(src_path, out_path, q_save=q)
        labels.append(label)
        print("ok")

    print(f"\nGenerating {n_plain} plain baseline authentic controls ...")
    for i in range(n_plain):
        src_path = draw()[0]
        name = f"auth_plain_{i:03d}.jpg"
        out_path = authentic_dir / name
        print(f"  [{i+1}/{n_plain}] auth/plain {name} ...", end=" ", flush=True)
        label = save_authentic_plain(src_path, out_path)
        labels.append(label)
        print("ok")

    # ── Write labels.json ─────────────────────────────────────────────────

    summary = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "seed": seed,
        "count_requested": count,
        "count_generated": len(labels),
        "errors": errors,
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
            "(non-commercial cleared under Option C corpus strategy). "
            "Generated composites are derivative works under CC-BY. "
            "Compatible with PolyForm Noncommercial and Option C production model strategy."
        ),
        "images": labels,
    }

    labels_path = output_dir / "labels.json"
    labels_path.write_text(json.dumps(summary, indent=2))
    print(f"\nLabels written: {labels_path}")

    if errors:
        print(f"\nWARNING: {len(errors)} images failed to generate: {errors}", file=sys.stderr)

    n_spliced_actual = sum(1 for l in labels if l["label"] == "spliced")
    n_auth_actual = sum(1 for l in labels if l["label"] == "authentic")
    print(f"\nCorpus complete: {len(labels)} images ({n_spliced_actual} spliced, {n_auth_actual} authentic)")
    print(f"Output: {output_dir}")


def main() -> None:
    default_corpus = (
        "/Volumes/MAC SSD/Training Data/corpus/training/authentic"
    )
    default_output = Path(__file__).parent.parent / "models" / "splice_calibration_150"

    parser = argparse.ArgumentParser(
        description="Build synthetic splice test corpus for JPEG Ghost weight calibration (S28-FU9)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        default=default_corpus,
        help=f"Path to authentic training corpus directory (default: {default_corpus})",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=default_output,
        help=f"Output directory for generated corpus (default: {default_output})",
    )
    parser.add_argument(
        "--count",
        type=int,
        default=150,
        help="Total images to generate (default: 150; scales 75/75 splice/authentic ratio)",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed for reproducibility (default: 42)",
    )
    args = parser.parse_args()

    if args.count < 8:
        print("Error: --count must be at least 8 (minimum viable corpus per category).", file=sys.stderr)
        sys.exit(1)

    if not args.corpus.exists():
        print(f"Error: corpus directory not found: {args.corpus}", file=sys.stderr)
        print("Mount the USB drive and verify the path.", file=sys.stderr)
        sys.exit(1)

    print("Jura Trace — Synthetic Splice Corpus Builder (S28-FU9)")
    print(f"  Corpus:  {args.corpus}")
    print(f"  Output:  {args.output}")
    print(f"  Count:   {args.count}")
    print(f"  Seed:    {args.seed}")

    start = datetime.now()
    build_corpus(
        corpus_dir=args.corpus,
        output_dir=args.output,
        count=args.count,
        seed=args.seed,
    )
    elapsed = (datetime.now() - start).total_seconds()
    print(f"\nCompleted in {elapsed:.1f}s")


if __name__ == "__main__":
    main()
