#!/usr/bin/env python3
"""
Jura Trace — Authentic Image Crawler

Downloads known-authentic images from public sources to build a
training corpus for classifier calibration and false-positive testing.

Sources:
  - COCO val2017 — real photographs
  - Wikimedia Commons Featured — curated CC images
  - Unsplash Lite — high-quality CC0 photos
  - Open Images V7 — Google's annotated real images

Usage:
    python -m scripts.agents.crawl_authentic_images
    python -m scripts.agents.crawl_authentic_images --sources coco,wikimedia --max 100
"""

import argparse
import hashlib
import json
import random
import sys
import time
from datetime import datetime, timezone
from io import BytesIO
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config

# COCO val2017 image IDs (a sampling of known good IDs)
# Full list: https://images.cocodataset.org/annotations/annotations_trainval2017.zip
_COCO_BASE = "http://images.cocodataset.org/val2017"


def _download_url(url: str, timeout: int = 30) -> bytes | None:
    """Download a URL and return bytes, or None on failure."""
    try:
        req = Request(url, headers={"User-Agent": "JuraTrace-CorpusAgent/1.0"})
        with urlopen(req, timeout=timeout) as resp:
            return resp.read()
    except Exception:
        return None


def download_cifar10_authentic(output_dir: Path, max_images: int) -> list[dict]:
    """Download real photographs from CIFAR-10 test set via HuggingFace."""
    output_dir.mkdir(parents=True, exist_ok=True)
    print(f"\n  [cifar10] Downloading from CIFAR-10 test set (max {max_images})...")

    try:
        from datasets import load_dataset
    except ImportError:
        print("  Error: 'datasets' library required. pip install datasets")
        return []

    try:
        ds = load_dataset("cifar10", split="test")
    except Exception as e:
        print(f"  [cifar10] Failed to load dataset: {e}")
        return []

    entries = []
    downloaded = 0

    for item in ds:
        if downloaded >= max_images:
            break

        img = item.get("img")
        if img is None:
            continue

        buf = BytesIO()
        img.save(buf, format="PNG")
        img_bytes = buf.getvalue()

        sha = hashlib.sha256(img_bytes).hexdigest()
        out_name = f"cifar10_{downloaded:04d}.png"
        out_path = output_dir / out_name
        out_path.write_bytes(img_bytes)

        entries.append({
            "filename": out_name,
            "source": "cifar10",
            "sha256": sha,
            "size_bytes": len(img_bytes),
            "label": "authentic",
            "downloaded_at": datetime.now(timezone.utc).isoformat(),
        })

        downloaded += 1
        if downloaded % 50 == 0:
            print(f"  [cifar10] {downloaded}/{max_images} downloaded...")

    print(f"  [cifar10] {downloaded} images downloaded")
    return entries


def download_wikimedia(output_dir: Path, max_images: int) -> list[dict]:
    """Download featured images from Wikimedia Commons."""
    output_dir.mkdir(parents=True, exist_ok=True)
    print(f"\n  [wikimedia] Downloading from Wikimedia Commons Featured (max {max_images})...")

    entries = []
    downloaded = 0
    gcmcontinue = ""

    while downloaded < max_images:
        url = (
            "https://commons.wikimedia.org/w/api.php?"
            "action=query&generator=categorymembers"
            "&gcmtitle=Category:Featured_pictures_on_Wikimedia_Commons"
            "&gcmtype=file&gcmlimit=50"
            "&prop=imageinfo&iiprop=url|size|mime"
            f"&format=json{f'&gcmcontinue={gcmcontinue}' if gcmcontinue else ''}"
        )

        data = _download_url(url)
        if data is None:
            break

        result = json.loads(data)
        pages = result.get("query", {}).get("pages", {})

        for page_id, page in pages.items():
            if downloaded >= max_images:
                break

            imageinfo = page.get("imageinfo", [{}])[0]
            img_url = imageinfo.get("url", "")
            mime = imageinfo.get("mime", "")

            # Only download JPEG/PNG images under 10 MB
            if not mime.startswith("image/") or mime not in ("image/jpeg", "image/png"):
                continue
            if imageinfo.get("size", 0) > 10_000_000:
                continue

            img_data = _download_url(img_url)
            if img_data is None or len(img_data) < 5000:
                continue

            sha = hashlib.sha256(img_data).hexdigest()
            ext = ".jpg" if mime == "image/jpeg" else ".png"
            out_name = f"wikimedia_{downloaded:04d}{ext}"
            out_path = output_dir / out_name
            out_path.write_bytes(img_data)

            entries.append({
                "filename": out_name,
                "source": "wikimedia",
                "source_url": img_url,
                "sha256": sha,
                "size_bytes": len(img_data),
                "label": "authentic",
                "downloaded_at": datetime.now(timezone.utc).isoformat(),
            })

            downloaded += 1
            if downloaded % 20 == 0:
                print(f"  [wikimedia] {downloaded}/{max_images} downloaded...")

            time.sleep(config.REQUEST_DELAY_SECONDS)

        cont = result.get("continue", {})
        gcmcontinue = cont.get("gcmcontinue", "")
        if not gcmcontinue:
            break

    print(f"  [wikimedia] {downloaded} images downloaded")
    return entries


