#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
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

# Register HEIC opener so PIL.Image.open() can read .heic files produced by
# the multi-format augmentation (sips on macOS).  pillow-heif is required for
# the v10 retrain; older v9 corpora are JPEG-only so this is a v10-only need.
try:
    import pillow_heif
    pillow_heif.register_heif_opener()
    _HEIC_SUPPORTED = True
except ImportError:
    _HEIC_SUPPORTED = False
    print("WARN: pillow-heif not installed — .heic files will be skipped.")

# Register AVIF opener for v11 augmentation (AVIF/low-res/Q=95 variants).
# pillow-avif-plugin 1.5.5 is required; graceful degradation if absent.
try:
    import pillow_avif  # noqa: F401  -- registers AVIF opener as side-effect
    _AVIF_SUPPORTED = True
except ImportError:
    _AVIF_SUPPORTED = False
    print("WARN: pillow-avif-plugin not installed — .avif files will be skipped.")

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "sidecar"))

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".tiff", ".tif", ".webp"}
if _HEIC_SUPPORTED:
    IMAGE_EXTENSIONS = IMAGE_EXTENSIONS | {".heic", ".heif"}
if _AVIF_SUPPORTED:
    IMAGE_EXTENSIONS = IMAGE_EXTENSIONS | {".avif"}

# Variant tags for platform-forwarded images (used to label per-subset metrics)
PLT_TAGS = {"_plt75", "_plt85", "_plt2x"}
# Variant tags for v10 multi-format augmentation
FMT_TAGS = {"_fmt_png", "_fmt_tiff", "_fmt_webp", "_fmt_heic"}
# Variant tags for v11 AVIF/low-res augmentation (augment_corpus_avif_lowres.py)
AVIF_LR_TAGS = {"_fmt_avif_q60", "_fmt_avif_q80", "_lowres_700", "_lowres_1024", "_jpeg_q95"}

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
    """Return the variant tag if this is an augmented file, else None.

    Recognises platform-forwarded (_plt75 / _plt85 / _plt2x), v10
    multi-format (_fmt_png / _fmt_tiff / _fmt_webp / _fmt_heic), and v11
    AVIF/low-res (_fmt_avif_q60 / _fmt_avif_q80 / _lowres_700 /
    _lowres_1024 / _jpeg_q95) variants.
    """
    stem = path.stem
    # v11 AVIF/low-res tags checked first (longer suffix, avoids false match
    # if a future tag is a prefix of a v11 tag)
    for tag in AVIF_LR_TAGS | PLT_TAGS | FMT_TAGS:
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

    print("  Loading CLIP ViT-B-32 (laion2b_s34b_b79k) via open_clip + PyTorch...")
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


# ---------------------------------------------------------------------------
# Sidecar-aligned ONNX feature extractor (v10onnx, post-mortem 2026-05-11)
#
# WHY: open_clip+PyTorch's preprocess uses torchvision Resize(BICUBIC,
# antialias=True), which is bit-different from PIL.Image.resize(BICUBIC).
# The shipped sidecar uses the PIL path (JTV-143), so a probe trained on
# PyTorch embeddings is calibrated on a different embedding distribution
# than the one it serves at inference time (mean cos sim ~0.996, NOT 1.0).
# Diagnostic: scripts/diagnose_clip_onnx_divergence.py, 2026-05-11.
#
# This path mirrors the sidecar's `_preprocess_image` + ONNX inference path
# bit-exactly so training embeddings ARE the inference embeddings.
# ---------------------------------------------------------------------------

# CLIP normalisation constants — identical to sidecar/app/services/clip_detector.py
_ONNX_CLIP_MEAN = None
_ONNX_CLIP_STD = None


