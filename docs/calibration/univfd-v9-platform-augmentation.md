# UnivFD v9 — Platform-Forwarded Augmentation Retrain

**Status**: CANDIDATE — awaiting human promotion decision  
**Date**: 2026-04-11  
**Backlog item**: #16 — platform-forwarded augmentation retrain  
**Author**: ML data scientist agent  

---

## 1. Context

### Why this retrain happened

The v3 JPEG Ghost splice calibration corpus (`scripts/build_splice_corpus_v3.py`, run 2026-04-11) exposed a structural detection blind spot: when a spliced image is re-encoded by a social platform (Twitter, WhatsApp), the differential JPEG ghost signal collapses.

| Corpus condition | JPEG Ghost p95 |
|---|---|
| Authentic plain | 0.035 |
| Platform-forwarded splice | 0.041 |
| Clean splice (no re-encoding) | ~0.12 |

A p95 of 0.041 on a platform-forwarded splice is indistinguishable from authentic at 0.035. The JPEG Ghost detector becomes a noise floor contributor under these conditions.

### TruFor rejection (2026-04-11)

The content-authenticity-expert agent proposed integrating TruFor (Guillaro et al., CVPR 2023) — a learned camera-fingerprint residual detector specifically designed to survive social-media re-encoding. After licence review, TruFor was rejected:

> TruFor's `LICENSE.txt` prohibits "industrial or profit-oriented activities". Jura Labs' CIC status does not exempt it from this restriction. The entire GRIP-UNINA academic ecosystem uses equivalent non-commercial locks.

Reference: `docs/decisions/trufor-spike-rejected.md`

### The in-house alternative

Retrain the existing UnivFD probe (currently v8) on a platform-forwarded augmentation of the training corpus. The probe learns to classify synthetic versus authentic content from CLIP ViT-B/32 embeddings; CLIP features operate at the semantic/texture level and are more robust to JPEG recompression than low-level forensic signals. Exposing the probe to re-encoded training examples should improve recall on platform-forwarded content.

---

## 2. Methodology

### Corpus composition

| Subset | Authentic | AI-generated | Total |
|---|---|---|---|
| Original (v8 training corpus) | 5,727 | 4,985 | 10,712 |
| Platform-forwarded augmentation | ~17,181 | ~14,955 | ~32,136 |
| **Grand total (pre-split)** | **~22,908** | **~19,940** | **~42,848** |

Augmented images are pure Pillow JPEG re-compressions of the original corpus. No new source images were introduced. All augmented images inherit the licence of their source.

### Augmentation variants

| Tag | Procedure | Platform analogue |
|---|---|---|
| `_plt75` | PIL save Q=75, reload | Twitter standard recompression |
| `_plt85` | PIL save Q=85, reload | WhatsApp standard recompression |
| `_plt2x` | PIL save Q=85, reload, save Q=75, reload | Cross-platform forwarding chain |

**Subsampling**: `subsampling=2` (4:2:0 chroma, matching default platform behaviour).

**Filters applied**: images with short side < 256 px or file size > 20 MB were excluded from augmentation.

**Output location**: `/Volumes/Samsung USB/Training Data/corpus/training_platform_forwarded/` — sibling of the original corpus. Not committed to git.

**Idempotency**: script checks source SHA-256 against the manifest before regenerating. Re-running is safe.

### Training / validation split

- **Method**: 10% stratified held-out test set, stratified by (label, source subdirectory, variant tag)
- **Seed**: 42
- **Split persistence**: `models/univfd_v9_split.json`

This ensures the test set contains proportional representation from every generator family and every augmentation variant.

### Model configuration

| Hyperparameter | Value |
|---|---|
| Model class | `sklearn.linear_model.LogisticRegression` |
| Regularisation C | 0.5 (same as v8 production probe) |
| Solver | lbfgs |
| Class weights | balanced |
| Max iterations | 1000 |
| Random seed | 42 |
| CLIP backbone | ViT-B-32 (laion2b_s34b_b79k) — frozen, unchanged from v8 |

### Reproducibility commands

```bash
# Step 1 — generate augmented corpus (pilot first)
python scripts/augment_corpus_platform_forwarded.py --count 30
python scripts/augment_corpus_platform_forwarded.py

# Step 2 — train v9 probe
python scripts/build_augmented_training_set.py

# Step 3 — inspect candidate
ls -lh models/univfd_probe_v9.joblib
cat models/univfd_probe_v9_meta.json
```

---

## 3. Baseline (UnivFD v8)

Trained 2026-04-07, production model.

| Metric | Value |
|---|---|
| Training corpus | 10,712 images (5,727 authentic, 4,985 AI) |
| AUC-ROC | 0.9911 |
| Authentic FP rate | 5.01% |
| AI recall | 96.01% |
| CLIP backbone | ViT-B-32 (laion2b_s34b_b79k) |
| SHA-256 | `d16fb22baf3981d62888e2458733c1a4c0743a5895470d82e9de76766f776908` |

v8 was trained exclusively on the original, unaugmented corpus. It has never seen platform-forwarded examples.

---

## 4. Results

