#!/usr/bin/env python3
"""
Jura Trace -- Build Augmented Training Set + Train UnivFD v9 Probe

Combines the original 10,709-image training corpus with the 32,127-image
platform-forwarded augmentation produced by augment_corpus_platform_forwarded.py,
stratifies a 10% held-out test set, trains a new LogisticRegression probe
(UnivFD v9), evaluates it, and saves the candidate model + metadata.

The production model (models/univfd_probe.joblib) is NOT touched.
v9 is saved as models/univfd_probe_v9.joblib pending human promotion decision.

Usage
-----
    # Train v9 on original + augmented corpus (primary usage)
    python scripts/build_augmented_training_set.py

    # Override corpus paths
    python scripts/build_augmented_training_set.py \\
        --original-authentic "/Volumes/MAC SSD/Training Data/corpus/training/authentic" \\
        --original-ai        "/Volumes/MAC SSD/Training Data/corpus/training/ai_generated" \\
        --aug-authentic      "/Volumes/MAC SSD/Training Data/corpus/training_platform_forwarded/authentic" \\
        --aug-ai             "/Volumes/MAC SSD/Training Data/corpus/training_platform_forwarded/ai_generated"

    # Skip the augmented corpus (baseline-only mode, for comparison)
    python scripts/build_augmented_training_set.py --original-only

    # Adjust regularisation (default matches v8 probe: C=0.5 from train_univfd_probe.py)
    python scripts/build_augmented_training_set.py --C 0.5

Dependencies: open_clip, torch, sklearn, joblib, Pillow, numpy
"""

import argparse
import hashlib
import json
import os
import random
import sys
import time
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

try:
    import numpy as np
    from PIL import Image
except ImportError:
    print("ERROR: numpy and Pillow are required.")
    sys.exit(1)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}

# Variant tags for platform-forwarded images (used to label per-subset metrics)
PLT_TAGS = {"_plt75", "_plt85", "_plt2x"}

RANDOM_SEED = 42
TEST_FRACTION = 0.10


# ---------------------------------------------------------------------------
# Image collection helpers
# ---------------------------------------------------------------------------

def collect_images(directory: str | Path) -> list[Path]:
    """Collect image files, skipping macOS AppleDouble sidecars (._*).

    The training corpus lives on an exFAT USB drive where macOS writes
    AppleDouble resource-fork files alongside every real image. These
    share the same file extension as real images but cannot be opened
    by PIL. Filtering them at the collection stage eliminates a large
    tail of FAILED messages in the CLIP extraction log without affecting
    the training set size (they were never valid training inputs).
    """
    d = Path(directory)
    if not d.exists():
        print(f"  WARNING: directory not found: {d}")
        return []
    return sorted(
        f for f in d.rglob("*")
        if f.is_file()
        and f.suffix.lower() in IMAGE_EXTENSIONS
        and not f.name.startswith("._")  # AppleDouble sidecars
    )


def variant_tag(path: Path) -> str | None:
    """Return the platform variant tag if this is an augmented file, else None."""
    stem = path.stem
    for tag in PLT_TAGS:
        if stem.endswith(tag):
            return tag.lstrip("_")
    return None


def subdir_key(path: Path, root: Path) -> str:
    """Return the immediate subdirectory name relative to root."""
    try:
        rel = path.relative_to(root)
        parts = rel.parts
        return parts[0] if len(parts) > 1 else "__root__"
    except ValueError:
        return "__unknown__"


# ---------------------------------------------------------------------------
# CLIP embedding extraction
# ---------------------------------------------------------------------------

def load_clip(device="cpu"):
    try:
        import torch
        import open_clip
    except ImportError:
        print("ERROR: open_clip and torch are required.\n"
              "Install: pip install open-clip-torch torch")
        sys.exit(1)

    print("  Loading CLIP ViT-B-32 (laion2b_s34b_b79k)...")
    model, _, preprocess = open_clip.create_model_and_transforms(
        "ViT-B-32", pretrained="laion2b_s34b_b79k"
    )
    model.eval()
    return model, preprocess


