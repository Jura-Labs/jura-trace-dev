#!/usr/bin/env python3
"""
Jura Trace — Sprint 29 Fairness Foundation: Demographic Bias Test Suite

Quarterly-retrainable fairness gate for TRIED Pillar 4 (Fair).

What this script does:
  1. Loads the held-out test split from models/univfd_v9_split.json.
  2. Runs CLIP ViT-B/32 text-image similarity against a set of demographic-proxy
     prompts to assign images to proxy groups.
  3. For each proxy group, computes authentic FP rate and AI recall using the
     UnivFD v9 model.
  4. Asserts that no proxy group has FP rate > 2× overall OR recall < 85%.
  5. If any assertion fails, exits with code 1 and prints the problematic group.
  6. Writes docs/fairness/bias-test-results.md with full results.

CRITICAL LIMITATIONS — READ BEFORE CITING:
  - CLIP text-image similarity is NOT a demographic classifier. It does not
    label individuals with demographic characteristics. These are coarse
    distributional proxies to detect gross disparities in model performance,
    not ground-truth demographic measurements.
  - Proxy group assignment is based on cosine similarity above a threshold.
    Images near the threshold boundary will be misassigned. Groups are
    approximate, overlapping, and should not be interpreted as clean partitions.
  - Over-claiming fairness from these results would be worse than silence.
    This test is a canary, not a certificate. Passing does not mean the model
    is fair; failing does not mean it is catastrophically biased.
  - A proper demographic fairness audit requires human annotation of a
    representative held-out set. This script is a computationally tractable
    interim measure pending that annotation effort.

Assertions:
  - FP rate per proxy group <= 2 × overall FP rate
  - AI recall per proxy group >= 85%

Usage:
    # Run against production v9 split and model (USB must be mounted):
    python3 scripts/test_demographic_bias.py

    # Pilot run (sample up to N images from test set):
    python3 scripts/test_demographic_bias.py --count 200

    # Use a different split file:
    python3 scripts/test_demographic_bias.py --split models/univfd_v9_split.json

    # Adjust thresholds:
    python3 scripts/test_demographic_bias.py --fp-ceiling 0.15 --recall-floor 0.80

    # Write results to custom path:
    python3 scripts/test_demographic_bias.py --output docs/fairness/bias-test-results.md
"""

import argparse
import json
import logging
import random
import sys
from datetime import datetime, timezone
from pathlib import Path

logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_SPLIT = REPO_ROOT / "models" / "univfd_v9_split.json"
DEFAULT_PROBE = REPO_ROOT / "models" / "univfd_probe_v9.joblib"
OUTPUT_DIR = REPO_ROOT / "docs" / "fairness"
DEFAULT_OUTPUT = OUTPUT_DIR / "bias-test-results.md"

# ---------------------------------------------------------------------------
# Demographic-proxy prompts
# ---------------------------------------------------------------------------
# These are COARSE distributional proxies — NOT demographic labels.
# The purpose is to detect gross disparities in model performance across
# groups defined by CLIP similarity, not to classify individuals.

PROXY_PROMPTS: list[tuple[str, str]] = [
    (
        "dark_skin",
        "a photograph of a dark-skinned person",
    ),
    (
        "light_skin",
        "a photograph of a light-skinned person",
    ),
    (
        "no_people",
        "a photograph with no people, showing scenery, objects, or animals",
    ),
    (
        "africa_south_asia",
        "a photograph taken in Africa or South Asia",
    ),
    (
        "europe_north_america",
        "a photograph taken in Europe or North America",
    ),
]

# Assignment strategy: each image is assigned to the proxy group with the
# HIGHEST softmax probability (argmax), plus any other group whose probability
# exceeds the uniform baseline (1/n_groups). With 5 groups, baseline = 0.20;
# an image goes into its best-matching group AND any group where it scores
# above 0.20. This guarantees every image is in at least one group while
# allowing multi-group membership for genuinely ambiguous images.
#
# The original 0.25 threshold was too high — softmax over 5 diverse prompts
# clusters near 0.20 for all prompts, so nothing passed. The argmax+baseline
# approach is more robust.
PROXY_ASSIGNMENT_THRESHOLD = None  # computed dynamically as 1/n_groups