def load_clip_onnx(onnx_path: Path):
    """Load the CLIP ViT-B/32 ONNX vision encoder via onnxruntime.

    Returns (session, input_name) — caller uses extract_embedding_onnx() to
    process individual images through the same preprocess + inference path
    used by the shipped sidecar.
    """
    global _ONNX_CLIP_MEAN, _ONNX_CLIP_STD
    try:
        import onnxruntime as ort
    except ImportError:
        print("ERROR: onnxruntime is required for --feature-extractor sidecar_onnx.\n"
              "Install: pip install onnxruntime")
        sys.exit(1)

    if not onnx_path.exists():
        print(f"ERROR: ONNX model not found: {onnx_path}")
        sys.exit(1)

    print(f"  Loading CLIP ViT-B/32 via ONNX runtime: {onnx_path}")
    session = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    input_name = session.get_inputs()[0].name

    _ONNX_CLIP_MEAN = np.array(
        [0.48145466, 0.4578275, 0.40821073], dtype=np.float32
    ).reshape(3, 1, 1)
    _ONNX_CLIP_STD = np.array(
        [0.26862954, 0.26130258, 0.27577711], dtype=np.float32
    ).reshape(3, 1, 1)
    return session, input_name


def _onnx_preprocess(img: Image.Image) -> np.ndarray:
    """Bit-identical mirror of sidecar/app/services/clip_detector.py:_preprocess_image."""
    if img.mode != "RGB":
        img = img.convert("RGB")
    w, h = img.size
    if w < h:
        new_w, new_h = 224, int(round(h * 224 / w))
    else:
        new_w, new_h = int(round(w * 224 / h)), 224
    img = img.resize((new_w, new_h), Image.BICUBIC)
    left = (new_w - 224) // 2
    top = (new_h - 224) // 2
    img = img.crop((left, top, left + 224, top + 224))
    arr = np.asarray(img, dtype=np.float32) / 255.0
    arr = arr.transpose(2, 0, 1)
    arr = (arr - _ONNX_CLIP_MEAN) / _ONNX_CLIP_STD
    return arr[np.newaxis, ...]


def extract_embedding_onnx(img_path: Path, session, input_name: str) -> np.ndarray | None:
    """ONNX feature extraction — production-aligned path."""
    try:
        img = Image.open(img_path).convert("RGB")
        tensor = _onnx_preprocess(img)
        out = session.run(None, {input_name: tensor})[0]
        feat = out[0]
        norm = float(np.linalg.norm(feat)) + 1e-12
        return (feat / norm).astype(np.float32)
    except Exception as e:
        print(f"FAILED ({e})")
        return None


