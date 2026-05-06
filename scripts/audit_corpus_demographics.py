#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Sprint 29 Fairness Foundation: Corpus Demographic Audit

Inventories the training corpus and produces a demographic profile for TRIED
Pillar 4 (Fair) compliance and the EMIF grant application.

What this script does:
  1. Walks authentic/ and ai_generated/ corpus directories.
  2. Reports image counts, licences, and geographic proxy per subdirectory.
  3. Uses CLIP ViT-B/32 (if available) to classify images as containing people
     vs. not — a COARSE binary filter only. This is NOT a demographic classifier.
  4. Flags subdirectories where >80% of person-containing images come from a
     single source (concentration risk).
  5. Writes docs/fairness/corpus-demographic-profile.md.

Limitations (explicitly stated, not elided):
  - CLIP person-detection is a rough binary signal. It misses partial figures,
    groups in complex scenes, and may false-positive on mannequins and art.
  - Directory structure carries no ground-truth demographic information.
    Geographic proxies are qualitative inferences, not measurements.
  - Skin tone, gender, and age cannot be determined from directory metadata.
    This script does NOT attempt to classify individuals.

Usage:
    python3 scripts/audit_corpus_demographics.py --help
    python3 scripts/audit_corpus_demographics.py --count 50   # pilot sample
    python3 scripts/audit_corpus_demographics.py              # full run (USB must be mounted)
    python3 scripts/audit_corpus_demographics.py --no-clip    # skip CLIP, counts only
