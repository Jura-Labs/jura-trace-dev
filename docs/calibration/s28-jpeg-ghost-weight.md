---
title: "JPEG Ghost 0.5× Weight Validation — S28-FU9"
status: Outcome C — No splice corpus available locally. Sourcing plan below.
date: 2026-04-07
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

Not available. **Outcome C.** No splice corpus exists locally. Execute Step 5 (Sourcing
Plan) before returning to this section.

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

#### Option 2 — CASIA Image Tampering Detection v2 (CASIA v2)

- **URL**: https://github.com/namtpham/casia2groundtruth
  (ground-truth masks only; images via
  https://github.com/CasiaNuoDB/CASIA-CMFD/releases)
- **Scale**: 7,491 images (1,701 authentic, 5,123 spliced, 667 copy-move)
- **Licence**: Research-only. **Not suitable for commercial production model
  training.** Under Option C corpus strategy (`docs/decisions/option-c-corpus-strategy.md`),
  research-licensed data must not enter the production model training set. May be
  used for a separate research artefact under OpenRAIL-M.
- **JPEG Ghost relevance**: CASIA v2 includes both JPEG and non-JPEG authentic
  sources and a mix of copy-paste methods. Strongest benchmark for splice recall.
  Contains small-region (< 10% area) splice examples that are the hardest case.
- **Estimated effort**: 2–3 hours (registration form + download + mask alignment).

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
only option that is licence-clean for a commercial PolyForm NC product under the
Option C binding strategy.

Use **Option 2 (CASIA v2) as a research benchmark only** — run the same sweep against
CASIA v2 in a separate research artefact evaluation and record results separately from
the production calibration. This satisfies TRIED Pillar 1 (real-world adaptability)
evidence without contaminating the production corpus.

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
- [ ] (Optional) Submit CASIA v2 research access form and run research benchmark
      in parallel.

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

*Document status: Outcome C (no splice corpus). Sourcing plan adopted.*
*Next action: implement `scripts/build_splice_corpus.py` and return to Section 4.*
