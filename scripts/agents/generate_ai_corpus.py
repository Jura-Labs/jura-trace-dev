#!/usr/bin/env python3
"""
Jura Trace — AI Image Generation Agent

Generates AI images from multiple APIs to build a diverse training corpus.
Supports: Google Gemini, and extensible for OpenAI DALL-E, Ideogram, etc.

Each generator produces images with diverse prompts covering categories
that real-world AI-generated content falls into: people, landscapes,
products, food, architecture, art, animals, news-style, documents.

Usage:
    # Gemini (requires GOOGLE_API_KEY or GEMINI_API_KEY)
    export GEMINI_API_KEY=your_key_here
    python -m scripts.agents.generate_ai_corpus --generator gemini --count 200

    # All available generators
    python -m scripts.agents.generate_ai_corpus --generator all --count 100
"""

import argparse
import base64
import hashlib
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config

# Diverse prompts covering categories that AI-generated content typically falls into
PROMPTS = [
    # People / portraits
    "A professional headshot of a middle-aged woman in a business suit",
    "A street photographer capturing candid moments in a busy market",
    "A group of friends laughing at a cafe table",
    "An elderly man reading a newspaper in a park",
    "A child playing with a golden retriever in a garden",

    # Landscapes / nature
    "A misty mountain landscape at sunrise with pine trees",
    "An aerial view of a winding river through autumn forest",
    "A dramatic storm approaching over an open wheat field",
    "A calm lake reflecting snow-capped mountains",
    "A desert sand dune at golden hour with long shadows",

    # Architecture / urban
    "A modern glass skyscraper reflecting clouds at sunset",
    "A narrow cobblestone alley in an old European city",
    "An abandoned factory with broken windows and ivy",
    "A traditional Japanese temple surrounded by cherry blossoms",
    "A busy intersection in a major city at night with light trails",

    # Food / products
    "A beautifully plated gourmet meal in a fine dining restaurant",
    "A rustic loaf of sourdough bread on a wooden cutting board",
    "A luxury watch on a marble surface with dramatic lighting",
    "A fresh fruit smoothie in a glass jar with berries",
    "A vintage camera on a leather desk with warm lighting",

    # Animals
    "A fox in a snowy forest looking directly at the camera",
    "A hummingbird hovering near a red flower",
    "A lion resting under an acacia tree on the savannah",
    "An underwater photograph of a sea turtle",
    "A barn owl perched on a fence post at dusk",

    # News / documentary style
    "A press conference with microphones on a podium",
    "Emergency workers responding to a flood in a residential area",
    "A protest march with people holding signs in a city square",
    "Scientists working in a laboratory with microscopes",
    "A satellite image of a coastal city",

    # Art / creative
    "An oil painting of a stormy sea in the style of Turner",
    "A minimalist geometric pattern in earth tones",
    "A surreal image of a staircase leading into clouds",
    "A watercolour painting of a village market scene",
    "A photorealistic rendering of a futuristic city",

    # Miscellaneous / edge cases
    "A close-up macro photograph of a butterfly wing",
    "A black and white photograph of a train station",
    "A drone view of a sports stadium during a match",
    "A medical X-ray of a human hand",
    "A satellite weather map showing a hurricane",
]


def generate_gemini(output_dir: Path, count: int) -> list[dict]:
    """Generate images using Google Gemini API."""
    api_key = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
    if not api_key:
        print("  ERROR: Set GEMINI_API_KEY or GOOGLE_API_KEY environment variable")
        print("  Get a key at: https://aistudio.google.com/apikey")
        return []

    try:
        from google import genai
        from google.genai import types
    except ImportError:
        print("  ERROR: google-genai not installed. Run: pip install google-genai")
        return []

    output_dir.mkdir(parents=True, exist_ok=True)
    from PIL import Image
    from io import BytesIO

    client = genai.Client(api_key=api_key)

    entries = []
    downloaded = 0
    prompt_idx = 0

    print(f"  Generating {count} images via Gemini (Imagen 4)...")

    while downloaded < count:
        prompt = PROMPTS[prompt_idx % len(PROMPTS)]
        prompt_idx += 1

        # Add variation suffix for repeated prompts
        if prompt_idx > len(PROMPTS):
            variation = prompt_idx // len(PROMPTS)
            prompt = f"{prompt}, variation {variation}"

        try:
            # Try Imagen 4 first (dedicated image gen), fall back to Gemini Flash
            try:
                response = client.models.generate_images(
                    model="imagen-4.0-generate-001",
                    prompt=prompt,
                    config=types.GenerateImagesConfig(
                        number_of_images=1,
                    ),
                )
                # Imagen returns generated_images list
                if response.generated_images:
                    img_data = response.generated_images[0].image.image_bytes
                    if isinstance(img_data, str):
                        img_data = base64.b64decode(img_data)
                else:
                    raise Exception("No images returned")
            except Exception:
                # Fallback to Gemini Flash multimodal
                response = client.models.generate_content(
                    model="gemini-2.0-flash",
                    contents=f"Generate a photorealistic image: {prompt}",
                    config=types.GenerateContentConfig(
                        response_modalities=["IMAGE", "TEXT"],
                    ),
                )
                img_data = None
                for part in response.candidates[0].content.parts:
                    if part.inline_data and part.inline_data.mime_type.startswith("image/"):
                        img_data = part.inline_data.data
                        if isinstance(img_data, str):
                            img_data = base64.b64decode(img_data)
                        break
                if img_data is None:
                    raise Exception("No image in response")

            # Crop bottom-right 8% to remove Google's visual watermark,
            # making detection more challenging for the classifier.
            # The watermark is typically a small badge in the bottom-right corner.
            try:
                pil_img = Image.open(BytesIO(img_data))
                w, h = pil_img.size
                # Crop 8% from bottom and 8% from right
                crop_w = int(w * 0.92)
                crop_h = int(h * 0.92)
                if crop_w >= 128 and crop_h >= 128:
                    pil_img = pil_img.crop((0, 0, crop_w, crop_h))
                buf = BytesIO()
                pil_img.save(buf, format="PNG")
                img_data = buf.getvalue()
            except Exception:
                pass  # Use uncropped if crop fails

            sha = hashlib.sha256(img_data).hexdigest()
            name = f"gemini_{downloaded:04d}.png"
            (output_dir / name).write_bytes(img_data)

            entries.append({
                "filename": name,
                "source": "gemini",
                "generator": "imagen-4.0 / gemini-2.0-flash",
                "prompt": prompt,
                "sha256": sha,
                "size_bytes": len(img_data),
                "label": "ai_generated",
                "downloaded_at": datetime.now(timezone.utc).isoformat(),
            })

            downloaded += 1
            if downloaded % 10 == 0:
                print(f"  {downloaded}/{count} generated...")

        except Exception as e:
            print(f"  Warning: generation failed for prompt [{prompt[:50]}...]: {e}")
            time.sleep(2)  # Back off on errors

        # Rate limit: 15 requests/minute for free tier
        time.sleep(4.5)

    print(f"  {downloaded} Gemini images generated")
    return entries


