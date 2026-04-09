---
title: "FP Telemetry — Feature Vector Reconstruction-Feasibility Assessment"
decision-id: ASM-2026-04-09-001
date: 9 April 2026
status: DRAFT — awaiting solicitor review
author: ml-data-scientist agent
brief: docs/legal/fp-telemetry-solicitor-brief.md
solicitor-response: docs/legal/fp-telemetry-solicitor-response-2026-04-09.md
design-doc: docs/design/fp-telemetry-endpoint.md
cross-ref: docs/decisions/option-c-corpus-strategy.md
---

# FP Telemetry — Feature Vector Reconstruction-Feasibility Assessment

This assessment responds to the follow-up action at Q2 of
`docs/legal/fp-telemetry-solicitor-response-2026-04-09.md`. The solicitor
disclaimed technical expertise on whether the 84-element feature vector could
be used to reconstruct the original image and requested a written assessment
to inform the Article 4(1) personal data determination.

This document is intended for inclusion in the Phase B review bundle alongside
the DPIA supplement, the privacy notice draft, and the Fly.io DPA cover sheet.

---

## Section 1 — Summary Verdict

**Reconstruction is not feasible for this feature vector under current or
foreseeable machine-learning techniques.**

The Jura Trace deepfake-detection feature vector contains 84 floating-point
numbers (336 bytes in total) derived from statistical properties of an image
— properties such as how noisy the image is, how steeply its brightness
fluctuates across spatial frequencies, and how consistent the texture patterns
are across image regions. These numbers carry no pixel values, no colour layout,
no facial geometry, no background scene, and no text. The forward computation
that produces the 84 numbers from an image discards approximately 99.996% of
the information in the original file: a typical 3-megapixel JPEG input contains
roughly 9 million bytes of pixel data; the feature vector is 336 bytes, a
compression ratio of approximately 27,000:1. Because infinitely many distinct
images produce numerically identical feature vectors (the mapping is
many-to-one by mathematical construction), there is no procedure — gradient
descent, neural network, or otherwise — that could recover which specific image
produced a given vector. An adversary who obtained a feature vector from the
telemetry payload would learn only statistical properties of the image category
(e.g. "this was probably a photograph, not an AI-generated render") — not the
content of any specific image.

---

## Section 2 — The Feature Vector in Concrete Detail

The canonical definition of the 84 features lives in
`sidecar/app/services/deepfake.py`, lines 63–173, in the constant
`FEATURE_NAMES`. The extraction entry point is
`extract_features_for_training()` (lines 184–227). Features are organised into
ten extractor functions. Each function takes the downsampled image (longest
edge capped at 512 px, line 56) and returns a dictionary of scalar floats that
are assembled into the vector.

### 2.1 Frequency domain features (9 values, lines 1028–1100)

Function: `_extract_frequency_features(grey)`.

The greyscale image is transformed into the frequency domain via a 2D Fast
Fourier Transform. The resulting power spectrum is summarised in nine scalars:
the slope of the azimuthally-averaged spectral decay (a single number
describing how quickly power falls off as spatial frequency increases), the
fractions of total energy in the high- and mid-frequency bands, the ratio of
those two fractions, the azimuthal variance in each of four radial bands, and
the spectral entropy of the power spectral density. None of these values
encodes position, colour, or any spatially localised information from the
original image. They exist in the classifier because AI generators
characteristically produce images whose high-frequency energy is suppressed
relative to natural photographs (the "spectral decay" signature).

### 2.2 Noise residual features (9 values, lines 1103–1155)

Function: `_extract_noise_features(img_bgr)`.

A median filter is subtracted from the greyscale image to isolate the noise
residual — the component that is not explained by local averaging. From that
residual the extractor computes: mean absolute magnitude, standard deviation,
kurtosis, skewness, three spatial autocorrelation coefficients (horizontal,
vertical, diagonal, each computed at lag 1), the coefficient of variation of
block-wise noise variance, and a spectral flatness measure. These are
population statistics over the entire image. They exist in the classifier
because camera sensor noise has a characteristic structure (PRNU, see
`noise_autocorr_h1` etc.) that AI generators do not reproduce.

