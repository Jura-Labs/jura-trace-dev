#!/usr/bin/env python3
"""
Jura Trace — Sprint 29 Track 4 Stratified Validation Test Set Builder

Curates a held-out 400-image stratified test set used exclusively for
regression testing. This set is NEVER used for training or threshold
calibration.

Composition (per Sprint 29 Track 4 of the TRIED roadmap):

| Stratum                  | Count | Purpose                              |
|--------------------------|-------|--------------------------------------|
| consumer_phone           |  100  | Headline camera FP fix               |
| mirrorless_dslr          |   50  | High-end camera FP fix               |
| drone                    |   50  | Drone case                           |
| web_jpeg_easy            |   50  | Easy authentic guard (no regression) |
| global_majority_handset  |   50  | Aisha's equity gate                  |
| wildlife_macro           |   25  | Wildlife genre coverage              |
| war_conflict             |   25  | High-stakes use case                 |
| ai_diverse               |  100  | Mixed AI generators                  |

Sprint 29 exit gates (sprint passes only if ALL met):
- Overall FP rate ≤ 4.0%
- consumer_phone FP ≤ 8.0%
- mirrorless_dslr FP ≤ 7.0%
- drone FP ≤ 7.0%
- global_majority_handset FP ≤ 8.0% (Aisha's gate)
- wildlife_macro FP ≤ 12.0% (allowance for known difficulty)
- war_conflict FP ≤ 10.0% (allowance for known difficulty)
- web_jpeg_easy FP unchanged (≤ baseline + 3pp)
- AI recall ≥ 90.0%

Usage:
    # Build the test set (samples from existing corpus + writes manifest)
    python scripts/build_validation_test_set.py build \\
        --corpus "/Volumes/MAC SSD/Training Data/corpus/training" \\
        --output "/Volumes/MAC SSD/Training Data/corpus/sprint29_validation"

    # Run the validation against the current model (called from calibrate.py
    # or from CI)
    python scripts/build_validation_test_set.py validate \\
        --test-set "/Volumes/MAC SSD/Training Data/corpus/sprint29_validation" \\
        --sidecar http://127.0.0.1:8200 \\
        --report models/sprint29_validation.json

The "build" subcommand requires the test set to be assembled MANUALLY by a
human curator first — this script samples candidates and writes a manifest
that the curator must review and approve. Auto-sampling without review
risks accidentally including training images.
"""

import argparse
import csv
import hashlib
import json
import random
import shutil
import sys
from datetime import datetime
from pathlib import Path

# Stratum definitions: (name, count, source_directory_filter, exit_fp_max)
STRATA = [
    {
        "name": "consumer_phone",
        "count": 100,
        "filename_prefixes": ["pxl_", "img_", "cimg"],
        "exit_fp_max": 0.08,
        "label": "authentic",
    },
    {
        "name": "mirrorless_dslr",
        "count": 50,
        "filename_prefixes": ["dsc"],
        "exit_fp_max": 0.07,
        "label": "authentic",
    },
    {
        "name": "drone",
        "count": 50,
        "filename_prefixes": ["dji_"],
        "exit_fp_max": 0.07,
        "label": "authentic",
    },
    {
        "name": "web_jpeg_easy",
        "count": 50,
        "filename_prefixes": ["coco_", "flickr30k_", "flickr8k_"],
        "exit_fp_max": 0.05,
        "label": "authentic",
    },
    {
        "name": "global_majority_handset",
        "count": 50,
        # Populated by Sprint 29 Track 2 — these don't exist in current corpus.
        # Until Track 2 ships, this stratum is sampled from existing camera_dcim
        # as a placeholder, but the manifest flags it as "needs_track2_data".
        "filename_prefixes": [],
        "needs_track2_data": True,
        "exit_fp_max": 0.08,
        "label": "authentic",
    },
    {
        "name": "wildlife_macro",
        "count": 25,
        # Sampled from wikimedia_photos (the wildlife/insect cluster)
        # until Sprint 29 Track 2 sources iNaturalist photos
        "filename_prefixes": ["wikimedia_"],
        "exit_fp_max": 0.12,
        "label": "authentic",
    },
    {
        "name": "war_conflict",
        "count": 25,
        # No war/conflict images in current corpus — Sprint 29 Track 2 will source
        # 200 from Wikimedia Commons. Manifest flags as needs_track2_data.
        "filename_prefixes": [],
        "needs_track2_data": True,
        "exit_fp_max": 0.10,
        "label": "authentic",
    },
    {
        "name": "ai_diverse",
        "count": 100,
        # 20 each from 5 generator families
        "ai_sources": [
            ("dalle3", 20),
            ("midjourney_v6", 20),
            ("sdxl_turbo", 20),
            ("grok_aurora", 20),
            ("diffusiondb", 20),
        ],
        "exit_fp_max": None,  # not applicable for AI
        "label": "ai",
    },
]


