#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
check_clip_pytorch_vs_onnx.py — verify numerical equivalence between the
PyTorch+open_clip CLIP path (used during UnivFD probe training) and the
ONNX runtime CLIP path (used at inference time in the shipped sidecar).

Why this exists
---------------
The UnivFD LogReg probe is trained on CLIP ViT-B/32 embeddings extracted by
PyTorch+open_clip from the laion2b_s34b_b79k weights.  At inference time the
shipped sidecar extracts embeddings via ONNX runtime from an exported version
of the same weights.  If the two pipelines diverge numerically, the probe is
calibrated on one distribution and applied to another → wrong scores.

This script extracts CLIP embeddings via both paths on a fixed test image
set and reports per-image cosine similarity + L2 distance.  Pass criterion:
mean cosine similarity ≥ 0.999 (effectively identical up to float rounding).

Usage:
    python3 scripts/check_clip_pytorch_vs_onnx.py [--n-images N] [--samples-from PATH]
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
import random

import numpy as np
from PIL import Image

# Try optional pillow_heif registration (in case test set includes HEIC)
try:
    import pillow_heif
    pillow_heif.register_heif_opener()
except ImportError:
    pass


def extract_pytorch(image_paths: list[Path]) -> np.ndarray:
    """Extract CLIP ViT-B/32 embeddings via PyTorch + open_clip."""
    import torch
    import open_clip

    device = torch.device("mps" if torch.backends.mps.is_available()
                          else "cuda" if torch.cuda.is_available()
                          else "cpu")
    print(f"  PyTorch device: {device}")

    model, _, preprocess = open_clip.create_model_and_transforms(
        "ViT-B-32", pretrained="laion2b_s34b_b79k"
    )
    model.eval()
    model.to(device)

    embeddings = []
    for i, p in enumerate(image_paths, 1):
        img = Image.open(p).convert("RGB")
        tensor = preprocess(img).unsqueeze(0).to(device)
        with torch.no_grad():
            feat = model.encode_image(tensor)
            feat = feat / feat.norm(dim=-1, keepdim=True)
        embeddings.append(feat.squeeze().cpu().numpy().astype(np.float32))
        if i % 10 == 0:
            print(f"    PyTorch progress: {i}/{len(image_paths)}")

    return np.stack(embeddings)


def extract_onnx(image_paths: list[Path], onnx_model_path: Path) -> np.ndarray:
    """Extract CLIP ViT-B/32 embeddings via ONNX runtime — mirrors the
    sidecar's clip_detector.py preprocessing + inference path."""
    import onnxruntime as ort

    # Mirror the sidecar's preprocess pipeline from clip_detector.py.
    # open_clip's CLIP ViT-B/32 transform is:
    #   1. Resize shortest side to 224, interp=BICUBIC
    #   2. CenterCrop to 224x224
    #   3. ToTensor (HWC uint8 → CHW float32 / 255.0)
    #   4. Normalise with mean=(0.48145466,0.4578275,0.40821073),
    #      std=(0.26862954,0.26130258,0.27577711)
    CLIP_MEAN = np.array([0.48145466, 0.4578275, 0.40821073], dtype=np.float32).reshape(3, 1, 1)
    CLIP_STD  = np.array([0.26862954, 0.26130258, 0.27577711], dtype=np.float32).reshape(3, 1, 1)

    session = ort.InferenceSession(str(onnx_model_path), providers=["CPUExecutionProvider"])
    input_name = session.get_inputs()[0].name
    print(f"  ONNX input: {input_name}, providers: {session.get_providers()}")

    def preprocess_one(img: Image.Image) -> np.ndarray:
        # Resize shortest side to 224, BICUBIC
        w, h = img.size
        if w < h:
            new_w, new_h = 224, int(round(h * 224 / w))
        else:
            new_w, new_h = int(round(w * 224 / h)), 224
        img = img.resize((new_w, new_h), Image.BICUBIC)
        # CenterCrop to 224x224
        l = (new_w - 224) // 2
        t = (new_h - 224) // 2
        img = img.crop((l, t, l + 224, t + 224))
        # ToTensor + normalise
        arr = np.array(img, dtype=np.float32) / 255.0  # HWC
        arr = arr.transpose(2, 0, 1)  # CHW
        arr = (arr - CLIP_MEAN) / CLIP_STD
        return arr.astype(np.float32)

    embeddings = []
    for i, p in enumerate(image_paths, 1):
        img = Image.open(p).convert("RGB")
        tensor = preprocess_one(img)[None, ...]  # batch dim
        feat = session.run(None, {input_name: tensor})[0]
        feat = feat / np.linalg.norm(feat, axis=-1, keepdims=True)
        embeddings.append(feat.squeeze().astype(np.float32))
        if i % 10 == 0:
            print(f"    ONNX progress: {i}/{len(image_paths)}")

    return np.stack(embeddings)


