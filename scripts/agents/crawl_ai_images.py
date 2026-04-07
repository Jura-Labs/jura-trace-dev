#!/usr/bin/env python3
"""
Jura Trace — AI-Generated Image Crawler

Downloads AI-generated images from known public datasets to build a
training corpus for classifier calibration and constraint validation.

Sources:
  - DiffusionDB (Stable Diffusion 1.x) — poloclub/diffusiondb
  - CIFAKE (CIFAR-10 style AI) — birgermoell/cifake-real-and-ai-generated-synthetic-images
  - GenImage (multi-generator) — HKUST-AIGC/GenImage or local mirror
  - JourneyDB (Midjourney) — JourneyDB/JourneyDB

Usage:
    python -m scripts.agents.crawl_ai_images
    python -m scripts.agents.crawl_ai_images --sources diffusiondb,cifake --max 100
    python -m scripts.agents.crawl_ai_images --output corpus/training/ai_generated
"""

import argparse
import hashlib
import json
import sys
import time
from datetime import datetime, timezone
from io import BytesIO
from pathlib import Path

# Add project root to path for imports
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config


SOURCES = {
    "elsa": {
        "dataset_id": "elsaEU/ELSA1M_track1",
        "subset": None,
        "split": "train",
        "description": "ELSA 1M — multi-model AI-generated images (SD, DALL-E, MJ)",
        "image_key": "image",
        "label": "elsa_multimodel",
        "streaming": True,
    },
    "cifar10_ai": {
        "dataset_id": "cifar10",
        "subset": None,
        "split": "test",
        "description": "CIFAR-10 test set — used as low-res AI baseline",
        "image_key": "img",
        "filter_key": "label",
        "filter_value": None,  # no filter — all classes
        "label": "cifar10_baseline",
    },
}