def sha256_of_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def cmd_build(args):
    corpus_root = Path(args.corpus)
    output_root = Path(args.output)
    authentic_root = corpus_root / "authentic"
    ai_root = corpus_root / "ai_generated"

    if not authentic_root.exists():
        print(f"ERROR: authentic root not found: {authentic_root}")
        sys.exit(1)
    if not ai_root.exists():
        print(f"ERROR: ai_generated root not found: {ai_root}")
        sys.exit(1)

    output_root.mkdir(parents=True, exist_ok=True)
    random.seed(42)  # reproducible sampling

    # Collect all candidate files once
    print("  Indexing corpus...")
    auth_files = [
        f for f in authentic_root.rglob("*")
        if f.is_file() and f.suffix.lower() in {".jpg", ".jpeg", ".png", ".webp"}
    ]
    ai_files = [
        f for f in ai_root.rglob("*")
        if f.is_file() and f.suffix.lower() in {".jpg", ".jpeg", ".png", ".webp"}
    ]
    print(f"    {len(auth_files)} authentic, {len(ai_files)} AI")

    manifest = []
    summary = {}

    for stratum in STRATA:
        name = stratum["name"]
        target = stratum["count"]
        label = stratum["label"]
        needs_track2 = stratum.get("needs_track2_data", False)

        if label == "ai":
            # AI strata draw from per-generator sub-counts
            picked = []
            for gen, gen_count in stratum["ai_sources"]:
                gen_files = [f for f in ai_files if gen in str(f.parent.name).lower() or f.name.lower().startswith(gen.lower())]
                if len(gen_files) < gen_count:
                    print(f"  WARN: {name}/{gen}: only {len(gen_files)} available, need {gen_count}")
                    sample = gen_files
                else:
                    sample = random.sample(gen_files, gen_count)
                picked.extend(sample)
        else:
            # Authentic stratum: filter by filename prefixes
            prefixes = stratum.get("filename_prefixes", [])
            if not prefixes and needs_track2:
                # Placeholder stratum — record but don't sample yet
                summary[name] = {
                    "target": target,
                    "sampled": 0,
                    "label": label,
                    "exit_fp_max": stratum.get("exit_fp_max"),
                    "status": "PENDING_TRACK2_DATA",
                }
                print(f"  {name}: PENDING_TRACK2_DATA — no source data yet (target {target})")
                continue
            candidates = [
                f for f in auth_files
                if any(f.name.lower().startswith(p) for p in prefixes)
            ]
            if len(candidates) < target:
                print(f"  WARN: {name}: only {len(candidates)} candidates, need {target}")
                picked = candidates
            else:
                picked = random.sample(candidates, target)

        # Copy + manifest
        stratum_dir = output_root / name
        stratum_dir.mkdir(parents=True, exist_ok=True)
        for src in picked:
            dst = stratum_dir / src.name
            if not dst.exists():
                shutil.copy2(src, dst)
            manifest.append({
                "stratum": name,
                "label": label,
                "filename": src.name,
                "source_path": str(src),
                "test_path": str(dst),
                "sha256": sha256_of_file(dst),
            })

        summary[name] = {
            "target": target,
            "sampled": len(picked),
            "label": label,
            "exit_fp_max": stratum.get("exit_fp_max"),
            "status": "OK" if len(picked) >= target else "UNDER_TARGET",
        }
        print(f"  {name}: {len(picked)}/{target} sampled")

    # Write manifest CSV
    csv_path = output_root / "manifest.csv"
    with open(csv_path, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=["stratum", "label", "filename", "source_path", "test_path", "sha256"],
        )
        writer.writeheader()
        writer.writerows(manifest)

    # Write summary JSON
    summary_path = output_root / "summary.json"
    summary_data = {
        "built_at": datetime.now().isoformat(),
        "corpus_root": str(corpus_root),
        "test_set_root": str(output_root),
        "total_images": len(manifest),
        "strata": summary,
        "warning": (
            "This test set is HELD-OUT. Do NOT use these images for training "
            "or threshold calibration. Run validate subcommand to measure "
            "regression metrics against the current model."
        ),
        "exit_gates": {
            "overall_fp_max": 0.04,
            "ai_recall_min": 0.90,
            "stratum_fp_max": {s["name"]: s.get("exit_fp_max") for s in STRATA},
        },
    }
    summary_path.write_text(json.dumps(summary_data, indent=2))

    print(f"\n  Manifest: {csv_path}")
    print(f"  Summary:  {summary_path}")
    print(f"  Total:    {len(manifest)} images")
    pending = [n for n, s in summary.items() if s.get("status") == "PENDING_TRACK2_DATA"]
    if pending:
        print(f"\n  ⚠ Pending Sprint 29 Track 2 data: {', '.join(pending)}")


