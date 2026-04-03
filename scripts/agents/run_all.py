#!/usr/bin/env python3
"""
Jura Trace — Corpus Agent Orchestrator

Runs all corpus agents in sequence:
  1. Crawl AI-generated images
  2. Crawl authentic images
  3. Apply protections (fingerprint, watermark, C2PA, combined)
  4. Verify all corpus images via REST API
  5. Validate the AI+protection constraint (trust < 0.70)

Prerequisites:
  - Jura Trace app running (REST API on port 8300)
  - Python ML sidecar running (port 8200)
  - JURA_API_KEY environment variable set
  - 'datasets' Python library installed

Usage:
    python -m scripts.agents.run_all
    python -m scripts.agents.run_all --quick
    python -m scripts.agents.run_all --skip-crawl
    python -m scripts.agents.run_all --skip-protect
    python -m scripts.agents.run_all --max-crawl 50 --max-protect 20 --max-verify 100
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config
from scripts.agents.api_client import check_api_health, check_sidecar_health


def run_agent(module: str, args: list[str], label: str) -> bool:
    """Run an agent module as a subprocess."""
    print(f"\n{'─' * 60}")
    print(f"  Stage: {label}")
    print(f"{'─' * 60}")

    cmd = [sys.executable, "-m", module] + args
    result = subprocess.run(cmd, cwd=str(config._ROOT))
    return result.returncode == 0


def main():
    parser = argparse.ArgumentParser(
        description="Jura Trace — Corpus Agent Orchestrator"
    )
    parser.add_argument(
        "--quick",
        action="store_true",
        help="Quick mode: fewer images, quick verify mode",
    )
    parser.add_argument("--skip-crawl", action="store_true", help="Skip crawling stage")
    parser.add_argument("--skip-protect", action="store_true", help="Skip protection stage")
    parser.add_argument("--skip-verify", action="store_true", help="Skip verification stage")
    parser.add_argument("--max-crawl", type=int, default=None, help="Max images per crawl source")
    parser.add_argument("--max-protect", type=int, default=None, help="Max images to protect")
    parser.add_argument("--max-verify", type=int, default=None, help="Max images to verify")
    parser.add_argument(
        "--threshold",
        type=float,
        default=config.AI_TRUST_CEILING,
        help=f"AI trust ceiling (default: {config.AI_TRUST_CEILING})",
    )
    parser.add_argument(
        "--ai-sources",
        default="diffusiondb,cifake",
        help="AI image sources (default: diffusiondb,cifake)",
    )
    parser.add_argument(
        "--authentic-sources",
        default="wikimedia,openimages",
        help="Authentic image sources (default: wikimedia,openimages)",
    )
    args = parser.parse_args()

    # Quick mode defaults
    if args.quick:
        args.max_crawl = args.max_crawl or 20
        args.max_protect = args.max_protect or 10
        args.max_verify = args.max_verify or 50
        verify_mode = "quick"
    else:
        args.max_crawl = args.max_crawl or 100
        args.max_protect = args.max_protect or 50
        args.max_verify = args.max_verify or 200
        verify_mode = "standard"

    print("=" * 60)
    print("  Jura Trace — Corpus Agent Orchestrator")
    print("=" * 60)
    print(f"  Mode: {'quick' if args.quick else 'standard'}")
    print(f"  Crawl: {args.max_crawl}/source | Protect: {args.max_protect} | Verify: {args.max_verify}")
    print(f"  AI trust ceiling: {args.threshold}")

    # ── Preflight checks ─────────────────────────────────────────────
    print(f"\n  Preflight checks...")

    api = check_api_health()
    if api is None:
        print("  FAIL: REST API (port 8300) is not available.")
        print("  Start the Tauri app with the API server enabled.")
        sys.exit(1)
    print(f"  REST API: online (v{api.get('version', '?')})")

    sidecar = check_sidecar_health()
    if sidecar is None:
        print("  WARN: Sidecar (port 8200) is offline. Forensic analysis will be degraded.")
    else:
        print(f"  Sidecar: online")

    if not config.JURA_API_KEY:
        print("  WARN: JURA_API_KEY not set. API calls may fail with 401.")

    start = time.time()
    stages_passed = 0
    stages_total = 0

    # ── Stage 1: Crawl ───────────────────────────────────────────────
    if not args.skip_crawl:
        stages_total += 2

        ok = run_agent(
            "scripts.agents.crawl_ai_images",
            ["--sources", args.ai_sources, "--max", str(args.max_crawl)],
            "Crawl AI-Generated Images",
        )
        if ok:
            stages_passed += 1

        ok = run_agent(
            "scripts.agents.crawl_authentic_images",
            ["--sources", args.authentic_sources, "--max", str(args.max_crawl)],
            "Crawl Authentic Images",
        )
        if ok:
            stages_passed += 1

    # ── Stage 2: Protect ─────────────────────────────────────────────
    if not args.skip_protect:
        stages_total += 1
        ok = run_agent(
            "scripts.agents.apply_protections",
            ["--corpus", str(config.CORPUS_AI), "--max", str(args.max_protect)],
            "Apply Protections to AI Images",
        )
        if ok:
            stages_passed += 1

    # ── Stage 3: Verify ──────────────────────────────────────────────
    if not args.skip_verify:
        stages_total += 1
        corpus_dirs = [
            str(config.CORPUS_AI),
            str(config.CORPUS_AUTHENTIC),
            str(config.PROTECTED_BASE),
        ]
        ok = run_agent(
            "scripts.agents.verify_corpus",
            ["--corpus"] + corpus_dirs + ["--mode", verify_mode, "--max", str(args.max_verify)],
            "Verify Corpus Images",
        )
        if ok:
            stages_passed += 1

    # ── Stage 4: Validate Constraint ─────────────────────────────────
    stages_total += 1
    results_file = config.RESULTS_BASE / "verification_results.json"
    if results_file.exists():
        ok = run_agent(
            "scripts.agents.validate_constraint",
            ["--results", str(results_file), "--threshold", str(args.threshold)],
            "Validate AI+Protection Constraint",
        )
        if ok:
            stages_passed += 1
    else:
        print(f"\n  SKIP: No verification results found at {results_file}")

    # ── Summary ──────────────────────────────────────────────────────
    elapsed = time.time() - start
    print(f"\n{'=' * 60}")
    print(f"  ORCHESTRATION COMPLETE")
    print(f"  Stages: {stages_passed}/{stages_total} passed")
    print(f"  Time: {elapsed:.1f}s")
    print(f"{'=' * 60}")

    # Read constraint report if available
    report_path = config.RESULTS_BASE / "constraint_report.json"
    if report_path.exists():
        import json
        report = json.loads(report_path.read_text())
        constraint_status = "PASS" if report.get("overall_pass") else "FAIL"
        print(f"\n  CONSTRAINT: {constraint_status}")
        print(f"  AI images tested: {report.get('total_ai_images', 0)}")
        print(f"  Violations: {report.get('total_violations', 0)}")
        for rec in report.get("recommendations", []):
            print(f"  * {rec}")

    sys.exit(0 if stages_passed == stages_total else 1)


if __name__ == "__main__":
    main()
