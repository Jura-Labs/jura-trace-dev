#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace -- Build AI-generated image corpus.

Downloads AI-generated images from freely available sources to expand
the training corpus beyond the local images.

Current strategy:
1. Use local images from /Users/paulgriffiths/Desktop/Fake Images AI/ (19 images)
2. Optionally download from HuggingFace datasets API

Usage:
    python scripts/build_corpus_ai.py
    python scripts/build_corpus_ai.py --output corpus/ai_generated --download
"""

import argparse
import os
import shutil
import sys
from pathlib import Path


IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}

# Local AI images source
LOCAL_AI_SOURCE = "/Users/paulgriffiths/Desktop/Fake Images AI"


def copy_local_images(source: str, dest: Path) -> int:
    """Copy AI images from local source to corpus directory."""
    src = Path(source)
    if not src.exists():
        print(f"  WARNING: Local source not found: {src}")
        return 0

    count = 0
    for f in sorted(src.iterdir()):
        if f.suffix.lower() in IMAGE_EXTENSIONS:
            target = dest / f"local_{f.name}"
            if not target.exists():
                shutil.copy2(f, target)
                count += 1
                print(f"    Copied: {f.name}")
            else:
                print(f"    Exists: {f.name}")
                count += 1

    return count


def download_huggingface_samples(dest: Path, max_images: int = 50) -> int:
    """Download AI-generated images from HuggingFace datasets.

    Requires: pip install datasets pillow
    Uses the 'poloclub/diffusiondb' dataset (CC0 licence) which contains
    Stable Diffusion outputs.
    """
    try:
        from datasets import load_dataset
    except ImportError:
        print("  SKIP: 'datasets' library not installed.")
        print("  Install with: pip install datasets")
        return 0

    print(f"  Downloading up to {max_images} images from HuggingFace diffusiondb...")
    try:
        # Use the 2m_random_1k subset for speed
        ds = load_dataset(
            "poloclub/diffusiondb",
            "2m_random_1k",
            split="train",
            trust_remote_code=True,
        )
    except Exception as e:
        print(f"  FAILED: Could not load dataset: {e}")
        return 0

    count = 0
    for i, sample in enumerate(ds):
        if count >= max_images:
            break
        try:
            img = sample["image"]
            fname = f"diffusiondb_{i:04d}.png"
            target = dest / fname
            if not target.exists():
                img.save(target, format="PNG")
                count += 1
                if count % 10 == 0:
                    print(f"    Downloaded {count}/{max_images}...")
        except Exception:
            continue

    print(f"  Downloaded {count} images from HuggingFace")
    return count


def main():
    parser = argparse.ArgumentParser(description="Build AI-generated image corpus")
    parser.add_argument(
        "--output",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "corpus", "ai_generated"),
        help="Output directory for AI corpus",
    )
    parser.add_argument(
        "--local-source",
        type=str,
        default=LOCAL_AI_SOURCE,
        help="Path to local AI images",
    )
    parser.add_argument(
        "--download",
        action="store_true",
        help="Download additional images from HuggingFace",
    )
    parser.add_argument(
        "--max-download",
        type=int,
        default=50,
        help="Maximum images to download from HuggingFace",
    )
    args = parser.parse_args()

    print("Jura Trace -- AI Corpus Builder")
    print("=" * 60)

    dest = Path(args.output)
    dest.mkdir(parents=True, exist_ok=True)
    print(f"  Output: {dest}")

    # Copy local images
    print("\nCopying local AI images...")
    local_count = copy_local_images(args.local_source, dest)
    print(f"  Local images: {local_count}")

    # Optional download
    dl_count = 0
    if args.download:
        print("\nDownloading from HuggingFace...")
        dl_count = download_huggingface_samples(dest, max_images=args.max_download)

    # Summary
    total = sum(1 for f in dest.iterdir() if f.suffix.lower() in IMAGE_EXTENSIONS)
    print(f"\nCorpus summary:")
    print(f"  Local copied: {local_count}")
    print(f"  Downloaded: {dl_count}")
    print(f"  Total in corpus: {total}")
    print(f"  Location: {dest}")
    print("\nDone.")


if __name__ == "__main__":
    main()