def extract_embedding(img_path: Path, model, preprocess) -> np.ndarray | None:
    import torch
    try:
        img = Image.open(img_path).convert("RGB")
        tensor = preprocess(img).unsqueeze(0)
        with torch.no_grad():
            feat = model.encode_image(tensor)
            feat = feat / feat.norm(dim=-1, keepdim=True)
        return feat.squeeze().numpy().astype(np.float32)
    except Exception as e:
        print(f"FAILED ({e})")
        return None


def batch_extract(
    images: list[Path],
    label: int,
    model,
    preprocess,
    desc: str,
) -> tuple[list[np.ndarray], list[int], list[Path]]:
    embeddings, labels, paths = [], [], []
    n = len(images)
    for i, p in enumerate(images):
        if i % 200 == 0 or i == n - 1:
            print(f"    {desc}: [{i+1}/{n}]...", flush=True)
        emb = extract_embedding(p, model, preprocess)
        if emb is not None:
            embeddings.append(emb)
            labels.append(label)
            paths.append(p)
    return embeddings, labels, paths


# ---------------------------------------------------------------------------
# Stratified split
# ---------------------------------------------------------------------------

def stratified_split(
    embeddings: list[np.ndarray],
    labels: list[int],
    paths: list[Path],
    orig_auth_root: Path,
    orig_ai_root: Path,
    aug_auth_root: Path,
    aug_ai_root: Path,
    test_fraction: float,
    seed: int,
) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, list[str], list[str]]:
    """
    Stratify by (label, subdir, variant_tag) so each stratum is proportionally
    represented in train and test. Returns X_train, X_test, y_train, y_test,
    train_paths_str, test_paths_str.
    """
    rng = random.Random(seed)

    # Group indices by stratum key
    strata: dict[str, list[int]] = defaultdict(list)
    for idx, (label, path) in enumerate(zip(labels, paths)):
        tag = variant_tag(path) or "original"
        if label == 0:
            root = aug_auth_root if tag != "original" else orig_auth_root
        else:
            root = aug_ai_root if tag != "original" else orig_ai_root
        sub = subdir_key(path, root)
        key = f"{label}_{sub}_{tag}"
        strata[key].append(idx)

    train_idx, test_idx = [], []
    for key, idxs in sorted(strata.items()):
        rng.shuffle(idxs)
        n_test = max(1, round(len(idxs) * test_fraction))
        test_idx.extend(idxs[:n_test])
        train_idx.extend(idxs[n_test:])

    X = np.array(embeddings, dtype=np.float32)
    y = np.array(labels, dtype=np.int32)

    X_train = X[train_idx]
    X_test  = X[test_idx]
    y_train = y[train_idx]
    y_test  = y[test_idx]

    train_paths = [str(paths[i]) for i in train_idx]
    test_paths  = [str(paths[i]) for i in test_idx]

    return X_train, X_test, y_train, y_test, train_paths, test_paths


# ---------------------------------------------------------------------------
# Metrics helpers
# ---------------------------------------------------------------------------

def compute_metrics(y_true, y_pred, y_prob) -> dict:
    from sklearn.metrics import (
        roc_auc_score, confusion_matrix, classification_report
    )
    auc = float(roc_auc_score(y_true, y_prob))
    cm = confusion_matrix(y_true, y_pred)
    tn, fp, fn, tp = cm.ravel()
    fp_rate = float(fp / max(tn + fp, 1))
    recall = float(tp / max(tp + fn, 1))
    report = classification_report(
        y_true, y_pred,
        target_names=["authentic", "ai_generated"],
        output_dict=True,
    )
    return {
        "auc_roc": round(auc, 4),
        "fp_rate": round(fp_rate, 4),
        "ai_recall": round(recall, 4),
        "tn": int(tn), "fp": int(fp), "fn": int(fn), "tp": int(tp),
        "classification_report": report,
    }


def per_variant_metrics(
    y_true,
    y_pred,
    y_prob,
    test_paths: list[str],
) -> dict:
    """AUC per augmentation variant tag on the test set."""
    from sklearn.metrics import roc_auc_score

    tag_groups: dict[str, tuple[list, list]] = defaultdict(lambda: ([], []))
    for yt, yp, pp in zip(y_true, y_prob, test_paths):
        path = Path(pp)
        tag = variant_tag(path) or "original"
        tag_groups[tag][0].append(yt)
        tag_groups[tag][1].append(yp)

    result = {}
    for tag, (yt_list, yp_list) in sorted(tag_groups.items()):
        if len(set(yt_list)) < 2:
            result[tag] = {"auc_roc": None, "n": len(yt_list), "note": "single class"}
            continue
        auc = float(roc_auc_score(yt_list, yp_list))
        result[tag] = {"auc_roc": round(auc, 4), "n": len(yt_list)}
    return result


