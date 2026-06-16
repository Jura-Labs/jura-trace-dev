---
title: "Jura Trace Sample Media Pack"
description: "Specification and build notes for the new-user practice pack shipped as a release asset."
last-updated: 16 June 2026
status: built
---

# Sample Media Pack

A small set of practice files so new users can see what each kind of result looks like before testing their own content. The pack ships as a release asset on the public `Jura-Labs/jura-trace` repository, named `jura-trace-sample-images.zip`.

This directory holds the **build script and the canonical `readme.txt`**. The distributable ZIP and its images are generated, not committed (they are gitignored).

## Contents of the ZIP

Verdicts below were confirmed through the real verify pipeline on the v1.0 launch models (GBM v4 + UnivFD v10onnx); they are approximate and move with the model versions.

| File | What it is | Confirmed result | What it teaches |
|---|---|---|---|
| `authentic.jpg` | Real photograph (Google Pixel 8). GPS and serial numbers scrubbed; camera identification kept. | High Trust, about 84%. Camera EXIF present, no manipulation or AI signal. | What a clean, camera-original file looks like. |
| `ai-generated-openai.png` | AI image generated via the OpenAI Media Service API. Carries valid C2PA Content Credentials declaring `digitalSourceType: trainedAlgorithmicMedia`. | Low Trust, about 15%. Deepfake detector reads synthetic; Content Credentials confirm AI origin. | A valid Content Credential proves origin, not authenticity. Here it is valid AND says the image is AI-generated. |
| `resaved-uncertain.jpg` | `authentic.jpg` downscaled to 800px and re-saved with metadata stripped (a messaging-app-style forward). | Uncertain, about 55%. | "Uncertain" is not "fake". Re-saving weakens the signals the checks rely on. |
| `jura-trace-signed.jpg` | `authentic.jpg` signed by Jura Trace in Local Signing mode via the Protect API. | High Trust, about 94%, with Jura Trace Content Credentials. | A complete protect-then-verify round-trip. External tools showing "untrusted" is expected for Local Signing. |
| `readme.txt` | Plain-text version of this table. | n/a | Anchors expectations and reduces support load. |

## Sourcing rules

- The authentic photo must be one the team owns outright. Scrub GPS and serial-number EXIF before shipping; keep the camera make/model so it remains a useful "authentic with metadata" demo.
- The AI image must be cleared for redistribution and its provenance recorded in `readme.txt`. The shipped pack uses an OpenAI-generated image (output owned by the generator under OpenAI's terms); its embedded C2PA credentials are what make it a rich teaching sample. Confirm any AI image verifies as Low Trust before use, web-compressed AI downloads often do not.
- No identifiable private individual should be the subject. Synthetic people in an AI image are not real individuals.

## Build

`scripts/build-sample-pack.sh` builds the ZIP. It needs the two source images, `exiftool`, `python3`+Pillow, and a running app with the local REST API plus an API key (`JURA_API_KEY`). It scrubs the authentic photo, copies the AI image, generates the re-saved variant, signs the Local Signing sample, verifies every file's verdict, writes `readme.txt`, and zips the result. See the header of that script for usage.

Note: a single Q70 re-save is not enough to reach "Uncertain" (the trust model is forensic-led). The shipped re-save (800px, Q92, no EXIF) lands at about 55%. Any re-save trips the noise detector, so this sample shows "Uncertain" but does not satisfy the in-app benign-uncertain note, which is gated on no suspicious detectors.

## Maintenance

- Re-sign `jura-trace-signed.jpg` if the signing certificate rotates (note it in `CHANGELOG.md`).
- Re-check the confirmed verdicts whenever the detector lineup or the trust-score weighting changes.
- Version-pin the pack alongside each release tag.

## Status

Built. `jura-trace-sample-images.zip` is produced by the build script and ready to attach to the launch release. Rebuild it if the source images or the models change.
