#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
find_clip_preprocess_fix.py — A/B test PIL preprocess variants to find one
that matches torchvision's antialiased BICUBIC (the reference used at probe
training time) to within cos ≥ 0.999.

Tested variants (all use the same normalisation; only resize differs):

  V0  PIL BICUBIC                       (current sidecar — baseline, cos ~0.996)
  V1  PIL LANCZOS                       (sinc kernel, has implicit antialias)
  V2  PIL HAMMING                       (small antialias window)
  V3  PIL BILINEAR                      (PIL ≥10 antialiases bilinear by default)
  V4  PIL BICUBIC + post-Gaussian blur  (manual prefilter approximation)
  V5  PIL BICUBIC two-pass (reduce + BICUBIC)
                                        (avoids large single-step BICUBIC kernel)

Pass criterion: mean cos ≥ 0.999 AND min cos ≥ 0.989 vs PyTorch reference.
"""

from __future__ import annotations

import argparse
import random
import sys
from pathlib import Path
from typing import Callable

import numpy as np
from PIL import Image, ImageFilter

CLIP_MEAN = np.array([0.48145466, 0.4578275, 0.40821073], dtype=np.float32).reshape(3, 1, 1)
CLIP_STD = np.array([0.26862954, 0.26130258, 0.27577711], dtype=np.float32).reshape(3, 1, 1)


def normalise_and_pack(img_224: Image.Image) -> np.ndarray:
    arr = np.asarray(img_224, dtype=np.float32) / 255.0
    arr = arr.transpose(2, 0, 1)
    arr = (arr - CLIP_MEAN) / CLIP_STD
    return arr[np.newaxis, ...]


def make_preprocess(resize_fn: Callable[[Image.Image, int, int], Image.Image]):
    def fn(img: Image.Image) -> np.ndarray:
        if img.mode != "RGB":
            img = img.convert("RGB")
        w, h = img.size
        if w < h:
            new_w, new_h = 224, int(round(h * 224 / w))
        else:
            new_w, new_h = int(round(w * 224 / h)), 224
        img = resize_fn(img, new_w, new_h)
        left = (new_w - 224) // 2
        top = (new_h - 224) // 2
        img = img.crop((left, top, left + 224, top + 224))
        return normalise_and_pack(img)
    return fn


def resize_bicubic(img, w, h):
    return img.resize((w, h), Image.BICUBIC)


def resize_lanczos(img, w, h):
    return img.resize((w, h), Image.LANCZOS)


def resize_hamming(img, w, h):
    return img.resize((w, h), Image.HAMMING)


def resize_bilinear(img, w, h):
    return img.resize((w, h), Image.BILINEAR)


def resize_bicubic_blurred(img, w, h):
    """Apply Gaussian blur before BICUBIC resize, sigma scaled by downsample ratio."""
    src_w, src_h = img.size
    ratio = max(src_w / w, src_h / h)
    if ratio > 1.0:
        sigma = (ratio - 1) / 2.0
        img = img.filter(ImageFilter.GaussianBlur(radius=sigma))
    return img.resize((w, h), Image.BICUBIC)


def resize_bicubic_two_pass(img, w, h):
    """Two-pass: thumbnail down to 2× target with bicubic, then exact bicubic."""
    src_w, src_h = img.size
    if src_w > w * 2 and src_h > h * 2:
        img = img.resize((w * 2, h * 2), Image.BICUBIC)
    return img.resize((w, h), Image.BICUBIC)


VARIANTS = {
    "V0_bicubic":        resize_bicubic,
    "V1_lanczos":        resize_lanczos,
    "V2_hamming":        resize_hamming,
    "V3_bilinear":       resize_bilinear,
    "V4_bicubic_blur":   resize_bicubic_blurred,
    "V5_bicubic_2pass":  resize_bicubic_two_pass,
}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--n-images", type=int, default=20)
    ap.add_argument("--samples-from", type=Path,
                    default=Path("/Volumes/MAC SSD/Training Data/corpus/training/ai_generated/civitai_sfw"))
    ap.add_argument("--onnx-model", type=Path,
                    default=Path(__file__).parent.parent / "models" / "clip-vit-b32-vision.onnx")
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    import torch
    import open_clip
    import onnxruntime as ort

    EXTS = {".jpg", ".jpeg", ".png", ".tif", ".tiff", ".webp", ".heic"}
    all_imgs = sorted(p for p in args.samples_from.rglob("*")
                      if p.is_file() and p.suffix.lower() in EXTS)
    rng = random.Random(args.seed)
    rng.shuffle(all_imgs)
    images = all_imgs[:args.n_images]

    print(f"\nPreprocess A/B test — {len(images)} images")
    print("=" * 70)

    device = torch.device("mps" if torch.backends.mps.is_available()
                          else "cuda" if torch.cuda.is_available()
                          else "cpu")
    pt_model, _, pt_preprocess = open_clip.create_model_and_transforms(
        "ViT-B-32", pretrained="laion2b_s34b_b79k"
    )
    pt_model.eval().to(device)

    session = ort.InferenceSession(str(args.onnx_model), providers=["CPUExecutionProvider"])
    onnx_input = session.get_inputs()[0].name

    # Reference: PyTorch preprocess → PyTorch model
    print("Computing PyTorch reference embeddings...")
    ref = []
    for p in images:
        img = Image.open(p).convert("RGB")
        t = pt_preprocess(img).unsqueeze(0)
        with torch.no_grad():
            feat = pt_model.encode_image(t.to(device))
            feat = feat / feat.norm(dim=-1, keepdim=True)
        ref.append(feat.squeeze().cpu().numpy().astype(np.float32))
    ref = np.stack(ref)

    # Test each variant: PIL variant preprocess → ONNX model → compare to ref
    print(f"\n{'Variant':<22} {'Mean cos':>10} {'Min cos':>10} {'Max L2':>10}   Verdict")
    print("-" * 70)
    results = []
    for vname, resize_fn in VARIANTS.items():
        pre = make_preprocess(resize_fn)
        embs = []
        for p in images:
            img = Image.open(p).convert("RGB")
            t = pre(img)
            out = session.run(None, {onnx_input: t})[0]
            feat = out / np.linalg.norm(out, axis=-1, keepdims=True)
            embs.append(feat.squeeze().astype(np.float32))
        embs = np.stack(embs)

        cos = np.sum(ref * embs, axis=-1)
        l2 = np.linalg.norm(ref - embs, axis=-1)
        mean_cos = float(cos.mean())
        min_cos = float(cos.min())
        max_l2 = float(l2.max())
        verdict = "PASS" if (mean_cos >= 0.999 and min_cos >= 0.989) else \
                  "BORDERLINE" if mean_cos >= 0.998 else "FAIL"
        marker = " ←" if verdict == "PASS" else ""
        print(f"  {vname:<20} {mean_cos:>10.6f} {min_cos:>10.6f} {max_l2:>10.6f}   {verdict}{marker}")
        results.append((vname, mean_cos, min_cos, verdict))

    print("=" * 70)
    passing = [r for r in results if r[3] == "PASS"]
    if passing:
        best = max(passing, key=lambda r: r[1])
        print(f"  BEST: {best[0]} (mean cos {best[1]:.6f})")
        print(f"  Patch clip_detector.py preprocess to use this variant.")
    else:
        borderline = [r for r in results if r[3] == "BORDERLINE"]
        if borderline:
            best = max(borderline, key=lambda r: r[1])
            print(f"  No variant fully passes. Best borderline: {best[0]} (cos {best[1]:.6f})")
            print(f"  Options: (a) accept borderline, (b) retrain probe on sidecar-pre embeddings.")
        else:
            print(f"  NO PIL variant gets close. Will need to retrain probe.")

    return 0


if __name__ == "__main__":
    sys.exit(main())
