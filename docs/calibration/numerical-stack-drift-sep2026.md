# numpy, scipy, scikit-learn, OpenCV and pillow-avif bumps: decode and pipeline drift test

**Date**: 8 September 2026
**Result**: **Four of five pass with no change to any detector output. scikit-learn 1.9.0 fails: the shipped GBM classifier does not load, and the failure is silent.**
**Consequence**: numpy 2.5.2, scipy 1.18.1, opencv-python-headless 5.0.0.93 and pillow-avif-plugin 1.6.0 can be taken without recalibration. scikit-learn 1.9.0 cannot be taken until `deepfake_classifier.joblib` is re-serialised under 1.9.0 and both SHA-256 constants are updated, and that is a model-artefact change that needs its own evidence. OpenCV 5 also adds behaviour, not just numbers: two detectors that previously errored on AVIF now run on it.

## Why this test exists

BL-DEPS-004 established that the sidecar's Python dependencies can change in the release without CI having tested the new version. Five Dependabot pull requests (#1, #4, #5, #9, #10) had been held for exactly that reason: the packages they bump sit directly under the detector arithmetic, and the verdict thresholds were calibrated against the current versions. The Pillow bump of 4 September was taken only after the decode-drift test in `pillow-12-decode-drift-sep2026.md`. This test applies the same method to the remaining five, run together as one candidate environment, and then separately to attribute any difference to a single package.

The decision to run it now rather than defer it to the v1.1.1 window was Paul's, at the terminal, 8 September.

## The specific hazards

Each package touches a different part of the pipeline, so the probes were designed to reach each one.

- **numpy** and **scipy** compute the 84-value feature vector in `sidecar/app/services/deepfake.py` (`np.polyfit` at 1281, 1715 and 1857; `scipy.fft.fft2`, `fftshift` and `dctn`; `scipy.ndimage.laplace`), `np.linalg.lstsq` in `gan_fingerprint.py:67`, and `scipy.stats.kurtosis` and `skew` in `npr.py:40-41`. A change in FFT normalisation, the LAPACK driver, or float accumulation order moves feature values with no change to our code. GBM v4 consumes the first 80 of those values.
- **scikit-learn** unpickles two shipped estimators: the GradientBoostingClassifier in `deepfake_classifier.joblib` and the LogisticRegression UnivFD probe. scikit-learn makes no cross-version pickle guarantee. Both files sit behind SHA-256 gates, so they cannot simply be re-pickled without updating the constants.
- **OpenCV** feeds `copy_move.py` (SIFT, Lowe ratio, `estimateAffinePartial2D` with RANSAC, then DBSCAN), `gan_fingerprint.py` (`cv2.imdecode`), and the noise-residual features in `deepfake.py:1942` and `2045` (`cv2.bilateralFilter`). A major version reimplements exactly these.
- **pillow-avif-plugin** is a decoder, registered in `sidecar/main.py`. AVIF is the format behind the Thai-photo false-negative investigation.

## Method

Corpus: the Samsung USB training corpus at `/Volumes/Samsung USB/Training Data/corpus`, classes `authentic`, `ai_generated`, `test_set` and `protected`.

**Golden set: 84 images**, selected with seed 20260908, minimum side 128 px, stratified across the four classes. Formats as in the Pillow test: 24 PNG, 18 JPEG, 14 AVIF, 14 HEIC, 14 WebP, with the AVIF, HEIC and WebP variants generated from corpus originals under the baseline environment.

**Six environments.** All built from `sidecar/requirements-ci.txt` on Python 3.13.5, Pillow 12.3.0, onnxruntime 1.24.4.

| Environment | numpy | scipy | scikit-learn | opencv-python-headless | pillow-avif-plugin |
|---|---|---|---|---|---|
| baseline | 2.2.4 | 1.17.1 | 1.8.0 | 4.11.0.86 | 1.5.5 |
| candidate (all five) | 2.5.2 | 1.18.1 | 1.9.0 | 5.0.0.93 | 1.6.0 |
| numpy_only | 2.5.2 | 1.17.1 | 1.8.0 | 4.11.0.86 | 1.5.5 |
| scipy_only | 2.2.4 | 1.18.1 | 1.8.0 | 4.11.0.86 | 1.5.5 |
| sklearn_only | 2.2.4 | 1.17.1 | 1.9.0 | 4.11.0.86 | 1.5.5 |
| opencv_only | 2.2.4 | 1.17.1 | 1.8.0 | 5.0.0.93 | 1.5.5 |

The candidate's `pip freeze` differs from the baseline's in exactly the five packages plus one new transitive dependency, `narwhals 2.26.0`, pulled by scikit-learn 1.9.0. Nothing else moved. The single-package environments exist so that a difference in the candidate can be attributed rather than argued about.

