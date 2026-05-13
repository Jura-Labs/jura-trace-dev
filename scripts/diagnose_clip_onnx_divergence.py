#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
diagnose_clip_onnx_divergence.py — isolate whether the 0.996 cosine-similarity
gap between PyTorch+open_clip and the shipped ONNX runtime is caused by:

  (A) PREPROCESS divergence — open_clip uses torchvision's antialiased BICUBIC
      `Resize`, the sidecar uses PIL.Image.resize. Same nominal algorithm,
      different downsampling kernels.

  (B) MODEL GRAPH divergence — the exported ONNX captures different math than
      the live PyTorch encoder (precision loss, op-fusion change, weight drift).

Method
------
On the same 20-image set, run three pipelines:

  P1  PyTorch preprocess  →  PyTorch model  (reference; this is what the
                                              UnivFD probe was trained on)
  P2  PyTorch preprocess  →  ONNX model     (isolates model-graph delta)
  P3  Sidecar preprocess  →  ONNX model     (the live production path)

Interpretation:
  cos(P1, P2) ≥ 0.999  → model graph is fine; gap is preprocessing (Suspect A)
  cos(P1, P2) <  0.999 → model graph diverges (Suspect B)
  cos(P2, P3) ≥ 0.999  → preprocesses match at the tensor level
  cos(P2, P3) <  0.999 → preprocesses produce different tensors → fix preprocess
