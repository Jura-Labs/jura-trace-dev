# UnivFD v10 Multi-Format Augmentation Retrain — Execution Plan

**Authored**: 2026-05-11 by ml-data-scientist agent (full plan in agent-memory; this is the canonical execution doc).
**Decided**: 2026-05-11 by Paul Griffiths — full retrain committed, willing to accept v1.0 launch slip if necessary.
**Trigger incident**: tester re-encoded a known-AI JPEG into PNG; verdict flipped from "Low Trust 35%" to "High Trust 70%". Safety cap (`Moderate ceiling without positive authenticity signal`) landed 2026-05-11 as emergency mitigation. This plan is the underlying-capability fix.

---

## Scope

Multi-format augmentation retrain of **UnivFD v9 → v10** to address AI-detection failures on non-JPEG image formats. Four target formats:

- **PNG** (lossless, common AI export from Firefly + Midjourney + diffusers)
- **TIFF** (lossless, archive-grade — important for museum / cultural-heritage workflow)
- **WebP** (modern lossy q=80, increasingly common as AI export)
- **HEIC** (iPhone default; AI images saved to Photos roll convert to HEIC)

**GBM v4 retraining is deferred to v1.0.1** — full GBM v5 retrain not feasible in the v1.0 window. Instead, a 30-min "format-aware confidence floor" patch lands in v1.0: on `codec_class == "lossless"`, floor the GBM score at 0.3 to prevent worst-case tree misfires. GBM v5 ships in v1.0.1 alongside Article 50 audit-log + ProofMode bundle compatibility.

## Failure-mode precision

**UnivFD v9 (primary cause)**: training corpus was 95% JPEG (10,712 originals + 32,142 JPEG re-encodes vs a small PNG minority). The CLIP ViT-B/32 embedding for a lossless PNG re-encode of an AI JPEG is slightly different from the original JPEG embedding; the LogReg probe boundary trained on the JPEG-dominant embedding distribution does not generalise to the PNG/TIFF/HEIC distribution and outputs low AI probability on them.

**GBM v4 (secondary)**: the 84-feature vector always runs `_extract_jpeg_features`, computing DCT statistics on the decoded pixel array regardless of source format. On PNG re-encodes, `blocking_strength` reads near 0.0 (no JPEG quantisation noise) and `dct_benford_div` falls outside the JPEG training range — pushing the tree toward "authentic". Conversely, the lossless-only features (`lsb_randomness`, `lsb_entropy_mean`, `demosaic_peak_count`, `demosaic_peak_strength`) are populated with actual values on PNG/TIFF but were near-constant zero in JPEG training — the tree has not learned to use them. GBM v4 confidence floor for `codec_class == "lossless"` patches the worst case without a full retrain.

## Schedule

| Week | Dates | Output | Hard gate at week-end |
|---|---|---|---|
| **W1** | 12-18 May 2026 | ~6,000 format-augmented samples (1,500 per format), SHA-256 manifest, authentic non-JPEG coverage check + top-up to ≥150 per format | **Gate 1**: authentic balance check passes — no severe imbalance that would produce format-as-label confound |
| **W2** | 19-25 May 2026 | CLIP embeddings extracted (~15 min MPS), UnivFD v10 LogReg retrain with C-grid search, JPEG regression evaluation | **Gate 2 (hard)**: JPEG FP ≤ 5.12% (= 4.12% + 1.0pp ceiling) AND recall ≥ 94.7% (= 95.70% − 1.0pp) AND demographic bias test passes — if fails, abort v10, ship v1.0 with safety cap only |
| **W3** | 26 May - 1 Jun 2026 | Per-format native-export benchmark (~800 images: 50/generator × 4 generators × 4 formats from Firefly + Midjourney + SDXL + Flux), threshold calibration | **Gate 3**: per-format AUC ≥ 0.85 (PNG / TIFF / WebP) and ≥ 0.80 (HEIC if generated successfully) |
| **W4** | 2-8 Jun 2026 | GBM v4 format-aware confidence floor patch, v10 wired into sidecar, sidecar tests, methodology paper finalised | **Gate 4 (hard cut-off 15 Jun)**: sidecar tests pass, demographic bias pass — v10 promoted OR deferred to v1.0.1 with safety cap as backstop |
| Buffer | 9-15 Jun 2026 | rc.25 integration, final promotion decision | If gates all pass: v10 ships in v1.0. If not: safety cap is the v1.0 mitigation, v10 → v1.0.1 |

