#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace -- False Positive Rate Analysis

Loads the current GBM classifier, runs it against the corpus (authentic
and AI-generated), and produces a detailed analysis of false positives
and false negatives. Identifies which types of authentic images cause
false positives and recommends threshold adjustments.

Outputs:
  - Console report with key metrics
  - docs/fp-analysis-report.md -- detailed Markdown report
  - models/fp_analysis.json -- machine-readable results
  - models/threshold_sweep.csv -- FP/FN rates at every threshold
  - models/per_sample_results.csv -- per-image predictions

Usage:
    python scripts/analyse_fp_rate.py
    python scripts/analyse_fp_rate.py --model models/deepfake_classifier.joblib
    python scripts/analyse_fp_rate.py --test-split 0.2 --seed 42
"""

import argparse
import csv
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path

# Add sidecar to path for feature extraction
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

import numpy as np


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BASE_DIR = Path(__file__).resolve().parent.parent
CORPUS_DIR = BASE_DIR / "corpus"
AUTHENTIC_DIR = CORPUS_DIR / "authentic"
AI_DIR = CORPUS_DIR / "ai_generated"
MODELS_DIR = BASE_DIR / "models"
DOCS_DIR = BASE_DIR / "docs"
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".tiff", ".tif"}


# ---------------------------------------------------------------------------
# Feature extraction
# ---------------------------------------------------------------------------


def detect_mime(path: str, data: bytes) -> str:
    """Auto-detect MIME type from file bytes."""
    if data[:4] == b"\x89PNG":
        return "image/png"
    if data[:4] == b"RIFF" and len(data) > 12 and data[8:12] == b"WEBP":
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


def collect_images(directory: Path) -> list[Path]:
    """Collect image files from a directory."""
    if not directory.exists():
        return []
    return sorted(f for f in directory.iterdir() if f.suffix.lower() in IMAGE_EXTENSIONS)


def extract_features_batch(images: list[Path], label: int, verbose: bool = True) -> tuple:
    """Extract feature vectors for a list of images.

    Returns (X_rows, y_labels, filenames, metadata_list).
    """
    from app.services.deepfake import (
        FEATURE_NAMES,
        extract_feature_vector,
        extract_features_for_training,
    )

    X_rows = []
    y_labels = []
    filenames = []
    metadata_list = []

    for i, img_path in enumerate(images):
        if verbose:
            print(f"    [{i + 1}/{len(images)}] {img_path.name}...", end=" ", flush=True)

        try:
            data = img_path.read_bytes()
            mime = detect_mime(str(img_path), data)
            features, codec_class = extract_features_for_training(data, mime_type=mime)
            vec = extract_feature_vector(features)
            vec_clean = [0.0 if np.isnan(v) else v for v in vec]

            X_rows.append(vec_clean)
            y_labels.append(label)
            filenames.append(img_path.name)
            metadata_list.append({
                "filename": img_path.name,
                "mime": mime,
                "codec_class": codec_class,
                "size_bytes": len(data),
                "label": "authentic" if label == 0 else "ai_generated",
                "features": {name: val for name, val in zip(FEATURE_NAMES, vec_clean)},
            })
            if verbose:
                print("OK")
        except Exception as e:
            if verbose:
                print(f"FAILED ({e})")

    return X_rows, y_labels, filenames, metadata_list


# ---------------------------------------------------------------------------
# Threshold sweep
# ---------------------------------------------------------------------------


def threshold_sweep(
    y_true: np.ndarray,
    y_proba: np.ndarray,
    thresholds: list[float] | None = None,
) -> list[dict]:
    """Evaluate FP rate and detection rate across a range of thresholds.

    Returns a list of dicts with threshold, fp_rate, fn_rate, detection_rate,
    precision, recall, f1, and sample counts.
    """
    if thresholds is None:
        thresholds = [round(t, 3) for t in np.arange(0.05, 0.96, 0.025)]

    results = []
    n_authentic = int(np.sum(y_true == 0))
    n_ai = int(np.sum(y_true == 1))

    for thresh in thresholds:
        y_pred = (y_proba >= thresh).astype(int)

        # Authentic images predicted as AI (false positives)
        fp = int(np.sum((y_true == 0) & (y_pred == 1)))
        # AI images predicted as authentic (false negatives)
        fn = int(np.sum((y_true == 1) & (y_pred == 0)))
        # True positives (AI correctly flagged)
        tp = int(np.sum((y_true == 1) & (y_pred == 1)))
        # True negatives (authentic correctly passed)
        tn = int(np.sum((y_true == 0) & (y_pred == 0)))

        fp_rate = fp / n_authentic if n_authentic > 0 else 0
        fn_rate = fn / n_ai if n_ai > 0 else 0
        detection_rate = tp / n_ai if n_ai > 0 else 0
        precision = tp / (tp + fp) if (tp + fp) > 0 else 0
        recall = detection_rate
        f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0

        results.append({
            "threshold": round(thresh, 3),
            "fp_count": fp,
            "fn_count": fn,
            "tp_count": tp,
            "tn_count": tn,
            "fp_rate": round(fp_rate, 4),
            "fn_rate": round(fn_rate, 4),
            "detection_rate": round(detection_rate, 4),
            "precision": round(precision, 4),
            "recall": round(recall, 4),
            "f1": round(f1, 4),
        })

    return results


def find_optimal_thresholds(sweep_results: list[dict]) -> dict:
    """Find key threshold points from a sweep."""
    # Best threshold for < 5% FP
    under_5_fp = [r for r in sweep_results if r["fp_rate"] < 0.05]
    best_under_5 = max(under_5_fp, key=lambda r: r["detection_rate"]) if under_5_fp else None

    # Best threshold for < 10% FP
    under_10_fp = [r for r in sweep_results if r["fp_rate"] < 0.10]
    best_under_10 = max(under_10_fp, key=lambda r: r["detection_rate"]) if under_10_fp else None

    # Best F1
    best_f1 = max(sweep_results, key=lambda r: r["f1"])

    # Equal error rate (FP rate ~ FN rate)
    eer = min(sweep_results, key=lambda r: abs(r["fp_rate"] - r["fn_rate"]))

    return {
        "best_under_5pct_fp": best_under_5,
        "best_under_10pct_fp": best_under_10,
        "best_f1": best_f1,
        "equal_error_rate": eer,
    }


# ---------------------------------------------------------------------------
# FP analysis
# ---------------------------------------------------------------------------


def analyse_false_positives(
    metadata_list: list[dict],
    y_true: np.ndarray,
    y_proba: np.ndarray,
    threshold: float = 0.5,
) -> dict:
    """Deep analysis of which authentic images produce false positives.

    Groups FPs by source, codec, and feature values to identify patterns.
    """
    y_pred = (y_proba >= threshold).astype(int)

    # Identify FP images
    fp_images = []
    for i, meta in enumerate(metadata_list):
        if y_true[i] == 0 and y_pred[i] == 1:
            meta_copy = dict(meta)
            meta_copy["ai_probability"] = round(float(y_proba[i]), 4)
            fp_images.append(meta_copy)

    # Identify FN images
    fn_images = []
    for i, meta in enumerate(metadata_list):
        if y_true[i] == 1 and y_pred[i] == 0:
            meta_copy = dict(meta)
            meta_copy["ai_probability"] = round(float(y_proba[i]), 4)
            fn_images.append(meta_copy)

    # Group FPs by characteristics
    fp_by_codec = {}
    fp_by_source = {}
    fp_feature_stats = {}

    for fp in fp_images:
        codec = fp.get("codec_class", "unknown")
        fp_by_codec[codec] = fp_by_codec.get(codec, 0) + 1

        # Infer source from filename prefix
        fname = fp["filename"]
        if fname.startswith("coco_"):
            source = "coco"
        elif fname.startswith("unsplash_"):
            source = "unsplash"
        elif fname.startswith("wikimedia_"):
            source = "wikimedia"
        elif fname.startswith("edge_"):
            source = "edge_case"
        elif "guardian" in fname.lower() or fname[0:4].isdigit():
            source = "guardian"
        else:
            source = "other"
        fp_by_source[source] = fp_by_source.get(source, 0) + 1

        # Collect feature values for FP images
        for feat_name, feat_val in fp.get("features", {}).items():
            if feat_name not in fp_feature_stats:
                fp_feature_stats[feat_name] = []
            fp_feature_stats[feat_name].append(feat_val)

    # Compute feature statistics for FP images vs all authentic
    auth_feature_stats = {}
    for i, meta in enumerate(metadata_list):
        if y_true[i] == 0:
            for feat_name, feat_val in meta.get("features", {}).items():
                if feat_name not in auth_feature_stats:
                    auth_feature_stats[feat_name] = []
                auth_feature_stats[feat_name].append(feat_val)

    # Find features where FP images differ most from authentic population
    feature_divergence = []
    for feat_name in fp_feature_stats:
        if feat_name not in auth_feature_stats:
            continue
        fp_vals = np.array(fp_feature_stats[feat_name])
        auth_vals = np.array(auth_feature_stats[feat_name])
        if len(fp_vals) < 2 or len(auth_vals) < 2:
            continue
        # Use effect size (Cohen's d) to measure divergence
        pooled_std = np.sqrt((np.var(fp_vals) + np.var(auth_vals)) / 2)
        if pooled_std > 1e-10:
            cohens_d = abs(np.mean(fp_vals) - np.mean(auth_vals)) / pooled_std
        else:
            cohens_d = 0.0
        feature_divergence.append({
            "feature": feat_name,
            "cohens_d": round(cohens_d, 3),
            "fp_mean": round(float(np.mean(fp_vals)), 4),
            "auth_mean": round(float(np.mean(auth_vals)), 4),
            "fp_std": round(float(np.std(fp_vals)), 4),
            "auth_std": round(float(np.std(auth_vals)), 4),
        })

    feature_divergence.sort(key=lambda x: x["cohens_d"], reverse=True)

    return {
        "fp_count": len(fp_images),
        "fn_count": len(fn_images),
        "fp_by_codec": fp_by_codec,
        "fp_by_source": fp_by_source,
        "fp_images": fp_images[:50],  # Top 50 FPs for review
        "fn_images": fn_images[:30],  # Top 30 FNs for review
        "top_divergent_features": feature_divergence[:20],
    }


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------


def write_markdown_report(
    analysis: dict,
    sweep_results: list[dict],
    optimal: dict,
    n_authentic: int,
    n_ai: int,
    model_path: str,
    auc: float,
) -> str:
    """Generate the FP analysis Markdown report."""

    lines = [
        "# Jura Trace -- False Positive Analysis Report",
        "",
        f"**Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M')}",
        f"**Model:** `{model_path}`",
        f"**Corpus:** {n_authentic} authentic + {n_ai} AI-generated = {n_authentic + n_ai} total",
        f"**Cross-validated AUC-ROC:** {auc:.4f}",
        "",
        "---",
        "",
        "## Executive Summary",
        "",
    ]

    fp_rate_50 = next((r for r in sweep_results if abs(r["threshold"] - 0.5) < 0.01), None)
    if fp_rate_50:
        lines.append(f"At the current threshold (0.50), the false positive rate is "
                      f"**{fp_rate_50['fp_rate'] * 100:.1f}%** ({fp_rate_50['fp_count']} of {n_authentic} "
                      f"authentic images flagged as AI-generated). The detection rate is "
                      f"**{fp_rate_50['detection_rate'] * 100:.1f}%** ({fp_rate_50['tp_count']} of {n_ai} "
                      f"AI images correctly identified).")
        lines.append("")

    best5 = optimal.get("best_under_5pct_fp")
    if best5:
        lines.append(f"**Achievable with threshold tuning alone:** Moving the threshold to "
                      f"**{best5['threshold']}** reduces the FP rate to "
                      f"**{best5['fp_rate'] * 100:.1f}%** while maintaining a detection rate of "
                      f"**{best5['detection_rate'] * 100:.1f}%**.")
    else:
        lines.append("**WARNING:** No threshold achieves < 5% FP rate on this corpus. "
                      "Model architecture changes (CLIP features, calibration layer) "
                      "are required.")
    lines.append("")

    # False positive breakdown
    lines.extend([
        "---",
        "",
        "## False Positive Breakdown (threshold = 0.50)",
        "",
    ])

    # By source
    fp_by_source = analysis.get("fp_by_source", {})
    if fp_by_source:
        lines.extend([
            "### By Source",
            "",
            "| Source | FP Count | Notes |",
            "|--------|----------|-------|",
        ])
        for src, count in sorted(fp_by_source.items(), key=lambda x: -x[1]):
            lines.append(f"| {src} | {count} | |")
        lines.append("")

    # By codec
    fp_by_codec = analysis.get("fp_by_codec", {})
    if fp_by_codec:
        lines.extend([
            "### By Codec / Format",
            "",
            "| Codec | FP Count |",
            "|-------|----------|",
        ])
        for codec, count in sorted(fp_by_codec.items(), key=lambda x: -x[1]):
            lines.append(f"| {codec} | {count} |")
        lines.append("")

    # Most divergent features (what makes FP images different)
    divergent = analysis.get("top_divergent_features", [])
    if divergent:
        lines.extend([
            "### Features Most Different in FP Images",
            "",
            "These features have the largest effect size (Cohen's d) between",
            "false-positive authentic images and correctly classified authentic images.",
            "They indicate what the classifier is misinterpreting.",
            "",
            "| Feature | Cohen's d | FP Mean | Auth Mean | FP Std | Auth Std |",
            "|---------|-----------|---------|-----------|--------|----------|",
        ])
        for f in divergent[:15]:
            lines.append(
                f"| {f['feature']} | {f['cohens_d']} | {f['fp_mean']} | "
                f"{f['auth_mean']} | {f['fp_std']} | {f['auth_std']} |"
            )
        lines.append("")

    # Specific FP images
    fp_images = analysis.get("fp_images", [])
    if fp_images:
        lines.extend([
            "### Top False Positive Images",
            "",
            "| Filename | AI Probability | Codec | Size |",
            "|----------|---------------|-------|------|",
        ])
        for fp in sorted(fp_images, key=lambda x: -x["ai_probability"])[:25]:
            size_kb = fp.get("size_bytes", 0) // 1024
            lines.append(
                f"| {fp['filename']} | {fp['ai_probability']:.3f} | "
                f"{fp.get('codec_class', '?')} | {size_kb} KB |"
            )
        lines.append("")

    # Threshold sweep table
    lines.extend([
        "---",
        "",
        "## Threshold Sweep",
        "",
        "| Threshold | FP Rate | Detection Rate | Precision | F1 | FP Count | FN Count |",
        "|-----------|---------|---------------|-----------|-----|----------|----------|",
    ])
    for r in sweep_results:
        # Show every 5th row plus key thresholds
        if int(r["threshold"] * 100) % 5 == 0 or r["threshold"] in (0.5, 0.65, 0.7, 0.75):
            marker = " **" if r["fp_rate"] < 0.05 and r["detection_rate"] > 0.65 else ""
            lines.append(
                f"| {r['threshold']:.3f} | {r['fp_rate'] * 100:.1f}% | "
                f"{r['detection_rate'] * 100:.1f}% | {r['precision'] * 100:.1f}% | "
                f"{r['f1']:.3f} | {r['fp_count']} | {r['fn_count']} |{marker}"
            )
    lines.append("")

    # Recommendations
    lines.extend([
        "---",
        "",
        "## Recommendations",
        "",
    ])

    if best5:
        lines.extend([
            f"1. **Set CLASSIFIER_THRESHOLD = {best5['threshold']}** in "
            f"`sidecar/app/services/deepfake.py`. This achieves {best5['fp_rate'] * 100:.1f}% FP "
            f"with {best5['detection_rate'] * 100:.1f}% detection.",
            "",
        ])
    else:
        lines.extend([
            "1. **Threshold tuning alone is insufficient.** No single threshold achieves < 5% FP "
            "with acceptable detection rate. Proceed with model architecture changes:",
            "   - Add CLIP embedding features (4 features) to the feature vector",
            "   - Apply isotonic calibration post-processor",
            "   - Consider stacking ensemble if CLIP features are insufficient",
            "",
        ])

    # Corpus-specific recommendations
    if fp_by_source:
        major_fp_source = max(fp_by_source.items(), key=lambda x: x[1])
        lines.extend([
            f"2. **Corpus gap:** {major_fp_source[1]} FPs come from '{major_fp_source[0]}' images. "
            f"Add more diverse images from this source to the training set.",
            "",
        ])

    if divergent:
        top_feat = divergent[0]
        lines.extend([
            f"3. **Feature investigation:** The feature '{top_feat['feature']}' has the largest "
            f"effect size (d={top_feat['cohens_d']}) between FP and correctly-classified authentic images. "
            f"This suggests the classifier is over-relying on this feature for authentic images "
            f"with unusual characteristics.",
            "",
        ])

    # FN analysis
    fn_images = analysis.get("fn_images", [])
    if fn_images:
        lines.extend([
            "---",
            "",
            "## False Negative Analysis",
            "",
            "These AI-generated images were missed by the classifier at threshold 0.50:",
            "",
            "| Filename | AI Probability | Codec |",
            "|----------|---------------|-------|",
        ])
        for fn in sorted(fn_images, key=lambda x: x["ai_probability"])[:20]:
            lines.append(f"| {fn['filename']} | {fn['ai_probability']:.3f} | {fn.get('codec_class', '?')} |")
        lines.append("")

    lines.extend([
        "---",
        "",
        f"*Report generated by `scripts/analyse_fp_rate.py` on {datetime.now().strftime('%Y-%m-%d')}*",
        "",
    ])

    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    parser = argparse.ArgumentParser(description="Jura Trace -- False Positive Rate Analysis")
    parser.add_argument(
        "--model", type=str,
        default=str(MODELS_DIR / "deepfake_classifier.joblib"),
        help="Path to classifier model (default: models/deepfake_classifier.joblib)",
    )
    parser.add_argument(
        "--authentic", type=str,
        default=str(AUTHENTIC_DIR),
        help="Path to authentic image directory",
    )
    parser.add_argument(
        "--ai", type=str,
        default=str(AI_DIR),
        help="Path to AI-generated image directory",
    )
    parser.add_argument(
        "--test-split", type=float, default=0.0,
        help="If > 0, hold out this fraction for testing (e.g. 0.2). "
             "When 0, analyse the full corpus (will be overfit, but useful for diagnostics).",
    )
    parser.add_argument("--seed", type=int, default=42, help="Random seed for test split")
    parser.add_argument("--threshold", type=float, default=0.5, help="Default threshold for FP analysis")
    parser.add_argument(
        "--output-dir", type=str, default=str(MODELS_DIR),
        help="Output directory for analysis files",
    )
    parser.add_argument("--verbose", action="store_true", help="Print per-image extraction progress")
    args = parser.parse_args()

    print("=" * 68)
    print("Jura Trace -- False Positive Rate Analysis")
    print("=" * 68)

    # Load model
    model_path = Path(args.model)
    if not model_path.exists():
        print(f"ERROR: model not found: {model_path}")
        sys.exit(1)

    import joblib
    clf = joblib.load(model_path)
    print(f"  Model: {model_path}")

    # Collect images
    authentic_images = collect_images(Path(args.authentic))
    ai_images = collect_images(Path(args.ai))
    print(f"  Authentic images: {len(authentic_images)}")
    print(f"  AI-generated images: {len(ai_images)}")

    if len(authentic_images) < 3:
        print("ERROR: Need at least 3 authentic images.")
        sys.exit(1)

    # Optional test split
    if args.test_split > 0:
        from sklearn.model_selection import train_test_split

        # Combine and split
        all_images = [(img, 0) for img in authentic_images] + [(img, 1) for img in ai_images]
        np.random.seed(args.seed)
        np.random.shuffle(all_images)

        split_idx = int(len(all_images) * (1 - args.test_split))
        train_set = all_images[:split_idx]
        test_set = all_images[split_idx:]

        print(f"  Test split: {args.test_split} ({len(test_set)} test, {len(train_set)} train)")

        # Use only test set for analysis
        authentic_images = [img for img, label in test_set if label == 0]
        ai_images = [img for img, label in test_set if label == 1]
        print(f"  Test set: {len(authentic_images)} authentic, {len(ai_images)} AI")

    # Extract features
    print("\nExtracting features from authentic images...")
    X_auth, y_auth, f_auth, meta_auth = extract_features_batch(
        authentic_images, label=0, verbose=args.verbose
    )

    print(f"\nExtracting features from AI-generated images...")
    X_ai, y_ai, f_ai, meta_ai = extract_features_batch(
        ai_images, label=1, verbose=args.verbose
    )

    if len(X_auth) < 2:
        print("ERROR: Too few valid authentic images after feature extraction.")
        sys.exit(1)

    # Build matrices
    X = np.array(X_auth + X_ai, dtype=np.float64)
    y = np.array(y_auth + y_ai, dtype=np.int32)
    filenames = f_auth + f_ai
    all_metadata = meta_auth + meta_ai

    X = np.nan_to_num(X, nan=0.0, posinf=1e6, neginf=-1e6)

    print(f"\n  Total samples: {len(y)} (authentic={sum(y == 0)}, ai={sum(y == 1)})")

    # Predict
    print("\nRunning classifier predictions...")
    y_proba = clf.predict_proba(X)[:, 1]  # P(AI-generated)

    # AUC
    try:
        from sklearn.metrics import roc_auc_score
        auc = roc_auc_score(y, y_proba)
    except Exception:
        auc = 0.0
    print(f"  AUC-ROC: {auc:.4f}")

    # Threshold sweep
    print("\nRunning threshold sweep...")
    sweep_results = threshold_sweep(y, y_proba)
    optimal = find_optimal_thresholds(sweep_results)

    # Current threshold analysis
    current_fp_rate = next(
        (r["fp_rate"] for r in sweep_results if abs(r["threshold"] - args.threshold) < 0.02),
        None,
    )
    if current_fp_rate is not None:
        print(f"  FP rate at threshold {args.threshold}: {current_fp_rate * 100:.1f}%")

    # Detailed FP analysis
    print("\nAnalysing false positives...")
    analysis = analyse_false_positives(all_metadata, y, y_proba, threshold=args.threshold)
    print(f"  False positives: {analysis['fp_count']}")
    print(f"  False negatives: {analysis['fn_count']}")

    # Print optimal thresholds
    print("\nOptimal thresholds:")
    for label, result in optimal.items():
        if result:
            print(f"  {label}: threshold={result['threshold']}, "
                  f"FP={result['fp_rate'] * 100:.1f}%, "
                  f"detect={result['detection_rate'] * 100:.1f}%")

    # Write outputs
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # 1. Per-sample CSV
    csv_path = output_dir / "per_sample_results.csv"
    with open(csv_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["filename", "true_label", "ai_probability", "pred_at_0.50", "correct"])
        for i, fname in enumerate(filenames):
            true_label = "authentic" if y[i] == 0 else "ai_generated"
            pred = "ai_generated" if y_proba[i] >= 0.5 else "authentic"
            correct = "yes" if (y[i] == 1) == (y_proba[i] >= 0.5) else "no"
            writer.writerow([fname, true_label, f"{y_proba[i]:.4f}", pred, correct])
    print(f"\n  Per-sample results: {csv_path}")

    # 2. Threshold sweep CSV
    sweep_csv_path = output_dir / "threshold_sweep.csv"
    with open(sweep_csv_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(sweep_results[0].keys()))
        writer.writeheader()
        writer.writerows(sweep_results)
    print(f"  Threshold sweep: {sweep_csv_path}")

    # 3. Machine-readable JSON
    json_path = output_dir / "fp_analysis.json"
    json_report = {
        "generated_at": datetime.now().isoformat(),
        "model_path": str(model_path),
        "n_authentic": int(sum(y == 0)),
        "n_ai": int(sum(y == 1)),
        "auc_roc": round(float(auc), 4),
        "current_threshold": args.threshold,
        "fp_rate_at_current": round(float(current_fp_rate), 4) if current_fp_rate is not None else None,
        "optimal_thresholds": {
            k: {
                "threshold": v["threshold"],
                "fp_rate": v["fp_rate"],
                "detection_rate": v["detection_rate"],
            } if v else None
            for k, v in optimal.items()
        },
        "fp_breakdown": {
            "by_source": analysis["fp_by_source"],
            "by_codec": analysis["fp_by_codec"],
        },
        "top_divergent_features": analysis["top_divergent_features"][:10],
    }
    json_path.write_text(json.dumps(json_report, indent=2))
    print(f"  JSON analysis: {json_path}")

    # 4. Markdown report
    report_md = write_markdown_report(
        analysis, sweep_results, optimal,
        n_authentic=int(sum(y == 0)),
        n_ai=int(sum(y == 1)),
        model_path=str(model_path),
        auc=auc,
    )
    report_path = DOCS_DIR / "fp-analysis-report.md"
    report_path.write_text(report_md)
    print(f"  Markdown report: {report_path}")

    # Summary
    print("\n" + "=" * 68)
    print("SUMMARY")
    print("=" * 68)
    print(f"  AUC-ROC: {auc:.4f}")
    print(f"  FP rate at 0.50: {current_fp_rate * 100:.1f}%" if current_fp_rate is not None else "")
    print(f"  FPs: {analysis['fp_count']} / {sum(y == 0)} authentic images")
    print(f"  FNs: {analysis['fn_count']} / {sum(y == 1)} AI images")

    best5 = optimal.get("best_under_5pct_fp")
    if best5:
        print(f"\n  TARGET ACHIEVABLE: threshold {best5['threshold']} gives "
              f"FP={best5['fp_rate'] * 100:.1f}%, detect={best5['detection_rate'] * 100:.1f}%")
    else:
        print(f"\n  TARGET NOT ACHIEVABLE with threshold tuning alone.")
        print(f"  Proceed with CLIP features and/or calibration layer in Sprint 22.")

    print(f"\n  Full report: {report_path}")


if __name__ == "__main__":
    main()
