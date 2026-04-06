#!/usr/bin/env python3
"""
Jura Trace — Social Media Compression Resilience Test Suite

Measures detector accuracy across simulated social media compression chains.
Takes a set of test images, applies compression profiles mimicking WhatsApp,
Telegram, Facebook, Twitter/X, and double-screenshot, then runs the forensic
pipeline on each variant to measure accuracy degradation.

TRIED Pillar 1: "tools must handle compressed, re-encoded files from social platforms"

Usage:
    python scripts/compression_test_suite.py
    python scripts/compression_test_suite.py --input corpus/test_set --output models/compression_resilience.csv
    python scripts/compression_test_suite.py --sidecar-url http://127.0.0.1:8200

Requires: PIL/Pillow, requests (for sidecar API calls)
"""

import argparse
import csv
import json
import os
import sys
import tempfile
import time
from datetime import datetime
from io import BytesIO
from pathlib import Path

from PIL import Image

# Compression profiles simulating social media platform behaviour
PROFILES = {
    "original": {
        "description": "Original file — no modification",
        "transform": None,
    },
    "whatsapp": {
        "description": "WhatsApp (JPEG Q75, max 1600px, strip EXIF)",
        "jpeg_quality": 75,
        "max_dimension": 1600,
        "strip_exif": True,
    },
    "telegram": {
        "description": "Telegram (JPEG Q80, max 2560px)",
        "jpeg_quality": 80,
        "max_dimension": 2560,
        "strip_exif": False,
    },
    "facebook": {
        "description": "Facebook (JPEG Q71, max 2048px, strip EXIF)",
        "jpeg_quality": 71,
        "max_dimension": 2048,
        "strip_exif": True,
    },
    "twitter": {
        "description": "Twitter/X (JPEG Q85, strip EXIF)",
        "jpeg_quality": 85,
        "max_dimension": 4096,
        "strip_exif": True,
    },
    "screenshot_2x": {
        "description": "Double screenshot (render + re-save as PNG, then JPEG Q90)",
        "jpeg_quality": 90,
        "max_dimension": None,
        "strip_exif": True,
        "double_save": True,
    },
}

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}


def collect_images(directory: str, limit: int = 50) -> list[Path]:
    """Collect image files from a directory."""
    d = Path(directory)
    if not d.exists():
        print(f"  WARNING: directory not found: {d}")
        return []
    images = sorted(
        f for f in d.rglob("*")
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
    )
    return images[:limit]


def apply_compression(img_path: Path, profile_name: str, profile: dict) -> bytes | None:
    """Apply a compression profile to an image, return JPEG bytes."""
    if profile.get("transform") is None and profile_name == "original":
        return img_path.read_bytes()

    try:
        img = Image.open(img_path).convert("RGB")
    except Exception as e:
        print(f"    Cannot open {img_path.name}: {e}")
        return None

    # Resize if max_dimension specified
    max_dim = profile.get("max_dimension")
    if max_dim:
        w, h = img.size
        if max(w, h) > max_dim:
            ratio = max_dim / max(w, h)
            new_w, new_h = int(w * ratio), int(h * ratio)
            img = img.resize((new_w, new_h), Image.LANCZOS)

    # Double-save simulation (screenshot chain)
    if profile.get("double_save"):
        buf1 = BytesIO()
        img.save(buf1, format="PNG")
        buf1.seek(0)
        img = Image.open(buf1).convert("RGB")

    # Save as JPEG with specified quality
    quality = profile.get("jpeg_quality", 85)
    buf = BytesIO()
    img.save(buf, format="JPEG", quality=quality)
    return buf.getvalue()


def call_sidecar_detector(
    endpoint: str,
    image_bytes: bytes,
    sidecar_url: str,
    timeout: int = 30,
) -> dict | None:
    """Call a sidecar forensic endpoint with image bytes."""
    import requests

    try:
        files = {"file": ("test.jpg", image_bytes, "image/jpeg")}
        resp = requests.post(
            f"{sidecar_url}{endpoint}",
            files=files,
            timeout=timeout,
        )
        if resp.status_code == 200:
            return resp.json()
        return None
    except Exception:
        return None