"""

import argparse
import logging
import random
import sys
from datetime import datetime, timezone
from pathlib import Path

from PIL import Image as PILImage

logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

CORPUS_ROOT = Path("/Volumes/MAC SSD/Training Data/corpus/training")
AUTHENTIC_ROOT = CORPUS_ROOT / "authentic"
AI_ROOT = CORPUS_ROOT / "ai_generated"

REPO_ROOT = Path(__file__).resolve().parent.parent
OUTPUT_DIR = REPO_ROOT / "docs" / "fairness"
OUTPUT_MD = OUTPUT_DIR / "corpus-demographic-profile.md"

IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".bmp", ".tiff", ".tif"}

# ---------------------------------------------------------------------------
# Subdirectory metadata tables
# ---------------------------------------------------------------------------

# Licence per authentic subdirectory.
# Sources: build_splice_corpus_v2.py CC_BY_DIRS, expand_corpus_v2.py, corpus docs.
AUTHENTIC_LICENCES: dict[str, str] = {
    "coco": "CC-BY 4.0 (COCO val2017)",
    "coco_extra": "CC-BY 4.0 (COCO additional sample)",
    "coco_extra2": "CC-BY 4.0 (COCO additional sample)",
    "coco_train": "CC-BY 4.0 (COCO train2017)",
    "flickr30k": "Option C corpus licence (non-commercial cleared — see docs/decisions/option-c-corpus-strategy.md)",
    "flickr8k": "Option C corpus licence (non-commercial cleared — see docs/decisions/option-c-corpus-strategy.md)",
    "openimages": "CC-BY 2.0 (Open Images v7)",
    "unsplash_photos": "Unsplash Licence (free for commercial use, attribution encouraged)",
    "wikimedia_photos": "CC-BY / CC-BY-SA (per individual file — Wikimedia featured images)",
    "imagenet_vl": "ImageNet licence (non-commercial research — see corpus strategy note)",
    "imagenet_extra": "ImageNet licence (non-commercial research — see corpus strategy note)",
    "conceptual_12m": "CC-BY 4.0 (Conceptual Captions 12M)",
    "celeba": "Research/non-commercial (CelebA licence — restricted, face images only)",
    "camera_dcim": "Proprietary / owned by Jura Labs (original camera captures)",
    "google_photos": "Proprietary / owned by contributor (single photographer's library)",
    "aesthetic_photos": "Mixed — source-dependent; assumed CC0 / public domain",
    "war_conflict": "Mixed — press/documentary; assumed editorial use",
    "wildlife_macro": "Mixed — assumed CC0 / public domain or contributor-owned",
}

# Licence per AI-generated subdirectory.
AIGENERATED_LICENCES: dict[str, str] = {
    "civitai_sfw": "Civitai platform terms (SFW community outputs — model-dependent)",
    "dalle3": "OpenAI Terms of Service (outputs owned by requester)",
    "diffusiondb": "CC0 1.0 Universal (DiffusionDB dataset)",
    "elsa": "ELSA 1M dataset licence (research permitted)",
    "elsa_diverse": "ELSA 1M dataset licence (research permitted)",
    "flux_dev": "FLUX.1-dev Community Licence (non-commercial permitted)",
    "gemini": "Google Terms of Service (outputs owned by requester)",
    "grok": "xAI Terms of Service (outputs owned by requester)",
    "grok_aurora": "xAI Terms of Service (outputs owned by requester)",
    "midjourney_v6": "Midjourney Community Showcase / ToS (pro plan outputs — commercial permitted)",
    "artbench": "ArtBench dataset licence (research permitted)",
    "sdxl_turbo": "SDXL-Turbo community licence",
    "sd_v14": "CreativeML Open RAIL-M",
    "sd15": "CreativeML Open RAIL-M",
    "sd21": "CreativeML Open RAIL-M",
    "ssd1b": "Apache 2.0 (SegmindSD 1B)",
    "synthetic_faces": "Mixed — see individual manifests",
    "synthetic_faces_tpdne": "TPDNE (This Person Does Not Exist) — public access",
    "tpdne_faces": "TPDNE — public access",
    "user_ai": "Contributor-submitted — assumed researcher-owned outputs",
    "wikimedia_ai": "CC-BY / CC-BY-SA per file (Wikimedia AI-labelled uploads)",
    "aragon_ai": "Aragon AI platform terms",
    "cifake": "CIFAKE dataset licence (research permitted)",
    "foxy_ai": "Foxy AI platform terms",
    "leonardo": "Leonardo AI platform terms (commercial plan outputs)",
}

# Geographic / cultural proxy per authentic subdirectory.
# These are qualitative inferences from source characteristics, not measurements.
# "unknown" means no reliable inference is possible.
AUTHENTIC_GEO_PROXY: dict[str, str] = {
    "coco": "Predominantly US/Western European settings (MS COCO sourced from Flickr, US-centric object detection tasks). Global minority communities likely under-represented.",
    "coco_extra": "Same sourcing as coco — US/Western European bias expected.",
    "coco_extra2": "Same sourcing as coco — US/Western European bias expected.",
    "coco_train": "Same sourcing as coco — US/Western European bias expected.",
    "flickr30k": "Primarily US and Western European (Flickr 2014 snapshot; US English-speaking communities dominant on Flickr at that time).",
    "flickr8k": "Primarily US and Western European — same Flickr sourcing as Flickr30k.",
    "openimages": "More geographically diverse than COCO (Google Open Images includes African, Asian, and Latin American scenes) but still Western-skewed in validation set.",
    "unsplash_photos": "Global photographer pool but heavily English-speaking-market users; urban/lifestyle bias; Global Majority communities likely under-represented.",
    "wikimedia_photos": "Geographically diverse (Wikimedia editors worldwide) but art/heritage content may over-represent European cultural artefacts.",
    "imagenet_vl": "Primarily object/scene images (non-person content dominant); geographic diversity of person-containing subset unknown.",
    "imagenet_extra": "Same as imagenet_vl — geographic diversity unknown.",
    "conceptual_12m": "Web-crawled English-language caption pairs — US/Western European bias expected.",
    "celeba": "Chinese entertainment industry face images (predominantly East Asian subjects); geographically and demographically narrow.",
    "camera_dcim": "Single photographer (Jura Labs founder) — UK locations, European and some Global Majority subjects. Very small sample (59 images). Unknown.",
    "google_photos": "Single photographer's personal library — specific geographic scope unknown. Cannot assess.",
    "aesthetic_photos": "Unknown — source not documented.",
    "war_conflict": "Likely skews toward conflict zones in Global Majority regions (Middle East, sub-Saharan Africa, South/Southeast Asia) based on global press coverage patterns. This may create unexpected demographic concentration in the person-containing authentic subset.",
    "wildlife_macro": "Non-person content dominant (wildlife/macro). Geographic origin unknown.",
}

AI_GEO_PROXY: dict[str, str] = {
    "civitai_sfw": "Global community uploads but English-language-dominant platform; aesthetic bias toward Western anime/fantasy styles. Person content likely skews toward lighter-skinned, East Asian, and fantasy-coded appearances.",
    "dalle3": "DALL-E 3 outputs are prompt-driven; without prompt logs geographic/demographic distribution is unknown. OpenAI has published fairness studies on output diversity for default prompts.",
    "diffusiondb": "Stable Diffusion 1.x outputs; DiffusionDB prompts are community-submitted, English-dominant. Demographic and geographic distribution unknown.",
    "elsa": "ELSA 1M is a European project dataset (EU Horizon); outputs drawn from SD/DALL-E mix. European editorial context.",
    "elsa_diverse": "Same as elsa.",
    "flux_dev": "Flux.1-dev outputs; training data and demographic biases not fully disclosed by Black Forest Labs.",
    "gemini": "Google Gemini outputs; Google has disclosed diversity interventions in image generation.",
    "grok": "xAI Grok outputs; training data and demographic biases not publicly disclosed.",
    "grok_aurora": "xAI Grok Aurora outputs; same disclosure limitations as grok.",
    "midjourney_v6": "Midjourney v6 community showcase outputs; aesthetic bias toward stylised Western art styles.",
    "artbench": "ArtBench dataset — art-style generations. Non-photorealistic content; demographic signals attenuated.",
    "sdxl_turbo": "SDXL-Turbo outputs; same Stable Diffusion lineage limitations as diffusiondb.",
    "sd_v14": "Stable Diffusion 1.4 — older model with documented bias toward Western/lighter-skinned faces.",
    "sd15": "Stable Diffusion 1.5 — same lineage as sd_v14.",
    "sd21": "Stable Diffusion 2.1 — improved aesthetic diversity but demographic biases persist.",
    "ssd1b": "Segmind SD 1B — SDXL-derived, similar bias profile.",
    "synthetic_faces": "Mixed provenance — unknown.",
    "tpdne_faces": "StyleGAN-based face generation; trained on FFHQ (Flickr faces, predominantly lighter-skinned). Known bias toward lighter skin tones and Western facial aesthetics.",
    "user_ai": "Contributor-submitted — unknown demographics and geographic distribution.",
    "wikimedia_ai": "AI-labelled Wikimedia uploads — diverse subject matter but very small sample.",
    "aragon_ai": "Unknown — no images present in current corpus.",
    "cifake": "CIFAKE dataset (CIFAR-10 + SD synthetic) — primarily object/scene content, limited person content.",
    "foxy_ai": "Unknown — no images present in current corpus.",
    "leonardo": "Leonardo AI platform — unknown — no images present in current corpus.",
}

# ---------------------------------------------------------------------------
# CLIP person-detection
# ---------------------------------------------------------------------------

PERSON_PROMPT = "a photograph of a person"
NO_PERSON_PROMPT = "a photograph of scenery, objects, or animals without people"

CLIP_BATCH_SIZE = 64


def _load_clip():
    """Attempt to load CLIP. Returns (model, preprocess, tokenizer) or None."""
    try:
        import open_clip
        import torch  # noqa: F401

        logger.info("Loading CLIP ViT-B-32 (may take a moment)...")
        model, _, preprocess = open_clip.create_model_and_transforms(
            "ViT-B-32", pretrained="laion2b_s34b_b79k"
        )
        model.eval()
        tokenizer = open_clip.get_tokenizer("ViT-B-32")
        logger.info("CLIP model ready.")
        return model, preprocess, tokenizer
    except ImportError:
        logger.warning(
            "open_clip not installed — CLIP person-detection unavailable. "
            "Run: pip install open-clip-torch"
        )
        return None
    except Exception as exc:
        logger.warning("CLIP load failed: %s", exc)
        return None


def _clip_person_scores(
    paths: list[Path],
    model,
    preprocess,
    tokenizer,
) -> list[float]:
    """Return cosine-similarity score for PERSON_PROMPT for each path.

    Score > 0.5 (softmax over two prompts) is treated as 'contains person'.
    Returns a list of floats in [0, 1].
    """
    import torch

    texts = tokenizer([PERSON_PROMPT, NO_PERSON_PROMPT])
    with torch.no_grad():
        text_features = model.encode_text(texts)
        text_features = text_features / text_features.norm(dim=-1, keepdim=True)

    scores = []
    for i in range(0, len(paths), CLIP_BATCH_SIZE):
        batch_paths = paths[i : i + CLIP_BATCH_SIZE]
        images = []
        valid_indices = []
        for j, p in enumerate(batch_paths):
            try:
                img = preprocess(PILImage.open(p).convert("RGB"))
                images.append(img)
                valid_indices.append(j)
            except Exception:
                pass

        if not images:
            scores.extend([0.0] * len(batch_paths))
            continue

        import torch

        img_tensor = torch.stack(images)
        with torch.no_grad():
            img_features = model.encode_image(img_tensor)
            img_features = img_features / img_features.norm(dim=-1, keepdim=True)
            logits = (img_features @ text_features.T) * 100.0
            probs = logits.softmax(dim=-1)

        batch_scores = [0.0] * len(batch_paths)
        for idx, prob in zip(valid_indices, probs):
            batch_scores[idx] = float(prob[0])  # prob of PERSON_PROMPT

        scores.extend(batch_scores)

    return scores


# ---------------------------------------------------------------------------
# Corpus collection
# ---------------------------------------------------------------------------

def collect_images(root: Path, sample_n: int | None = None) -> dict[str, list[Path]]:
    """Return {subdir_name: [image_paths]} for all subdirectories under root.

    Skips macOS AppleDouble resource-fork files (prefixed with '._').
    Returns files at root level under key '__root__'.
    """
    result: dict[str, list[Path]] = {}

    if not root.exists():
        logger.error("Directory not found: %s", root)
        return result

    # Images directly in root (legacy loose files)
    root_images = [
        f for f in root.iterdir()
        if f.is_file()
        and f.suffix.lower() in IMAGE_EXTENSIONS
        and not f.name.startswith("._")
    ]
    if root_images:
        result["__root__"] = sorted(root_images)

    for subdir in sorted(root.iterdir()):
        if not subdir.is_dir() or subdir.name.startswith("."):
            continue
        imgs = sorted(
            f for f in subdir.rglob("*")
            if f.is_file()
            and f.suffix.lower() in IMAGE_EXTENSIONS
            and not f.name.startswith("._")
        )
        if imgs:
            result[subdir.name] = imgs

    # Apply sample limit: keep proportional representation across subdirs
    if sample_n is not None and sample_n > 0:
        total = sum(len(v) for v in result.values())
        if total > sample_n:
            sampled: dict[str, list[Path]] = {}
            for k, v in result.items():
                keep = max(1, round(len(v) * sample_n / total))
                sampled[k] = random.sample(v, min(keep, len(v)))
            return sampled

    return result


# ---------------------------------------------------------------------------
# Analysis
# ---------------------------------------------------------------------------

PERSON_THRESHOLD = 0.5  # softmax score above which image is flagged 'contains person'
CONCENTRATION_THRESHOLD = 0.80  # flag if >80% of person images from single source


def analyse(
    authentic_images: dict[str, list[Path]],
    ai_images: dict[str, list[Path]],
    clip_state: tuple | None,
    person_threshold: float = PERSON_THRESHOLD,
) -> dict:
    """Run person-content detection and compile statistics."""

    results = {
        "authentic": {},
        "ai_generated": {},
    }

    clip_available = clip_state is not None
    model, preprocess, tokenizer = clip_state if clip_state else (None, None, None)

    for class_name, image_map, licence_map, geo_map in [
        ("authentic", authentic_images, AUTHENTIC_LICENCES, AUTHENTIC_GEO_PROXY),
        ("ai_generated", ai_images, AIGENERATED_LICENCES, AI_GEO_PROXY),
    ]:
        for subdir, paths in image_map.items():
            n = len(paths)
            person_count = 0
            person_paths = []

            if clip_available and n > 0:
                scores = _clip_person_scores(paths, model, preprocess, tokenizer)
                person_flags = [s >= person_threshold for s in scores]
                person_count = sum(person_flags)
                person_paths = [str(p) for p, f in zip(paths, person_flags) if f]
                logger.info(
                    "  %s / %s: %d/%d person images detected",
                    class_name, subdir, person_count, n,
                )

            results[class_name][subdir] = {
                "n": n,
                "licence": licence_map.get(subdir, "Unknown"),
                "geo_proxy": geo_map.get(subdir, "Unknown — not yet assessed"),
                "person_count": person_count if clip_available else None,
                "person_fraction": (person_count / n) if (clip_available and n > 0) else None,
                "clip_available": clip_available,
            }

    return results


def compute_concentration_risk(results: dict) -> list[dict]:
    """Identify subdirectories contributing >CONCENTRATION_THRESHOLD of
    person-containing images within their class.
    """
    flags = []
    for class_name in ("authentic", "ai_generated"):
        subdirs = results[class_name]
        total_persons = sum(
            v["person_count"] or 0
            for v in subdirs.values()
            if v["person_count"] is not None
        )
        if total_persons == 0:
            continue
        for subdir, info in subdirs.items():
            pc = info.get("person_count") or 0
            if pc == 0:
                continue
            fraction = pc / total_persons
            if fraction > CONCENTRATION_THRESHOLD:
                flags.append({
                    "class": class_name,
                    "subdir": subdir,
                    "person_count": pc,
                    "fraction_of_class_persons": fraction,
                })
    return flags


# ---------------------------------------------------------------------------
# Report generation
# ---------------------------------------------------------------------------

def _licence_table(subdirs: dict, class_name: str) -> str:
    lines = [
        "| Subdirectory | Images | Licence | Geographic Proxy | Person Content |",
        "|---|---|---|---|---|",
    ]
    for subdir, info in sorted(subdirs.items()):
        n = info["n"]
        lic = info["licence"]
        geo = info["geo_proxy"].split(".")[0]  # first sentence only for table
        if info["clip_available"] and info["person_fraction"] is not None:
            pct = f"{info['person_fraction']*100:.0f}%"
        else:
            pct = "n/a (CLIP unavailable)"
        lines.append(f"| `{subdir}` | {n:,} | {lic} | {geo} | {pct} |")
    return "\n".join(lines)


def write_report(
    results: dict,
    concentration_flags: list[dict],
    authentic_images: dict[str, list[Path]],
    ai_images: dict[str, list[Path]],
    clip_available: bool,
    sample_n: int | None,
    generated_at: str,
) -> str:
    """Produce the Markdown report content."""

    auth_total = sum(len(v) for v in authentic_images.values())
    ai_total = sum(len(v) for v in ai_images.values())
    total = auth_total + ai_total

    auth_person_total = sum(
        v["person_count"] or 0
        for v in results["authentic"].values()
        if v["person_count"] is not None
    )
    ai_person_total = sum(
        v["person_count"] or 0
        for v in results["ai_generated"].values()
        if v["person_count"] is not None
    )

    sample_note = (
        f"\n> **Pilot run**: analysis performed on a {sample_n}-image sample "
        f"(proportional across subdirectories). Full-corpus numbers will differ.\n"
        if sample_n
        else ""
    )

    clip_note = (
        "CLIP ViT-B/32 person-detection was used for person-content classification."
        if clip_available
        else (
            "**CLIP was not available at audit time.** Person-content fractions could not "
            "be computed. Install `open-clip-torch` and re-run for complete results."
        )
    )

    concentration_section = ""
    if concentration_flags:
        concentration_section = "\n## 5. Concentration Risk Flags\n\n"
        concentration_section += (
            "The following subdirectories contribute more than 80% of person-containing "
            "images within their class. This represents a demographic concentration risk: "
            "if that source has systematic demographic gaps, those gaps propagate to the "
            "majority of person-depicting training examples.\n\n"
        )
        concentration_section += "| Class | Subdirectory | Person Images | Share of Class |\n"
        concentration_section += "|---|---|---|---|\n"
        for flag in concentration_flags:
            concentration_section += (
                f"| {flag['class']} | `{flag['subdir']}` "
                f"| {flag['person_count']:,} "
                f"| {flag['fraction_of_class_persons']*100:.1f}% |\n"
            )
    else:
        if clip_available:
            concentration_section = (
                "\n## 5. Concentration Risk Flags\n\n"
                "No single subdirectory accounts for more than 80% of person-containing "
                "images within its class. No concentration risk detected at this threshold.\n"
            )
        else:
            concentration_section = (
                "\n## 5. Concentration Risk Flags\n\n"
                "Concentration risk analysis requires CLIP person-detection. "
                "CLIP was not available at audit time — re-run with open-clip-torch installed.\n"
            )

    report = f"""---