**Hard cut-off for v10 in v1.0**: 15 June 2026. After this date, v10 ships in v1.0.1 regardless of progress.

**Launch implications**: user accepted launch slip if necessary. The locked 22 June date may flex to early-mid July if Gate 4 fails on schedule but the workstream remains active.

## Generation pipeline per format

### PNG (lossless re-encode)
```python
from PIL import Image
img = Image.open(source_jpeg)
img.save(output_path, format="PNG", optimize=False)
```
Pillow-only, ~0.3 s/image.

### TIFF (lossless, LZW compression)
```python
img.save(output_path, format="TIFF", compression="lzw")
```
Pillow-only, ~0.4 s/image. LZW (universally readable, not patent-encumbered post-2003).

### WebP (lossy q=80, method=6)
```python
img.save(output_path, format="WEBP", quality=80, method=6)
```
Pillow + libwebp (pip default on macOS). ~0.8 s/image. q=80 matches Midjourney/Firefly/Runway default downloads.

### HEIC (Apple-licensed sips)
```bash
sips -s format heic -s formatOptions 80 source.jpg --out output.heic
```
macOS-native `sips` uses Apple's licensed codec (AVFoundation). Patent-clean for personal/training use on Apple platforms; product never writes HEIC so distributed-product patent question does not arise. ~1.2 s/image with subprocess overhead.

**Total generation time**: ~70 min serially; ~20 min with `ProcessPoolExecutor(max_workers=4)` (HEIC capped at 2 workers to avoid sips contention).

**Storage**: 6,000 images × ~2 MB avg = ~12 GB. Check USB headroom before starting.

## Authentic non-JPEG coverage (Step 1c — critical)

The v4 fix added authentic PNGs; verify count. Target ≥500 per format (preferred ≥1,500 for true 1:1 augmentation balance — see Risk 5 below). Format breakdown:
- **Authentic PNG**: count via `find`. If ≥500, OK; else top-up via re-encoding authentic JPEG corpus.
- **Authentic TIFF**: likely near zero in source corpus. Generate ~200 via same pipeline.
- **Authentic WebP**: likely near zero. Generate ~200.
- **Authentic HEIC**: actual iPhone-shot HEICs are ideal if present. Else generate ~200 from authentic JPEG corpus.

**Hard stop**: if authentic non-JPEG coverage severely imbalanced (e.g. 0 authentic TIFF vs 1,500 AI TIFF), GBM and UnivFD will learn format-as-label.

## CLIP embedding extraction (Week 2)

Extend `scripts/build_augmented_training_set.py` with `--multi-format-aug-ai` and `--multi-format-aug-authentic` arguments. Tag variants with `_fmt_png`, `_fmt_tiff`, `_fmt_webp`, `_fmt_heic` (parallel to existing `_plt75` / `_plt85` / `_plt2x`).

**Time**: ~15 min on MPS-accelerated batch (batch_size=32) with `open_clip` ViT-B/32.

**Output**: 6,000 new 512-dim float32 embeddings = ~12 MB. Store as `.npz` archive alongside split JSON for reproducibility.

## LogReg retrain (Week 2)

- Training corpus: existing 39,016 + ~6,000 new format-augmented samples ≈ 45,000 total.
- Grid search `C ∈ {0.1, 0.5, 1.0, 2.0, 5.0}` via 5-fold CV on combined training set excluding held-out test split.
- Re-use `models/univfd_v9_split.json` test set as JPEG regression holdout. **Do NOT add multi-format samples to the test split** — they are training augmentation only.
- Create separate per-format validation set from ~10% of new format samples (150 images per format).
- Save as `models/univfd_probe_v10.joblib`. Do not touch production until Gate 4.

