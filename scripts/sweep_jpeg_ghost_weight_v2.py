#!/usr/bin/env python3
"""
Jura Trace — JPEG Ghost Weight Sweep v2 (direct-call variant for v2 corpus)

Differs from sweep_jpeg_ghost_weight.py in two ways:

1. Calls sidecar detector functions directly via Python import rather than
   over HTTP. Faster (no multipart encoding, no network) and removes the
   dependency on a running uvicorn instance.

2. Defaults to the v2 corpus at models/splice_calibration_150_v2.

All metrics, pass/fail criteria, and output schema match v1 so the doc
update can compare v1 and v2 rows directly.

Usage:
    python scripts/sweep_jpeg_ghost_weight_v2.py
    python scripts/sweep_jpeg_ghost_weight_v2.py --skip-sanity
"""

import argparse
import json
import random
import sys
from pathlib import Path

import numpy as np
from sklearn.metrics import roc_auc_score, roc_curve

# Add sidecar to path so `import app...` resolves
REPO_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(REPO_ROOT / "sidecar"))

from app.services.jpeg_ghost import perform_jpeg_ghost_detection  # noqa: E402
from app.services.ela import perform_ela  # noqa: E402
from app.services.noise_analysis import perform_noise_analysis  # noqa: E402
from app.services.copy_move import perform_copy_move_detection  # noqa: E402

CANDIDATE_WEIGHTS = [0.0, 0.25, 0.5, 0.75, 1.0]
TRUST_THRESHOLD = 0.55


def compute_trust(
    ela_score: float | None,
    noise_score: float | None,
    copy_move_score: float | None,
    jpeg_ghost_score: float | None,
    jpeg_ghost_weight: float,
) -> float | None:
    """Mirror of Rust compute_trust manipulation signal group."""
    signals: list[tuple[float, float]] = []
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
    total = sum(w for _, w in signals)
    return sum(v * w for v, w in signals) / total


def collect_scores(corpus_dir: Path) -> list[dict]:
    labels_path = corpus_dir / "labels.json"
    data = json.loads(labels_path.read_text())
    entries = data["images"]

    results: list[dict] = []
    for i, entry in enumerate(entries):
        label = entry["label"]
        fname = entry["filename"]
        if label == "spliced":
            path = corpus_dir / "spliced" / fname
        else:
            path = corpus_dir / "authentic" / fname
        if not path.exists():
            print(f"  WARNING: missing {path}", file=sys.stderr)
            continue
        img_bytes = path.read_bytes()

        scores: dict[str, float | None] = {
            "ela_score": None,
            "noise_score": None,
            "copy_move_score": None,
            "jpeg_ghost_score": None,
        }
        try:
            scores["jpeg_ghost_score"] = float(perform_jpeg_ghost_detection(img_bytes).score)
        except Exception as exc:
            print(f"\n    jpeg_ghost error on {fname}: {exc}", file=sys.stderr)
        try:
            scores["ela_score"] = float(perform_ela(img_bytes).score)
        except Exception as exc:
            print(f"\n    ela error on {fname}: {exc}", file=sys.stderr)
        try:
            scores["noise_score"] = float(perform_noise_analysis(img_bytes).score)
        except Exception as exc:
            print(f"\n    noise error on {fname}: {exc}", file=sys.stderr)
        try:
            scores["copy_move_score"] = float(perform_copy_move_detection(img_bytes).score)
        except Exception as exc:
            print(f"\n    copy_move error on {fname}: {exc}", file=sys.stderr)

        results.append({"filename": fname, "label": label, **scores})
        if (i + 1) % 10 == 0:
            print(f"  [{i+1}/{len(entries)}]", flush=True)

    return results