title: "Corpus Demographic Profile — Jura Trace Training Corpus"
sprint: "Sprint 29 — Fairness Foundation"
pillar: "TRIED Pillar 4 (Fair)"
generated: "{generated_at}"
status: "audit output — do not modify manually"
---

# Corpus Demographic Profile

**Generated:** {generated_at}
**Sprint:** 29 — Fairness Foundation
**TRIED Pillar:** 4 (Fair)
**Purpose:** EMIF concept note evidence; quarterly retraining fairness gate
{sample_note}
---

## 1. Corpus Overview

| Class | Subdirectories | Images |
|---|---|---|
| Authentic | {len(authentic_images)} | {auth_total:,} |
| AI-generated | {len(ai_images)} | {ai_total:,} |
| **Total** | **{len(authentic_images) + len(ai_images)}** | **{total:,}** |

The production model at audit time is **UnivFD v9** (trained 2026-04-11, 39,016 samples,
AUC-ROC 0.9933, FP 4.12%, AI recall 95.70%). The audit covers the base corpus on the
external USB drive; the 32,142 platform-forwarded augmentation variants are not re-audited
separately (they are derived from the same source images).

---

## 2. Authentic Subdirectories

{_licence_table(results["authentic"], "authentic")}

### Geographic proxy detail

"""

    for subdir, info in sorted(results["authentic"].items()):
        report += f"**`{subdir}`**: {info['geo_proxy']}\n\n"

    report += f"""---

