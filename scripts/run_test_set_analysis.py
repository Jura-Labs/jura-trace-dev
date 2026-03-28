#!/usr/bin/env python3
"""
Jura Trace -- Test Set Classifier Analysis

Runs the GBM classifier against the test set and produces a detailed
analysis report broken down by content category.

This wraps the feature extraction from analyse_fp_rate.py but adds
support for the test set's nested directory structure and multi-category
labelling (authentic, ai_generated, edge_case, mixed).

Usage:
    python scripts/run_test_set_analysis.py
"""

import csv
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

import numpy as np

BASE_DIR = Path(__file__).resolve().parent.parent
TEST_SET_DIR = BASE_DIR / "corpus" / "test_set"
MANIFEST_PATH = TEST_SET_DIR / "manifest.json"
MODELS_DIR = BASE_DIR / "models"
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".tiff", ".tif"}


def collect_images_recursive(directory: Path) -> list[Path]:
    """Collect image files recursively from a directory tree."""
    images = []
    if not directory.exists():
        return images
    for root, dirs, files in os.walk(directory):
        for f in sorted(files):
            if Path(f).suffix.lower() in IMAGE_EXTENSIONS:
                images.append(Path(root) / f)
    return sorted(images)


def detect_mime(path: str, data: bytes) -> str:
    if data[:4] == b"\x89PNG":
        return "image/png"
    if data[:2] == b"\xff\xd8":
        return "image/jpeg"
    if data[:4] == b"RIFF" and len(data) > 12 and data[8:12] == b"WEBP":
        return "image/webp"
    ext = Path(path).suffix.lower()
    return {".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
            ".webp": "image/webp"}.get(ext, "image/jpeg")


def extract_features(images: list[Path], verbose: bool = True):
    """Extract feature vectors from images."""
    from app.services.deepfake import (
        FEATURE_NAMES,
        extract_feature_vector,
        extract_features_for_training,
    )

    results = []
    for i, img_path in enumerate(images):
        if verbose and (i % 10 == 0 or i == len(images) - 1):
            print(f"  [{i + 1}/{len(images)}] {img_path.name}...", flush=True)
        try:
            data = img_path.read_bytes()
            mime = detect_mime(str(img_path), data)
            features, codec_class = extract_features_for_training(data, mime_type=mime)
            vec = extract_feature_vector(features)
            vec_clean = [0.0 if np.isnan(v) else v for v in vec]
            results.append({
                "path": str(img_path),
                "filename": img_path.name,
                "relative": str(img_path.relative_to(TEST_SET_DIR)),
                "vector": vec_clean,
                "codec_class": codec_class,
                "size_bytes": len(data),
                "mime": mime,
            })
        except Exception as e:
            if verbose:
                print(f"    FAILED: {img_path.name} ({e})")
            results.append({
                "path": str(img_path),
                "filename": img_path.name,
                "relative": str(img_path.relative_to(TEST_SET_DIR)),
                "vector": None,
                "error": str(e),
            })
    return results


def classify(results: list[dict], clf) -> list[dict]:
    """Run the classifier on extracted features."""
    for r in results:
        if r.get("vector") is None:
            r["ai_probability"] = None
            continue
        X = np.array([r["vector"]], dtype=np.float64)
        X = np.nan_to_num(X, nan=0.0, posinf=1e6, neginf=-1e6)
        proba = clf.predict_proba(X)[0, 1]
        r["ai_probability"] = float(proba)
    return results


def load_manifest() -> dict:
    """Load the manifest and build a lookup by filename."""
    manifest = json.loads(MANIFEST_PATH.read_text())
    lookup = {}
    for entry in manifest["images"]:
        lookup[entry["filename"]] = entry
    return manifest, lookup