def cosine_similarity(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """Per-row cosine similarity between matched embeddings.
    Assumes both already L2-normalised."""
    return np.sum(a * b, axis=-1)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--n-images", type=int, default=20,
                    help="Number of test images (default 20)")
    ap.add_argument("--samples-from", type=Path,
                    default=Path("/Volumes/MAC SSD/Training Data/corpus/training/ai_generated/civitai_sfw"),
                    help="Directory to sample test images from")
    ap.add_argument("--onnx-model", type=Path,
                    default=Path(__file__).parent.parent / "models" / "clip-vit-b32-vision.onnx",
                    help="Path to CLIP ONNX vision model")
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    # Collect test images
    if not args.samples_from.is_dir():
        print(f"ERROR: samples directory not found: {args.samples_from}")
        return 2

    if not args.onnx_model.exists():
        print(f"ERROR: ONNX model not found: {args.onnx_model}")
        return 2

    EXTS = {".jpg", ".jpeg", ".png", ".tif", ".tiff", ".webp", ".heic"}
    all_imgs = sorted(p for p in args.samples_from.rglob("*")
                      if p.is_file() and p.suffix.lower() in EXTS)
    rng = random.Random(args.seed)
    rng.shuffle(all_imgs)
    images = all_imgs[:args.n_images]

    print(f"\nCLIP PyTorch vs ONNX Numerical Equivalence Check")
    print("=" * 60)
    print(f"  Test images:    {len(images)}")
    print(f"  Source:         {args.samples_from}")
    print(f"  ONNX model:     {args.onnx_model}")

    # Extract via PyTorch
    print("\n[1/2] Extracting via PyTorch + open_clip...")
    emb_pt = extract_pytorch(images)
    print(f"  PyTorch embeddings shape: {emb_pt.shape}")

    # Extract via ONNX
    print("\n[2/2] Extracting via ONNX runtime...")
    emb_onnx = extract_onnx(images, args.onnx_model)
    print(f"  ONNX embeddings shape:    {emb_onnx.shape}")

    # Compare
    print("\nComparing per-image embeddings:")
    print("-" * 60)
    cos_sim = cosine_similarity(emb_pt, emb_onnx)
    l2_dist = np.linalg.norm(emb_pt - emb_onnx, axis=-1)
    mean_cos = float(cos_sim.mean())
    min_cos = float(cos_sim.min())
    mean_l2 = float(l2_dist.mean())
    max_l2 = float(l2_dist.max())

    print(f"  Mean cosine similarity:  {mean_cos:.6f}")
    print(f"  Min cosine similarity:   {min_cos:.6f}")
    print(f"  Mean L2 distance:        {mean_l2:.6f}")
    print(f"  Max L2 distance:         {max_l2:.6f}")

    # Per-image breakdown for worst N
    print("\nPer-image (5 worst):")
    worst = np.argsort(cos_sim)[:5]
    for idx in worst:
        print(f"  cos={cos_sim[idx]:.6f}  L2={l2_dist[idx]:.6f}  {images[idx].name}")

    # Pass / fail
    print("\n" + "=" * 60)
    pass_threshold = 0.999
    if mean_cos >= pass_threshold and min_cos >= pass_threshold * 0.99:
        print(f"  PASS — embeddings are numerically equivalent")
        print(f"  (mean cos {mean_cos:.6f} ≥ {pass_threshold}, min cos {min_cos:.6f})")
        print(f"  The UnivFD probe trained on PyTorch embeddings will")
        print(f"  produce calibrated scores when applied to ONNX embeddings.")
        return 0
    else:
        print(f"  FAIL — embeddings diverge beyond tolerance")
        print(f"  (mean cos {mean_cos:.6f}, threshold {pass_threshold})")
        print(f"  The shipped sidecar's UnivFD probe scores may be miscalibrated.")
        print(f"  Investigate: ONNX export procedure, preprocessing pipeline,")
        print(f"  or compute the probe on ONNX embeddings during training instead.")
        return 1


if __name__ == "__main__":
    sys.exit(main())