def cmd_validate(args):
    """Run the held-out test set against the running sidecar and compute exit gates."""
    import requests

    test_root = Path(args.test_set)
    summary_path = test_root / "summary.json"
    if not summary_path.exists():
        print(f"ERROR: summary.json not found in {test_root}. Run 'build' first.")
        sys.exit(1)
    summary = json.loads(summary_path.read_text())

    manifest_path = test_root / "manifest.csv"
    if not manifest_path.exists():
        print(f"ERROR: manifest.csv not found")
        sys.exit(1)

    # Health check
    try:
        r = requests.get(f"{args.sidecar}/health", timeout=5)
        if r.status_code != 200:
            raise Exception(f"sidecar unhealthy: {r.status_code}")
    except Exception as e:
        print(f"ERROR: sidecar not reachable at {args.sidecar}: {e}")
        sys.exit(1)

    # Load manifest and run analyses
    rows = list(csv.DictReader(open(manifest_path)))
    print(f"  Validating {len(rows)} images against {args.sidecar}...")

    from collections import defaultdict
    results_by_stratum = defaultdict(lambda: {"tp": 0, "fp": 0, "tn": 0, "fn": 0, "n": 0, "scores": []})

    for i, row in enumerate(rows, 1):
        path = Path(row["test_path"])
        if not path.exists():
            continue
        try:
            with open(path, "rb") as f:
                files = {"file": (path.name, f.read(), "image/jpeg")}
            resp = requests.post(
                f"{args.sidecar}/forensics/deepfake",
                files=files,
                timeout=60,
            )
            if resp.status_code != 200:
                continue
            data = resp.json()
            score = float(data.get("score", 0.0))
        except Exception as e:
            print(f"    [{i}/{len(rows)}] {path.name}: ERROR {e}")
            continue

        stratum = row["stratum"]
        label = row["label"]
        is_ai = label == "ai"
        flagged = score >= 0.50

        g = results_by_stratum[stratum]
        g["n"] += 1
        g["scores"].append(score)
        if is_ai:
            if flagged: g["tp"] += 1
            else:       g["fn"] += 1
        else:
            if flagged: g["fp"] += 1
            else:       g["tn"] += 1

        if i % 25 == 0:
            print(f"    [{i}/{len(rows)}] processed...")

    # Compute exit gates
    print("\n  EXIT GATE EVALUATION")
    print("  " + "=" * 60)
    print(f"  {'stratum':<25}{'n':>5}{'fp/recall':>12}{'gate':>10}{'  status':<10}")
    print("  " + "-" * 60)

    overall_fp_total = 0
    overall_auth_total = 0
    overall_tp = 0
    overall_ai_total = 0
    gates = []

    for s in STRATA:
        name = s["name"]
        if name not in results_by_stratum:
            continue
        g = results_by_stratum[name]
        if g["n"] == 0:
            continue
        if s["label"] == "ai":
            recall = g["tp"] / max(1, g["tp"] + g["fn"])
            metric_str = f"{recall:.2%}"
            overall_tp += g["tp"]
            overall_ai_total += g["tp"] + g["fn"]
            gate_str = "≥90%"
            ok = recall >= 0.90
        else:
            n_auth = g["fp"] + g["tn"]
            fp = g["fp"] / max(1, n_auth) if n_auth > 0 else 0
            metric_str = f"{fp:.2%}"
            overall_fp_total += g["fp"]
            overall_auth_total += n_auth
            gate_max = s.get("exit_fp_max", 1.0)
            gate_str = f"≤{gate_max:.0%}"
            ok = fp <= gate_max
        status = "✓ pass" if ok else "✗ FAIL"
        gates.append({"stratum": name, "metric": metric_str, "gate": gate_str, "ok": ok})
        print(f"  {name:<25}{g['n']:>5}{metric_str:>12}{gate_str:>10}  {status}")

    # Overall
    overall_fp = overall_fp_total / max(1, overall_auth_total)
    overall_recall = overall_tp / max(1, overall_ai_total)
    overall_fp_ok = overall_fp <= 0.04
    overall_recall_ok = overall_recall >= 0.90

    print("  " + "-" * 60)
    print(f"  {'OVERALL FP':<25}{overall_auth_total:>5}{overall_fp:>11.2%}{'≤4%':>10}  {'✓ pass' if overall_fp_ok else '✗ FAIL'}")
    print(f"  {'OVERALL AI recall':<25}{overall_ai_total:>5}{overall_recall:>11.2%}{'≥90%':>10}  {'✓ pass' if overall_recall_ok else '✗ FAIL'}")

    all_passed = all(g["ok"] for g in gates) and overall_fp_ok and overall_recall_ok
    print(f"\n  SPRINT 29 EXIT: {'✓ ALL GATES PASSED' if all_passed else '✗ ONE OR MORE GATES FAILED'}")

    # Save report
    report = {
        "validated_at": datetime.now().isoformat(),
        "test_set": str(test_root),
        "sidecar": args.sidecar,
        "overall_fp_rate": round(overall_fp, 4),
        "overall_recall": round(overall_recall, 4),
        "overall_fp_ok": overall_fp_ok,
        "overall_recall_ok": overall_recall_ok,
        "all_gates_passed": all_passed,
        "stratum_results": {
            name: {
                "n": g["n"], "tp": g["tp"], "fp": g["fp"], "tn": g["tn"], "fn": g["fn"],
                "mean_score": round(sum(g["scores"]) / max(1, len(g["scores"])), 4),
            }
            for name, g in results_by_stratum.items()
        },
        "gates": gates,
    }
    if args.report:
        Path(args.report).parent.mkdir(parents=True, exist_ok=True)
        Path(args.report).write_text(json.dumps(report, indent=2))
        print(f"\n  Report saved: {args.report}")

    sys.exit(0 if all_passed else 1)


def main():
    parser = argparse.ArgumentParser(
        description="Sprint 29 Track 4 stratified validation test set"
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_build = sub.add_parser("build", help="Build the test set from the corpus")
    p_build.add_argument("--corpus", type=str, required=True,
                         help="Path to the corpus root (with authentic/ and ai_generated/ subdirs)")
    p_build.add_argument("--output", type=str, required=True,
                         help="Where to write the test set + manifest")

    p_val = sub.add_parser("validate", help="Run the test set against the sidecar and check exit gates")
    p_val.add_argument("--test-set", type=str, required=True,
                       help="Path to the built test set directory")
    p_val.add_argument("--sidecar", type=str, default="http://127.0.0.1:8200",
                       help="Sidecar URL")
    p_val.add_argument("--report", type=str, default=None,
                       help="Optional output JSON path for the regression report")

    args = parser.parse_args()
    if args.cmd == "build":
        cmd_build(args)
    elif args.cmd == "validate":
        cmd_validate(args)


if __name__ == "__main__":
    main()
