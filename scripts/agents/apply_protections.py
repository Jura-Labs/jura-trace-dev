#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Protection Pipeline Agent

Applies fingerprinting, watermarking, and C2PA signing to corpus images
via the Jura Trace REST API (port 8300) and sidecar (port 8200).

Produces four variants of each image:
  1. fingerprinted/ — original with fingerprint hashes recorded
  2. watermarked/   — DWT-DCT-SVD invisible watermark embedded
  3. c2pa_signed/   — C2PA manifest signed
  4. combined/      — watermark + C2PA sign (all protections)

The key constraint: AI-generated images with protection applied must
still be identifiable as AI-generated (trust < 0.70).

Usage:
    python -m scripts.agents.apply_protections
    python -m scripts.agents.apply_protections --corpus corpus/training/ai_generated --max 50
    python -m scripts.agents.apply_protections --operations fingerprint,watermark,sign
"""

import argparse
import json
import shutil
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config
from scripts.agents.api_client import (
    check_api_health,
    check_sidecar_health,
    fingerprint_file,
    sign_c2pa_sidecar,
    watermark_embed_sidecar,
)


def collect_images(corpus_dir: Path, max_images: int) -> list[Path]:
    """Collect image files from the corpus directory tree."""
    images = []
    for ext in config.IMAGE_EXTENSIONS:
        images.extend(corpus_dir.rglob(f"*{ext}"))
    images.sort()
    return images[:max_images]


def apply_fingerprints(
    images: list[Path], output_dir: Path
) -> list[dict]:
    """Fingerprint each image and record hashes."""
    output_dir.mkdir(parents=True, exist_ok=True)
    results = []

    for i, img_path in enumerate(images):
        result = fingerprint_file(img_path)
        if result is None:
            continue

        # Copy original to fingerprinted dir
        dest = output_dir / img_path.name
        shutil.copy2(img_path, dest)

        hashes = result.get("data", {}).get("hashes", [])
        results.append({
            "original": str(img_path),
            "protected_path": str(dest),
            "protection": "fingerprint",
            "hashes": hashes,
            "timestamp": datetime.now(timezone.utc).isoformat(),
        })

        if (i + 1) % 10 == 0:
            print(f"    Fingerprinted {i + 1}/{len(images)}")

    print(f"  [fingerprint] {len(results)}/{len(images)} processed")
    return results


def apply_watermarks(
    images: list[Path], output_dir: Path, payload: str, strength: str = "medium"
) -> list[dict]:
    """Embed invisible watermarks into each image."""
    output_dir.mkdir(parents=True, exist_ok=True)
    results = []

    for i, img_path in enumerate(images):
        wm_bytes = watermark_embed_sidecar(img_path, payload, strength)
        if wm_bytes is None:
            continue

        dest = output_dir / f"{img_path.stem}_wm.png"
        dest.write_bytes(wm_bytes)

        results.append({
            "original": str(img_path),
            "protected_path": str(dest),
            "protection": "watermark",
            "payload": payload,
            "strength": strength,
            "size_bytes": len(wm_bytes),
            "timestamp": datetime.now(timezone.utc).isoformat(),
        })

        if (i + 1) % 10 == 0:
            print(f"    Watermarked {i + 1}/{len(images)}")

    print(f"  [watermark] {len(results)}/{len(images)} processed")
    return results


def apply_c2pa_signing(
    images: list[Path], output_dir: Path, creator_name: str
) -> list[dict]:
    """C2PA-sign each image."""
    output_dir.mkdir(parents=True, exist_ok=True)
    results = []

    for i, img_path in enumerate(images):
        signed_bytes = sign_c2pa_sidecar(img_path, creator_name)
        if signed_bytes is None:
            continue

        dest = output_dir / f"{img_path.stem}_c2pa{img_path.suffix}"
        dest.write_bytes(signed_bytes)

        results.append({
            "original": str(img_path),
            "protected_path": str(dest),
            "protection": "c2pa",
            "creator_name": creator_name,
            "size_bytes": len(signed_bytes),
            "timestamp": datetime.now(timezone.utc).isoformat(),
        })

        if (i + 1) % 10 == 0:
            print(f"    C2PA signed {i + 1}/{len(images)}")

    print(f"  [c2pa] {len(results)}/{len(images)} processed")
    return results


def apply_combined(
    images: list[Path], output_dir: Path, payload: str, creator_name: str
) -> list[dict]:
    """Apply watermark then C2PA sign (combined protection)."""
    output_dir.mkdir(parents=True, exist_ok=True)
    results = []

    for i, img_path in enumerate(images):
        # Step 1: Watermark
        wm_bytes = watermark_embed_sidecar(img_path, payload)
        if wm_bytes is None:
            continue

        # Write temp watermarked file for C2PA signing
        wm_path = output_dir / f"{img_path.stem}_tmp_wm.png"
        wm_path.write_bytes(wm_bytes)

        # Step 2: C2PA sign the watermarked file
        signed_bytes = sign_c2pa_sidecar(wm_path, creator_name)

        # Clean up temp
        wm_path.unlink(missing_ok=True)

        if signed_bytes is None:
            continue

        dest = output_dir / f"{img_path.stem}_combined.png"
        dest.write_bytes(signed_bytes)

        results.append({
            "original": str(img_path),
            "protected_path": str(dest),
            "protection": "combined",
            "payload": payload,
            "creator_name": creator_name,
            "size_bytes": len(signed_bytes),
            "timestamp": datetime.now(timezone.utc).isoformat(),
        })

        if (i + 1) % 10 == 0:
            print(f"    Combined {i + 1}/{len(images)}")

    print(f"  [combined] {len(results)}/{len(images)} processed")
    return results


OPERATIONS = {
    "fingerprint": "Perceptual hash fingerprinting",
    "watermark": "DWT-DCT-SVD invisible watermark",
    "sign": "C2PA content credential signing",
    "combined": "Watermark + C2PA (all protections)",
}


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace — Protection Pipeline Agent"
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        default=config.CORPUS_AI,
        help="Input corpus directory",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=config.PROTECTED_BASE,
        help="Output directory for protected files",
    )
    parser.add_argument(
        "--operations",
        default=",".join(OPERATIONS.keys()),
        help=f"Comma-separated operations: {','.join(OPERATIONS.keys())}",
    )
    parser.add_argument(
        "--max",
        type=int,
        default=50,
        help="Max images to process (default: 50)",
    )
    parser.add_argument(
        "--payload",
        default=config.C2PA_CREATOR_NAME,
        help="Watermark payload text",
    )
    parser.add_argument(
        "--creator",
        default=config.C2PA_CREATOR_NAME,
        help="C2PA creator name",
    )
    args = parser.parse_args()

    ops = [o.strip() for o in args.operations.split(",") if o.strip()]

    print("=" * 60)
    print("  Jura Trace — Protection Pipeline Agent")
    print("=" * 60)

    # Health checks
    needs_api = any(o in ("fingerprint", "sign", "combined") for o in ops)
    needs_sidecar = any(o in ("watermark", "combined") for o in ops)

    if needs_api:
        api_health = check_api_health()
        if api_health is None:
            print("  ERROR: REST API (port 8300) is not available.")
            print("  Start the Tauri app or set JURA_API_URL.")
            sys.exit(1)
        print(f"  REST API: online (v{api_health.get('version', '?')})")

    if needs_sidecar:
        sidecar_health = check_sidecar_health()
        if sidecar_health is None:
            print("  ERROR: Sidecar (port 8200) is not available.")
            sys.exit(1)
        print(f"  Sidecar: online")

    # Collect images
    images = collect_images(args.corpus, args.max)
    if not images:
        print(f"  No images found in {args.corpus}")
        sys.exit(1)
    print(f"  Images to process: {len(images)}")

    all_results = []
    start = time.time()

    if "fingerprint" in ops:
        print(f"\n  --- Fingerprinting ---")
        results = apply_fingerprints(images, args.output / "fingerprinted")
        all_results.extend(results)

    if "watermark" in ops:
        print(f"\n  --- Watermarking ---")
        results = apply_watermarks(
            images, args.output / "watermarked", args.payload
        )
        all_results.extend(results)

    if "sign" in ops:
        print(f"\n  --- C2PA Signing ---")
        results = apply_c2pa_signing(
            images, args.output / "c2pa_signed", args.creator
        )
        all_results.extend(results)

    if "combined" in ops:
        print(f"\n  --- Combined Protection ---")
        results = apply_combined(
            images, args.output / "combined", args.payload, args.creator
        )
        all_results.extend(results)

    # Write results manifest
    config.RESULTS_BASE.mkdir(parents=True, exist_ok=True)
    results_path = config.RESULTS_BASE / "protection_results.json"
    results_manifest = {
        "created_at": datetime.now(timezone.utc).isoformat(),
        "corpus_source": str(args.corpus),
        "operations": ops,
        "total_processed": len(all_results),
        "entries": all_results,
    }
    results_path.write_text(json.dumps(results_manifest, indent=2))

    elapsed = time.time() - start
    print(f"\n{'=' * 60}")
    print(f"  Complete: {len(all_results)} protection operations in {elapsed:.1f}s")
    print(f"  Results: {results_path}")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
