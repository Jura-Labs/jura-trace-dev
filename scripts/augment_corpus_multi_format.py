#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
augment_corpus_multi_format.py — generate format-augmented training samples
for the UnivFD v10 multi-format retrain (PNG + TIFF + WebP + HEIC).

WHY: UnivFD v9 was trained on a ~95% JPEG corpus. The CLIP ViT-B/32 LogReg
probe boundary does not generalise to non-JPEG embedding distributions, so
AI images re-encoded as PNG/TIFF/HEIC pass as authentic. This script
produces augmentation samples to retrain v10 with multi-format coverage.

See: docs/calibration/univfd-v10-multi-format-augmentation-plan.md

USAGE:
    # AI corpus augmentation (generate ~1,500 samples per format)
    python3 scripts/augment_corpus_multi_format.py \\
        --source "/Volumes/MAC SSD/Training Data/corpus/training/ai_generated" \\
        --out    "/Volumes/MAC SSD/Training Data/corpus/training_multi_format/ai_generated" \\
        --formats tiff,webp,heic \\
        --per-format 1500

    # Authentic top-up (~500 per format)
    python3 scripts/augment_corpus_multi_format.py \\
        --source "/Volumes/MAC SSD/Training Data/corpus/training/authentic" \\
        --out    "/Volumes/MAC SSD/Training Data/corpus/training_multi_format/authentic" \\
        --formats tiff,webp,heic \\
        --per-format 500

OUTPUT NAMING:
    {original_stem}_fmt_{format}.{ext}
    e.g. midjourney_v6_00042_fmt_tiff.tif

IDEMPOTENCY:
    Manifest at {out}/manifest_{format}.json — re-runs skip existing outputs.
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import hashlib
import json
import logging
import os
import random
import subprocess
import sys
from pathlib import Path
from typing import Optional

from PIL import Image

# ── Format configuration ──────────────────────────────────────────────
# Each entry: (output extension, generator function)
# HEIC is special: macOS sips required, no Pillow encoder.

FORMAT_EXTENSIONS = {
    "png": "png",
    "tiff": "tif",
    "webp": "webp",
    "heic": "heic",
}

SOURCE_EXTENSIONS = {".jpg", ".jpeg", ".JPG", ".JPEG"}

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
    datefmt="%H:%M:%S",
)


# ── Format generators ─────────────────────────────────────────────────


def generate_png(src: Path, dst: Path) -> None:
    """PNG: lossless re-encode. Pillow only."""
    with Image.open(src) as img:
        img.save(dst, format="PNG", optimize=False)


def generate_tiff(src: Path, dst: Path) -> None:
    """TIFF: LZW lossless compression. Pillow only."""
    with Image.open(src) as img:
        img.save(dst, format="TIFF", compression="lzw")


def generate_webp(src: Path, dst: Path) -> None:
    """WebP: lossy q=80, method=6 (slowest/best). Pillow + libwebp."""
    with Image.open(src) as img:
        img.save(dst, format="WEBP", quality=80, method=6)


