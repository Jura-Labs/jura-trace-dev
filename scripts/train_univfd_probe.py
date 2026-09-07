#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace -- Train a UnivFD-style linear probe on CLIP ViT-B/32 embeddings.

Extracts CLIP image embeddings from authentic and AI-generated corpora,
trains a LogisticRegression probe, evaluates via 5-fold stratified CV,
and saves the probe weights + metadata.

The probe is a lightweight (6KB) linear classifier on top of frozen CLIP
features — the same approach used by the UnivFD paper (Ojha et al. 2023)
for universal fake detection.

Usage:
    python scripts/train_univfd_probe.py
    python scripts/train_univfd_probe.py --authentic corpus/authentic --ai "/path/to/ai/images"
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path

# Add sidecar to path so we can import shared utilities
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

import numpy as np
from PIL import Image

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}


def collect_images(directory: str) -> list[Path]:
    """Collect image files from a directory."""
    d = Path(directory)
    if not d.exists():
        print(f"  WARNING: directory not found: {d}")
        return []
    return sorted(
        f for f in d.rglob("*")
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
    )


def extract_clip_embedding(img_path: Path, model, preprocess) -> np.ndarray | None:
    """Extract a normalised CLIP image embedding (512-dim for ViT-B/32).

    Returns None if the image cannot be processed.
    """
    import torch

    try:
        pil_img = Image.open(img_path).convert("RGB")
        image_input = preprocess(pil_img).unsqueeze(0)

        with torch.no_grad():
            features = model.encode_image(image_input)
            # L2-normalise to unit sphere (matches UnivFD approach)
            features = features / features.norm(dim=-1, keepdim=True)

        return features.squeeze().numpy().astype(np.float32)
    except Exception as e:
        print(f"FAILED ({e})")
        return None


def extract_all_embeddings(
    images: list[Path], label: int, model, preprocess,
) -> tuple[list[np.ndarray], list[int], list[str]]:
    """Extract CLIP embeddings for a list of images.

    Returns (embeddings, labels, filenames).
    """
    embeddings: list[np.ndarray] = []
    labels: list[int] = []
    filenames: list[str] = []

    for i, img_path in enumerate(images):
        print(f"    [{i + 1}/{len(images)}] {img_path.name}...", end=" ", flush=True)
        emb = extract_clip_embedding(img_path, model, preprocess)
        if emb is not None:
            embeddings.append(emb)
            labels.append(label)
            filenames.append(img_path.name)
            print("OK")
        # extract_clip_embedding already prints FAILED on error

    return embeddings, labels, filenames


