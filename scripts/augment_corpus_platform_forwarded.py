#!/usr/bin/env python3
"""
Jura Trace -- Platform-Forwarded Augmentation Pipeline (backlog item #16)

Generates three JPEG re-compression variants of every image in the authentic
and ai_generated training corpora, simulating lossy platform re-encoding by
Twitter, WhatsApp, and cross-platform forwarding chains.

Background
----------
JPEG Ghost calibration (v3, 2026-04-11) revealed that platform-forwarded
splices drop differential ghost p95 from ~0.12 to ~0.041 -- indistinguishable
from authentic plain images (0.035). The TruFor learned-residual approach
(CVPR 2023) was evaluated but rejected on licence grounds (non-commercial
restriction incompatible with CIC operations). This augmentation pipeline
is the in-house alternative: teach the UnivFD probe to recognise synthetic
content via CLIP features that survive re-encoding, without relying on
fragile JPEG differential signatures.

Augmentation variants
---------------------
  _plt75  -- Q=75 single pass (Twitter-equivalent lossy recompression)
  _plt85  -- Q=85 single pass (WhatsApp-equivalent)
  _plt2x  -- Q=85 then Q=75 double-pass (cross-platform forwarding chain)

Idempotency
-----------
If an output variant already exists AND the source SHA-256 recorded in the
manifest matches the current source file, the variant is skipped. Run the
script multiple times safely.

Usage
-----
    # Pilot -- process 30 source images only (smoke-test)
    python scripts/augment_corpus_platform_forwarded.py --count 30

    # Full run
    python scripts/augment_corpus_platform_forwarded.py

    # Override source / output roots
    python scripts/augment_corpus_platform_forwarded.py \\
        --source-root "/Volumes/Samsung USB/Training Data/corpus/training" \\
        --output-root "/Volumes/Samsung USB/Training Data/corpus/training_platform_forwarded" \\
        --manifest models/platform_augmentation_manifest.json

    # Parallel workers (default: cpu_count or 4, whichever is lower)
    python scripts/augment_corpus_platform_forwarded.py --workers 8

Dependencies: stdlib + Pillow + numpy (already in sidecar/requirements.txt)
"""

import argparse
import hashlib
import json
import multiprocessing
import os
import sys
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

try:
    from PIL import Image
    import numpy as np
except ImportError:
    print("ERROR: Pillow and numpy are required. Install with: pip install Pillow numpy")
    sys.exit(1)

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}

AUTHENTIC_SUBDIR = "authentic"
AI_SUBDIR = "ai_generated"

# Platform re-encoding quality levels
VARIANTS = {
    "plt75": {"passes": [75]},          # Twitter: single pass Q=75
    "plt85": {"passes": [85]},          # WhatsApp: single pass Q=85
    "plt2x": {"passes": [85, 75]},      # Cross-platform: Q=85 then Q=75
}

# Image size filters
MIN_SHORT_SIDE_PX = 256     # Skip images smaller than this
MAX_FILE_SIZE_BYTES = 20 * 1024 * 1024  # 20 MB

# Manifest schema version -- bump when manifest structure changes
MANIFEST_VERSION = "1.0"


# ---------------------------------------------------------------------------
# Source image collection
# ---------------------------------------------------------------------------

def collect_sources(source_root: Path, subdirs: list[str]) -> list[Path]:
    """Recursively collect image files from the specified subdirectories."""
    images: list[Path] = []
    for subdir in subdirs:
        d = source_root / subdir
        if not d.exists():
            print(f"  WARNING: source subdir not found: {d}")
            continue
        found = sorted(
            f for f in d.rglob("*")
            if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
        )
        images.extend(found)
        print(f"  {subdir}: {len(found)} images")
    return images


# ---------------------------------------------------------------------------
# Disk space check
# ---------------------------------------------------------------------------