### 2.3 Colour distribution features (22 values, lines 1158–1193)

Function: `_extract_color_features(img_bgr)`.

For each of the three colour channels (blue, green, red) the extractor records:
mean, standard deviation, skewness, kurtosis, and histogram entropy. These are
five global statistics per channel, yielding 15 values. Three inter-channel
Pearson correlation coefficients are added, plus three saturation statistics
(mean, standard deviation, kurtosis in HSV space) and one colour gamut coverage
measure (proportion of 16×16×16 quantised colour space occupied). Total: 22
values. A mean pixel value of 127 tells you nothing about which person, place,
or object was in the image.

### 2.4 Texture features — LBP and GLCM (15 values, lines 1196–1244)

Function: `_extract_texture_features(grey)`.

Local Binary Pattern (LBP) descriptors encode the spatial arrangement of local
intensity contrasts. From the LBP histogram (10 bins for uniform patterns) the
extractor derives: entropy, uniformity, mean, and variance — plus the mean,
standard deviation, and coefficient of variation of LBP variance computed
over 64×64 pixel blocks (7 values total). Grey-Level Co-occurrence Matrix
(GLCM) descriptors are computed at two distances (1 and 3 pixels) and four
angles (0°, 45°, 90°, 135°), yielding matrices at 64 grey levels; from these
the mean and standard deviation of contrast, homogeneity, energy, and
correlation are extracted (8 values). Total: 15. LBP and GLCM features detect
the homogeneous smoothness of AI-generated skin or fabric textures.

### 2.5 JPEG/DCT compression artefact features (5 values, lines 1247–1310)

Function: `_extract_jpeg_features(grey)`.

The greyscale image is divided into non-overlapping 8×8 blocks and the
Type-II Discrete Cosine Transform is applied to each. From the collected AC
coefficients (DC omitted) the extractor computes: a Benford's law divergence
score (how much the distribution of first digits deviates from the expected
natural-number distribution), mean, standard deviation, and kurtosis of the AC
magnitudes, and a blocking artefact strength measure (ratio of pixel
differences at 8-pixel boundaries versus interior differences). These exist in
the classifier because JPEG compression leaves characteristic coefficient
distributions that differ between camera photos, AI renders, and double-
compressed images.

### 2.6 Edge and structural features (9 values, lines 1313–1356)

Function: `_extract_edge_features(grey)`.

Sobel operators in the horizontal and vertical directions produce a gradient
magnitude image and a gradient direction image. The extractor records: mean,
standard deviation, and kurtosis of gradient magnitude; entropy and uniformity
of the gradient direction histogram (36 bins); and Laplacian variance and mean
absolute value. A block-wise sharpness consistency measure (coefficient of
variation of per-block Laplacian variance) completes the 9 values. AI-generated
images tend to have artificially uniform sharpness across regions.

### 2.7 Patch spectral, multi-scale gradient, and noise autocorrelation
features (3 values, lines 1359–1500)

Three separate one-value extractors add:
- `patch_spectral_cv`: coefficient of variation of high-frequency energy across
  64×64 spatial patches (line 1390) — captures spatial uniformity of spectral
  content.
- `multiscale_gradient_ratio`: ratio of mean gradient magnitudes at two spatial
  scales (lines ~1395–1450) — captures scale-space sharpness transitions.
- `noise_autocorr_tau`: exponential decay constant fit to the lag-1 noise
  autocorrelation (lines ~1455–1500) — captures noise correlation length.

### 2.8 Cross-channel noise, VAE grid, chromatic aberration, and saturation
features (6 values, lines 1430–1594)