## 3. AI-Generated Subdirectories

{_licence_table(results["ai_generated"], "ai_generated")}

### Geographic/demographic proxy detail

"""

    for subdir, info in sorted(results["ai_generated"].items()):
        report += f"**`{subdir}`**: {info['geo_proxy']}\n\n"

    report += f"""---

## 4. Person-Content Fraction

{clip_note}

"""

    if clip_available:
        report += (
            f"| Class | Total Images | Person-Containing | Person Fraction |\n"
            f"|---|---|---|---|\n"
            f"| Authentic | {auth_total:,} | {auth_person_total:,} | "
            f"{auth_person_total/auth_total*100:.1f}% |\n"
            f"| AI-generated | {ai_total:,} | {ai_person_total:,} | "
            f"{ai_person_total/ai_total*100:.1f}% |\n\n"
        )
        report += (
            "Person-content detection used CLIP ViT-B/32 with a two-prompt softmax "
            f"('a photograph of a person' vs 'a photograph of scenery, objects, or animals "
            f"without people'), threshold {PERSON_THRESHOLD}. This is a coarse binary "
            "filter. It does NOT classify demographic characteristics of depicted individuals.\n"
        )
    else:
        report += "Person-content fractions: not computed (CLIP unavailable).\n"

    report += concentration_section

    report += """