def check_disk_space(output_root: Path, required_bytes: int) -> bool:
    """Return True if output_root's volume has at least required_bytes free."""
    import shutil
    output_root.mkdir(parents=True, exist_ok=True)
    free = shutil.disk_usage(output_root).free
    print(f"  Disk free on output volume: {free / (1024**3):.2f} GB")
    print(f"  Estimated required:         {required_bytes / (1024**3):.2f} GB")
    return free >= required_bytes


# ---------------------------------------------------------------------------
# Per-image SHA-256
# ---------------------------------------------------------------------------

def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# Single-image augmentation (runs in worker process)
# ---------------------------------------------------------------------------

def _augment_one(args_tuple) -> dict:
    """
    Worker function executed in a subprocess.

    Returns a dict with keys:
        source_path, source_hash, outputs (list of output records),
        skipped (bool), skip_reason (str|None), error (str|None)

    Takes an explicit `variants_subset` so that child workers honour the
    --variants filter. Child processes are spawned (macOS default), which
    re-imports this module and resets the module-level VARIANTS dict, so
    relying on a parent-side del on VARIANTS does not propagate. The
    active subset must be passed through the worker payload explicitly.
    """
    (
        source_path_str,
        output_root_str,
        source_root_str,
        existing_hashes,   # dict variant_path_str -> source_hash for idempotency
        variants_subset,   # list of variant tags to generate this run
    ) = args_tuple

    source_path = Path(source_path_str)
    output_root = Path(output_root_str)
    source_root = Path(source_root_str)
    # Filter VARIANTS locally inside the worker using the passed subset.
    active_variants = {tag: VARIANTS[tag] for tag in variants_subset if tag in VARIANTS}

    result = {
        "source_path": source_path_str,
        "source_hash": None,
        "outputs": [],
        "skipped": False,
        "skip_reason": None,
        "error": None,
    }

    # --- Size filter (file size) ---
    try:
        file_size = source_path.stat().st_size
    except OSError as e:
        result["error"] = f"stat failed: {e}"
        return result

    if file_size > MAX_FILE_SIZE_BYTES:
        result["skipped"] = True
        result["skip_reason"] = f"file too large ({file_size} bytes > {MAX_FILE_SIZE_BYTES})"
        return result

    # --- Open image and check short side ---
    try:
        img = Image.open(source_path).convert("RGB")
    except Exception as e:
        result["error"] = f"PIL open failed: {e}"
        return result

    w, h = img.size
    short_side = min(w, h)
    if short_side < MIN_SHORT_SIDE_PX:
        result["skipped"] = True
        result["skip_reason"] = f"short side {short_side}px < {MIN_SHORT_SIDE_PX}px"
        return result

    # --- Source hash (lazy, computed once) ---
    try:
        source_hash = sha256_file(source_path)
    except OSError as e:
        result["error"] = f"hash failed: {e}"
        return result
    result["source_hash"] = source_hash

    # --- Determine output subpath: mirror source_root directory structure ---
    try:
        rel = source_path.relative_to(source_root)
    except ValueError:
        result["error"] = "source path not under source_root"
        return result

    stem = source_path.stem
    parent_rel = rel.parent  # preserves subdir (e.g. authentic/coco/)

    outputs = []
    for variant_tag, spec in active_variants.items():
        out_name = f"{stem}_{variant_tag}.jpg"
        out_path = output_root / parent_rel / out_name
        out_path_str = str(out_path)

        # Idempotency check
        if out_path.exists() and existing_hashes.get(out_path_str) == source_hash:
            outputs.append({
                "variant": variant_tag,
                "path": out_path_str,
                "size_bytes": out_path.stat().st_size,
                "status": "skipped_exists",
            })
            continue

        # Apply compression passes
        try:
            current = img.copy()
            for quality in spec["passes"]:
                import io
                buf = io.BytesIO()
                current.save(buf, format="JPEG", quality=quality, subsampling=2)
                buf.seek(0)
                current = Image.open(buf).convert("RGB")

            out_path.parent.mkdir(parents=True, exist_ok=True)
            current.save(out_path, format="JPEG", quality=spec["passes"][-1], subsampling=2)

            outputs.append({
                "variant": variant_tag,
                "path": out_path_str,
                "size_bytes": out_path.stat().st_size,
                "status": "generated",
            })
        except Exception as e:
            outputs.append({
                "variant": variant_tag,
                "path": out_path_str,
                "size_bytes": 0,
                "status": "error",
                "error": str(e),
            })

    result["outputs"] = outputs
    return result