def download_from_huggingface(
    source_key: str,
    source_config: dict,
    output_dir: Path,
    max_images: int,
) -> list[dict]:
    """Download images from a HuggingFace dataset.

    Returns a list of manifest entries for successfully downloaded images.
    """
    try:
        from datasets import load_dataset
    except ImportError:
        print("  Error: 'datasets' library required. pip install datasets")
        return []

    output_dir.mkdir(parents=True, exist_ok=True)
    dataset_id = source_config["dataset_id"]
    subset = source_config.get("subset")
    split = source_config.get("split", "train")
    image_key = source_config.get("image_key", "image")
    filter_key = source_config.get("filter_key")
    filter_value = source_config.get("filter_value")
    label = source_config.get("label", source_key)

    streaming = source_config.get("streaming", False)
    print(f"\n  [{source_key}] Loading {dataset_id} (split={split}, streaming={streaming})...")

    try:
        if subset:
            ds = load_dataset(dataset_id, subset, split=split, streaming=streaming)
        else:
            ds = load_dataset(dataset_id, split=split, streaming=streaming)
    except Exception as e:
        print(f"  [{source_key}] Failed to load dataset: {e}")
        return []

    entries = []
    downloaded = 0

    # Resume-safe: count existing files and skip duplicate content
    exts = {".jpg", ".jpeg", ".png", ".webp"}
    existing_files = sorted(p for p in output_dir.iterdir() if p.is_file() and p.suffix.lower() in exts) if output_dir.exists() else []
    start_idx = len(existing_files)
    seen_hashes = set()
    for p in existing_files[:2000]:  # cap hash loading at 2,000 for speed
        try:
            seen_hashes.add(hashlib.sha256(p.read_bytes()).hexdigest())
        except Exception:
            pass
    if start_idx:
        print(f"  [{source_key}] Resuming — {start_idx} existing files, {len(seen_hashes)} hashes loaded")

    for i, item in enumerate(ds):
        if downloaded >= max_images:
            break

        # Apply filter if specified (e.g. CIFAKE label==1 for fake)
        if filter_key and item.get(filter_key) != filter_value:
            continue

        img = item.get(image_key)
        if img is None:
            continue

        # Convert PIL image to PNG bytes
        buf = BytesIO()
        img.save(buf, format="PNG")
        img_bytes = buf.getvalue()

        # Skip tiny images
        if len(img_bytes) < 5000:
            continue

        # Deduplicate by SHA-256
        sha = hashlib.sha256(img_bytes).hexdigest()
        if sha in seen_hashes:
            continue
        seen_hashes.add(sha)

        # Per-generator labelling when dataset provides a model field
        model_id = item.get("model") or label
        model_slug = str(model_id).lower().replace(" ", "_").replace("/", "_")[:30]

        # Save with resume-safe index offset
        filename = f"{label}_{start_idx + downloaded:05d}.png"
        filepath = output_dir / filename
        filepath.write_bytes(img_bytes)

        entries.append({
            "filename": filename,
            "source": source_key,
            "dataset_id": dataset_id,
            "sha256": sha,
            "size_bytes": len(img_bytes),
            "label": "ai_generated",
            "generator": model_slug,
            "downloaded_at": datetime.now(timezone.utc).isoformat(),
        })

        downloaded += 1
        if downloaded % 50 == 0:
            print(f"  [{source_key}] {downloaded}/{max_images} downloaded...")

    print(f"  [{source_key}] {downloaded} images downloaded to {output_dir}")
    return entries


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace — AI-Generated Image Crawler"
    )
    parser.add_argument(
        "--sources",
        default=",".join(SOURCES.keys()),
        help=f"Comma-separated source keys: {','.join(SOURCES.keys())}",
    )
    parser.add_argument(
        "--max",
        type=int,
        default=config.DEFAULT_MAX_PER_SOURCE,
        help=f"Max images per source (default: {config.DEFAULT_MAX_PER_SOURCE})",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=config.CORPUS_AI,
        help=f"Output directory (default: {config.CORPUS_AI})",
    )
    args = parser.parse_args()

    sources = [s.strip() for s in args.sources.split(",") if s.strip()]
    invalid = [s for s in sources if s not in SOURCES]
    if invalid:
        print(f"Unknown sources: {invalid}. Available: {list(SOURCES.keys())}")
        sys.exit(1)

    print("=" * 60)
    print("  Jura Trace — AI-Generated Image Crawler")
    print("=" * 60)
    print(f"  Sources: {sources}")
    print(f"  Max per source: {args.max}")
    print(f"  Output: {args.output}")

    all_entries = []
    start = time.time()

    for source_key in sources:
        source_dir = args.output / source_key
        entries = download_from_huggingface(
            source_key, SOURCES[source_key], source_dir, args.max
        )
        all_entries.extend(entries)

    # Write combined manifest
    args.output.mkdir(parents=True, exist_ok=True)
    manifest_path = args.output / "manifest.json"
    manifest = {
        "corpus_type": "ai_generated",
        "created_at": datetime.now(timezone.utc).isoformat(),
        "total_images": len(all_entries),
        "sources": {k: SOURCES[k]["description"] for k in sources},
        "entries": all_entries,
    }

    # Merge with existing manifest if present
    if manifest_path.exists():
        try:
            existing = json.loads(manifest_path.read_text())
            existing_entries = existing.get("entries", [])
            existing_hashes = {e["sha256"] for e in existing_entries}
            new_entries = [e for e in all_entries if e["sha256"] not in existing_hashes]
            manifest["entries"] = existing_entries + new_entries
            manifest["total_images"] = len(manifest["entries"])
            print(f"\n  Merged {len(new_entries)} new images with {len(existing_entries)} existing")
        except Exception:
            pass

    manifest_path.write_text(json.dumps(manifest, indent=2))

    elapsed = time.time() - start
    print(f"\n{'=' * 60}")
    print(f"  Complete: {manifest['total_images']} AI images in {elapsed:.1f}s")
    print(f"  Manifest: {manifest_path}")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
