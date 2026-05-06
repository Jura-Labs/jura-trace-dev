#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — GBM Classifier Threshold Calibration

Runs the trained GBM classifier on a held-out corpus and sweeps the decision
threshold to find the operating point that satisfies given FP/recall targets.

Usage:
    python scripts/calibrate_gbm.py --model models/deepfake_classifier_v2.joblib \\
        --authentic "/Volumes/Samsung USB/.../authentic" \\
        --ai "/Volumes/Samsung USB/.../ai_generated" \\
        --target-fp 0.05 --min-recall 0.90

The script reuses the same feature extraction pipeline as training so the
results are directly comparable.
"""

import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path

# Reuse the sidecar feature extraction pipeline
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

import numpy as np
from app.services.deepfake import (
    FEATURE_NAMES,
    extract_feature_vector,
    extract_features_for_training,
)

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}


def collect_images(directory: str) -> list[Path]:
    d = Path(directory)
    if not d.exists():
        return []
    return sorted(
        f for f in d.rglob("*")
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
    )


def detect_mime(path: str, data: bytes) -> str:
    if data[:4] == b"\x89PNG":
        return "image/png"
    if data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        return "image/webp"
    return "image/jpeg"


def extract_all(images: list[Path], label: int) -> tuple[list[list[float]], list[int], list[str]]:
    X_rows: list[list[float]] = []
    y_labels: list[int] = []
    filenames: list[str] = []
    for i, p in enumerate(images, start=1):
        try:
            data = p.read_bytes()
            mime = detect_mime(str(p), data)
            features_dict = extract_features_for_training(data, mime, str(p))
            if features_dict is None:
                continue
            vec = extract_feature_vector(features_dict)
            if vec is None or len(vec) != len(FEATURE_NAMES):
                continue
            X_rows.append(vec)
            y_labels.append(label)
            filenames.append(p.name)
            if i % 200 == 0:
                print(f"    [{i}/{len(images)}] processed...")
        except Exception as e:
            print(f"    [{i}/{len(images)}] {p.name}: ERROR {e}")
    return X_rows, y_labels, filenames


def main():
    parser = argparse.ArgumentParser(description="Calibrate GBM classifier threshold")
    parser.add_argument("--model", required=True, help="Path to GBM .joblib model file")
    parser.add_argument("--authentic", required=True, help="Path to authentic image corpus")
    parser.add_argument("--ai", required=True, help="Path to AI-generated image corpus")
    parser.add_argument("--target-fp", type=float, default=0.05,
                        help="Target false positive rate (default 0.05 = 5%)")
    parser.add_argument("--min-recall", type=float, default=0.90,
                        help="Minimum AI recall to maintain (default 0.90 = 90%)")
    parser.add_argument("--output", default=None,
                        help="Output JSON path (default: alongside model file)")
    parser.add_argument("--sample", type=int, default=0,
                        help="Sample N images per class for fast calibration (0 = all)")
    args = parser.parse_args()

    print("=" * 60)
    print("  Jura Trace — GBM Classifier Calibration")
    print("=" * 60)

    # Load model
    import joblib
    print(f"\nLoading model: {args.model}")
    clf = joblib.load(args.model)
    print(f"  Model type: {type(clf).__name__}")

    # Collect images
    authentic_images = collect_images(args.authentic)
    ai_images = collect_images(args.ai)

    if args.sample > 0:
        import random
        random.seed(42)
        authentic_images = random.sample(authentic_images, min(args.sample, len(authentic_images)))
        ai_images = random.sample(ai_images, min(args.sample, len(ai_images)))

    print(f"\n  Authentic: {len(authentic_images)} images")
    print(f"  AI:        {len(ai_images)} images")

    # Extract features
    print("\nExtracting features from authentic...")
    X_auth, y_auth, _ = extract_all(authentic_images, label=0)
    print(f"  Got {len(X_auth)} valid feature vectors")

    print("\nExtracting features from AI...")
    X_ai, y_ai, _ = extract_all(ai_images, label=1)
    print(f"  Got {len(X_ai)} valid feature vectors")

    X = np.array(X_auth + X_ai, dtype=np.float64)
    y = np.array(y_auth + y_ai, dtype=np.int32)

    # Predict probabilities
    print("\nPredicting probabilities...")
    probs = clf.predict_proba(X)[:, 1]  # P(AI)

    # Threshold sweep
    print("\nThreshold sweep (0.10 → 0.90 in 0.02 steps):")
    print(f"  {'thresh':<10}{'FP rate':<12}{'AI recall':<12}{'precision':<12}{'F1':<8}")
    print("  " + "-" * 54)

    results = []
    for t in np.arange(0.10, 0.91, 0.02):
        pred = (probs >= t).astype(np.int32)
        tp = int(np.sum((pred == 1) & (y == 1)))
        fp = int(np.sum((pred == 1) & (y == 0)))
        tn = int(np.sum((pred == 0) & (y == 0)))
        fn = int(np.sum((pred == 0) & (y == 1)))
        n_auth = tn + fp
        n_ai = tp + fn
        fp_rate = fp / n_auth if n_auth > 0 else 0
        recall = tp / n_ai if n_ai > 0 else 0
        precision = tp / (tp + fp) if (tp + fp) > 0 else 0
        f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
        results.append({
            "threshold": round(float(t), 4),
            "fp_rate": round(fp_rate, 4),
            "recall": round(recall, 4),
            "precision": round(precision, 4),
            "f1": round(f1, 4),
            "tp": tp, "fp": fp, "tn": tn, "fn": fn,
        })
        marker = ""
        if fp_rate <= args.target_fp and recall >= args.min_recall:
            marker = "  ★"
        print(f"  {t:<10.2f}{fp_rate:<12.3%}{recall:<12.3%}{precision:<12.3%}{f1:<8.3f}{marker}")

    # Find best threshold
    print("\n" + "=" * 60)
    print(f"  Recommended threshold (target FP ≤ {args.target_fp:.0%}, recall ≥ {args.min_recall:.0%}):")

    candidates = [r for r in results if r["fp_rate"] <= args.target_fp and r["recall"] >= args.min_recall]
    if candidates:
        # Pick the one with highest F1
        best = max(candidates, key=lambda r: r["f1"])
        print(f"  → threshold = {best['threshold']:.2f}")
        print(f"     FP rate:   {best['fp_rate']:.3%}")
        print(f"     AI recall: {best['recall']:.3%}")
        print(f"     Precision: {best['precision']:.3%}")
        print(f"     F1:        {best['f1']:.3f}")
    else:
        # Fall back: best F1 overall
        best = max(results, key=lambda r: r["f1"])
        print(f"  No threshold satisfies both constraints. Best F1:")
        print(f"  → threshold = {best['threshold']:.2f}")
        print(f"     FP rate:   {best['fp_rate']:.3%}")
        print(f"     AI recall: {best['recall']:.3%}")
        print(f"     F1:        {best['f1']:.3f}")

    # Save results
    output_path = Path(args.output) if args.output else Path(args.model).with_suffix(".calibration.json")
    output_data = {
        "calibrated_at": datetime.now().isoformat(),
        "model": str(args.model),
        "n_authentic": int(np.sum(y == 0)),
        "n_ai": int(np.sum(y == 1)),
        "target_fp_rate": args.target_fp,
        "min_recall": args.min_recall,
        "recommended_threshold": best["threshold"],
        "recommended_metrics": {
            "fp_rate": best["fp_rate"],
            "recall": best["recall"],
            "precision": best["precision"],
            "f1": best["f1"],
        },
        "sweep": results,
    }
    output_path.write_text(json.dumps(output_data, indent=2))
    print(f"\n  Calibration saved: {output_path}")
    print("\n  Done.")


if __name__ == "__main__":
    main()