The baseline was also run twice, so that any nondeterminism in RANSAC could be separated from version-to-version change. It was not needed: every metric was identical between the two baseline runs, including SIFT keypoints and clone regions. The copy-move path is deterministic on this set.

**Five probes**, run identically in every environment, calling the sidecar's own code paths rather than reimplementing them.

1. **Decode.** SHA-256 of the decoded RGB buffer, decoder class and reported format, with the format plugins registered as `sidecar/main.py` does.
2. **GBM feature vector.** The 84 values from `deepfake.py`'s own extractor, hashed and saved as floats so deltas can be measured, plus the heuristic score and triggered signals.
3. **Classifier.** `deepfake_classifier.joblib` loaded via the sidecar's own `_load_classifier`, with load warnings and exceptions captured verbatim, then `predict_proba` on the baseline feature vectors.
4. **UnivFD.** The CLIP preprocessing array (hashed, as in the Pillow test), the ONNX embedding, and the LogisticRegression probe score.
5. **OpenCV, NPR and GAN.** Copy-move end to end (SIFT keypoints and descriptors, matched pairs, clone regions, score, suspicious flag, visualisation), eleven individual cv2 operations hashed, the NPR detector's raw statistics and outputs, and the GAN fingerprint detector's raw and final outputs.

## Results

### Probe 1, decode: 84 of 84 bit-identical

Decoded pixels, decoder class and reported format all identical. pillow-avif-plugin 1.6.0 handles AVIF exactly as 1.5.5 did. Both environments report PIL's native AVIF support as present and the plugin still takes precedence, as in the Pillow test.

### Probe 2, GBM feature vector: values moved, output did not

| Environment | Max abs delta vs baseline | Rows changed | Feature indices changed |
|---|---|---|---|
| candidate | 2.655e-4 | 84 / 84 | 44 indices |
| **scipy_only** | **2.655e-4** | 84 / 84 | 16 indices: 0 to 8, 55 to 58, 68, 73, 79 |
| numpy_only | 7.3e-12 | 84 / 84 | 39 indices |
| opencv_only | 3.4e-6 | 4 / 84 | 80, 82, 83 only |
| sklearn_only | 0 | 0 / 84 | none |

The whole of the measurable drift is scipy. Indices 0 to 8 and 55 to 58 are the FFT and DCT-derived spectral features. numpy's contribution is float-accumulation noise eleven orders of magnitude smaller. OpenCV's contribution is confined to indices 80, 82 and 83, which are the Sprint 29 Track 3 additions that GBM v4 does not consume (the vector is trimmed to `n_features_in_ = 80` before prediction), and it comes from `cv2.bilateralFilter` producing different output on four images, which is a real implementation change in OpenCV 5 with no effect on any shipped number.

The heuristic score and the set of triggered heuristic signals were identical on all 84 images in every environment.

The NaN pattern (240 cells across indices 76 to 79, the lossless-only demosaicing features, on images where they do not apply) was identical in every environment. The sidecar replaces NaN with 0.0 before prediction, and this test did the same.

### Probe 3, classifier: bit-identical where it loads, and it does not load under 1.9.0

**Output.** With the baseline GradientBoostingClassifier, `predict_proba` on the candidate feature vectors is **bit-identical to `predict_proba` on the baseline vectors on all 84 images**: max abs delta 0.0, zero verdict flips at the sidecar's own thresholds (`AUTHENTIC_THRESHOLD` 0.25, `SYNTHETIC_THRESHOLD` 0.58) and at 0.30, 0.50 and 0.70. The same holds for the scipy_only, numpy_only and opencv_only vectors. A 2.6e-4 shift in the spectral features did not cross a single split in the tree ensemble on this set. The baseline scores span 0.0057 to 0.9968 with median 0.65, so the set is not clustered away from the thresholds.

**Load.** Under scikit-learn 1.9.0, in both the candidate and sklearn_only environments, `joblib.load` on `deepfake_classifier.joblib` raises

```
ModuleNotFoundError: No module named '_loss'
```

The pickle references a private scikit-learn module path that 1.9.0 has moved. The file's bytes are unchanged, so the SHA-256 gate passes and is not involved. The estimator simply cannot be reconstructed.

**And the failure is silent.** `deepfake.py:262-280`, `_load_classifier`, wraps the load in `except Exception: _classifier = None` with no log line. Under 1.9.0 the deepfake detector's classifier branch would be skipped on every image, `classifier_available` would be false, and verdicts would fall back to the heuristic path, with nothing in the log saying so. This is the BL-SILENT-001 pattern in the one place it matters most. The UnivFD LogisticRegression probe, by contrast, loads under 1.9.0 with an `InconsistentVersionWarning` and scores identically; the two estimators do not fail the same way.