# ---------------------------------------------------------------------------
# Manifest load / save
# ---------------------------------------------------------------------------

def load_manifest(manifest_path: Path) -> dict:
    if manifest_path.exists():
        with open(manifest_path) as f:
            return json.load(f)
    return {
        "schema_version": MANIFEST_VERSION,
        "generated_at": None,
        "source_root": None,
        "output_root": None,
        "licence_note": (
            "All augmented variants are pure PIL JPEG re-compressions of CC-BY-compatible "
            "source images. No external data was downloaded during augmentation. No CASIA "
            "or non-commercial-licensed data is included. The augmentation operation itself "
            "introduces no new copyright — the output licence follows the source image licence."
        ),
        "augmentation_variants": {
            tag: spec for tag, spec in VARIANTS.items()
        },
        "filters": {
            "min_short_side_px": MIN_SHORT_SIDE_PX,
            "max_file_size_bytes": MAX_FILE_SIZE_BYTES,
        },
        "corpus_totals": {},
        "skipped": [],
        "errors": [],
        "records": [],
    }


def save_manifest(manifest: dict, manifest_path: Path) -> None:
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)


def build_existing_hash_index(manifest: dict) -> dict:
    """Build {output_path_str: source_hash} from existing manifest records."""
    index = {}
    for rec in manifest.get("records", []):
        source_hash = rec.get("source_hash")
        if not source_hash:
            continue
        for out in rec.get("outputs", []):
            if out.get("status") == "generated":
                index[out["path"]] = source_hash
    return index


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Generate platform-forwarded augmentation variants of the Jura Trace training corpus"
    )
    parser.add_argument(
        "--source-root",
        default="/Volumes/Samsung USB/Training Data/corpus/training",
        help="Root directory containing authentic/ and ai_generated/ subdirectories",
    )
    parser.add_argument(
        "--output-root",
        default="/Volumes/Samsung USB/Training Data/corpus/training_platform_forwarded",
        help="Root directory for augmented output (must NOT be inside source-root)",
    )
    parser.add_argument(
        "--manifest",
        default=os.path.join(os.path.dirname(__file__), "..", "models", "platform_augmentation_manifest.json"),
        help="Path to manifest JSON (idempotency state)",
    )
    parser.add_argument(
        "--count",
        type=int,
        default=None,
        help="Limit to first N source images (pilot mode)",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=min(multiprocessing.cpu_count(), 4),
        help="Parallel worker processes for PIL augmentation",
    )
    parser.add_argument(
        "--skip-space-check",
        action="store_true",
        help="Skip disk space pre-flight check (use with caution)",
    )
    parser.add_argument(
        "--variants",
        default=",".join(VARIANTS.keys()),
        help=(
            "Comma-separated variant subset to generate (default: all). "
            "Choose from: plt75, plt85, plt2x. Use to cut disk usage when "
            "the output volume cannot hold all three variants."
        ),
    )
    args = parser.parse_args()

    # Filter VARIANTS to the requested subset. Mutates the module-level dict
    # so all downstream consumers (estimator, worker pool, manifest writer)
    # see the same subset consistently.
    requested = {v.strip() for v in args.variants.split(",") if v.strip()}
    unknown = requested - set(VARIANTS.keys())
    if unknown:
        print(f"\nERROR: Unknown variant tags: {sorted(unknown)}")
        print(f"  Valid tags: {sorted(VARIANTS.keys())}")
        sys.exit(1)
    for tag in list(VARIANTS.keys()):
        if tag not in requested:
            del VARIANTS[tag]
    if not VARIANTS:
        print("\nERROR: No variants selected. Aborting.")
        sys.exit(1)

    source_root = Path(args.source_root)
    output_root = Path(args.output_root)
    manifest_path = Path(args.manifest)

    print("Jura Trace -- Platform-Forwarded Corpus Augmentation (backlog #16)")
    print("=" * 70)
    print(f"  Source root:  {source_root}")
    print(f"  Output root:  {output_root}")
    print(f"  Manifest:     {manifest_path}")
    print(f"  Workers:      {args.workers}")
    if args.count:
        print(f"  PILOT MODE:   {args.count} source images only")

    # --- Sanity: output must not be inside source ---
    try:
        output_root.resolve().relative_to(source_root.resolve())
        print("\nERROR: output-root is inside source-root. Aborting to protect original corpus.")
        sys.exit(1)
    except ValueError:
        pass  # Good -- they are distinct

    # --- Check source root exists ---
    if not source_root.exists():
        print(f"\nERROR: Source root not found: {source_root}")
        print("  Is the USB drive mounted? Check with: ls /Volumes/")
        sys.exit(1)

    # --- Collect source images ---
    print("\nCollecting source images...")
    all_sources = collect_sources(source_root, [AUTHENTIC_SUBDIR, AI_SUBDIR])
    print(f"  Total source images found: {len(all_sources)}")

    if not all_sources:
        print("\nERROR: No source images found. Check source-root path.")
        sys.exit(1)

    if args.count:
        all_sources = all_sources[: args.count]
        print(f"  Limiting to pilot set: {len(all_sources)} images")

    # --- Disk space estimate ---
    # Heuristic: augmented JPEGs typically 40-70% of source size after recompression.
    # Use 0.6 × source total × 3 variants as a conservative estimate.
    # We sample the first 20 source file sizes to estimate average.
    sample_sizes = [p.stat().st_size for p in all_sources[:20] if p.exists()]
    avg_source_bytes = sum(sample_sizes) / max(len(sample_sizes), 1)
    estimated_bytes = int(avg_source_bytes * 0.6 * len(all_sources) * len(VARIANTS))
    safety_margin = int(estimated_bytes * 1.25)  # 25% headroom

    print(f"\n  Estimated disk usage: {estimated_bytes / (1024**3):.2f} GB")
    print(f"  With 25%% safety margin: {safety_margin / (1024**3):.2f} GB")

    if not args.skip_space_check:
        print("\nChecking disk space on output volume...")
        if not check_disk_space(output_root, safety_margin):
            print("\nERROR: Insufficient disk space on output volume. Aborting.")
            print("  Free up space on the USB drive or use --skip-space-check to bypass.")
            sys.exit(1)
        print("  Disk space OK.")
    else:
        output_root.mkdir(parents=True, exist_ok=True)

    # --- Load manifest (idempotency state) ---
    print("\nLoading manifest...")
    manifest = load_manifest(manifest_path)
    existing_hashes = build_existing_hash_index(manifest)
    print(f"  Previously processed records: {len(manifest.get('records', []))}")
    print(f"  Known completed variants: {len(existing_hashes)}")

    # --- Build worker arg tuples ---
    # Pass the active variants_subset explicitly so child workers honour
    # the --variants filter. Relying on the parent-side del on VARIANTS
    # does not propagate through multiprocessing spawn (macOS default).
    active_variants_list = list(VARIANTS.keys())
    worker_args = [
        (str(src), str(output_root), str(source_root), existing_hashes, active_variants_list)
        for src in all_sources
    ]

    # --- Run augmentation ---
    print(f"\nStarting augmentation with {args.workers} workers...")
    t_start = time.time()

    records_by_source = {
        rec["source_path"]: rec
        for rec in manifest.get("records", [])
    }

    generated_count = 0
    skipped_count = 0
    error_count = 0
    total_bytes = 0

    # Use ProcessPoolExecutor for CPU-bound PIL work
    with ProcessPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(_augment_one, arg): arg[0] for arg in worker_args}
        processed = 0
        for future in as_completed(futures):
            processed += 1
            src_str = futures[future]
            try:
                result = future.result()
            except Exception as exc:
                print(f"  WORKER EXCEPTION {src_str}: {exc}")
                error_count += 1
                continue

            if result.get("error"):
                print(f"  ERROR {Path(src_str).name}: {result['error']}")
                error_count += 1
                manifest.setdefault("errors", []).append({
                    "source": src_str,
                    "error": result["error"],
                })
                continue

            if result.get("skipped"):
                print(f"  SKIP {Path(src_str).name}: {result['skip_reason']}")
                skipped_count += 1
                manifest.setdefault("skipped", []).append({
                    "source": src_str,
                    "reason": result["skip_reason"],
                })
                continue

            # Tally outputs
            new_this = sum(1 for o in result["outputs"] if o["status"] == "generated")
            generated_count += new_this
            total_bytes += sum(o["size_bytes"] for o in result["outputs"] if o["status"] == "generated")

            # Update manifest record
            records_by_source[src_str] = {
                "source_path": src_str,
                "source_hash": result["source_hash"],
                "outputs": result["outputs"],
                "processed_at": datetime.now(timezone.utc).isoformat(),
            }

            if processed % 50 == 0 or processed == len(worker_args):
                elapsed = time.time() - t_start
                rate = processed / max(elapsed, 0.001)
                eta = (len(worker_args) - processed) / max(rate, 0.001)
                print(
                    f"  [{processed}/{len(worker_args)}] "
                    f"generated={generated_count} skipped={skipped_count} errors={error_count} "
                    f"disk={total_bytes / (1024**3):.2f}GB "
                    f"elapsed={elapsed:.0f}s ETA={eta:.0f}s"
                )

                # Incremental manifest save every 50 images
                manifest["records"] = list(records_by_source.values())
                manifest["corpus_totals"] = {
                    "source_images_processed": processed,
                    "variants_generated": generated_count,
                    "variants_skipped_idempotent": skipped_count,
                    "errors": error_count,
                    "total_output_bytes": total_bytes,
                }
                manifest["generated_at"] = datetime.now(timezone.utc).isoformat()
                manifest["source_root"] = str(source_root)
                manifest["output_root"] = str(output_root)
                save_manifest(manifest, manifest_path)

    # --- Final manifest save ---
    elapsed = time.time() - t_start
    manifest["records"] = list(records_by_source.values())
    manifest["corpus_totals"] = {
        "source_images_processed": len(worker_args),
        "variants_generated": generated_count,
        "variants_skipped_idempotent": skipped_count,
        "errors": error_count,
        "total_output_bytes": total_bytes,
        "wall_time_seconds": round(elapsed, 1),
    }
    manifest["generated_at"] = datetime.now(timezone.utc).isoformat()
    manifest["source_root"] = str(source_root)
    manifest["output_root"] = str(output_root)
    save_manifest(manifest, manifest_path)

    # --- Summary ---
    print("\n" + "=" * 70)
    print("AUGMENTATION COMPLETE")
    print("=" * 70)
    print(f"  Source images processed:    {len(worker_args)}")
    print(f"  Variants generated:         {generated_count}")
    print(f"  Skipped (idempotent):       {skipped_count}")
    print(f"  Errors:                     {error_count}")
    print(f"  Total output:               {total_bytes / (1024**3):.2f} GB")
    print(f"  Wall time:                  {elapsed:.0f}s ({elapsed/60:.1f} min)")
    print(f"  Manifest:                   {manifest_path}")

    if error_count > 0:
        print(f"\n  WARNING: {error_count} errors. Review manifest 'errors' array.")

    if args.count:
        print(
            f"\n  PILOT MODE complete. Inspect output at:\n  {output_root}"
            f"\n  Then run without --count for the full corpus."
        )

    print("\nNext step:")
    print("  python scripts/build_augmented_training_set.py --help")


if __name__ == "__main__":
    main()
