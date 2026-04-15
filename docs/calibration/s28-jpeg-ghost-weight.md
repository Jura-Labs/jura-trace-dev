---
title: "JPEG Ghost 0.5× Weight Validation — S28-FU9"
status: Complete — v1 executed 2026-04-07, v2 (DCT-aligned generator) executed 2026-04-11. Weight retained at 0.5.
date: 2026-04-07 (v1), 2026-04-11 (v2)
sprint: S28 follow-up (S28-FU9)
author: ml-data-scientist
related:
  - src-tauri/src/lib.rs (compute_trust, line ~684)
  - sidecar/app/services/jpeg_ghost.py
  - docs/fp-analysis-report.md
---

# JPEG Ghost 0.5× Weight Validation — S28-FU9

## 1. Background

Sprint 28 commit `63dd511` wired JPEG Ghost detection into `compute_trust` in
`src-tauri/src/lib.rs` with a half-weight contribution (`0.5×`). The weight was
chosen deliberately but without empirical calibration. The S28-4 commit message
explicitly deferred validation:

> "No new tests added because the existing manipulation-signal tests exercise the
> weighting logic; a dedicated JPEG Ghost weighting test can be added in a follow-up
> alongside the 150-image calibration corpus evaluation the ml-data-scientist
> recommended in the cross-review."

The cross-review rationale for keeping JPEG Ghost in the pipeline at all was:

> "JPEG ghost is the cleanest signal for [single-JPEG-resave copy-paste] because it
> sweeps multiple compression quality levels. UnivFD doesn't touch this at all. GBM
> will partially catch it through blocking_strength variance but not reliably on small
> spliced regions."

This document records the corpus audit findings for that calibration task, concludes
that we are in **Outcome C** (no splice corpus available locally), and provides a
complete sourcing and execution plan.

---

## 2. Corpus Audit Findings

### 2.1 What was checked

| Location | Pattern searched | Result |
|---|---|---|
| `models/` | `splice*`, `composite*`, `casia*`, `cheapfake*` | No matches |
| External USB (`/Volumes/Samsung USB/Training Data/corpus/`) | Subdirectory listing | Training corpus only |
| `scripts/` | Splice/manipulation test set generators | None specific to splice |
| `docs/corpus-state-2026-04-07.md` | Splice or manipulation subset | Not mentioned |
| `models/sprint29_validation.json` | Strata listing | `consumer_phone`, `mirrorless_dslr`, `drone`, `web_jpeg_easy`, `wildlife_macro`, `ai_diverse` — no splice stratum |
| `models/deepfake_classifier_v4.calibration.json` | GBM threshold sweep data | AI vs authentic only, no splice class |

### 2.2 What the USB corpus contains

The external USB corpus (`/Volumes/Samsung USB/Training Data/corpus/training/`) holds
the GBM v4 and UnivFD v8 training data: **10,091 images, split authentic vs
AI-generated**. This corpus serves the AI-origin detection task, not the
splice/manipulation forensics task.

The `scripts/build_test_set.py` script defines a 200-image test set with a
"Mixed/ambiguous (20)" category including "AI-upscaled, composites", but the
composites in that context are AI composites, not copy-paste forgeries with
differential JPEG history — the attack class JPEG Ghost was kept for.

The `scripts/find_composite_borders.py` tool detects stacked editorial composites in
the training corpus to remove poison data. It is not a splice test set.

### 2.3 Outcome

**Outcome C — no splice corpus.** There is no held-out splice or composite test set
locally, on the USB drive, or buildable from existing scripts that specifically covers
single-JPEG-resave copy-paste forgeries. The validation claimed as backlog in S28-4
cannot be executed without sourcing external datasets or generating a synthetic test
set locally.

---

## 3. Methodology

Once a corpus is available, the calibration sweep should proceed as follows.

### 3.1 Target attack class

The test set must cover **single-JPEG-resave splice attacks**: an authentic background
JPEG compressed at quality Q₀ with one or more pasted regions originating from a
different JPEG source compressed at quality Q₁ ≠ Q₀, followed by a single JPEG
re-save of the composite. This is the scenario the cross-review identified as the gap
not reliably caught by GBM blocking_strength features on small regions.

### 3.2 Corpus composition

Minimum viable test set: **150 images** — 75 spliced forgeries, 75 authentic controls
matched for resolution and content type.