def main():
    parser = argparse.ArgumentParser(
        description="Train UnivFD-style linear probe on CLIP embeddings"
    )
    parser.add_argument(
        "--authentic",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "corpus", "authentic"),
        help="Path to authentic image corpus",
    )
    parser.add_argument(
        "--ai",
        type=str,
        default=os.environ.get(
            "JURA_AI_CORPUS",
            os.path.join(os.path.dirname(__file__), "..", "corpus", "ai-generated"),
        ),
        help="Path to AI-generated image corpus",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "models"),
        help="Output directory for model files",
    )
    parser.add_argument(
        "--C",
        type=float,
        default=0.5,
        help="Regularisation strength for LogisticRegression (higher = less regularisation)",
    )
    args = parser.parse_args()

    # Check dependencies
    try:
        import torch
        import open_clip
    except ImportError:
        print(
            "Error: open_clip and torch required.\n"
            "Install with: pip install open-clip-torch torch"
        )
        sys.exit(1)

    from sklearn.linear_model import LogisticRegression
    from sklearn.model_selection import StratifiedKFold, cross_val_predict
    from sklearn.metrics import classification_report, roc_auc_score, confusion_matrix
    import joblib

    print("Jura Trace -- UnivFD Linear Probe Training")
    print("=" * 60)

    # Load CLIP model
    print("\nLoading CLIP ViT-B-32 model...")
    model, _, preprocess = open_clip.create_model_and_transforms(
        "ViT-B-32", pretrained="laion2b_s34b_b79k"
    )
    model.eval()
    print("  Model loaded successfully.")

    # Collect images
    authentic_images = collect_images(args.authentic)
    ai_images = collect_images(args.ai)
    print(f"\n  Authentic images: {len(authentic_images)}")
    print(f"  AI-generated images: {len(ai_images)}")

    if len(authentic_images) < 3 or len(ai_images) < 3:
        print("\nERROR: Need at least 3 images per class for training.")
        sys.exit(1)

    # Extract embeddings
    print("\nExtracting CLIP embeddings from authentic images...")
    emb_auth, y_auth, f_auth = extract_all_embeddings(
        authentic_images, label=0, model=model, preprocess=preprocess,
    )

    print("\nExtracting CLIP embeddings from AI-generated images...")
    emb_ai, y_ai, f_ai = extract_all_embeddings(
        ai_images, label=1, model=model, preprocess=preprocess,
    )

    if len(emb_auth) < 3 or len(emb_ai) < 3:
        print("\nERROR: Too few valid images after extraction.")
        sys.exit(1)

    # Build matrices
    X = np.array(emb_auth + emb_ai, dtype=np.float32)
    y = np.array(y_auth + y_ai, dtype=np.int32)
    filenames = f_auth + f_ai

    print(f"\nTotal samples: {len(y)} (authentic={sum(y == 0)}, ai={sum(y == 1)})")
    print(f"Embedding dimension: {X.shape[1]}")

    # Train probe: LogisticRegression with balanced class weights.
    # class_weight='balanced' offsets the authentic:AI imbalance by weighting
    # each class inversely proportional to its frequency. Without it, the
    # decision boundary shifts toward predicting "authentic" when the authentic
    # corpus is larger, causing AI recall to drop dramatically.
    # C=0.5 (was 0.1) — less aggressive regularisation with more training data.
    print(f"\nTraining LogisticRegression probe (C={args.C}, balanced, max_iter=1000)...")

    probe = LogisticRegression(
        C=args.C,
        max_iter=1000,
        random_state=42,
        solver="lbfgs",
        class_weight="balanced",
    )

    # 5-fold stratified cross-validation
    n_splits = min(5, min(sum(y == 0), sum(y == 1)))
    if n_splits < 2:
        print(f"\nWARNING: Only {n_splits} samples in smallest class.")
        n_splits = min(len(y), 5)

    print(f"  Cross-validation: {n_splits}-fold stratified")

    cv = StratifiedKFold(n_splits=n_splits, shuffle=True, random_state=42)

    t0 = time.time()
    y_pred_proba = cross_val_predict(probe, X, y, cv=cv, method="predict_proba")
    cv_time = time.time() - t0
    y_pred = (y_pred_proba[:, 1] >= 0.5).astype(int)

    print(f"  Cross-validation time: {cv_time:.1f}s")

    # Metrics
    print("\n" + "=" * 60)
    print("CROSS-VALIDATION RESULTS")
    print("=" * 60)

    try:
        auc = roc_auc_score(y, y_pred_proba[:, 1])
        print(f"\n  AUC-ROC: {auc:.4f}")
    except ValueError:
        auc = 0.0
        print("\n  AUC-ROC: could not compute (single class in fold)")

    print(f"\n{classification_report(y, y_pred, target_names=['authentic', 'ai_generated'])}")

    cm = confusion_matrix(y, y_pred)
    print("Confusion Matrix:")
    print(f"  {'':12s} pred_auth  pred_ai")
    print(f"  {'authentic':12s}  {cm[0, 0]:5d}     {cm[0, 1]:5d}")
    print(f"  {'ai_generated':12s}  {cm[1, 0]:5d}     {cm[1, 1]:5d}")

    # Per-sample results
    print("\nPer-sample predictions (cross-validated):")
    for i, fname in enumerate(filenames):
        label_str = "authentic" if y[i] == 0 else "ai"
        pred_str = "authentic" if y_pred[i] == 0 else "ai"
        correct = "OK" if y[i] == y_pred[i] else "WRONG"
        print(
            f"  {fname:40s}  true={label_str:10s}  pred={pred_str:10s}"
            f"  prob_ai={y_pred_proba[i, 1]:.3f}  {correct}"
        )

    # Train final model on all data
    print("\n" + "=" * 60)
    print("Training final probe on all data...")
    probe.fit(X, y)

    # Save probe
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    model_path = output_dir / "univfd_probe.joblib"
    joblib.dump(probe, model_path)
    print(f"\nProbe saved: {model_path}")
    print(f"  File size: {model_path.stat().st_size / 1024:.1f} KB")

    # Save metadata
    meta = {
        "trained_at": datetime.now().isoformat(),
        "approach": "UnivFD-style linear probe on CLIP ViT-B/32 embeddings",
        "clip_model": "ViT-B-32",
        "clip_pretrained": "laion2b_s34b_b79k",
        "embedding_dim": int(X.shape[1]),
        "n_samples": int(len(y)),
        "n_authentic": int(sum(y == 0)),
        "n_ai_generated": int(sum(y == 1)),
        "cv_folds": n_splits,
        "cv_auc_roc": round(float(auc), 4),
        "probe_params": {
            "C": 0.1,
            "max_iter": 1000,
            "solver": "lbfgs",
        },
        "cv_classification_report": classification_report(
            y, y_pred, target_names=["authentic", "ai_generated"], output_dict=True,
        ),
    }
    meta_path = output_dir / "univfd_probe_meta.json"
    meta_path.write_text(json.dumps(meta, indent=2))
    print(f"Metadata saved: {meta_path}")

    print("\nDone.")


if __name__ == "__main__":
    main()