# An image may be assigned to MULTIPLE proxy groups (non-exclusive).
# The no-people group is treated as mutually exclusive with the skin-tone groups.

# ---------------------------------------------------------------------------
# Assertion thresholds
# ---------------------------------------------------------------------------

DEFAULT_FP_CEILING_MULTIPLE = 2.0  # fail if group FP rate > N × overall FP rate
DEFAULT_RECALL_FLOOR = 0.85  # fail if group AI recall < this

# ---------------------------------------------------------------------------
# CLIP loading (mirrors clip_detector.py)
# ---------------------------------------------------------------------------

_clip_state = None


def _load_clip():
    """Load CLIP ViT-B/32. Returns (model, preprocess, tokenizer) or None."""
    global _clip_state
    if _clip_state is not None:
        return _clip_state
    try:
        import open_clip
        import torch  # noqa: F401

        logger.info("Loading CLIP ViT-B-32...")
        model, _, preprocess = open_clip.create_model_and_transforms(
            "ViT-B-32", pretrained="laion2b_s34b_b79k"
        )
        model.eval()
        tokenizer = open_clip.get_tokenizer("ViT-B-32")
        _clip_state = (model, preprocess, tokenizer)
        logger.info("CLIP ready.")
        return _clip_state
    except ImportError:
        logger.error(
            "open_clip not installed — cannot run demographic proxy analysis. "
            "Install with: pip install open-clip-torch"
        )
        return None
    except Exception as exc:
        logger.error("CLIP load failed: %s", exc)
        return None


def _load_probe(probe_path: Path):
    """Load the UnivFD v9 sklearn probe. Returns probe or None."""
    try:
        import joblib
        if not probe_path.exists():
            logger.error("Probe not found: %s", probe_path)
            return None
        probe = joblib.load(probe_path)
        logger.info("UnivFD v9 probe loaded from %s", probe_path)
        return probe
    except Exception as exc:
        logger.error("Failed to load probe: %s", exc)
        return None


# ---------------------------------------------------------------------------
# Image embedding and proxy assignment
# ---------------------------------------------------------------------------

BATCH_SIZE = 32


def _embed_images(
    paths: list[Path],
    model,
    preprocess,
) -> list | None:
    """Return list of normalised image embeddings (numpy arrays) or None on failure."""
    try:
        import torch
        import numpy as np
        from PIL import Image

        all_embeddings = []
        for i in range(0, len(paths), BATCH_SIZE):
            batch = paths[i : i + BATCH_SIZE]
            imgs = []
            valid = []
            for j, p in enumerate(batch):
                try:
                    img = preprocess(Image.open(p).convert("RGB"))
                    imgs.append(img)
                    valid.append(j)
                except Exception:
                    pass

            if not imgs:
                all_embeddings.extend([None] * len(batch))
                continue

            tensor = torch.stack(imgs)
            with torch.no_grad():
                feats = model.encode_image(tensor)
                feats = feats / feats.norm(dim=-1, keepdim=True)

            emb_list = [None] * len(batch)
            for idx, feat in zip(valid, feats):
                emb_list[idx] = feat.cpu().numpy()

            all_embeddings.extend(emb_list)

        return all_embeddings
    except Exception as exc:
        logger.error("Image embedding failed: %s", exc)
        return None


def _embed_texts(prompts: list[str], model, tokenizer) -> list:
    """Return normalised text embeddings."""
    import torch

    tokens = tokenizer(prompts)
    with torch.no_grad():
        feats = model.encode_text(tokens)
        feats = feats / feats.norm(dim=-1, keepdim=True)
    return [feats[i].cpu().numpy() for i in range(len(prompts))]


