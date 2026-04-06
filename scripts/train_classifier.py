#!/usr/bin/env python3
"""
Jura Trace -- Train AI-generated image classifier (GBM).

Extracts features from the authentic and AI-generated corpora using the
same pipeline as the production deepfake detector, then trains a
GradientBoostingClassifier and saves it to models/.

Usage:
    python scripts/train_classifier.py
    python scripts/train_classifier.py --authentic corpus/authentic --ai "/path/to/ai/images"
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path

# Add sidecar to path so we can import feature extraction directly
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

import numpy as np
from app.services.deepfake import (
    FEATURE_NAMES,
    extract_feature_vector,
    extract_features_for_training,
)

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}


def detect_mime(path: str, data: bytes) -> str:
    """Auto-detect MIME type from file bytes."""
    if data[:4] == b"\x89PNG":
        return "image/png"
    if data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "image/webp"
    if data[:2] == b"\xff\xd8":
        return "image/jpeg"
    ext = Path(path).suffix.lower()
    return {
        ".png": "image/png",
        ".jpg": "image/jpeg",
        ".jpeg": "image/jpeg",
        ".webp": "image/webp",
        ".tiff": "image/tiff",
        ".tif": "image/tiff",
    }.get(ext, "image/jpeg")


def collect_images(directory: str) -> list[Path]:
    """Collect image files from a directory (recursive)."""
    d = Path(directory)
    if not d.exists():
        print(f"  WARNING: directory not found: {d}")
        return []
    return sorted(
        f for f in d.rglob("*")
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
    )


def extract_all(images: list[Path], label: int) -> tuple[list[list[float]], list[int], list[str]]:
    """Extract feature vectors for a list of images.

    Returns (X_rows, y_labels, filenames).
    """
    X_rows: list[list[float]] = []
    y_labels: list[int] = []
    filenames: list[str] = []

    for i, img_path in enumerate(images):
        print(f"    [{i + 1}/{len(images)}] {img_path.name}...", end=" ", flush=True)
        try:
            data = img_path.read_bytes()
            mime = detect_mime(str(img_path), data)
            features, codec_class = extract_features_for_training(data, mime_type=mime)
            vec = extract_feature_vector(features)
            # Replace NaN with 0.0 for training
            vec_clean = [0.0 if np.isnan(v) else v for v in vec]
            X_rows.append(vec_clean)
            y_labels.append(label)
            filenames.append(img_path.name)
            print("OK")
        except Exception as e:
            print(f"FAILED ({e})")

    return X_rows, y_labels, filenames


def main():
    parser = argparse.ArgumentParser(description="Train AI-generated image classifier")
    parser.add_argument(
        "--authentic",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "corpus", "authentic"),
        help="Path to authentic image corpus",
    )
    parser.add_argument(
        "--ai",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "corpus", "ai_generated"),
        help="Path to AI-generated image corpus (default: corpus/ai_generated)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "models"),
        help="Output directory for model files",
    )
    args = parser.parse_args()

    print("Jura Trace -- AI Image Classifier Training")
    print("=" * 60)

    # Collect images
    authentic_images = collect_images(args.authentic)
    ai_images = collect_images(args.ai)
    print(f"  Authentic images: {len(authentic_images)}")
    print(f"  AI-generated images: {len(ai_images)}")

    if len(authentic_images) < 3 or len(ai_images) < 3:
        print("\nERROR: Need at least 3 images per class for training.")
        sys.exit(1)

    # Extract features
    print("\nExtracting features from authentic images...")
    X_auth, y_auth, f_auth = extract_all(authentic_images, label=0)

    print("\nExtracting features from AI-generated images...")
    X_ai, y_ai, f_ai = extract_all(ai_images, label=1)

    if len(X_auth) < 3 or len(X_ai) < 3:
        print("\nERROR: Too few valid images after extraction.")
        sys.exit(1)

    # Build matrices
    X = np.array(X_auth + X_ai, dtype=np.float64)
    y = np.array(y_auth + y_ai, dtype=np.int32)
    filenames = f_auth + f_ai

    print(f"\nTotal samples: {len(y)} (authentic={sum(y == 0)}, ai={sum(y == 1)})")
    print(f"Feature vector size: {X.shape[1]}")

    # Replace any remaining inf/nan
    X = np.nan_to_num(X, nan=0.0, posinf=1e6, neginf=-1e6)

    # Train
    from sklearn.ensemble import GradientBoostingClassifier
    from sklearn.model_selection import StratifiedKFold, cross_val_predict
    from sklearn.metrics import (
        classification_report,
        roc_auc_score,
        confusion_matrix,
    )

    print("\nTraining GradientBoostingClassifier...")
    print("  n_estimators=200, max_depth=4, learning_rate=0.05")

    clf = GradientBoostingClassifier(
        n_estimators=200,
        max_depth=4,
        learning_rate=0.05,
        random_state=42,
        min_samples_leaf=2,
        subsample=0.8,
    )

    # 5-fold stratified cross-validation
    n_splits = min(5, min(sum(y == 0), sum(y == 1)))
    if n_splits < 2:
        print(f"\nWARNING: Only {n_splits} samples in smallest class, using leave-one-out.")
        n_splits = min(len(y), 5)

    print(f"  Cross-validation: {n_splits}-fold stratified")

    cv = StratifiedKFold(n_splits=n_splits, shuffle=True, random_state=42)

    t0 = time.time()
    y_pred_proba = cross_val_predict(clf, X, y, cv=cv, method="predict_proba")
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
        print(f"  {fname:40s}  true={label_str:10s}  pred={pred_str:10s}  prob_ai={y_pred_proba[i, 1]:.3f}  {correct}")

    # Train final model on all data
    print("\n" + "=" * 60)
    print("Training final model on all data...")
    clf.fit(X, y)

    # Feature importance
    importances = clf.feature_importances_
    top_indices = np.argsort(importances)[::-1][:15]
    print("\nTop 15 feature importances:")
    for idx in top_indices:
        print(f"  {FEATURE_NAMES[idx]:35s}  {importances[idx]:.4f}")

    # Save model
    import joblib
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    model_path = output_dir / "deepfake_classifier.joblib"
    joblib.dump(clf, model_path)
    print(f"\nModel saved: {model_path}")

    # Save metadata
    meta = {
        "trained_at": datetime.now().isoformat(),
        "n_samples": int(len(y)),
        "n_authentic": int(sum(y == 0)),
        "n_ai_generated": int(sum(y == 1)),
        "n_features": int(X.shape[1]),
        "feature_names": FEATURE_NAMES,
        "cv_folds": n_splits,
        "cv_auc_roc": round(float(auc), 4),
        "model_params": {
            "n_estimators": 200,
            "max_depth": 4,
            "learning_rate": 0.05,
            "min_samples_leaf": 2,
            "subsample": 0.8,
        },
        "top_features": [
            {"name": FEATURE_NAMES[idx], "importance": round(float(importances[idx]), 4)}
            for idx in top_indices
        ],
    }
    meta_path = output_dir / "deepfake_classifier_meta.json"
    meta_path.write_text(json.dumps(meta, indent=2))
    print(f"Metadata saved: {meta_path}")

    print("\nDone.")


if __name__ == "__main__":
    main()
