#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Train the audio deepfake detection ensemble (Sprint 35).

Two-stage ensemble:

    Stage 1  MFCC (160-dim) → GradientBoostingClassifier  (~20 ms inference)
    Stage 2  Wav2Vec2-Base embeddings (768-dim) → LogisticRegression  (~150 ms)
    Ensemble  final_score = 0.3 × stage1 + 0.7 × stage2

When LightGBM becomes available (``pip install lightgbm``) swap
``GradientBoostingClassifier`` for ``LGBMClassifier`` — the joblib
interface is identical.

Usage:
    python scripts/train_audio_deepfake.py \\
        --authentic-dir /path/to/authentic/clips \\
        --synthetic-dir /path/to/synthetic/clips \\
        --output-dir models/

Expected corpus layout (flat or sub-directories, any depth):
    <authentic-dir>/
        librispeech_train-clean-100/*.flac
        ...
    <synthetic-dir>/
        asvspoof2019_LA_train/flac/*.flac
        ...

The script expects the corpus to exist at runtime.  When the Sprint 35
LibriSpeech / ASVspoof clips arrive, running this script once is
sufficient to produce the two probe files.
"""

import argparse
import hashlib
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# Add sidecar to path for shared feature-extraction helpers.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

import numpy as np

# ── Audio file discovery ──────────────────────────────────────────────────────

AUDIO_EXTENSIONS = {".wav", ".mp3", ".flac", ".ogg", ".aac", ".m4a", ".aiff", ".opus"}


def collect_audio_files(directory: str) -> list[Path]:
    """Walk a directory and return all audio file paths (sorted)."""
    d = Path(directory)
    if not d.exists():
        print(f"  WARNING: directory not found: {d}")
        return []
    return sorted(
        f for f in d.rglob("*")
        if f.is_file() and f.suffix.lower() in AUDIO_EXTENSIONS
    )


# ── Feature extraction ────────────────────────────────────────────────────────

def extract_all_mfcc(
    files: list[Path], label: int
) -> tuple[list[np.ndarray], list[int], list[str]]:
    """Extract MFCC vectors for all audio files.

    Returns (features, labels, filenames).
    Files for which extraction fails are skipped with a WARNING.
    """
    from app.services.audio_deepfake import extract_mfcc_features

    features: list[np.ndarray] = []
    labels: list[int] = []
    filenames: list[str] = []

    for i, path in enumerate(files):
        print(f"    [{i + 1}/{len(files)}] MFCC {path.name} ...", end=" ", flush=True)
        vec = extract_mfcc_features(str(path))
        if vec is not None:
            features.append(vec)
            labels.append(label)
            filenames.append(path.name)
            print("OK")
        else:
            print("SKIP")

    return features, labels, filenames


def extract_all_wav2vec2(
    files: list[Path], label: int
) -> tuple[list[np.ndarray], list[int], list[str]]:
    """Extract Wav2Vec2 embeddings for all audio files.

    Gracefully skips if torch/transformers are not installed or the model
    has not been downloaded (prints a one-time WARNING and returns empty lists).
    """
    from app.services.audio_deepfake import extract_wav2vec2_embedding

    # Probe with first file to detect unavailability early.
    if files:
        test_emb = extract_wav2vec2_embedding(str(files[0]))
        if test_emb is None:
            print(
                "  WARNING: Wav2Vec2 model unavailable — Stage 2 will be skipped.\n"
                "  To enable: ensure transformers + torch are installed and that\n"
                "  facebook/wav2vec2-base has been downloaded (runs automatically\n"
                "  on first call when internet is available)."
            )
            return [], [], []

    embeddings: list[np.ndarray] = []
    labels: list[int] = []
    filenames: list[str] = []

    for i, path in enumerate(files):
        print(f"    [{i + 1}/{len(files)}] Wav2Vec2 {path.name} ...", end=" ", flush=True)
        emb = extract_wav2vec2_embedding(str(path))
        if emb is not None:
            embeddings.append(emb)
            labels.append(label)
            filenames.append(path.name)
            print("OK")
        else:
            print("SKIP")

    return embeddings, labels, filenames


# ── Training helpers ──────────────────────────────────────────────────────────

def _build_stage1_classifier():
    """Return the Stage 1 classifier (GBM or LightGBM if available)."""
    try:
        from lightgbm import LGBMClassifier  # type: ignore[import-untyped]
        print("  Using LightGBM (LGBMClassifier) for Stage 1.")
        return LGBMClassifier(
            n_estimators=400,
            max_depth=6,
            learning_rate=0.05,
            num_leaves=63,
            subsample=0.8,
            colsample_bytree=0.8,
            random_state=42,
            n_jobs=-1,
        )
    except ImportError:
        from sklearn.ensemble import GradientBoostingClassifier
        print(
            "  LightGBM not installed — using sklearn GradientBoostingClassifier.\n"
            "  Install lightgbm for ~3× faster training and better performance."
        )
        return GradientBoostingClassifier(
            n_estimators=300,
            max_depth=5,
            learning_rate=0.05,
            subsample=0.8,
            random_state=42,
        )


def _build_stage2_classifier():
    from sklearn.linear_model import LogisticRegression
    return LogisticRegression(
        C=1.0,
        max_iter=1000,
        solver="lbfgs",
        random_state=42,
        n_jobs=-1,
    )


def _evaluate(clf, X_test: np.ndarray, y_test: np.ndarray, stage_name: str) -> dict:
    """Compute AUC-ROC, accuracy, precision, recall, F1 for a trained classifier."""
    from sklearn.metrics import (
        roc_auc_score,
        accuracy_score,
        precision_score,
        recall_score,
        f1_score,
    )

    y_pred = clf.predict(X_test)
    y_prob = clf.predict_proba(X_test)[:, 1]

    metrics = {
        "stage": stage_name,
        "auc_roc": round(float(roc_auc_score(y_test, y_prob)), 4),
        "accuracy": round(float(accuracy_score(y_test, y_pred)), 4),
        "precision": round(float(precision_score(y_test, y_pred, zero_division=0)), 4),
        "recall": round(float(recall_score(y_test, y_pred, zero_division=0)), 4),
        "f1": round(float(f1_score(y_test, y_pred, zero_division=0)), 4),
        "n_test": int(len(y_test)),
        "n_synthetic_test": int(np.sum(y_test == 1)),
        "n_authentic_test": int(np.sum(y_test == 0)),
    }
    return metrics


def _ensemble_evaluate(
    clf1, clf2,
    X1_test: np.ndarray, X2_test: np.ndarray, y_test: np.ndarray,
    w1: float = 0.3, w2: float = 0.7,
) -> dict:
    """Evaluate the weighted ensemble assuming the same test samples in same order."""
    from sklearn.metrics import roc_auc_score, accuracy_score

    p1 = clf1.predict_proba(X1_test)[:, 1]
    p2 = clf2.predict_proba(X2_test)[:, 1]
    ensemble = w1 * p1 + w2 * p2
    y_pred = (ensemble >= 0.5).astype(int)
    return {
        "stage": "ensemble",
        "auc_roc": round(float(roc_auc_score(y_test, ensemble)), 4),
        "accuracy": round(float(accuracy_score(y_test, y_pred)), 4),
        "n_test": int(len(y_test)),
    }


def _sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _print_comparison_table(results: list[dict]) -> None:
    print("\n── Evaluation summary ──────────────────────────────────────────")
    header = f"{'Stage':<14}{'AUC-ROC':>9}{'Accuracy':>10}{'F1':>8}{'N test':>8}"
    print(header)
    print("─" * len(header))
    for r in results:
        auc = f"{r.get('auc_roc', '—'):.4f}" if isinstance(r.get("auc_roc"), float) else "—"
        acc = f"{r.get('accuracy', '—'):.4f}" if isinstance(r.get("accuracy"), float) else "—"
        f1  = f"{r.get('f1', '—'):.4f}"       if isinstance(r.get("f1"), float) else "—"
        n   = str(r.get("n_test", "—"))
        print(f"{r['stage']:<14}{auc:>9}{acc:>10}{f1:>8}{n:>8}")
    print()


# ── Main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Train audio deepfake ensemble (Stage 1 MFCC+GBM, Stage 2 Wav2Vec2+LogReg)"
    )
    parser.add_argument(
        "--authentic-dir",
        default=os.path.join(os.path.dirname(__file__), "..", "corpus", "audio", "authentic"),
        help="Directory of authentic audio clips (LibriSpeech, field recordings, etc.)",
    )
    parser.add_argument(
        "--synthetic-dir",
        default=os.path.join(os.path.dirname(__file__), "..", "corpus", "audio", "synthetic"),
        help="Directory of synthetic/deepfake audio clips (ASVspoof, TTS systems, etc.)",
    )
    parser.add_argument(
        "--output-dir",
        default=os.path.join(os.path.dirname(__file__), "..", "models"),
        help="Output directory for probe .joblib files and metadata JSON",
    )
    parser.add_argument(
        "--test-split",
        type=float,
        default=0.10,
        help="Fraction of data held out for evaluation (default: 0.10)",
    )
    parser.add_argument(
        "--skip-stage2",
        action="store_true",
        help="Skip Wav2Vec2 embedding extraction (faster; trains Stage 1 only)",
    )
    args = parser.parse_args()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # ── Discover corpus ────────────────────────────────────────────────────────
    print("\n── Discovering corpus ──────────────────────────────────────────")
    authentic_files = collect_audio_files(args.authentic_dir)
    synthetic_files = collect_audio_files(args.synthetic_dir)
    print(f"  Authentic : {len(authentic_files)} files")
    print(f"  Synthetic : {len(synthetic_files)} files")

    if not authentic_files or not synthetic_files:
        print(
            "\nERROR: One or both corpus directories are empty.\n"
            "  Authentic : {}\n"
            "  Synthetic : {}\n"
            "Download LibriSpeech (train-clean-100) and ASVspoof2019-LA before running.".format(
                args.authentic_dir, args.synthetic_dir
            )
        )
        sys.exit(1)

    from sklearn.model_selection import train_test_split
    import joblib

    eval_results: list[dict] = []
    metadata: dict = {
        "trained_at": datetime.now(timezone.utc).isoformat(),
        "n_authentic": len(authentic_files),
        "n_synthetic": len(synthetic_files),
        "test_split": args.test_split,
        "ensemble_weights": {"stage1": 0.3, "stage2": 0.7},
        "stages": {},
    }

    # ── Stage 1: MFCC + GBM ──────────────────────────────────────────────────
    print("\n── Stage 1: MFCC feature extraction ───────────────────────────")
    t_start = time.perf_counter()

    auth_mfcc, auth_mfcc_labels, _ = extract_all_mfcc(authentic_files, label=0)
    synth_mfcc, synth_mfcc_labels, _ = extract_all_mfcc(synthetic_files, label=1)

    if not auth_mfcc or not synth_mfcc:
        print("ERROR: MFCC extraction yielded no samples.  Check audio file formats.")
        sys.exit(1)

    X1 = np.vstack(auth_mfcc + synth_mfcc)
    y1 = np.array(auth_mfcc_labels + synth_mfcc_labels)
    print(f"  Stage 1 dataset: {len(y1)} samples ({np.sum(y1 == 0)} authentic, {np.sum(y1 == 1)} synthetic)")

    X1_train, X1_test, y1_train, y1_test = train_test_split(
        X1, y1, test_size=args.test_split, random_state=42, stratify=y1
    )

    print(f"\n── Stage 1: Training GBM on {len(y1_train)} samples …")
    clf1 = _build_stage1_classifier()
    clf1.fit(X1_train, y1_train)
    elapsed1 = time.perf_counter() - t_start
    print(f"  Training time: {elapsed1:.1f} s")

    stage1_metrics = _evaluate(clf1, X1_test, y1_test, "stage1")
    eval_results.append(stage1_metrics)

    stage1_path = str(output_dir / "audio_deepfake_stage1.joblib")
    joblib.dump(clf1, stage1_path)
    stage1_sha = _sha256_file(stage1_path)
    print(f"  Saved: {stage1_path}")
    print(f"  SHA-256: {stage1_sha}")
    metadata["stages"]["stage1"] = {
        **stage1_metrics,
        "feature_dim": int(X1.shape[1]),
        "output_file": stage1_path,
        "sha256": stage1_sha,
    }

    # ── Stage 2: Wav2Vec2 + LogReg ────────────────────────────────────────────
    stage2_trained = False
    clf2 = None
    X2_test = y2_test = None

    if not args.skip_stage2:
        print("\n── Stage 2: Wav2Vec2 embedding extraction ──────────────────────")
        t2_start = time.perf_counter()

        auth_emb, auth_emb_labels, _ = extract_all_wav2vec2(authentic_files, label=0)
        synth_emb, synth_emb_labels, _ = extract_all_wav2vec2(synthetic_files, label=1)

        if auth_emb and synth_emb:
            X2 = np.vstack(auth_emb + synth_emb)
            y2 = np.array(auth_emb_labels + synth_emb_labels)
            print(f"  Stage 2 dataset: {len(y2)} samples")

            X2_train, X2_test, y2_train, y2_test = train_test_split(
                X2, y2, test_size=args.test_split, random_state=42, stratify=y2
            )

            print(f"\n── Stage 2: Training LogReg on {len(y2_train)} samples …")
            clf2 = _build_stage2_classifier()
            clf2.fit(X2_train, y2_train)
            elapsed2 = time.perf_counter() - t2_start
            print(f"  Training time: {elapsed2:.1f} s")

            stage2_metrics = _evaluate(clf2, X2_test, y2_test, "stage2")
            eval_results.append(stage2_metrics)

            stage2_path = str(output_dir / "audio_deepfake_stage2.joblib")
            joblib.dump(clf2, stage2_path)
            stage2_sha = _sha256_file(stage2_path)
            print(f"  Saved: {stage2_path}")
            print(f"  SHA-256: {stage2_sha}")
            metadata["stages"]["stage2"] = {
                **stage2_metrics,
                "feature_dim": int(X2.shape[1]),
                "output_file": stage2_path,
                "sha256": stage2_sha,
            }
            stage2_trained = True
        else:
            print("  Skipped Stage 2 (Wav2Vec2 unavailable — see WARNING above).")

    # ── Ensemble evaluation (only when both stages ran on same samples) ────────
    if stage2_trained and clf2 is not None and X2_test is not None and y2_test is not None:
        # Find common test indices: use the smaller test split as the reference.
        # In practice both splits are drawn from the same file list, so we
        # evaluate them independently and report both rather than forcing alignment.
        print("\n── Ensemble evaluation (stage1 subset aligned to stage2 test) …")
        # Re-predict stage1 on stage2 test set filenames — use a subset of X1_test
        # matching size of X2_test for a fair comparison table.
        n_overlap = min(len(X1_test), len(X2_test))
        try:
            ens_metrics = _ensemble_evaluate(
                clf1, clf2,
                X1_test[:n_overlap], X2_test[:n_overlap], y2_test[:n_overlap],
            )
            eval_results.append(ens_metrics)
        except Exception as exc:
            print(f"  WARNING: ensemble evaluation failed — {exc}")

    # ── Print comparison table ────────────────────────────────────────────────
    _print_comparison_table(eval_results)

    # ── Save metadata JSON ────────────────────────────────────────────────────
    meta_path = output_dir / "audio_deepfake_metadata.json"
    with open(meta_path, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2)
    print(f"Metadata saved: {meta_path}")
    print("\nDone.  Deploy the .joblib files and restart the sidecar.\n")


if __name__ == "__main__":
    main()
