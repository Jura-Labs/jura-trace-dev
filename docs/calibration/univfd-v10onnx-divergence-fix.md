# UnivFD Probe v10onnx — PyTorch↔ONNX Calibration Fix

**Date**: 2026-05-11
**Author**: Paul (Jura Labs CIC) — diagnosis + retrain executed in a single session
**Tickets**: JTV-143 (ONNX switch), v10 retrain commit
**Status**: Promoted to production 2026-05-11
**Supersedes**: UnivFD v9 (2026-04-12) and v10 PyTorch candidate (2026-05-11 morning)

---

## What changed

Replaced the v10 probe with **v10onnx** — a re-trained LogReg probe whose CLIP ViT-B/32 training embeddings come from the **production sidecar's PIL + ONNX runtime path**, not from `open_clip` + PyTorch.

| File | Before | After |
|------|--------|-------|
| `src-tauri/models/univfd_probe.joblib` | v9 (`ed691b45…`) | v10onnx (`0534a9e8…`) |
| `sidecar/app/services/clip_detector.py:_UNIVFD_PROBE_SHA256` | `ed691b45…` | `0534a9e8…` |
| `sidecar/app/services/clip_detector.py:_MODEL_VERSION` | `"univfd-probe-v9"` | `"univfd-probe-v10onnx"` |

---

## Why a re-train was needed

UnivFD v9 (2026-04-12) and the v10 PyTorch candidate (2026-05-11 morning) both extracted CLIP ViT-B/32 embeddings via `open_clip.create_model_and_transforms("ViT-B-32", pretrained="laion2b_s34b_b79k")`. open_clip's `preprocess` callable is a `torchvision.transforms.Compose` ending in `Resize(224, interpolation=BICUBIC, antialias=True)`.

Since **JTV-143** (3 May 2026, commit `e2ea8a2`), the shipped sidecar instead extracts embeddings via **ONNX runtime** + a manually-written PIL preprocess (`_preprocess_image` in `clip_detector.py:130-168`).

**The two preprocess paths are NOT bit-equivalent**:

- torchvision's `Resize` with `antialias=True` applies an explicit antialiasing prefilter before downsampling.
- PIL's `Image.resize(..., BICUBIC)` applies the raw cubic-convolution kernel **without** an antialiasing prefilter.

Same nominal algorithm; **different downsampling kernels**. The resulting embeddings diverge with **mean cosine similarity 0.996** and **min 0.978** on a 20-image civitai_sfw test set (see `scripts/diagnose_clip_onnx_divergence.py`).

A probe trained on one distribution and served on a different distribution is calibrated for the wrong inputs. In production this manifested as:

- AI images re-encoded JPEG→PNG passing as authentic (the original symptom that triggered this work)
- An undocumented score-distribution offset across all inputs

---

## What was investigated

A four-stage diagnostic locked the root cause to preprocessing:

1. **scripts/check_clip_pytorch_vs_onnx.py** — naive PyTorch-vs-ONNX comparison. Result: mean cos 0.996, far below the 0.999 equivalence threshold.

2. **scripts/diagnose_clip_onnx_divergence.py** — three-pipeline isolation:
   - P1: PyTorch preprocess + PyTorch model (reference)
   - P2: PyTorch preprocess + ONNX model
   - P3: PIL preprocess (sidecar) + ONNX model (production)

   Results:
   - `P1 vs P2`: mean cos **1.000000**, max L2 0.000005 → ONNX model is bit-identical to PyTorch model.
   - `P2 vs P3`: mean cos **0.995988**, max L2 0.209 → preprocesses produce different tensors.
   - `P1 vs P3`: mean cos 0.995988 → combined divergence.

   **Diagnosis**: PREPROCESS divergence (Suspect A). Model graph is faithful.

3. **scripts/find_clip_preprocess_fix.py** — A/B test of six PIL preprocess variants against torchvision's antialiased BICUBIC:

   | Variant | Mean cos | Min cos |
   |---------|----------|---------|
   | V0 BICUBIC (current) | 0.995988 | 0.978119 |
   | V1 LANCZOS | 0.987470 | 0.973477 |
   | V2 HAMMING | 0.989923 | 0.973235 |
   | V3 BILINEAR | 0.984497 | 0.964504 |
   | V4 BICUBIC + Gaussian blur | 0.970435 | 0.950183 |
   | V5 BICUBIC two-pass | 0.994675 | 0.976289 |

   **Conclusion**: no PIL variant can match torchvision's antialiased BICUBIC. The current implementation is already the closest available.