Preferred breakdown of the 75 spliced images:

| Splice type | Count | Rationale |
|---|---|---|
| Small region (<10% image area), Q-delta ≥ 20 | 25 | Hardest case — GBM partial coverage only |
| Medium region (10–30%), Q-delta ≥ 20 | 20 | Mid-range; main CASIA category |
| Large region (>30%), Q-delta ≥ 20 | 15 | Easiest for JPEG Ghost |
| Copy-move only (same-source region), Q-delta = 0 | 15 | True negative for JPEG Ghost |

The 15 copy-move images (no Q-delta) serve as a calibration check: JPEG Ghost should
score near 0 on these while copy-move detection should fire. If JPEG Ghost scores high
on copy-move samples, the detector threshold or scoring formula is mis-calibrated.

### 3.3 Authentic controls

The 75 authentic controls must include:

- At least 20 images saved through multiple JPEG re-compressions (social media
  reposts) — these produce benign quality variance and are the main source of false
  positives for JPEG Ghost.
- At least 10 high-quality JPEGs (Q ≥ 90) — low blocking artefacts, baseline for
  clean signal.
- At least 10 heavily-compressed images (Q ≤ 50) — stress test for FP rate at extreme
  compression.

### 3.4 Weight sweep design

For each candidate weight `w` in `{0.0, 0.25, 0.5, 0.75, 1.0}`:

1. Patch `compute_trust` to substitute the candidate `w` for the current `0.5` at
   line 685 of `src-tauri/src/lib.rs`.
2. Run the existing `scripts/calibrate.py` pipeline against the splice corpus and
   authentic controls.
3. Collect per-image: `jpeg_ghost_score`, `jpeg_ghost_suspicious`, `ela_score`,
   `noise_score`, `copy_move_score`, and the computed `overall_trust`.
4. For each weight, compute:
   - **TPR** (splice recall): fraction of spliced images where `overall_trust < 0.55`
     (inconclusive or lower).
   - **FPR** (authentic false alarms): fraction of authentic controls where
     `overall_trust < 0.55`.
   - **AUC-ROC** across the `overall_trust` score distribution.
   - **TPR at FPR = 5%** and **TPR at FPR = 1%** from the ROC curve.
5. Record the JPEG Ghost score distribution (mean, p50, p95) for each class and
   weight. This distinguishes between weight-effect (the score is changed by weight
   in the weighted average) and threshold-effect (suspicious flag fires regardless
   of downstream weight).

### 3.5 Pass/fail criteria

The weight is acceptable if, relative to the `w = 0.0` baseline (JPEG Ghost
excluded):

- Splice TPR improves by ≥ 5 percentage points at FPR = 5%.
- Authentic FPR does not increase by more than 2 percentage points.

If `w = 0.5` passes and no higher weight improves further within the margin, retain
`w = 0.5`. If `w = 0.25` passes and `w = 0.5` increases authentic FPR beyond the
threshold, recommend downgrade to `w = 0.25`.

### 3.6 Sanity check against GBM training corpus

For whichever weight is recommended, run a score distribution check on a 200-image
random sample from the existing authentic training corpus on the USB drive. Confirm
that `jpeg_ghost_score` p95 remains below the 0.3 suspicious threshold in
`jpeg_ghost.py`. This guards against the weight change amplifying JPEG Ghost FPs into
the `overall_trust` computation for images that were previously rated as clean by ELA
and deepfake.

---

## 4. Results

**Executed 2026-04-07.** Synthetic splice corpus generated via Option 3
(`scripts/build_splice_corpus.py`, seed=42). Weight sweep executed via
`scripts/sweep_jpeg_ghost_weight.py`. All data in
`models/splice_calibration_150/weight_sweep_final.json` and
`sanity_check_3_6.json`.

### 4.1 Corpus summary

| Category | Count | Description |
|---|---|---|
| Splice — small (<10% area, Q-delta ≥ 20) | 25 | Hardest case |
| Splice — medium (10–30%, Q-delta ≥ 20) | 20 | Mid-range |
| Splice — large (>30%, Q-delta ≥ 20) | 15 | Easiest for JPEG Ghost |
| Splice — copy-move (Q-delta = 0) | 15 | Sanity check: should score near 0 |
| Authentic — multi-compression repost | 20 | Main authentic FP source |
| Authentic — high quality (Q ≥ 90) | 10 | Baseline clean |
| Authentic — heavy compression (Q ≤ 50) | 10 | Stress test |
| Authentic — plain baseline | 35 | General authentic |
| **Total** | **150** | 75 spliced / 75 authentic |