def per_generator_recall(
    y_true,
    y_pred,
    test_paths: list[str],
    aug_ai_root: Path,
) -> dict:
    """AI recall broken down by generator family (subdir name)."""
    gen_groups: dict[str, tuple[list, list]] = defaultdict(lambda: ([], []))
    for yt, yp, pp in zip(y_true, y_pred, test_paths):
        if yt != 1:
            continue
        sub = subdir_key(Path(pp), aug_ai_root)
        gen_groups[sub][0].append(yt)
        gen_groups[sub][1].append(yp)

    result = {}
    for gen, (yt_list, yp_list) in sorted(gen_groups.items()):
        n = len(yt_list)
        recalled = sum(1 for yp in yp_list if yp == 1)
        result[gen] = {"recall": round(recalled / max(n, 1), 4), "n": n}
    return result


# ---------------------------------------------------------------------------
# SHA-256 of model file
# ---------------------------------------------------------------------------

def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Build augmented training set and train UnivFD v9 probe"
    )
    usb = "/Volumes/MAC SSD/Training Data/corpus"
    parser.add_argument("--original-authentic", default=f"{usb}/training/authentic")
    parser.add_argument("--original-ai",        default=f"{usb}/training/ai_generated")
    parser.add_argument("--aug-authentic",       default=f"{usb}/training_platform_forwarded/authentic")
    parser.add_argument("--aug-ai",              default=f"{usb}/training_platform_forwarded/ai_generated")
    parser.add_argument("--output-dir",          default="models",
                        help="Directory for model and metadata output")
    parser.add_argument("--split-json",          default="models/univfd_v9_split.json",
                        help="Path to persist train/test split for reproducibility")
    parser.add_argument("--C",  type=float, default=0.5,
                        help="LogisticRegression regularisation (default 0.5, matches v8)")
    parser.add_argument("--original-only", action="store_true",
                        help="Train on original corpus only (baseline comparison mode)")
    args = parser.parse_args()

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    orig_auth_root = Path(args.original_authentic)
    orig_ai_root   = Path(args.original_ai)
    aug_auth_root  = Path(args.aug_authentic)
    aug_ai_root    = Path(args.aug_ai)

    print("Jura Trace -- UnivFD v9 Augmented Training (backlog #16)")
    print("=" * 70)
    print(f"  Original authentic: {orig_auth_root}")
    print(f"  Original AI:        {orig_ai_root}")
    if not args.original_only:
        print(f"  Aug authentic:      {aug_auth_root}")
        print(f"  Aug AI:             {aug_ai_root}")
    print(f"  C:                  {args.C}")
    print(f"  Seed:               {RANDOM_SEED}")

    # --- Load CLIP ---
    model, preprocess = load_clip()

    # --- Collect all images ---
    print("\nCollecting images...")
    orig_auth_imgs = collect_images(orig_auth_root)
    orig_ai_imgs   = collect_images(orig_ai_root)
    print(f"  Original authentic: {len(orig_auth_imgs)}")
    print(f"  Original AI:        {len(orig_ai_imgs)}")

    aug_auth_imgs, aug_ai_imgs = [], []
    if not args.original_only:
        aug_auth_imgs = collect_images(aug_auth_root)
        aug_ai_imgs   = collect_images(aug_ai_root)
        print(f"  Aug authentic:      {len(aug_auth_imgs)}")
        print(f"  Aug AI:             {len(aug_ai_imgs)}")

    all_auth_imgs = orig_auth_imgs + aug_auth_imgs
    all_ai_imgs   = orig_ai_imgs   + aug_ai_imgs
    print(f"\n  Total authentic:    {len(all_auth_imgs)}")
    print(f"  Total AI:           {len(all_ai_imgs)}")
    print(f"  Grand total:        {len(all_auth_imgs) + len(all_ai_imgs)}")

    # --- Extract embeddings ---
    print("\nExtracting CLIP embeddings...")
    t0 = time.time()

    emb_a, lbl_a, pth_a = batch_extract(all_auth_imgs, 0, model, preprocess, "authentic")
    emb_ai, lbl_ai, pth_ai = batch_extract(all_ai_imgs,   1, model, preprocess, "ai_generated")

    all_emb = emb_a + emb_ai
    all_lbl = lbl_a + lbl_ai
    all_pth = pth_a + pth_ai

    extract_time = time.time() - t0
    print(f"\n  Embeddings extracted: {len(all_emb)} in {extract_time:.0f}s")
    print(f"  Embedding dimension:  {all_emb[0].shape[0] if all_emb else 'N/A'}")

    if len(all_emb) < 10:
        print("ERROR: Too few embeddings. Aborting.")
        sys.exit(1)

    # --- Stratified split ---
    print(f"\nSplitting {TEST_FRACTION:.0%} held-out test set (stratified)...")
    X_train, X_test, y_train, y_test, train_paths, test_paths = stratified_split(
        all_emb, all_lbl, all_pth,
        orig_auth_root, orig_ai_root,
        aug_auth_root, aug_ai_root,
        TEST_FRACTION, RANDOM_SEED,
    )

    print(f"  Train: {len(y_train)} (auth={sum(y_train==0)}, ai={sum(y_train==1)})")
    print(f"  Test:  {len(y_test)}  (auth={sum(y_test==0)}, ai={sum(y_test==1)})")

    # --- Persist split ---
    split_data = {
        "seed": RANDOM_SEED,
        "test_fraction": TEST_FRACTION,
        "n_train": int(len(y_train)),
        "n_test": int(len(y_test)),
        "train_auth": int(sum(y_train == 0)),
        "train_ai": int(sum(y_train == 1)),
        "test_auth": int(sum(y_test == 0)),
        "test_ai": int(sum(y_test == 1)),
        "train_paths": train_paths,
        "test_paths": test_paths,
        "generated_at": datetime.now(timezone.utc).isoformat(),
    }
    split_path = Path(args.split_json)
    split_path.parent.mkdir(parents=True, exist_ok=True)
    split_path.write_text(json.dumps(split_data, indent=2))
    print(f"  Split persisted: {split_path}")

    # --- Train probe ---
    from sklearn.linear_model import LogisticRegression
    from sklearn.metrics import roc_auc_score

    print(f"\nTraining LogisticRegression probe (C={args.C}, balanced, max_iter=1000)...")
    t1 = time.time()
    probe = LogisticRegression(
        C=args.C,
        max_iter=1000,
        random_state=RANDOM_SEED,
        solver="lbfgs",
        class_weight="balanced",
    )
    probe.fit(X_train, y_train)
    train_time = time.time() - t1
    print(f"  Training time: {train_time:.1f}s")

    # --- Evaluate on held-out test set ---
    print("\nEvaluating on held-out test set...")
    y_prob = probe.predict_proba(X_test)[:, 1]
    y_pred = (y_prob >= 0.5).astype(int)

    metrics = compute_metrics(y_test, y_pred, y_prob)
    print(f"  AUC-ROC:    {metrics['auc_roc']:.4f}")
    print(f"  FP rate:    {metrics['fp_rate']:.4f}  ({metrics['fp']}/{metrics['tn']+metrics['fp']})")
    print(f"  AI recall:  {metrics['ai_recall']:.4f}  ({metrics['tp']}/{metrics['tp']+metrics['fn']})")

    # Platform-forwarded subset AUC
    print("\nPer-variant AUC on test set:")
    pv = per_variant_metrics(y_test, y_pred, y_prob, test_paths)
    for tag, m in pv.items():
        auc_str = f"{m['auc_roc']:.4f}" if m.get("auc_roc") is not None else "N/A"
        print(f"  {tag:12s}: AUC={auc_str}  n={m['n']}")

    # Per-generator recall
    print("\nPer-generator recall on AI test set:")
    pg = per_generator_recall(y_test, y_pred, test_paths, aug_ai_root)
    for gen, m in pg.items():
        print(f"  {gen:30s}: recall={m['recall']:.4f}  n={m['n']}")

    # --- Save model ---
    import joblib
    model_path = output_dir / "univfd_probe_v9.joblib"
    joblib.dump(probe, model_path)
    model_sha = sha256_file(model_path)
    print(f"\nModel saved: {model_path}")
    print(f"  Size: {model_path.stat().st_size / 1024:.1f} KB")
    print(f"  SHA-256: {model_sha}")

    # --- v8 baseline for comparison ---
    V8_BASELINE = {
        "auc_roc": 0.9911,
        "fp_rate": 0.0501,
        "ai_recall": 0.9601,
        "n_samples": 10712,
        "trained_at": "2026-04-07T14:29:54",
    }

    print("\n" + "=" * 70)
    print("v8 vs v9 COMPARISON")
    print("=" * 70)
    print(f"  {'Metric':25s}  {'v8 (baseline)':>14s}  {'v9 (candidate)':>14s}  {'Delta':>10s}")
    print(f"  {'-'*25}  {'-'*14}  {'-'*14}  {'-'*10}")
    for k, label in [
        ("auc_roc", "AUC-ROC"),
        ("fp_rate", "FP rate"),
        ("ai_recall", "AI recall"),
    ]:
        v8_val = V8_BASELINE[k]
        v9_val = metrics[k]
        delta = v9_val - v8_val
        sign = "+" if delta >= 0 else ""
        print(f"  {label:25s}  {v8_val:>14.4f}  {v9_val:>14.4f}  {sign}{delta:>+9.4f}")
    print(f"  {'Training samples':25s}  {V8_BASELINE['n_samples']:>14d}  {len(y_train):>14d}")

    # --- Regression flag ---
    if metrics["auc_roc"] < V8_BASELINE["auc_roc"]:
        print(f"\n  FLAG: AUC regression vs v8 ({metrics['auc_roc']:.4f} < {V8_BASELINE['auc_roc']:.4f})")
    if metrics["fp_rate"] > V8_BASELINE["fp_rate"] * 1.1:
        print(f"\n  FLAG: FP rate >10% worse than v8 ({metrics['fp_rate']:.4f} vs {V8_BASELINE['fp_rate']:.4f})")
    if metrics["ai_recall"] < V8_BASELINE["ai_recall"] - 0.01:
        print(f"\n  FLAG: AI recall regression vs v8 ({metrics['ai_recall']:.4f} < {V8_BASELINE['ai_recall']:.4f})")

    # --- Save metadata ---
    meta = {
        "model_version": "v9",
        "trained_at": datetime.now(timezone.utc).isoformat(),
        "approach": "UnivFD-style linear probe on CLIP ViT-B/32 embeddings (platform-forwarded augmentation)",
        "training_task": "backlog_item_16_platform_forwarded_augmentation",
        "clip_model": "ViT-B-32",
        "clip_pretrained": "laion2b_s34b_b79k",
        "embedding_dim": int(X_train.shape[1]),
        "random_seed": RANDOM_SEED,
        "probe_params": {"C": args.C, "max_iter": 1000, "solver": "lbfgs", "class_weight": "balanced"},
        "training_corpus": {
            "original_authentic": str(orig_auth_root),
            "original_ai": str(orig_ai_root),
            "aug_authentic": str(aug_auth_root) if not args.original_only else None,
            "aug_ai": str(aug_ai_root) if not args.original_only else None,
            "n_train": int(len(y_train)),
            "n_test": int(len(y_test)),
            "n_train_authentic": int(sum(y_train == 0)),
            "n_train_ai": int(sum(y_train == 1)),
        },
        "evaluation": {
            "held_out_set": "10% stratified by subdir+variant",
            "split_json": str(split_path),
            "threshold": 0.5,
            **metrics,
        },
        "per_variant_auc": pv,
        "per_generator_recall": pg,
        "v8_baseline": V8_BASELINE,
        "sha256": model_sha,
        "model_path": str(model_path),
    }

    meta_path = output_dir / "univfd_probe_v9_meta.json"
    meta_path.write_text(json.dumps(meta, indent=2))
    print(f"\nMetadata saved: {meta_path}")

    print("\nDone. Review metrics above before deciding on promotion to production.")
    print("Next step: review docs/calibration/univfd-v9-platform-augmentation.md")


if __name__ == "__main__":
    main()