4. **Decision**: rather than emulate torchvision (which would require either bundling torchvision back into the sidecar — defeating JTV-143's whole purpose — or writing a custom antialiased-bicubic kernel in numpy), **retrain the probe on the production preprocess path** so the training distribution and the serving distribution match.

---

## How the retrain was executed

`scripts/build_augmented_training_set.py` was extended with a `--feature-extractor sidecar_onnx` flag that swaps the PyTorch+open_clip feature-extraction backend for the **bit-identical mirror** of `sidecar/app/services/clip_detector.py:_preprocess_image` + ONNX runtime.

```bash
python3 scripts/build_augmented_training_set.py \
    --feature-extractor sidecar_onnx \
    --model-version v10onnx \
    --split-json models/univfd_v10onnx_split.json
```

Corpus (unchanged from v10):

| Source | Authentic | AI |
|--------|-----------|----|
| Original training corpus | 7,128 | 4,985 |
| Platform-forwarded augmentation (q=75/85/2×) | 17,727 | 14,505 |
| Multi-format augmentation (PNG/TIFF/WebP/HEIC) | 5,999 | 6,000 |
| **Total** | **30,854** | **25,490** |

Total: 56,344 samples. 10% held-out stratified test = 5,633 (3,078 auth + 2,555 AI). LogReg `C=0.5` (matches v9, v10).

---

## Held-out test results

All three hard gates pass; per-format and per-generator stay strong:

| Metric | v9 baseline | v10onnx | Gate |
|--------|-------------|---------|------|
| AUC-ROC | 0.9933 | **0.9929** | ≥ 0.9883 ✅ |
| FP rate | 0.0412 | **0.0387** | ≤ 0.0512 ✅ |
| AI recall | 0.9570 | **0.9577** | ≥ 0.9470 ✅ |

**Per-format AUC** (multi-format retrain target):

| Format | AUC | n |
|--------|-----|---|
| PNG | 0.9978 | 300 |
| TIFF | 0.9945 | 300 |
| WebP | 0.9926 | 300 |
| HEIC | 0.9900 | 300 |
| Original JPEG (regression) | 0.9911 | 1,211 |
| plt2x (platform-forwarded) | 0.9952 | 1,074 |
| plt75 | 0.9912 | 1,074 |
| plt85 | 0.9950 | 1,074 |

**Per-generator recall** (no regressions vs v10 PyTorch candidate):

| Generator | Recall | n |
|-----------|--------|---|
| grok_aurora | 1.0000 | 150 |
| midjourney_v6 | 1.0000 | 45 |
| dalle3 | 0.9933 | 150 |
| diffusiondb | 0.9800 | 150 |
| civitai_sfw | 0.9800 | 150 |
| synthetic_faces | 0.9889 | 90 |
| sdxl_turbo | 0.9333 | 90 |
| elsa | 0.9205 | 390 |
| flux_dev | 0.9136 | 81 |
| gemini | 0.8667 | 15 (low confidence) |
| __root__ | 0.8194 | 72 |

---

## Production model artefact

- **Path**: `src-tauri/models/univfd_probe.joblib`
- **SHA-256**: `0534a9e80e352a5bd8af5fc447d03e37be2e1aa68a05d81f05736d6ef8956a86`
- **Version string**: `univfd-probe-v10onnx`
- **Decision boundary**: synthetic ≥ 0.60, authentic ≤ 0.35 (unchanged from v9; thresholds are probe-output-domain, not embedding-domain)
- **Surfaced on every `VerificationResult`** via `methodology.univfd_probe_model_hash` (JTV-181, see `docs/API_WRAPPER.md`)

---

## What this fixes

1. **Score drift on every CLIP-detected image** — previously the probe applied a v9-trained boundary to embeddings that differed by ~0.996 cos sim from training-time. The boundary itself was off by an unknown amount. Now matched.
2. **PNG/TIFF/WebP/HEIC AI-detection** — the multi-format augmentation that was the original v10 motivation. Per-format AUC ≥ 0.99 across all four target formats.
3. **The "AI re-encoded as PNG passes as authentic" regression** that surfaced during pre-launch testing on 2026-05-10.

## What this does NOT fix

- The torchvision↔PIL preprocess gap itself still exists in the code; we just no longer rely on the training and serving paths agreeing. If the sidecar's PIL preprocess is ever modified (e.g. changing interpolation flag) without re-training, the probe will silently miscalibrate again.
- **Gate for future preprocess edits**: any change to `_preprocess_image` in `clip_detector.py` MUST trigger a probe re-train against the new preprocess before promotion.

## Forward-compatibility note

If a future Pillow / numpy update changes the resize semantics of `Image.resize(..., BICUBIC)` (e.g. opting into antialiasing by default), the calibration will drift again. The sidecar startup probe-SHA check (`_UNIVFD_PROBE_SHA256`) will not catch this — it verifies the probe file, not the preprocess.

**Mitigation**: a CI smoke test that scores a fixed set of canary images and asserts the probe outputs stay within ±0.01 of recorded baselines. Tracked under v1.0.1 hardening.

---

## References

- `scripts/diagnose_clip_onnx_divergence.py` — root-cause diagnostic
- `scripts/find_clip_preprocess_fix.py` — A/B preprocess search
- `scripts/build_augmented_training_set.py` — retrain entry point with `--feature-extractor sidecar_onnx`
- `sidecar/app/services/clip_detector.py:130-168` — production preprocess (referenced canonical implementation)
- JTV-143 — commit that introduced the divergence
- `docs/decisions/jura-cli-binary-name-reservation.md` — JTV-181 / JTV-182 binary reservation
- `docs/API_WRAPPER.md#methodology` — `univfd_probe_model_hash` provenance block

---

## Promotion log

| Step | Status | Notes |
|------|--------|-------|
| Diagnose divergence | ✅ | Mean cos 0.996, isolated to preprocess |
| Re-extract training embeddings via PIL+ONNX | ✅ | 56,344 samples, single-shot run |
| Fit LogReg `C=0.5` | ✅ | Same hyper-params as v9 / v10 |
| Hard gate: FP ≤ 0.0512 | ✅ | 0.0387 |
| Hard gate: Recall ≥ 0.9470 | ✅ | 0.9577 |
| Hard gate: AUC ≥ 0.9883 | ✅ | 0.9929 |
| Per-format gate: all four ≥ 0.85 | ✅ | PNG 0.998, TIFF 0.995, WebP 0.993, HEIC 0.990 |
| Copy to `src-tauri/models/univfd_probe.joblib` | ✅ | SHA matches |
| Update `_UNIVFD_PROBE_SHA256` | ✅ | Now `0534a9e8…` |
| Update `_MODEL_VERSION` + `_THRESHOLD_BASIS` | ✅ | v10onnx |
| Rebuild Tauri | ⏳ | Pending — will produce the .dmg that pilot retests |
| Update memory `project_v10_retrain_committed.md` | ⏳ | Pending |
