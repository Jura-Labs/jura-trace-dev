# numpy, scipy, OpenCV and pillow-avif bump: drift rerun before merge

**Date**: 15 October 2026
**Result**: **No user-visible output changed on any of the 84 golden images.** Score, verdict level, triggered signals, heatmap, GBM probability, UnivFD probability and copy-move results are bit-identical between the current lock and the bumped one.
**Consequence**: numpy 2.5.4, scipy 1.18.1, opencv-python-headless 5.0.0.93 and pillow-avif-plugin 1.6.0 are taken in v1.2.0 (C1 step 4) without recalibration. One behaviour change ships with them: the GAN fingerprint detector now runs on AVIF.

## Why it was rerun

`numerical-stack-drift-sep2026.md` already tested these four packages on
8 September. Two things had changed since:

- numpy's candidate moved from 2.5.2 to 2.5.4.
- The September environments were `pip freeze` outputs of
  `requirements-ci.txt` taken that day. Today's lock differs from them in
  transitive pins that matter here, most of all pillow-heif, which is 1.8.0
  in the lock and was 1.6.0 in the September baseline.

So this run compares the lock as it is on `main` with the lock as it will
be after the bump, which is what CI installs before and after the merge.
scikit-learn stays at 1.8.0; the 1.9.0 block in the September note still
stands.

## Method

The committed harness in `scripts/drift/` (README there), unchanged, on the
same 84-image golden set: resolved from the corpus on the Samsung USB, and
`gen_variants.py` confirmed all 84 files byte-identical to
`golden_manifest.json`.

| Environment | Built from | numpy | scipy | opencv-python-headless | pillow-avif-plugin |
|---|---|---|---|---|---|
| `baseline` | `sidecar/requirements.lock` on `main` at `8decebd2` | 2.2.4 | 1.17.1 | 4.11.0.86 | 1.5.5 |
| `candidate` | the regenerated lock in this change | 2.5.4 | 1.18.1 | 5.0.0.93 | 1.6.0 |

Both are Python 3.13.5, built with `uv` from the hash-pinned lock, and
committed as `scripts/drift/envs/baseline.txt` and `candidate.txt`. The
regenerated lock differs from the old one in exactly those four versions.
It was produced with `uv pip compile` because `pip-compile` downloads every
wheel to hash it and failed twice on connection resets; the command is in
the lock's header.

## Results

| Probe | Result |
|---|---|
| 1, decode | Decoded RGB, decoder class and reported format identical, 84 of 84. No decode errors in either environment |
| 2, GBM features | Heuristic score and signals identical, 84 of 84. The 84-value feature vector differs on 44 indices, by at most 2.65e-4 (index 55, a spectral feature, from scipy's FFT), the same size and source as September. Indices 80 to 83 differ by up to 3.4e-6 from OpenCV, and GBM v4 does not read them |
| 3, classifier | Loads in both. `predict_proba` identical on the baseline features and on each environment's own features, 84 of 84, delta 0.0. End to end: score, verdict level, signals, heatmap, classifier score and UnivFD score identical, 84 of 84 |
| 4, UnivFD | CLIP input array, 512-value embedding and probe score identical, 84 of 84, delta 0.0 |
| 5, OpenCV, NPR, GAN | Copy-move score, matches, clone regions and visualisation identical, 84 of 84. NPR score and verdict identical. Raw intermediates differ below the output: SIFT keypoints on 73 images, `bilateralFilter` on 4, NPR raw statistics by at most 2.3e-13. None reaches a detector result |

The sidecar test suite under the candidate environment: 366 passed,
71 skipped.

## The behaviour change: GAN fingerprint on AVIF

As in September: OpenCV 4.11's `cv2.imdecode` cannot read AVIF, and 5.0
can. On the 14 AVIF images the GAN fingerprint detector now produces a
result where it recorded an error before. HEIC still fails `imdecode` in
both. A user comparing an AVIF result across the upgrade sees that detector
row change from unavailable to a result. It is in the CHANGELOG.

## Speed

One candidate run of probe 3 took about 9 minutes where the baseline took
44 seconds, with little CPU time used. It did not reproduce: rerun alone,
it took 33 seconds. Timed directly, `perform_deepfake_detection_with_features`
over all 84 images took 34.5 s in the baseline and 31.0 s in the candidate,
with per-format medians within 0.02 s. The slow run is treated as a stall
on the machine, not a property of the bump.

## One warning to watch

Loading the GBM classifier under numpy 2.5 raises a `DeprecationWarning`
eight times from `joblib/numpy_pickle.py:207` (joblib 1.5.3): "Setting the
shape on a NumPy array has been deprecated in NumPy 2.5". It changes
nothing today. If a later numpy turns it into an error, the classifier
would fail to load, which `_load_classifier` now logs (#38) rather than
hiding. The next joblib or numpy bump should check it.

## What this does not show

84 images, as in September. The end-to-end score nearest a threshold is
0.0049 from 0.30 (`webp_test_set_04`); nearest 0.25 is 0.0064, 0.50 is
0.0060 and 0.70 is 0.0090. The closest is about eighteen times the largest
feature drift. That is a margin, not a proof.
