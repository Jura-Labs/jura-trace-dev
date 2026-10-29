# Model card: UnivFD probe v10onnx (AI-image detector)

Documentation version 1.0, 29 October 2026. Describes the probe shipped in
Jura Trace v1.0.0 to v1.2.0.

This card is structured following NIST's public-facing AI documentation
zero draft (July 2026), NIST AI 300-1 initial public draft, Table 2 "Model
documentation template" and the default model profile in its Annex A.3.
It is not an attestation of conformity to that draft.

Every figure below names the file it comes from. Repository paths are in
this repository. Files under `models/` are not in the public source
snapshot; where a figure is also in the public release asset
`model-card-univfd.json` attached to the
[v1.2.0 release](https://github.com/Jura-Labs/jura-trace/releases/tag/v1.2.0),
that is said. The companion card for the other detection model is
[gbm-v4.md](gbm-v4.md).

## 1. Identifying descriptors

| Field | Value | Source |
|---|---|---|
| Name | UnivFD linear probe v10onnx | `models/univfd_probe_v10onnx_meta.json` (`model_version`); `model-card-univfd.json` |
| Version identifier in results | `univfd-probe-v10onnx` | `sidecar/app/services/clip_detector.py:92` (`_MODEL_VERSION`) |
| Shipped file | `univfd_probe.joblib` | `scripts/build_model_card_assets.py`; `model-card-univfd.json` |
| SHA-256 of the shipped file | `0534a9e80e352a5bd8af5fc447d03e37be2e1aa68a05d81f05736d6ef8956a86` | `models/univfd_probe_v10onnx_meta.json` (`sha256`); `model-card-univfd.json` (`shipped_sha256`); pinned in `sidecar/app/services/clip_detector.py:69` |
| Trained | 11 May 2026 | `models/univfd_probe_v10onnx_meta.json` (`trained_at`) |
| In production since | 11 May 2026 | `docs/calibration/univfd-v10onnx-divergence-fix.md` (Status) |
| Developer | Jura Labs CIC | `THIRD-PARTY-LICENSES.txt` ("First-party artefacts trained by Jura Labs CIC") |
| Point of contact | feedback@juralabs.org | |
| Lineage | UnivFD probe v9 (12 April 2026), then a v10 candidate trained on PyTorch embeddings (11 May 2026, not shipped), then v10onnx | `docs/calibration/univfd-v10onnx-divergence-fix.md` ("Supersedes") |

## 2. Intended use

**Intended task.** Give one score, from 0 to 1, for how closely a still
image resembles the AI-generated images the probe was trained on. The score
is one input to Jura Trace's verification result. It is not shown or used as
a verdict on its own.

**Intended users.** People checking an image before relying on it, such as
journalists, researchers and human-rights documentation teams, who read the
score next to the provenance (C2PA) result and the other forensic checks,
and who make the decision themselves.

**How it is deployed.** Inside the app the probe's score is blended with the
GBM v4 score and a heuristic score: 0.50 probe, 0.30 GBM, 0.20 heuristic
when all three are available (`sidecar/app/services/deepfake.py:1115`).
After further adjustments (a cap for images with camera EXIF at
`deepfake.py:1158`, a MakerNote adjustment after it, and screenshot and
document suppression) the result feeds a trust score with three bands:
Trusted from 0.70, Uncertain from 0.40 up to 0.70, and Untrusted below
0.40 (`src-tauri/src/verify/verdict.rs:23-25`; the API band names are
`trusted`, `uncertain` and `untrusted`). The probe's own verdict levels
are synthetic above 0.60 and authentic below 0.35, with the interval
between them inconclusive (`clip_detector.py:90-91`).

The trust score also takes other detectors that these cards do not
describe. One is JPEG ghost, counted at half weight; that weight was kept
after tests on synthetic splice images only, because no real splice test
set exists (`docs/calibration/s28-jpeg-ghost-weight.md`, "Outcome C").
These cards cover the two classifiers, not the composite score.

**The Uncertain band is the expected outcome for many real images.** It
means the evidence does not support a call either way. It does not mean the
image is fake.

**Out-of-scope uses.**

- Deciding on its own that an image is real or fake, or that a person did
  something. The score is evidence to weigh, not a finding.
- Video, audio or documents. The probe was trained and evaluated on still
  images only. Video deepfake and audio analysis were taken out of v1.0
  (`CHANGELOG.md`, "2 May 2026: JTV-138").
- Images smaller than 128 by 128 pixels, which the pipeline refuses
  (`sidecar/app/services/deepfake.py:827-854`).
- Paintings, illustrations, screenshots, scanned documents and other
  non-photographic content (see section 6.4).
- Images from generators released after the training data was gathered
  (see section 6.4).
- Identifying which generator made an image. Per-family recall is reported
  below for transparency, not as an attribution feature.

## 3. Usage rights

The probe weights are distributed with Jura Trace under AGPL-3.0-or-later
(`THIRD-PARTY-LICENSES.txt`, "GBM deepfake classifier and the UnivFD probe
weights"; `LICENSE`). The CLIP backbone it depends on is a third-party model
under the MIT licence (section 4). The copyright holder's position on using
this source as AI training data is in [TRAINING.md](../../TRAINING.md); it is
a request, not a licence term.

## 4. Design

**Algorithm.** Logistic regression on 512-dimensional image embeddings from
a frozen CLIP ViT-B/32 image encoder, following the UnivFD method (Ojha, Li
and Lee, "Towards Universal Fake Image Detectors that Generalize Across
Generative Models", CVPR 2023). Parameters: C=0.5, L2 penalty,
`class_weight=balanced`, lbfgs solver, 1,000 iterations, seed 42
(`models/univfd_probe_v10onnx_meta.json`, `probe_params`, `random_seed`,
`embedding_dim`).

**Backbone.** CLIP ViT-B/32 with the OpenCLIP checkpoint
`laion2b_s34b_b79k` (`models/univfd_probe_v10onnx_meta.json`, `clip_model`,
`clip_pretrained`), run through ONNX Runtime in the app. Jura Labs did not
train the backbone and does not modify it.

**Why "onnx".** The embeddings the probe was trained on come from the same
PIL preprocessing and ONNX Runtime path the app uses. An earlier probe
trained on PyTorch embeddings was calibrated for slightly different inputs
(mean cosine similarity 0.996, minimum 0.978, on a 20-image test set), which
let some AI images re-encoded from JPEG to PNG pass as authentic
(`docs/calibration/univfd-v10onnx-divergence-fix.md`, "Why a re-train was
needed").

**Explainability.** The probe gives one probability. It does not show which
part of an image drove the score. The app shows other detectors' signals
and heatmaps alongside it; those are not explanations of this score.

## 5. Training

### What is ours and what is not

| Part | Who made it | What is known |
|---|---|---|
| Backbone pre-training data | Not Jura Labs | Per the checkpoint's published model card, the backbone "was trained with the 2 Billion sample English subset of LAION-5B", by Romain Beaumont on the stability.ai cluster, under the MIT licence. The same card says "this large-scale dataset is uncurated" and recommends the dataset for research purposes, not "for creating ready-to-go industrial products" ([laion/CLIP-ViT-B-32-laion2B-s34B-b79K](https://huggingface.co/laion/CLIP-ViT-B-32-laion2B-s34B-b79K), read 29 October 2026). Jura Labs has no further information on that data. Credit and licence: `THIRD-PARTY-LICENSES.txt`, "CLIP ViT-B/32 (ONNX)". |
| Probe training corpus | Jura Labs | Assembled by Jura Labs. Counts below. |
| Probe weights | Jura Labs | Trained 11 May 2026. |

### Probe training corpus

| Figure | Value | Source |
|---|---|---|
| Training images | 50,710 | `models/univfd_probe_v10onnx_meta.json` (`training_corpus.n_train`); `model-card-univfd.json` |
| of which authentic | 27,775 | same, `n_train_authentic` |
| of which AI-generated | 22,935 | same, `n_train_ai` |
| Held-out test images | 5,633 (3,078 authentic, 2,555 AI-generated) | `models/univfd_v10onnx_split.json` (`test_auth`, `test_ai`) |

The corpus has three parts (`models/univfd_probe_v10onnx_meta.json`,
`training_corpus`, `approach`): original images; "platform-forwarded"
copies of them re-saved at JPEG quality 75, quality 85 and a double re-save,
to imitate messaging and social-media forwarding; and multi-format copies
re-encoded as PNG, TIFF, WebP and HEIC.

**Sources.** Authentic images come from real camera photographs (consumer
phones, high-end cameras and drones), Wikimedia Commons photographs
(curated, non-art) and public research image datasets. AI-generated images
come from the generator families listed in section 6.3. Since June 2026 the
names of the external datasets and generators are withheld from public
documentation (commit `773411b5`); `models/generator_family_labels.json`
maps them to the public labels used here.

**Gaps in what we can say about the corpus.** The corpus is not published.
How each part was collected and under what licence or consent is not yet
written up publicly, so this card makes no claim that it was collected with
consent or under licence. The demographic make-up of the corpus has not been
measured with human annotation (section 6.5).

## 6. Evaluation

### 6.1 Method

A 10% split of the corpus, stratified by source folder and augmentation
variant, seed 42 (`models/univfd_probe_v10onnx_meta.json`,
`evaluation.held_out_set`; `models/univfd_v10onnx_split.json`). Figures are
at a decision threshold of 0.5 (`evaluation.threshold`), which is not one of
the thresholds the app uses (section 2).

**The test set is not independent of the training set.** The split was made
by file, not by source image, so a test image can be a re-encoded copy of a
training image. Matching file names after removing the variant suffix
(`_plt75`, `_plt85`, `_plt2x`, `_fmt_png` and so on), 5,553 of the 5,633
test images share a source name with at least one training image; 1,142 of
the 1,211 un-augmented test originals have an augmented copy in training.
Name matching is approximate (two unrelated photos can share a camera file
name), but the scale is clear. The figures below are therefore likely to be
optimistic even for images like the training data. This analysis was made
for this card from `models/univfd_v10onnx_split.json`; it is not in the
training metadata.

The test set is also drawn from the same generators as training. None of the
generators released in 2026 has been measured (`backlog/BL-DET-001-the-ai-detectors-have-never-seen-a-2026-generator.md`).

### 6.2 Third-party evaluation

None of this probe. Ren et al. (arXiv:2602.07814, 8 February 2026) report
that open-source AI-image detectors average 75% at best out of the box and
18 to 30% against Flux Dev, Firefly v4 and Midjourney v7; that study did not
test this probe.

### 6.3 Results on the held-out split

All from `models/univfd_probe_v10onnx_meta.json` (`evaluation`), also in
`model-card-univfd.json`.

| Metric | Value |
|---|---|
| AUC-ROC | 0.9929 |
| False positive rate on authentic images | 3.87% (119 of 3,078) |
| AI-generated recall | 95.77% (2,447 of 2,555) |
| Missed AI-generated images | 108 |
| Accuracy | 95.97% |

**By variant** (`per_variant_auc`, AUC-ROC): original 0.9911 (n=1,211);
JPEG re-save quality 75 0.9912, quality 85 0.9950, double re-save 0.9952
(n=1,074 each); PNG 0.9978, TIFF 0.9945, WebP 0.9926, HEIC 0.9900 (n=300
each).

**By generator family** (recall on AI-generated test images; labels from
`models/generator_family_labels.json`, figures from `per_generator_recall`,
also `model-card-univfd.json` `per_generator_family_recall`):

| Family | Recall | n |
|---|---|---|
| AI imagery with no generator attribution | 96.55% | 1,100 |
| Latent-space artistic benchmark | 92.05% | 390 |
| Closed-source commercial generator A | 100% | 150 |
| Closed-source commercial generator C | 99.33% | 150 |
| Community-curated diffusion SFW subset | 98.00% | 150 |
| Diffusion-image research dataset | 98.00% | 150 |
| Open-weights diffusion model (turbo variant) | 93.33% | 90 |
| Synthetic face generator | 98.89% | 90 |
| Open-weights diffusion model (dev variant) | 91.36% | 81 |
| Unlabelled / generic AI imagery | 81.94% | 72 |
| Art-style benchmark dataset | 95.00% | 60 |
| Closed-source commercial generator B | 100% | 45 |
| Closed-source commercial generator D | 86.67% | 15 |
| User-contributed AI imagery | 100% | 6 |
| Open-weights diffusion model (earlier generation) | 100% | 3 |
| Openly licensed AI imagery from a public media archive | 100% | 3 |

Families with fewer than about 100 test images give rough figures; the
smallest are not meaningful rates.

### 6.4 Performance limitations

- **Generators newer than the training data.** The training families are
  2025 or earlier. Accuracy on 2026 generators is unknown, and published
  2026 studies of this kind of detector report large drops
  (`backlog/BL-DET-001-the-ai-detectors-have-never-seen-a-2026-generator.md`;
  Ren et al., section 6.2).
- **Uncertain results.** Many authentic and AI images land between the
  thresholds. That is by design, and the app says Uncertain rather than
  guessing.
- **AVIF and low-resolution images.** On one known AI-generated AVIF image
  the probe scored about 0.67 across file formats, a thin margin over its
  0.60 synthetic threshold, and 0.628 on a 700 by 438 down-scaled copy
  (`docs/calibration/gbm-v5-v11-archive-jun2026.md`, section 2). That is a
  single case, not a rate. A June 2026 retrain aimed at AVIF and low
  resolution failed its gates and was not shipped (same document).
- **Screenshots and documents.** Screenshots share features with AI images
  (no EXIF, PNG, rendered pixels) (`docs/screenshot-detection-findings.md`).
  The app is meant to switch the AI detectors off for screenshots and
  documents, but that suppression did not run from April 2026 until a fix in
  v1.2.0 (15 October 2026); with it, 22 of the 217 golden-set images moved,
  all authentic screenshots or documents
  (`backlog/BL-SILENT-002-content-type-suppression-has-never-run.md`;
  `docs/calibration/golden-verdicts-oct2026/README.md`).
- **Non-photographic content.** Paintings, digital illustrations and
  stylised artwork can be scored as AI-generated. Wikimedia art and
  illustrations were removed from the training corpus because of this
  (`ui/src/routes/help/model-cards/+page.svelte`, UnivFD "Known
  Limitations").
- **Build environment.** Small numerical differences between builds move
  scores. Between the v1.1.0 and v1.2.0 macOS builds the blended deepfake
  score moved on 68 of 217 golden-set images and one verdict changed; the
  CLIP score itself moved on one image by 0.0001
  (`docs/calibration/golden-verdicts-oct2026/README.md`).
- **Golden set size.** The regression set that checks verdicts between
  builds has 217 images (213 test images and the 4 sample-pack images)
  (`docs/calibration/golden-verdicts-oct2026/README.md`). It detects changes
  between builds. It is too small to estimate accuracy, and it is not an
  accuracy benchmark.
- **Video and audio** are not covered (section 2).

### 6.5 Fairness

**No fairness evaluation of v10onnx exists.** Specifically:

- No evaluation by skin tone with human-annotated ground truth.
- No evaluation by capture device, including the handset families common in
  the Global Majority (`ui/src/routes/help/model-cards/+page.svelte`,
  "Device Coverage").
- The only demographic check was run in April 2026 on the previous probe,
  v9, using CLIP text prompts as a proxy for skin tone and region
  ([docs/fairness/bias-test-results.md](../fairness/bias-test-results.md)).
  It was not re-run on v10onnx, so its figures do not describe this model.
  The same document says the proxy method "is NOT a demographic classifier"
  and that passing it "is not a fairness certificate".

Both evaluations, by skin tone and by device, are planned and not yet done.
Until then this card makes no claim that the probe performs equally across
groups.

### 6.6 Risks

- **Over-reliance.** A low or high score read as proof. Mitigated by the
  three-band result and the wording "Uncertain does not mean fake", not
  removed.
- **False accusation.** A false positive on an authentic photograph,
  especially of a person, can harm the person shown or the person who took
  it. The 3.87% held-out rate is likely to understate the rate in
  use (section 6.1).
- **Evasion.** A generator outside the training families, or deliberate
  post-processing, can lower the score. The detector does not prove an image
  is authentic.
- **Inherited bias.** The backbone was trained on uncurated, English-language
  web data (section 5) and may carry its biases into the probe.

## 7. Maintenance and monitoring

- **Maintainer.** Jura Labs CIC, feedback@juralabs.org.
- **Change log.** v9 (12 April 2026) was replaced by v10onnx on 11 May 2026
  (`docs/calibration/univfd-v10onnx-divergence-fix.md`). A v11 candidate
  (June 2026) failed its regression gates and was archived
  (`docs/calibration/gbm-v5-v11-archive-jun2026.md`).
- **Next step.** Measure v10onnx on 2026 generators and on recent phone
  photographs before any retrain; a new model ships only if it passes the
  gates (`backlog/BL-DET-001-the-ai-detectors-have-never-seen-a-2026-generator.md`).
  Results will be published in this card, including poor ones.
- **Regression gates for v10onnx.** False positive ceiling 5.12%, recall
  floor 94.70%, AUC floor 0.9883, all passed
  (`models/univfd_probe_v10onnx_meta.json`, `regression_gates`).
- **Post-deployment reports.** Users can report a false positive from the
  app; the report is sent by email only if the user chooses to send it.
  No figures from those reports are published yet.

## 8. Governance

- Thresholds are set by the maintainer, and no large language model is in
  the verdict pipeline (README, "On the use of Generative AI in this
  codebase"). How generative AI is used in developing Jura Trace:
  [GENAI_USE_POLICY.md](../../GENAI_USE_POLICY.md).
- Known open issues with the published figures:
  `backlog/BL-CLAIM-005-the-in-app-model-card-shows-the-previous-models-figures.md`
  and
  `backlog/BL-CLAIM-006-the-detection-figures-we-publish-are-out-of-date-and-out-of-context.md`.
- Terms. **False positive rate**: share of authentic images scored as
  AI-generated. **Recall**: share of AI-generated images scored as
  AI-generated. **AUC-ROC**: how well the score ranks AI images above
  authentic ones across all thresholds, from 0.5 (chance) to 1.
