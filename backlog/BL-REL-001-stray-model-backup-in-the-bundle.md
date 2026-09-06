# BL-REL-001: a stray model backup sits inside the bundled resources glob

**Status**: Open. Found 3 September 2026.
**Raised**: 3 September 2026
**Severity**: Low in consequence, medium in what it says. This is the third
time a stray file in a bundle directory has been found, and the first two
were caught before a release.

## What is wrong

`src-tauri/tauri.conf.json:53-56` bundles resources by glob:

```json
    "resources": [
      "models/*",
      "sidecar-bundle/**/*"
    ],
```

`src-tauri/models/` currently holds:

```
  1,076,604  clip-vit-b32-text.onnx
254,017,536  clip-vit-b32-text.onnx.data
  1,233,927  clip-vit-b32-vision.onnx
351,404,032  clip-vit-b32-vision.onnx.data
    474,489  deepfake_classifier.joblib
    474,441  deepfake_classifier.joblib.bak-1.7.2      <- not a model we load
      4,911  univfd_probe.joblib
```

`deepfake_classifier.joblib.bak-1.7.2`, dated 12 May 2026, matches
`models/*` and is therefore copied into the application bundle. Nothing
loads it. It is a hand-made backup taken during a scikit-learn version bump
and never removed.

macOS releases are built locally from this working tree, per
`docs/release/mac-release-runbook.md`, so any macOS build made from this
checkout since 12 May carries it.

## Why it matters more than 474 KB

**It is a classifier.** Of all the files to ship by accident, an older set
of trained weights for the deepfake detector is a poor choice. It invites
the question of which one the product actually used, and the honest answer
requires reading the code rather than the bundle. For a product whose
subject is provenance, a bundle that contains an unexplained second copy of
the model is a bad artefact to have to explain.

**The glob will do it again.** `models/*` bundles whatever is in the
directory at build time, on a machine where models are copied in by hand
from an external drive during the release run. The failure mode is
structural, not a one-off.

**It has happened before.** The April 2026 technical debt audit recorded a
`.backup` file as a release blocker. The same shape of finding, five months
apart, is a sign the check is a memory rather than a gate.

## What would fix it

1. **Delete the file.** It is not tracked in git and nothing references it.
   If the older weights are worth keeping, they belong in
   `docs/calibration/` alongside the v5 and v11 archive notes, not in a
   directory that is copied wholesale into a shipped application.

2. **Name the resources explicitly** instead of globbing:

   ```json
       "resources": [
         "models/clip-vit-b32-text.onnx",
         "models/clip-vit-b32-text.onnx.data",
         "models/clip-vit-b32-vision.onnx",
         "models/clip-vit-b32-vision.onnx.data",
         "models/deepfake_classifier.joblib",
         "models/univfd_probe.joblib",
         "sidecar-bundle/**/*"
       ],
   ```

   This is more to maintain and that is the point: adding a model becomes a
   deliberate edit, and a stray file cannot ride along.

3. **Add a pre-bundle assertion** to `scripts/build-local-mac.sh` that
   fails if `src-tauri/models/` holds anything not on the expected list.
   The runbook already has a pre-flight section; this belongs in it.

## What not to do

Do not add `*.bak*` to `.gitignore` and consider it handled. The file is
already untracked. Git is not what puts it in the bundle; the glob is.