def analyse_image(
    image_bytes: bytes,
    sidecar_url: str,
    is_ai: bool,
) -> dict:
    """Run core detectors on image bytes and return scores."""
    results = {}

    # ELA
    ela = call_sidecar_detector("/forensics/ela", image_bytes, sidecar_url)
    if ela:
        results["ela_score"] = ela.get("overall_score", 0)

    # Noise
    noise = call_sidecar_detector("/forensics/noise", image_bytes, sidecar_url)
    if noise:
        results["noise_score"] = noise.get("overall_score", 0)

    # Deepfake
    deepfake = call_sidecar_detector("/forensics/deepfake", image_bytes, sidecar_url)
    if deepfake:
        results["deepfake_score"] = deepfake.get("overall_score", 0)
        results["deepfake_is_ai"] = deepfake.get("is_ai_generated", False)

    # CLIP (if available)
    clip = call_sidecar_detector("/forensics/clip-detect", image_bytes, sidecar_url)
    if clip:
        results["clip_score"] = clip.get("ai_probability", 0)
        results["clip_is_ai"] = clip.get("is_ai_generated", False)

    results["ground_truth"] = "ai" if is_ai else "authentic"
    return results


def compute_accuracy(rows: list[dict], detector: str, score_key: str, threshold: float) -> dict:
    """Compute accuracy metrics for a detector at a given threshold."""
    tp = fp = tn = fn = 0
    valid = 0

    for row in rows:
        score = row.get(score_key)
        if score is None:
            continue
        valid += 1
        is_ai = row["ground_truth"] == "ai"
        flagged = score > threshold

        if is_ai and flagged:
            tp += 1
        elif is_ai and not flagged:
            fn += 1
        elif not is_ai and flagged:
            fp += 1
        else:
            tn += 1

    total = tp + fp + tn + fn
    return {
        "detector": detector,
        "valid_samples": valid,
        "accuracy": (tp + tn) / total if total > 0 else 0,
        "fp_rate": fp / (fp + tn) if (fp + tn) > 0 else 0,
        "fn_rate": fn / (fn + tp) if (fn + tp) > 0 else 0,
        "tp": tp,
        "fp": fp,
        "tn": tn,
        "fn": fn,
    }