## Per-format native-export benchmark (Week 3)

**Generator matrix** (50 images × 4 generators × 4 formats = 800 images total):

| Generator | Native PNG | Native TIFF | Native WebP | Native HEIC | Notes |
|---|---|---|---|---|---|
| Firefly | Yes | Photoshop save-as | Save-for-Web | sips post-conv | Adobe CC subscription, 25 free credits/mo |
| Midjourney v6 | Yes (PNG download) | Manual resave | Manual resave | sips post-conv | Discord subscription |
| SDXL local | Yes | Yes | Yes | sips post-conv | HuggingFace diffusers, no quota |
| Flux.1-dev local | Yes | Yes | Yes | sips post-conv | HuggingFace diffusers, no quota |

**Honest caveat documented in methodology paper**: HEIC is never generator-native; benchmark is "re-encoded from PNG", which mirrors real-world (AI image saved to iPhone Photos → HEIC).

**Time**: ~3 hours generation + ~1 hour organisation.

**Fallback if subscription-gated tools are constrained**: 100 images from SDXL + Flux + SD3.5 local + 25 each from Firefly + MJ = ~150 non-JPEG benchmark images total. Adequate for per-format AUC estimation (±0.05 CI at n=150).

## Threshold calibration (Week 3)

Current production threshold: 0.5 (single threshold for all inputs). Examine whether per-format threshold improves FP without hurting recall. Decision rule: if optimal PNG threshold differs from JPEG by >0.05 AND empirical benefit clear (≥1pp FP improvement, no recall loss), add format-conditional thresholds in `sidecar/app/services/deepfake.py`. Otherwise keep single threshold.

## GBM v4 format-aware confidence floor patch (Week 4)

30-min one-line patch in `sidecar/app/services/deepfake.py`:
```python
if codec_class == "lossless" and gbm_score < 0.3:
    gbm_score = 0.3  # uncertainty floor — GBM v4 not trained on lossless format distribution
```
Documented as known limitation. GBM v5 ships in v1.0.1.

## Trust formula adjustment after v10

**Pre-v10 (today)**: Moderate ceiling without positive authenticity signal (`cameraAuthenticityBonus > 0.5` OR valid C2PA without AI declaration).

**Post-v10 promotion**: relax for non-lossless formats:
- Allow High Trust if: UnivFD v10 AI probability < 0.3 AND GBM v4 AI probability < 0.35 AND `codec_class != "lossless"`.
- **Keep Moderate cap for lossless formats** (PNG, TIFF, HEIC, WebP) until GBM v5 ships in v1.0.1.

Documented audit trail for legal/compliance side: "we knew about the GBM gap, we capped it, we document when the cap is lifted."

## Risk register

| # | Risk | Probability | Mitigation |
|---|---|---|---|
| 1 | PNG augmentation overfits, regresses JPEG performance | Moderate | Enforce ≥500 authentic PNG samples. Gate 2 hard stop. If PNG augmentation causes JPEG FP regression >0.5pp, reduce PNG count from 1,500 to 750, retrain once before checking 1.0pp gate. |
| 2 | HEIC corpus generation flakey on macOS `sips` | Low-moderate | Pilot 10-image validation first. Fallback: `ffmpeg -i input.jpg output.heic` with libheif. If HEIC unsupportable, drop from augmentation, document explicitly. HEIC is least critical (Apple converts to JPEG on share). |
| 3 | Firefly/MJ native-PNG benchmark too small | Moderate | Fallback to SDXL + Flux + SD3.5 local only (zero subscription dependency). Benchmark composition documented in methodology paper. |
| 4 | Codeberg migration block (JTV-149 to JTV-155) consumes the retrain window | Moderate | v10 deferred to v1.0.1 if migration consumes Weeks 2-3. Safety cap is the v1.0 mitigation regardless. |
| 5 | JPEG regression narrowly fails (4.8-5.1% FP) due to class imbalance | Low-moderate | Rebalance augmented corpus to 1:1 authentic:AI ratio (1,500 each per format). Adds ~1 day corpus work. |