def batch_extract(
    images: list[Path],
    label: int,
    model,
    preprocess,
    desc: str,
    *,
    backend: str = "open_clip",
    onnx_session=None,
    onnx_input_name: str | None = None,
) -> tuple[list[np.ndarray], list[int], list[Path]]:
    embeddings, labels, paths = [], [], []
    n = len(images)
    for i, p in enumerate(images):
        if i % 200 == 0 or i == n - 1:
            print(f"    {desc}: [{i+1}/{n}]...", flush=True)
        if backend == "sidecar_onnx":
            emb = extract_embedding_onnx(p, onnx_session, onnx_input_name)
        else:
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
    # v10 multi-format augmentation paths (PNG + TIFF + WebP + HEIC re-encodes
    # of the AI corpus).  Optional — when present, samples are added to the
    # training pool alongside platform-forwarded augmentation.
    parser.add_argument("--multi-format-aug-authentic",
                        default=f"{usb}/training_multi_format/authentic",
                        help="v10 multi-format augmentation (authentic, optional)")
    parser.add_argument("--multi-format-aug-ai",
                        default=f"{usb}/training_multi_format/ai_generated",
                        help="v10 multi-format augmentation (AI, optional)")
    # v11 AVIF/low-res augmentation paths (AVIF q=60/80, lowres_700/1024, jpeg_q95).
    # Generated by scripts/augment_corpus_avif_lowres.py.
    parser.add_argument("--avif-lowres-aug-authentic",
                        default=f"{usb}/training_avif_low_res/authentic",
                        help="v11 AVIF/low-res augmentation (authentic, optional)")
    parser.add_argument("--avif-lowres-aug-ai",
                        default=f"{usb}/training_avif_low_res/ai_generated",
                        help="v11 AVIF/low-res augmentation (AI, optional)")
    parser.add_argument("--skip-avif-lowres", action="store_true",
                        help="Skip v11 AVIF/low-res augmentation (use for v9/v10 replays)")
    parser.add_argument("--output-dir",          default="models",
                        help="Directory for model and metadata output")
    parser.add_argument("--model-version",       default="v10",
                        help="Model version tag for output files (default v10)")
    parser.add_argument("--split-json",          default=None,
                        help="Path to persist train/test split (default models/univfd_{version}_split.json)")
    parser.add_argument("--C",  type=float, default=0.5,
                        help="LogisticRegression regularisation (default 0.5, matches v9)")
    parser.add_argument("--original-only", action="store_true",
                        help="Train on original corpus only (baseline comparison mode)")
    parser.add_argument("--skip-multi-format", action="store_true",
                        help="Skip multi-format augmentation (v9-style training)")
    parser.add_argument(
        "--feature-extractor",
        choices=["open_clip", "sidecar_onnx"],
        default="open_clip",
        help=(
            "CLIP feature extraction backend. "
            "'open_clip' = PyTorch + open_clip (default, v9/v10 historic path). "
            "'sidecar_onnx' = bit-identical to shipped sidecar (PIL+ONNX) — "
            "use when training a probe that will be served via the production "
            "ONNX runtime. Required after 2026-05-11 PyTorch↔ONNX divergence "
            "diagnosis (cos sim 0.996, not 1.0). See diagnose_clip_onnx_divergence.py."
        ),
    )
    parser.add_argument(
        "--onnx-model",
        default=str(Path(__file__).parent.parent / "models" / "clip-vit-b32-vision.onnx"),
        help="CLIP ViT-B/32 ONNX vision encoder path (used when --feature-extractor=sidecar_onnx)",
    )
    args = parser.parse_args()

    if args.split_json is None:
        args.split_json = f"models/univfd_{args.model_version}_split.json"

    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    orig_auth_root = Path(args.original_authentic)
    orig_ai_root   = Path(args.original_ai)
    aug_auth_root  = Path(args.aug_authentic)
    aug_ai_root    = Path(args.aug_ai)
    mf_auth_root   = Path(args.multi_format_aug_authentic)
    mf_ai_root     = Path(args.multi_format_aug_ai)
    al_auth_root   = Path(args.avif_lowres_aug_authentic)
    al_ai_root     = Path(args.avif_lowres_aug_ai)
    use_multi_format = not args.skip_multi_format and not args.original_only
    use_avif_lowres  = (
        not getattr(args, "skip_avif_lowres", False) and not args.original_only
    )

    print(f"Jura Trace -- UnivFD {args.model_version} Augmented Training")
    print("=" * 70)
    print(f"  Original authentic: {orig_auth_root}")
    print(f"  Original AI:        {orig_ai_root}")
    if not args.original_only:
        print(f"  Aug authentic:      {aug_auth_root}")
        print(f"  Aug AI:             {aug_ai_root}")
    if use_multi_format:
        print(f"  MF aug authentic:   {mf_auth_root}")
        print(f"  MF aug AI:          {mf_ai_root}")
    if use_avif_lowres:
        print(f"  AL aug authentic:   {al_auth_root}")
        print(f"  AL aug AI:          {al_ai_root}")
    print(f"  C:                  {args.C}")
    print(f"  Seed:               {RANDOM_SEED}")
    print(f"  HEIC support:       {_HEIC_SUPPORTED}")
    print(f"  AVIF support:       {_AVIF_SUPPORTED}")
    print(f"  Feature extractor:  {args.feature_extractor}")

    # --- Load CLIP (backend selected by --feature-extractor) ---
    onnx_session = None
    onnx_input_name = None
    model = None
    preprocess = None
    if args.feature_extractor == "sidecar_onnx":
        onnx_session, onnx_input_name = load_clip_onnx(Path(args.onnx_model))
    else:
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

    mf_auth_imgs, mf_ai_imgs = [], []
    if use_multi_format:
        mf_auth_imgs = collect_images(mf_auth_root)
        mf_ai_imgs   = collect_images(mf_ai_root)
        print(f"  MF aug authentic:   {len(mf_auth_imgs)}")
        print(f"  MF aug AI:          {len(mf_ai_imgs)}")

    al_auth_imgs, al_ai_imgs = [], []
    if use_avif_lowres:
        al_auth_imgs = collect_images(al_auth_root)
        al_ai_imgs   = collect_images(al_ai_root)
        print(f"  AL aug authentic:   {len(al_auth_imgs)}")
        print(f"  AL aug AI:          {len(al_ai_imgs)}")
        if not al_auth_imgs and not al_ai_imgs:
            print("  WARNING: AVIF/low-res augmentation directories are empty.")
            print("    Run scripts/augment_corpus_avif_lowres.py first.")

    all_auth_imgs = orig_auth_imgs + aug_auth_imgs + mf_auth_imgs + al_auth_imgs
    all_ai_imgs   = orig_ai_imgs   + aug_ai_imgs   + mf_ai_imgs   + al_ai_imgs
    print(f"\n  Total authentic:    {len(all_auth_imgs)}")
    print(f"  Total AI:           {len(all_ai_imgs)}")
    print(f"  Grand total:        {len(all_auth_imgs) + len(all_ai_imgs)}")

    # --- Extract embeddings ---
    print("\nExtracting CLIP embeddings...")
    t0 = time.time()

    emb_a, lbl_a, pth_a = batch_extract(
        all_auth_imgs, 0, model, preprocess, "authentic",
        backend=args.feature_extractor,
        onnx_session=onnx_session, onnx_input_name=onnx_input_name,
    )
    emb_ai, lbl_ai, pth_ai = batch_extract(
        all_ai_imgs, 1, model, preprocess, "ai_generated",
        backend=args.feature_extractor,
        onnx_session=onnx_session, onnx_input_name=onnx_input_name,
    )

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
    model_path = output_dir / f"univfd_probe_{args.model_version}.joblib"
    joblib.dump(probe, model_path)
    model_sha = sha256_file(model_path)
    print(f"\nModel saved: {model_path}")
    print(f"  Size: {model_path.stat().st_size / 1024:.1f} KB")
    print(f"  SHA-256: {model_sha}")

    # --- v9 baseline for comparison (current production) ---
    V9_BASELINE = {
        "auc_roc": 0.9933,
        "fp_rate": 0.0412,
        "ai_recall": 0.9570,
        "n_samples": 39016,
        "trained_at": "2026-04-12T00:00:00",
    }

    print("\n" + "=" * 70)
    print(f"v9 vs {args.model_version} COMPARISON")
    print("=" * 70)
    print(f"  {'Metric':25s}  {'v9 (baseline)':>14s}  {args.model_version + ' (candidate)':>15s}  {'Delta':>10s}")
    print(f"  {'-'*25}  {'-'*14}  {'-'*15}  {'-'*10}")
    for k, label in [
        ("auc_roc", "AUC-ROC"),
        ("fp_rate", "FP rate"),
        ("ai_recall", "AI recall"),
    ]:
        baseline_val = V9_BASELINE[k]
        candidate_val = metrics[k]
        delta = candidate_val - baseline_val
        sign = "+" if delta >= 0 else ""
        print(f"  {label:25s}  {baseline_val:>14.4f}  {candidate_val:>15.4f}  {sign}{delta:>+9.4f}")
    print(f"  {'Training samples':25s}  {V9_BASELINE['n_samples']:>14d}  {len(y_train):>15d}")

    # --- Hard regression gates ---
    # v10onnx baselines: FP 3.87%, recall 95.77%, AUC 0.9929
    # v11 gates: FP ≤ 5.12%, recall ≥ 94.7%, AUC ≥ 0.9883
    # New v11 per-format gates: AVIF AUC ≥ 0.90, low-res AUC ≥ 0.90
    FP_CEILING   = 0.0512
    RECALL_FLOOR = 0.9470
    AUC_FLOOR    = 0.9883  # v10onnx AUC 0.9929 - 0.005pp slack

    print("\n  Hard regression gates (v11 promotion criteria):")
    fp_pass     = metrics["fp_rate"] <= FP_CEILING
    recall_pass = metrics["ai_recall"] >= RECALL_FLOOR
    auc_pass    = metrics["auc_roc"] >= AUC_FLOOR
    print(f"    FP rate ≤ {FP_CEILING:.4f}:   {'PASS' if fp_pass else 'FAIL'}  ({metrics['fp_rate']:.4f})")
    print(f"    Recall ≥ {RECALL_FLOOR:.4f}:   {'PASS' if recall_pass else 'FAIL'}  ({metrics['ai_recall']:.4f})")
    print(f"    AUC ≥ {AUC_FLOOR:.4f}:       {'PASS' if auc_pass else 'FAIL'}  ({metrics['auc_roc']:.4f})")

    # Per-format v11 gates
    AVIF_AUC_FLOOR   = 0.90
    LOWRES_AUC_FLOOR = 0.90
    LEGACY_AUC_FLOOR = 0.99  # v10 gate for existing formats

    avif_tags   = {"fmt_avif_q60", "fmt_avif_q80"}
    lowres_tags = {"lowres_700", "lowres_1024"}
    legacy_tags = {"fmt_png", "fmt_tiff", "fmt_webp", "fmt_heic"}

    all_gates_pass = fp_pass and recall_pass and auc_pass

    for tag, m in pv.items():
        if m.get("auc_roc") is None:
            continue
        tag_auc = m["auc_roc"]
        if tag in avif_tags:
            gate = AVIF_AUC_FLOOR
            gate_label = "AVIF AUC"
        elif tag in lowres_tags:
            gate = LOWRES_AUC_FLOOR
            gate_label = "low-res AUC"
        elif tag in legacy_tags:
            gate = LEGACY_AUC_FLOOR
            gate_label = "legacy-fmt AUC"
        else:
            continue
        tag_pass = tag_auc >= gate
        if not tag_pass:
            all_gates_pass = False
        print(f"    {gate_label} ({tag:20s}) ≥ {gate:.2f}: {'PASS' if tag_pass else 'FAIL'}  ({tag_auc:.4f})")

    if not all_gates_pass:
        print(f"\n  ONE OR MORE GATES FAILED -- DO NOT PROMOTE {args.model_version}.")
    else:
        print(f"\n  All gates PASS -- {args.model_version} is a promotion candidate.")

    # --- Save metadata ---
    if use_avif_lowres and use_multi_format:
        approach_str = (
            "UnivFD-style linear probe on CLIP ViT-B/32 embeddings "
            "(platform-forwarded + multi-format + AVIF/low-res augmentation)"
        )
        task_str = "avif_lowres_augmentation_v11"
    elif use_multi_format:
        approach_str = (
            "UnivFD-style linear probe on CLIP ViT-B/32 embeddings "
            "(platform-forwarded + multi-format augmentation)"
        )
        task_str = "JTV-180_multi_format_augmentation_v10"
    else:
        approach_str = (
            "UnivFD-style linear probe on CLIP ViT-B/32 embeddings "
            "(platform-forwarded augmentation)"
        )
        task_str = "backlog_item_16_platform_forwarded_augmentation"

    meta = {
        "model_version": args.model_version,
        "trained_at": datetime.now(timezone.utc).isoformat(),
        "approach": approach_str,
        "training_task": task_str,
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
            "mf_aug_authentic": str(mf_auth_root) if use_multi_format else None,
            "mf_aug_ai": str(mf_ai_root) if use_multi_format else None,
            "al_aug_authentic": str(al_auth_root) if use_avif_lowres else None,
            "al_aug_ai": str(al_ai_root) if use_avif_lowres else None,
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
        "v9_baseline": V9_BASELINE,
        "v10onnx_baseline": {
            "auc_roc": 0.9929,
            "fp_rate": 0.0387,
            "ai_recall": 0.9577,
            "n_samples": 56344,
            "promoted_at": "2026-05-11",
        },
        "regression_gates": {
            "fp_ceiling": FP_CEILING,
            "recall_floor": RECALL_FLOOR,
            "auc_floor": AUC_FLOOR,
            "avif_auc_floor": AVIF_AUC_FLOOR,
            "lowres_auc_floor": LOWRES_AUC_FLOOR,
            "legacy_fmt_auc_floor": LEGACY_AUC_FLOOR,
            "fp_pass": fp_pass,
            "recall_pass": recall_pass,
            "auc_pass": auc_pass,
            "all_gates_pass": all_gates_pass,
        },
        "heic_support_at_training": _HEIC_SUPPORTED,
        "avif_support_at_training": _AVIF_SUPPORTED,
        "sha256": model_sha,
        "model_path": str(model_path),
    }

    meta_path = output_dir / f"univfd_probe_{args.model_version}_meta.json"
    meta_path.write_text(json.dumps(meta, indent=2))
    print(f"\nMetadata saved: {meta_path}")

    print("\nDone. Review metrics above before deciding on promotion to production.")
    print(f"Next step: review docs/calibration/univfd-{args.model_version}-avif-lowres-augmentation.md")


if __name__ == "__main__":
    main()