---

## 6. What We Do Not Know

This audit is deliberately honest about the limits of what directory-structure analysis
and CLIP-based person detection can tell us:

1. **Skin tone**: Not measured, not estimated. Skin-tone classification from image pixels
   requires either human annotation or a dedicated skin-tone model — both are out of scope
   for this audit. Any claim of skin-tone distribution would be fabricated.

2. **Gender and age**: Not measured. We do not run facial recognition, gender classification,
   or age estimation. These are ethically fraught and technically unreliable without
   ground-truth annotations.

3. **Geographic ground truth**: The geographic proxies in sections 2 and 3 are qualitative
   inferences from published dataset papers and known platform demographics. They are not
   derived from image metadata, GPS, or any per-image measurement.

4. **CLIP person-detection accuracy**: CLIP ViT-B/32 was not designed or validated as a
   person detector. It may miss people in group shots, partial figures, or unusual contexts,
   and may false-positive on mannequins, statues, or representational art. The person-content
   fraction should be treated as an order-of-magnitude estimate.

5. **Augmented corpus**: The 32,142 platform-forwarded augmentation variants (Sprint 29 /
   backlog item 16) inherit the demographic distribution of their source images. They are
   not separately audited.

6. **AI generator training data**: The demographic biases of each AI generator's training
   data are largely undisclosed. Our AI-generated subdirectory audit reflects what we know
   from published model cards and academic papers, not from direct inspection.

