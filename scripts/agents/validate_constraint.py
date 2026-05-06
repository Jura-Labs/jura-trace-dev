#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Constraint Validation Agent

THE CRITICAL AGENT: Enforces the rule that AI-generated content must
ALWAYS score below 0.70 trust, regardless of what protections have been
applied (fingerprinting, watermarking, C2PA signing, or all combined).

Protection provenance must NEVER mask AI origin.

Reads verification results from verify_corpus.py and produces a pass/fail
report with per-protection breakdowns and recommended threshold adjustments.

Usage:
    python -m scripts.agents.validate_constraint
    python -m scripts.agents.validate_constraint --results corpus/results/verification_results.json
    python -m scripts.agents.validate_constraint --threshold 0.65
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config


def load_results(results_path: Path) -> list[dict]:
    """Load verification results JSON."""
    data = json.loads(results_path.read_text())
    return data.get("results", [])


def validate(results: list[dict], threshold: float) -> dict:
    """Run constraint validation.

    Returns a report dict with pass/fail status and details.
    """
    ai_results = [r for r in results if r.get("label") == "ai_generated" and not r.get("error")]
    authentic_results = [r for r in results if r.get("label") == "authentic" and not r.get("error")]

    # Group AI results by protection type
    by_protection = {}
    for r in ai_results:
        prot = r.get("protection", "none")
        by_protection.setdefault(prot, []).append(r)

    # ── Primary constraint: AI + any protection < threshold ──────────
    violations = []
    protection_stats = {}

    for prot, items in sorted(by_protection.items()):
        trust_scores = [r["overall_trust"] for r in items if r.get("overall_trust") is not None]
        if not trust_scores:
            protection_stats[prot] = {"count": len(items), "no_scores": True}
            continue

        passing = [s for s in trust_scores if s < threshold]
        failing = [s for s in trust_scores if s >= threshold]

        for r in items:
            trust = r.get("overall_trust")
            if trust is not None and trust >= threshold:
                violations.append({
                    "filename": r["filename"],
                    "path": r.get("path", ""),
                    "protection": prot,
                    "overall_trust": trust,
                    "deepfake_score": r.get("deepfake_score"),
                    "deepfake_verdict": r.get("deepfake_verdict"),
                    "c2pa_valid": r.get("c2pa_valid"),
                    "watermark_detected": r.get("watermark_detected"),
                    "verdict": r.get("verdict"),
                })

        protection_stats[prot] = {
            "count": len(items),
            "scores": len(trust_scores),
            "mean_trust": sum(trust_scores) / len(trust_scores),
            "min_trust": min(trust_scores),
            "max_trust": max(trust_scores),
            "pass_count": len(passing),
            "fail_count": len(failing),
            "pass_rate": len(passing) / len(trust_scores) if trust_scores else 0,
        }

    # ── Authentic FP analysis ────────────────────────────────────────
    authentic_trust = [
        r["overall_trust"] for r in authentic_results
        if r.get("overall_trust") is not None
    ]
    authentic_stats = {}
    if authentic_trust:
        # FP = authentic image scored as synthetic (trust < 0.40)
        fp_count = sum(1 for t in authentic_trust if t < 0.40)
        authentic_stats = {
            "count": len(authentic_trust),
            "mean_trust": sum(authentic_trust) / len(authentic_trust),
            "min_trust": min(authentic_trust),
            "max_trust": max(authentic_trust),
            "fp_count": fp_count,
            "fp_rate": fp_count / len(authentic_trust),
        }

    # ── Overall verdict ──────────────────────────────────────────────
    all_pass = len(violations) == 0

    # ── Recommendations ──────────────────────────────────────────────
    recommendations = []
    if not all_pass:
        recommendations.append(
            f"CRITICAL: {len(violations)} AI images scored >= {threshold} "
            f"after protection. Protection must not mask AI origin."
        )
        # Check if watermarking is the culprit
        wm_violations = [v for v in violations if v["protection"] in ("watermarked", "combined")]
        if wm_violations:
            recommendations.append(
                "Watermarked AI images are bypassing detection. "
                "Consider: (a) tighten deepfake classifier threshold, "
                "(b) add watermark-aware penalty in compute_trust, "
                "(c) retrain classifier with watermarked AI samples."
            )
        c2pa_violations = [v for v in violations if v["protection"] in ("c2pa_signed", "combined")]
        if c2pa_violations:
            recommendations.append(
                "C2PA-signed AI images are scoring too high. "
                "The +0.10 C2PA bonus may be inflating trust for AI content. "
                "Consider capping C2PA bonus when deepfake_score > 0.5."
            )
    else:
        recommendations.append(
            f"PASS: All {len(ai_results)} AI images scored below {threshold} "
            f"across all protection types."
        )

    return {
        "constraint": f"AI-generated trust < {threshold}",
        "threshold": threshold,
        "overall_pass": all_pass,
        "total_ai_images": len(ai_results),
        "total_authentic_images": len(authentic_results),
        "total_violations": len(violations),
        "protection_breakdown": protection_stats,
        "authentic_stats": authentic_stats,
        "violations": violations,
        "recommendations": recommendations,
    }