### Probe 4, UnivFD: 84 of 84 bit-identical on everything

CLIP input array, ONNX embedding (max abs delta 0.0 across all 512 dimensions), probe score, class probabilities, suspicious flag and verdict level. Identical in every environment.

### Probe 5, OpenCV, NPR and GAN

| Metric | Identical | Note |
|---|---|---|
| Copy-move score, suspicious, clone regions, matched pairs, visualisation | 84 / 84 | |
| SIFT keypoint count | 84 / 84 | same number of keypoints on every image |
| SIFT keypoint positions | differ on 73 images | OpenCV 5 changed SIFT |
| SIFT descriptors | differ on 21 images | OpenCV 5 changed SIFT |
| `cv2.bilateralFilter` | 80 / 84 | four images differ |
| `cv2.resize` INTER_AREA, Sobel, Laplacian, GaussianBlur, medianBlur, pyrDown, cvtColor (three), calcHist | 84 / 84 | |
| NPR score, suspicious, three ratios, heatmap | 84 / 84 | |
| NPR raw kurtosis and skew statistics | 2 / 84 identical, max abs delta 2.3e-13 | float noise from scipy.stats |
| GAN confidence, periodic-artefact flag, peak count, attribution | identical on every image both environments could decode | |

SIFT changed under OpenCV 5 on most images (the same number of keypoints, at different positions, with different descriptors on a quarter of the set), and the copy-move detector's outputs did not change on any. That is the right way round: the detector is insensitive to the descriptor-level differences OpenCV 5 introduced, and the baseline-vs-baseline run confirms the comparison is not noise.

**OpenCV 5 adds AVIF decoding.** `cv2.imdecode` failed on all 14 AVIF and all 14 HEIC images under 4.11 and on only the 14 HEIC under 5.0. The GAN fingerprint detector and the `gan_fingerprint` least-squares path therefore now produce results on AVIF where they previously recorded an error. This is a behaviour change, not drift: AVIF images gain a detector result they did not have. It is probably desirable and it needs to be known, because a user comparing an AVIF verdict before and after the release may see an extra detector row.

## What this licenses, and what it does not

**Licensed, no recalibration.** numpy 2.5.2, scipy 1.18.1, opencv-python-headless 5.0.0.93 and pillow-avif-plugin 1.6.0, taken together or singly. No detector output, verdict, score, flag, heatmap or embedding changed on any of the 84 images. Where feature values moved, they moved by at most 2.7e-4 and produced identical classifier output.

Two qualifications on the scipy result, stated plainly. First, "no split crossed on 84 images" is an observation, not a proof: a gradient-boosted ensemble has discrete thresholds, and an image whose spectral feature sits within 2.7e-4 of one would flip. The 84 images span the score range, three of them sit within 0.01 of the 0.25 authentic threshold and the nearest is 0.0028 from it, and none moved. Second, the drift is in the training features as well as the inference features, so a future GBM retrain under scipy 1.18 will be trained on slightly different values than v4 was; that is normal and is what the calibration gates exist for.

**Not licensed: scikit-learn 1.9.0.** Taking it as-is would ship a product whose deepfake classifier never loads and never says so. Taking it correctly requires re-serialising `deepfake_classifier.joblib` under 1.9.0, updating `_DEEPFAKE_CLASSIFIER_SHA256` in `deepfake.py` and the matching constant for `src-tauri/models/`, checking whether the UnivFD probe should be re-serialised at the same time to clear its version warning, and proving that the re-serialised estimator gives bit-identical `predict_proba` on this golden set. That is a model-artefact change and belongs in its own pull request with its own calibration note. The `.bak-1.7.2` file already in `src-tauri/models/` shows this migration has been done once before.

**Independent of any bump.** `_load_classifier`'s bare `except Exception` should log the exception. Had scikit-learn 1.9.0 shipped, the only evidence would have been a subtle change in verdict distribution. One `logger.exception` line turns that into a startup log entry.

**Not exercised.** TIFF, GIF and BMP, as in the Pillow test. Real device AVIF and HEIC files rather than generated variants. The `test_set` and `protected` classes were sampled, not exhausted. Detectors that were not separable from the request path (ELA, JPEG ghost, EXIF anomaly) were not probed directly, though their inputs (decode, resize, cv2 primitives) were and did not change.

## Reproducing it

Probe scripts, the six environment build logs, `golden_set.json` with the 84 selected paths and generation parameters, both `pip freeze` outputs and the full `comparison.json` are in the session scratchpad at `~/.claude/jobs/de5265ce/tmp/drift/`, not committed, since they depend on a corpus that is not in the repository. Selection uses seed 20260908 against the Samsung USB corpus with minimum side 128 px. The method above is sufficient to rewrite the probes; each one imports the sidecar's own module and calls the same function the request path calls.
