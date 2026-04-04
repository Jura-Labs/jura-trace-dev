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
    # People / portraits (describe features, not identities)
    "Portrait of a South Asian woman, mid-30s, medium build, wearing a blue blazer, studio lighting",
    "Candid street photo of a tall European man, early 60s, grey hair, reading on a bench",
    "Group photo of three East Asian adults, 20s, athletic build, at an outdoor cafe",
    "Headshot of a Middle Eastern man, late 40s, short beard, wearing glasses",
    "Portrait of an African woman, early 50s, slim build, wearing a patterned headwrap",
    "Photo of a Caucasian teenager, red hair, freckles, sitting in a library",
    "Candid photo of a Latin American woman, 70s, petite, smiling in a garden",

    # Landscapes / nature
    "A misty mountain landscape at sunrise with pine trees",
    "An aerial view of a winding river through autumn forest",
    "A dramatic storm approaching over an open wheat field",
    "A calm lake reflecting snow-capped mountains",
    "A desert sand dune at golden hour with long shadows",
    "A tropical beach with turquoise water and palm trees",
    "A rolling green hillside with sheep and stone walls",
    "A dense bamboo forest with dappled sunlight",

    # Architecture / urban
    "A modern glass office building reflecting clouds at sunset",
    "A narrow cobblestone street in a Mediterranean village",
    "An old brick warehouse converted into apartments",
    "A wooden pagoda surrounded by autumn trees",
    "A city skyline at night with illuminated bridges",
    "A Victorian terrace house with a red front door",
    "A brutalist concrete car park in overcast light",

    # Food / products
    "A rustic loaf of sourdough bread on a wooden cutting board",
    "A luxury wristwatch on a marble surface with dramatic lighting",
    "A fresh fruit smoothie in a glass jar with berries",
    "A vintage film camera on a leather desk",
    "A bowl of ramen with chopsticks and steam rising",
    "A ceramic vase of wildflowers on a windowsill",
    "A stack of old leather-bound books on an oak shelf",

    # Animals
    "A fox in a snowy forest looking directly at the camera",
    "A hummingbird hovering near a red flower in sunlight",
    "A lion resting under an acacia tree on the African savannah",
    "An underwater photograph of a sea turtle swimming over coral",
    "A barn owl perched on a wooden fence post at dusk",
    "A tabby cat curled up on a woollen blanket",
    "A golden retriever running through a field of tall grass",

    # Documentary / workplace
    "A row of microphones on an empty podium in a conference room",
    "A laboratory bench with microscopes and glass beakers",
    "A satellite image of a coastline with river delta",
    "A weather radar screen showing a large storm system",
    "An empty courtroom with wooden benches and a judge's chair",

    # Art / creative
    "An oil painting of a stormy sea with crashing waves",
    "A minimalist geometric pattern in earth tones on canvas",
    "A surreal image of a staircase leading into clouds",
    "A watercolour painting of a village market with fruit stalls",
    "A photorealistic digital rendering of a futuristic city",

    # Conflict / humanitarian (documentary style, no graphic content)
    "A damaged residential building with broken windows and rubble on the street",
    "A refugee camp with rows of white tents in a dry landscape",
    "An abandoned checkpoint with concrete barriers on a dusty road",
    "A burnt-out vehicle on the side of a rural road",
    "A makeshift shelter built from tarpaulin and corrugated metal",
    "A bombed bridge with twisted metal over a river",
    "An empty school classroom with overturned desks and debris",

    # Political / protest (environments, not specific events)
    "A large crowd gathered in a public square with banners and flags",
    "A wall covered in political graffiti and posters",
    "A line of riot shields on an empty street",
    "A podium with multiple national flags at a summit venue",
    "A ballot box on a table in a school gymnasium",

    # Climate change / environmental
    "A glacier calving into the ocean with chunks of ice falling",
    "A dried-up riverbed with cracked mud and dead fish",
    "A wildfire burning through a pine forest at night with orange sky",
    "A flooded residential street with water up to the windows of houses",
    "A coral reef showing severe bleaching with white and grey coral",
    "An aerial photo of deforestation showing bare red earth next to dense forest",
    "A solar panel farm stretching across a desert landscape",
    "A smokestack emitting thick grey emissions against a cloudy sky",
    "A stranded polar bear on a small piece of sea ice",
    "A coastal town with sandbag flood defences along the shore",

    # Jura Labs branding / marketing (save separately for brand use)
    "A geological cross-section showing layered rock strata in warm earth tones",
    "A close-up of polished obsidian stone with reflective surface on dark background",
    "Layered sandstone cliff face showing millions of years of geological history",
    "A single eye reflected in a magnifying glass examining a photograph",
    "A glowing lapis lazuli gemstone on a dark slate surface with warm lighting",
    "A digital forensic analyst examining an image on a large monitor in a dimly lit room",
    "An abstract representation of data layers like geological strata with blue and amber tones",
    "A museum archivist carefully handling a historical photograph with white gloves",
    "A compass and magnifying glass on a vintage map suggesting investigation and truth",
    "Crystalline mineral formations in deep blue and green tones suggesting trust and permanence",

    # Miscellaneous / edge cases
    "A close-up macro photograph of a butterfly wing showing scales",
    "A black and white photograph of a steam train at a station",
    "A drone aerial view of a circular sports stadium",
    "A flat lay of office supplies on a white desk",
    "A time-lapse photograph of star trails over a mountain",
    "An infrared photograph of a forest in false colour",
    "A tilt-shift photograph making a city look like a miniature model",
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
