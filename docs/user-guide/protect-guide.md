---
title: "Protect Workflow Guide — Jura Trace"
description: "A complete guide to protecting your digital assets with C2PA provenance signing, invisible watermarking, and batch operations in Jura Trace."
last-updated: 25 March 2026
status: published
---

# Protect Workflow Guide

The PROTECT section of Jura Trace gives you tools to embed lasting, verifiable provenance into your digital files. Once protected, a file carries a tamper-evident record of its origins — a provenance chain that travels with it no matter where it is shared, published, or downloaded. This guide explains how each protection method works, when to use it, and what it cannot do.

---

## Table of Contents

1. [C2PA provenance signing](#c2pa-provenance-signing)
2. [Invisible watermarking](#invisible-watermarking)
3. [Batch watermarking](#batch-watermarking)
4. [Asset catalogue and fingerprinting](#asset-catalogue-and-fingerprinting)
5. [Best practices](#best-practices)
6. [Limitations](#limitations)

---

## C2PA provenance signing

### What is C2PA?

C2PA stands for the Coalition for Content Provenance and Authenticity (C2PA), an open industry standard for embedding tamper-evident provenance information directly inside a file. It is the same standard used by Adobe Photoshop, the BBC, and Microsoft to mark the origins of digital content.

When you sign a file with Jura Trace, a C2PA Content Credential is embedded into the file itself — not stored in a separate sidecar file, not held on a server. The credential travels with the file.

### What gets embedded

A C2PA credential records:

- A cryptographic hash of the file's content at the moment of signing. If any pixel, frame, or byte is later changed, this hash will no longer match, and the credential will be reported as invalid.
- The date and time of signing.
- The claim generator — the application and version that created the credential (Jura Trace, version number).
- Any assertions you choose to add: creator name, organisation, copyright notice, or a description.

The credential does **not** contain your name or contact details unless you explicitly enter them. It does not track who has opened or shared the file.

### Supported formats for C2PA signing

| Format | File types | Notes |
|---|---|---|
| Images | JPEG, PNG, WebP, TIFF | Most common archive and web formats |
| Video | MP4 (H.264/H.265), MOV (QuickTime) | Requires FFmpeg for video metadata |
| Audio | WAV, MP3 | Requires FFmpeg for audio metadata |

PDF signing is not yet supported. Document verification is available (see the [Verify workflow guide](./verify-guide.md)), but PDF files cannot currently receive a C2PA credential.

> **Note**: 3D model support (GLB, OBJ) is planned for a future release and is not yet available.

### How to sign a file

1. Open **PROTECT** from the navigation bar.
2. Drag your file into the import area, or click 'Browse'.
3. Review the metadata panel. Jura Trace reads the file's existing EXIF metadata (camera model, creation date, location) and displays it. You can add a creator name, organisation, or description.
4. Click **'Sign with C2PA'**.
5. Choose where to save the protected file. The original is never overwritten.

[Screenshot: PROTECT page showing the metadata panel and Sign with C2PA button]

The signed file will have `-protected` added to its filename by default. When someone later verifies this file with Jura Trace (or any other C2PA-compatible tool), they will see the credential and whether it remains valid.

### What "valid" and "invalid" mean

- **Valid credential** — the file has not been altered since signing. The content matches the signed hash.
- **Invalid credential** — the file has been modified since signing, or the credential has been tampered with. This does not necessarily mean deliberate manipulation — converting a JPEG to PNG, re-saving at a different quality, or stripping metadata can all invalidate a credential.
- **No credential** — the file was never signed, or the credential was removed.

A valid credential proves the file matches its signed version. It does not prove the signed version was authentic — someone could sign an AI-generated image and create a valid credential for it. See [Limitations](#limitations) below.

---

## Invisible watermarking

### How it works

The watermarking system uses a frequency-domain algorithm (DWT-DCT-SVD) to embed an invisible pattern into an image. The pattern encodes a 128-bit unique identifier — a UUID assigned to that specific asset.

The pattern is embedded in the middle frequencies of the image, away from the edges that human vision is most sensitive to. It is invisible to the naked eye and survives common transformations because frequency-domain modifications tend to preserve mid-range information even as they discard high-frequency detail.

[Screenshot: Watermark strength selector showing Low, Medium, High options]

### Strength levels

| Level | Image quality (PSNR) | What it survives | When to use |
|---|---|---|---|
| Low | ~48 dB (minimal impact) | Light JPEG compression, moderate resizing | Archival originals where image fidelity is paramount |
| Medium | ~42 dB | JPEG Q70+, proportional resize, 30% crop | Most publishing and web use cases |
| High | ~36 dB | Heavy JPEG compression, aggressive cropping, format conversion | Assets that will be widely shared or used commercially without your consent |

PSNR (Peak Signal-to-Noise Ratio) measures image fidelity. A higher PSNR means a smaller difference between the original and the watermarked version. At Low strength, the difference is imperceptible in almost all contexts.

### What the watermark contains

The watermark encodes only a UUID — a unique identifier that Jura Trace assigned to the asset when it was added to your catalogue. It does not embed your name, organisation, or any personally identifiable information directly in the watermark pattern. The link between the UUID and your organisation exists only in your local database.

This is intentional: it means the watermark cannot be used to identify you if the file is found somewhere unexpected, but you can confirm ownership by running a verification on a file and matching the extracted UUID against your catalogue.

### How to add a watermark

1. Open **PROTECT** and import a file.
2. In the watermark panel, select a strength level.
3. Click **'Add watermark'**.
4. Save the watermarked file.

You can apply a watermark at the same time as C2PA signing, or separately. Adding both provides the strongest protection: the credential proves the file's content at the time of signing, and the watermark persists even if the credential is stripped.

### Supported formats for watermarking

Watermarking currently supports image files: JPEG, PNG, WebP, TIFF.

> **Note**: Video watermarking is planned for a future release and is not yet available.

---

## Batch watermarking

If you have a large collection of images to protect, you do not need to process them one at a time.

### How to watermark multiple files

1. Open **PROTECT** and go to the 'Batch' section.
2. Select or drag in a folder of images.
3. Choose a watermark strength level. This will be applied to all files in the batch.
4. Click **'Watermark all images'**.

A progress bar shows how many files have been processed. You can cancel the batch at any time — files already processed will retain their watermarks; unprocessed files will remain unchanged.

[Screenshot: Batch watermarking UI showing progress bar and file count]

When the batch completes, a summary shows:
- How many files were successfully watermarked
- How many were skipped (unsupported formats or errors)
- Where the protected files were saved

Batch watermarking uses Medium strength by default. You can change this in the dropdown before starting the batch.

### Batch C2PA signing

> **Note**: Batch C2PA signing is planned for a future release. Currently, C2PA signing must be done one file at a time.

---

## Asset catalogue and fingerprinting

Every file you import into PROTECT is added to your asset catalogue — a local database of your protected assets.

### What the catalogue records

For each asset, Jura Trace stores:

- The file's original path and filename
- The file's format and content type
- Three perceptual fingerprints (aHash, dHash, pHash) — these capture the visual content of an image in a compact form, allowing Jura Trace to recognise if a near-identical copy appears later, even after cropping, colour adjustment, or format conversion
- The watermark UUID, if a watermark was applied
- The watermark strength level
- The date and time the asset was added

### What fingerprinting is for

A perceptual fingerprint is different from a cryptographic hash. A cryptographic hash changes completely if even one pixel is altered. A perceptual fingerprint compares the visual content — so a JPEG and a PNG of the same image will have nearly identical fingerprints, even though their cryptographic hashes are different.

This allows Jura Trace to detect near-duplicate copies of your assets, even when those copies have been modified slightly to avoid detection. If you verify a file later and its fingerprint matches one in your catalogue, Jura Trace will report this in the verification results.

### Accessing the catalogue

The catalogue is available from the PROTECT section. You can search by filename, filter by date, and view the protection history of any asset. You can also export a CSV of your catalogue for record-keeping.

---

## Best practices

### For museums and archives

- Sign new digital acquisitions with C2PA at the point of ingest. This establishes an unbroken provenance chain from the moment the file enters your system.
- Use Medium or High watermark strength for files that will be made available for public download or licensed for external use.
- Keep your local database backed up. The watermark UUID links back to your catalogue — without the catalogue, the UUID cannot be matched to a specific asset.
- Use Archival mode when verifying files that you have received from external sources. It creates a full chain-of-custody log that can be referenced in acquisition records.

### For journalists and documentary makers

- Sign all original source material at the point of receipt. A C2PA credential on your copy of an image or video establishes when and how you received it.
- Watermark any images you intend to publish with attribution — a High-strength watermark will persist through typical web publishing workflows.
- Remember that a C2PA credential proves the file was not altered after signing. It does not prove the content is true or that it was captured in the circumstances claimed.

### For independent creators and photographers

- Apply Low or Medium strength watermarks to work you publish online. The watermark will not be visible to viewers but will allow you to confirm ownership if your work appears without attribution.
- Sign files before sharing with clients or publishers. If a dispute arises about the original content, the C2PA credential provides evidence.

---

## Limitations

Understanding what Jura Trace cannot do is as important as understanding what it can.

**C2PA credentials do not prove the content is authentic.**
A credential proves the file has not been altered since signing. Anyone can sign a file — including an AI-generated image. A valid credential means you are looking at an unmodified copy of what was signed; it does not mean what was signed was real.

**C2PA credentials can be stripped.**
The credential is embedded in the file's metadata. Most image editors, social media platforms, and messaging apps strip metadata when processing or compressing images. A stripped credential is not the same as an invalid one — it simply means the credential is no longer present. The watermark provides a secondary signal that does not depend on metadata.

**Watermarks can be removed with sufficient effort.**
The DWT-DCT-SVD watermark is robust against common transformations but is not indestructible. Extreme manipulation — heavy AI-based upscaling, frame reconstruction, or deliberate adversarial attacks — can degrade or remove the watermark. High strength provides more resilience, but no watermark is permanently unremovable.

**The watermark UUID alone does not identify you.**
The UUID only means something if you have your local catalogue to match it against. If the database is lost, the UUID cannot be traced back to your organisation without your records.

**Batch watermarking applies a uniform strength.**
You cannot set different strength levels for different files in the same batch. Process files separately if you need to vary the strength.

**Video and audio watermarking are not yet available.**
These formats are supported for C2PA signing and forensic verification, but invisible watermarking for video and audio is planned for a future release.
