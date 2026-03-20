#!/usr/bin/env python3
"""
Jura Trace — Detector Calibration Pipeline

Runs all forensic detectors against a corpus of known-authentic images
and produces threshold recommendations based on the score distributions.

Usage:
    python scripts/calibrate.py --corpus corpus/authentic
    python scripts/calibrate.py --corpus corpus/authentic --sidecar http://127.0.0.1:8200
"""

import argparse
import csv
import json
import os
import statistics
import sys
import time
from datetime import datetime
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError
from urllib.parse import urljoin


DETECTORS = [
    ("ela", "/forensics/ela"),
    ("noise", "/forensics/noise"),
    ("copy_move", "/forensics/copy-move"),
    ("deepfake", "/forensics/deepfake"),
    ("jpeg_ghost", "/forensics/jpeg-ghost"),
    ("npr", "/forensics/npr"),
    ("chromatic_aberration", "/forensics/chromatic-aberration"),
    ("segmented_ela", "/forensics/segmented-ela"),
    ("shadow_consistency", "/forensics/shadow-consistency"),
    ("colour_temperature", "/forensics/colour-temperature"),
    ("splice_boundary", "/forensics/splice-boundary"),
]


def check_sidecar(base_url: str) -> bool:
    """Check if the sidecar is running."""
    try:
        req = Request(f"{base_url}/health", headers={"Accept": "application/json"})
        with urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read())
            return data.get("status") == "ok"
    except Exception:
        return False


def run_detector(base_url: str, endpoint: str, image_path: str) -> dict | None:
    """Run a single detector against an image file."""
    import http.client
    import mimetypes

    url = f"{base_url}{endpoint}"
    boundary = "----JuraTraceCalibration"
    filename = os.path.basename(image_path)
    content_type = mimetypes.guess_type(image_path)[0] or "image/jpeg"

    with open(image_path, "rb") as f:
        file_data = f.read()

    body = (
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'
        f"Content-Type: {content_type}\r\n\r\n"
    ).encode() + file_data + f"\r\n--{boundary}--\r\n".encode()

    req = Request(
        url,
        data=body,
        headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Accept": "application/json",
        },
        method="POST",
    )

    try:
        with urlopen(req, timeout=60) as resp:
            result = json.loads(resp.read())
            return result
    except Exception as e:
        return {"error": str(e)}


def analyse_scores(detector_name: str, scores: list[float]) -> dict:
    """Compute statistics for a detector's score distribution."""
    if not scores:
        return {"detector": detector_name, "count": 0}

    sorted_scores = sorted(scores)
    n = len(sorted_scores)

    return {
        "detector": detector_name,
        "count": n,
        "min": round(min(scores), 4),
        "max": round(max(scores), 4),
        "mean": round(statistics.mean(scores), 4),
        "median": round(statistics.median(scores), 4),
        "stdev": round(statistics.stdev(scores), 4) if n > 1 else 0,
        "p90": round(sorted_scores[int(n * 0.90)], 4),
        "p95": round(sorted_scores[int(n * 0.95)], 4),
        "p99": round(sorted_scores[min(int(n * 0.99), n - 1)], 4),
        "false_positive_count": sum(1 for s in scores if s > 0.5),
        "false_positive_rate": round(sum(1 for s in scores if s > 0.5) / n * 100, 1),
    }