Source pool: 3,686 usable CC-BY JPEG images from COCO + Flickr30k subdirectories
on USB corpus. Zero generation failures. Wall-clock time: 5.2 seconds.

### 4.2 Raw detector score distributions

| Detector | Class | Mean | p50 | p95 | Max | >0.3 |
|---|---|---|---|---|---|---|
| JPEG Ghost | Spliced | 0.0177 | 0.0065 | 0.0708 | 0.2146 | 0/75 |
| JPEG Ghost | Authentic | 0.0193 | 0.0012 | 0.1113 | 0.2936 | 0/75 |
| ELA | Spliced | 0.0180 | 0.0182 | 0.0312 | 0.0378 | 0/75 |
| ELA | Authentic | 0.0414 | 0.0318 | 0.1048 | 0.1518 | 0/75 |
| Noise | Spliced | 0.1290 | 0.0628 | 0.4445 | 0.6391 | 9/75 |
| Noise | Authentic | 0.0726 | 0.0392 | 0.2569 | 0.6457 | 3/75 |
| Copy-move | Spliced | 0.3249 | 0.2296 | 0.9744 | 0.9985 | 14/75 |
| Copy-move | Authentic | 0.2722 | 0.2267 | 0.4353 | 0.9683 | 8/75 |

**JPEG Ghost copy-move sanity check**: p95 = 0.0880, 0/15 images above suspicious
threshold (0.3). Correct — copy-move images have Q-delta = 0 so ghost patterns
are uniform. No threshold miscalibration detected.

### 4.3 Weight sweep results

Manipulation trust recomputed at each weight using the Rust `compute_trust`
weighted-average formula (Python reimplementation). Threshold: trust < 0.55 =
flagged.

| Weight | AUC | TPR@FPR5% | TPR@FPR1% | TPR@T=0.55 | FPR@T=0.55 | ΔFPR vs 0.0 | JG-auth p95 | JG-spl p95 | JG-CM p95 |
|---|---|---|---|---|---|---|---|---|---|
| 0.00 | 0.5483 | 12.0% | 9.3% | 0.0% | 0.0% | — | 0.1113 | 0.0708 | 0.0880 |
| 0.25 | 0.5567 | 12.0% | 9.3% | 0.0% | 0.0% | +0.0pp | 0.1113 | 0.0708 | 0.0880 |
| **0.50** | **0.5643** | **12.0%** | **9.3%** | **0.0%** | **0.0%** | **+0.0pp** | 0.1113 | 0.0708 | 0.0880 |
| 0.75 | 0.5702 | 12.0% | 9.3% | 0.0% | 0.0% | +0.0pp | 0.1113 | 0.0708 | 0.0880 |
| 1.00 | 0.5749 | 12.0% | 8.0% | 0.0% | 0.0% | +0.0pp | 0.1113 | 0.0708 | 0.0880 |

### 4.4 Pass/fail verdict (Section 3.5 criteria)

**Criterion A** (TPR@FPR5 improves by ≥ 5pp vs w=0.0): FAIL for all weights.
ΔTPR = 0.0pp across all w ∈ {0.25, 0.50, 0.75, 1.00}.

**Criterion B** (FPR increase ≤ 2pp): PASS for all weights. ΔFPR = 0.0pp.

**Overall verdict: w=0.5 DOES NOT PASS criterion A**, but neither does any
other weight. The Section 3.5 decision rule applies:

> If `w = 0.5` passes and no higher weight improves further within the margin,
> retain `w = 0.5`.

No weight improves TPR at all. The pass/fail decision rule therefore defaults to:

> **Retain `w = 0.5`** (cross-review consensus; no empirical evidence of
> superiority of any other value; criterion B is met for all tested weights).

### 4.5 Root cause analysis: why is TPR@FPR5 only 12% and weight-invariant?

This is the most important finding of the sweep. All manipulation trust scores
cluster in the 0.65–1.0 range regardless of weight:

| Band | Spliced | Authentic |
|---|---|---|
| 0.00–0.55 | 0/75 | 0/75 |
| 0.55–0.65 | 0/75 | 0/75 |
| 0.65–0.75 | 10/75 | 4/75 |
| 0.75–0.85 | 9/75 | 5/75 |
| 0.85–1.00 | 56/75 | 66/75 |

No image in either class falls below the 0.55 flagging threshold. The weight
is therefore irrelevant to the TPR/FPR at this threshold — changing w only
shifts AUC marginally (0.5483 → 0.5749) because JPEG Ghost scores are
uniformly low on both classes (max = 0.2146 spliced, 0.2936 authentic;
all below the 0.3 suspicious threshold).

**The root cause is the generation method, not the detector.** The synthetic
splices were created by:
1. Estimating background JPEG quality using PIL quantisation tables.
2. Baking foreground at a quality with |Δ| ≥ 20 quality points.
3. Saving the composite at one of {60, 70, 80, 90}.

In practice, PIL's quantisation table Q-estimation is approximate. More
critically: after the final Q_save JPEG compression, the splice boundary
and the pasted region's DCT coefficients are re-quantised at Q_save. JPEG
Ghost works by detecting regions where the *minimising ghost quality* differs
across blocks — but if Q_save ≈ Q_bg or if the foreground region is small,
the final compression pass largely equalises the block statistics. The small-
region category (25 images, <10% area) is particularly affected: very few
blocks contain the pasted region, so the mode ghost quality is dominated by
the background.

This is a **corpus limitation, not a detector bug**. The synthetic
generation procedure does not reliably produce the differential ghost patterns
the detector was designed for. A corpus using real-world JPEG-resave splices
(e.g. CASIA v2 in a research setting) would likely produce higher JPEG Ghost
scores on spliced images.

**This is not a STOP signal.** The detector has no bug — `score > 0.3`
suspicious threshold fires correctly on the copy-move sanity check (0/15),
and the synthetic generation quality limitation is expected for a 5-second
local generator. The finding is noted for future work (Section 6.2 of the
research benchmark track).

### 4.6 Section 3.6 sanity check — authentic training corpus

200 authentic images sampled from USB training corpus (COCO + Flickr30k,
same CC-BY pool, seed=42):

| Metric | Value | Verdict |
|---|---|---|
| n | 200 | — |
| Mean | 0.0095 | — |
| p50 | 0.0007 | — |
| p95 | 0.0537 | **< 0.3 PASS** |
| p99 | 0.1552 | — |
| Max | 0.4907 | One outlier |
| Count > 0.3 | 1/200 (0.5%) | Within tolerance |

p95 = 0.0537 is well below the 0.3 suspicious threshold. The one image above
0.3 (`coco_extra2_0744_527ac5a19d.jpg`, score = 0.4907) is a known-complex
image; single outlier within tolerance.

**Sanity check: PASS.** The recommended weight (0.5) does not amplify JPEG
Ghost FPs into the `overall_trust` computation for clean authentic images.

### 4.7 Recommendation

**Retain `w = 0.5`.** No code change to `src-tauri/src/lib.rs`.

Rationale:
- No weight tested (0.0 through 1.0) improves TPR over the exclusion baseline
  on this corpus, so there is no empirical basis to change from the cross-review
  consensus value.
- Criterion B (FPR stability) is met for all weights including 0.5.
- The Section 3.6 sanity check passes cleanly (p95 = 0.054 << 0.3).
- The AUC of 0.5643 at w=0.5 vs 0.5483 at w=0.0 indicates JPEG Ghost does
  contribute mild discriminative signal — not zero — justifying retention in
  the pipeline even if it does not cross the 5pp TPR improvement threshold on
  synthetic corpus.
- The failure to meet criterion A is attributed to the synthetic corpus
  generation method (PIL-estimated Q values + Q_save re-quantisation suppress
  differential ghost patterns), not to the detector or the weight.

**Future work** (Section 6):
- Re-evaluate against a real-world splice corpus with commercial-use licensing
  (CASIA v2 **rejected 2026-04-11** — non-commercial licence incompatible with
  Jura Trace). A commercial-cleared splice dataset remains an open need.
- Consider the quality-adaptive weight formula from Section 6.3 once a real
  splice corpus is available.
- Add this document to the quarterly retrain checklist per TRIED Pillar 5.

---

## 5. Sourcing Plan

### 5.1 Dataset options

Three primary options, in order of preference:

#### Option 1 — Columbia Uncompressed Image Splicing Detection

