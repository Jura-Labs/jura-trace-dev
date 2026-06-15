---
title: "Jura Trace Quick Start"
description: "Verify your first image in about a minute. The shortest path from install to a result for new Jura Trace users."
last-updated: 15 June 2026
status: published
---

# Quick Start

This is the shortest path from a fresh install to your first result. For the full walkthrough, see [Getting Started](getting-started.md).

## 1. Open Jura Trace

Launch the app. On first run it starts a local Analysis Engine in the background. The first verification can take an extra 30 to 90 seconds while the engine warms up. This is expected, not a freeze.

If you see "Analysis Engine offline", see [Troubleshooting](troubleshooting.md). The app still verifies with its built-in checks while the engine starts.

## 2. Get the practice files (optional but recommended)

Download the sample pack (`jura-trace-sample-images.zip`) from the [latest release](https://github.com/Jura-Labs/jura-trace/releases/latest). It contains an authentic photo, a known AI-generated image, and a Jura Trace signed file, so you can see what each result looks like before you test your own content. Each file lists its expected verdict in the included `readme.txt`.

## 3. Verify a file

1. Click the **Verify** tab.
2. Drop an image onto the panel, or click to browse. Supported formats: JPEG, PNG, TIFF, WebP, HEIC, AVIF.
3. Wait for the result.

## 4. Read the result

You get a plain-English headline, a short list of findings, and a recommended action. Jura Trace reports forensic signals, it does not issue a verdict of proof. A score of "Uncertain" often means the file was re-saved, screenshotted, or forwarded through a messaging app, not that it is fake. See [Understanding your trust score](getting-started.md#your-first-verification) for what the number means.

## 5. Protect a file (optional)

Open the **Protect** tab, drop an image, and sign it. This embeds a C2PA content credential that records who signed it and when. Protect accepts JPEG, PNG, TIFF, and WebP. By default this uses Local Signing, which works fully offline. External tools may show the signer as "untrusted". This is expected in Local Signing mode and does not mean anything is wrong with the file. The signature is still cryptographically valid. In the in-app **Help** tab, read "Local and Conformant Signing" for what it means and when it matters.

## Next

- [Getting Started](getting-started.md) for the full guide
- [Troubleshooting](troubleshooting.md) if something will not run
- In-app **Help** tab for the detector reference and methodology