def generate_heic(src: Path, dst: Path) -> None:
    """HEIC: macOS sips (Apple-licensed AVFoundation codec)."""
    result = subprocess.run(
        ["sips", "-s", "format", "heic", "-s", "formatOptions", "80",
         str(src), "--out", str(dst)],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise RuntimeError(f"sips HEIC conversion failed: {result.stderr.strip()}")


GENERATORS = {
    "png": generate_png,
    "tiff": generate_tiff,
    "webp": generate_webp,
    "heic": generate_heic,
}


# ── Manifest ──────────────────────────────────────────────────────────


def sha256_file(path: Path) -> str:
    """Compute SHA-256 of a file."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def load_manifest(path: Path) -> dict:
    if path.exists():
        try:
            return json.loads(path.read_text())
        except Exception as e:
            logging.warning("Could not read existing manifest %s: %s", path, e)
    return {"entries": {}}


def write_manifest(path: Path, manifest: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest, indent=2, sort_keys=True))


# ── Stratified sampling ───────────────────────────────────────────────


def collect_sources_stratified(root: Path, target_count: int, seed: int = 42) -> list[Path]:
    """
    Sample target_count JPEGs from the corpus, stratified by immediate
    subdirectory (generator family for AI corpus, dataset name for authentic).

    Preserves the per-source proportional distribution.
    """
    rng = random.Random(seed)
    by_subdir: dict[str, list[Path]] = {}

    for sub in sorted(root.iterdir()):
        if not sub.is_dir():
            continue
        files = [
            f for f in sub.rglob("*")
            if f.is_file() and f.suffix in SOURCE_EXTENSIONS
        ]
        if files:
            by_subdir[sub.name] = files

    if not by_subdir:
        # Flat directory — sample directly
        all_files = [
            f for f in root.rglob("*")
            if f.is_file() and f.suffix in SOURCE_EXTENSIONS
        ]
        rng.shuffle(all_files)
        return all_files[:target_count]

    total_available = sum(len(v) for v in by_subdir.values())
    logging.info(
        "Stratification: %d subdirs, %d total source files, target %d",
        len(by_subdir), total_available, target_count,
    )

    if total_available < target_count:
        logging.warning(
            "Source corpus has %d files, target %d — will use all available",
            total_available, target_count,
        )
        result = []
        for files in by_subdir.values():
            result.extend(files)
        return result

    # Proportional sampling per subdir
    sampled: list[Path] = []
    for subname, files in by_subdir.items():
        share = round(target_count * len(files) / total_available)
        share = min(share, len(files))
        rng.shuffle(files)
        chosen = files[:share]
        sampled.extend(chosen)
        logging.info("  %s: %d sampled (of %d available)", subname, len(chosen), len(files))

    # Trim or top up to exactly target_count
    rng.shuffle(sampled)
    if len(sampled) > target_count:
        sampled = sampled[:target_count]
    elif len(sampled) < target_count:
        # Pull extras from largest subdir
        extras_needed = target_count - len(sampled)
        all_remaining = []
        for files in by_subdir.values():
            for f in files:
                if f not in sampled:
                    all_remaining.append(f)
        rng.shuffle(all_remaining)
        sampled.extend(all_remaining[:extras_needed])

    return sampled


# ── Worker ────────────────────────────────────────────────────────────


def convert_one(src: Path, out_dir: Path, fmt: str, manifest: dict) -> Optional[str]:
    """
    Convert one source to the target format. Returns the manifest key on
    success, or None if skipped/failed.
    """
    ext = FORMAT_EXTENSIONS[fmt]
    out_name = f"{src.stem}_fmt_{fmt}.{ext}"
    dst = out_dir / out_name

    # Idempotency: skip if already in manifest AND file exists
    key = out_name
    if key in manifest["entries"] and dst.exists():
        return None  # silently skipped

    try:
        dst.parent.mkdir(parents=True, exist_ok=True)
        GENERATORS[fmt](src, dst)
    except Exception as e:
        logging.warning("Conversion failed (%s → %s): %s", src.name, fmt, e)
        return None

    if not dst.exists() or dst.stat().st_size == 0:
        logging.warning("Output empty/missing for %s", out_name)
        return None

    manifest["entries"][key] = {
        "source": str(src),
        "source_sha256": sha256_file(src)[:16],
        "output_sha256": sha256_file(dst)[:16],
        "size_bytes": dst.stat().st_size,
    }
    return key


# ── Main ──────────────────────────────────────────────────────────────


def run_format(source_root: Path, out_root: Path, fmt: str, per_format: int,
               workers: int) -> tuple[int, int]:
    """Run augmentation for one format. Returns (success_count, skipped_count)."""
    out_dir = out_root / fmt
    manifest_path = out_dir / f"manifest_{fmt}.json"
    manifest = load_manifest(manifest_path)

    existing = sum(
        1 for k in manifest["entries"]
        if (out_dir / k).exists()
    )
    remaining = per_format - existing
    if remaining <= 0:
        logging.info("[%s] target %d already met (%d existing) — nothing to do",
                     fmt, per_format, existing)
        return 0, existing

    logging.info("[%s] target %d, existing %d, generating %d more",
                 fmt, per_format, existing, remaining)

    sources = collect_sources_stratified(source_root, remaining, seed=42 + hash(fmt) % 1000)

    # HEIC uses subprocess (sips); cap workers at 2 to avoid sips contention
    effective_workers = 2 if fmt == "heic" else workers

    success = 0
    failed = 0
    with cf.ProcessPoolExecutor(max_workers=effective_workers) as pool:
        futures = {pool.submit(convert_one, src, out_dir, fmt, manifest): src
                   for src in sources}
        for i, future in enumerate(cf.as_completed(futures), 1):
            try:
                result = future.result()
                if result:
                    success += 1
                else:
                    failed += 1
            except Exception as e:
                logging.warning("Worker exception: %s", e)
                failed += 1
            if i % 100 == 0:
                logging.info("[%s] progress %d/%d (success %d, failed %d)",
                             fmt, i, len(futures), success, failed)

    write_manifest(manifest_path, manifest)
    logging.info("[%s] done: %d new, %d failed/skipped, manifest at %s",
                 fmt, success, failed, manifest_path)
    return success, failed


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--source", required=True, type=Path,
                    help="Source corpus root (e.g. ai_generated/)")
    ap.add_argument("--out", required=True, type=Path,
                    help="Output corpus root")
    ap.add_argument("--formats", default="png,tiff,webp,heic",
                    help="Comma-separated list of formats to generate")
    ap.add_argument("--per-format", type=int, default=1500,
                    help="Target sample count per format (default 1500)")
    ap.add_argument("--workers", type=int, default=4,
                    help="Parallel workers (default 4; HEIC capped at 2)")
    args = ap.parse_args()

    if not args.source.is_dir():
        logging.error("Source root does not exist: %s", args.source)
        return 2

    requested = [f.strip().lower() for f in args.formats.split(",")]
    invalid = [f for f in requested if f not in FORMAT_EXTENSIONS]
    if invalid:
        logging.error("Unknown formats: %s. Supported: %s",
                      invalid, list(FORMAT_EXTENSIONS.keys()))
        return 2

    args.out.mkdir(parents=True, exist_ok=True)

    total_new = 0
    for fmt in requested:
        logging.info("=" * 60)
        logging.info("Generating format: %s", fmt)
        logging.info("=" * 60)
        new, _ = run_format(args.source, args.out, fmt, args.per_format, args.workers)
        total_new += new

    logging.info("=" * 60)
    logging.info("ALL DONE. Total new samples generated: %d", total_new)
    logging.info("Output root: %s", args.out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
