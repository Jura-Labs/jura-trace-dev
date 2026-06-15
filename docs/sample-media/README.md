---
title: "Jura Trace Sample Media Pack"
description: "Specification and build notes for the new-user practice pack shipped as a release asset."
last-updated: 15 June 2026
status: draft
---

# Sample Media Pack

A small set of practice files so new users can see what each kind of result looks like before testing their own content. The pack ships as a release asset on the public `Jura-Labs/jura-trace` repository, named `jura-trace-sample-images.zip` (target size under 10 MB).

This directory holds the **source files and the build script**. The distributable ZIP is generated, not committed.

## Contents of the ZIP

| File | What it is | Expected result | What it teaches |
|---|---|---|---|
| `authentic-camera.jpg` | Unmodified JPEG straight from a consumer camera or phone, full EXIF intact. No recognisable faces. | Trust score in the upper band. No editing or AI signals. | What a clean, camera-original file looks like. |
| `ai-generated.jpg` | An openly licensed AI-generated image (CC0 or public domain), generation parameters recorded below. | AI generation flagged by the UnivFD and GBM detectors. | How an AI verdict presents, and that it is a signal not a certainty. |
| `c2pa-signed.jpg` | `authentic-camera.jpg` re-signed by Jura Trace in Local Signing mode during the build. | Valid C2PA provenance row. May show as "untrusted" signer in external tools, which is expected for Local Signing. | A complete credential round-trip. |
| `jpeg-resaved.jpg` | `authentic-camera.jpg` re-saved through three quality reductions (Q75, then Q60, then Q50). | Likely "Uncertain". Compression-analysis signals raised. | That re-saving degrades the signal. "Uncertain" is not "fake". |
| `readme.txt` | Plain-text version of this table. | n/a | Anchors expectations and reduces support load. |

## Sourcing rules

- **No recognisable faces** in any file (UK GDPR and biometric-data caution).
- The authentic photo must be one the team owns outright (shoot it, or an unmodified team photo).
- The AI image must be CC0 or public domain. Record the source URL, model, and prompt in `readme.txt`.
- The C2PA file must be signed by the build script so it always validates against the shipped app version.

## Build

`scripts/build-sample-pack.sh` (to be written) should:

1. Copy the source `authentic-camera.jpg` and `ai-generated.jpg` from this directory.
2. Generate `jpeg-resaved.jpg` from the authentic file via three `convert -quality` passes (ImageMagick).
3. Sign `authentic-camera.jpg` in Local Signing mode to produce `c2pa-signed.jpg` (CLI groundwork lands in v1.0.1; until then sign manually through the Protect tab).
4. Write `readme.txt` from the table above.
5. Zip into `jura-trace-sample-images.zip`.

## Maintenance

- Re-sign `c2pa-signed.jpg` if the signing certificate rotates (note it in `CHANGELOG.md`).
- Re-check expected verdicts whenever the detector lineup or the trust-score weighting changes.
- Version-pin the pack alongside each release tag.

## Status

Source images **not yet added**. This README is the spec. Drop the two source JPEGs into this directory, confirm they produce the expected verdicts, then build the ZIP and attach it to the launch release.
