#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Deep Corpus Review Pipeline

Comprehensive training and testing pipeline:
  - 500 AI-generated images from ELSA 1M (multi-model: SD, DALL-E, MJ)
  - 500 authentic images from Wikimedia Commons + CIFAR-10
  - 50 protected variants (fingerprint, watermark, C2PA, combined)
  - Verification across standard, deep, and archival modes
  - Constraint validation: AI+protection must score < 0.70

This runs all stages sequentially with detailed progress reporting.
Expected runtime: 2-4 hours depending on hardware.

Usage:
    python -m scripts.agents.deep_review
    python -m scripts.agents.deep_review --resume  # skip crawling if corpus exists
"""

import argparse
import csv
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config
from scripts.agents.api_client import (
    check_api_health,
    check_sidecar_health,
    verify_file,
    fingerprint_file,
    watermark_embed_sidecar,
    sign_c2pa_sidecar,
)


# ── Configuration ────────────────────────────────────────────────────────
AI_TARGET = 500
AUTHENTIC_TARGET = 500
PROTECT_TARGET = 50
VERIFY_MODES = ["standard", "deep", "archival"]
TRUST_THRESHOLD = 0.70


def timestamp():
    return datetime.now().strftime("%H:%M:%S")


def banner(text: str):
    print(f"\n{'=' * 70}")
    print(f"  [{timestamp()}] {text}")
    print(f"{'=' * 70}")


def stage_header(text: str):
    print(f"\n{'─' * 70}")
    print(f"  [{timestamp()}] {text}")
    print(f"{'─' * 70}")


# ── Stage 1: Crawl ──────────────────────────────────────────────────────

def crawl_ai_images(target: int) -> int:
    """Crawl AI-generated images."""
    stage_header(f"CRAWL: {target} AI-generated images")

    from scripts.agents.crawl_ai_images import download_from_huggingface, SOURCES

    output_dir = config.CORPUS_AI
    all_entries = []

    # Use ELSA 1M as primary source — it has multi-model outputs
    source_key = "elsa"
    if source_key in SOURCES:
        source_dir = output_dir / source_key
        entries = download_from_huggingface(
            source_key, SOURCES[source_key], source_dir, target
        )
        all_entries.extend(entries)

    # Write manifest
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "manifest.json"

    if manifest_path.exists():
        existing = json.loads(manifest_path.read_text())
        existing_entries = existing.get("entries", [])
        existing_hashes = {e["sha256"] for e in existing_entries}
        new_entries = [e for e in all_entries if e["sha256"] not in existing_hashes]
        all_entries = existing_entries + new_entries

    manifest = {
        "corpus_type": "ai_generated",
        "created_at": datetime.now(timezone.utc).isoformat(),
        "total_images": len(all_entries),
        "entries": all_entries,
    }
    manifest_path.write_text(json.dumps(manifest, indent=2))

    print(f"  Total AI images in corpus: {len(all_entries)}")
    return len(all_entries)


def crawl_authentic_images(target: int) -> int:
    """Crawl authentic images."""
    stage_header(f"CRAWL: {target} authentic images")

    from scripts.agents.crawl_authentic_images import SOURCES

    output_dir = config.CORPUS_AUTHENTIC
    all_entries = []

    # Split target across sources
    per_source = target // len(SOURCES) + 1

    for source_key, download_fn in SOURCES.items():
        source_dir = output_dir / source_key
        entries = download_fn(source_dir, per_source)
        all_entries.extend(entries)
        if len(all_entries) >= target:
            break

    # Write manifest
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "manifest.json"

    if manifest_path.exists():
        existing = json.loads(manifest_path.read_text())
        existing_entries = existing.get("entries", [])
        existing_hashes = {e["sha256"] for e in existing_entries}
        new_entries = [e for e in all_entries if e["sha256"] not in existing_hashes]
        all_entries = existing_entries + new_entries

    manifest = {
        "corpus_type": "authentic",
        "created_at": datetime.now(timezone.utc).isoformat(),
        "total_images": len(all_entries),
        "entries": all_entries,
    }
    manifest_path.write_text(json.dumps(manifest, indent=2))

    print(f"  Total authentic images in corpus: {len(all_entries)}")
    return len(all_entries)


# ── Stage 2: Apply Protections ──────────────────────────────────────────

def apply_protections(max_images: int) -> dict:
    """Apply all four protection types to AI images."""
    stage_header(f"PROTECT: Apply 4 protection types to {max_images} AI images")

    # Collect AI images
    images = []
    for ext in config.IMAGE_EXTENSIONS:
        images.extend(config.CORPUS_AI.rglob(f"*{ext}"))
    images.sort()
    images = images[:max_images]

    if not images:
        print("  No AI images found for protection.")
        return {"total": 0}

    print(f"  Processing {len(images)} images...")

    results = {
        "fingerprinted": [],
        "watermarked": [],
        "c2pa_signed": [],
        "combined": [],
    }

    for i, img_path in enumerate(images):
        pct = (i + 1) / len(images) * 100
        if (i + 1) % 10 == 0 or i == 0:
            print(f"  [{timestamp()}] Image {i + 1}/{len(images)} ({pct:.0f}%) — {img_path.name}")

        # Fingerprint
        fp = fingerprint_file(img_path)
        if fp:
            results["fingerprinted"].append({
                "original": str(img_path),
                "hashes": fp.get("data", {}).get("hashes", []),
            })

        # Watermark
        wm_dir = config.PROTECTED_BASE / "watermarked"
        wm_dir.mkdir(parents=True, exist_ok=True)
        wm_bytes = watermark_embed_sidecar(img_path, config.C2PA_CREATOR_NAME)
        if wm_bytes:
            wm_path = wm_dir / f"{img_path.stem}_wm.png"
            wm_path.write_bytes(wm_bytes)
            results["watermarked"].append({
                "original": str(img_path),
                "protected_path": str(wm_path),
            })

        # C2PA sign
        c2pa_dir = config.PROTECTED_BASE / "c2pa_signed"
        c2pa_dir.mkdir(parents=True, exist_ok=True)
        signed = sign_c2pa_sidecar(img_path, config.C2PA_CREATOR_NAME)
        if signed:
            c2pa_path = c2pa_dir / f"{img_path.stem}_c2pa{img_path.suffix}"
            c2pa_path.write_bytes(signed)
            results["c2pa_signed"].append({
                "original": str(img_path),
                "protected_path": str(c2pa_path),
            })

        # Combined (watermark + C2PA)
        if wm_bytes:
            combined_dir = config.PROTECTED_BASE / "combined"
            combined_dir.mkdir(parents=True, exist_ok=True)
            wm_tmp = combined_dir / f"_tmp_{img_path.stem}.png"
            wm_tmp.write_bytes(wm_bytes)
            combined_signed = sign_c2pa_sidecar(wm_tmp, config.C2PA_CREATOR_NAME)
            wm_tmp.unlink(missing_ok=True)
            if combined_signed:
                combo_path = combined_dir / f"{img_path.stem}_combined.png"
                combo_path.write_bytes(combined_signed)
                results["combined"].append({
                    "original": str(img_path),
                    "protected_path": str(combo_path),
                })

    for ptype, items in results.items():
        print(f"  {ptype}: {len(items)} images")

    # Save results
    config.RESULTS_BASE.mkdir(parents=True, exist_ok=True)
    (config.RESULTS_BASE / "protection_results.json").write_text(
        json.dumps(results, indent=2)
    )

    return {k: len(v) for k, v in results.items()}


# ── Stage 3: Verify ─────────────────────────────────────────────────────

def collect_all_verify_files(max_per_category: int = 500) -> list[tuple[Path, str, str]]:
    """Collect all files for verification with labels."""
    files = []

    # AI originals
    for ext in config.IMAGE_EXTENSIONS:
        for f in config.CORPUS_AI.rglob(f"*{ext}"):
            files.append((f, "ai_generated", "none"))
    ai_count = len(files)

    # Authentic originals
    auth_start = len(files)
    for ext in config.IMAGE_EXTENSIONS:
        for f in config.CORPUS_AUTHENTIC.rglob(f"*{ext}"):
            files.append((f, "authentic", "none"))
    auth_count = len(files) - auth_start

    # Protected variants
    prot_start = len(files)
    for prot_type in ["watermarked", "c2pa_signed", "combined"]:
        prot_dir = config.PROTECTED_BASE / prot_type
        if prot_dir.exists():
            for ext in config.IMAGE_EXTENSIONS:
                for f in prot_dir.rglob(f"*{ext}"):
                    if not f.name.startswith("_tmp_"):
                        files.append((f, "ai_generated", prot_type))
    prot_count = len(files) - prot_start

    print(f"  Files collected: {ai_count} AI original, {auth_count} authentic, {prot_count} protected")
    return files


def _verify_single(args: tuple) -> dict:
    """Verify a single file — worker function for thread pool."""
    file_path, label, protection, mode = args
    resp = verify_file(file_path, mode=mode)

    result = {
        "filename": file_path.name,
        "path": str(file_path),
        "label": label,
        "protection": protection,
        "mode": mode,
        "error": resp is None,
    }

    if resp is not None:
        data = resp.get("data", {})
        result.update({
            "overall_trust": data.get("overallTrust"),
            "verdict": data.get("verdict"),
            "content_type": data.get("contentType"),
            "degraded": resp.get("degraded", False),
        })

        df = data.get("deepfakeResult")
        if df:
            result["deepfake_score"] = df.get("score")
            result["deepfake_verdict"] = df.get("verdict")
            result["classifier_score"] = df.get("classifierScore")

        ela = data.get("elaResult")
        if ela:
            result["ela_score"] = ela.get("score")

        c2pa = data.get("c2paResult")
        if c2pa:
            result["c2pa_valid"] = c2pa.get("valid")
            result["c2pa_ai_declared"] = c2pa.get("aiDeclared")

        wm = data.get("watermarkExtractResult")
        if wm:
            result["watermark_detected"] = wm.get("hasWatermark")
            result["watermark_confidence"] = wm.get("confidence")

    return result


# Default concurrency — 4 parallel verify requests. The sidecar uses
# CPU-bound OpenCV/numpy so more than 4 gives diminishing returns and
# may cause OOM on machines with < 8 GB RAM.
DEFAULT_WORKERS = 4


def verify_across_modes(
    files: list[tuple[Path, str, str]],
    modes: list[str],
    max_workers: int = DEFAULT_WORKERS,
) -> list[dict]:
    """Verify all files across multiple modes using a thread pool."""
    from concurrent.futures import ThreadPoolExecutor, as_completed

    all_results = []
    total_ops = len(files) * len(modes)
    completed = 0
    start = time.time()

    for mode in modes:
        stage_header(f"VERIFY: {len(files)} files in {mode} mode ({max_workers} workers)")
        mode_start = time.time()

        work_items = [(fp, label, prot, mode) for fp, label, prot in files]
        mode_results = []

        with ThreadPoolExecutor(max_workers=max_workers) as pool:
            futures = {pool.submit(_verify_single, item): i for i, item in enumerate(work_items)}

            for future in as_completed(futures):
                result = future.result()
                mode_results.append(result)
                completed += 1

                if completed % 25 == 0 or completed == 1:
                    elapsed = time.time() - start
                    rate = completed / elapsed if elapsed > 0 else 0
                    eta = (total_ops - completed) / rate if rate > 0 else 0
                    done_mode = len(mode_results)
                    print(
                        f"  [{timestamp()}] {mode}: {done_mode}/{len(files)} "
                        f"(total {completed}/{total_ops}, "
                        f"{rate:.1f}/s, ETA {eta / 60:.0f}m)"
                    )

        all_results.extend(mode_results)
        mode_elapsed = time.time() - mode_start
        print(f"  [{timestamp()}] {mode} mode complete in {mode_elapsed:.0f}s")

    return all_results


# ── Stage 4: Validate ───────────────────────────────────────────────────

def validate_results(results: list[dict]) -> dict:
    """Run constraint validation across all modes."""
    stage_header("VALIDATE: AI+protection constraint (trust < 0.70)")

    report = {
        "threshold": TRUST_THRESHOLD,
        "modes_tested": VERIFY_MODES,
        "by_mode": {},
        "violations": [],
        "overall_pass": True,
    }

    for mode in VERIFY_MODES:
        mode_results = [r for r in results if r.get("mode") == mode and not r.get("error")]

        ai_results = [r for r in mode_results if r.get("label") == "ai_generated"]
        auth_results = [r for r in mode_results if r.get("label") == "authentic"]

        # AI by protection type
        by_prot = {}
        mode_violations = []

        for r in ai_results:
            prot = r.get("protection", "none")
            by_prot.setdefault(prot, []).append(r)

            trust = r.get("overall_trust")
            if trust is not None and trust >= TRUST_THRESHOLD:
                mode_violations.append(r)
                report["overall_pass"] = False

        # Compute stats
        prot_stats = {}
        for prot, items in by_prot.items():
            scores = [r["overall_trust"] for r in items if r.get("overall_trust") is not None]
            if scores:
                prot_stats[prot] = {
                    "count": len(scores),
                    "mean": round(sum(scores) / len(scores), 4),
                    "min": round(min(scores), 4),
                    "max": round(max(scores), 4),
                    "violations": sum(1 for s in scores if s >= TRUST_THRESHOLD),
                }

        # Authentic FP stats
        auth_scores = [r["overall_trust"] for r in auth_results if r.get("overall_trust") is not None]
        auth_stats = {}
        if auth_scores:
            auth_stats = {
                "count": len(auth_scores),
                "mean": round(sum(auth_scores) / len(auth_scores), 4),
                "min": round(min(auth_scores), 4),
                "max": round(max(auth_scores), 4),
                "fp_count": sum(1 for s in auth_scores if s < 0.40),
                "fp_rate": round(sum(1 for s in auth_scores if s < 0.40) / len(auth_scores), 4),
            }

        report["by_mode"][mode] = {
            "total_verified": len(mode_results),
            "ai_count": len(ai_results),
            "authentic_count": len(auth_results),
            "violations": len(mode_violations),
            "protection_breakdown": prot_stats,
            "authentic_stats": auth_stats,
        }
        report["violations"].extend(mode_violations)

    return report


def print_validation_report(report: dict):
    """Print the constraint validation report."""
    status = "PASS" if report["overall_pass"] else "FAIL"

    banner(f"CONSTRAINT VALIDATION: {status}")
    print(f"  Threshold: AI trust < {report['threshold']}")
    print(f"  Total violations: {len(report['violations'])}")

    for mode, stats in report["by_mode"].items():
        print(f"\n  ── {mode.upper()} MODE ──")
        print(f"  Verified: {stats['total_verified']} ({stats['ai_count']} AI, {stats['authentic_count']} authentic)")
        print(f"  Violations: {stats['violations']}")

        if stats["protection_breakdown"]:
            print(f"  Protection breakdown:")
            for prot, ps in stats["protection_breakdown"].items():
                flag = " *** VIOLATIONS" if ps["violations"] > 0 else ""
                print(
                    f"    {prot:15s}: n={ps['count']}, "
                    f"mean={ps['mean']:.3f}, "
                    f"range=[{ps['min']:.3f}, {ps['max']:.3f}], "
                    f"violations={ps['violations']}{flag}"
                )

        if stats["authentic_stats"]:
            a = stats["authentic_stats"]
            print(
                f"  Authentic: n={a['count']}, mean={a['mean']:.3f}, "
                f"FP rate={a['fp_rate']:.1%} ({a['fp_count']}/{a['count']})"
            )

    if report["violations"]:
        print(f"\n  ── TOP VIOLATIONS ──")
        for v in sorted(report["violations"], key=lambda x: x.get("overall_trust", 0), reverse=True)[:20]:
            print(
                f"  {v['filename']:35s} mode={v.get('mode','?'):10s} "
                f"trust={v.get('overall_trust', '?'):>6} "
                f"prot={v.get('protection','?'):12s} "
                f"df={v.get('deepfake_score', '?')}"
            )


# ── Main ─────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Jura Trace — Deep Corpus Review")
    parser.add_argument("--resume", action="store_true", help="Skip crawling if corpus exists")
    parser.add_argument("--skip-protect", action="store_true", help="Skip protection stage")
    parser.add_argument("--ai-count", type=int, default=AI_TARGET)
    parser.add_argument("--authentic-count", type=int, default=AUTHENTIC_TARGET)
    parser.add_argument("--protect-count", type=int, default=PROTECT_TARGET)
    parser.add_argument("--workers", type=int, default=DEFAULT_WORKERS, help="Parallel verify workers (default: 4)")
    args = parser.parse_args()

    banner("JURA TRACE — DEEP CORPUS REVIEW PIPELINE")
    print(f"  AI images: {args.ai_count}")
    print(f"  Authentic images: {args.authentic_count}")
    print(f"  Protected variants: {args.protect_count}")
    print(f"  Verify modes: {', '.join(VERIFY_MODES)}")
    print(f"  Trust ceiling: {TRUST_THRESHOLD}")

    # Preflight
    stage_header("PREFLIGHT CHECKS")
    api = check_api_health()
    if api is None:
        print("  FAIL: REST API (port 8300) is offline.")
        sys.exit(1)
    print(f"  REST API: online (v{api.get('version', '?')})")
    print(f"  Sidecar available: {api.get('sidecarAvailable', False)}")

    sidecar = check_sidecar_health()
    if sidecar is None:
        print("  FAIL: Sidecar (port 8200) is offline. Full forensic analysis requires it.")
        sys.exit(1)

    caps = sidecar.get("capabilities", {})
    active = sum(1 for v in caps.values() if v)
    print(f"  Sidecar: online ({active} capabilities)")

    pipeline_start = time.time()

    # ── Stage 1: Crawl ───────────────────────────────────────────────
    if args.resume and config.CORPUS_AI.exists():
        ai_manifest = config.CORPUS_AI / "manifest.json"
        if ai_manifest.exists():
            existing = json.loads(ai_manifest.read_text())
            ai_count = existing.get("total_images", 0)
            print(f"\n  Resuming: {ai_count} AI images already in corpus")
        else:
            ai_count = crawl_ai_images(args.ai_count)
    else:
        ai_count = crawl_ai_images(args.ai_count)

    if args.resume and config.CORPUS_AUTHENTIC.exists():
        auth_manifest = config.CORPUS_AUTHENTIC / "manifest.json"
        if auth_manifest.exists():
            existing = json.loads(auth_manifest.read_text())
            auth_count = existing.get("total_images", 0)
            print(f"  Resuming: {auth_count} authentic images already in corpus")
        else:
            auth_count = crawl_authentic_images(args.authentic_count)
    else:
        auth_count = crawl_authentic_images(args.authentic_count)

    # ── Stage 2: Protect ─────────────────────────────────────────────
    if not args.skip_protect:
        prot_stats = apply_protections(args.protect_count)
    else:
        print(f"\n  Skipping protection stage")

    # ── Stage 3: Verify ──────────────────────────────────────────────
    all_files = collect_all_verify_files()

    if not all_files:
        print("  No files to verify.")
        sys.exit(1)

    all_results = verify_across_modes(all_files, VERIFY_MODES, max_workers=args.workers)

    # Save raw results
    config.RESULTS_BASE.mkdir(parents=True, exist_ok=True)
    results_path = config.RESULTS_BASE / "deep_review_results.json"
    output = {
        "created_at": datetime.now(timezone.utc).isoformat(),
        "config": {
            "ai_target": args.ai_count,
            "authentic_target": args.authentic_count,
            "protect_target": args.protect_count,
            "modes": VERIFY_MODES,
            "threshold": TRUST_THRESHOLD,
        },
        "total_verifications": len(all_results),
        "results": all_results,
    }
    results_path.write_text(json.dumps(output, indent=2))

    # CSV for spreadsheet analysis
    csv_path = config.RESULTS_BASE / "deep_review_results.csv"
    with open(csv_path, "w", newline="") as f:
        if all_results:
            writer = csv.DictWriter(f, fieldnames=all_results[0].keys())
            writer.writeheader()
            writer.writerows(all_results)

    # ── Stage 4: Validate ────────────────────────────────────────────
    report = validate_results(all_results)
    report["created_at"] = datetime.now(timezone.utc).isoformat()
    report_path = config.RESULTS_BASE / "deep_review_constraint_report.json"
    report_path.write_text(json.dumps(report, indent=2))

    print_validation_report(report)

    # ── Final Summary ────────────────────────────────────────────────
    total_elapsed = time.time() - pipeline_start
    banner("DEEP REVIEW COMPLETE")
    print(f"  Total time: {total_elapsed / 60:.1f} minutes")
    print(f"  AI corpus: {ai_count} images")
    print(f"  Authentic corpus: {auth_count} images")
    print(f"  Total verifications: {len(all_results)}")
    print(f"  Constraint: {'PASS' if report['overall_pass'] else 'FAIL'}")
    print(f"  Violations: {len(report['violations'])}")
    print(f"\n  Results: {results_path}")
    print(f"  CSV: {csv_path}")
    print(f"  Report: {report_path}")


if __name__ == "__main__":
    main()