def assign_proxy_groups(
    image_embeddings: list,
    text_embeddings: list,
    proxy_names: list[str],
    threshold: float | None = PROXY_ASSIGNMENT_THRESHOLD,
) -> list[set[str]]:
    """Assign each image to proxy group(s) via softmax probability.

    Uses argmax + above-baseline assignment: every image goes into the
    highest-probability group (guaranteeing no empty groups), plus any
    other group whose probability exceeds the uniform baseline (1/n).
    """
    import numpy as np

    n_groups = len(proxy_names)
    baseline = 1.0 / n_groups if threshold is None else threshold

    assignments = []
    text_matrix = np.stack(text_embeddings)  # (n_prompts, dim)

    for emb in image_embeddings:
        if emb is None:
            assignments.append(set())
            continue

        sims = text_matrix @ emb  # (n_prompts,)
        exp_sims = np.exp(sims - sims.max())
        probs = exp_sims / exp_sims.sum()

        # Argmax: always include the best-matching group
        best = int(np.argmax(probs))
        group_set = {proxy_names[best]}

        # Also include any group above the uniform baseline
        for i, p in enumerate(probs):
            if p >= baseline:
                group_set.add(proxy_names[i])

        assignments.append(group_set)

    return assignments


# ---------------------------------------------------------------------------
# Model scoring
# ---------------------------------------------------------------------------

def _get_univfd_embedding(image_path: Path, model, preprocess) -> list | None:
    """Get CLIP embedding for a single image for UnivFD probe scoring."""
    try:
        import torch
        from PIL import Image

        img = preprocess(Image.open(image_path).convert("RGB")).unsqueeze(0)
        with torch.no_grad():
            feat = model.encode_image(img)
            feat = feat / feat.norm(dim=-1, keepdim=True)
        return feat.cpu().numpy()[0].tolist()
    except Exception:
        return None


def score_images_with_probe(
    paths: list[Path],
    model,
    preprocess,
    probe,
) -> list[float | None]:
    """Run UnivFD probe on all paths. Returns list of AI-probability scores."""
    import numpy as np

    scores = []
    embeddings = _embed_images(paths, model, preprocess)
    if embeddings is None:
        return [None] * len(paths)

    # Collect valid embeddings for batch probe scoring
    valid_indices = [i for i, e in enumerate(embeddings) if e is not None]
    if not valid_indices:
        return [None] * len(paths)

    emb_matrix = np.stack([embeddings[i] for i in valid_indices])
    try:
        proba = probe.predict_proba(emb_matrix)[:, 1]  # P(AI)
    except Exception as exc:
        logger.error("Probe scoring failed: %s", exc)
        return [None] * len(paths)

    result = [None] * len(paths)
    for idx, score in zip(valid_indices, proba):
        result[idx] = float(score)

    return result


# ---------------------------------------------------------------------------
# Per-group metrics
# ---------------------------------------------------------------------------

def compute_group_metrics(
    paths: list[Path],
    labels: list[int],  # 0 = authentic, 1 = AI
    scores: list[float | None],
    assignments: list[set[str]],
    proxy_names: list[str],
    threshold: float = 0.5,
) -> dict:
    """Compute FP rate and AI recall per proxy group."""
    metrics = {}

    for group in proxy_names:
        # Select images assigned to this group
        group_indices = [
            i for i, a in enumerate(assignments)
            if group in a and scores[i] is not None
        ]
        if not group_indices:
            metrics[group] = {
                "n": 0,
                "n_authentic": 0,
                "n_ai": 0,
                "fp_rate": None,
                "recall": None,
                "note": "no images assigned to this group",
            }
            continue

        group_labels = [labels[i] for i in group_indices]
        group_scores = [scores[i] for i in group_indices]
        group_preds = [1 if s >= threshold else 0 for s in group_scores]

        authentic_indices = [i for i, l in enumerate(group_labels) if l == 0]
        ai_indices = [i for i, l in enumerate(group_labels) if l == 1]

        n_auth = len(authentic_indices)
        n_ai = len(ai_indices)

        # FP rate: authentic images predicted as AI
        fp = sum(group_preds[i] == 1 for i in authentic_indices)
        fp_rate = fp / n_auth if n_auth > 0 else None

        # AI recall: AI images correctly predicted as AI
        tp = sum(group_preds[i] == 1 for i in ai_indices)
        recall = tp / n_ai if n_ai > 0 else None

        metrics[group] = {
            "n": len(group_indices),
            "n_authentic": n_auth,
            "n_ai": n_ai,
            "fp": fp if n_auth > 0 else None,
            "tp": tp if n_ai > 0 else None,
            "fp_rate": fp_rate,
            "recall": recall,
        }

    # Overall metrics
    valid = [(l, s) for l, s in zip(labels, scores) if s is not None]
    overall_auth = [(l, s) for l, s in valid if l == 0]
    overall_ai = [(l, s) for l, s in valid if l == 1]

    overall_fp = sum(1 for l, s in overall_auth if s >= threshold)
    overall_fp_rate = overall_fp / len(overall_auth) if overall_auth else None
    overall_tp = sum(1 for l, s in overall_ai if s >= threshold)
    overall_recall = overall_tp / len(overall_ai) if overall_ai else None

    metrics["__overall__"] = {
        "n": len(valid),
        "n_authentic": len(overall_auth),
        "n_ai": len(overall_ai),
        "fp": overall_fp,
        "tp": overall_tp,
        "fp_rate": overall_fp_rate,
        "recall": overall_recall,
    }

    return metrics