## Go/no-go gate detail

**Gate 1 (W1)** — corpus ready:
- Authentic non-JPEG ≥150 per format: proceed.
- HEIC generation fully failed: proceed with PNG + TIFF + WebP only, document deferred HEIC.

**Gate 2 (W2)** — hard JPEG regression check:
- FP ≤ 5.12% AND recall ≥ 94.7% AND demographic bias passes: proceed.
- FP > 5.12% OR recall < 94.7%: attempt rebalance (1:1) and retrain once. Second failure: abort v10, ship v1.0 with safety cap.
- FP in 4.12%-5.12% range, recall ≥ 94.7%, bias passes: proceed but flag "marginal pass" in methodology paper.

**Gate 3 (W3)** — per-format benchmark:
- Per-format AUC ≥ 0.85 on PNG/TIFF/WebP AND ≥ 0.80 on HEIC: promote candidate, proceed to integration.
- One format <0.80 + Gate 2 passed: proceed but document weak format explicitly. Safety cap remains backstop for all lossless formats.
- Multiple formats <0.80: rebalance augmentation or consider dropping a format.

**Gate 4 (15 Jun hard cut-off)** — integration complete or deferred:
- All gates pass + sidecar tests pass: promote to production. Update CLAUDE.md, `tauri.conf.json` bundles (per `project_models_dual_path_p0.md` — use the `src-tauri/models/` path), tag rc.25 with v10.
- Any gate failed or integration incomplete: ship v1.0 with safety cap only. File "UnivFD v10 multi-format" as tracked v1.0.1 item.

## Methodology paper

`docs/calibration/univfd-v10-multi-format-augmentation.md` — written in W4. Skeleton:

1. Motivation and failure mode (~150 words)
2. Training corpus extension (~200 words)
3. CLIP embedding extraction (~100 words)
4. LogReg retrain — hyperparameter search (~150 words)
5. JPEG regression validation (~200 words)
6. Per-format native-export benchmark (~300 words)
7. Threshold calibration (~150 words)
8. GBM v4 format-aware floor patch (~100 words)
9. Trust formula post-v10 (~100 words)
10. Known limitations and future work (~150 words)

## Operational notes

- **Codeberg migration + retrain compete for the same 4-6 weeks.** Do not interleave on the same day — context switching cost is high. Corpus generation (W1) and embedding extraction (W2 first half) automate well and can run overnight.
- **Authentic non-JPEG coverage gap is the easiest thing to underweight** and the most likely cause of a JPEG regression failure. Budget half a day in W1 just for this.
- **Per-format benchmark (W3) is the most deferrable.** If behind at Gate 2 decision, cut to SDXL + Flux local only; treat Firefly + MJ as v1.0.1 validation.
- **If Gate 2 passes marginally (FP 4.6%, within ceiling but up from 4.12%)**: do not promote v10 silently — document the regression in methodology paper and promotion decision record. Demographic bias test may explain part of it; understand cause before shipping.

## Cross-references

- Memory `project_v1_live_release.md` — 22 June 2026 launch target (may flex pursuant to this retrain)
- Memory `project_competitor_landscape_may2026.md` — competitor positioning (UnivFD v10 closes a known gap)
- Memory `project_splice_benchmark_licensing.md` — CASIA rejected; commercial-cleared corpora only; self-generated benchmark images are unambiguously cleared
- Memory `project_models_dual_path_p0.md` — corpus on external USB; `tauri.conf.json` bundles `src-tauri/models/` not canonical `models/`
- Memory `project_detection_improvement.md` — FP-reduction history
- CLAUDE.md — current model state (GBM v4 + UnivFD v9)
