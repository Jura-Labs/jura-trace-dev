#!/usr/bin/env python3
"""
Jura Trace -- Evaluate trained AI-generated image classifier.

Loads the trained model and evaluates it on the training corpus
(with cross-validation results from metadata) and optionally on
a separate test set.

Usage:
    python scripts/evaluate_classifier.py
    python scripts/evaluate_classifier.py --test-dir /path/to/test/images --label ai
"""

import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

import numpy as np
import joblib

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
    """Collect image files from a directory."""
    d = Path(directory)
    if not d.exists():
        print(f"  WARNING: directory not found: {d}")
        return []
    return sorted(
        f for f in d.iterdir()
        if f.suffix.lower() in IMAGE_EXTENSIONS
    )


def main():
    parser = argparse.ArgumentParser(description="Evaluate AI-generated image classifier")
    parser.add_argument(
        "--model",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "models", "deepfake_classifier.joblib"),
        help="Path to trained model",
    )
    parser.add_argument(
        "--meta",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "models", "deepfake_classifier_meta.json"),
        help="Path to model metadata",
    )
    parser.add_argument(
        "--test-dir",
        type=str,
        default=None,
        help="Optional test directory to evaluate on",
    )
    parser.add_argument(
        "--label",
        type=str,
        choices=["authentic", "ai"],
        default="ai",
        help="Label for test-dir images (default: ai)",
    )
    parser.add_argument(
        "--authentic",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "corpus", "authentic"),
        help="Path to authentic corpus (for re-evaluation)",
    )
    parser.add_argument(
        "--ai",
        type=str,
        default="/Users/paulgriffiths/Desktop/Fake Images AI",
        help="Path to AI corpus (for re-evaluation)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "models"),
        help="Output directory for evaluation report",
    )
    args = parser.parse_args()

    print("Jura Trace -- Classifier Evaluation")
    print("=" * 60)

    # Load model
    model_path = Path(args.model)
    if not model_path.exists():
        print(f"ERROR: Model not found: {model_path}")
        sys.exit(1)

    clf = joblib.load(model_path)
    print(f"  Model loaded: {model_path}")

    # Load metadata
    meta_path = Path(args.meta)
    meta = {}
    if meta_path.exists():
        meta = json.loads(meta_path.read_text())
        print(f"  Training AUC-ROC: {meta.get('cv_auc_roc', 'N/A')}")
        print(f"  Training samples: {meta.get('n_samples', 'N/A')}")
        print(f"  Features: {meta.get('n_features', 'N/A')}")

    # Re-evaluate on training data
    print("\n" + "-" * 60)
    print("RE-EVALUATION ON TRAINING CORPORA")
    print("-" * 60)

    from sklearn.metrics import classification_report, roc_auc_score, confusion_matrix

    all_X = []
    all_y = []
    all_names = []

    for corpus_path, label, label_name in [
        (args.authentic, 0, "authentic"),
        (args.ai, 1, "ai_generated"),
    ]:
        images = collect_images(corpus_path)
        print(f"\n  Processing {label_name}: {len(images)} images")

        for i, img_path in enumerate(images):
            try:
                data = img_path.read_bytes()
                mime = detect_mime(str(img_path), data)
                features, _ = extract_features_for_training(data, mime_type=mime)
                vec = extract_feature_vector(features)
                vec_clean = [0.0 if np.isnan(v) else v for v in vec]
                all_X.append(vec_clean)
                all_y.append(label)
                all_names.append(img_path.name)
            except Exception as e:
                print(f"    FAILED: {img_path.name} ({e})")

    if all_X:
        X = np.array(all_X, dtype=np.float64)
        y = np.array(all_y, dtype=np.int32)
        X = np.nan_to_num(X, nan=0.0, posinf=1e6, neginf=-1e6)

        y_pred_proba = clf.predict_proba(X)
        y_pred = (y_pred_proba[:, 1] >= 0.5).astype(int)

        try:
            auc = roc_auc_score(y, y_pred_proba[:, 1])
            print(f"\n  AUC-ROC: {auc:.4f}")
        except ValueError:
            auc = 0.0
            print("\n  AUC-ROC: could not compute")

        print(f"\n{classification_report(y, y_pred, target_names=['authentic', 'ai_generated'])}")

        cm = confusion_matrix(y, y_pred)
        print("  Confusion Matrix:")
        print(f"    {'':12s} pred_auth  pred_ai")
        print(f"    {'authentic':12s}  {cm[0, 0]:5d}     {cm[0, 1]:5d}")
        print(f"    {'ai_generated':12s}  {cm[1, 0]:5d}     {cm[1, 1]:5d}")

        # Per-sample details
        print("\n  Per-sample predictions:")
        for i, fname in enumerate(all_names):
            label_str = "authentic" if y[i] == 0 else "ai"
            pred_str = "authentic" if y_pred[i] == 0 else "ai"
            correct = "OK" if y[i] == y_pred[i] else "WRONG"
            print(f"    {fname:40s}  true={label_str:10s}  pred={pred_str:10s}  prob_ai={y_pred_proba[i, 1]:.3f}  {correct}")

    # Optional test directory
    if args.test_dir:
        print("\n" + "-" * 60)
        print(f"TEST SET EVALUATION: {args.test_dir}")
        print("-" * 60)

        test_images = collect_images(args.test_dir)
        test_label = 1 if args.label == "ai" else 0
        print(f"  Images: {len(test_images)}, expected label: {args.label}")

        test_X = []
        test_y = []
        test_names = []

        for img_path in test_images:
            try:
                data = img_path.read_bytes()
                mime = detect_mime(str(img_path), data)
                features, _ = extract_features_for_training(data, mime_type=mime)
                vec = extract_feature_vector(features)
                vec_clean = [0.0 if np.isnan(v) else v for v in vec]
                test_X.append(vec_clean)
                test_y.append(test_label)
                test_names.append(img_path.name)
            except Exception as e:
                print(f"    FAILED: {img_path.name} ({e})")

        if test_X:
            X_test = np.nan_to_num(
                np.array(test_X, dtype=np.float64), nan=0.0, posinf=1e6, neginf=-1e6,
            )
            y_test = np.array(test_y, dtype=np.int32)
            y_test_proba = clf.predict_proba(X_test)
            y_test_pred = (y_test_proba[:, 1] >= 0.5).astype(int)

            correct = sum(y_test == y_test_pred)
            print(f"\n  Accuracy: {correct}/{len(y_test)} ({100 * correct / len(y_test):.1f}%)")

            for i, fname in enumerate(test_names):
                pred_str = "authentic" if y_test_pred[i] == 0 else "ai"
                match = "OK" if y_test[i] == y_test_pred[i] else "WRONG"
                print(f"    {fname:40s}  pred={pred_str:10s}  prob_ai={y_test_proba[i, 1]:.3f}  {match}")

    # Save evaluation report
    report = {
        "evaluated_at": datetime.now().isoformat(),
        "model_path": str(model_path),
        "training_meta": meta,
        "reeval_auc_roc": round(float(auc), 4) if all_X else None,
        "reeval_n_samples": len(all_y) if all_X else 0,
    }
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)
    report_path = output_dir / "evaluation_report.json"
    report_path.write_text(json.dumps(report, indent=2))
    print(f"\nEvaluation report saved: {report_path}")
    print("Done.")


if __name__ == "__main__":
    main()