- `cross_channel_noise_corr_mean` / `_max`: Pearson correlation of per-channel
  noise residuals; camera sensors share correlated read noise, AI generators do
  not.
- `vae_grid_energy_ratio`: energy in FFT bands corresponding to the 8×
  upsampling grid used by VAE decoders in diffusion models (lines 1533–1558).
- `ca_radial_trend`: slope of the radial chromatic aberration profile (lines
  1562–1594); real lenses bend light wavelengths differentially with radius; AI
  generators omit this.
- `sat_lum_extreme_ratio`: fraction of pixels simultaneously saturated in hue
  and extreme in luminance — an indicator of AI over-saturation.

### 2.9 Lossless-format-only features (4 values, lines 1503–1629)

`lsb_randomness` and `lsb_entropy_mean` (lines 1503–1530): camera LSBs are
random due to sensor noise; AI LSBs are structured because they were generated
from a deterministic model.

`demosaic_peak_count` and `demosaic_peak_strength` (lines 1597–1629): presence
and strength of Bayer-pattern spectral peaks in the inter-channel difference
maps. These four features are only populated for lossless formats (PNG, TIFF,
etc.) and default to NaN for JPEG inputs.

### 2.10 Sprint 29 Track 3 camera ISP discriminators (4 values, lines 1644–1800)

`noise_lf_hf_ratio` (lines 1644–1690): power ratio of low- versus
high-frequency components of the bilateral-filter noise residual. Camera ISPs
(iPhone Deep Fusion, Pixel HDR+, DJI) preserve low-frequency noise structure
while suppressing high-frequency read noise; VAE decoders suppress both.

`demosaic_inter_channel_coherence` (lines 1693–1741): mean Pearson correlation
of the three colour channels' FFT magnitude maps in the neighbourhood of the
Bayer demosaicing frequency. Cameras produce a coherent shared artefact; AI
generators do not.

`noise_anisotropy_mean` / `_std` (lines 1744–end): mean and standard deviation
of the log horizontal-to-vertical noise variance ratio across a 4×4 grid of
cells. Camera sensor readout circuits introduce a directional asymmetry that AI
noise lacks.

These four features are appended after the original 80 (lines 159–173) so that
GBM v4 — trained on 80 features — remains loadable. The backwards compatibility
trim at inference time is noted at lines 161–165.

---

## Section 3 — Forward-Pass Information Loss

### Compression ratio

A typical Jura Trace input image is resized to a maximum of 512 pixels on the
longest edge before feature extraction (line 56: `ANALYSIS_SIZE = 512`). At
512×512 pixels with 3 colour channels, the working array contains
786,432 bytes. The feature vector is 84 × 4 bytes = 336 bytes (4 bytes per
IEEE 754 single-precision float). The compression ratio from working array to
feature vector is therefore:

> 786,432 ÷ 336 ≈ **2,340:1**

For a 3-megapixel source JPEG (~9 million bytes of decoded pixel data) the
ratio before the resize step is:

> 9,000,000 ÷ 336 ≈ **27,000:1**

Even the first figure (2,340:1 after downsampling) represents a reduction so
severe that the problem of inverting it is profoundly ill-conditioned.

### Why the mapping is many-to-one by construction

Each extractor function reduces a spatial array of hundreds of thousands of
values to a handful of scalars via non-injective operations: sums, means,
variances, histograms, entropy integrals, and polynomial fits. Consider the
simplest case: `color_b_mean` (line 85) is the arithmetic mean of all blue
channel pixel values across the entire resized image. Infinitely many distinct
images share the same blue-channel mean; knowing the mean tells you nothing
about the spatial arrangement of those blue values.

More formally: the feature extraction function

    f : R^(512×512×3) → R^84

maps a very high-dimensional space to an 84-dimensional space. By the
pigeonhole principle (and because f is continuous), every point in R^84 has
uncountably infinitely many preimages in the input space. The preimage of any
single feature vector is a manifold of dimension at least 512×512×3 − 84 =
786,348 dimensions. Recovering any specific point in that manifold from the
vector alone — without additional information — is not a well-defined
mathematical problem.

