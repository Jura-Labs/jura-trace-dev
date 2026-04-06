#!/usr/bin/env python3
"""
Jura Trace — Local Stable Diffusion Image Generator

Generates AI images locally on Apple Silicon using SDXL-Turbo and SD 2.1.
Zero API cost — runs entirely on your machine's GPU.

SDXL-Turbo: ~5s/image on M1, 512x512, single-step inference
SD 2.1: ~10s/image on M1, 512x512, 20-step inference

First run downloads the model (~5GB for SDXL-Turbo, ~5GB for SD 2.1).

Usage:
    python scripts/generate_local_sd.py --model sdxl-turbo --count 300
    python scripts/generate_local_sd.py --model sd21 --count 300
    python scripts/generate_local_sd.py --model all --count 150
"""

import argparse
import hashlib
import json
import sys
import time
from datetime import datetime, timezone
from io import BytesIO
from pathlib import Path

# Reuse the prompt list from the Gemini generator
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from scripts.agents.generate_ai_corpus import PROMPTS


MODELS = {
    "sdxl-turbo": {
        "id": "stabilityai/sdxl-turbo",
        "label": "sdxl_turbo",
        "steps": 1,
        "guidance": 0.0,
        "size": 512,
        "description": "SDXL-Turbo (single-step, fastest)",
    },
    "sd21": {
        "id": "stabilityai/stable-diffusion-2-1-base",
        "label": "sd21",
        "steps": 20,
        "guidance": 7.5,
        "size": 512,
        "description": "Stable Diffusion 2.1 Base (20-step)",
    },
    "sd15": {
        "id": "runwayml/stable-diffusion-v1-5",
        "label": "sd15",
        "steps": 20,
        "guidance": 7.5,
        "size": 512,
        "description": "Stable Diffusion 1.5 (20-step)",
    },
    "ssd1b": {
        "id": "segmind/SSD-1B",
        "label": "ssd1b",
        "steps": 25,
        "guidance": 7.0,
        "size": 1024,
        "description": "SSD-1B — distilled SDXL (25-step, 1024px)",
    },
}


def generate(model_key: str, count: int, output_base: Path):
    """Generate images with a specific model."""
    import torch
    from diffusers import AutoPipelineForText2Image

    cfg = MODELS[model_key]
    output_dir = output_base / cfg["label"]
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"\n  Loading {cfg['description']}...")
    print(f"  (First run downloads ~5GB model)")

    # Use MPS (Apple Silicon GPU) if available
    device = "mps" if torch.backends.mps.is_available() else "cpu"
    dtype = torch.float16 if device == "mps" else torch.float32

    pipe = AutoPipelineForText2Image.from_pretrained(
        cfg["id"],
        torch_dtype=dtype,
        variant="fp16" if dtype == torch.float16 else None,
    )
    pipe = pipe.to(device)

    # Disable safety checker for training corpus (we need all outputs)
    if hasattr(pipe, "safety_checker"):
        pipe.safety_checker = None

    print(f"  Model loaded on {device}. Generating {count} images...")

    entries = []
    downloaded = 0
    prompt_idx = 0
    start = time.time()

    while downloaded < count:
        prompt = PROMPTS[prompt_idx % len(PROMPTS)]
        prompt_idx += 1

        if prompt_idx > len(PROMPTS):
            variation = prompt_idx // len(PROMPTS)
            prompt = f"{prompt}, variation {variation}, different angle"

        try:
            result = pipe(
                prompt=prompt,
                num_inference_steps=cfg["steps"],
                guidance_scale=cfg["guidance"],
                width=cfg["size"],
                height=cfg["size"],
            )

            img = result.images[0]
            buf = BytesIO()
            img.save(buf, format="PNG")
            img_data = buf.getvalue()

            sha = hashlib.sha256(img_data).hexdigest()
            name = f"{cfg['label']}_{downloaded:04d}.png"
            (output_dir / name).write_bytes(img_data)

            entries.append({
                "filename": name,
                "source": model_key,
                "generator": cfg["id"],
                "prompt": prompt,
                "sha256": sha,
                "size_bytes": len(img_data),
                "label": "ai_generated",
                "generated_at": datetime.now(timezone.utc).isoformat(),
            })

            downloaded += 1
            if downloaded % 25 == 0:
                elapsed = time.time() - start
                rate = downloaded / elapsed
                eta = (count - downloaded) / rate if rate > 0 else 0
                print(f"  {downloaded}/{count} ({rate:.1f}/s, ETA {eta:.0f}s)")

        except Exception as e:
            print(f"  Warning: generation failed: {str(e)[:80]}")
            continue

    elapsed = time.time() - start
    print(f"  {downloaded} images in {elapsed:.0f}s ({downloaded/elapsed:.1f}/s)")
    return entries


def main():
    parser = argparse.ArgumentParser(description="Jura Trace — Local SD Generator")
    parser.add_argument("--model", default="sdxl-turbo", choices=list(MODELS.keys()) + ["all", "no-auth"])
    parser.add_argument("--count", type=int, default=300)
    parser.add_argument("--output", type=Path, default=Path("corpus/training/ai_generated"))
    args = parser.parse_args()

    if args.model == "all":
        models = list(MODELS.keys())
    elif args.model == "no-auth":
        models = ["sdxl-turbo", "sd15", "ssd1b"]  # skip sd21 (needs HF auth)
    else:
        models = [args.model]

    print("=" * 60)
    print("  Jura Trace — Local SD Image Generator (zero API cost)")
    print("=" * 60)

    all_entries = []
    for model_key in models:
        entries = generate(model_key, args.count, args.output)
        all_entries.extend(entries)

    # Corpus totals
    import subprocess
    ai = subprocess.run(
        ["find", "corpus/training/ai_generated", "-name", "*.png", "-o", "-name", "*.jpg"],
        capture_output=True, text=True,
    ).stdout.count("\n")
    print(f"\n{'=' * 60}")
    print(f"  Generated: {len(all_entries)} images")
    print(f"  Total AI corpus: {ai}")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
