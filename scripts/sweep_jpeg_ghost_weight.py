#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — JPEG Ghost Weight Sweep (S28-FU9)

Runs the JPEG Ghost weight calibration sweep described in
docs/calibration/s28-jpeg-ghost-weight.md Section 3.4.

For each candidate weight w in {0.0, 0.25, 0.5, 0.75, 1.0}:
  - Scores all images in models/splice_calibration_150/ through the sidecar's
    JPEG ghost, ELA, noise, and copy-move detectors.
  - Recomputes overall_trust using a Python reimplementation of the Rust
    compute_trust weighted-average formula with the candidate w substituted.
  - Computes TPR (splice recall) and FPR (authentic false alarm) at the 0.55
    trust threshold, plus AUC-ROC and score distributions.

Usage:
    python scripts/sweep_jpeg_ghost_weight.py
    python scripts/sweep_jpeg_ghost_weight.py --corpus models/splice_calibration_150
    python scripts/sweep_jpeg_ghost_weight.py --sidecar http://127.0.0.1:8200
"""

import argparse
import json
import sys
import time
import io
import http.client
import mimetypes
import os
from pathlib import Path

import numpy as np
from sklearn.metrics import roc_auc_score

SIDECAR_DEFAULT = "http://127.0.0.1:8200"

# Weights to sweep
CANDIDATE_WEIGHTS = [0.0, 0.25, 0.5, 0.75, 1.0]

# Trust threshold below which an image is considered manipulated / flagged
TRUST_THRESHOLD = 0.55

# ─────────────────────────────────────────────────────────────────────────────
# Sidecar client
# ─────────────────────────────────────────────────────────────────────────────

def check_sidecar(base_url: str) -> bool:
    try:
        from urllib.request import urlopen, Request
        req = Request(f"{base_url}/health", headers={"Accept": "application/json"})
        with urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read())
            return data.get("status") == "ok"
    except Exception:
        return False


def run_detector(base_url: str, endpoint: str, image_path: Path) -> dict | None:
    from urllib.request import urlopen, Request
    url = f"{base_url}{endpoint}"
    boundary = "----JuraGhostSweep"
    filename = image_path.name
    content_type = mimetypes.guess_type(str(image_path))[0] or "image/jpeg"

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
        from urllib.request import urlopen
        with urlopen(req, timeout=90) as resp:
            return json.loads(resp.read())
    except Exception as e:
        return {"error": str(e)}


# ─────────────────────────────────────────────────────────────────────────────
# compute_trust reimplementation (matches Rust lib.rs weighted average)
# ─────────────────────────────────────────────────────────────────────────────

def compute_trust(
    ela_score: float | None,
    noise_score: float | None,
    copy_move_score: float | None,
    jpeg_ghost_score: float | None,
    jpeg_ghost_weight: float,
) -> float | None:
    """
    Reimplements the manipulation_trust weighted-average from compute_trust
    in src-tauri/src/lib.rs, substituting jpeg_ghost_weight for the
    production constant 0.5.

    Returns a trust value in [0, 1] or None if all signals are absent.

    Note: this intentionally omits deepfake and other signals that are not
    run in the splice sweep — this is correct because we are isolating the
    JPEG ghost weight effect on the manipulation signal group. The final
    overall_trust also incorporates deepfake and EXIF signals, but those
    are constant across the weight sweep and therefore do not affect the
    relative ordering.
    """
    signals: list[tuple[float, float]] = []  # (trust, weight)

    if ela_score is not None:
        signals.append((1.0 - ela_score, 1.0))
    if noise_score is not None:
        signals.append((1.0 - noise_score, 1.0))
    if copy_move_score is not None:
        signals.append((1.0 - copy_move_score, 1.0))
    if jpeg_ghost_score is not None:
        signals.append((1.0 - jpeg_ghost_score, jpeg_ghost_weight))

    if not signals:
        return None

    if len(signals) == 1:
        return signals[0][0]

    total_weight = sum(w for _, w in signals)
    weighted_avg = sum(v * w for v, w in signals) / total_weight
    return weighted_avg


# ─────────────────────────────────────────────────────────────────────────────
# Sweep
# ─────────────────────────────────────────────────────────────────────────────

def load_labels(corpus_dir: Path) -> dict[str, str]:
    """Returns {filename_stem: label} from labels.json."""
    labels_path = corpus_dir / "labels.json"
    if not labels_path.exists():
        print(f"Error: labels.json not found at {labels_path}", file=sys.stderr)
        sys.exit(1)
    data = json.loads(labels_path.read_text())
    return {entry["filename"]: entry["label"] for entry in data["images"]}


def collect_detector_scores(
    corpus_dir: Path,
    base_url: str,
    labels: dict[str, str],
) -> list[dict]:
    """
    Run ELA, noise, copy-move, and JPEG Ghost detectors against all corpus
    images. Returns a list of per-image result dicts with raw scores.
    """
    spliced_dir = corpus_dir / "spliced"
    authentic_dir = corpus_dir / "authentic"

    images: list[tuple[Path, str]] = []
    for fname, label in labels.items():
        if label == "spliced":
            p = spliced_dir / fname
        else:
            p = authentic_dir / fname
        if p.exists():
            images.append((p, label))
        else:
            print(f"  WARNING: {fname} not found", file=sys.stderr)

    print(f"\n  Collecting detector scores for {len(images)} images ...")
    print(f"  (This may take a few minutes — JPEG Ghost sweeps 10 quality levels per image)\n")

    results = []
    for i, (img_path, label) in enumerate(images):
        print(f"  [{i+1}/{len(images)}] {img_path.name} ({label}) ...", end=" ", flush=True)

        scores: dict[str, float | None] = {
            "ela": None,
            "noise": None,
            "copy_move": None,
            "jpeg_ghost": None,
        }

        for det_name, endpoint in [
            ("ela", "/forensics/ela"),
            ("noise", "/forensics/noise"),
            ("copy_move", "/forensics/copy-move"),
            ("jpeg_ghost", "/forensics/jpeg-ghost"),
        ]:
            r = run_detector(base_url, endpoint, img_path)
            if r and "error" not in r:
                scores[det_name] = r.get("score")
            else:
                err = r.get("error", "no response") if r else "no response"
                print(f"\n    DETECTOR ERROR ({det_name}): {err}", file=sys.stderr)

        results.append({
            "filename": img_path.name,
            "label": label,
            **{f"{k}_score": v for k, v in scores.items()},
        })
        print("ok")
        time.sleep(0.05)

    return results


def run_sweep(raw_results: list[dict]) -> list[dict]:
    """
    For each candidate weight, compute trust scores and metrics.
    Returns list of per-weight metric dicts.
    """
    sweep_results = []

    for w in CANDIDATE_WEIGHTS:
        trusts: list[float] = []
        labels_bin: list[int] = []  # 1 = spliced (positive), 0 = authentic

        for row in raw_results:
            t = compute_trust(
                row["ela_score"],
                row["noise_score"],
                row["copy_move_score"],
                row["jpeg_ghost_score"],
                jpeg_ghost_weight=w,
            )
            if t is None:
                t = 0.5  # neutral fallback (matches Rust behaviour)
            trusts.append(t)
            labels_bin.append(1 if row["label"] == "spliced" else 0)

        trusts_arr = np.array(trusts, dtype=float)
        labels_arr = np.array(labels_bin, dtype=int)

        # For ROC: a low trust = suspicious = positive prediction.
        # Invert trust so high score = more likely spliced.
        scores_arr = 1.0 - trusts_arr

        # Compute ROC AUC
        try:
            auc = float(roc_auc_score(labels_arr, scores_arr))
        except Exception:
            auc = float("nan")

        # TPR at FPR = 5% and FPR = 1%
        from sklearn.metrics import roc_curve
        fprs, tprs, thresholds = roc_curve(labels_arr, scores_arr)

        def tpr_at_fpr(target_fpr: float) -> float:
            # Find largest FPR <= target, take corresponding TPR
            idx = np.searchsorted(fprs, target_fpr, side="right") - 1
            idx = max(0, min(idx, len(tprs) - 1))
            return float(tprs[idx])

        tpr_fpr5 = tpr_at_fpr(0.05)
        tpr_fpr1 = tpr_at_fpr(0.01)

        # TPR and FPR at fixed threshold 0.55 (trust < 0.55 = flagged)
        flagged = trusts_arr < TRUST_THRESHOLD
        spliced_mask = labels_arr == 1
        authentic_mask = labels_arr == 0

        n_spliced = int(spliced_mask.sum())
        n_authentic = int(authentic_mask.sum())
        tpr_fixed = float((flagged & spliced_mask).sum()) / n_spliced if n_spliced > 0 else 0.0
        fpr_fixed = float((flagged & authentic_mask).sum()) / n_authentic if n_authentic > 0 else 0.0

        # Score distributions per class
        jg_scores = np.array([
            r["jpeg_ghost_score"] if r["jpeg_ghost_score"] is not None else 0.0
            for r in raw_results
        ])
        jg_spliced = jg_scores[spliced_mask]
        jg_authentic = jg_scores[authentic_mask]

        # Break out copy-move for sanity check
        copy_move_mask = np.array([r["filename"].startswith("splice_copy_move") for r in raw_results])
        jg_copymove = jg_scores[copy_move_mask]

        sweep_results.append({
            "weight": w,
            "auc": round(auc, 4),
            "tpr_at_fpr5": round(tpr_fpr5, 4),
            "tpr_at_fpr1": round(tpr_fpr1, 4),
            "tpr_threshold": round(tpr_fixed, 4),
            "fpr_threshold": round(fpr_fixed, 4),
            "n_spliced": n_spliced,
            "n_authentic": n_authentic,
            "jg_spliced_mean": round(float(np.mean(jg_spliced)), 4) if len(jg_spliced) > 0 else None,
            "jg_spliced_p50": round(float(np.median(jg_spliced)), 4) if len(jg_spliced) > 0 else None,
            "jg_spliced_p95": round(float(np.percentile(jg_spliced, 95)), 4) if len(jg_spliced) > 0 else None,
            "jg_authentic_mean": round(float(np.mean(jg_authentic)), 4) if len(jg_authentic) > 0 else None,
            "jg_authentic_p50": round(float(np.median(jg_authentic)), 4) if len(jg_authentic) > 0 else None,
            "jg_authentic_p95": round(float(np.percentile(jg_authentic, 95)), 4) if len(jg_authentic) > 0 else None,
            "jg_copymove_mean": round(float(np.mean(jg_copymove)), 4) if len(jg_copymove) > 0 else None,
            "jg_copymove_p95": round(float(np.percentile(jg_copymove, 95)), 4) if len(jg_copymove) > 0 else None,
        })

    return sweep_results


def print_sweep_table(sweep_results: list[dict]) -> None:
    baseline = next((r for r in sweep_results if r["weight"] == 0.0), None)
    header = (
        f"{'Weight':>8} | {'AUC':>6} | {'TPR@FPR5':>9} | {'TPR@FPR1':>9} | "
        f"{'TPR@0.55':>9} | {'FPR@0.55':>9} | {'ΔFPRvs0.0':>10} | "
        f"{'JG-auth-p95':>12} | {'JG-splice-p95':>14} | {'JG-CM-p95':>10}"
    )
    print()
    print("JPEG Ghost Weight Sweep Results")
    print("=" * len(header))
    print(header)
    print("-" * len(header))

    for r in sweep_results:
        d_fpr = ""
        if baseline and r["weight"] != 0.0:
            delta = r["fpr_threshold"] - baseline["fpr_threshold"]
            sign = "+" if delta >= 0 else ""
            d_fpr = f"{sign}{delta*100:.1f}pp"

        print(
            f"{r['weight']:>8.2f} | {r['auc']:>6.4f} | {r['tpr_at_fpr5']*100:>8.1f}% | "
            f"{r['tpr_at_fpr1']*100:>8.1f}% | {r['tpr_threshold']*100:>8.1f}% | "
            f"{r['fpr_threshold']*100:>8.1f}% | {d_fpr:>10} | "
            f"{r['jg_authentic_p95'] or 0:>12.4f} | "
            f"{r['jg_spliced_p95'] or 0:>14.4f} | "
            f"{r['jg_copymove_p95'] or 0:>10.4f}"
        )
    print("=" * len(header))


def apply_pass_fail(sweep_results: list[dict]) -> tuple[float, str]:
    """
    Apply Section 3.5 pass/fail criteria.
    Returns (recommended_weight, reasoning).
    """
    baseline = next((r for r in sweep_results if r["weight"] == 0.0), None)
    if baseline is None:
        return 0.5, "Cannot determine baseline; defaulting to 0.5."

    b_tpr = baseline["tpr_at_fpr5"]
    b_fpr = baseline["fpr_threshold"]

    reasoning_lines = [
        f"Baseline (w=0.0): TPR@FPR5={b_tpr*100:.1f}%, FPR@0.55={b_fpr*100:.1f}%",
        "Pass criteria (Section 3.5):",
        "  A) TPR@FPR5 improves by >= 5pp vs w=0.0",
        "  B) Authentic FPR increase <= 2pp vs w=0.0",
    ]

    passing_weights = []
    for r in sweep_results:
        if r["weight"] == 0.0:
            continue
        tpr_improvement = (r["tpr_at_fpr5"] - b_tpr) * 100
        fpr_increase = (r["fpr_threshold"] - b_fpr) * 100
        passes_a = tpr_improvement >= 5.0
        passes_b = fpr_increase <= 2.0
        reasoning_lines.append(
            f"  w={r['weight']:.2f}: TPR gain={tpr_improvement:+.1f}pp "
            f"FPR delta={fpr_increase:+.1f}pp "
            f"A={'PASS' if passes_a else 'FAIL'} B={'PASS' if passes_b else 'FAIL'}"
        )
        if passes_a and passes_b:
            passing_weights.append((r["weight"], r["auc"]))

    if not passing_weights:
        # No weight passes both criteria — default to lowest-FPR option
        # that still improves TPR at all
        reasoning_lines.append("\nNo weight passes both A+B criteria.")
        # Check if any weight improves TPR at all
        any_tpr_gain = [
            r for r in sweep_results
            if r["weight"] > 0.0 and r["tpr_at_fpr5"] > b_tpr
        ]
        if any_tpr_gain:
            best = min(any_tpr_gain, key=lambda r: r["fpr_threshold"])
            reasoning_lines.append(
                f"Best partial: w={best['weight']:.2f} "
                f"(TPR gain={(best['tpr_at_fpr5']-b_tpr)*100:+.1f}pp, "
                f"FPR delta={(best['fpr_threshold']-b_fpr)*100:+.1f}pp)."
            )
            reasoning_lines.append("RECOMMENDATION: Retain w=0.5 (cross-review consensus; no clear empirical improvement).")
            return 0.5, "\n".join(reasoning_lines)
        else:
            reasoning_lines.append("JPEG Ghost adds no TPR improvement on this corpus. Recommend w=0.25 (conservative retention).")
            return 0.25, "\n".join(reasoning_lines)

    # Among passing weights, prefer w=0.5 if it passes (cross-review consensus)
    if any(w == 0.5 for w, _ in passing_weights):
        # Check whether a higher weight is strictly better without breaching FPR
        best_auc = max(auc for _, auc in passing_weights)
        best_w = next(w for w, auc in passing_weights if auc == best_auc)
        if best_w == 0.5:
            reasoning_lines.append(f"\nw=0.5 passes both criteria with best AUC ({best_auc:.4f}). Retain 0.5.")
            return 0.5, "\n".join(reasoning_lines)
        else:
            # Higher weight gives better AUC and still passes
            reasoning_lines.append(
                f"\nw=0.5 passes, but w={best_w:.2f} gives better AUC ({best_auc:.4f}). "
                f"Recommend upgrading to w={best_w:.2f}."
            )
            return best_w, "\n".join(reasoning_lines)

    # w=0.5 doesn't pass — find best passing weight
    # Prefer 0.25 over higher weights if both pass (conservative)
    sorted_passing = sorted(passing_weights, key=lambda x: x[0])
    recommended_w, recommended_auc = sorted_passing[0]
    reasoning_lines.append(
        f"\nBest passing weight: w={recommended_w:.2f} (AUC={recommended_auc:.4f}). "
        "Recommendation: patch lib.rs."
    )
    return recommended_w, "\n".join(reasoning_lines)


# ─────────────────────────────────────────────────────────────────────────────
# Sanity check: authentic training corpus p95
# ─────────────────────────────────────────────────────────────────────────────

def run_sanity_check(
    authentic_corpus_dir: Path,
    base_url: str,
    n_samples: int = 200,
    seed: int = 42,
) -> dict:
    """
    Section 3.6: confirm jpeg_ghost_score p95 < 0.3 on a 200-image sample
    from the authentic training corpus.
    """
    import random
    rng = random.Random(seed)

    images = list(authentic_corpus_dir.rglob("*.jpg")) + list(authentic_corpus_dir.rglob("*.jpeg"))
    # Restrict to JPEG files only (PNG would always score 0.0 — misleading)
    images = [p for p in images if not p.name.startswith(".")]
    rng.shuffle(images)
    sample = images[:n_samples]

    print(f"\n  Sanity check: sampling {len(sample)} authentic JPEG images from training corpus ...")
    jg_scores = []
    for i, img_path in enumerate(sample):
        print(f"  [{i+1}/{len(sample)}] {img_path.name} ...", end=" ", flush=True)
        r = run_detector(base_url, "/forensics/jpeg-ghost", img_path)
        if r and "error" not in r:
            score = r.get("score", 0.0)
            jg_scores.append(float(score))
            print(f"score={score:.4f}")
        else:
            print(f"error: {r}")
        time.sleep(0.05)

    if not jg_scores:
        return {"error": "No scores collected", "p95_ok": False}

    arr = np.array(jg_scores)
    p95 = float(np.percentile(arr, 95))
    p99 = float(np.percentile(arr, 99))
    mean = float(np.mean(arr))

    return {
        "n_samples": len(jg_scores),
        "mean": round(mean, 4),
        "p50": round(float(np.median(arr)), 4),
        "p95": round(p95, 4),
        "p99": round(p99, 4),
        "p95_below_threshold": p95 < 0.3,
        "suspicious_threshold": 0.3,
    }


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main():
    repo_root = Path(__file__).parent.parent
    default_corpus = repo_root / "models" / "splice_calibration_150"
    default_authentic = Path("/Volumes/MAC SSD/Training Data/corpus/training/authentic")

    parser = argparse.ArgumentParser(description="JPEG Ghost weight sweep (S28-FU9)")
    parser.add_argument("--corpus", type=Path, default=default_corpus)
    parser.add_argument("--authentic", type=Path, default=default_authentic,
                        help="Authentic training corpus for sanity check (Section 3.6)")
    parser.add_argument("--sidecar", type=str, default=SIDECAR_DEFAULT)
    parser.add_argument("--skip-sanity", action="store_true",
                        help="Skip the 200-image authentic corpus sanity check")
    parser.add_argument("--output", type=Path, default=None,
                        help="Save raw scores to JSON (default: corpus/sweep_results.json)")
    args = parser.parse_args()

    print("Jura Trace — JPEG Ghost Weight Sweep (S28-FU9)")
    print(f"  Corpus:  {args.corpus}")
    print(f"  Sidecar: {args.sidecar}")

    if not check_sidecar(args.sidecar):
        print(f"\nError: sidecar not responding at {args.sidecar}", file=sys.stderr)
        print("Start it: cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200", file=sys.stderr)
        sys.exit(1)
    print("  Sidecar: connected\n")

    if not args.corpus.exists():
        print(f"Error: corpus not found: {args.corpus}", file=sys.stderr)
        print("Run: python scripts/build_splice_corpus.py", file=sys.stderr)
        sys.exit(1)

    # Load labels
    labels = load_labels(args.corpus)
    n_spliced = sum(1 for v in labels.values() if v == "spliced")
    n_authentic = sum(1 for v in labels.values() if v == "authentic")
    print(f"  Labels: {len(labels)} total ({n_spliced} spliced, {n_authentic} authentic)")

    # Collect raw detector scores once (no per-weight sidecar calls needed)
    raw_results = collect_detector_scores(args.corpus, args.sidecar, labels)

    # Save raw scores
    output_path = args.output or (args.corpus / "sweep_results.json")
    output_path.write_text(json.dumps({"raw": raw_results}, indent=2))
    print(f"\n  Raw scores saved: {output_path}")

    # Run weight sweep
    print("\n" + "=" * 80)
    print("Running weight sweep ...")
    sweep_results = run_sweep(raw_results)
    print_sweep_table(sweep_results)

    # Apply pass/fail
    recommended_w, reasoning = apply_pass_fail(sweep_results)
    print(f"\nPass/fail analysis:")
    print(reasoning)
    print(f"\nRECOMMENDED WEIGHT: {recommended_w}")

    # Sanity check (Section 3.6)
    sanity = None
    if not args.skip_sanity:
        if args.authentic.exists():
            sanity = run_sanity_check(args.authentic, args.sidecar)
            print("\nSanity check (authentic training corpus, Section 3.6):")
            print(f"  n_samples:   {sanity['n_samples']}")
            print(f"  mean score:  {sanity['mean']:.4f}")
            print(f"  p50:         {sanity['p50']:.4f}")
            print(f"  p95:         {sanity['p95']:.4f} {'< 0.3 PASS' if sanity['p95_below_threshold'] else '>= 0.3 FAIL'}")
            print(f"  p99:         {sanity['p99']:.4f}")
        else:
            print(f"\n  Sanity check skipped — authentic corpus not mounted at {args.authentic}")
    else:
        print("\n  Sanity check skipped (--skip-sanity)")

    # Save full results
    final = {
        "sweep": sweep_results,
        "recommended_weight": recommended_w,
        "reasoning": reasoning,
        "sanity_check": sanity,
    }
    full_output = args.corpus / "weight_sweep_final.json"
    full_output.write_text(json.dumps(final, indent=2))
    print(f"\nFull results saved: {full_output}")

    return recommended_w, sweep_results, sanity


if __name__ == "__main__":
    main()