"""

from __future__ import annotations

import argparse
import random
import sys
from pathlib import Path

import numpy as np
from PIL import Image

CLIP_MEAN = np.array([0.48145466, 0.4578275, 0.40821073], dtype=np.float32).reshape(3, 1, 1)
CLIP_STD = np.array([0.26862954, 0.26130258, 0.27577711], dtype=np.float32).reshape(3, 1, 1)


def sidecar_preprocess(img: Image.Image) -> np.ndarray:
    """Bit-identical copy of sidecar/app/services/clip_detector.py:_preprocess_image."""
    if img.mode != "RGB":
        img = img.convert("RGB")
    w, h = img.size
    if w < h:
        new_w, new_h = 224, int(round(h * 224 / w))
    else:
        new_w, new_h = int(round(w * 224 / h)), 224
    img = img.resize((new_w, new_h), Image.BICUBIC)
    left = (new_w - 224) // 2
    top = (new_h - 224) // 2
    img = img.crop((left, top, left + 224, top + 224))
    arr = np.asarray(img, dtype=np.float32) / 255.0
    arr = arr.transpose(2, 0, 1)
    arr = (arr - CLIP_MEAN) / CLIP_STD
    return arr[np.newaxis, ...]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
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

    print(f"\nCLIP PyTorch vs ONNX — Divergence isolation")
    print("=" * 70)
    print(f"  Test images:    {len(images)}")
    print(f"  ONNX:           {args.onnx_model}")

    device = torch.device("mps" if torch.backends.mps.is_available()
                          else "cuda" if torch.cuda.is_available()
                          else "cpu")
    print(f"  PyTorch device: {device}")

    print("\n[setup] Loading open_clip ViT-B-32 (laion2b_s34b_b79k)...")
    pt_model, _, pt_preprocess = open_clip.create_model_and_transforms(
        "ViT-B-32", pretrained="laion2b_s34b_b79k"
    )
    pt_model.eval().to(device)

    # Print the open_clip preprocess pipeline so we can see what torchvision is doing
    print(f"\n[setup] open_clip preprocess:\n  {pt_preprocess}")

    print("\n[setup] Loading ONNX session...")
    session = ort.InferenceSession(str(args.onnx_model), providers=["CPUExecutionProvider"])
    onnx_input_name = session.get_inputs()[0].name

    # ── P1: PyTorch preprocess + PyTorch model (reference) ──────────────
    print("\n[P1] PyTorch preprocess → PyTorch model (reference)...")
    p1, pt_tensors = [], []
    for i, p in enumerate(images, 1):
        img = Image.open(p).convert("RGB")
        t = pt_preprocess(img).unsqueeze(0)
        pt_tensors.append(t.numpy().astype(np.float32))  # save for P2
        with torch.no_grad():
            feat = pt_model.encode_image(t.to(device))
            feat = feat / feat.norm(dim=-1, keepdim=True)
        p1.append(feat.squeeze().cpu().numpy().astype(np.float32))
        if i % 10 == 0:
            print(f"    P1 progress: {i}/{len(images)}")
    p1 = np.stack(p1)

    # ── P2: PyTorch preprocess + ONNX model (isolates model-graph delta) ─
    print("\n[P2] PyTorch preprocess → ONNX model...")
    p2 = []
    for i, t in enumerate(pt_tensors, 1):
        out = session.run(None, {onnx_input_name: t})[0]
        feat = out / np.linalg.norm(out, axis=-1, keepdims=True)
        p2.append(feat.squeeze().astype(np.float32))
        if i % 10 == 0:
            print(f"    P2 progress: {i}/{len(images)}")
    p2 = np.stack(p2)

    # ── P3: Sidecar preprocess + ONNX model (the live prod path) ─────────
    print("\n[P3] Sidecar preprocess → ONNX model (production path)...")
    p3 = []
    sidecar_tensors = []
    for i, p in enumerate(images, 1):
        img = Image.open(p).convert("RGB")
        t = sidecar_preprocess(img)
        sidecar_tensors.append(t)
        out = session.run(None, {onnx_input_name: t})[0]
        feat = out / np.linalg.norm(out, axis=-1, keepdims=True)
        p3.append(feat.squeeze().astype(np.float32))
        if i % 10 == 0:
            print(f"    P3 progress: {i}/{len(images)}")
    p3 = np.stack(p3)

    # ── Compare ──────────────────────────────────────────────────────────
    def stats(a, b, label):
        cos = np.sum(a * b, axis=-1)
        l2 = np.linalg.norm(a - b, axis=-1)
        return f"  {label}: mean cos {cos.mean():.6f}  min cos {cos.min():.6f}  max L2 {l2.max():.6f}"

    print("\n" + "=" * 70)
    print("EMBEDDING-LEVEL COMPARISONS")
    print("-" * 70)
    print(stats(p1, p2, "P1 vs P2  [pt_pre + pt_model vs pt_pre + onnx_model] (model-graph)"))
    print(stats(p2, p3, "P2 vs P3  [pt_pre + onnx vs sidecar_pre + onnx]      (preprocess)"))
    print(stats(p1, p3, "P1 vs P3  [reference vs production path]              (combined)"))

    # ── Tensor-level preprocess comparison ───────────────────────────────
    print("\nPREPROCESS TENSOR-LEVEL COMPARISON (sidecar vs PyTorch tensors)")
    print("-" * 70)
    diffs = []
    for pt_t, sc_t in zip(pt_tensors, sidecar_tensors):
        # pt_t shape (1,3,224,224), sc_t shape (1,3,224,224)
        d = float(np.abs(pt_t - sc_t).max())
        m = float(np.abs(pt_t - sc_t).mean())
        diffs.append((d, m))
    max_abs = max(d[0] for d in diffs)
    mean_abs = float(np.mean([d[1] for d in diffs]))
    print(f"  Max abs(pt_pre - sidecar_pre):  {max_abs:.6f}")
    print(f"  Mean abs(pt_pre - sidecar_pre): {mean_abs:.6f}")
    if max_abs < 1e-5:
        print("  → Preprocesses produce IDENTICAL tensors.")
    elif max_abs < 0.05:
        print("  → Preprocesses produce SLIGHTLY DIFFERENT tensors (subpixel/antialias).")
    else:
        print("  → Preprocesses produce DIFFERENT tensors (>5% pixel-value delta).")

    # ── Verdict ──────────────────────────────────────────────────────────
    print("\n" + "=" * 70)
    print("DIAGNOSIS")
    print("-" * 70)
    p1p2 = float(np.sum(p1 * p2, axis=-1).mean())
    p2p3 = float(np.sum(p2 * p3, axis=-1).mean())
    if p1p2 >= 0.999 and p2p3 < 0.999:
        print("  → Root cause: PREPROCESS divergence (Suspect A)")
        print("    Model graph is faithful. Fix: align sidecar preprocess to open_clip's transform.")
    elif p1p2 < 0.999 and p2p3 >= 0.999:
        print("  → Root cause: MODEL GRAPH divergence (Suspect B)")
        print("    Preprocesses agree but ONNX model produces different math.")
        print("    Fix: re-export ONNX from the same weights with consistent op-fusion.")
    elif p1p2 < 0.999 and p2p3 < 0.999:
        print("  → Root cause: BOTH preprocess AND model graph diverge")
        print("    Fix both: re-export ONNX + align preprocess.")
    else:
        print("  → Both paths agree — gap is < 0.001 from reference; no fix needed.")

    return 0


if __name__ == "__main__":
    sys.exit(main())
