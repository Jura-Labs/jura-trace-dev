#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Test Corpus Evaluation Script.

Runs all forensic detectors against a directory of test images and
records results in a CSV for false positive / false negative analysis.

Usage:
    python scripts/evaluate_corpus.py /path/to/test_images/ --output results.csv

Directory structure expected:
    test_images/
    ├── real/           # Real camera photos (expected: authentic)
    ├── ai_generated/   # AI-generated images (expected: synthetic or inconclusive)
    └── manipulated/    # Manipulated real photos (expected: suspicious)

Each image filename should follow the convention:
    {source}_{scene}_{codec}_{number}.{ext}
    e.g. iphone16_indoor_jpeg_001.jpg
"""

import argparse
import csv
import json
import os
import sys
import time
from pathlib import Path

# Add sidecar root to path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from app.services.deepfake import perform_deepfake_detection
from app.services.ela import perform_ela
from app.services.noise_analysis import perform_noise_analysis
from app.services.npr import perform_npr_analysis
from app.services.jpeg_ghost import perform_jpeg_ghost_detection


# MIME type mapping
MIME_MAP = {
    ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
    ".png": "image/png", ".webp": "image/webp",
    ".avif": "image/avif", ".heic": "image/heic",
    ".tiff": "image/tiff", ".tif": "image/tiff",
    ".bmp": "image/bmp", ".gif": "image/gif",
}

EXPECTED_VERDICTS = {
    "real": "authentic",
    "ai_generated": "synthetic",
    "manipulated": "suspicious",
}


def get_mime_type(path: Path) -> str:
    return MIME_MAP.get(path.suffix.lower(), "image/jpeg")


def evaluate_image(path: Path, category: str) -> dict:
    """Run all detectors on a single image and return results dict."""
    with open(path, "rb") as f:
        data = f.read()

    mime = get_mime_type(path)
    has_exif = category == "real"  # Assume real photos have EXIF

    result = {
        "filename": path.name,
        "category": category,
        "file_size": len(data),
        "mime_type": mime,
        "expected_verdict": EXPECTED_VERDICTS.get(category, "unknown"),
    }

    # Deepfake ensemble
    try:
        df = perform_deepfake_detection(data, mime_type=mime, has_camera_exif=has_exif)
        result["deepfake_score"] = df.score
        result["deepfake_verdict"] = df.verdict_level
        result["deepfake_confidence"] = df.confidence
        result["deepfake_suspicious"] = df.suspicious
        result["signals_triggered"] = sum(1 for s in df.signals if s.triggered)
        result["signals_total"] = len(df.signals)
        triggered_names = [s.name for s in df.signals if s.triggered]
        result["triggered_signals"] = "; ".join(triggered_names)
        # Watermark detection
        wm_detected = [w for w in df.watermarks if w.detected]
        result["watermark_detected"] = len(wm_detected) > 0
        result["watermark_types"] = "; ".join(w.type for w in wm_detected)
    except Exception as e:
        result["deepfake_score"] = None
        result["deepfake_error"] = str(e)

    # ELA
    try:
        ela = perform_ela(data)
        result["ela_score"] = ela.score
        result["ela_suspicious"] = ela.suspicious
    except Exception:
        result["ela_score"] = None

    # Noise
    try:
        noise = perform_noise_analysis(data)
        result["noise_score"] = noise.score
        result["noise_suspicious"] = noise.suspicious
    except Exception:
        result["noise_score"] = None

    # NPR
    try:
        npr = perform_npr_analysis(data)
        result["npr_score"] = npr.score
        result["npr_suspicious"] = npr.suspicious
    except Exception:
        result["npr_score"] = None

    # JPEG Ghost
    try:
        jg = perform_jpeg_ghost_detection(data)
        result["jpeg_ghost_score"] = jg.score
        result["jpeg_ghost_quality"] = jg.ghost_quality
    except Exception:
        result["jpeg_ghost_score"] = None

    # CLIP (if available)
    try:
        from app.services.clip_detector import perform_clip_detection
        clip = perform_clip_detection(data)
        result["clip_score"] = clip.score
        result["clip_verdict"] = clip.verdict_level
        result["clip_available"] = clip.model_available
    except Exception:
        result["clip_score"] = None

    # Determine if result is correct
    expected = result["expected_verdict"]
    actual = result.get("deepfake_verdict", "unknown")
    if expected == "authentic":
        result["correct"] = actual == "authentic"
        result["false_positive"] = actual == "synthetic"
    elif expected == "synthetic":
        result["correct"] = actual in ("synthetic", "inconclusive")
        result["false_negative"] = actual == "authentic"
    elif expected == "suspicious":
        result["correct"] = actual != "authentic"
    else:
        result["correct"] = None

    return result


def main():
    parser = argparse.ArgumentParser(description="Evaluate test corpus against detection pipeline")
    parser.add_argument("corpus_dir", help="Directory containing real/, ai_generated/, manipulated/ subdirs")
    parser.add_argument("--output", "-o", default="corpus_results.csv", help="Output CSV path")
    args = parser.parse_args()

    corpus = Path(args.corpus_dir)
    if not corpus.is_dir():
        print(f"Error: {corpus} is not a directory")
        sys.exit(1)

    results = []
    image_exts = {".jpg", ".jpeg", ".png", ".webp", ".avif", ".heic", ".tiff", ".tif", ".bmp"}

    for category in ("real", "ai_generated", "manipulated"):
        cat_dir = corpus / category
        if not cat_dir.is_dir():
            print(f"Warning: {cat_dir} not found, skipping")
            continue

        images = sorted(p for p in cat_dir.iterdir() if p.suffix.lower() in image_exts)
        print(f"\n{category}: {len(images)} images")

        for i, img_path in enumerate(images):
            start = time.time()
            result = evaluate_image(img_path, category)
            elapsed = time.time() - start
            result["processing_time_s"] = round(elapsed, 2)

            verdict = result.get("deepfake_verdict", "?")
            score = result.get("deepfake_score", 0) or 0
            correct = result.get("correct", "?")
            mark = "OK" if correct else "FAIL" if correct is False else "?"

            print(f"  [{mark:4s}] {img_path.name:50s} score={score:.4f} verdict={verdict:13s} ({elapsed:.1f}s)")
            results.append(result)

    # Write CSV
    if results:
        fieldnames = list(results[0].keys())
        # Ensure all rows have all fields
        all_keys = set()
        for r in results:
            all_keys.update(r.keys())
        fieldnames = sorted(all_keys)

        with open(args.output, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction="ignore")
            writer.writeheader()
            writer.writerows(results)

        print(f"\nResults written to {args.output}")

    # Summary
    print("\n=== SUMMARY ===")
    real = [r for r in results if r["category"] == "real"]
    ai = [r for r in results if r["category"] == "ai_generated"]
    manip = [r for r in results if r["category"] == "manipulated"]

    if real:
        fp = sum(1 for r in real if r.get("false_positive"))
        inc = sum(1 for r in real if r.get("deepfake_verdict") == "inconclusive")
        print(f"Real photos:      {len(real)} total, {fp} false positives ({100*fp/len(real):.1f}% FPR), {inc} inconclusive")

    if ai:
        fn = sum(1 for r in ai if r.get("false_negative"))
        det = sum(1 for r in ai if r.get("deepfake_verdict") == "synthetic")
        inc = sum(1 for r in ai if r.get("deepfake_verdict") == "inconclusive")
        print(f"AI-generated:     {len(ai)} total, {fn} false negatives ({100*fn/len(ai):.1f}% FNR), {det} detected, {inc} inconclusive")

    if manip:
        correct = sum(1 for r in manip if r.get("correct"))
        print(f"Manipulated:      {len(manip)} total, {correct} correctly flagged ({100*correct/len(manip):.1f}%)")

    # Watermark stats
    wm = [r for r in ai if r.get("watermark_detected")]
    sd_images = [r for r in ai if "sd" in r["filename"].lower() or "flux" in r["filename"].lower() or "sdxl" in r["filename"].lower()]
    if sd_images:
        wm_sd = [r for r in sd_images if r.get("watermark_detected")]
        print(f"SD/SDXL watermark: {len(wm_sd)}/{len(sd_images)} detected ({100*len(wm_sd)/len(sd_images):.0f}%)")


if __name__ == "__main__":
    main()