def main():
    parser = argparse.ArgumentParser(
        description="Social media compression resilience test suite"
    )
    parser.add_argument(
        "--authentic",
        type=str,
        default="/Volumes/Samsung USB/Training Data/corpus/training/authentic",
        help="Path to authentic test images",
    )
    parser.add_argument(
        "--ai",
        type=str,
        default="/Volumes/Samsung USB/Training Data/corpus/training/ai_generated",
        help="Path to AI-generated test images",
    )
    parser.add_argument(
        "--count",
        type=int,
        default=25,
        help="Number of images per class (default: 25, total = 50)",
    )
    parser.add_argument(
        "--output",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "..", "models", "compression_resilience.csv"),
        help="Output CSV file path",
    )
    parser.add_argument(
        "--sidecar-url",
        type=str,
        default="http://127.0.0.1:8200",
        help="Sidecar URL (default: http://127.0.0.1:8200)",
    )
    args = parser.parse_args()

    print("=" * 60)
    print("  Jura Trace — Compression Resilience Test Suite")
    print("  TRIED Pillar 1: real-world content handling")
    print("=" * 60)

    # Collect test images
    authentic_images = collect_images(args.authentic, args.count)
    ai_images = collect_images(args.ai, args.count)

    print(f"\n  Authentic images: {len(authentic_images)}")
    print(f"  AI-generated images: {len(ai_images)}")

    if len(authentic_images) < 5 or len(ai_images) < 5:
        print("\nERROR: Need at least 5 images per class.")
        sys.exit(1)

    # Check sidecar
    import requests
    try:
        resp = requests.get(f"{args.sidecar_url}/health", timeout=5)
        if resp.status_code != 200:
            raise Exception("unhealthy")
        print(f"  Sidecar: online at {args.sidecar_url}")
    except Exception:
        print(f"\nERROR: Sidecar not available at {args.sidecar_url}")
        print("  Start with: cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200")
        sys.exit(1)

    all_images = [(p, False) for p in authentic_images] + [(p, True) for p in ai_images]
    total_analyses = len(all_images) * len(PROFILES)
    print(f"\n  Total analyses: {total_analyses} ({len(all_images)} images × {len(PROFILES)} profiles)")
    print(f"  Estimated time: {total_analyses * 3}–{total_analyses * 8} seconds\n")

    # Run analysis
    all_rows = []
    completed = 0

    for profile_name, profile in PROFILES.items():
        print(f"\n  Profile: {profile_name} — {profile.get('description', '')}")
        profile_rows = []

        for img_path, is_ai in all_images:
            # Apply compression
            if profile_name == "original":
                image_bytes = img_path.read_bytes()
            else:
                image_bytes = apply_compression(img_path, profile_name, profile)
                if image_bytes is None:
                    continue

            # Analyse
            scores = analyse_image(image_bytes, args.sidecar_url, is_ai)
            scores["profile"] = profile_name
            scores["filename"] = img_path.name
            scores["file_size_bytes"] = len(image_bytes)
            profile_rows.append(scores)

            completed += 1
            if completed % 25 == 0:
                print(f"    {completed}/{total_analyses} analyses completed...")

        all_rows.extend(profile_rows)

    # Write CSV
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    if all_rows:
        fieldnames = sorted(set().union(*(r.keys() for r in all_rows)))
        with open(output_path, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(all_rows)
        print(f"\n  Results written to: {output_path}")

    # Summary report
    print("\n" + "=" * 60)
    print("  COMPRESSION RESILIENCE SUMMARY")
    print("=" * 60)

    detectors = [
        ("ELA", "ela_score", 0.4),
        ("Noise", "noise_score", 0.4),
        ("Deepfake (GBM)", "deepfake_score", 0.5),
        ("CLIP", "clip_score", 0.5),
    ]

    for profile_name in PROFILES:
        print(f"\n  --- {profile_name} ---")
        profile_rows = [r for r in all_rows if r["profile"] == profile_name]

        for det_name, score_key, threshold in detectors:
            metrics = compute_accuracy(profile_rows, det_name, score_key, threshold)
            if metrics["valid_samples"] > 0:
                print(
                    f"    {det_name:20s}  acc={metrics['accuracy']:.1%}  "
                    f"FP={metrics['fp_rate']:.1%}  FN={metrics['fn_rate']:.1%}  "
                    f"(n={metrics['valid_samples']})"
                )
            else:
                print(f"    {det_name:20s}  — no data")

    # Identify degraded detectors (accuracy drops > 10pp from original)
    print("\n  --- DEGRADATION ALERTS ---")
    original_rows = [r for r in all_rows if r["profile"] == "original"]
    alerts = []

    for det_name, score_key, threshold in detectors:
        orig_metrics = compute_accuracy(original_rows, det_name, score_key, threshold)
        if orig_metrics["valid_samples"] == 0:
            continue

        for profile_name in PROFILES:
            if profile_name == "original":
                continue
            profile_rows = [r for r in all_rows if r["profile"] == profile_name]
            prof_metrics = compute_accuracy(profile_rows, det_name, score_key, threshold)
            if prof_metrics["valid_samples"] == 0:
                continue

            acc_drop = orig_metrics["accuracy"] - prof_metrics["accuracy"]
            if acc_drop > 0.10:
                alert = f"    {det_name} drops {acc_drop:.0%} under {profile_name} compression"
                alerts.append(alert)
                print(alert)

    if not alerts:
        print("    No detectors dropped >10 percentage points under any profile.")

    print(f"\n  Completed at: {datetime.now().isoformat()}")
    print("  Done.")


if __name__ == "__main__":
    main()
