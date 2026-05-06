# SPDX-License-Identifier: AGPL-3.0-or-later

"""
JTV-143 validation gate: torch vs ONNX cosine drift on real corpus images.

Decision rule (from sprint plan, hard gate Friday 15 May 2026):
    - mean cosine drift  < 0.002   → ship Option B-full as planned
    - mean cosine drift  < 0.005   → ship but flag for v1.0.1 probe retrain
    - mean cosine drift  >= 0.005  → ABORT B-full; either retrain probe on
                                     ONNX embeddings (+2-3 days) or fall back
                                     to B-fast (defer text encoder to v1.0.1)

The probe weights in models/univfd_probe.joblib were trained on torch FP32
embeddings from this exact checkpoint. If ONNX embeddings drift the input
distribution, the trained LogReg decision boundary no longer applies and
recall on flux_dev / sdxl_turbo (the v9 weak families at 88.9% / 91.1%)
degrades fastest.

Usage:
    python scripts/validate_clip_onnx_drift.py \
        --vision-onnx models/clip-vit-b32-vision.onnx \
        --text-onnx   models/clip-vit-b32-text.onnx \
        --image-dir   "/Volumes/Samsung USB/Training Data/corpus/authentic" \
        --sample      100
"""

from __future__ import annotations

import argparse
import json
import random
import sys
import time
from pathlib import Path

import numpy as np
import torch
import open_clip
import onnxruntime as ort
from PIL import Image


def _load_torch():
    model, _, preprocess = open_clip.create_model_and_transforms(
        "ViT-B-32", pretrained="laion2b_s34b_b79k"
    )
    model.eval()
    tokenizer = open_clip.get_tokenizer("ViT-B-32")
    return model, preprocess, tokenizer


def _gather_images(image_dir: Path, sample: int) -> list[Path]:
    candidates = [
        p for p in image_dir.rglob("*")
        if p.is_file() and p.suffix.lower() in {".jpg", ".jpeg", ".png", ".webp", ".tiff"}
    ]
    if not candidates:
        raise SystemExit(f"No images found under {image_dir}")
    random.seed(0xC11B)  # deterministic for reproducibility
    if len(candidates) > sample:
        candidates = random.sample(candidates, sample)
    print(f"  Validating against {len(candidates)} images from {image_dir}")
    return candidates