---

## Section 4 — Feature Inversion Literature

The academic literature on recovering images from learned representations is
well-established. The key references and their relevance to this specific
feature vector are assessed below.

### 4.1 Mahendran & Vedaldi, 2015 — "Understanding Deep Image Representations
by Inverting Them" (CVPR 2015)

This seminal paper demonstrated that gradient descent in pixel space can
approximately reconstruct images from CNN feature representations (AlexNet,
VGG). The technique minimises the Euclidean distance between the target feature
vector and the feature vector of a candidate image, regularised by natural-
image priors. It produces visually plausible reconstructions when the feature
representation being inverted is a CNN activation map with many thousands of
dimensions (AlexNet's conv5 has 43,264 dimensions; fc6 has 4,096 dimensions).

**Relevance to Jura Trace**: this technique requires a differentiable feature
function. The Jura Trace extractor is not differentiable through its
scipy/scikit-image pipeline in the standard sense, and more critically, its
84-dimensional target is orders of magnitude smaller than the representations
Mahendran & Vedaldi successfully inverted. The AlexNet fc8 representation
(4,096 dims) still allows gradient descent to recover approximate semantics; 84
dimensions does not provide enough constraint to recover anything beyond very
coarse statistical properties.

### 4.2 Dosovitskiy & Brox, 2016 — "Inverting Visual Representations with
Convolutional Networks" (CVPR 2016)

Rather than gradient descent, this paper trained a separate CNN to predict
pixels from feature vectors. The approach produces sharper reconstructions but
requires a large training corpus of (image, feature vector) pairs and still
operates on high-dimensional representations (AlexNet pool5: 9,216 dimensions).

**Relevance to Jura Trace**: a learned inverse network could in principle be
trained on pairs of (image, 84-float vector). However, as discussed in
Section 5 below, what such a network would learn is the class-conditional prior
— the most probable-looking image consistent with those 84 statistics — not any
specific user's image. At 84 dimensions there is insufficient constraint to
recover even semantic category, let alone identity.

### 4.3 Fredrikson et al., 2015 — "Model Inversion Attacks That Exploit
Confidence Information and Basic Countermeasures" (CCS 2015)

This paper introduced the model inversion threat model: an adversary who
receives a machine-learning model's confidence scores for a target class can
reconstruct a prototypical member of that class. The canonical example
recovered a facial image resembling patients in a genomic prediction model by
exploiting the model's output probabilities.

**Relevance to Jura Trace**: Fredrikson et al. reconstruct class prototypes, not
specific individuals, and they do so by querying a model with access to its
output probabilities — not by inverting an 84-float feature vector. The attack
also requires that the feature space encode spatially localised semantic content
(e.g. facial pixels), which the Jura Trace feature vector does not (all features
are global statistical moments). The telemetry payload does not transmit model
output probabilities; it transmits only the feature vector and the user-reported
correct label. Fredrikson's attack model does not apply.

### 4.4 More recent work (2020–2025) on inverting hand-engineered features

The more recent literature on feature inversion — Struppek et al. 2022
("Plug & Play Attacks"), Nguyen et al. 2023 ("Re-examining Model Inversion"),
and the broader generative model inversion literature — focuses almost
exclusively on inverting learned embeddings from deep CNNs, CLIP (512
dimensions), or classifier logit layers. The author is not aware of published
work demonstrating reconstruction from a 84-dimensional hand-engineered
statistical feature vector of the type described here. The theoretical barrier
is simple: there is no known technique that recovers pixel-level content from
statistical moments at this degree of compression.

**Key distinction**: the Mahendran / Dosovitskiy family of attacks work because
CNN activations at intermediate layers are (a) learned to preserve
semantically relevant spatial structure, (b) computed with millions of
parameters specifically tuned to discriminate perceptual content, and (c)
high-dimensional enough (typically ≥512, often ≥4,096) to provide meaningful
constraints. The Jura Trace feature vector fails all three criteria: the
features are hand-crafted population statistics that explicitly discard spatial
structure, they are derived from classical signal-processing operations with no
perceptual learning component, and 84 values is far below the minimum
dimensionality at which any published inversion technique produces intelligible
output.

---

## Section 5 — Attack Model and Feasibility Analysis

This section walks through each plausible attack strategy in turn.

### 5.1 Direct inversion (system of equations)

An adversary could attempt to treat the 84 extraction functions as a system of
84 equations in approximately 786,432 unknowns (after the 512-pixel resize step,
or ~9 million before). A system this severely underdetermined has no unique
solution: the solution set is a manifold of dimension ~786,348. Without
additional constraints (a prior on images) no numerical solver can isolate the
preimage of interest.

**Verdict**: not feasible. The system is irreducibly underdetermined by design.

### 5.2 Gradient-descent reconstruction

An adversary could start from a random noise image and iteratively adjust pixel
values to minimise the mean squared error between the current feature vector and
the target vector. This is the Mahendran & Vedaldi (2015) approach adapted to
the Jura Trace extractor.

Several barriers arise:

- **Non-differentiability**: several extractors use non-differentiable operations
  (sorted histograms at line 1205, the `graycomatrix` integer quantisation at
  line 1234, LBP integer comparisons at line 1203, DCT Benford first-digit
  extraction at line 1272). These require either approximation or subgradient
  methods, both of which produce unstable gradients.

- **Many-to-one collapse**: even if the optimisation converged (which is not
  guaranteed), it would converge to some member of the preimage manifold — an
  image that happens to produce the same 84 statistics — not the specific
  original image. Because the manifold has ~786,348 degrees of freedom, the
  recovered image would look like noise or a generic texture unless extremely
  strong image priors were imposed.

- **Dimensionality of the constraint**: in the Mahendran experiments, AlexNet
  fc6 (4,096 dimensions) produced recognisable reconstructions. Published
  experiments with progressively lower-dimensional targets show rapid degradation
  below ~512 dimensions, and complete failure to recover semantics below ~128
  dimensions. At 84 dimensions, gradient descent constrained to this feature
  space would at best recover an image with similar global colour statistics —
  no faces, no text, no recognisable scene.

**Verdict**: not feasible to recover any image content. The optimisation would
converge to a generic texture with similar noise and colour statistics but no
semantic correspondence to the original.

### 5.3 Learned inverse network

An adversary who possessed the Jura Trace training corpus could train a
conditional generative model (GAN, diffusion model, or regression network) to
predict pixels from 84-float vectors. The trained network would learn to sample
from the posterior p(image | feature vector) — i.e. the distribution of images
that produce that vector.

Two barriers make this attack non-threatening for privacy:

- **Training corpus inaccessibility**: the training corpus resides on an
  external USB drive (referenced in CLAUDE.md) and is not included in the
  telemetry payload or transmitted to the server. Without the corpus, the
  adversary cannot train the inverse network. Even if they obtained it (which
  would be a separate, unrelated security breach), the corpus contains no images
  of specific users or their submitted images — it consists of stock
  photographs and AI-generated training data.

- **Output is a class prior, not a reconstruction**: even with perfect corpus
  access, the learned network's output would be the most probable image for
  that feature vector given the training distribution. For a feature vector from
  a photograph of a specific person's face, the network would output a
  generic-looking face (because many faces share similar noise statistics and
  colour distributions), not the original face. The attack produces no useful
  identification information.

**Verdict**: theoretically possible in a heavily caveatted sense, but produces
no useful private information. The output is a generic image consistent with
the statistical class, not a reconstruction of any specific user's image.

### 5.4 Membership inference

A weaker attack: given a candidate image and the 84-float vector, can the
adversary confirm that the specific image produced that vector?

This is more plausible than reconstruction, but requires:
(a) The adversary already possesses the candidate image.
(b) The adversary can run the feature extractor themselves (the code is not
    currently public, though the extraction algorithm uses only documented
    signal-processing techniques).
(c) The adversary can compare the recomputed vector against the transmitted
    vector.

If conditions (a), (b), and (c) all hold, confirmation is possible. However:

- If the adversary already possesses the candidate image, then the image itself
  is the identifying information — the feature vector adds nothing. The privacy
  risk originates from image possession, not from the feature vector.
- The telemetry payload does not include a hash of the original image (per the
  design doc at `docs/design/fp-telemetry-endpoint.md` Part A.3), so the server
  cannot perform membership inference independently.
- The install UUID transmitted alongside the vector is pseudonymous (not linked
  to identity), as confirmed by the solicitor's Q3 response.

**Verdict**: theoretically possible under restrictive preconditions, but the
risk is from the pre-existing image possession, not from the feature vector
itself. The feature vector does not independently enable identification.

---

## Section 6 — Semantic Content Leakage

The question here is whether the 84-float vector, even if not invertible,
leaks semantically identifiable content — faces, landmarks, text, or
subject identity.

### 6.1 Would a classifier trained on vectors learn to recognise faces?

No. Face recognition requires encoding the spatial arrangement of facial
features — distances between landmarks, texture gradients around eyes and
mouths, frequency content at face-specific spatial scales. The Jura Trace
feature vector encodes none of these properties:

- The colour features (Section 2.3) are global per-channel histograms and
  channel correlations. Two images — one a portrait, one a landscape with
  similar colour balance — are indistinguishable in this subspace.
- The texture features (Section 2.4) capture global LBP distributions and GLCM
  statistics averaged over the entire image. These cannot discriminate between
  a face and any other organic texture with similar micro-contrast properties.
- The frequency features (Section 2.1) encode the global power spectral density.
  Faces, skin, and hair produce a broadly typical 1/f spectral decay, similar
  to foliage and most natural scenes.

Empirically, if one attempted to train a face-recognition classifier on Jura
Trace feature vectors, the classifier would perform at chance (AUC ≈ 0.5) for
identity recognition, because identity is not encoded in global statistical
moments of this type.

### 6.2 Would it learn to recognise specific landmarks or text?

No. Landmark recognition requires spatial encoding of distinctive geometric
structures. Text recognition requires encoding of letter-level patterns. The
feature vector contains no spatial localisation information; the LBP and GLCM
features aggregate over the entire image. An image of the Eiffel Tower and an
image of any other tall, thin vertical structure would produce very similar
feature vectors.

### 6.3 Would it distinguish a photograph of a person from a landscape?

Possibly at a very coarse level. A portrait (predominantly skin tone, smooth
texture, low spectral entropy) would produce different colour mean/std values
and different LBP entropy from a landscape (varied colours, high spatial
frequency, diverse texture). However:

- This level of coarse-category inference does not constitute identification of
  a natural person under GDPR Article 4(1).
- The information leaked (approximately: "this was a photograph with such-and-
  such average colour and texture complexity") is analogous to knowing the file
  size, which is equally uninformative about identity.

### 6.4 Empirical note on the training corpus

The GBM v4 classifier (trained on 10,709 images at AUC-ROC 0.9868) was trained
to distinguish AI-generated images from authentic photographs. Its training
performance confirms that the 84-dimensional vector carries sufficient
information to distinguish those two classes — but the separating hyperplane
in 84-dimensional space corresponds to forensic-signal differences (noise
structure, spectral decay, DCT statistics), not to semantic content such as
faces or places.

---

## Section 7 — Confidence Assessment

**High confidence: reconstruction is not feasible under any known technique,
including plausible near-future advances.**

The reasoning in Sections 3–5 does not depend on empirical observations about
current model performance. It rests on mathematical facts:

1. The feature extraction function is many-to-one: the preimage of any 84-float
   vector is a manifold of ~786,348 dimensions. This is not a contingent
   property of current ML capability; it is a consequence of the feature
   definitions.

2. The published literature on feature inversion does not include a successful
   attack on any representation of 84 dimensions or fewer, and the theoretical
   arguments for why higher-dimensional representations are invertible do not
   extend to this dimensionality regime.

3. The semantic content of the original image is not preserved in any of the
   84 features by design. The features were chosen specifically to capture
   forensic signals (noise physics, compression artefacts, spectral decay) that
   are independent of scene content.

One scenario that would reduce confidence: if a future technique could reliably
invert 84-dimensional hand-engineered statistical features to recover
approximate pixel content, the analysis would need to be revisited. The author
considers this scenario unlikely because such a technique would require
fundamentally new mathematics beyond the existing feature-inversion literature,
and the many-to-one argument is a hard mathematical barrier rather than a
computational one.

---

## Section 8 — Recommendations for Phase B Privacy Notice

The following bullet points are suitable for inclusion (verbatim or lightly
edited) in the user-facing telemetry privacy notice:

- The numbers sent to Jura Labs describe statistical properties of the image
  — how noisy it is, how its colour is distributed, how its spatial frequencies
  are shaped — not its content. It is not possible to reconstruct your image,
  your face, or any identifying features from these numbers.

- The feature data contains no pixel values, no colour layout, no recognisable
  elements of the original image, and no location information from the image
  itself.

- The 84 numbers represent a compression of approximately 27,000-to-1 from the
  original image. This compression is mathematically irreversible: infinitely
  many different images produce the same set of numbers.

- The feature data is sent alongside a randomly generated install identifier
  (not linked to your name, email address, or device identity). You can request
  erasure of all reports from your installation at any time via the Settings
  page.

- If you are concerned about privacy, you do not need to enable the feature
  telemetry option. It is switched off by default and requires your explicit
  consent to activate.

---

## Section 9 — Residual Risks and Caveats

The solicitor should be aware of three scenarios in which this analysis would
need to be revisited.

### 9.1 Combination with other information (membership inference)

As noted in Section 5.4, if an adversary already possesses the original image
and has access to the feature extractor code, they could confirm whether that
specific image produced a given feature vector. This is a membership inference
attack, not a reconstruction attack. Its practical impact is limited because:
(a) the adversary must already possess the image, meaning the image itself is
the information source; (b) the install UUID is pseudonymous and not linked to
identity; (c) the server does not store a hash of the original image. If a
future version of the telemetry payload includes an image hash (for deduplication
purposes), the membership inference risk would increase and this section should
be rewritten.

### 9.2 Growth of the telemetry payload

This assessment covers the payload as specified at
`docs/design/fp-telemetry-endpoint.md` Part A.3. If the payload is extended to
include additional fields — for example, a crop of the suspicious region, a
reduced-resolution thumbnail, or EXIF metadata from the original image — the
analysis must be repeated. EXIF metadata in particular can contain GPS
coordinates, camera serial numbers, and timestamps that are personal data
independently of the feature vector.

### 9.3 Publication of the GBM model weights

The current design does not publish the GBM model weights (`models/
deepfake_classifier.joblib`). If the weights were made publicly available
alongside the feature extraction code, an adversary could more easily train
a learned inverse network (Section 5.3), because the model's internal decision
boundary provides additional statistical signal about which features are most
discriminative. This would not enable reconstruction of specific images, but
it would make the class-prior estimation more accurate. The advice is: do not
publish model weights publicly unless this section is revisited.

---

*Prepared by the ml-data-scientist agent for inclusion in the Phase B legal
review bundle. This assessment is technical in nature and is not legal advice.
The solicitor should confirm the Article 4(1) determination on the basis of
this technical input and their own legal judgement.*