---

## 7. Recommendations

Based on this audit, the following corpus expansion priorities are identified for the
next retraining cycle (Q3 2026):

1. **Global Majority authentic photographs**: The authentic corpus is heavily COCO/Flickr
   sourced, which carries known US/Western European bias. Priority: source 1,000+ authentic
   images from African, South/Southeast Asian, and Latin American photography collections
   (Wikimedia Commons by geographic category, LAION subsets with geo-tags, or Creative
   Commons archives from regional photojournalism organisations).

2. **Resolve `war_conflict` concentration risk**: This subdirectory likely skews toward
   Global Majority settings (conflict journalism) but may simultaneously be a demographic
   outlier in the authentic person-containing subset. Review contents and rebalance or
   relabel if it creates a systematic FP risk for images from those regions.

3. **`celeba` demographic narrowness**: CelebA is predominantly East Asian (Chinese
   entertainment industry). If used in demographic bias testing, it should be labelled
   as such — not treated as a representative face corpus.

4. **`google_photos`**: A single photographer's library introduces geographic and
   personal-network demographic bias. At 828 images it is the largest authentic subdirectory.
   Consider stratified sampling or explicit documentation in SOURCES.md.

5. **AI generator diversity**: `tpdne_faces` (StyleGAN / FFHQ-trained) has documented
   bias toward lighter-skinned faces. If included in bias testing it should be treated
   as a potential source of lower recall on darker-skinned authentic subjects (not an
   equivalence problem, but worth noting for per-generator FP analysis).

