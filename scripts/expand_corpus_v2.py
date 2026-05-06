#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace -- Expanded Corpus Builder v2

Downloads diverse authentic and AI-generated images for classifier training.
Extends the original expand_corpus.py with multiple new sources, pHash
deduplication, format validation, and provenance tracking.

Authentic sources:
  - COCO 2017 validation set (diverse real-world photos)
  - Unsplash Lite dataset (CC0 high-quality photography)
  - Wikimedia Commons featured images (diverse subjects and cameras)
  - Edge-case images (user-supplied: night, scanned film, compressed, etc.)

AI-generated sources:
  - DiffusionDB (Stable Diffusion 1.x -- existing)
  - SDXL-Turbo via diffusers (local generation)
  - Flux.1-dev via HuggingFace Inference API
  - Ideogram v2 via API (requires IDEOGRAM_API_KEY)
  - GPT-4o images (user-supplied directory)
  - Midjourney images (user-supplied directory)

Usage:
    # Download all sources (respects rate limits, resumes on re-run):
    python scripts/expand_corpus_v2.py

    # Download specific sources only:
    python scripts/expand_corpus_v2.py --sources coco,unsplash,wikimedia

    # Dry run (show what would be downloaded):
    python scripts/expand_corpus_v2.py --dry-run

    # Set per-source limits:
    python scripts/expand_corpus_v2.py --coco-max 300 --unsplash-max 200

    # Deduplicate existing corpus:
    python scripts/expand_corpus_v2.py --deduplicate-only
"""

import argparse
import hashlib
import json
import os
import struct
import sys
import time
from datetime import datetime
from io import BytesIO
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BASE_DIR = Path(__file__).resolve().parent.parent
CORPUS_DIR = BASE_DIR / "corpus"
AUTHENTIC_DIR = CORPUS_DIR / "authentic"
AI_DIR = CORPUS_DIR / "ai_generated"
MANIFEST_PATH = CORPUS_DIR / "manifest.json"
SOURCES_PATH = CORPUS_DIR / "SOURCES.md"

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".tiff", ".tif"}
MIN_IMAGE_BYTES = 5_000
MIN_IMAGE_DIM = 224  # Minimum width and height in pixels

USER_AGENT = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)

# pHash Hamming distance threshold for deduplication
# Images with distance <= this are considered near-duplicates
PHASH_DEDUP_THRESHOLD = 10


# ---------------------------------------------------------------------------
# Utility helpers
# ---------------------------------------------------------------------------


def _ensure_dirs():
    """Create corpus directories if they don't exist."""
    AUTHENTIC_DIR.mkdir(parents=True, exist_ok=True)
    AI_DIR.mkdir(parents=True, exist_ok=True)