def main():
    print("=" * 68)
    print("Jura Trace -- Test Set Classifier Analysis")
    print("=" * 68)

    # Load classifier
    model_path = MODELS_DIR / "deepfake_classifier.joblib"
    if not model_path.exists():
        print(f"ERROR: Classifier not found at {model_path}")
        sys.exit(1)

    import joblib
    clf = joblib.load(model_path)
    print(f"  Model: {model_path}")

    # Load manifest
    manifest, manifest_lookup = load_manifest()
    print(f"  Manifest: {len(manifest['images'])} entries")

    # Collect ALL images from test set
    all_images = collect_images_recursive(TEST_SET_DIR)
    # Exclude manifest.json from count
    all_images = [p for p in all_images if p.suffix.lower() in IMAGE_EXTENSIONS]
    print(f"  Images on disk: {len(all_images)}")

    # Extract features
    print(f"\nExtracting features from {len(all_images)} images...")
    t0 = time.time()
    results = extract_features(all_images)
    elapsed = time.time() - t0
    print(f"  Feature extraction: {elapsed:.1f}s ({len(results)} images)")

    valid = [r for r in results if r.get("vector") is not None]
    failed = [r for r in results if r.get("vector") is None]
    print(f"  Valid: {len(valid)}, Failed: {len(failed)}")

    # Classify
    print("\nRunning classifier...")
    results = classify(results, clf)

    # Enrich with manifest labels
    for r in results:
        meta = manifest_lookup.get(r["filename"], {})
        r["expected_verdict"] = meta.get("expected_verdict", "unknown")
        r["category"] = meta.get("category", "unknown")
        r["notes"] = meta.get("notes", "")

    # --- Analysis ---
    print("\n" + "=" * 68)
    print("RESULTS")
    print("=" * 68)

    # Group by category
    categories = {}
    for r in results:
        cat = r["category"]
        if cat not in categories:
            categories[cat] = []
        categories[cat].append(r)

    # Define what counts as "flagged as AI" at threshold 0.5
    threshold = 0.5

    # Overall stats
    authentic_cats = {"phone", "social_media", "screenshot", "scan", "stock", "edge_case"}
    ai_cats = {"ai_generated"}
    mixed_cats = {"mixed"}

    all_authentic = [r for r in results if r["category"] in authentic_cats and r.get("ai_probability") is not None]
    all_ai = [r for r in results if r["category"] in ai_cats and r.get("ai_probability") is not None]
    all_mixed = [r for r in results if r["category"] in mixed_cats and r.get("ai_probability") is not None]

    auth_flagged = [r for r in all_authentic if r["ai_probability"] >= threshold]
    ai_detected = [r for r in all_ai if r["ai_probability"] >= threshold]
    mixed_flagged = [r for r in all_mixed if r["ai_probability"] >= threshold]

    fp_rate = len(auth_flagged) / len(all_authentic) * 100 if all_authentic else 0
    detect_rate = len(ai_detected) / len(all_ai) * 100 if all_ai else 0

    print(f"\n  OVERALL (threshold = {threshold}):")
    print(f"    Authentic images: {len(all_authentic)}")
    print(f"    AI-generated images: {len(all_ai)}")
    print(f"    Mixed images: {len(all_mixed)}")
    print(f"    False positives (auth flagged as AI): {len(auth_flagged)}/{len(all_authentic)} = {fp_rate:.1f}%")
    print(f"    Detection rate (AI correctly flagged): {len(ai_detected)}/{len(all_ai)} = {detect_rate:.1f}%")
    print(f"    Mixed flagged: {len(mixed_flagged)}/{len(all_mixed)}")

    # Per-category breakdown
    print(f"\n  PER-CATEGORY BREAKDOWN:")
    print(f"  {'Category':<20} {'Count':>6} {'Flagged':>8} {'Rate':>8} {'Avg P(AI)':>10}")
    print(f"  {'-'*20} {'-'*6} {'-'*8} {'-'*8} {'-'*10}")

    category_stats = {}
    for cat in sorted(categories.keys()):
        items = [r for r in categories[cat] if r.get("ai_probability") is not None]
        flagged = [r for r in items if r["ai_probability"] >= threshold]
        probas = [r["ai_probability"] for r in items]
        avg_p = np.mean(probas) if probas else 0

        rate = len(flagged) / len(items) * 100 if items else 0
        print(f"  {cat:<20} {len(items):>6} {len(flagged):>8} {rate:>7.1f}% {avg_p:>9.3f}")

        category_stats[cat] = {
            "count": len(items),
            "flagged": len(flagged),
            "rate_pct": round(rate, 1),
            "avg_ai_probability": round(float(avg_p), 4),
            "flagged_files": [r["filename"] for r in flagged],
        }

    # Worst false positives (authentic with highest AI probability)
    print(f"\n  TOP FALSE POSITIVES (authentic images scored highest for AI):")
    auth_sorted = sorted(all_authentic, key=lambda r: r["ai_probability"], reverse=True)
    print(f"  {'Filename':<45} {'P(AI)':>8} {'Category':<15} {'Codec':<8}")
    print(f"  {'-'*45} {'-'*8} {'-'*15} {'-'*8}")
    for r in auth_sorted[:20]:
        print(f"  {r['filename']:<45} {r['ai_probability']:>7.3f} {r['category']:<15} {r.get('codec_class', '?'):<8}")

    # Worst false negatives (AI images with lowest AI probability)
    if all_ai:
        print(f"\n  WORST MISSES (AI images scored lowest):")
        ai_sorted = sorted(all_ai, key=lambda r: r["ai_probability"])
        print(f"  {'Filename':<45} {'P(AI)':>8} {'Codec':<8}")
        print(f"  {'-'*45} {'-'*8} {'-'*8}")
        for r in ai_sorted[:15]:
            print(f"  {r['filename']:<45} {r['ai_probability']:>7.3f} {r.get('codec_class', '?'):<8}")

    # Threshold sweep for authentic vs AI only
    print(f"\n  THRESHOLD SWEEP (authentic vs AI-generated only):")
    print(f"  {'Threshold':>10} {'FP Rate':>10} {'Detect':>10} {'FP Count':>10} {'Missed':>10}")
    print(f"  {'-'*10} {'-'*10} {'-'*10} {'-'*10} {'-'*10}")

    sweep_results = []
    for t in [0.30, 0.35, 0.40, 0.45, 0.50, 0.55, 0.60, 0.65, 0.70, 0.75, 0.80]:
        fp_count = sum(1 for r in all_authentic if r["ai_probability"] >= t)
        tp_count = sum(1 for r in all_ai if r["ai_probability"] >= t)
        fp_r = fp_count / len(all_authentic) * 100 if all_authentic else 0
        det_r = tp_count / len(all_ai) * 100 if all_ai else 0
        missed = len(all_ai) - tp_count
        print(f"  {t:>10.2f} {fp_r:>9.1f}% {det_r:>9.1f}% {fp_count:>10} {missed:>10}")
        sweep_results.append({
            "threshold": t,
            "fp_rate_pct": round(fp_r, 1),
            "detection_rate_pct": round(det_r, 1),
            "fp_count": fp_count,
            "fn_count": missed,
        })

    # Write outputs
    output_dir = TEST_SET_DIR

    # Per-sample CSV
    csv_path = output_dir / "classifier_results.csv"
    with open(csv_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow([
            "filename", "relative_path", "category", "expected_verdict",
            "ai_probability", "flagged_at_0.5", "correct", "codec_class", "size_kb",
        ])
        for r in sorted(results, key=lambda x: x.get("ai_probability", 0) or 0, reverse=True):
            prob = r.get("ai_probability")
            if prob is None:
                continue
            flagged = "yes" if prob >= 0.5 else "no"
            expected = r["expected_verdict"]
            if expected == "authentic":
                correct = "yes" if prob < 0.5 else "no"
            elif expected == "ai_generated":
                correct = "yes" if prob >= 0.5 else "no"
            else:
                correct = "n/a"
            writer.writerow([
                r["filename"],
                r.get("relative", ""),
                r["category"],
                expected,
                f"{prob:.4f}",
                flagged,
                correct,
                r.get("codec_class", ""),
                r.get("size_bytes", 0) // 1024,
            ])
    print(f"\n  Per-sample CSV: {csv_path}")

    # JSON summary
    json_path = output_dir / "analysis_results.json"
    summary = {
        "generated_at": datetime.now().isoformat(),
        "model": str(model_path),
        "total_images": len(results),
        "valid_images": len(valid),
        "failed_images": len(failed),
        "threshold": threshold,
        "overall": {
            "authentic_count": len(all_authentic),
            "ai_count": len(all_ai),
            "mixed_count": len(all_mixed),
            "fp_count": len(auth_flagged),
            "fp_rate_pct": round(fp_rate, 1),
            "detection_count": len(ai_detected),
            "detection_rate_pct": round(detect_rate, 1),
        },
        "per_category": category_stats,
        "threshold_sweep": sweep_results,
        "top_false_positives": [
            {"filename": r["filename"], "ai_probability": round(r["ai_probability"], 4),
             "category": r["category"]}
            for r in auth_sorted[:20]
        ],
    }
    json_path.write_text(json.dumps(summary, indent=2))
    print(f"  JSON summary: {json_path}")

    print("\n" + "=" * 68)
    print("DONE")
    print("=" * 68)


if __name__ == "__main__":
    main()