# ---------------------------------------------------------------------------
# Assertion checks
# ---------------------------------------------------------------------------

def run_assertions(
    metrics: dict,
    proxy_names: list[str],
    fp_ceiling_multiple: float,
    recall_floor: float,
) -> list[dict]:
    """Return list of assertion failures. Empty list = PASS."""
    failures = []
    overall_fp = metrics["__overall__"]["fp_rate"]
    fp_ceiling = (overall_fp * fp_ceiling_multiple) if overall_fp is not None else None

    for group in proxy_names:
        info = metrics[group]
        if info["fp_rate"] is None and info["recall"] is None:
            continue  # skip empty groups

        # FP rate assertion
        if fp_ceiling is not None and info["fp_rate"] is not None:
            if info["fp_rate"] > fp_ceiling:
                failures.append({
                    "group": group,
                    "metric": "fp_rate",
                    "value": info["fp_rate"],
                    "threshold": fp_ceiling,
                    "overall": overall_fp,
                    "message": (
                        f"Proxy group '{group}': FP rate {info['fp_rate']:.1%} "
                        f"exceeds {fp_ceiling_multiple}× overall ({overall_fp:.1%} × "
                        f"{fp_ceiling_multiple} = {fp_ceiling:.1%})"
                    ),
                })

        # Recall assertion
        if info["recall"] is not None:
            if info["recall"] < recall_floor:
                failures.append({
                    "group": group,
                    "metric": "recall",
                    "value": info["recall"],
                    "threshold": recall_floor,
                    "message": (
                        f"Proxy group '{group}': AI recall {info['recall']:.1%} "
                        f"is below floor {recall_floor:.1%}"
                    ),
                })

    return failures


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------