def _cosine(a: np.ndarray, b: np.ndarray) -> float:
    a = a / (np.linalg.norm(a) + 1e-12)
    b = b / (np.linalg.norm(b) + 1e-12)
    return float(np.dot(a, b))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--vision-onnx", type=Path, required=True)
    ap.add_argument("--text-onnx", type=Path, required=True)
    ap.add_argument("--image-dir", type=Path, required=True)
    ap.add_argument("--sample", type=int, default=100)
    ap.add_argument("--output-json", type=Path, default=None)
    args = ap.parse_args()

    print("Loading torch reference (open_clip ViT-B-32 laion2b_s34b_b79k)...")
    torch_model, preprocess, tokenizer = _load_torch()

    print(f"Loading ONNX vision session ({args.vision_onnx.name})...")
    vision_sess = ort.InferenceSession(
        str(args.vision_onnx), providers=["CPUExecutionProvider"]
    )

    print(f"Loading ONNX text session ({args.text_onnx.name})...")
    text_sess = ort.InferenceSession(
        str(args.text_onnx), providers=["CPUExecutionProvider"]
    )

    # ── Vision drift ───────────────────────────────────────────────────
    print("\n=== Vision encoder drift ===")
    images = _gather_images(args.image_dir, args.sample)

    cosines: list[float] = []
    l2_diffs: list[float] = []
    t_torch_total = 0.0
    t_onnx_total = 0.0
    failed = 0

    for i, img_path in enumerate(images):
        try:
            img = Image.open(img_path).convert("RGB")
        except Exception as e:
            failed += 1
            continue

        tensor = preprocess(img).unsqueeze(0)  # (1, 3, 224, 224) torch float32

        # torch reference
        t0 = time.perf_counter()
        with torch.no_grad():
            torch_emb = torch_model.encode_image(tensor).cpu().numpy()[0]
        t_torch_total += time.perf_counter() - t0

        # ONNX
        np_input = tensor.cpu().numpy()
        t0 = time.perf_counter()
        onnx_emb = vision_sess.run(None, {vision_sess.get_inputs()[0].name: np_input})[0][0]
        t_onnx_total += time.perf_counter() - t0

        cosines.append(_cosine(torch_emb, onnx_emb))
        l2_diffs.append(float(np.linalg.norm(torch_emb - onnx_emb)))

        if (i + 1) % 25 == 0:
            print(f"  {i+1}/{len(images)} processed; running mean cosine={np.mean(cosines):.6f}")

    cosines_arr = np.array(cosines)
    l2_arr = np.array(l2_diffs)

    mean_cos = float(cosines_arr.mean())
    min_cos = float(cosines_arr.min())
    p99_drift = float((1.0 - cosines_arr).max())

    print(f"\nVision encoder cosine similarity (n={len(cosines)}):")
    print(f"  mean   {mean_cos:.6f}")
    print(f"  min    {min_cos:.6f}")
    print(f"  p99 drift (1 - cosine): {p99_drift:.6f}")
    print(f"  L2 diff mean: {float(l2_arr.mean()):.6f}, max: {float(l2_arr.max()):.6f}")
    print(f"  per-image torch latency: {1000*t_torch_total/len(cosines):.1f} ms")
    print(f"  per-image onnx  latency: {1000*t_onnx_total /len(cosines):.1f} ms")

    # ── Text drift ─────────────────────────────────────────────────────
    print("\n=== Text encoder drift ===")
    prompts = [
        "a photograph of a real scene",
        "a real photo taken by a camera",
        "an AI-generated image",
        "a synthetic image created by a neural network",
        "a manipulated photograph",
    ]
    tokens = tokenizer(prompts)

    with torch.no_grad():
        torch_text = torch_model.encode_text(tokens).cpu().numpy()

    # The exported text encoder graph has a reshape op that hard-codes the
    # dummy batch=1 dim; running batch=5 fails. Loop one prompt at a time —
    # zero-shot at runtime only encodes 5 prompts so the latency hit is
    # negligible (~5 ms total).
    onnx_text = np.stack([
        text_sess.run(
            None,
            {text_sess.get_inputs()[0].name: tokens[i:i+1].cpu().numpy()},
        )[0][0]
        for i in range(len(prompts))
    ])

    text_cosines = [_cosine(torch_text[i], onnx_text[i]) for i in range(len(prompts))]
    text_mean = float(np.mean(text_cosines))
    text_min = float(np.min(text_cosines))
    print(f"  per-prompt cosines: {[f'{c:.6f}' for c in text_cosines]}")
    print(f"  mean: {text_mean:.6f}, min: {text_min:.6f}")

    # ── Decision gate ──────────────────────────────────────────────────
    drift = 1.0 - mean_cos
    print(f"\n=== JTV-143 GATE ===")
    print(f"Vision mean drift: {drift:.6f}")
    if drift < 0.002:
        verdict = "PASS — ship Option B-full as planned"
        rc = 0
    elif drift < 0.005:
        verdict = "MARGINAL — ship but flag for v1.0.1 probe retrain"
        rc = 0
    else:
        verdict = "FAIL — abort B-full; retrain probe on ONNX embeddings (+2-3 days) or fall back to B-fast"
        rc = 2
    print(f"Verdict: {verdict}")

    if args.output_json:
        args.output_json.write_text(json.dumps({
            "vision": {
                "n": len(cosines),
                "mean_cosine": mean_cos,
                "min_cosine": min_cos,
                "p99_drift": p99_drift,
                "l2_mean": float(l2_arr.mean()),
                "l2_max": float(l2_arr.max()),
                "torch_ms": 1000*t_torch_total/len(cosines),
                "onnx_ms": 1000*t_onnx_total/len(cosines),
            },
            "text": {
                "n": len(prompts),
                "mean_cosine": text_mean,
                "min_cosine": text_min,
                "per_prompt_cosines": text_cosines,
            },
            "drift": drift,
            "verdict": verdict,
            "failed_loads": failed,
        }, indent=2))

    return rc


if __name__ == "__main__":
    sys.exit(main())