---

## 8. EMIF / TRIED Compliance Status

| Requirement | Status | Evidence |
|---|---|---|
| Corpus inventory documented | Done | This file |
| Licence per subdirectory | Done | Sections 2–3 |
| Geographic proxy assessment | Done (qualitative) | Sections 2–3 |
| Person-content fraction | Done if CLIP available | Section 4 |
| Concentration risk analysis | Done if CLIP available | Section 5 |
| Demographic ground truth (skin tone / gender / age) | Not done — requires human annotation | Section 6 explicitly states this |
| Per-demographic FP rate analysis | See bias-test-results.md | Sprint 29 Phase 2 |
| Quarterly retraining fairness gate | See test_demographic_bias.py | Sprint 29 Phase 3 |

**TRIED Pillar 4 citation readiness**: This document provides the corpus transparency
evidence. The EMIF concept note can now cite "documented corpus demographic profile with
explicit concentration risk analysis and honest limitations disclosure" as Pillar 4
evidence. It cannot claim skin-tone or gender parity without human annotation.
"""

    return report


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Audit Jura Trace training corpus for TRIED Pillar 4 (Fair) compliance.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--count",
        type=int,
        default=None,
        metavar="N",
        help="Process only N images (proportional sample across subdirs). Omit for full run.",
    )
    parser.add_argument(
        "--authentic-root",
        type=Path,
        default=AUTHENTIC_ROOT,
        help=f"Path to authentic/ corpus root (default: {AUTHENTIC_ROOT})",
    )
    parser.add_argument(
        "--ai-root",
        type=Path,
        default=AI_ROOT,
        help=f"Path to ai_generated/ corpus root (default: {AI_ROOT})",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=OUTPUT_MD,
        help=f"Output Markdown path (default: {OUTPUT_MD})",
    )
    parser.add_argument(
        "--no-clip",
        action="store_true",
        help="Skip CLIP person-detection entirely (counts and licence audit only).",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed for sample selection (default: 42).",
    )
    parser.add_argument(
        "--person-threshold",
        type=float,
        default=PERSON_THRESHOLD,
        help=f"CLIP softmax threshold for 'contains person' classification (default: {PERSON_THRESHOLD}).",
    )

    args = parser.parse_args()
    random.seed(args.seed)

    logger.info("Corpus Demographic Audit — Sprint 29 Fairness Foundation")
    logger.info("Authentic root: %s", args.authentic_root)
    logger.info("AI root: %s", args.ai_root)

    if not args.authentic_root.exists():
        logger.error(
            "Authentic corpus not found at %s — is the USB drive mounted?",
            args.authentic_root,
        )
        sys.exit(1)

    if not args.ai_root.exists():
        logger.error(
            "AI corpus not found at %s — is the USB drive mounted?",
            args.ai_root,
        )
        sys.exit(1)

    # Collect
    logger.info("Collecting image paths...")
    authentic_images = collect_images(args.authentic_root, sample_n=args.count)
    ai_images = collect_images(args.ai_root, sample_n=args.count)

    auth_total = sum(len(v) for v in authentic_images.values())
    ai_total = sum(len(v) for v in ai_images.values())
    logger.info(
        "Found %d authentic images across %d subdirs; %d AI images across %d subdirs.",
        auth_total, len(authentic_images), ai_total, len(ai_images),
    )

    # CLIP
    clip_state = None
    if not args.no_clip:
        clip_state = _load_clip()
        if clip_state is None:
            logger.warning(
                "Proceeding without CLIP. Person-content fractions will be 'n/a'."
            )
    else:
        logger.info("--no-clip specified: skipping person-detection.")

    # Analyse
    logger.info("Analysing corpus...")
    results = analyse(
        authentic_images,
        ai_images,
        clip_state,
        person_threshold=args.person_threshold,
    )

    concentration_flags = compute_concentration_risk(results)
    if concentration_flags:
        logger.warning(
            "%d concentration risk flag(s) detected: %s",
            len(concentration_flags),
            [f"{f['class']}/{f['subdir']}" for f in concentration_flags],
        )

    # Write report
    generated_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
    report_md = write_report(
        results=results,
        concentration_flags=concentration_flags,
        authentic_images=authentic_images,
        ai_images=ai_images,
        clip_available=(clip_state is not None),
        sample_n=args.count,
        generated_at=generated_at,
    )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(report_md, encoding="utf-8")
    logger.info("Report written to %s", args.output)

    # Summary to stdout
    print(f"\n--- Corpus Demographic Audit Summary ---")
    print(f"Authentic: {auth_total:,} images across {len(authentic_images)} subdirectories")
    print(f"AI-generated: {ai_total:,} images across {len(ai_images)} subdirectories")
    if clip_state is not None:
        auth_persons = sum(
            v["person_count"] or 0
            for v in results["authentic"].values()
        )
        ai_persons = sum(
            v["person_count"] or 0
            for v in results["ai_generated"].values()
        )
        print(f"Person content (authentic): {auth_persons:,} / {auth_total:,} ({auth_persons/auth_total*100:.1f}%)")
        print(f"Person content (AI): {ai_persons:,} / {ai_total:,} ({ai_persons/ai_total*100:.1f}%)")
    else:
        print("Person content: not computed (CLIP unavailable or --no-clip set)")
    if concentration_flags:
        print(f"Concentration risk flags: {len(concentration_flags)}")
        for f in concentration_flags:
            print(f"  - {f['class']}/{f['subdir']}: {f['fraction_of_class_persons']*100:.1f}% of class persons")
    else:
        print("Concentration risk: none detected" if clip_state else "Concentration risk: not computed")
    print(f"Report: {args.output}")


if __name__ == "__main__":
    main()