def _fetch(url: str, timeout: int = 20) -> bytes | None:
    """Fetch a URL with browser-like headers. Returns bytes or None."""
    req = Request(url, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    try:
        with urlopen(req, timeout=timeout) as resp:
            return resp.read()
    except (HTTPError, URLError, TimeoutError) as e:
        print(f"    Warning: fetch failed for {url[:80]}...: {e}")
        return None


def _fetch_json(url: str, headers: dict | None = None, timeout: int = 20) -> dict | None:
    """Fetch a URL and parse as JSON."""
    hdrs = {"User-Agent": USER_AGENT, "Accept": "application/json"}
    if headers:
        hdrs.update(headers)
    req = Request(url, headers=hdrs)
    try:
        with urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except (HTTPError, URLError, TimeoutError, json.JSONDecodeError) as e:
        print(f"    Warning: JSON fetch failed for {url[:80]}...: {e}")
        return None


def _validate_image(data: bytes) -> bool:
    """Check that image data is valid (not truncated, meets minimum size)."""
    if len(data) < MIN_IMAGE_BYTES:
        return False
    try:
        from PIL import Image
        img = Image.open(BytesIO(data))
        img.verify()
        # Re-open after verify (verify closes the file)
        img = Image.open(BytesIO(data))
        w, h = img.size
        if w < MIN_IMAGE_DIM or h < MIN_IMAGE_DIM:
            return False
        return True
    except Exception:
        return False


def _compute_phash(data: bytes) -> str | None:
    """Compute perceptual hash of image data. Returns hex string or None."""
    try:
        from PIL import Image
        import imagehash
        img = Image.open(BytesIO(data))
        h = imagehash.phash(img, hash_size=16)
        return str(h)
    except Exception:
        return None


def _hamming_distance(h1: str, h2: str) -> int:
    """Compute Hamming distance between two hex hash strings."""
    if len(h1) != len(h2):
        return 999
    # Convert hex to binary comparison
    n1 = int(h1, 16)
    n2 = int(h2, 16)
    return bin(n1 ^ n2).count("1")


def _detect_mime(data: bytes, path: str = "") -> str:
    """Detect MIME type from file header bytes."""
    if data[:4] == b"\x89PNG":
        return "image/png"
    if data[:2] == b"\xff\xd8":
        return "image/jpeg"
    if data[:4] == b"RIFF" and len(data) > 12 and data[8:12] == b"WEBP":
        return "image/webp"
    ext = Path(path).suffix.lower()
    return {
        ".png": "image/png",
        ".jpg": "image/jpeg",
        ".jpeg": "image/jpeg",
        ".webp": "image/webp",
        ".tiff": "image/tiff",
        ".tif": "image/tiff",
    }.get(ext, "application/octet-stream")


def _save_image(
    data: bytes,
    output_dir: Path,
    filename: str,
    existing_hashes: dict[str, str] | None = None,
) -> tuple[str | None, str | None]:
    """Save image data to disk after validation and deduplication.

    Returns (filename, phash_hex) or (None, None) if skipped.
    """
    filepath = output_dir / filename
    if filepath.exists():
        return filename, None  # Already exists

    if not _validate_image(data):
        return None, None

    # Compute pHash for deduplication
    phash = _compute_phash(data)
    if phash and existing_hashes:
        for existing_file, existing_hash in existing_hashes.items():
            dist = _hamming_distance(phash, existing_hash)
            if dist <= PHASH_DEDUP_THRESHOLD:
                return None, None  # Near-duplicate

    filepath.write_bytes(data)

    return filename, phash


# ---------------------------------------------------------------------------
# Authentic image sources
# ---------------------------------------------------------------------------


def download_coco_val2017(output_dir: Path, max_images: int = 300, existing_hashes: dict | None = None) -> list[dict]:
    """Download diverse images from COCO 2017 validation set.

    Uses the COCO API to get a category-balanced sample rather than
    sequential IDs, ensuring scene diversity.
    """
    print(f"\n  [COCO val2017] Downloading up to {max_images} images...")

    entries = []
    downloaded = 0

    # COCO val2017 annotations URL for category-balanced sampling
    # First try to load the annotation file for balanced sampling
    coco_base = "http://images.cocodataset.org/val2017/"

    # Try to get balanced categories via the annotation file
    try:
        from pycocotools.coco import COCO
        import tempfile

        print("    Downloading annotation index for balanced sampling...")
        ann_url = "http://images.cocodataset.org/annotations/instances_val2017.json"
        ann_data = _fetch(ann_url, timeout=120)
        if ann_data:
            ann_path = Path(tempfile.mktemp(suffix=".json"))
            ann_path.write_bytes(ann_data)
            coco = COCO(str(ann_path))

            # Get images balanced across super-categories
            all_cat_ids = coco.getCatIds()
            images_per_cat = max(max_images // len(all_cat_ids), 3)
            selected_ids = set()

            for cat_id in all_cat_ids:
                img_ids = coco.getImgIds(catIds=[cat_id])
                for img_id in img_ids[:images_per_cat]:
                    selected_ids.add(img_id)
                    if len(selected_ids) >= max_images:
                        break
                if len(selected_ids) >= max_images:
                    break

            ann_path.unlink(missing_ok=True)
            image_ids = list(selected_ids)[:max_images]
            print(f"    Selected {len(image_ids)} balanced images across {len(all_cat_ids)} categories")
        else:
            raise RuntimeError("Could not download annotations")

    except Exception as e:
        print(f"    pycocotools not available ({e}), falling back to sequential IDs...")
        # Fallback: use a broader range of known val2017 IDs
        # These are spread across the full 5000-image val set for diversity
        image_ids = list(range(139, 15500, max(1, 15500 // max_images)))[:max_images]

    for i, img_id in enumerate(image_ids):
        if downloaded >= max_images:
            break

        # Format as 12-digit zero-padded filename
        img_id_str = f"{img_id:012d}" if isinstance(img_id, int) else str(img_id)
        filename = f"coco_{img_id_str}.jpg"
        filepath = output_dir / filename

        if filepath.exists():
            downloaded += 1
            if downloaded % 50 == 0:
                print(f"    {downloaded}/{max_images} (cached)...")
            continue

        url = f"{coco_base}{img_id_str}.jpg"
        data = _fetch(url, timeout=15)
        if data is None:
            continue

        saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
        if saved_name:
            downloaded += 1
            entries.append({
                "filename": saved_name,
                "source": "coco-val2017",
                "source_url": url,
                "label": "authentic",
                "licence": "CC BY 2.0 (Flickr)",
                "phash": phash,
                "downloaded_at": datetime.now().isoformat(),
            })
            if phash and existing_hashes is not None:
                existing_hashes[saved_name] = phash

            if downloaded % 50 == 0:
                print(f"    {downloaded}/{max_images}...")

        time.sleep(0.1)  # Polite rate limiting

    print(f"    Downloaded {downloaded} COCO images")
    return entries


def download_unsplash_lite(output_dir: Path, max_images: int = 200, existing_hashes: dict | None = None) -> list[dict]:
    """Download images from the Unsplash Lite dataset via HuggingFace.

    The Unsplash Lite dataset contains 25,000 CC0-licensed photographs
    with diverse subjects, lighting, and camera equipment.
    """
    print(f"\n  [Unsplash Lite] Downloading up to {max_images} images...")

    entries = []
    downloaded = 0

    try:
        from datasets import load_dataset
    except ImportError:
        print("    ERROR: 'datasets' library not installed. Install with: pip install datasets")
        return entries

    try:
        # The Unsplash Lite dataset on HuggingFace
        # Try the official unsplash dataset first
        ds = load_dataset("unsplash/lite", split="train", streaming=True)
        print("    Loaded unsplash/lite dataset (streaming mode)")
    except Exception as e:
        print(f"    Warning: could not load unsplash/lite: {e}")
        # Fallback: try alternative Unsplash dataset
        try:
            ds = load_dataset("Chr0my/Unsplash-Photos-Lite", split="train", streaming=True)
            print("    Loaded fallback Unsplash dataset (streaming mode)")
        except Exception as e2:
            print(f"    ERROR: no Unsplash dataset available: {e2}")
            return entries

    for item in ds:
        if downloaded >= max_images:
            break

        # Dataset may contain image objects or URLs
        img = item.get("image") or item.get("photo")
        photo_url = item.get("photo_image_url") or item.get("url")
        photo_id = item.get("photo_id") or item.get("id") or hashlib.md5(str(item).encode()).hexdigest()[:10]

        if img is not None:
            # Direct image object from HuggingFace
            buf = BytesIO()
            img.save(buf, format="JPEG", quality=92)
            data = buf.getvalue()
        elif photo_url:
            # URL-based dataset -- download the image
            # Use the small version if available
            small_url = photo_url
            if "?" not in small_url:
                small_url += "?w=1024"
            data = _fetch(small_url, timeout=15)
            if data is None:
                continue
        else:
            continue

        filename = f"unsplash_{photo_id}.jpg"
        saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)

        if saved_name:
            downloaded += 1
            entries.append({
                "filename": saved_name,
                "source": "unsplash-lite",
                "label": "authentic",
                "licence": "CC0 (Unsplash Lite)",
                "phash": phash,
                "downloaded_at": datetime.now().isoformat(),
            })
            if phash and existing_hashes is not None:
                existing_hashes[saved_name] = phash

            if downloaded % 50 == 0:
                print(f"    {downloaded}/{max_images}...")

        time.sleep(0.05)

    print(f"    Downloaded {downloaded} Unsplash images")
    return entries


def download_wikimedia_featured(output_dir: Path, max_images: int = 150, existing_hashes: dict | None = None) -> list[dict]:
    """Download featured photographs from Wikimedia Commons.

    Uses the MediaWiki API to find recently featured photographs.
    """
    print(f"\n  [Wikimedia Commons] Downloading up to {max_images} featured images...")

    entries = []
    downloaded = 0

    # Query Wikimedia Commons API for featured pictures (photographs only)
    api_base = "https://commons.wikimedia.org/w/api.php"

    # Get members of Category:Featured_pictures_of_photographs
    params = {
        "action": "query",
        "list": "categorymembers",
        "cmtitle": "Category:Featured_pictures_on_Wikimedia_Commons",
        "cmtype": "file",
        "cmlimit": str(min(max_images * 2, 500)),  # Over-fetch to filter non-photos
        "format": "json",
    }
    query_url = f"{api_base}?{'&'.join(f'{k}={v}' for k, v in params.items())}"
    result = _fetch_json(query_url)

    if not result or "query" not in result:
        print("    Warning: Wikimedia API returned no results")
        return entries

    members = result["query"].get("categorymembers", [])
    print(f"    Found {len(members)} featured files to check")

    for member in members:
        if downloaded >= max_images:
            break

        title = member.get("title", "")
        if not any(title.lower().endswith(ext) for ext in (".jpg", ".jpeg", ".png")):
            continue

        # Get the actual image URL via imageinfo
        info_params = {
            "action": "query",
            "titles": title.replace(" ", "_"),
            "prop": "imageinfo",
            "iiprop": "url|size|mime",
            "iiurlwidth": "1024",  # Get a reasonable-sized thumbnail
            "format": "json",
        }
        info_url = f"{api_base}?{'&'.join(f'{k}={v}' for k, v in info_params.items())}"
        info = _fetch_json(info_url)

        if not info or "query" not in info:
            continue

        pages = info["query"].get("pages", {})
        for page_id, page_data in pages.items():
            imageinfo = page_data.get("imageinfo", [{}])
            if not imageinfo:
                continue

            img_info = imageinfo[0]
            mime = img_info.get("mime", "")
            if not mime.startswith("image/"):
                continue

            # Prefer the thumbnail URL (1024px wide) over full size
            img_url = img_info.get("thumburl") or img_info.get("url")
            if not img_url:
                continue

            # Clean up filename
            safe_title = title.replace("File:", "").replace(" ", "_")[:50]
            img_hash = hashlib.md5(img_url.encode()).hexdigest()[:8]
            ext = ".jpg" if "jpeg" in mime else ".png"
            filename = f"wikimedia_{img_hash}_{safe_title}{ext}"
            # Remove problematic characters from filename
            filename = "".join(c for c in filename if c.isalnum() or c in "._-")

            data = _fetch(img_url, timeout=20)
            if data is None:
                continue

            saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
            if saved_name:
                downloaded += 1
                entries.append({
                    "filename": saved_name,
                    "source": "wikimedia-featured",
                    "source_url": img_url,
                    "label": "authentic",
                    "licence": img_info.get("extmetadata", {}).get("LicenseShortName", {}).get("value", "CC BY-SA"),
                    "phash": phash,
                    "downloaded_at": datetime.now().isoformat(),
                })
                if phash and existing_hashes is not None:
                    existing_hashes[saved_name] = phash

                if downloaded % 30 == 0:
                    print(f"    {downloaded}/{max_images}...")

        time.sleep(0.3)  # Wikimedia rate limit

    print(f"    Downloaded {downloaded} Wikimedia images")
    return entries


def copy_edge_cases(source_dir: Path, output_dir: Path, existing_hashes: dict | None = None) -> list[dict]:
    """Copy user-supplied edge-case authentic images.

    These are the highest-value additions: night photography, scanned film,
    heavy compression, medical imaging, satellite imagery, social media
    re-uploads, computational photography (HDR, portrait mode, etc.).

    Expected directory structure:
        corpus/edge_cases/
            night/
            scanned_film/
            compressed/
            computational_photography/
            social_media/
            miscellaneous/
    """
    print(f"\n  [Edge cases] Checking {source_dir}...")

    if not source_dir.exists():
        print(f"    Directory not found: {source_dir}")
        print("    Create it and add edge-case images, then re-run.")
        print("    Suggested subdirectories: night/, scanned_film/, compressed/,")
        print("    computational_photography/, social_media/, miscellaneous/")
        return []

    entries = []
    copied = 0

    for subdir in sorted(source_dir.iterdir()):
        if subdir.is_file() and subdir.suffix.lower() in IMAGE_EXTENSIONS:
            # Files directly in edge_cases/
            _copy_one_edge_case(subdir, "miscellaneous", output_dir, existing_hashes, entries)
            copied += 1
        elif subdir.is_dir():
            category = subdir.name
            for f in sorted(subdir.iterdir()):
                if f.suffix.lower() in IMAGE_EXTENSIONS:
                    result = _copy_one_edge_case(f, category, output_dir, existing_hashes, entries)
                    if result:
                        copied += 1

    print(f"    Copied {copied} edge-case images")
    return entries


def _copy_one_edge_case(
    src: Path, category: str, output_dir: Path,
    existing_hashes: dict | None, entries: list[dict],
) -> bool:
    """Copy a single edge-case image to the corpus."""
    data = src.read_bytes()
    filename = f"edge_{category}_{src.name}"
    # Sanitise filename
    filename = "".join(c for c in filename if c.isalnum() or c in "._-")

    saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
    if saved_name:
        entries.append({
            "filename": saved_name,
            "source": f"edge-case-{category}",
            "label": "authentic",
            "licence": "Internal calibration only",
            "category": category,
            "original_name": src.name,
            "phash": phash,
            "downloaded_at": datetime.now().isoformat(),
        })
        if phash and existing_hashes is not None:
            existing_hashes[saved_name] = phash
        return True
    return False


# ---------------------------------------------------------------------------
# AI-generated image sources
# ---------------------------------------------------------------------------


def download_diffusiondb(output_dir: Path, max_images: int = 100, existing_hashes: dict | None = None) -> list[dict]:
    """Download from DiffusionDB (Stable Diffusion 1.x outputs)."""
    print(f"\n  [DiffusionDB] Downloading up to {max_images} images...")

    try:
        from datasets import load_dataset
    except ImportError:
        print("    ERROR: 'datasets' library not installed")
        return []

    entries = []
    downloaded = 0

    try:
        ds = load_dataset("poloclub/diffusiondb", "random_1k", split="train")
    except Exception as e:
        print(f"    Warning: could not load diffusiondb: {e}")
        return entries

    for item in ds:
        if downloaded >= max_images:
            break

        img = item.get("image")
        if img is None:
            continue

        buf = BytesIO()
        img.save(buf, format="PNG")
        data = buf.getvalue()

        img_hash = hashlib.md5(data).hexdigest()[:10]
        filename = f"diffusiondb_{img_hash}.png"

        saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
        if saved_name:
            downloaded += 1
            prompt = item.get("prompt", "unknown")
            entries.append({
                "filename": saved_name,
                "source": "diffusiondb-random-1k",
                "label": "ai_generated",
                "generator": "stable_diffusion_v1",
                "prompt": prompt[:200] if isinstance(prompt, str) else "",
                "phash": phash,
                "downloaded_at": datetime.now().isoformat(),
            })
            if phash and existing_hashes is not None:
                existing_hashes[saved_name] = phash

            if downloaded % 30 == 0:
                print(f"    {downloaded}/{max_images}...")

    print(f"    Downloaded {downloaded} DiffusionDB images")
    return entries


def generate_sdxl_turbo(output_dir: Path, max_images: int = 100, existing_hashes: dict | None = None) -> list[dict]:
    """Generate images locally using SDXL-Turbo via diffusers.

    Requires: pip install diffusers torch transformers accelerate
    SDXL-Turbo is fast (~1s per image on M1) and produces high-quality outputs.
    """
    print(f"\n  [SDXL-Turbo] Generating up to {max_images} images locally...")

    try:
        import torch
        from diffusers import AutoPipelineForText2Image
    except ImportError:
        print("    SDXL-Turbo requires: pip install diffusers torch transformers accelerate")
        print("    Skipping local generation.")
        return []

    entries = []
    generated = 0

    # Diverse photorealistic prompts designed to test classifier robustness
    prompts = [
        "A professional photograph of a busy city street at sunset",
        "Portrait of an elderly woman smiling, natural lighting",
        "Aerial view of a coastal town with blue water",
        "A dog running through a park, motion blur, sunny day",
        "Close-up photograph of fresh fruit at a market stall",
        "Black and white photograph of a jazz musician on stage",
        "Night photography of a neon-lit alleyway in Tokyo",
        "Underwater photograph of a coral reef with tropical fish",
        "Documentary photograph of a crowded train station",
        "Macro photograph of a butterfly on a flower",
        "A photojournalist's image of a protest march",
        "Wedding photograph of a couple in a garden",
        "Sports photograph of a football match in progress",
        "Landscape photograph of misty mountains at dawn",
        "Interior photograph of a modern kitchen",
        "Street photograph of a child playing with a ball",
        "Food photography of a gourmet meal on a white plate",
        "Fashion photograph of a model in a studio",
        "Medical photograph of a surgical procedure",
        "Satellite image of a river delta",
        "Photograph of an old church in rural England",
        "HDR photograph of a thunderstorm over a field",
        "Photograph of a busy restaurant kitchen",
        "Studio portrait with dramatic side lighting",
        "Photograph of autumn leaves floating on a pond",
        "Candid photograph at a family dinner table",
        "Photograph of a construction site with cranes",
        "A photograph of a crowded beach in summer",
        "Photograph of a foggy forest path",
        "Photograph of fireworks over a city skyline",
        "A high-resolution photograph of a vintage car",
        "Photograph of a mountain climber on a cliff face",
        "Photograph of a busy Asian night market",
        "Photograph of snow-covered rooftops in a village",
        "Photograph of a ballet dancer mid-leap on stage",
        "Photograph of a laboratory with scientific equipment",
        "A photorealistic image of a library interior",
        "Photograph of a farmer working in a wheat field",
        "Photograph of an astronaut floating in space",
        "Photograph of a modern glass office building",
        "Photograph of a cat sitting in a window",
        "Photograph of a historic castle on a hilltop",
        "Photograph of a surfer riding a large wave",
        "Photograph of a colourful graffiti wall in Berlin",
        "Photograph of a hot air balloon over Cappadocia",
        "Photograph of a rainy street with reflections",
        "Photograph of an orchestra performing on stage",
        "Photograph of a mechanic working under a car",
        "Photograph of a sleeping baby in a crib",
        "Photograph of a desert landscape with sand dunes",
    ]

    try:
        pipe = AutoPipelineForText2Image.from_pretrained(
            "stabilityai/sdxl-turbo",
            torch_dtype=torch.float16 if torch.cuda.is_available() else torch.float32,
            variant="fp16" if torch.cuda.is_available() else None,
        )
        if torch.backends.mps.is_available():
            pipe = pipe.to("mps")
        elif torch.cuda.is_available():
            pipe = pipe.to("cuda")
        print("    Model loaded.")
    except Exception as e:
        print(f"    Could not load SDXL-Turbo: {e}")
        return entries

    prompt_idx = 0
    while generated < max_images:
        prompt = prompts[prompt_idx % len(prompts)]
        # Vary the seed for each generation to get diverse outputs
        seed = 42 + generated + (prompt_idx // len(prompts)) * 1000

        try:
            generator = torch.Generator().manual_seed(seed)
            image = pipe(
                prompt=prompt,
                num_inference_steps=4,  # SDXL-Turbo uses few steps
                guidance_scale=0.0,  # Turbo doesn't use guidance
                generator=generator,
            ).images[0]

            buf = BytesIO()
            image.save(buf, format="PNG")
            data = buf.getvalue()

            img_hash = hashlib.md5(data).hexdigest()[:8]
            filename = f"sdxl_turbo_{img_hash}.png"

            saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
            if saved_name:
                generated += 1
                entries.append({
                    "filename": saved_name,
                    "source": "sdxl-turbo-local",
                    "label": "ai_generated",
                    "generator": "sdxl_turbo",
                    "prompt": prompt,
                    "seed": seed,
                    "phash": phash,
                    "downloaded_at": datetime.now().isoformat(),
                })
                if phash and existing_hashes is not None:
                    existing_hashes[saved_name] = phash

                if generated % 10 == 0:
                    print(f"    {generated}/{max_images}...")

        except Exception as e:
            print(f"    Warning: generation failed for prompt '{prompt[:40]}...': {e}")

        prompt_idx += 1
        if prompt_idx >= len(prompts) * 5:
            # Safety: stop if we've exhausted prompt variations
            break

    print(f"    Generated {generated} SDXL-Turbo images")
    return entries


def download_flux_dev(output_dir: Path, max_images: int = 100, existing_hashes: dict | None = None) -> list[dict]:
    """Download images generated by Flux.1-dev via HuggingFace Inference API.

    Requires HF_TOKEN environment variable with a valid HuggingFace token.
    The free tier supports a limited number of requests per hour.
    """
    print(f"\n  [Flux.1-dev] Generating up to {max_images} images via HF Inference API...")

    hf_token = os.environ.get("HF_TOKEN")
    if not hf_token:
        print("    HF_TOKEN not set. Set it with: export HF_TOKEN=hf_xxx")
        print("    Skipping Flux.1-dev generation.")
        return []

    entries = []
    generated = 0

    # Flux.1-dev produces very high-quality photorealistic outputs
    prompts = [
        "Professional press photograph of a political debate",
        "Award-winning nature photograph of a kingfisher diving",
        "Photograph from a war zone showing destroyed buildings",
        "Studio portrait of a CEO for a magazine cover",
        "Photograph of a refugee camp from above",
        "Microscopy photograph of cells under fluorescent light",
        "Photograph of the interior of a cathedral",
        "Photograph of a traditional Japanese tea ceremony",
        "Photograph of firefighters battling a forest fire",
        "Security camera still frame of a parking lot",
        "Photograph of a newborn in a hospital",
        "Fashion editorial photograph in black and white",
        "Drone photograph of a winding mountain road",
        "Photograph of a protest with police in riot gear",
        "Close-up photograph of an eye with a reflection",
        "Archaeological photograph of a dig site",
        "Photograph of an abandoned factory interior",
        "Selfie taken with a phone camera, slightly blurry",
        "Photograph of a market in Marrakech",
        "Sport photograph of a tennis serve, frozen motion",
        "Photograph of an oil painting in a museum gallery",
        "Photograph of a volcanic eruption at night",
        "Candid photograph of friends laughing at a cafe",
        "Photograph of a crowded subway car in New York",
        "Architectural photograph of a brutalist concrete building",
        "Photograph of a farmer's market with fresh vegetables",
        "Photograph of a chess tournament in progress",
        "Photograph of a child's birthday party",
        "Photograph of a lighthouse in a storm",
        "Photograph of a ballet rehearsal in a studio",
        "Photograph of a tattoo artist at work",
        "Photograph of rush hour traffic from above",
        "Photograph of a traditional weaving workshop",
        "Photograph of a polar bear on melting ice",
        "Medical X-ray photograph of a broken bone",
        "Photograph of a crowded music festival at night",
        "Photograph of a baker pulling bread from an oven",
        "Photograph of a shipwreck on a beach",
        "Photograph of a newborn baby's hand gripping a finger",
        "Photograph of the International Space Station from Earth",
        "Photograph of a wild horse running across a steppe",
        "Photograph of an elderly couple walking hand in hand",
        "Photograph of a science lab with bubbling flasks",
        "Photograph of a flooded street after a hurricane",
        "Photograph of a mime performer in a park",
        "Photograph of a sunset over a calm lake",
        "Photograph of a traditional fishing boat at sea",
        "Photograph of a skateboarder doing a trick",
        "Photograph of a snowy mountain peak at sunrise",
        "Photograph of a busy hospital emergency room",
    ]

    api_url = "https://api-inference.huggingface.co/models/black-forest-labs/FLUX.1-dev"

    prompt_idx = 0
    consecutive_errors = 0

    while generated < max_images and prompt_idx < len(prompts) * 3:
        prompt = prompts[prompt_idx % len(prompts)]

        req = Request(
            api_url,
            data=json.dumps({"inputs": prompt}).encode("utf-8"),
            headers={
                "Authorization": f"Bearer {hf_token}",
                "Content-Type": "application/json",
                "Accept": "image/png",
            },
            method="POST",
        )

        try:
            with urlopen(req, timeout=120) as resp:
                data = resp.read()

            if len(data) < MIN_IMAGE_BYTES:
                # Possibly an error response in JSON
                try:
                    err = json.loads(data)
                    if err.get("error"):
                        wait = err.get("estimated_time", 30)
                        print(f"    Model loading, waiting {wait}s...")
                        time.sleep(min(wait, 60))
                        consecutive_errors += 1
                        if consecutive_errors > 5:
                            print("    Too many errors, stopping Flux generation.")
                            break
                        prompt_idx += 1
                        continue
                except json.JSONDecodeError:
                    pass
                prompt_idx += 1
                continue

            consecutive_errors = 0
            img_hash = hashlib.md5(data).hexdigest()[:8]
            filename = f"flux1dev_{img_hash}.png"

            saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
            if saved_name:
                generated += 1
                entries.append({
                    "filename": saved_name,
                    "source": "flux1-dev-hf-inference",
                    "label": "ai_generated",
                    "generator": "flux1_dev",
                    "prompt": prompt,
                    "phash": phash,
                    "downloaded_at": datetime.now().isoformat(),
                })
                if phash and existing_hashes is not None:
                    existing_hashes[saved_name] = phash

                if generated % 10 == 0:
                    print(f"    {generated}/{max_images}...")

        except HTTPError as e:
            if e.code == 429:
                print("    Rate limited, waiting 60s...")
                time.sleep(60)
                consecutive_errors += 1
            elif e.code == 503:
                print("    Model loading, waiting 30s...")
                time.sleep(30)
                consecutive_errors += 1
            else:
                print(f"    HTTP error {e.code}: {e}")
                consecutive_errors += 1
        except Exception as e:
            print(f"    Error: {e}")
            consecutive_errors += 1

        if consecutive_errors > 10:
            print("    Too many consecutive errors, stopping.")
            break

        prompt_idx += 1
        time.sleep(2)  # Rate limit: ~30 req/min on free tier

    print(f"    Generated {generated} Flux.1-dev images")
    return entries


def download_ideogram_v2(output_dir: Path, max_images: int = 80, existing_hashes: dict | None = None) -> list[dict]:
    """Generate images via Ideogram v2 API.

    Requires IDEOGRAM_API_KEY environment variable.
    """
    print(f"\n  [Ideogram v2] Generating up to {max_images} images...")

    api_key = os.environ.get("IDEOGRAM_API_KEY")
    if not api_key:
        print("    IDEOGRAM_API_KEY not set. Skipping.")
        return []

    entries = []
    generated = 0

    prompts = [
        "A realistic photograph of a busy London street",
        "Professional headshot photograph for a corporate website",
        "Editorial photograph of a chef in a restaurant kitchen",
        "Landscape photograph of the Scottish Highlands",
        "Wildlife photograph of an eagle in flight",
        "Street photography of a rainy evening in Paris",
        "Photograph of a university graduation ceremony",
        "Photograph of an industrial warehouse interior",
        "Photograph of a traditional Indian wedding",
        "Candid photograph of a street musician playing guitar",
        "Photograph of a scientific laboratory with researchers",
        "Photograph of a vineyard in autumn",
        "Photograph of a crowded fish market in Tokyo",
        "Photograph of a modern art gallery interior",
        "Photograph of a mountain village in the Alps",
        "Photograph of a surfing competition",
        "Photograph of a blacksmith at work",
        "Photograph of Northern Lights over a fjord",
        "Photograph of a traditional bakery in France",
        "Photograph of a desert road stretching to the horizon",
    ]

    api_url = "https://api.ideogram.ai/generate"

    for i, prompt in enumerate(prompts):
        if generated >= max_images:
            break

        # Generate multiple images per prompt
        images_per_prompt = min(4, max_images - generated)

        req_data = json.dumps({
            "image_request": {
                "prompt": prompt,
                "model": "V_2",
                "magic_prompt_option": "OFF",
                "num_images": images_per_prompt,
                "resolution": "RESOLUTION_1024_1024",
            }
        }).encode("utf-8")

        req = Request(
            api_url,
            data=req_data,
            headers={
                "Api-Key": api_key,
                "Content-Type": "application/json",
            },
            method="POST",
        )

        try:
            with urlopen(req, timeout=60) as resp:
                result = json.loads(resp.read())

            images = result.get("data", [])
            for img_data in images:
                img_url = img_data.get("url")
                if not img_url:
                    continue

                data = _fetch(img_url, timeout=30)
                if data is None:
                    continue

                img_hash = hashlib.md5(data).hexdigest()[:8]
                filename = f"ideogram_v2_{img_hash}.png"

                saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
                if saved_name:
                    generated += 1
                    entries.append({
                        "filename": saved_name,
                        "source": "ideogram-v2-api",
                        "label": "ai_generated",
                        "generator": "ideogram_v2",
                        "prompt": prompt,
                        "phash": phash,
                        "downloaded_at": datetime.now().isoformat(),
                    })
                    if phash and existing_hashes is not None:
                        existing_hashes[saved_name] = phash

                    if generated % 10 == 0:
                        print(f"    {generated}/{max_images}...")

        except Exception as e:
            print(f"    Error: {e}")

        time.sleep(1)

    print(f"    Generated {generated} Ideogram v2 images")
    return entries


def copy_local_ai_images(source_dir: Path, output_dir: Path, generator_name: str, existing_hashes: dict | None = None) -> list[dict]:
    """Copy local AI-generated images (GPT-4o, Midjourney, etc.) to corpus."""
    print(f"\n  [Local: {generator_name}] Checking {source_dir}...")

    if not source_dir.exists():
        print(f"    Directory not found: {source_dir}")
        return []

    entries = []
    copied = 0

    for f in sorted(source_dir.iterdir()):
        if f.suffix.lower() not in IMAGE_EXTENSIONS:
            continue

        data = f.read_bytes()
        safe_gen = generator_name.lower().replace(" ", "_").replace("-", "_")
        img_hash = hashlib.md5(data).hexdigest()[:8]
        filename = f"{safe_gen}_{img_hash}{f.suffix.lower()}"

        saved_name, phash = _save_image(data, output_dir, filename, existing_hashes)
        if saved_name:
            copied += 1
            entries.append({
                "filename": saved_name,
                "source": f"local-{safe_gen}",
                "label": "ai_generated",
                "generator": safe_gen,
                "original_name": f.name,
                "phash": phash,
                "downloaded_at": datetime.now().isoformat(),
            })
            if phash and existing_hashes is not None:
                existing_hashes[saved_name] = phash

    print(f"    Copied {copied} {generator_name} images")
    return entries


# ---------------------------------------------------------------------------
# Deduplication
# ---------------------------------------------------------------------------


def build_hash_index(directory: Path) -> dict[str, str]:
    """Build pHash index for all images in a directory. Returns {filename: hash_hex}."""
    print(f"  Building pHash index for {directory}...")
    index = {}
    count = 0

    for f in sorted(directory.iterdir()):
        if f.suffix.lower() not in IMAGE_EXTENSIONS:
            continue
        try:
            data = f.read_bytes()
            phash = _compute_phash(data)
            if phash:
                index[f.name] = phash
                count += 1
        except Exception:
            pass

        if count % 100 == 0 and count > 0:
            print(f"    {count} images hashed...")

    print(f"    Indexed {count} images")
    return index


def find_duplicates(hash_index: dict[str, str], threshold: int = PHASH_DEDUP_THRESHOLD) -> list[tuple[str, str, int]]:
    """Find near-duplicate pairs in a hash index.

    Returns list of (file1, file2, hamming_distance) tuples.
    """
    items = list(hash_index.items())
    duplicates = []

    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            name_i, hash_i = items[i]
            name_j, hash_j = items[j]
            dist = _hamming_distance(hash_i, hash_j)
            if dist <= threshold:
                duplicates.append((name_i, name_j, dist))

    return duplicates


# ---------------------------------------------------------------------------
# Manifest and reporting
# ---------------------------------------------------------------------------


def write_manifest(all_entries: list[dict]):
    """Write the unified corpus manifest."""
    # Load existing manifest if present
    existing_entries = []
    if MANIFEST_PATH.exists():
        try:
            old = json.loads(MANIFEST_PATH.read_text())
            existing_entries = old.get("images", [])
        except Exception:
            pass

    # Merge: existing entries + new entries, deduplicated by filename
    seen_filenames = set()
    merged = []
    for entry in existing_entries + all_entries:
        fn = entry.get("filename")
        if fn and fn not in seen_filenames:
            seen_filenames.add(fn)
            merged.append(entry)

    # Compute statistics
    authentic_count = sum(1 for e in merged if e.get("label") == "authentic")
    ai_count = sum(1 for e in merged if e.get("label") == "ai_generated")

    # Source breakdown
    sources = {}
    for e in merged:
        src = e.get("source", "unknown")
        sources[src] = sources.get(src, 0) + 1

    manifest = {
        "corpus_name": "jura-trace-training-v2",
        "description": "Expanded training corpus for AI-generated image detection. "
                       "Multiple authentic and AI sources for classifier diversity.",
        "created_at": datetime.now().isoformat(),
        "total_images": len(merged),
        "authentic_count": authentic_count,
        "ai_generated_count": ai_count,
        "source_breakdown": sources,
        "dedup_threshold": PHASH_DEDUP_THRESHOLD,
        "min_image_dim": MIN_IMAGE_DIM,
        "images": merged,
    }

    MANIFEST_PATH.write_text(json.dumps(manifest, indent=2))
    print(f"\n  Manifest written: {MANIFEST_PATH}")
    print(f"  Total: {len(merged)} images ({authentic_count} authentic, {ai_count} AI)")
    print(f"  Sources: {json.dumps(sources, indent=4)}")


def write_sources_doc(all_entries: list[dict]):
    """Write SOURCES.md documenting all corpus sources."""
    sources = {}
    for e in all_entries:
        src = e.get("source", "unknown")
        if src not in sources:
            sources[src] = {
                "count": 0,
                "label": e.get("label", "unknown"),
                "licence": e.get("licence", "See source"),
                "generator": e.get("generator"),
            }
        sources[src]["count"] += 1

    lines = [
        "# Jura Trace Training Corpus — Sources",
        "",
        f"**Last updated:** {datetime.now().strftime('%Y-%m-%d')}",
        "",
        "This document lists all image sources used in the training corpus.",
        "Images are used solely for internal model calibration and are not redistributed.",
        "",
        "## Authentic Images",
        "",
        "| Source | Count | Licence | Notes |",
        "|--------|-------|---------|-------|",
    ]

    for src, info in sorted(sources.items()):
        if info["label"] == "authentic":
            lines.append(f"| {src} | {info['count']} | {info['licence']} | |")

    lines.extend([
        "",
        "## AI-Generated Images",
        "",
        "| Source | Count | Generator | Notes |",
        "|--------|-------|-----------|-------|",
    ])

    for src, info in sorted(sources.items()):
        if info["label"] == "ai_generated":
            gen = info.get("generator") or "mixed"
            lines.append(f"| {src} | {info['count']} | {gen} | |")

    total_auth = sum(i["count"] for i in sources.values() if i["label"] == "authentic")
    total_ai = sum(i["count"] for i in sources.values() if i["label"] == "ai_generated")

    lines.extend([
        "",
        "## Summary",
        "",
        f"- **Authentic images:** {total_auth}",
        f"- **AI-generated images:** {total_ai}",
        f"- **Total:** {total_auth + total_ai}",
        f"- **Deduplication:** pHash Hamming distance threshold = {PHASH_DEDUP_THRESHOLD}",
        f"- **Minimum image dimensions:** {MIN_IMAGE_DIM}x{MIN_IMAGE_DIM} pixels",
        "",
        "## Copyright Notice",
        "",
        "All images in this corpus are used solely for internal detector calibration",
        "and model training. They are not redistributed. Individual image copyrights",
        "remain with their respective creators. COCO images are CC BY 2.0 (Flickr).",
        "Unsplash Lite images are CC0. Wikimedia images are CC BY-SA unless noted.",
        "AI-generated images are outputs of generative models with no copyright claim.",
        "",
    ])

    SOURCES_PATH.write_text("\n".join(lines))
    print(f"  Sources documentation: {SOURCES_PATH}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


ALL_SOURCES = [
    "coco", "unsplash", "wikimedia", "edge_cases",
    "diffusiondb", "sdxl_turbo", "flux", "ideogram",
    "local_ai",
]


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace -- Expanded Corpus Builder v2",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Sources:
  Authentic: coco, unsplash, wikimedia, edge_cases
  AI:        diffusiondb, sdxl_turbo, flux, ideogram, local_ai

Examples:
  python scripts/expand_corpus_v2.py --sources coco,unsplash
  python scripts/expand_corpus_v2.py --sources sdxl_turbo --sdxl-max 50
  python scripts/expand_corpus_v2.py --deduplicate-only
        """,
    )
    parser.add_argument(
        "--sources", type=str, default="all",
        help=f"Comma-separated source names or 'all' (default: all). Options: {','.join(ALL_SOURCES)}",
    )
    parser.add_argument("--coco-max", type=int, default=300, help="Max COCO images (default: 300)")
    parser.add_argument("--unsplash-max", type=int, default=200, help="Max Unsplash images (default: 200)")
    parser.add_argument("--wikimedia-max", type=int, default=150, help="Max Wikimedia images (default: 150)")
    parser.add_argument("--diffusiondb-max", type=int, default=100, help="Max DiffusionDB images (default: 100)")
    parser.add_argument("--sdxl-max", type=int, default=100, help="Max SDXL-Turbo images (default: 100)")
    parser.add_argument("--flux-max", type=int, default=100, help="Max Flux.1-dev images (default: 100)")
    parser.add_argument("--ideogram-max", type=int, default=80, help="Max Ideogram v2 images (default: 80)")
    parser.add_argument(
        "--edge-cases-dir", type=str,
        default=str(CORPUS_DIR / "edge_cases"),
        help="Directory containing edge-case authentic images",
    )
    parser.add_argument(
        "--local-ai-dir", type=str,
        default="/Users/paulgriffiths/Desktop/Fake Images AI",
        help="Directory containing local AI-generated images",
    )
    parser.add_argument(
        "--gpt4o-dir", type=str, default="",
        help="Directory containing GPT-4o generated images",
    )
    parser.add_argument(
        "--midjourney-dir", type=str, default="",
        help="Directory containing Midjourney generated images",
    )
    parser.add_argument("--dry-run", action="store_true", help="Show what would be done without downloading")
    parser.add_argument("--deduplicate-only", action="store_true", help="Only run deduplication on existing corpus")
    args = parser.parse_args()

    _ensure_dirs()

    print("=" * 68)
    print("Jura Trace -- Expanded Corpus Builder v2")
    print("=" * 68)
    print(f"  Corpus directory: {CORPUS_DIR}")
    print(f"  Authentic output: {AUTHENTIC_DIR}")
    print(f"  AI output:        {AI_DIR}")
    print()

    # Parse requested sources
    if args.sources == "all":
        active_sources = set(ALL_SOURCES)
    else:
        active_sources = set(s.strip() for s in args.sources.split(","))

    # Deduplication mode
    if args.deduplicate_only:
        print("Running deduplication only...\n")
        for label, directory in [("authentic", AUTHENTIC_DIR), ("ai_generated", AI_DIR)]:
            index = build_hash_index(directory)
            dupes = find_duplicates(index)
            if dupes:
                print(f"\n  Found {len(dupes)} near-duplicate pairs in {label}:")
                for f1, f2, dist in dupes[:20]:
                    print(f"    {f1} <-> {f2} (distance: {dist})")
                if len(dupes) > 20:
                    print(f"    ... and {len(dupes) - 20} more")
            else:
                print(f"\n  No near-duplicates found in {label}")
        return

    if args.dry_run:
        print("DRY RUN -- no images will be downloaded\n")
        print("Planned downloads:")
        if "coco" in active_sources:
            print(f"  COCO val2017:     up to {args.coco_max} authentic")
        if "unsplash" in active_sources:
            print(f"  Unsplash Lite:    up to {args.unsplash_max} authentic")
        if "wikimedia" in active_sources:
            print(f"  Wikimedia:        up to {args.wikimedia_max} authentic")
        if "edge_cases" in active_sources:
            print(f"  Edge cases:       from {args.edge_cases_dir}")
        if "diffusiondb" in active_sources:
            print(f"  DiffusionDB:      up to {args.diffusiondb_max} AI (SD v1)")
        if "sdxl_turbo" in active_sources:
            print(f"  SDXL-Turbo:       up to {args.sdxl_max} AI (local gen)")
        if "flux" in active_sources:
            print(f"  Flux.1-dev:       up to {args.flux_max} AI (HF API)")
        if "ideogram" in active_sources:
            print(f"  Ideogram v2:      up to {args.ideogram_max} AI (API)")
        if "local_ai" in active_sources:
            print(f"  Local AI images:  from {args.local_ai_dir}")
        return

    # Build initial pHash index for deduplication
    print("Building pHash index of existing corpus...")
    existing_hashes = {}
    for directory in [AUTHENTIC_DIR, AI_DIR]:
        if directory.exists():
            idx = build_hash_index(directory)
            existing_hashes.update(idx)
    print(f"  {len(existing_hashes)} existing images indexed\n")

    all_entries: list[dict] = []

    # ---- Authentic sources ----

    print("=" * 68)
    print("AUTHENTIC SOURCES")
    print("=" * 68)

    if "coco" in active_sources:
        entries = download_coco_val2017(AUTHENTIC_DIR, args.coco_max, existing_hashes)
        all_entries.extend(entries)

    if "unsplash" in active_sources:
        entries = download_unsplash_lite(AUTHENTIC_DIR, args.unsplash_max, existing_hashes)
        all_entries.extend(entries)

    if "wikimedia" in active_sources:
        entries = download_wikimedia_featured(AUTHENTIC_DIR, args.wikimedia_max, existing_hashes)
        all_entries.extend(entries)

    if "edge_cases" in active_sources:
        entries = copy_edge_cases(Path(args.edge_cases_dir), AUTHENTIC_DIR, existing_hashes)
        all_entries.extend(entries)

    # ---- AI-generated sources ----

    print("\n" + "=" * 68)
    print("AI-GENERATED SOURCES")
    print("=" * 68)

    if "local_ai" in active_sources:
        entries = copy_local_ai_images(Path(args.local_ai_dir), AI_DIR, "Fake_Images_AI", existing_hashes)
        all_entries.extend(entries)
        # Also copy GPT-4o and Midjourney if specified
        if args.gpt4o_dir:
            entries = copy_local_ai_images(Path(args.gpt4o_dir), AI_DIR, "GPT-4o", existing_hashes)
            all_entries.extend(entries)
        if args.midjourney_dir:
            entries = copy_local_ai_images(Path(args.midjourney_dir), AI_DIR, "Midjourney", existing_hashes)
            all_entries.extend(entries)

    if "diffusiondb" in active_sources:
        entries = download_diffusiondb(AI_DIR, args.diffusiondb_max, existing_hashes)
        all_entries.extend(entries)

    if "sdxl_turbo" in active_sources:
        entries = generate_sdxl_turbo(AI_DIR, args.sdxl_max, existing_hashes)
        all_entries.extend(entries)

    if "flux" in active_sources:
        entries = download_flux_dev(AI_DIR, args.flux_max, existing_hashes)
        all_entries.extend(entries)

    if "ideogram" in active_sources:
        entries = download_ideogram_v2(AI_DIR, args.ideogram_max, existing_hashes)
        all_entries.extend(entries)

    # ---- Manifest and reporting ----

    print("\n" + "=" * 68)
    print("FINALISING")
    print("=" * 68)

    write_manifest(all_entries)
    write_sources_doc(all_entries)

    # Final counts from filesystem
    auth_count = sum(1 for f in AUTHENTIC_DIR.iterdir() if f.suffix.lower() in IMAGE_EXTENSIONS) if AUTHENTIC_DIR.exists() else 0
    ai_count = sum(1 for f in AI_DIR.iterdir() if f.suffix.lower() in IMAGE_EXTENSIONS) if AI_DIR.exists() else 0

    print(f"\n  Corpus on disk:")
    print(f"    Authentic:    {auth_count} images")
    print(f"    AI-generated: {ai_count} images")
    print(f"    Total:        {auth_count + ai_count}")

    # Check deduplication
    print("\n  Running deduplication check...")
    dupes_auth = find_duplicates(build_hash_index(AUTHENTIC_DIR))
    dupes_ai = find_duplicates(build_hash_index(AI_DIR))
    if dupes_auth:
        print(f"    WARNING: {len(dupes_auth)} near-duplicate pairs in authentic/")
    else:
        print(f"    No duplicates in authentic/")
    if dupes_ai:
        print(f"    WARNING: {len(dupes_ai)} near-duplicate pairs in ai_generated/")
    else:
        print(f"    No duplicates in ai_generated/")

    print(f"\nNext steps:")
    print(f"  1. Review corpus/SOURCES.md")
    print(f"  2. Add edge-case images to corpus/edge_cases/ if needed")
    print(f"  3. Analyse FP rate: python scripts/analyse_fp_rate.py")
    print(f"  4. Retrain classifier: python scripts/train_classifier.py \\")
    print(f"       --authentic corpus/authentic --ai corpus/ai_generated")


if __name__ == "__main__":
    main()