def _existing_image_count(output_dir: Path) -> int:
    """Count existing image files so repeated runs don't overwrite."""
    if not output_dir.exists():
        return 0
    exts = {".jpg", ".jpeg", ".png", ".webp"}
    return sum(1 for p in output_dir.iterdir() if p.is_file() and p.suffix.lower() in exts)


def _existing_hashes(output_dir: Path) -> set[str]:
    """Return sha256 hashes of existing images to avoid re-downloading dupes."""
    if not output_dir.exists():
        return set()
    hashes = set()
    exts = {".jpg", ".jpeg", ".png", ".webp"}
    for p in output_dir.iterdir():
        if p.is_file() and p.suffix.lower() in exts:
            try:
                hashes.add(hashlib.sha256(p.read_bytes()).hexdigest())
            except Exception:
                pass
    return hashes


def download_flickr30k(output_dir: Path, max_images: int) -> list[dict]:
    """Download photographs from Flickr30k via HuggingFace.

    Replaces the previous `openimages` source (djghosh/open-images-v7-selected
    was removed from the Hub in early 2026). Flickr30k contains ~30,000
    real photographs with wide subject diversity: people in candid and
    posed settings, events, sports, everyday scenes. Ideal authentic
    corpus supplement for a forensic training pipeline.

    Dataset: lmms-lab/flickr30k, split=test (~31,014 photographs).

    Resume-safe: counts existing files and offsets filename numbering
    so repeated invocations accumulate rather than overwrite.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    start_idx = _existing_image_count(output_dir)
    existing_hashes = _existing_hashes(output_dir)
    print(f"\n  [flickr30k] Downloading from Flickr30k (max {max_images}, existing {start_idx})...")

    try:
        from datasets import load_dataset
    except ImportError:
        print("  Error: 'datasets' library required. pip install datasets")
        return []

    try:
        ds = load_dataset(
            "lmms-lab/flickr30k",
            split="test",
            streaming=True,
        )
    except Exception as e:
        print(f"  [flickr30k] Failed to load dataset: {e}")
        return []

    entries = []
    downloaded = 0
    skipped_dupes = 0

    for item in ds:
        if downloaded >= max_images:
            break

        img = item.get("image")
        if img is None:
            continue

        try:
            w, h = img.size
        except Exception:
            continue
        if w < 256 or h < 256:
            continue

        buf = BytesIO()
        fmt = "JPEG" if img.mode == "RGB" else "PNG"
        img.save(buf, format=fmt, quality=92)
        img_bytes = buf.getvalue()

        if len(img_bytes) < 5000:
            continue

        sha = hashlib.sha256(img_bytes).hexdigest()
        if sha in existing_hashes:
            skipped_dupes += 1
            continue
        existing_hashes.add(sha)

        ext = ".jpg" if fmt == "JPEG" else ".png"
        out_name = f"flickr30k_{start_idx + downloaded:05d}{ext}"
        out_path = output_dir / out_name
        out_path.write_bytes(img_bytes)

        entries.append({
            "filename": out_name,
            "source": "flickr30k",
            "sha256": sha,
            "size_bytes": len(img_bytes),
            "label": "authentic",
            "downloaded_at": datetime.now(timezone.utc).isoformat(),
        })

        downloaded += 1
        if downloaded % 100 == 0:
            print(f"  [flickr30k] {downloaded}/{max_images} downloaded...")

    print(f"  [flickr30k] {downloaded} images downloaded ({skipped_dupes} dupes skipped)")
    return entries


# Backward-compat alias so existing `--sources openimages` invocations still work.
def download_openimages(output_dir: Path, max_images: int) -> list[dict]:
    """Alias for flickr30k — the OpenImages HF mirror was removed in early 2026."""
    return download_flickr30k(output_dir, max_images)


def download_flickr8k(output_dir: Path, max_images: int) -> list[dict]:
    """Download photographs from Flickr8k via HuggingFace.

    Replaces the previous `unsplash` source (the HF script-based Unsplash
    datasets were deprecated in early 2026). Flickr8k contains ~8,000
    amateur and semi-professional photographs — a good complement to
    Flickr30k (broader subjects) and COCO (annotated scenes).

    Dataset: jxie/flickr8k, split=train.

    Resume-safe: counts existing files and skips duplicates by sha256.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    start_idx = _existing_image_count(output_dir)
    existing_hashes = _existing_hashes(output_dir)
    print(f"\n  [flickr8k] Downloading from Flickr8k (max {max_images}, existing {start_idx})...")

    try:
        from datasets import load_dataset
    except ImportError:
        print("  Error: 'datasets' library required. pip install datasets")
        return []

    try:
        ds = load_dataset("jxie/flickr8k", split="train", streaming=True)
    except Exception as e:
        print(f"  [flickr8k] Failed to load dataset: {e}")
        return []

    entries = []
    downloaded = 0
    skipped_dupes = 0

    for item in ds:
        if downloaded >= max_images:
            break

        img = item.get("image")
        if img is None:
            continue

        try:
            w, h = img.size
        except Exception:
            continue
        if w < 256 or h < 256:
            continue

        buf = BytesIO()
        fmt = "JPEG" if img.mode == "RGB" else "PNG"
        img.save(buf, format=fmt, quality=92)
        img_bytes = buf.getvalue()

        if len(img_bytes) < 5000:
            continue

        sha = hashlib.sha256(img_bytes).hexdigest()
        if sha in existing_hashes:
            skipped_dupes += 1
            continue
        existing_hashes.add(sha)

        ext = ".jpg" if fmt == "JPEG" else ".png"
        out_name = f"flickr8k_{start_idx + downloaded:05d}{ext}"
        out_path = output_dir / out_name
        out_path.write_bytes(img_bytes)

        entries.append({
            "filename": out_name,
            "source": "flickr8k",
            "sha256": sha,
            "size_bytes": len(img_bytes),
            "label": "authentic",
            "downloaded_at": datetime.now(timezone.utc).isoformat(),
        })

        downloaded += 1
        if downloaded % 50 == 0:
            print(f"  [flickr8k] {downloaded}/{max_images} downloaded...")

    print(f"  [flickr8k] {downloaded} images downloaded ({skipped_dupes} dupes skipped)")
    return entries