*This section will be populated after the training run completes. The scripts are ready; execution requires Bash access and the USB drive to be mounted.*

### 4.1 Overall held-out test set metrics

| Metric | v8 (baseline) | v9 (candidate) | Delta |
|---|---|---|---|
| AUC-ROC | 0.9911 | TBD | TBD |
| Authentic FP rate | 5.01% | TBD | TBD |
| AI recall | 96.01% | TBD | TBD |
| Training samples | 10,712 | ~38,563 (90%) | +~27,851 |

### 4.2 Platform-forwarded-specific AUC

*The main question: does exposure to augmented training examples improve detection of re-encoded synthetic content?*

| Test subset | v8 AUC (estimated) | v9 AUC | Delta |
|---|---|---|---|
| original (no re-encoding) | ~0.9911 | TBD | TBD |
| plt75 (Q=75) | TBD | TBD | TBD |
| plt85 (Q=85) | TBD | TBD | TBD |
| plt2x (double-pass) | TBD | TBD | TBD |

### 4.3 Per-generator AI recall

*Any generator family whose recall drops more than 2 percentage points from v8 is flagged as a regression.*

| Generator family | v8 recall | v9 recall | Delta |
|---|---|---|---|
| DALL-E 3 | 91.4% | TBD | TBD |
| Midjourney | ~100% | TBD | TBD |
| Flux | ~100% | TBD | TBD |
| SDXL | ~100% | TBD | TBD |
| Civitai SFW | 75.8% | TBD | TBD |
| DiffusionDB | 67.6% | TBD | TBD |
| (others) | ~100% | TBD | TBD |

---

## 5. Verdict

*Pending execution. Decision criteria:*

**Promote to production** if:
- v9 AUC-ROC >= v8 (0.9911) on the overall test set, AND
- v9 platform-forwarded AUC (plt75/plt85/plt2x subsets) shows meaningful improvement vs estimated v8 on the same subsets, AND
- No per-generator recall regression > 2pp, AND
- Authentic FP rate does not exceed 6.5% (v8 + 30% tolerance).

**Retain v8** if:
- v9 AUC-ROC < 0.9850 (hard floor — significant regression), OR
- Any generator family drops below 60% recall, OR
- Authentic FP rate exceeds 8%.

**Collect more data** if:
- v9 improves platform-forwarded AUC but the overall AUC regresses modestly (< 1pp) — the augmentation strategy is working but the volume of original clean data needs to grow proportionally.

**Trade-off zone**: if v9 gains on platform-forwarded subsets but loses < 0.5pp on clean original images, this is a known expected trade-off (the model is attending more to features that survive recompression and less to features that are destroyed by it). This should trigger a human decision, not automatic promotion or rejection.

---

## 6. Known Limitations

1. **Pillow is not a real platform pipeline.** PIL's JPEG encoder uses the IJG libjpeg quantisation tables, which differ from Twitter's (WebP first, then JPEG fallback) and WhatsApp's (custom tables, aggressive chroma subsampling). Real platform re-encoding also strips XMP/EXIF metadata and applies resolution limits. The augmentation is a useful approximation, not a faithful simulation.

2. **No platform-forwarded authentic ground truth.** We apply the same re-encoding to authentic images, which creates the correct training signal for the authentic class — but we do not have real-world platform-forwarded authentic photos from actual device/upload pipelines. This is a minor concern because the CLIP backbone is largely format-agnostic, but it is worth noting.

3. **CLIP features may be partially immune to Q-level variation already.** If CLIP ViT-B/32 was pre-trained on images at various quality levels (likely, given LAION-2B scale), the v8 baseline may already be reasonably robust. The augmentation retrain tests this hypothesis empirically.

4. **DiffusionDB and Civitai SFW already have low recall in v8** (67.6% and 75.8%). These generators produce older SD 1.x-style outputs whose CLIP embedding distribution overlaps more with authentic images. Augmenting with platform-forwarded versions of these generators may not fix the underlying distribution overlap.

5. **Demographic composition unchanged.** This retrain adds augmented variants of the existing corpus; it does not change the demographic composition (skin tone, gender presentation, geographic origin of authentic photos). A TRIED Pillar 4 demographic audit of the augmented corpus is a follow-up task.

---

## 7. Reproducibility Record

| Item | Value |
|---|---|
| Random seed | 42 |
| Test fraction | 10% stratified |
| Split JSON | `models/univfd_v9_split.json` |
| Corpus manifest | `models/platform_augmentation_manifest.json` |
| Augmentation script | `scripts/augment_corpus_platform_forwarded.py` |
| Training script | `scripts/build_augmented_training_set.py` |
| Candidate model | `models/univfd_probe_v9.joblib` |
| Candidate metadata | `models/univfd_probe_v9_meta.json` |
| CLIP backbone | ViT-B-32 laion2b_s34b_b79k (unchanged from v8) |
| Python environment | See `sidecar/requirements.lock` |
| Training date | 2026-04-11 (scripts), execution TBD |

SHA-256 of the candidate model: recorded in `models/univfd_probe_v9_meta.json` after training run completes.
