# UnivFD v10 Retrain Plan — Screenshot FP Reduction

**Status**: Planning — not yet executed
**Created**: 13 April 2026
**Owner**: TBD
**Prerequisite**: Complete v1.0 release (content-type classifier ships as interim mitigation)

## Problem

UnivFD v9 produces **100% false positives on authentic screenshots** (20/20 in April 2026 validation). The GBM v4 deepfake classifier produces 65% FP on the same sample.

Root cause: the training corpus contained no authentic screenshot class. Screenshots share signal profile with AI-generated images (no camera EXIF, uniform compression, smooth gradients, no demosaicing artefacts) and the probe has learned this conflation.

**Current mitigation (shipped v1.0)**: content-type classifier at `/forensics/content-type` heuristically detects screenshots pre-detection; the Rust verify pipeline suppresses deepfake + CLIP contributions to the trust score when `ai_detection_suitable=false`. UI banner informs the user.

The heuristic classifier is a patch, not a fix. A proper retrain is the long-term solution.

## Goal

UnivFD v10 probe that:
1. Maintains AI recall ≥95% (current v9: 95.70%)
2. Maintains authentic FP ≤5% overall (current v9: 4.12%)
3. Reduces screenshot-specific FP from ~100% → <10%
4. Does not regress known-weak generator families

## Corpus additions required

### New authentic category: screenshots

Target: **600 authentic screenshots**, balanced across:

| Source | Count | Notes |
|--------|-------|-------|
| macOS screenshots (Retina, standard) | 100 | Native `.png` + `xattr com.apple.metadata:_kMDItemUserTags` preserved |
| Windows 11 screenshots | 100 | PrintScreen + Snipping Tool variety |
| iOS screenshots (various devices) | 100 | iPhone 13–15, iPad, with HEIC and PNG variants |
| Android screenshots | 100 | Pixel, Samsung, OnePlus flagship resolutions |
| Social media screenshots (Twitter/X, Instagram, TikTok) | 100 | Screenshots of *authentic* posts, not of AI content |
| Document scans + receipts | 100 | A4 scans, phone-camera-of-document, receipt photos |

**Acquisition approach:**
- Solicit from pilot testers with consent (anonymised — no personal content)
- Scrape CC-BY-licensed UI screenshot datasets (if any exist commercially-clean)
- Generate synthetic screenshots using headless Chrome + random websites (Wikipedia, BBC, Gov.uk — public domain sources)
- Document scans: use Project Gutenberg page scans, National Archives public records

**Licensing**: must be commercially usable (no CASIA, no CelebA). Prefer CC-BY, CC0, or owned corpus.

### New AI category: screenshots of AI images (laundering)

Target: **400 screenshots of AI-generated content**, balanced across:

| Source | Count | Notes |
|--------|-------|-------|
| Screenshot of closed-source commercial generator output | 100 | macOS + iOS screenshots |
| Screenshot of open-weights diffusion model variants | 100 | Outputs screenshot via Chrome |
| JPEG re-save of AI images (social media forwarding) | 100 | Q=75/85/2x (extend existing platform-forwarded corpus) |
| HEIC re-encode of AI images | 100 | iOS forwarding pattern |

This is the laundering test set — AI content should still score AI even after screenshot.

## Training corpus composition (v10)

| Category | Current v9 | v10 target |
|----------|-----------|-----------|
| Authentic — camera photos | 5,724 | 5,724 (unchanged) |
| Authentic — screenshots (NEW) | 0 | 600 |
| Authentic — documents (NEW) | 0 | 100 |
| AI — raw generator output | 4,985 | 4,985 (unchanged) |
| AI — platform-forwarded (Q=75/85/2x) | 32,142 | 32,142 (unchanged) |
| AI — screenshots of AI (NEW) | 0 | 400 |
| **Total training samples** | **39,016** | **~43,050** |

## Pipeline

1. **Collect 700 authentic screenshots + 400 AI screenshots** (solo effort, 2–3 days with tooling)
2. **Extend `scripts/build_augmented_training_set.py`** to include the new categories with balanced class weights
3. **Retrain LogisticRegression probe** on CLIP ViT-B/32 embeddings (same architecture as v9; only corpus changes)
4. **Validate on held-out test set**:
   - 100 new authentic screenshots (unseen)
   - 50 new AI screenshots (unseen)
   - Full v9 test set (ensure no regression on existing categories)
5. **Stratified metric check**:
   - Overall AUC-ROC ≥ 0.99
   - Authentic FPR ≤ 5%
   - Screenshot-specific FPR < 10%
   - Per-generator recall within −2 pp of v9 baselines

## Promotion criteria

Promote v10 over v9 only if ALL of these hold:

- Overall AUC-ROC not worse than v9 (0.9933) − 0.005
- Authentic FPR (all sources) ≤ 5.0%
- Screenshot-specific FPR ≤ 10%
- No generator family drops more than 3 pp recall vs v9
- Human review of 50 boundary cases (scores 0.4–0.6) agrees with model

If any criterion fails: iterate the corpus (more screenshots, more laundered AI, class weights adjustment) rather than promoting a regressive model.

## Rollback

- v9 joblib retained at `models/univfd_probe_v9.joblib`
- Hash pinned in `sidecar/app/services/clip_detector.py`
- Settings toggle to revert to v9 in case of field issues
- Feature flag: `JURA_UNIVFD_VERSION=v9|v10` env var for pilot comparison

## Estimated effort

- Corpus collection: 2–3 focused days (with scripts)
- Training + validation: 1 day (CLIP embedding extraction + LogReg fit is fast)
- Integration + testing: 1 day
- **Total**: ~1 week of focused work

## Timing

- **v1.0 release**: ships with content-type heuristic classifier (current mitigation)
- **v1.1 or v1.2**: ships with UnivFD v10 retrain
- Gated on: pilot feedback confirms the heuristic classifier is adequate short-term, or proves insufficient and needs acceleration

## Success metric for the pilot

During the pilot, track:
- How often the heuristic classifier fires (`ai_detection_suitable=false`)
- How often it's correct (user confirms "yes this is a screenshot")
- How often it misclassifies (photo flagged as screenshot → detection unnecessarily suppressed)

If the heuristic classifier achieves ≥85% accuracy on pilot submissions, v10 retrain can be deferred to v1.2. If accuracy is <85%, accelerate to v1.1.

## Related

- `docs/calibration/univfd-v9-platform-augmentation.md` — v9 training plan and metrics
- `sidecar/app/services/content_type.py` — interim heuristic classifier
- `docs/model-cards.md` — will need v10 update when trained
