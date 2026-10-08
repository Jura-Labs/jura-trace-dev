# Golden verdicts for the v1.2.0 size changes (B4 and A2)

**Date**: 5 October 2026
**Result**: **B4 (no CLIP text encoder) changes nothing. A2 (macOS sidecar
built from the pinned manifest) moves the macOS deepfake score on 68 of 217
images and changes one verdict.**

This is the verification run that section 5.2 of
`docs/design/v1.2.0-windows-onedir-and-size.md` asks for.

## The set

217 images: the 213 in the corpus `test_set` folder and the 4 in the sample
pack (`docs/sample-media/jura-trace-sample-images.zip`). Rows are keyed by
the SHA-256 of the image bytes. Corpus file names are not published, so
`input` is the class folder plus the first 12 hex digits of the hash.

## How each run was made

Every run used the `jura-trace-api` and `jura` binaries from the build
under test, `jura-trace-api --ephemeral`, and
`jura verify --mode deep --format ndjson` once per image.
`scripts/golden_verdicts.py extract` produced the rows and
`golden_verdicts.py compare --expect 217` compared them at tolerance 0.

| File | Sidecar and models |
|---|---|
| `v1.1.0-macos-installed.jsonl` | The frozen sidecar and models inside the installed v1.1.0 macOS app (`--sidecar-binary`, `--models-dir`). Python 3.13 from the build machine's miniconda, onnxruntime 1.23.2, Pillow 11.2.1, text encoder present |
| `v1.2.0-b4-a2-macos-built.jsonl` | The signed, notarised app built by `scripts/build-local-mac.sh` from this branch. Python 3.12 from `sidecar/requirements-ci.txt`, onnxruntime 1.24.4, Pillow 12.3.0, precomputed prompt embeddings, no text encoder. DMG SHA-256 `b8b676eafc17eeddb8464247f422cfe61c7a4145c1f7e93cf8ae2a3981c34e95` |

Both report engine 1.1.0, because the version had not been bumped.

## Results

| Comparison | Result |
|---|---|
| Pinned environment from source, text encoder vs precomputed embeddings | Identical on 217 |
| Built app vs pinned environment from source | Identical on 217 |
| Installed v1.1.0 sidecar vs the miniconda Python from source | Identical on 217 |
| Installed v1.1.0 sidecar vs built app | 68 images differ |
| Pinned environment with Pillow 11.2.1 vs built app | Identical on 217 |
| Pinned environment with onnxruntime 1.23.2 vs built app | 3 differences, all 0.0001 on a CLIP score |

**B4.** The first two rows are the proof. CLIP ran with its model on every
image in every run, and `assert_sidecar_ran.py --require-clip` passes on
the built app.

**A2.** Between the v1.1.0 macOS sidecar and the built app:

- `deepfakeScore` differs on 68 images. Median 0.003, largest 0.056, 20
  above 0.01. Of the 20 largest, 11 are authentic screenshots.
- `overallTrust` differs on 8 images.
- One verdict changes: `test_set/authentic/screenshots/b3b1c79c0360.png`,
  deepfake score 0.5593 to 0.6067, deepfake verdict inconclusive to
  synthetic, band uncertain to untrusted.
- The CLIP score differs on one image by 0.0001. The five class
  probabilities are equal everywhere.

## What causes the A2 difference

The build environment as a whole. The miniconda Python run from source
reproduces the v1.1.0 app exactly, and the pinned environment reproduces
the built app exactly. Swapping Pillow or onnxruntime back to the v1.1.0
versions inside the pinned environment does not reproduce the v1.1.0
scores. That leaves Python 3.13 with conda-built numerical libraries
against Python 3.12 with the PyPI wheels. It has not been bisected further.

## Not tested

- That the macOS scores now equal the Windows and Linux ones. All three are
  now frozen from the same manifest, so they should be closer than before,
  but no run here compares platforms.
- The per-platform bit-identical check of the prompt embeddings on Windows
  and Linux. It runs inside `release.yml` and has only been run on macOS
  arm64.