def main():
    parser = argparse.ArgumentParser(description="Calibrate detectors against known-authentic corpus")
    parser.add_argument("--corpus", type=str, required=True, help="Path to corpus directory")
    parser.add_argument("--sidecar", type=str, default="http://127.0.0.1:8200", help="Sidecar URL")
    parser.add_argument("--output", type=str, default=None, help="Output directory (default: <corpus>/calibration)")
    parser.add_argument("--max", type=int, default=0, help="Max images to process (0=all)")
    parser.add_argument("--detectors", type=str, default="all", help="Comma-separated detector names or 'all'")
    args = parser.parse_args()

    corpus_dir = Path(args.corpus)
    if not corpus_dir.exists():
        print(f"Error: corpus directory not found: {corpus_dir}")
        sys.exit(1)

    output_dir = Path(args.output) if args.output else corpus_dir / "calibration"
    output_dir.mkdir(parents=True, exist_ok=True)

    # Check sidecar
    print("Jura Trace — Detector Calibration Pipeline")
    print(f"  Corpus: {corpus_dir}")
    print(f"  Sidecar: {args.sidecar}")
    print(f"  Output: {output_dir}")

    if not check_sidecar(args.sidecar):
        print(f"\nError: sidecar not responding at {args.sidecar}")
        print("Start it with: cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200")
        sys.exit(1)
    print("  Sidecar: connected")

    # Find images
    image_extensions = {".jpg", ".jpeg", ".png", ".tiff", ".webp"}
    images = sorted([
        f for f in corpus_dir.iterdir()
        if f.suffix.lower() in image_extensions
    ])

    if args.max > 0:
        images = images[:args.max]

    print(f"  Images: {len(images)}")

    # Select detectors
    if args.detectors == "all":
        active_detectors = DETECTORS
    else:
        names = set(args.detectors.split(","))
        active_detectors = [(n, e) for n, e in DETECTORS if n in names]

    print(f"  Detectors: {', '.join(n for n, _ in active_detectors)}")
    print()

    # Run calibration
    all_results = []  # list of dicts: {filename, detector, score, suspicious, ...}
    scores_by_detector = {name: [] for name, _ in active_detectors}

    for i, image_path in enumerate(images):
        print(f"  [{i+1}/{len(images)}] {image_path.name}...", end=" ", flush=True)
        row = {"filename": image_path.name, "label": "authentic"}

        for det_name, endpoint in active_detectors:
            result = run_detector(args.sidecar, endpoint, str(image_path))

            if result and "error" not in result:
                score = result.get("score", 0)
                suspicious = result.get("suspicious", False)
                row[f"{det_name}_score"] = score
                row[f"{det_name}_suspicious"] = suspicious
                scores_by_detector[det_name].append(score)
            else:
                error = result.get("error", "unknown") if result else "no response"
                row[f"{det_name}_score"] = None
                row[f"{det_name}_suspicious"] = None
                row[f"{det_name}_error"] = error

        all_results.append(row)

        # Print summary for this image
        flagged = [n for n, _ in active_detectors if row.get(f"{n}_suspicious")]
        if flagged:
            print(f"FLAGGED by: {', '.join(flagged)}")
        else:
            print("clean")

        time.sleep(0.1)

    # Write raw results CSV
    csv_path = output_dir / "raw_results.csv"
    if all_results:
        fieldnames = list(all_results[0].keys())
        with open(csv_path, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction="ignore")
            writer.writeheader()
            writer.writerows(all_results)
        print(f"\nRaw results: {csv_path}")

    # Compute statistics
    print("\n" + "=" * 72)
    print("CALIBRATION RESULTS — Known Authentic Images")
    print("=" * 72)
    print(f"Corpus: {len(images)} images")
    print()

    stats = []
    for det_name, _ in active_detectors:
        scores = scores_by_detector[det_name]
        stat = analyse_scores(det_name, scores)
        stats.append(stat)

        if stat["count"] > 0:
            fp_indicator = f" ** {stat['false_positive_count']} FALSE POSITIVES **" if stat["false_positive_count"] > 0 else ""
            print(f"  {det_name:25s}  mean={stat['mean']:.3f}  p95={stat['p95']:.3f}  p99={stat['p99']:.3f}  FP={stat['false_positive_rate']}%{fp_indicator}")
        else:
            print(f"  {det_name:25s}  (no data)")

    # Threshold recommendations
    print()
    print("-" * 72)
    print("THRESHOLD RECOMMENDATIONS")
    print("-" * 72)
    print()
    print("Set the 'suspicious' threshold for each detector to its p99 value")
    print("from the authentic corpus — this ensures <1% false positive rate:")
    print()

    for stat in stats:
        if stat["count"] > 0:
            current_fp = stat["false_positive_rate"]
            recommended = stat["p99"]
            print(f"  {stat['detector']:25s}  recommended threshold: {recommended:.3f}  (current FP rate: {current_fp}%)")

    # Write stats JSON
    stats_path = output_dir / "calibration_stats.json"
    report = {
        "generated_at": datetime.now().isoformat(),
        "corpus_size": len(images),
        "corpus_label": "authentic",
        "detectors": stats,
    }
    stats_path.write_text(json.dumps(report, indent=2))
    print(f"\nFull statistics: {stats_path}")

    # Summary
    total_fp = sum(s.get("false_positive_count", 0) for s in stats)
    print(f"\nTotal false positives across all detectors: {total_fp}")
    if total_fp > 0:
        print("Review the raw_results.csv to identify which images triggered false positives.")
        print("These images may need special handling or the detector thresholds need adjustment.")


if __name__ == "__main__":
    main()