def print_report(report: dict):
    """Print a human-readable constraint report."""
    status = "PASS" if report["overall_pass"] else "FAIL"
    print(f"\n{'=' * 60}")
    print(f"  CONSTRAINT VALIDATION: {status}")
    print(f"{'=' * 60}")
    print(f"  Rule: {report['constraint']}")
    print(f"  AI images tested: {report['total_ai_images']}")
    print(f"  Authentic images tested: {report['total_authentic_images']}")
    print(f"  Violations: {report['total_violations']}")

    print(f"\n  --- Protection Breakdown ---")
    for prot, stats in report["protection_breakdown"].items():
        if stats.get("no_scores"):
            print(f"  {prot:15s}: {stats['count']} images (no trust scores)")
            continue
        print(
            f"  {prot:15s}: "
            f"{stats['count']} images, "
            f"mean={stats['mean_trust']:.3f}, "
            f"max={stats['max_trust']:.3f}, "
            f"pass={stats['pass_count']}/{stats['scores']} "
            f"({stats['pass_rate']:.0%})"
        )

    if report["authentic_stats"]:
        stats = report["authentic_stats"]
        print(f"\n  --- Authentic Image Stats ---")
        print(
            f"  {stats['count']} images, "
            f"mean trust={stats['mean_trust']:.3f}, "
            f"FP rate={stats['fp_rate']:.1%} "
            f"({stats['fp_count']}/{stats['count']} scored < 0.40)"
        )

    if report["violations"]:
        print(f"\n  --- Violations (AI images scoring >= threshold) ---")
        for v in report["violations"][:20]:
            print(
                f"  {v['filename']:30s} "
                f"trust={v['overall_trust']:.3f} "
                f"protection={v['protection']:12s} "
                f"deepfake={v.get('deepfake_score', '?')}"
            )
        if len(report["violations"]) > 20:
            print(f"  ... and {len(report['violations']) - 20} more")

    print(f"\n  --- Recommendations ---")
    for rec in report["recommendations"]:
        print(f"  * {rec}")
    print(f"{'=' * 60}")


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace — Constraint Validation Agent"
    )
    parser.add_argument(
        "--results",
        type=Path,
        default=config.RESULTS_BASE / "verification_results.json",
        help="Verification results JSON file",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=config.AI_TRUST_CEILING,
        help=f"Trust ceiling for AI content (default: {config.AI_TRUST_CEILING})",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=config.RESULTS_BASE / "constraint_report.json",
        help="Output constraint report JSON",
    )
    args = parser.parse_args()

    if not args.results.exists():
        print(f"  Results file not found: {args.results}")
        print("  Run verify_corpus.py first.")
        sys.exit(1)

    print("=" * 60)
    print("  Jura Trace — Constraint Validation Agent")
    print("=" * 60)
    print(f"  Threshold: {args.threshold}")
    print(f"  Results: {args.results}")

    results = load_results(args.results)
    report = validate(results, args.threshold)

    # Print report
    print_report(report)

    # Write report JSON
    args.output.parent.mkdir(parents=True, exist_ok=True)
    report["created_at"] = datetime.now(timezone.utc).isoformat()
    args.output.write_text(json.dumps(report, indent=2))
    print(f"\n  Report saved: {args.output}")


if __name__ == "__main__":
    main()
