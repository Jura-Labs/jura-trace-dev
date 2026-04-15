---
title: "Getting Started with Jura Trace"
description: "Download, install, and run your first verification and protection in Jura Trace — the local-first content verification and protection tool from Jura Labs CIC."
last-updated: 25 March 2026
status: published
---

# Getting Started with Jura Trace

Jura Trace is a local-first desktop application for content verification and protection. It helps cultural institutions protect their digital assets from unauthorised AI use, and helps journalists, fact-checkers, and community organisations verify whether media is authentic or AI-generated. All processing happens on your own computer — no files are uploaded, no data leaves your device, and no internet connection is required for core functions.

---

## Table of Contents

1. [What is Jura Trace?](#what-is-jura-trace)
2. [System requirements](#system-requirements)
3. [Installation](#installation)
4. [First launch](#first-launch)
5. [Your first verification](#your-first-verification)
6. [Your first protection](#your-first-protection)
7. [Next steps](#next-steps)
8. [Frequently asked questions](#frequently-asked-questions)

---

## What is Jura Trace?

Jura Trace gives you two tools in one application.

**PROTECT** lets you embed tamper-evident credentials into your images, documents, audio, and video files. You can attach your organisation's identity to a file using C2PA provenance signing (an open industry standard), and add an invisible watermark that survives copying, resizing, and conversion. Once protected, a file carries layers of verification that persist even when it is shared or downloaded by others.

**VERIFY** analyses any file you receive or find online, giving you a trust score and a set of forensic signals that indicate whether the content has been manipulated or AI-generated. It checks for EXIF anomalies (inconsistencies in the file's recorded technical history), image splicing patterns, AI-generation signatures, embedded credentials, and more.

The tagline is "Know What's Real" — that is the bedrock of what Jura Trace is built to help you do.

Jura Trace is developed by **Juralabs Community Interest Company (UK)** and is free for non-commercial use under the **PolyForm Noncommercial 1.0.0** licence.

---

## System requirements

| Platform | Minimum version | Notes |
|---|---|---|
| macOS | 13 (Ventura) | macOS 14 and 15 fully supported |
| Windows | 10 (64-bit) | Windows 11 also supported |
| Linux | Ubuntu 22.04 LTS | Other Debian-based distributions may work |

**Hardware requirements (all platforms)**

| Component | Minimum | Recommended |
|---|---|---|
| CPU | Any 64-bit (x86-64 or ARM64) | Quad-core or better |
| RAM | 4 GB | 8 GB or more |
| Disk space | 500 MB free | 2 GB free (for optional ML models) |
| Display | 1280 × 720 | 1440 × 900 or larger |

**Optional components**

The core features of Jura Trace work without any additional software. The following are optional and unlock additional capabilities:

- **FFmpeg** — required for video and audio file analysis. Without it, image and document verification still works fully.
- **Ollama** — required for AI-powered claim checking and image description. Without it, all other forensic analysis still runs.
- **Internet connection** — only required for downloading optional Ollama models on first use. All analysis is local.

---

## Installation

Platform-specific installation instructions are in the install guides:

- [Installing on macOS](../install-guides/macos-unsigned.md)
- [Installing on Windows](../install-guides/windows-unsigned.md)
- [Installing on Linux](../install-guides/linux-requirements.md)

Each guide covers system requirements, download instructions, and steps to verify the installation is working.

---

## First launch

When you open Jura Trace for the first time, you will see the onboarding screen. This gives a brief overview of what PROTECT and VERIFY do. You can dismiss it at any time by clicking 'Get started'.

### Step 1 — Check service status

Open **SETTINGS** from the navigation bar. Scroll to the 'Services' section. You will see the status of the optional ML analysis service (the forensics engine) and Ollama (the local AI engine).

- A green indicator means the service is running and available.
- A grey indicator means the service is not running. Core features still work — forensic analysis will be skipped.

[Screenshot: Settings page showing service status indicators]

You do not need to start any services to use the basic features. If you want full forensic analysis, start the sidecar service before running a verification. See the install guide for your platform for instructions.

### Step 2 — Choose your database location (optional)

Jura Trace stores your catalogued assets and verification history in a local database. The default location is in your application data folder. If you would prefer a different location — for example, a shared network drive in an institutional setting — you can change it in **SETTINGS** under 'Database'.

Your database never syncs to any cloud service.

---

## Your first verification

Verifying a file tells you whether there are signs it has been manipulated or AI-generated.

### Step 1 — Go to VERIFY

Click **VERIFY** in the navigation bar.

[Screenshot: The VERIFY page with the file drop area visible]

### Step 2 — Drop a file or enter a URL

Drag an image file from your desktop or file manager into the drop area. You can also click 'Browse' to select a file. Supported formats include JPEG, PNG, WebP, TIFF, GIF, BMP, MP4, MOV, MP3, WAV, and PDF.

To verify a web image, paste the URL into the URL field and press Enter. Jura Trace will fetch the file for local analysis — nothing about your query is sent to a third party.

[Screenshot: File dropped onto the VERIFY drop area, ready to analyse]

### Step 3 — Choose an investigation mode

Select a mode from the dropdown:

- **Standard** — runs the core pipeline in a few seconds. Good for quick checks.
- **Deep** — runs all forensic detectors including region-based analysis. Takes 30–60 seconds. Recommended for anything you plan to act on.
- **Archival** — the most thorough analysis. Creates a full chain-of-custody log. Use this when the result will be cited in a report or legal record.

For your first test, choose **Standard**.

### Step 4 — Run the analysis

Click 'Analyse'. Jura Trace works through the file locally. A progress indicator shows which checks are running.

[Screenshot: Analysis in progress — progress indicator visible]

### Step 5 — Read the result

The results page has three main sections:

**Trust score and verdict** — a number from 0 to 100 and one of three verdicts: 'Authentic', 'Inconclusive', or 'Synthetic'. The score reflects how consistent the file's technical signals are with an unmanipulated, genuine source. A higher score means fewer red flags.

**Metadata findings** — a list of flags raised by the metadata analysis. Common flags include missing camera data, GPS coordinates stripped out, or a mismatch between the stated creation date and the file's internal timestamps.

**Forensic signals** — a breakdown of each individual detector's finding. Each signal shows a score and whether it was flagged as suspicious. Green means no issue found; amber or red means the signal warrants attention.

[Screenshot: Results page showing verdict, trust score, and signal breakdown]

A verdict of 'Inconclusive' does not mean the file is fake — it means the evidence is mixed or insufficient to reach a firm conclusion. Always use your own judgement alongside the technical results.

---

## Your first protection

Protecting a file embeds your organisation's identity and a tamper-evident record into the file itself.

### Step 1 — Go to PROTECT

Click **PROTECT** in the navigation bar.

[Screenshot: The PROTECT page with the import area visible]

### Step 2 — Import a file

Drag an image (JPEG, PNG, WebP, TIFF) into the import area, or click 'Browse'. You can also import video files (MP4, MOV) or audio files (WAV, MP3).

### Step 3 — Review the metadata

Jura Trace automatically reads the file's existing metadata — camera model, creation date, location if available. Review what will be embedded. You can add a description or creator name in the fields provided.

[Screenshot: Metadata panel showing extracted EXIF data and editable fields]

### Step 4 — Sign with C2PA provenance

Click 'Sign with C2PA'. This embeds a tamper-evident credential into the file that records:

- The file's content hash at the moment of signing
- The date and time of signing
- The claim generator (Jura Trace, version number)

The credential is unbreakable: if the file is later altered, the credential becomes invalid and Jura Trace will report this when the file is verified.

### Step 5 — Add an invisible watermark (optional)

Click 'Add watermark'. Choose a strength level:

- **Low** — least visible impact on image quality; use for archival originals where quality is paramount.
- **Medium** — recommended for most use cases. Survives typical web compression and resizing.
- **High** — most robust. Suitable for assets that will be heavily shared or converted.

The watermark encodes a unique identifier for your asset. It is invisible to the naked eye and survives JPEG compression, resizing, and 30% cropping.

[Screenshot: Watermark strength selector on the PROTECT page]

### Step 6 — Save the protected file

Click 'Save'. The protected file is saved alongside the original (the original is never overwritten). The asset is added to your catalogue in the PROTECT tab for future reference.

---

## Next steps

Once you are comfortable with the basics, explore the following:

- **[Protect workflow guide](./protect-guide.md)** — batch operations, best practices for institutions, and a full explanation of what C2PA credentials contain.
- **[Verify workflow guide](./verify-guide.md)** — a detailed guide to investigation modes, reading forensic signals, exporting results, and understanding what the verdicts mean.
- **In-app help** — every section of the application has a built-in help page accessible from the '?' icon in the top-right corner.
- **Methodology** — the in-app Methodology page explains every forensic detector in plain language, including what it measures, its limitations, and which investigation modes it runs in.

---

## Frequently asked questions

**Is Jura Trace free?**

Yes, for non-commercial use. Jura Trace is released under the PolyForm Noncommercial 1.0.0 licence. This means it is free for individuals, journalists, cultural institutions, community organisations, and educational use. Commercial use — including use within a for-profit company — requires a Professional or Enterprise licence. See [juralabs.org](https://juralabs.org) for pricing.

**Does Jura Trace need an internet connection?**

No. All analysis runs locally on your device. The only time an internet connection is used is when downloading optional AI models (Ollama) for the first time — and even that is a one-time download. Once models are downloaded, Jura Trace works completely offline.

**What file formats does it support?**

For verification: JPEG, PNG, WebP, TIFF, GIF, BMP (images); MP4, MOV (video); MP3, WAV (audio); PDF (documents).

For C2PA signing and watermarking: JPEG, PNG, WebP, TIFF (images); MP4, MOV (video); MP3, WAV (audio).

Video and audio analysis requires FFmpeg to be installed. See the [install guide](../install-guides/) for your platform.

**How accurate is the AI detection?**

The deepfake and AI-generation detector is an ensemble of two trained classifiers: GBM v4 (trained on 10,709 images, cross-validation AUC-ROC 0.9868, authentic false-positive rate 4.54%) and the UnivFD v8 probe on CLIP embeddings (AUC-ROC 0.9911, recall 96.01%). In plain terms: it correctly identifies most AI-generated images across 14 generator families, and flags authentic images as fake roughly 5% of the time — typically on heavily-compressed phone photos, social media re-uploads, and extreme macro or wildlife shots.

However, no detector is infallible. Detection accuracy depends on image quality, the AI tool used to generate the content, and how much the image has been compressed or edited since generation. Jura Trace gives you evidence — not certainty. Always combine results with your own judgement and, where the stakes are high, use Deep or Archival mode for a fuller picture.

**Does it work with video?**

Yes. Jura Trace can verify MP4 and MOV video files, extracting codec information, resolution, and audio track details. It also runs per-frame deepfake analysis across evenly-spaced frames and reports temporal consistency signals — patterns that reveal whether the video was consistently generated or contains mixed sources. FFmpeg must be installed for video analysis.

**What happens to my files and results?**

Everything stays on your device. Your files are never uploaded. Your verification results and asset catalogue are stored in a local SQLite database on your computer. You can change the database location at any time in Settings. No usage data, no telemetry, and no analytics are collected.