def run_sweep(raw_results: list[dict]) -> list[dict]:
    sweep_results = []
    for w in CANDIDATE_WEIGHTS:
        trusts, labels_bin = [], []
        for row in raw_results:
            t = compute_trust(
                row["ela_score"],
                row["noise_score"],
                row["copy_move_score"],
                row["jpeg_ghost_score"],
                jpeg_ghost_weight=w,
            )
            trusts.append(0.5 if t is None else t)
            labels_bin.append(1 if row["label"] == "spliced" else 0)

        trusts_arr = np.array(trusts)
        labels_arr = np.array(labels_bin)
        scores_arr = 1.0 - trusts_arr

        try:
            auc = float(roc_auc_score(labels_arr, scores_arr))
        except Exception:
            auc = float("nan")

        fprs, tprs, _ = roc_curve(labels_arr, scores_arr)

        def tpr_at(target_fpr: float) -> float:
            idx = np.searchsorted(fprs, target_fpr, side="right") - 1
            idx = max(0, min(idx, len(tprs) - 1))
            return float(tprs[idx])

        tpr_fpr5 = tpr_at(0.05)
        tpr_fpr1 = tpr_at(0.01)

        flagged = trusts_arr < TRUST_THRESHOLD
        spliced_mask = labels_arr == 1
        authentic_mask = labels_arr == 0
        n_spliced = int(spliced_mask.sum())
        n_authentic = int(authentic_mask.sum())
        tpr_fixed = float((flagged & spliced_mask).sum()) / n_spliced if n_spliced else 0.0
        fpr_fixed = float((flagged & authentic_mask).sum()) / n_authentic if n_authentic else 0.0

        jg = np.array([r["jpeg_ghost_score"] or 0.0 for r in raw_results])
        jg_spl = jg[spliced_mask]
        jg_auth = jg[authentic_mask]
        cm_mask = np.array([r["filename"].startswith("splice_copy_move") for r in raw_results])
        jg_cm = jg[cm_mask]

        sweep_results.append({
            "weight": w,
            "auc": round(auc, 4),
            "tpr_at_fpr5": round(tpr_fpr5, 4),
            "tpr_at_fpr1": round(tpr_fpr1, 4),
            "tpr_threshold": round(tpr_fixed, 4),
            "fpr_threshold": round(fpr_fixed, 4),
            "n_spliced": n_spliced,
            "n_authentic": n_authentic,
            "jg_spliced_mean": round(float(jg_spl.mean()), 4) if len(jg_spl) else None,
            "jg_spliced_p95": round(float(np.percentile(jg_spl, 95)), 4) if len(jg_spl) else None,
            "jg_authentic_mean": round(float(jg_auth.mean()), 4) if len(jg_auth) else None,
            "jg_authentic_p95": round(float(np.percentile(jg_auth, 95)), 4) if len(jg_auth) else None,
            "jg_copymove_p95": round(float(np.percentile(jg_cm, 95)), 4) if len(jg_cm) else None,
        })
    return sweep_results


def print_table(sweep: list[dict]) -> None:
    baseline = next((r for r in sweep if r["weight"] == 0.0), None)
    print()
    print("JPEG Ghost Weight Sweep — v2 Corpus")
    print("=" * 120)
    hdr = f"{'w':>5} | {'AUC':>6} | {'TPR@FPR5':>9} | {'TPR@FPR1':>9} | {'TPR@0.55':>9} | {'FPR@0.55':>9} | {'ΔFPR':>8} | {'JG-spl-p95':>11} | {'JG-auth-p95':>12} | {'JG-CM-p95':>10}"
    print(hdr)
    print("-" * 120)
    for r in sweep:
        d_fpr = ""
        if baseline and r["weight"] != 0.0:
            delta = r["fpr_threshold"] - baseline["fpr_threshold"]
            sign = "+" if delta >= 0 else ""
            d_fpr = f"{sign}{delta*100:.1f}pp"
        print(
            f"{r['weight']:>5.2f} | {r['auc']:>6.4f} | {r['tpr_at_fpr5']*100:>8.1f}% | "
            f"{r['tpr_at_fpr1']*100:>8.1f}% | {r['tpr_threshold']*100:>8.1f}% | "
            f"{r['fpr_threshold']*100:>8.1f}% | {d_fpr:>8} | "
            f"{r['jg_spliced_p95'] or 0:>11.4f} | {r['jg_authentic_p95'] or 0:>12.4f} | {r['jg_copymove_p95'] or 0:>10.4f}"
        )
    print("=" * 120)