# Backward-compat alias so existing `--sources unsplash` invocations still work.
def download_unsplash(output_dir: Path, max_images: int) -> list[dict]:
    """Alias for flickr8k — the Unsplash HF mirrors were removed in early 2026."""
    return download_flickr8k(output_dir, max_images)


# CIFAR-10 removed from default sources: 32x32 images are too small for
# forensic analysis and produce 54% false positive rate (upscaling artefacts
# mimic AI texture smoothness). Keep the function for baseline testing but
# do not include in production corpus builds.
SOURCES = {
    "wikimedia": download_wikimedia,
    "openimages": download_openimages,   # alias for flickr30k
    "flickr30k": download_flickr30k,
    "unsplash": download_unsplash,        # alias for flickr8k
    "flickr8k": download_flickr8k,
}


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace — Authentic Image Crawler"
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
        default=config.CORPUS_AUTHENTIC,
        help=f"Output directory (default: {config.CORPUS_AUTHENTIC})",
    )
    args = parser.parse_args()

    sources = [s.strip() for s in args.sources.split(",") if s.strip()]
    invalid = [s for s in sources if s not in SOURCES]
    if invalid:
        print(f"Unknown sources: {invalid}. Available: {list(SOURCES.keys())}")
        sys.exit(1)

    print("=" * 60)
    print("  Jura Trace — Authentic Image Crawler")
    print("=" * 60)

    all_entries = []
    start = time.time()

    for source_key in sources:
        source_dir = args.output / source_key
        entries = SOURCES[source_key](source_dir, args.max)
        all_entries.extend(entries)

    # Write manifest
    args.output.mkdir(parents=True, exist_ok=True)
    manifest_path = args.output / "manifest.json"
    manifest = {
        "corpus_type": "authentic",
        "created_at": datetime.now(timezone.utc).isoformat(),
        "total_images": len(all_entries),
        "sources": list(sources),
        "entries": all_entries,
    }

    if manifest_path.exists():
        try:
            existing = json.loads(manifest_path.read_text())
            existing_entries = existing.get("entries", [])
            existing_hashes = {e["sha256"] for e in existing_entries}
            new_entries = [e for e in all_entries if e["sha256"] not in existing_hashes]
            manifest["entries"] = existing_entries + new_entries
            manifest["total_images"] = len(manifest["entries"])
        except Exception:
            pass

    manifest_path.write_text(json.dumps(manifest, indent=2))

    elapsed = time.time() - start
    print(f"\n{'=' * 60}")
    print(f"  Complete: {manifest['total_images']} authentic images in {elapsed:.1f}s")
    print(f"  Manifest: {manifest_path}")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
