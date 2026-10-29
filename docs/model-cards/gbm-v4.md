# Model card: GBM v4 (forensic-feature classifier)

Documentation version 1.0, 29 October 2026. Describes the classifier
shipped in Jura Trace v1.0.0 to v1.2.0.

This card is structured following NIST's public-facing AI documentation
zero draft (July 2026), NIST AI 300-1 initial public draft, Table 2 "Model
documentation template" and the default model profile in its Annex A.3.
It is not an attestation of conformity to that draft.

Every figure below names the file it comes from. Repository paths are in
this repository. Files under `models/` are not in the public source
snapshot. The public release asset `model-card-gbm.json` on the
[v1.2.0 release](https://github.com/Jura-Labs/jura-trace/releases/tag/v1.2.0)
carries the training counts, cross-validation AUC, parameters and feature
importances, but not the false positive rate or recall. The companion card,
which also describes how the two models are combined in the app, is
[v10onnx.md](v10onnx.md).

## 1. Identifying descriptors

| Field | Value | Source |
|---|---|---|
| Name | GBM classifier v4 | `model-card-gbm.json` (`model`) |
| Version identifier in results | `gbm-v4` | `sidecar/app/services/deepfake.py:651` (`MODEL_VERSION`) |
| Shipped file | `deepfake_classifier.joblib` | `scripts/build_model_card_assets.py`; `model-card-gbm.json` |
| SHA-256 of the shipped file | `512def7ec62cbeb023c5343859a15606a02742d48b1fca31df11667c0b9ba14a` | `model-card-gbm.json` (`shipped_sha256`); matches `src-tauri/models/deepfake_classifier.joblib` |
| Trained | 7 April 2026 | `models/deepfake_classifier_v4_meta.json` (`trained_at`); `model-card-gbm.json` |
| Developer | Jura Labs CIC | `THIRD-PARTY-LICENSES.txt` |
| Point of contact | feedback@juralabs.org | |
| Lineage | Development versions v1 to v4, 18 March to 7 April 2026; a v5 candidate (June 2026) failed its gates and was not shipped | `CHANGELOG.md` ("Model-cards simplified"); `docs/calibration/gbm-v5-v11-archive-jun2026.md` |

## 2. Intended use

**Intended task.** Give one score, from 0 to 1, for how closely the
statistical texture of a still image (noise, frequency content, colour,
local patterns, compression traces) resembles the AI-generated images in
its training set.

**How it is deployed.** The score is one of three inputs to the app's
deepfake score, weighted 0.30 when the UnivFD probe is also available
(`sidecar/app/services/deepfake.py:1115`). Its own verdict levels are
synthetic above 0.58 and authentic below 0.25, with the interval between
inconclusive (`deepfake.py:649-650`). The thresholds, the blending and the
trust-score bands, including what the Uncertain band means, are described
in [v10onnx.md](v10onnx.md), section 2.

**Out-of-scope uses.** The same as for the UnivFD probe
([v10onnx.md](v10onnx.md), section 2): not a verdict on its own; not for
video, audio or documents; not for images under 128 by 128 pixels; not for
non-photographic content; not for generators newer than the training data;
not for attributing an image to a generator. In addition, this model is
sensitive to the file format an image is carried in (section 6.4), so it
should not be read on its own for re-encoded images.

## 3. Usage rights

Distributed with Jura Trace under AGPL-3.0-or-later (`THIRD-PARTY-LICENSES.txt`;
`LICENSE`). It uses no third-party pre-trained model. The copyright holder's
position on AI training use of this source: [TRAINING.md](../../TRAINING.md).

## 4. Design

**Algorithm.** scikit-learn gradient boosting: 200 trees, maximum depth 4,
learning rate 0.05, minimum 2 samples per leaf, subsample 0.8
(`models/deepfake_classifier_v4_meta.json`, `model_params`;
`model-card-gbm.json`).

**Input.** 80 hand-crafted features computed from the image
(`n_features`, `feature_names` in the same files). By the feature names,
they cover: spectral decay and frequency-band energy; noise residual
statistics and autocorrelation; per-channel colour statistics and
correlations; saturation; local binary patterns; grey-level co-occurrence
texture; DCT coefficient statistics and Benford divergence; JPEG blocking
strength; edge and Laplacian sharpness statistics; multi-scale gradients;
cross-channel noise correlation; a VAE grid-energy ratio; a chromatic
aberration trend; least-significant-bit randomness and entropy; and
demosaicing peaks. The pipeline extracts 84 features and v4 uses 80 of them
(`deepfake.py:1199`).

**Most important features** (`top_features`, share of total importance):
`lsb_entropy_mean` 0.2051, `lsb_randomness` 0.1835, `blocking_strength`
0.1214, `az_var_band_0` 0.0835, `noise_mean_abs` 0.0732. The two
least-significant-bit features and JPEG blocking together carry about half
of the model's importance, which is one reason it reacts to re-encoding
(section 6.4).

**Explainability.** The app shows per-feature checks next to the score.
Those are explanatory thresholds on single features; the classifier can
give a high score without any single check firing (`deepfake.py:1197-1202`).

## 5. Training

All training data was assembled by Jura Labs. No third-party pre-trained
weights are involved.

| Figure | Value | Source |
|---|---|---|
| Training images | 10,712 | `models/deepfake_classifier_v4_meta.json` (`n_samples`); `model-card-gbm.json` |
| of which authentic | 5,727 | same, `n_authentic` |
| of which AI-generated | 4,985 | same, `n_ai_generated` |

**Sources.** Authentic images come from real camera photographs (consumer
phones, high-end cameras and drones), Wikimedia Commons photographs and
public research image datasets. AI-generated images come from the same kind
of generator families as the UnivFD probe's corpus
([v10onnx.md](v10onnx.md), section 6.3). Since June 2026 dataset and
generator names are withheld from public documentation (commit `773411b5`).

**Gaps.** The corpus is not published, and how each part was collected and
under what licence or consent is not yet written up publicly. This card
makes no claim either way. The training metadata records no augmentation
for platform forwarding or file formats; those were added for the UnivFD
probe.

## 6. Evaluation

### 6.1 Method

Two different measurements exist, and they are not equally strong.

1. **5-fold stratified cross-validation** during training
   (`scripts/train_classifier.py`, `cross_val_predict`; `cv_folds`,
   `cv_auc_roc` in `models/deepfake_classifier_v4_meta.json`). This is the
   only out-of-sample figure for GBM v4.
2. **A by-source check and a threshold sweep on 7 April 2026**
   (`models/deepfake_classifier_v4.bias_check.json`,
   `models/deepfake_classifier_v4.calibration.json`). These ran the trained
   model over 5,724 authentic and 4,985 AI-generated images
   (`scripts/calibrate_gbm.py` loads the fitted model and calls
   `predict_proba`). The AI count is identical to the training set's and the
   authentic count is three short of it, so **these figures were almost
   certainly measured on the training images** and are likely optimistic.
   The files do not record which images were used.

There is no held-out test set for GBM v4. None of the 2026 generators has
been measured (`backlog/BL-DET-001-the-ai-detectors-have-never-seen-a-2026-generator.md`).

### 6.2 Third-party evaluation

None.

### 6.3 Results

| Metric | Value | Source | Out of sample? |
|---|---|---|---|
| AUC-ROC | 0.9868 | `cv_auc_roc`, `models/deepfake_classifier_v4_meta.json`; `model-card-gbm.json` | Yes, 5-fold cross-validation |
| False positive rate on authentic images, threshold 0.5 | 4.54% | `overall_fp_rate`, `models/deepfake_classifier_v4.bias_check.json` | Probably not (section 6.1) |
| AI-generated recall, threshold 0.5 | 92.52% | `overall_recall`, same file | Probably not |

The calibration sweep's "recommended" threshold of 0.49 is not used by the
app, which uses 0.25 and 0.58 (section 2), so its figures are not repeated
here.

**False positive rate by kind of authentic image** (threshold 0.5,
`authentic_sources` in `models/deepfake_classifier_v4.bias_check.json`,
probably in-sample):

| Kind | False positive rate | Count |
|---|---|---|
| Wikimedia Commons photographs | 24.80% | 63 of 254 |
| High-end cameras and drones | 10.32% | 13 of 126 |
| Consumer cameras and phones | 8.81% | 26 of 295 |
| Other | 6.12% | 3 of 49 |
| Six public research image dataset sources | from 1.00% to 5.25% | 155 of 5,000 in total |

**Weakest AI-generated sources** (`ai_sources`, same file, probably
in-sample): the diffusion-image research dataset 67.6% recall (338 of 500)
and the community-curated diffusion subset 75.8% (379 of 500). Recall was
not recorded per generator family at training time (`model-card-gbm.json`,
`note`).

### 6.4 Performance limitations

- **Real photographs from good cameras and nature photography** are the
  largest false positive groups (section 6.3). Wikimedia's false positives
  are driven largely by wildlife and insect macro photographs
  (`ui/src/routes/help/model-cards/+page.svelte`, GBM "Known Limitations").
  The app reduces scores for images with camera EXIF or a recognised
  MakerNote (`deepfake.py:1158` onwards), but EXIF can be copied onto an AI
  image, so that adjustment stops at inconclusive rather than authentic.
- **File format and re-encoding.** On one known AI-generated image the
  model scored 0.984 as PNG, 0.226 as JPEG quality 95 and 0.233 as AVIF
  (`docs/calibration/gbm-v5-v11-archive-jun2026.md`, section 2). That is a
  single case, not a rate, but it shows the model can be pushed under its
  thresholds by re-saving. A v5 retrain to fix this regressed on every gate
  (AUC 0.9755, false positives 6.11%, recall 88.88%) and was not shipped
  (same document, section 1).
- **Heavy or repeated JPEG compression** degrades the features the model
  relies on most (section 4).
- **Screenshots and documents**, **build-environment drift** and the
  **217-image golden set**: as for the UnivFD probe
  ([v10onnx.md](v10onnx.md), section 6.4).
- **Video and audio** are not covered.

### 6.5 Fairness

**No fairness evaluation of GBM v4 exists.** There is no evaluation by skin
tone, with or without human annotation, and none by capture device
beyond the coarse camera groups in section 6.3. The April 2026 CLIP-proxy
check ([docs/fairness/bias-test-results.md](../fairness/bias-test-results.md))
was run on the UnivFD probe v9 only, never on this model. Evaluations by
skin tone and by device are planned and not yet done. This card makes no
claim that the model performs equally across groups.

### 6.6 Risks

As for the UnivFD probe ([v10onnx.md](v10onnx.md), section 6.6). In
addition, this model gives its highest false positive rates on careful
photography by capable cameras, which is the kind of image a journalist or
documentation team is most likely to submit.

## 7. Maintenance and monitoring

- **Maintainer.** Jura Labs CIC, feedback@juralabs.org.
- **Change log.** v4 trained 7 April 2026 and shipped from v1.0.0. The
  synthetic threshold was raised from 0.55 to 0.58 on 19 May 2026
  (`deepfake.py`, `THRESHOLD_BASIS`). The v5 candidate of June 2026 was
  archived.
- **Next step.** Measure v4 on 2026 generators, recent phone photographs
  and the golden set before any retrain, and publish the results here
  (`backlog/BL-DET-001-the-ai-detectors-have-never-seen-a-2026-generator.md`).
  A held-out test set for this model is needed so that its false positive
  rate can be stated out of sample.

## 8. Governance

As for the UnivFD probe ([v10onnx.md](v10onnx.md), section 8), including
[GENAI_USE_POLICY.md](../../GENAI_USE_POLICY.md) and the open issues
BL-CLAIM-005 and BL-CLAIM-006.