def generate_coco_authentic(output_dir: Path, count: int) -> list[dict]:
    """Download authentic images from COCO dataset."""
    output_dir.mkdir(parents=True, exist_ok=True)

    try:
        from datasets import load_dataset
        from io import BytesIO
    except ImportError:
        print("  ERROR: datasets library required")
        return []

    print(f"  Downloading {count} COCO authentic images...")
    ds = load_dataset("detection-datasets/coco", split="val", streaming=True)

    entries = []
    downloaded = 0

    for item in ds:
        if downloaded >= count:
            break
        img = item.get("image")
        if img is None:
            continue
        w, h = img.size
        if w < 128 or h < 128:
            continue

        buf = BytesIO()
        img.save(buf, format="JPEG", quality=95)
        data = buf.getvalue()
        if len(data) < 5000:
            continue

        sha = hashlib.sha256(data).hexdigest()
        name = f"coco_{downloaded:04d}.jpg"
        (output_dir / name).write_bytes(data)

        entries.append({
            "filename": name,
            "source": "coco",
            "sha256": sha,
            "size_bytes": len(data),
            "label": "authentic",
            "downloaded_at": datetime.now(timezone.utc).isoformat(),
        })

        downloaded += 1
        if downloaded % 50 == 0:
            print(f"  {downloaded}/{count} downloaded...")

    print(f"  {downloaded} COCO images downloaded")
    return entries


GENERATORS = {
    "gemini": ("Google Gemini", generate_gemini, "ai_generated"),
    "coco": ("COCO val2017", generate_coco_authentic, "authentic"),
}


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace — AI Image Generation Agent"
    )
    parser.add_argument(
        "--generator",
        default="gemini",
        help=f"Generator: {','.join(GENERATORS.keys())} or 'all'",
    )
    parser.add_argument("--count", type=int, default=200, help="Images per generator")
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Output directory (auto-detected from generator type)",
    )
    args = parser.parse_args()

    generators = list(GENERATORS.keys()) if args.generator == "all" else [args.generator]

    print("=" * 60)
    print("  Jura Trace — AI Image Generation Agent")
    print("=" * 60)

    all_entries = []

    for gen_key in generators:
        if gen_key not in GENERATORS:
            print(f"  Unknown generator: {gen_key}")
            continue

        name, fn, label_type = GENERATORS[gen_key]
        if label_type == "ai_generated":
            out_dir = args.output or config.CORPUS_AI / gen_key
        else:
            out_dir = args.output or config.CORPUS_AUTHENTIC / gen_key

        print(f"\n  --- {name} ({args.count} images) ---")
        entries = fn(out_dir, args.count)
        all_entries.extend(entries)

    # Summary
    ai_total = sum(1 for e in all_entries if e.get("label") == "ai_generated")
    auth_total = sum(1 for e in all_entries if e.get("label") == "authentic")

    print(f"\n{'=' * 60}")
    print(f"  Generated: {ai_total} AI + {auth_total} authentic")
    print(f"  Total corpus now:")

    import subprocess
    ai = subprocess.run(
        ["find", "corpus/training/ai_generated", "-type", "f",
         "(", "-name", "*.png", "-o", "-name", "*.jpg", ")"],
        capture_output=True, text=True
    ).stdout.count("\n")
    auth = subprocess.run(
        ["find", "corpus/training/authentic", "-type", "f",
         "(", "-iname", "*.png", "-o", "-iname", "*.jpg", "-o", "-iname", "*.jpeg", ")"],
        capture_output=True, text=True
    ).stdout.count("\n")
    print(f"    AI: {ai}")
    print(f"    Authentic: {auth}")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