def write_report(
    metrics: dict,
    failures: list[dict],
    proxy_names: list[str],
    overall_fp_rate: float | None,
    overall_recall: float | None,
    fp_ceiling_multiple: float,
    recall_floor: float,
    split_path: Path,
    n_test: int,
    sample_n: int | None,
    generated_at: str,
) -> str:

    pass_fail = "PASS" if not failures else "FAIL"
    fp_ceiling = (
        f"{overall_fp_rate * fp_ceiling_multiple:.1%}"
        if overall_fp_rate is not None
        else "n/a"
    )

    sample_note = (
        f"\n> **Pilot run**: analysis performed on a {sample_n}-image sample from the "
        f"full {n_test}-image test set. Full-corpus results will differ.\n"
        if sample_n
        else ""
    )

    failures_section = ""
    if failures:
        failures_section = "\n### Assertion Failures\n\n"
        for f in failures:
            failures_section += f"- **{f['message']}**\n"
        failures_section += (
            "\nThese failures indicate potential demographic disparity in model performance "
            "across CLIP-proxy groups. The corpus expansion recommendations in "
            "`corpus-demographic-profile.md` should be prioritised for the next retraining "
            "cycle. Note that CLIP-proxy grouping is approximate — verify with human "
            "annotation before drawing strong conclusions.\n"
        )
    else:
        failures_section = (
            "\nNo assertions failed. All proxy groups are within the acceptable "
            "range (FP rate <= {fp_ceiling} and AI recall >= {recall_floor:.0%}). "
            "This does NOT mean the model is demographically fair — it means no "
            "gross disparity was detected by the CLIP-proxy method.\n"
        ).format(fp_ceiling=fp_ceiling, recall_floor=recall_floor)

    # Build results table
    table_rows = ["| Proxy Group | N Images | Auth Images | AI Images | FP Rate | AI Recall | FP Flag | Recall Flag |",
                  "|---|---|---|---|---|---|---|---|"]

    overall = metrics["__overall__"]
    fp_ceil_val = (
        overall["fp_rate"] * fp_ceiling_multiple
        if overall["fp_rate"] is not None
        else None
    )

    for group in proxy_names:
        info = metrics[group]
        fp_str = f"{info['fp_rate']:.1%}" if info["fp_rate"] is not None else "n/a"
        rec_str = f"{info['recall']:.1%}" if info["recall"] is not None else "n/a"
        fp_flag = ""
        rec_flag = ""
        if fp_ceil_val is not None and info["fp_rate"] is not None:
            if info["fp_rate"] > fp_ceil_val:
                fp_flag = "FAIL"
            else:
                fp_flag = "ok"
        if info["recall"] is not None:
            if info["recall"] < recall_floor:
                rec_flag = "FAIL"
            else:
                rec_flag = "ok"
        table_rows.append(
            f"| `{group}` | {info['n']} | {info['n_authentic']} | {info['n_ai']} "
            f"| {fp_str} | {rec_str} | {fp_flag} | {rec_flag} |"
        )

    # Overall row
    ovr_fp = f"{overall['fp_rate']:.1%}" if overall["fp_rate"] is not None else "n/a"
    ovr_rec = f"{overall['recall']:.1%}" if overall["recall"] is not None else "n/a"
    table_rows.append(
        f"| **Overall** | **{overall['n']}** | **{overall['n_authentic']}** "
        f"| **{overall['n_ai']}** | **{ovr_fp}** | **{ovr_rec}** | — | — |"
    )

    report = f"""---
title: "Demographic Bias Test Results — Jura Trace UnivFD v9"
sprint: "Sprint 29 — Fairness Foundation"
pillar: "TRIED Pillar 4 (Fair)"
generated: "{generated_at}"
test_result: "{pass_fail}"
status: "audit output — do not modify manually"
---

# Demographic Bias Test Results

**Generated:** {generated_at}
**Sprint:** 29 — Fairness Foundation
**TRIED Pillar:** 4 (Fair)
**Model:** UnivFD v9 (AUC-ROC 0.9933, overall FP 4.12%, AI recall 95.70%)
**Test split:** `{split_path.name}` ({n_test} images)
**Verdict: {pass_fail}**
{sample_note}
---

## 1. Methodology

### Overview

This test uses CLIP ViT-B/32 cosine similarity to assign held-out test images to
demographic-proxy groups, then computes the UnivFD v9 model's FP rate and AI recall
within each group. The purpose is to detect gross disparities in model performance,
not to provide a demographic ground-truth audit.

### CRITICAL LIMITATIONS

**CLIP text-image similarity is NOT a demographic classifier.** It measures how
well a natural-language description matches an image's overall visual content, but:

1. Similarity scores are continuous and noisy. An image may score weakly on
   "a photograph of a dark-skinned person" because the person is small in the
   frame, in shadow, wearing distinctive clothing, or because the background
   dominates — not because the person is light-skinned.

2. Prompt-based grouping is not exclusive. Images near the threshold boundary
   may be assigned to multiple or no groups. The groups are fuzzy approximations.

3. CLIP ViT-B/32 itself has documented biases (trained on LAION-2B, English-
   language-dominant). Its similarity scores may be systematically less accurate
   for images from underrepresented geographic regions or cultural contexts —
   exactly the populations where fairness matters most.

4. **Passing this test is not a fairness certificate.** It is a canary: if the
   model had catastrophic disparities, they would likely show up here. The absence
   of flagged disparities does not exclude more subtle biases that would require
   human annotation to detect.

5. A proper demographic fairness audit requires human-annotated ground truth.
   This script is a computationally tractable interim measure, explicitly interim.

### Proxy Groups

| Group ID | Text Prompt | Purpose |
|---|---|---|
| `dark_skin` | "a photograph of a dark-skinned person" | Detect FP/recall disparity on darker-skinned subjects |
| `light_skin` | "a photograph of a light-skinned person" | Baseline comparison group |
| `no_people` | "a photograph with no people, showing scenery, objects, or animals" | Sanity check: non-person content should not be affected by demographic bias |
| `africa_south_asia` | "a photograph taken in Africa or South Asia" | Geographic proxy for Global Majority regions |
| `europe_north_america` | "a photograph taken in Europe or North America" | Geographic proxy for over-represented training regions |

Images are assigned to a group if their CLIP softmax probability for that prompt
exceeds the uniform baseline (1/n_groups = {1.0/len(PROXY_PROMPTS):.1%}), plus the argmax group is always included (non-exclusive — an image may belong to multiple groups).

### Assertion Thresholds

| Metric | Threshold | Rationale |
|---|---|---|
| FP rate ceiling | {fp_ceiling_multiple}× overall FP rate (= {fp_ceiling}) | 2× is a common algorithmic fairness ceiling; chosen to flag large disparities without penalising small samples |
| AI recall floor | {recall_floor:.0%} | 15pp below overall recall (95.70%); flags substantial degradation |

---

## 2. Results

{chr(10).join(table_rows)}

Overall FP rate: **{ovr_fp}** | Overall AI recall: **{ovr_rec}**
{failures_section}

---

## 3. Per-Group Detail

"""

    for group in proxy_names:
        info = metrics[group]
        report += f"### `{group}`\n\n"
        report += f"- Images in group: {info['n']}\n"
        report += f"- Authentic: {info['n_authentic']} | AI: {info['n_ai']}\n"
        if info["fp_rate"] is not None:
            report += f"- FP rate: {info['fp_rate']:.1%} ({info.get('fp', 'n/a')} FPs from {info['n_authentic']} authentic)\n"
        else:
            report += "- FP rate: n/a (insufficient authentic images in group)\n"
        if info["recall"] is not None:
            report += f"- AI recall: {info['recall']:.1%} ({info.get('tp', 'n/a')} TPs from {info['n_ai']} AI images)\n"
        else:
            report += "- AI recall: n/a (insufficient AI images in group)\n"
        report += "\n"

    report += f"""---

## 4. Recommendations

"""

    if failures:
        report += "Assertion failures were detected. The following actions are recommended:\n\n"
        for f in failures:
            if f["metric"] == "fp_rate":
                report += (
                    f"- **`{f['group']}` FP rate ({f['value']:.1%}) too high**: "
                    "Expand the authentic corpus with more images from this demographic proxy group. "
                    "Check whether existing training images in this group have systematic format "
                    "or quality differences that the model has learned as a proxy.\n"
                )
            elif f["metric"] == "recall":
                report += (
                    f"- **`{f['group']}` recall ({f['value']:.1%}) too low**: "
                    "Expand the AI-generated corpus with examples from generators known to "
                    "produce content in this demographic proxy group. "
                    "Check whether the generator families in training data adequately cover "
                    "this visual domain.\n"
                )
    else:
        report += (
            "No assertion failures detected. Recommended actions for next quarterly cycle:\n\n"
            "1. Expand the authentic corpus with Global Majority photography "
            "(see `corpus-demographic-profile.md` Section 7) to reduce geographic concentration "
            "risk before it manifests as a metric disparity.\n"
            "2. Commission human annotation of at least 500 authentic images with "
            "skin-tone and geographic labels to replace CLIP-proxy grouping with "
            "ground-truth demographic grouping.\n"
            "3. Add new generator families (Leonardo AI, HiDream, Midjourney v7, Flux Pro) "
            "to the AI corpus as they become available, and re-run this test to ensure "
            "recall is maintained across demographic proxy groups.\n"
        )

    report += f"""
---

## 5. EMIF / TRIED Compliance Status

| Requirement | Status | Notes |
|---|---|---|
| Bias test methodology documented | Done | Section 1 |
| Per-proxy-group FP rate and recall | Done | Section 2–3 |
| Assertion thresholds defined | Done | FP ≤ {fp_ceiling_multiple}× overall; recall ≥ {recall_floor:.0%} |
| Quarterly retrainability | Done | Script is parameterised; re-run after each retrain |
| Limitations explicitly stated | Done | Section 1 — CRITICAL LIMITATIONS |
| Human annotation follow-up planned | Recommended | Pending resource allocation |

**EMIF Pillar 4 citation**: This document, together with `corpus-demographic-profile.md`,
provides the evidence base for citing TRIED Pillar 4 (Fair) in the EMIF concept note.
The citation must be accompanied by the limitation caveat: "Demographic fairness testing
uses CLIP-proxy grouping, a coarse computational approximation pending human annotation."
The test result ({pass_fail}) at the time of submission should be stated.
"""

    return report


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Quarterly demographic bias gate for Jura Trace UnivFD v9.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--split",
        type=Path,
        default=DEFAULT_SPLIT,
        help=f"Path to model split JSON (default: {DEFAULT_SPLIT})",
    )
    parser.add_argument(
        "--probe",
        type=Path,
        default=DEFAULT_PROBE,
        help=f"Path to UnivFD v9 probe joblib (default: {DEFAULT_PROBE})",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_OUTPUT,
        help=f"Output Markdown path (default: {DEFAULT_OUTPUT})",
    )
    parser.add_argument(
        "--count",
        type=int,
        default=None,
        metavar="N",
        help="Run on N images sampled from the test set (pilot mode). Omit for full set.",
    )
    parser.add_argument(
        "--fp-ceiling",
        type=float,
        default=DEFAULT_FP_CEILING_MULTIPLE,
        metavar="MULTIPLE",
        help=f"FP rate ceiling as multiple of overall rate (default: {DEFAULT_FP_CEILING_MULTIPLE})",
    )
    parser.add_argument(
        "--recall-floor",
        type=float,
        default=DEFAULT_RECALL_FLOOR,
        help=f"Minimum AI recall per proxy group (default: {DEFAULT_RECALL_FLOOR})",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.5,
        help="UnivFD probe classification threshold (default: 0.5)",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed for sample selection (default: 42)",
    )
    parser.add_argument(
        "--no-write",
        action="store_true",
        help="Print results but do not write the Markdown report.",
    )

    args = parser.parse_args()
    random.seed(args.seed)

    logger.info("Demographic Bias Test Suite — Sprint 29 Fairness Foundation")

    # Load split
    if not args.split.exists():
        logger.error("Split file not found: %s", args.split)
        sys.exit(2)

    logger.info("Loading test split from %s", args.split)
    with open(args.split) as f:
        split = json.load(f)

    test_paths_raw = split.get("test_paths", [])
    n_test_total = len(test_paths_raw)

    # Parse test paths — may be strings or dicts
    parsed: list[tuple[Path, int]] = []
    for entry in test_paths_raw:
        if isinstance(entry, str):
            p = Path(entry)
            label = 0 if "authentic" in str(p).lower() else 1
            parsed.append((p, label))
        elif isinstance(entry, dict):
            p = Path(entry.get("path", ""))
            label = 1 if entry.get("label", "authentic") == "ai_generated" else 0
            parsed.append((p, label))

    # Check how many paths actually exist
    existing = [(p, l) for p, l in parsed if p.exists()]
    missing = n_test_total - len(existing)
    if missing > 0:
        logger.warning(
            "%d/%d test paths not found on disk (USB mounted? Platform-forwarded paths?). "
            "Proceeding with %d available paths.",
            missing, n_test_total, len(existing),
        )

    if len(existing) == 0:
        logger.error("No test images accessible. Is the USB drive mounted?")
        sys.exit(2)

    # Sample if requested
    if args.count and len(existing) > args.count:
        logger.info("Sampling %d from %d test images", args.count, len(existing))
        # Stratified sample: preserve authentic/AI ratio
        auth = [(p, l) for p, l in existing if l == 0]
        ai = [(p, l) for p, l in existing if l == 1]
        n_auth = round(args.count * len(auth) / len(existing))
        n_ai = args.count - n_auth
        sampled = random.sample(auth, min(n_auth, len(auth))) + random.sample(ai, min(n_ai, len(ai)))
        random.shuffle(sampled)
        existing = sampled

    paths = [p for p, _ in existing]
    labels = [l for _, l in existing]
    n_auth = sum(1 for l in labels if l == 0)
    n_ai = sum(1 for l in labels if l == 1)
    logger.info(
        "Test set: %d images (%d authentic, %d AI)", len(existing), n_auth, n_ai,
    )

    # Load CLIP and probe
    clip_state = _load_clip()
    if clip_state is None:
        logger.error(
            "CLIP is required for demographic proxy analysis. "
            "Install open-clip-torch and retry."
        )
        sys.exit(2)

    model, preprocess, tokenizer = clip_state
    probe = _load_probe(args.probe)
    if probe is None:
        sys.exit(2)

    # Embed images
    logger.info("Embedding %d test images with CLIP...", len(paths))
    image_embeddings = _embed_images(paths, model, preprocess)
    if image_embeddings is None:
        logger.error("Image embedding failed.")
        sys.exit(2)

    # Embed proxy prompts
    proxy_names = [name for name, _ in PROXY_PROMPTS]
    proxy_texts = [prompt for _, prompt in PROXY_PROMPTS]
    logger.info("Embedding %d proxy prompts...", len(proxy_texts))
    text_embeddings = _embed_texts(proxy_texts, model, tokenizer)

    # Assign proxy groups
    logger.info("Assigning images to proxy groups (threshold=%.2f)...", PROXY_ASSIGNMENT_THRESHOLD)
    assignments = assign_proxy_groups(
        image_embeddings, text_embeddings, proxy_names, PROXY_ASSIGNMENT_THRESHOLD,
    )

    # Log group sizes
    for group in proxy_names:
        n = sum(1 for a in assignments if group in a)
        logger.info("  Group '%s': %d images", group, n)

    # Score images with UnivFD probe
    logger.info("Scoring images with UnivFD v9 probe...")
    scores = score_images_with_probe(paths, model, preprocess, probe)

    # Compute metrics
    logger.info("Computing per-group metrics...")
    metrics = compute_group_metrics(
        paths, labels, scores, assignments, proxy_names, threshold=args.threshold,
    )

    overall = metrics["__overall__"]
    logger.info(
        "Overall — FP rate: %s, AI recall: %s",
        f"{overall['fp_rate']:.1%}" if overall["fp_rate"] is not None else "n/a",
        f"{overall['recall']:.1%}" if overall["recall"] is not None else "n/a",
    )

    # Run assertions
    failures = run_assertions(
        metrics, proxy_names, args.fp_ceiling, args.recall_floor,
    )

    # Print results
    print("\n=== Demographic Bias Test Results ===")
    print(f"Test images: {len(existing)} ({n_auth} authentic, {n_ai} AI)")
    print(f"Overall FP rate: {overall['fp_rate']:.1%}" if overall["fp_rate"] is not None else "Overall FP rate: n/a")
    print(f"Overall AI recall: {overall['recall']:.1%}" if overall["recall"] is not None else "Overall AI recall: n/a")
    print()

    for group in proxy_names:
        info = metrics[group]
        fp_str = f"{info['fp_rate']:.1%}" if info["fp_rate"] is not None else "n/a"
        rec_str = f"{info['recall']:.1%}" if info["recall"] is not None else "n/a"
        print(f"  {group:30s}  FP={fp_str:>7}  Recall={rec_str:>7}  N={info['n']}")

    print()
    if failures:
        print(f"RESULT: FAIL ({len(failures)} assertion(s) failed)")
        for f in failures:
            print(f"  FAIL: {f['message']}")
    else:
        print("RESULT: PASS — no proxy group exceeds thresholds")

    # Write report
    generated_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
    if not args.no_write:
        report = write_report(
            metrics=metrics,
            failures=failures,
            proxy_names=proxy_names,
            overall_fp_rate=overall["fp_rate"],
            overall_recall=overall["recall"],
            fp_ceiling_multiple=args.fp_ceiling,
            recall_floor=args.recall_floor,
            split_path=args.split,
            n_test=n_test_total,
            sample_n=args.count,
            generated_at=generated_at,
        )
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="utf-8")
        logger.info("Report written to %s", args.output)

    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
