#!/usr/bin/env python3
"""
Jura Trace — Corpus Verification Agent

Runs the full verify pipeline against corpus images (original and
protected variants) via the REST API. Records trust scores, verdicts,
and detector results for constraint validation.

Usage:
    python -m scripts.agents.verify_corpus
    python -m scripts.agents.verify_corpus --corpus corpus/protected --mode standard
    python -m scripts.agents.verify_corpus --corpus corpus/training/ai_generated --mode quick
"""

import argparse
import csv
import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config
from scripts.agents.api_client import check_api_health, verify_file


def collect_files(corpus_dir: Path, max_files: int) -> list[tuple[Path, str, str]]:
    """Collect files and infer their label and protection type.

    Returns list of (path, label, protection_type) tuples.
    label: "ai_generated" or "authentic"
    protection_type: "none", "fingerprinted", "watermarked", "c2pa_signed", "combined"
    """
    files = []

    for ext in config.IMAGE_EXTENSIONS:
        for f in corpus_dir.rglob(f"*{ext}"):
            # Infer label from path
            path_str = str(f).lower()
            if "ai_generated" in path_str or "ai-generated" in path_str:
                label = "ai_generated"
            elif "authentic" in path_str:
                label = "authentic"
            else:
                label = "unknown"

            # Infer protection from path or filename
            if "combined" in path_str:
                protection = "combined"
            elif "watermark" in path_str or "_wm" in f.stem:
                protection = "watermarked"
            elif "c2pa" in path_str or "_c2pa" in f.stem:
                protection = "c2pa_signed"
            elif "fingerprint" in path_str:
                protection = "fingerprinted"
            else:
                protection = "none"

            files.append((f, label, protection))

    files.sort(key=lambda x: str(x[0]))
    return files[:max_files]


def extract_result_fields(api_response: dict) -> dict:
    """Extract key fields from a verify API response."""
    data = api_response.get("data", {})
    return {
        "overall_trust": data.get("overallTrust"),
        "verdict": data.get("verdict"),
        "mode": data.get("mode"),
        "content_type": data.get("contentType"),
        "deepfake_score": data.get("deepfakeResult", {}).get("score")
        if data.get("deepfakeResult")
        else None,
        "deepfake_verdict": data.get("deepfakeResult", {}).get("verdict")
        if data.get("deepfakeResult")
        else None,
        "ela_score": data.get("elaResult", {}).get("score")
        if data.get("elaResult")
        else None,
        "c2pa_valid": data.get("c2paResult", {}).get("valid")
        if data.get("c2paResult")
        else None,
        "c2pa_ai_declared": data.get("c2paResult", {}).get("aiDeclared")
        if data.get("c2paResult")
        else None,
        "watermark_detected": data.get("watermarkExtractResult", {}).get("hasWatermark")
        if data.get("watermarkExtractResult")
        else None,
        "degraded": api_response.get("degraded", False),
    }


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace — Corpus Verification Agent"
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        nargs="+",
        default=[config.CORPUS_AI, config.PROTECTED_BASE],
        help="Corpus directories to verify",
    )
    parser.add_argument(
        "--mode",
        default="standard",
        choices=["quick", "standard", "deep", "archival"],
        help="Verification mode (default: standard)",
    )
    parser.add_argument(
        "--max",
        type=int,
        default=200,
        help="Max files to verify (default: 200)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=config.RESULTS_BASE / "verification_results.json",
        help="Output JSON file",
    )
    args = parser.parse_args()

    print("=" * 60)
    print("  Jura Trace — Corpus Verification Agent")
    print("=" * 60)

    # Health check
    api_health = check_api_health()
    if api_health is None:
        print("  ERROR: REST API (port 8300) is not available.")
        sys.exit(1)
    print(f"  REST API: online (v{api_health.get('version', '?')})")
    print(f"  Mode: {args.mode}")

    # Collect files from all specified corpus dirs
    all_files = []
    for corpus_dir in args.corpus:
        if corpus_dir.exists():
            files = collect_files(corpus_dir, args.max)
            all_files.extend(files)
            print(f"  {corpus_dir}: {len(files)} files")

    if not all_files:
        print("  No files found in corpus directories.")
        sys.exit(1)

    all_files = all_files[: args.max]
    print(f"  Total files to verify: {len(all_files)}")

    # Run verification
    results = []
    start = time.time()
    failed = 0

    for i, (file_path, label, protection) in enumerate(all_files):
        resp = verify_file(file_path, mode=args.mode)

        if resp is None:
            failed += 1
            results.append({
                "filename": file_path.name,
                "path": str(file_path),
                "label": label,
                "protection": protection,
                "error": True,
            })
            continue

        fields = extract_result_fields(resp)
        fields.update({
            "filename": file_path.name,
            "path": str(file_path),
            "label": label,
            "protection": protection,
            "error": False,
        })
        results.append(fields)

        if (i + 1) % 10 == 0:
            elapsed = time.time() - start
            rate = (i + 1) / elapsed if elapsed > 0 else 0
            print(f"  Verified {i + 1}/{len(all_files)} ({rate:.1f}/s)")

    # Write results
    args.output.parent.mkdir(parents=True, exist_ok=True)
    output = {
        "created_at": datetime.now(timezone.utc).isoformat(),
        "mode": args.mode,
        "total_verified": len(results),
        "failed": failed,
        "corpus_dirs": [str(d) for d in args.corpus],
        "results": results,
    }
    args.output.write_text(json.dumps(output, indent=2))

    # Also write CSV for easy analysis
    csv_path = args.output.with_suffix(".csv")
    with open(csv_path, "w", newline="") as f:
        if results:
            writer = csv.DictWriter(f, fieldnames=results[0].keys())
            writer.writeheader()
            writer.writerows(results)

    elapsed = time.time() - start
    print(f"\n{'=' * 60}")
    print(f"  Complete: {len(results)} verified, {failed} failed in {elapsed:.1f}s")
    print(f"  JSON: {args.output}")
    print(f"  CSV:  {csv_path}")
    print(f"{'=' * 60}")


if __name__ == "__main__":
    main()
