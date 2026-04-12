---
title: "Corpus Demographic Profile — Jura Trace Training Corpus"
sprint: "Sprint 29 — Fairness Foundation"
pillar: "TRIED Pillar 4 (Fair)"
generated: "2026-04-12T09:51:37+00:00"
status: "audit output — do not modify manually"
---

# Corpus Demographic Profile

**Generated:** 2026-04-12T09:51:37+00:00
**Sprint:** 29 — Fairness Foundation
**TRIED Pillar:** 4 (Fair)
**Purpose:** EMIF concept note evidence; quarterly retraining fairness gate

---

## 1. Corpus Overview

| Class | Subdirectories | Images |
|---|---|---|
| Authentic | 13 | 6,134 |
| AI-generated | 15 | 4,985 |
| **Total** | **28** | **11,119** |

The production model at audit time is **UnivFD v9** (trained 2026-04-11, 39,016 samples,
AUC-ROC 0.9933, FP 4.12%, AI recall 95.70%). The audit covers the base corpus on the
external USB drive; the 32,142 platform-forwarded augmentation variants are not re-audited
separately (they are derived from the same source images).

---

## 2. Authentic Subdirectories

| Subdirectory | Images | Licence | Geographic Proxy | Person Content |
|---|---|---|---|---|
| `camera_dcim` | 59 | Proprietary / owned by Jura Labs (original camera captures) | Single photographer (Jura Labs founder) — UK locations, European and some Global Majority subjects | 24% |
| `celeba` | 200 | Research/non-commercial (CelebA licence — restricted, face images only) | Chinese entertainment industry face images (predominantly East Asian subjects); geographically and demographically narrow | 100% |
| `coco` | 500 | CC-BY 4.0 (COCO val2017) | Predominantly US/Western European settings (MS COCO sourced from Flickr, US-centric object detection tasks) | 62% |
| `coco_extra` | 300 | CC-BY 4.0 (COCO additional sample) | Same sourcing as coco — US/Western European bias expected | 64% |
| `coco_extra2` | 800 | CC-BY 4.0 (COCO additional sample) | Same sourcing as coco — US/Western European bias expected | 60% |
| `coco_train` | 800 | CC-BY 4.0 (COCO train2017) | Same sourcing as coco — US/Western European bias expected | 58% |
| `flickr30k` | 1,300 | Option C corpus licence (non-commercial cleared — see docs/decisions/option-c-corpus-strategy.md) | Primarily US and Western European (Flickr 2014 snapshot; US English-speaking communities dominant on Flickr at that time) | 89% |
| `flickr8k` | 500 | Option C corpus licence (non-commercial cleared — see docs/decisions/option-c-corpus-strategy.md) | Primarily US and Western European — same Flickr sourcing as Flickr30k | 85% |
| `google_photos` | 414 | Proprietary / owned by contributor (single photographer's library) | Single photographer's personal library — specific geographic scope unknown | 62% |
| `imagenet_vl` | 600 | ImageNet licence (non-commercial research — see corpus strategy note) | Primarily object/scene images (non-person content dominant); geographic diversity of person-containing subset unknown | 70% |
| `war_conflict` | 203 | Mixed — press/documentary; assumed editorial use | Likely skews toward conflict zones in Global Majority regions (Middle East, sub-Saharan Africa, South/Southeast Asia) based on global press coverage patterns | 70% |
| `wikimedia_photos` | 254 | CC-BY / CC-BY-SA (per individual file — Wikimedia featured images) | Geographically diverse (Wikimedia editors worldwide) but art/heritage content may over-represent European cultural artefacts | 61% |
| `wildlife_macro` | 204 | Mixed — assumed CC0 / public domain or contributor-owned | Non-person content dominant (wildlife/macro) | 25% |

### Geographic proxy detail

**`camera_dcim`**: Single photographer (Jura Labs founder) — UK locations, European and some Global Majority subjects. Very small sample (59 images). Unknown.

**`celeba`**: Chinese entertainment industry face images (predominantly East Asian subjects); geographically and demographically narrow.

**`coco`**: Predominantly US/Western European settings (MS COCO sourced from Flickr, US-centric object detection tasks). Global minority communities likely under-represented.

**`coco_extra`**: Same sourcing as coco — US/Western European bias expected.

**`coco_extra2`**: Same sourcing as coco — US/Western European bias expected.

**`coco_train`**: Same sourcing as coco — US/Western European bias expected.

**`flickr30k`**: Primarily US and Western European (Flickr 2014 snapshot; US English-speaking communities dominant on Flickr at that time).

**`flickr8k`**: Primarily US and Western European — same Flickr sourcing as Flickr30k.

**`google_photos`**: Single photographer's personal library — specific geographic scope unknown. Cannot assess.

**`imagenet_vl`**: Primarily object/scene images (non-person content dominant); geographic diversity of person-containing subset unknown.

**`war_conflict`**: Likely skews toward conflict zones in Global Majority regions (Middle East, sub-Saharan Africa, South/Southeast Asia) based on global press coverage patterns. This may create unexpected demographic concentration in the person-containing authentic subset.

**`wikimedia_photos`**: Geographically diverse (Wikimedia editors worldwide) but art/heritage content may over-represent European cultural artefacts.

**`wildlife_macro`**: Non-person content dominant (wildlife/macro). Geographic origin unknown.

---

## 3. AI-Generated Subdirectories

| Subdirectory | Images | Licence | Geographic Proxy | Person Content |
|---|---|---|---|---|
| `__root__` | 389 | Unknown | Unknown — not yet assessed | 67% |
| `artbench` | 200 | ArtBench dataset licence (research permitted) | ArtBench dataset — art-style generations | 32% |
| `civitai_sfw` | 500 | Civitai platform terms (SFW community outputs — model-dependent) | Global community uploads but English-language-dominant platform; aesthetic bias toward Western anime/fantasy styles | 79% |
| `dalle3` | 500 | OpenAI Terms of Service (outputs owned by requester) | DALL-E 3 outputs are prompt-driven; without prompt logs geographic/demographic distribution is unknown | 65% |
| `diffusiondb` | 500 | CC0 1.0 Universal (DiffusionDB dataset) | Stable Diffusion 1 | 75% |
| `elsa` | 1,300 | ELSA 1M dataset licence (research permitted) | ELSA 1M is a European project dataset (EU Horizon); outputs drawn from SD/DALL-E mix | 71% |
| `flux_dev` | 268 | FLUX.1-dev Community Licence (non-commercial permitted) | Flux | 52% |
| `gemini` | 50 | Google Terms of Service (outputs owned by requester) | Google Gemini outputs; Google has disclosed diversity interventions in image generation | 42% |
| `grok_aurora` | 500 | xAI Terms of Service (outputs owned by requester) | xAI Grok Aurora outputs; same disclosure limitations as grok | 57% |
| `midjourney_v6` | 150 | Midjourney Community Showcase / ToS (pro plan outputs — commercial permitted) | Midjourney v6 community showcase outputs; aesthetic bias toward stylised Western art styles | 69% |
| `sd15` | 3 | CreativeML Open RAIL-M | Stable Diffusion 1 | 100% |
| `sdxl_turbo` | 300 | SDXL-Turbo community licence | SDXL-Turbo outputs; same Stable Diffusion lineage limitations as diffusiondb | 50% |
| `synthetic_faces` | 300 | Mixed — see individual manifests | Mixed provenance — unknown | 100% |
| `user_ai` | 19 | Contributor-submitted — assumed researcher-owned outputs | Contributor-submitted — unknown demographics and geographic distribution | 68% |
| `wikimedia_ai` | 6 | CC-BY / CC-BY-SA per file (Wikimedia AI-labelled uploads) | AI-labelled Wikimedia uploads — diverse subject matter but very small sample | 67% |

### Geographic/demographic proxy detail

**`__root__`**: Unknown — not yet assessed

**`artbench`**: ArtBench dataset — art-style generations. Non-photorealistic content; demographic signals attenuated.

**`civitai_sfw`**: Global community uploads but English-language-dominant platform; aesthetic bias toward Western anime/fantasy styles. Person content likely skews toward lighter-skinned, East Asian, and fantasy-coded appearances.

**`dalle3`**: DALL-E 3 outputs are prompt-driven; without prompt logs geographic/demographic distribution is unknown. OpenAI has published fairness studies on output diversity for default prompts.

**`diffusiondb`**: Stable Diffusion 1.x outputs; DiffusionDB prompts are community-submitted, English-dominant. Demographic and geographic distribution unknown.

**`elsa`**: ELSA 1M is a European project dataset (EU Horizon); outputs drawn from SD/DALL-E mix. European editorial context.

**`flux_dev`**: Flux.1-dev outputs; training data and demographic biases not fully disclosed by Black Forest Labs.

**`gemini`**: Google Gemini outputs; Google has disclosed diversity interventions in image generation.

**`grok_aurora`**: xAI Grok Aurora outputs; same disclosure limitations as grok.

**`midjourney_v6`**: Midjourney v6 community showcase outputs; aesthetic bias toward stylised Western art styles.

**`sd15`**: Stable Diffusion 1.5 — same lineage as sd_v14.

**`sdxl_turbo`**: SDXL-Turbo outputs; same Stable Diffusion lineage limitations as diffusiondb.

**`synthetic_faces`**: Mixed provenance — unknown.

**`user_ai`**: Contributor-submitted — unknown demographics and geographic distribution.

**`wikimedia_ai`**: AI-labelled Wikimedia uploads — diverse subject matter but very small sample.

---

## 4. Person-Content Fraction

CLIP ViT-B/32 person-detection was used for person-content classification.

| Class | Total Images | Person-Containing | Person Fraction |
|---|---|---|---|
| Authentic | 6,134 | 4,266 | 69.5% |
| AI-generated | 4,985 | 3,363 | 67.5% |

Person-content detection used CLIP ViT-B/32 with a two-prompt softmax ('a photograph of a person' vs 'a photograph of scenery, objects, or animals without people'), threshold 0.5. This is a coarse binary filter. It does NOT classify demographic characteristics of depicted individuals.

## 5. Concentration Risk Flags

No single subdirectory accounts for more than 80% of person-containing images within its class. No concentration risk detected at this threshold.

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
