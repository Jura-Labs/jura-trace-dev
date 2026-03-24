---
title: "Verify Workflow Guide — Jura Trace"
description: "A complete guide to verifying digital content with Jura Trace — investigation modes, trust scores, forensic signals, C2PA credentials, video analysis, and exporting results."
last-updated: 25 March 2026
status: published
---

# Verify Workflow Guide

The VERIFY section of Jura Trace analyses a file and gives you a structured picture of its technical integrity. It runs a layered pipeline of forensic detectors — each one examining a different aspect of the file — and combines the findings into a trust score and a three-way verdict. This guide explains what each part of the analysis means, how to interpret the results, and what the analysis cannot tell you.

---

## Table of Contents

1. [Investigation modes](#investigation-modes)
2. [Trust scores and verdicts](#trust-scores-and-verdicts)
3. [Metadata analysis](#metadata-analysis)
4. [Forensic signals](#forensic-signals)
5. [C2PA Content Credentials](#c2pa-content-credentials-in-verification)
6. [Video and audio verification](#video-and-audio-verification)
7. [Document verification](#document-verification)
8. [Exporting results](#exporting-results)
9. [Investigate further](#investigate-further)
10. [Limitations and honest disclaimers](#limitations-and-honest-disclaimers)

---

## Investigation modes

Every verification starts with choosing an investigation mode. The mode controls which detectors run and how much processing time the analysis takes.

| Mode | Analysis depth | Approximate time | When to use |
|---|---|---|---|
| Standard | Core pipeline only | 3–10 seconds | Quick triage — initial assessment of whether a file warrants closer attention |
| Deep | All detectors, including region-based analysis | 30–60 seconds | When you plan to act on the result — write a story, publish, include in a report |
| Archival | Maximum depth + chain-of-custody log | 60–90 seconds | When the result will be cited in a formal record, legal document, or institutional archive |

**Standard mode** runs the essential checks: EXIF metadata analysis, Error Level Analysis (a technique that reveals compression inconsistencies), noise analysis, copy-move detection, perceptual fingerprinting, and C2PA credential verification.

**Deep mode** adds four region-based forensic detectors that identify composite images — cases where different parts of an image originate from different sources. It also runs the full deepfake classification pipeline, including the trained AI classifier.

**Archival mode** runs everything in Deep mode and additionally creates a full chain-of-custody log in your local database. This log records every step of the analysis with timestamps and a hash chain that confirms the log has not been altered. Use this mode when you need evidence that will withstand scrutiny.

For video files, the mode also determines how many frames are sampled for deepfake analysis: 6 frames (Standard), 20 frames (Deep), or 40 frames (Archival).

---

## Trust scores and verdicts

After analysis, Jura Trace presents a trust score and a verdict.

### Trust score

The trust score is a number from 0 to 100. It represents how consistent the file's technical signals are with an unmanipulated, authentic source.

- A score of 80–100 indicates few or no red flags.
- A score of 50–79 indicates some anomalies that may warrant closer examination.
- A score below 50 indicates significant concerns.

The score is calculated by combining the findings from all detectors that ran, weighted by the investigation mode. Detectors that fire multiple red flags amplify each other — the scoring applies a composite amplification penalty when several regional detectors flag the same file simultaneously.

### Verdicts

Jura Trace returns one of three verdicts:

**Authentic** — the evidence is consistent with a genuine, unmanipulated file. The trust score is typically above 70 and no individual detector has raised a significant flag.

**Inconclusive** — the evidence is mixed. Some detectors found anomalies; others found none. This often occurs with heavily compressed images, files that have been resaved multiple times, or files where the ML analysis service was unavailable. An inconclusive result does not mean the file is fake — it means the available evidence cannot support a firm conclusion.

**Synthetic** — the evidence strongly suggests the file was generated or significantly manipulated by AI. This may be triggered by a high deepfake score, the detection of an AI watermark embedded by a generative model, a valid C2PA credential explicitly stating the file is AI-generated, or several forensic detectors firing simultaneously.

A verdict is the starting point for your judgement, not a substitute for it.

---

## Metadata analysis

The metadata analysis section appears in all investigation modes. It examines the file's EXIF data (a standardised block of technical information embedded in most image files) for internal inconsistencies.

[Screenshot: Metadata analysis section showing flags list and trust contribution]

### Common metadata flags

**Missing camera data** — the file has no record of a camera model or lens. Authentic photographs almost always contain this information. AI-generated images often do not.

**Timestamp inconsistency** — the date embedded in the EXIF data does not match the file's modification date, or internal timestamps are mutually inconsistent.

**GPS data stripped** — location information is absent from an image where it would be expected (e.g. a press photograph where other metadata is present and intact).

**Software field present** — the EXIF data records that the image was processed by specific software (e.g. Adobe Photoshop, GIMP, Stable Diffusion). This is a flag, not a finding — many legitimate photographs pass through editing software.

**Fields populated / fields total** — the proportion of EXIF fields that contain values. A very low proportion can suggest the metadata was created or reconstructed rather than captured by a camera.

Metadata flags contribute to the trust score but are not individually determinative. A photograph that has been professionally edited will often have several flags that are entirely legitimate.

---

## Forensic signals

The forensic signals section shows the output of each individual detector. In Standard mode you will see the core signals; Deep and Archival modes add the regional detectors.

[Screenshot: Forensic signals panel showing a list of detectors with scores and status badges]

### Core detectors

**Error Level Analysis (ELA)** — re-compresses the image at a known quality level and examines the difference between the re-compressed version and the original. Areas that have been copied, pasted, or edited at a different time often show a different error level from the surrounding image. A high ELA score or clearly delineated regions in the ELA heatmap are a signal of potential manipulation.

**Noise Analysis** — examines the distribution of random pixel-level variation (noise) across the image. Authentic camera images have consistent noise patterns. Composited images, where sections have been pasted from different sources, often show abrupt noise changes at the boundaries. The noise heatmap visualises this variation across the image.

**Copy-Move Detection** — looks for regions within the image that have been duplicated and moved to another position. This is a common technique in image manipulation, used to cover up or duplicate elements. Matched regions are highlighted in the visualisation.

**Deepfake and AI Detection** — a two-stage pipeline. First, it runs a set of signal-based heuristics (chromatic aberration patterns, JPEG ghost analysis, neighbouring pixel relationships). Then, a trained GradientBoosting classifier (AUC-ROC 0.945) combines 80 features from these signals to produce a final score. The classifier was trained on a corpus of 326 authentic photographs and 219 AI-generated images.

The deepfake score reflects the probability that the image was generated or significantly manipulated by AI. A score above 0.65 returns a 'Synthetic' classification; below 0.30 returns 'Authentic'; between 0.30 and 0.65 returns 'Inconclusive'.

**Chromatic Aberration** — real camera lenses produce a subtle colour fringing effect at the edges of objects (particularly towards the image corners) caused by different wavelengths of light refracting at slightly different angles. AI-generated images often lack this pattern or show it in an unnaturally uniform way. This detector checks whether the aberration pattern is consistent with a real lens.

**JPEG Ghost Analysis** — analyses the image at multiple compression quality levels and looks for regions that appear to originate from a different compression history. A region that was copied from a JPEG file and pasted into another JPEG file will typically show a characteristic "ghost" at certain quality levels.

**Neighbouring Pixel Relationships (NPR)** — examines the statistical relationships between adjacent pixels. Authentic camera images show characteristic correlation patterns determined by the camera's sensor and processing pipeline. AI-generated images often show subtly different patterns.

### Regional detectors (Deep and Archival modes only)

**Segmented ELA** — divides the image into an 8×8 grid and runs Error Level Analysis on each region independently. This reveals inconsistencies between regions that might not be visible in a whole-image analysis — for example, a sky that was generated separately from the foreground.

**Shadow Consistency** — estimates the direction of the dominant light source in each region of the image and checks whether these directions are consistent across the whole image. Composite images often have inconsistent shadow directions because the different source images were lit differently.

**Colour Temperature** — analyses the colour balance of different regions and checks for consistency. Composited images often have visible colour temperature shifts at region boundaries, even after attempts to colour-correct the composite.

**Splice Boundary Detection** — runs three separate edge-detection signals (JPEG grid alignment, noise boundaries, feathering patterns) to identify the boundaries between regions that may have been composited together.

When two or more regional detectors fire simultaneously, the trust score receives an additional composite amplification penalty, reflecting that multiple independent signals pointing to the same conclusion is a stronger finding than any one signal alone.

---

## C2PA Content Credentials in verification

When Jura Trace analyses a file, it automatically checks for an embedded C2PA Content Credential. The result appears in the credentials section of the results page.

[Screenshot: C2PA credential panel showing status, signing date, and claim generator]

### Credential status

**Valid** — the credential is present and the file's content matches the signed hash. The file has not been altered since signing.

**Invalid** — the credential is present but the content has been altered since signing. This could indicate deliberate manipulation, but it could also mean the file was re-saved, compressed, or had its metadata stripped by a platform.

**Not present** — no C2PA credential was found. This is the most common result — the majority of files in circulation have never been signed.

### AI declaration detection

Some generative AI tools embed a C2PA credential that explicitly declares the content is AI-generated. When Jura Trace finds such a credential, it reports this clearly in the results — and this declaration **lowers the trust score**, even if the credential itself is valid.

This is correct and deliberate behaviour. A valid C2PA credential on an AI-generated image is evidence that the content is synthetic, not evidence that it is authentic. The verdict will reflect this accordingly.

This is one case where a valid credential produces a 'Synthetic' verdict. A valid credential is evidence of what was signed; it is not a guarantee that what was signed was real.

### What a valid credential does and does not prove

A valid credential proves the file you are looking at is an unmodified copy of what was signed with Jura Trace (or another C2PA-compatible tool). It does not prove:
- That the content depicts what it claims to depict
- That the events shown occurred
- That the file was not manipulated before it was signed

---

## Video and audio verification

Jura Trace supports MP4 and MOV video files, and MP3 and WAV audio files. Video and audio analysis requires FFmpeg to be installed on your system.

### Video metadata

For every video file, Jura Trace extracts and displays:
- Codec (e.g. H.264, H.265/HEVC, AV1)
- Resolution and aspect ratio
- Frame rate
- Duration
- Audio track information (codec, sample rate, channels, bitrate)

This information can reveal inconsistencies — for example, a video that claims to be original camera footage but has a codec or bitrate consistent with re-encoding.

### Video deepfake analysis

Jura Trace samples frames from the video at even intervals and runs the full deepfake pipeline on each frame. The number of frames depends on the investigation mode (6 for Standard, 20 for Deep, 40 for Archival).

The frame timeline shows each sampled frame with a colour-coded deepfake score badge:
- Green: score below 0.30 (no significant concern)
- Amber: score 0.30–0.65 (inconclusive)
- Red: score above 0.65 (strong synthetic signal)

[Screenshot: Video frame timeline showing sampled frames with score badges]

Click any frame to expand the accordion and see its individual forensic signals, classifier score, and heatmap.

### Temporal consistency signals

In addition to per-frame scores, Jura Trace calculates three temporal signals by comparing how metrics change across frames:

- **Noise drift** — whether the noise pattern is consistent across frames. Edited or AI-generated videos often show noise that varies more than a real camera recording would.
- **Spectral drift** — whether the frequency characteristics of the image change in ways inconsistent with natural camera variation.
- **LBP drift** — whether the texture patterns across frames are consistent.

The final video deepfake score combines the per-frame results with the temporal signals: 50% weight on the mean per-frame score, 30% on the highest individual frame score, and 20% on the temporal consistency signals.

### Audio analysis

For audio files and the audio tracks in video files, Jura Trace reports codec, sample rate, channel configuration, and bitrate. Audio deepfake detection (voice cloning, AI speech synthesis) is planned for a future release.

### Transcription

If Ollama is running, Jura Trace can transcribe the spoken content of audio and video files using the faster-whisper model (a local speech recognition system). The transcript appears in a panel on the results page. If you have claim-checking enabled, the transcript is automatically fed into the RAG claim checker (see [Forensic signals](#forensic-signals)).

The transcription model downloads automatically on first use (approximately 500 MB). Once downloaded, transcription runs entirely offline.

---

## Document verification

Jura Trace can analyse PDF files, but the analysis is more limited than for images or video.

**What runs on PDF files:**
- C2PA credential check
- Metadata analysis (title, author, creation date, software used)
- Perceptual fingerprinting for duplicate detection

**What does not run on PDF files:**
- Image forensics (ELA, noise, copy-move, deepfake)
- Watermark extraction
- Regional analysis

The trust score for a PDF reflects only the metadata and credential signals. If the PDF contains embedded images, those images are not extracted and analysed individually in the current version.

> **Note**: Deeper document analysis — including embedded image forensics and text authenticity checking — is planned for a future release.

---

## Exporting results

Jura Trace gives you two ways to export your verification results.

### PDF Trust Report

Click **'Export PDF'** on the results page to generate a formatted Trust Report. The report contains:

- The file name, format, and analysis date
- The overall trust score and verdict
- The investigation mode used
- A summary of each detector's findings
- The full metadata flag list
- The C2PA credential status

The Trust Report is designed to be attached to editorial decisions, institutional acquisition records, or case files. It includes a note that the report was generated by Jura Trace and the version number.

[Screenshot: Example PDF Trust Report showing header, verdict, and signal summary]

### ZIP Case Export

Click **'Export case'** to generate a ZIP archive containing:

- The PDF Trust Report
- A JSON file with the full structured results data (all scores, all detector outputs, all metadata)
- A copy of the analysed file (optional — you can deselect this)
- The ELA heatmap, noise heatmap, and copy-move visualisation as separate image files
- For video: the frame thumbnails and timeline data

The ZIP export is designed for analysts who need to share findings with colleagues, retain evidence, or import results into another system. The JSON file contains all the raw data that the PDF summarises.

---

## Investigate further

At the bottom of the results page, Jura Trace offers reverse image search links.

Clicking a reverse image search link opens your default web browser and submits the image to a search engine. This allows you to check whether the image appears elsewhere online — which may reveal the original source, earlier uses, or context that contradicts the claims being made about it.

> **Important — privacy note**: Reverse image search sends the image (or a thumbnail of it) to an external search engine. This is the only action in Jura Trace that transmits data outside your device. It is opt-in: you must click the link to trigger it. If you are working with sensitive material — images from a conflict zone, identifying information, legal evidence — consider whether reverse image search is appropriate before clicking.

Jura Trace does not control what the search engines do with your submission. Check the privacy policies of the search engines you use.

The 'Investigate further' section also links to relevant in-app resources: the Methodology page (which explains each detector in plain language) and the in-app help for the VERIFY section.

---

## Limitations and honest disclaimers

Jura Trace is a forensic tool, not an oracle. Understanding its limitations is essential to using it responsibly.

**A clean result is not proof of authenticity.**
A trust score of 90 and an 'Authentic' verdict means the file shows no technical signs of manipulation. It does not mean the events depicted occurred, that the file was captured when or where it is claimed, or that the original source is trustworthy. Forensic analysis tells you about the technical history of a file — not about the world it depicts.

**AI-generated images are improving rapidly.**
The deepfake classifier was trained on a corpus of images from tools available up to early 2026. New generative models may produce images that show fewer of the technical signatures the classifier looks for. Detection accuracy will be lower for images generated by tools that post-date the training corpus.

**Compression erases evidence.**
Heavy JPEG compression, social media processing, and screen-capture re-encoding all degrade the forensic signals that detectors rely on. A heavily compressed image may return an 'Inconclusive' verdict not because it is authentic, but because the evidence has been eroded. When possible, work from the highest-quality copy available.

**The regional detectors do not run in Standard mode.**
If you need to detect composite images — where different regions of an image come from different sources — you must use Deep or Archival mode. Standard mode is not sufficient for this use case.

**Metadata can be fabricated.**
EXIF data can be written or rewritten by any image editing application. The presence of realistic-looking EXIF data does not confirm a photograph is genuine — it confirms that EXIF data is present.

**C2PA does not solve everything.**
As described above, a valid C2PA credential proves the file has not been altered since signing. It does not prove the content is authentic, and signing an AI-generated image creates a valid credential for that AI-generated image.

**The system is not deterministic across formats.**
Different file formats, codecs, and quality settings produce different baseline characteristics. A finding that is unusual for a camera JPEG might be normal for a WebP exported from a browser. Jura Trace accounts for this where possible, but results should always be interpreted in the context of the file's origin.

**Results should inform judgement, not replace it.**
These tools are designed to support — not supplant — the judgement of the person using them. A trained journalist, archivist, or fact-checker brings contextual knowledge that no automated system can replicate. Use the results as one layer of evidence alongside your own assessment.

---

## Related guides

- [Getting Started with Jura Trace](./getting-started.md) — installation and first steps
- [Protect Workflow Guide](./protect-guide.md) — embedding C2PA credentials and watermarks
- [Architecture overview](../ARCHITECTURE.md) — technical detail on how the pipeline works
- In-app Methodology — available from the '?' menu inside Jura Trace; explains every detector in plain language with its accuracy, limitations, and the investigation modes in which it runs