def recommend(sweep: list[dict]) -> tuple[float, str]:
    baseline = next((r for r in sweep if r["weight"] == 0.0), None)
    if baseline is None:
        return 0.5, "No baseline; default 0.5."
    b_tpr = baseline["tpr_at_fpr5"]
    b_fpr = baseline["fpr_threshold"]
    lines = [
        f"Baseline (w=0.0): TPR@FPR5={b_tpr*100:.1f}%, FPR@0.55={b_fpr*100:.1f}%",
        "Pass criteria: A) TPR@FPR5 gain >= 5pp   B) FPR increase <= 2pp",
    ]
    passing = []
    for r in sweep:
        if r["weight"] == 0.0:
            continue
        gain = (r["tpr_at_fpr5"] - b_tpr) * 100
        inc = (r["fpr_threshold"] - b_fpr) * 100
        a = gain >= 5.0
        b = inc <= 2.0
        lines.append(
            f"  w={r['weight']:.2f}: TPR gain={gain:+.1f}pp FPR delta={inc:+.1f}pp "
            f"A={'PASS' if a else 'FAIL'} B={'PASS' if b else 'FAIL'}"
        )
        if a and b:
            passing.append((r["weight"], r["auc"]))
    if not passing:
        lines.append("\nNo weight passes both criteria. Retain w=0.5 (cross-review consensus).")
        return 0.5, "\n".join(lines)
    if any(w == 0.5 for w, _ in passing):
        best_auc = max(a for _, a in passing)
        best_w = next(w for w, a in passing if a == best_auc)
        lines.append(f"\nBest passing: w={best_w:.2f} (AUC={best_auc:.4f}).")
        return best_w, "\n".join(lines)
    sorted_p = sorted(passing, key=lambda x: x[0])
    w, auc = sorted_p[0]
    lines.append(f"\nBest passing: w={w:.2f} (AUC={auc:.4f}).")
    return w, "\n".join(lines)


def sanity_check(authentic_dir: Path, n: int = 200, seed: int = 42) -> dict:
    rng = random.Random(seed)
    images = list(authentic_dir.rglob("*.jpg")) + list(authentic_dir.rglob("*.jpeg"))
    images = [p for p in images if not p.name.startswith(".")]
    rng.shuffle(images)
    sample = images[:n]
    print(f"\n  Sanity check: {len(sample)} authentic JPEGs from training corpus ...")
    scores = []
    for i, p in enumerate(sample):
        try:
            s = float(perform_jpeg_ghost_detection(p.read_bytes()).score)
            scores.append(s)
        except Exception as exc:
            print(f"    skip {p.name}: {exc}", file=sys.stderr)
        if (i + 1) % 50 == 0:
            print(f"    [{i+1}/{len(sample)}]", flush=True)
    if not scores:
        return {"error": "no scores"}
    arr = np.array(scores)
    p95 = float(np.percentile(arr, 95))
    return {
        "n_samples": len(scores),
        "mean": round(float(arr.mean()), 4),
        "p50": round(float(np.median(arr)), 4),
        "p95": round(p95, 4),
        "p99": round(float(np.percentile(arr, 99)), 4),
        "max": round(float(arr.max()), 4),
        "count_above_0_3": int((arr > 0.3).sum()),
        "p95_below_threshold": p95 < 0.3,
    }


def main():
    default_corpus = REPO_ROOT / "models" / "splice_calibration_150_v2"
    default_authentic = Path("/Volumes/Samsung USB/Training Data/corpus/training/authentic")

    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=default_corpus)
    parser.add_argument("--authentic", type=Path, default=default_authentic)
    parser.add_argument("--skip-sanity", action="store_true")
    args = parser.parse_args()

    print(f"Jura Trace — JPEG Ghost Weight Sweep v2 (direct-call)")
    print(f"  Corpus: {args.corpus}")
    if not args.corpus.exists():
        print(f"Error: {args.corpus} not found", file=sys.stderr)
        sys.exit(1)

    print("\n  Collecting detector scores (ELA + noise + copy-move + JPEG ghost) ...")
    raw = collect_scores(args.corpus)
    (args.corpus / "sweep_raw_v2.json").write_text(json.dumps({"raw": raw}, indent=2))
    print(f"\n  Raw scores: {len(raw)} images")

    sweep = run_sweep(raw)
    print_table(sweep)

    rec, reasoning = recommend(sweep)
    print("\nPass/fail:")
    print(reasoning)
    print(f"\nRECOMMENDED WEIGHT: {rec}")

    sanity = None
    if not args.skip_sanity and args.authentic.exists():
        sanity = sanity_check(args.authentic)
        print("\nSanity check:")
        for k, v in sanity.items():
            print(f"  {k}: {v}")

    out = {
        "corpus_version": "v2",
        "sweep": sweep,
        "recommended_weight": rec,
        "reasoning": reasoning,
        "sanity_check": sanity,
    }
    (args.corpus / "weight_sweep_final_v2.json").write_text(json.dumps(out, indent=2))
    print(f"\nFull results: {args.corpus / 'weight_sweep_final_v2.json'}")


if __name__ == "__main__":
    main()