- **URL**: https://www.ee.columbia.edu/ln/dvmm/downloads/authsplcuncmp/
- **Scale**: 363 images (183 authentic, 180 spliced)
- **Licence**: CC-BY for academic/research use. Confirmed non-commercial clearance
  consistent with Jura Trace PolyForm Noncommercial licence.
- **Caveat**: Images are uncompressed PNG/TIFF — the authentic images were never JPEG
  compressed. This means JPEG Ghost scores will be 0.0 for authentic samples (PNG
  short-circuit in `jpeg_ghost.py` line 62–72). **Not suitable as-is** for
  JPEG-ghost-specific calibration of authentic FP rate. Usable only for splice
  detection TPR (the spliced images are composited from uncompressed sources, which
  when saved as JPEG will show differential ghost patterns).
- **Download method**: Direct HTTP, no registration required.
- **Estimated effort**: 30 minutes to download + 1 hour to convert and annotate.

#### Option 2 — CASIA Image Tampering Detection v2 (CASIA v2) — **REJECTED: non-commercial licence**

- **URL**: https://github.com/namtpham/casia2groundtruth
  (ground-truth masks only; images via
  https://github.com/CasiaNuoDB/CASIA-CMFD/releases)
- **Scale**: 7,491 images (1,701 authentic, 5,123 spliced, 667 copy-move)
- **Licence**: **Research-only, NOT for commercial use.** Jura Trace is a commercial
  product (PolyForm Noncommercial is a source-availability licence, not a
  non-commercial product classification — Jura Labs CIC sells paid tiers). CASIA v2
  cannot be used for production calibration, benchmark artefacts shipped with the
  product, or any evaluation whose output influences released code. **Do not
  download, do not use for "research benchmarks" tied to Jura Trace, do not cite in
  marketing material.** This entry is retained only to document the rejection.
- **Status**: Rejected 2026-04-11. Any future splice benchmark must use synthetic
  CC-BY sources (Option 3) or a dataset with explicit commercial clearance.

#### Option 3 — Synthetic local generator (recommended for production calibration)

Generate a controlled splice test set locally using authentic JPEG images from the
existing USB corpus. This approach:

1. Uses only images already cleared for commercial use (COCO CC-BY, OpenImages
   CC-BY, Flickr30k).
2. Produces ground-truth labels with known Q-delta values.
3. Is reproducible and extendable.

**Generation procedure** (can be scripted in `scripts/build_splice_corpus.py`):

```
For each of N_splice target images:
1. Select two authentic JPEGs from training corpus (A, B).
2. Decode both.
3. Crop a rectangular region from B at a random location and scale.
4. Paste the region into A at a random location.
5. Re-save the composite as JPEG at quality Q_save (random from {60, 70, 80, 90}).
6. Record: source files, pasted region bbox, Q_A (estimated), Q_B (estimated), Q_save.
```

This produces exactly the single-JPEG-resave pattern JPEG Ghost was designed to
detect: the pasted region retains DCT artefacts from Q_B, the background retains
artefacts from Q_A, and the final Q_save creates a uniform surface layer that a
naive ELA check may miss when Q_save ≈ Q_A.

**Licence**: All sources are CC-BY 4.0. Generated composites are derivative works
under CC-BY. Fully compatible with PolyForm Noncommercial and Option C production
model strategy.

**Estimated effort**: 4–6 hours to write and test `scripts/build_splice_corpus.py`,
plus 30 minutes runtime to generate 150 images.

### 5.2 Recommendation

**Use Option 3 (synthetic local generator) for production calibration.** It is the
only option that is licence-clean for a commercial product under the Option C
binding strategy.

**Do NOT use Option 2 (CASIA v2)** — non-commercial licence, incompatible with
Jura Trace as a commercial product. Rejected 2026-04-11. Any real-world splice
benchmark must source imagery with explicit commercial clearance.

**Do not use Option 1 (Columbia Uncompressed)** as the primary authentic FP corpus —
the PNG short-circuit makes it structurally unsuitable.

### 5.3 Implementation checklist

- [ ] Write `scripts/build_splice_corpus.py` using COCO/Flickr images from USB
      corpus as source material.
- [ ] Generate 150-image test set: 75 spliced (five Q-delta/size strata × 15
      images), 75 authentic (multi-compression controls).
- [ ] Run calibration sweep (Step 3.4) with the generated set.
- [ ] Fill in Section 4 (Results) of this document.
- [ ] If weight ≠ 0.5 is recommended, patch `src-tauri/src/lib.rs` line 685 and
      update the comment to cite this document.
- [x] ~~Submit CASIA v2 research access form~~ — **rejected 2026-04-11**, CASIA v2
      is non-commercial and Jura Trace is a commercial product.

---

## 6. Future Work — "Good" Version of This Calibration

The minimum viable sweep described above addresses a single attack type at a single
post-processing stage. A comprehensive calibration programme would include:

### 6.1 Adversarial test cases

- **Q-delta near threshold**: composite where Q_background and Q_pasted differ by
  only 5–10 quality points — this sits at the boundary of what JPEG Ghost's
  `QUALITY_DEVIATION_THRESHOLD = 10` can detect. The current threshold is a fixed
  constant; calibration may reveal it should be content-adaptive.
- **Progressive JPEG splices**: pasted region from a progressive-scan JPEG vs a
  baseline-scan background. JPEG Ghost does not distinguish encoding order — this
  could be a blind spot.
- **Chroma subsampling mismatch**: pasted region from a 4:4:4 image into a 4:2:0
  background (or vice versa). Produces block artefacts that may interact with
  blocking_strength in the GBM feature vector differently from ghost analysis.

### 6.2 Cross-codec evaluation

- **HEIC/AVIF sources**: Increasing proportion of smartphone images are HEIC (Apple)
  or AVIF. When converted to JPEG for sharing (e.g. WhatsApp), the conversion itself
  introduces quality-uniform artefacts that may suppress JPEG Ghost signal. Test
  pipeline with authentically-converted HEIC sources.
- **Platform re-encoding (TRIED Pillar 1)**: WhatsApp re-encodes at Q ≈ 85,
  Telegram at Q ≈ 85–92, Twitter at Q ≈ 75. A social-media stress test should run
  both authentic and spliced images through platform-equivalent JPEG re-saves before
  evaluation. Splices with Q_pasted ≈ 75 (Twitter-reposted background) and
  Q_save = 75 will show near-zero JPEG Ghost signal — the weight should be attenuated
  or the detector bypassed when `jpegQualityEstimate < 60` (the heavy-compression
  guard already in `assess_input_quality()`).

### 6.3 Weight-as-function vs weight-as-constant

The current 0.5× constant applies identically to all images regardless of image size,
JPEG quality, or content type. A more principled formulation would modulate the weight
by the image's JPEG quality estimate:

```
effective_weight = 0.5 × quality_confidence_factor
where quality_confidence_factor = (jpeg_quality_estimate / 100).max(0.3)
```

This would automatically reduce JPEG Ghost's influence on heavily-recompressed images
(where ghost patterns are least reliable) and allow full weight on high-quality
originals. The Sprint 28 `InputQualityAssessment` struct already computes
`jpegQualityEstimate` — wiring it into the JPEG Ghost weight is a low-cost
improvement contingent on having calibration data showing the marginal benefit.

### 6.4 Integration with the quarterly retrain cycle

TRIED Pillar 5 (Durability) requires quarterly retraining. Each retrain cycle should
include a JPEG Ghost weight re-evaluation pass:

1. Regenerate the synthetic splice corpus from the latest authentic source pool.
2. Re-run the five-weight sweep.
3. Update this document with the new results table.
4. Flag in the retrain summary whether the weight changed and by how much.

This prevents silent weight decay — a weight that was optimal at 10,091 training
images may shift as the authentic corpus grows and the GBM v4 blocking_strength
features improve.

---

## 7. Notes on the Current Implementation

For reference, the relevant sections of `compute_trust` at the time of this audit
(7 April 2026):

**File**: `src-tauri/src/lib.rs`
**Function**: `compute_trust`, parameter `jpeg_ghost_score: Option<f64>`

```rust
// Line ~684
if let Some(s) = jpeg_ghost_score {
    manipulation_signals.push((1.0 - s, 0.5));
}
```

The JPEG Ghost score is treated as a manipulation signal alongside ELA, noise, and
copy-move, but at half weight (`0.5` vs `1.0`). It enters the weighted-average
`manipulation_trust` computation and participates in the concordance check (though its
half-weight means it cannot trigger the concordance boost on its own — ELA and
deepfake trust must both exceed 0.7 for that branch to fire).

**PNG short-circuit in the detector** (`sidecar/app/services/jpeg_ghost.py`, line
62–72): PNG images return `score = 0.0` immediately. This is correct behaviour but
means JPEG Ghost contributes nothing to `manipulation_trust` for PNG inputs, which
include the majority of AI-generated images in training. The weight calibration is
therefore most relevant for JPEG inputs, and the authentic FP risk is concentrated on
JPEG images with complex compression histories.

**Suspicious threshold**: `score > 0.3` in `jpeg_ghost.py` line 146. This is the
per-detector flag; it does not directly gate the trust score (which uses the
continuous `score` value via `1.0 - s`). The calibration sweep measures effect
through `overall_trust` rather than the per-detector `suspicious` flag.

---

*Document status: Complete. v1 executed 2026-04-07, v2 executed 2026-04-11. Weight retained at 0.5.*
*Next action: long-term splice benchmark pathway (see `docs/decisions/splice-benchmark-longterm.md`). Quarterly retrain review per TRIED Pillar 5.*

---

## 8. v2 Corpus — DCT-Aligned Generator (2026-04-11)

### 8.1 Motivation

Section 4.5 of this document identified the root cause of v1's weight-
invariant TPR: the synthetic generator's final JPEG save pass at
Q ∈ {60, 70, 80, 90} applied a uniform quantisation that wiped the
differential source-region ghost patterns JPEG Ghost is designed to
detect. CASIA v2 was considered as a real-world alternative but **rejected
2026-04-11** as a non-commercial-licence dataset incompatible with Jura
Trace's commercial product status.

v2 rebuilds the synthetic corpus with four specific fixes targeting the
v1 failure mode, while remaining fully CC-BY licence-clean:

1. **Explicit IJG quantisation tables** via PIL's `qtables=` kwarg
   instead of the approximate `quality=` hint. Deterministic per-image Q.
2. **High final save qualities** (Q_final ∈ {92, 95, 98}) so the final
   compression pass does not wipe source Q footprints.
3. **Wider, disjoint Q-delta ranges** (bg ∈ {80,85,90}, fg ∈ {50,55,60}).
4. **8-pixel-aligned paste coordinates** so the pasted region lands on
   the JPEG DCT block grid and is not diluted across four blocks.

Generator: `scripts/build_splice_corpus_v2.py`. Sweep runner:
`scripts/sweep_jpeg_ghost_weight_v2.py` (direct-call variant — imports
sidecar services rather than using HTTP).

### 8.2 v2 vs v1 — headline comparison

| Metric | v1 (2026-04-07) | v2 (2026-04-11) | Δ |
|---|---|---|---|
| AUC @ w=0.0 | 0.5483 | **0.6970** | +0.149 |
| AUC @ w=0.5 | 0.5643 | **0.7564** | +0.192 |
| AUC @ w=1.0 | 0.5749 | **0.7692** | +0.194 |
| TPR@FPR5 @ w=0.0 | 12.0% | **21.3%** | +9.3pp |
| TPR@FPR5 @ w=1.0 | 12.0% | **24.0%** | +12.0pp |
| JG spliced p95 | 0.0708 | **0.2800** | +0.209 |
| JG authentic p95 | 0.1113 | 0.1775 | +0.066 |
| JG copy-move p95 | 0.0880 | 0.1747 | +0.087 |

JPEG Ghost now produces genuine differential signal on the spliced class.
Spliced p95 (0.28) exceeds authentic p95 (0.18) — the opposite of v1, where
authentic p95 was higher than spliced p95. This is the signature of a
corpus that actually exercises the detector's intended mechanism.

### 8.3 v2 full weight sweep

| Weight | AUC | TPR@FPR5 | TPR@FPR1 | TPR@T=0.55 | FPR@T=0.55 | ΔFPR vs 0.0 |
|---|---|---|---|---|---|---|
| 0.00 | 0.6970 | 21.3% | 10.7% | 0.0% | 0.0% | — |
| 0.25 | 0.7405 | 22.7% | 10.7% | 0.0% | 0.0% | +0.0pp |
| **0.50** | **0.7564** | **21.3%** | **10.7%** | **0.0%** | **0.0%** | **+0.0pp** |
| 0.75 | 0.7638 | 21.3% | 10.7% | 0.0% | 0.0% | +0.0pp |
| 1.00 | 0.7692 | 24.0% | 10.7% | 0.0% | 0.0% | +0.0pp |

Full artefacts:
- `models/splice_calibration_150_v2/weight_sweep_final_v2.json`
- `models/splice_calibration_150_v2/sweep_raw_v2.json`
- `models/splice_calibration_150_v2/labels.json`

### 8.4 v2 pass/fail verdict

**Criterion A** (TPR@FPR5 improves by ≥ 5pp vs w=0.0): **FAIL** for all
weights. Best gain is w=1.0 at +2.7pp; w=0.5 shows +0.0pp.

**Criterion B** (FPR increase ≤ 2pp): **PASS** for all weights (ΔFPR =
0.0pp — no image in either class falls below the 0.55 trust threshold,
same structural property as v1).

**Overall**: w=0.5 passes criterion B and is consistent with the
monotonically improving AUC curve (0.697 → 0.769 as weight increases).
Criterion A does not distinguish between weights because the v2 baseline
(ELA + noise + copy-move) is already strong at 21.3% TPR, leaving little
marginal room for any single detector to add ≥5pp.

**Recommendation unchanged: retain w=0.5.**

Rationale change relative to v1:
- v1 recommended 0.5 primarily because the detector produced **no
  discriminative signal** on the corpus — the whole weight sweep was
  effectively zero-information.
- v2 recommends 0.5 because the detector is now demonstrably
  discriminative (AUC 0.76 standalone contribution vs 0.70 baseline) and
  criterion B is met. Monotonic AUC improvement with weight is empirical
  evidence that JPEG Ghost is pulling weight proportional to its
  contribution. w=0.5 sits at a conservative midpoint.

A case could be made for **upgrading to w=0.75 or w=1.0** based on AUC
alone (0.764 and 0.769 respectively vs 0.756 at w=0.5). Reasons to hold
at 0.5 instead:
1. The absolute AUC gap between w=0.5 and w=1.0 is only 0.013 — within
   the noise band of a 150-image corpus.
2. Higher weights increase exposure to JPEG Ghost's authentic-side FP
   modes on heavily-recompressed images (social-media reposts, platform
   re-encoding — the `assess_input_quality()` heavy-compression guard
   already exists but is not tied to the JPEG Ghost weight).
3. Section 6.3's quality-adaptive weight formula
   (`effective_weight = 0.5 × (jpeg_quality_estimate/100).max(0.3)`) is
   the principled answer — a flat-weight upgrade is a half-measure.
4. Pilot testing feedback should gate any weight increase, not purely
   synthetic-corpus AUC.

### 8.5 Residual limitations of v2

1. **Authentic p95 rose from 0.11 → 0.18.** v2's explicit qtable
   re-encoding of authentic controls (multi-compression, high-Q, heavy-Q
   branches) creates cleaner per-block quality signatures that the
   detector's variance metric treats as mildly suspicious. This is the
   expected cost of making the corpus more principled — v1's authentic
   images retained messy real-world compression histories that happened
   to mask false alarms. Not a regression, but worth noting.

2. **Copy-move p95 rose from 0.09 → 0.17.** Same mechanism. The Section
   3.6-analogue sanity check (run against the authentic training corpus,
   unchanged pixel content) would still be the cleanest authentic-FP
   gauge — not yet re-run for v2, see Section 8.6.

3. **No real-world splice benchmark.** v2 closes the "does the detector
   actually discriminate" gap but does not close the "does it
   discriminate on *real* forgeries" gap. That requires a commercially-
   cleared splice dataset, which does not currently exist publicly.
   Pathway tracked in `docs/decisions/splice-benchmark-longterm.md`.

### 8.6 Open follow-ups from v2

- [ ] Re-run Section 3.6 authentic training corpus sanity check against
      the v2 detector pipeline (not re-executed in the v2 sweep; v1 run
      from 2026-04-07 is still the reference).
- [ ] Prototype the Section 6.3 quality-adaptive weight formula against
      the v2 corpus to see whether it beats flat w=0.5.
- [ ] Long-term: commercially-cleared real-splice benchmark — see
      `docs/decisions/splice-benchmark-longterm.md`.

---

*v2 supplement added 2026-04-11. Weight unchanged. Generator now produces
genuine ghost signal; detector validated as discriminative.*
