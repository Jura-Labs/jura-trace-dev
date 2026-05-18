# Jura Trace -- Real-World Forensic Detection Test Plan

**Version:** 1.0
**Date:** 28 March 2026
**Author:** Content Authenticity Testing Team
**Status:** Ready for execution

---

## 1. Objectives

This test plan validates Jura Trace's forensic detection pipeline against real-world content -- the actual images and videos that users will submit for verification. The goal is to measure:

1. **False positive rate by source type** -- which categories of authentic content are incorrectly flagged as synthetic.
2. **Detection rate by generator** -- which AI generators are reliably detected and which evade detection.
3. **Per-detector reliability** -- which of the 21 forensic signals are most and least informative for real-world content.
4. **Pipeline robustness** -- how the full verify pipeline (C2PA + EXIF + deepfake + regional + CLIP) behaves on diverse inputs.
5. **Analysis mode consistency** -- whether Standard and Deep modes produce materially different verdicts for the same content.

### Relationship to Existing Work

This plan extends the FP-reduction workstream (corpus expansion + GBM retraining, tracked internally) by evaluating the **entire pipeline end-to-end** -- including the heuristic scorer, classifier blending logic, EXIF-based FP reduction, regional forensics, and trust scoring.

---

## 2. Test Matrix -- Authentic Content

Each category targets a specific real-world scenario where false positives would be harmful.

### 2.1 Phone Photos (Computational Photography)

| ID | Source | Count | Format | Acquisition Method |
|----|--------|-------|--------|-------------------|
| A-01 | iPhone 15/16 (main camera) | 20 | HEIC/JPEG | Take photos on device, AirDrop to Mac. Include daylight, indoor, night mode, portrait mode. |
| A-02 | iPhone (ultrawide + macro) | 10 | HEIC/JPEG | Same device, ultrawide and macro lens. Night mode ON for 5 images. |
| A-03 | Samsung Galaxy S24 | 15 | JPEG/HEIC | Transfer via USB or cloud. Include 200MP mode, standard, and night mode. |
| A-04 | Google Pixel 8/9 | 15 | JPEG | Transfer via Google Photos export (original quality). Include Night Sight, astrophotography mode. |
| A-05 | Older phones (iPhone 11, Pixel 4) | 10 | JPEG | Source from personal archives. Tests older computational photography. |

**Expected challenges:**
- Computational photography (HDR stacking, neural noise reduction) can produce unnaturally clean noise residuals, triggering `noise_var_cv` and `noise_spectral_flatness`.
- HEIC files converted to JPEG may show double-compression artefacts in ELA.
- Night mode long-exposure stacking creates texture smoothness that overlaps with diffusion model signatures.
- Portrait mode (bokeh simulation) creates artificial depth-of-field that may trigger `sharpness_cv`.

**Pass criteria:** Verdict MUST be Authentic or Inconclusive. Trust score >= 0.40. Any Synthetic verdict is a test failure.

### 2.2 DSLR / Mirrorless Camera Photos

| ID | Source | Count | Format | Acquisition Method |
|----|--------|-------|--------|-------------------|
| A-06 | Nikon (Z-series or D-series) | 10 | JPEG (from RAW export) | Lightroom export at quality 85-95. Include landscape, portrait, sport. |
| A-07 | Canon (R-series or EOS) | 10 | JPEG | Camera JPEG (in-body processing) or Lightroom export. |
| A-08 | Sony Alpha | 10 | JPEG | Mixed: some in-body JPEG, some RAW-to-JPEG via Capture One. |
| A-09 | Fujifilm X-series | 5 | JPEG | In-body JPEG with film simulation (Velvia, Acros). Film simulations may trigger colour anomalies. |
| A-10 | Medium format (Hasselblad/Phase One) | 5 | JPEG/TIFF | If available. Very high resolution, clean noise. |

**Expected challenges:**
- Clean EXIF metadata should trigger the `has_camera_exif` FP reduction pathway, capping score at 0.45 when heuristic < 0.6.
- Fujifilm film simulations produce distinctive colour channel statistics that differ from typical camera output.
- In-body noise reduction at high ISO can produce smooth noise fields.

**Pass criteria:** Verdict MUST be Authentic. Trust score >= 0.60. EXIF anomaly score should be very low (high trust). Any Synthetic or Inconclusive verdict is a test failure.

### 2.3 Social Media Downloads

| ID | Source | Count | Format | Acquisition Method |
|----|--------|-------|--------|-------------------|
| A-11 | Instagram (saved posts) | 15 | JPEG | Download via Instagram "Save" or third-party tool. Include photos, graphics, text overlays. |
| A-12 | X/Twitter (saved images) | 15 | JPEG/PNG | Right-click save. Twitter re-encodes at specific quality levels. |
| A-13 | Facebook (saved photos) | 10 | JPEG | Download from Facebook albums. Heavy re-encoding. |
| A-14 | WhatsApp forwarded images | 10 | JPEG | Forward images through WhatsApp; save on receiving device. Extreme compression (Q ~40-60). |
| A-15 | Reddit / Imgur uploads | 10 | JPEG/PNG | Direct save from Reddit image posts. |

**Expected challenges:**
- EXIF metadata is stripped by all platforms -- `has_camera_exif` will be false, losing the FP reduction pathway.
- Instagram and Facebook re-encode at aggressive quality levels (Q ~70-80), creating uniform compression artefacts.
- WhatsApp compression is extreme (Q ~40-60), destroying fine-grained frequency features and potentially triggering JPEG ghost.
- Multiple re-saves create complex compression histories that ELA cannot reliably interpret.
- Twitter serves PNG for images with few colours or transparency, confounding format-based features.

**Pass criteria:** Verdict Authentic or Inconclusive. Trust score >= 0.30. Synthetic verdict is a test failure. Inconclusive is acceptable given metadata loss.

### 2.4 Screenshots

| ID | Source | Count | Format | Acquisition Method |
|----|--------|-------|--------|-------------------|
| A-16 | macOS screenshots | 10 | PNG | Cmd+Shift+4 captures of websites, documents, apps. Include Retina and non-Retina. |
| A-17 | Windows screenshots | 10 | PNG | Win+Shift+S or Print Screen. Include different DPI scales (100%, 150%, 200%). |
| A-18 | iOS screenshots | 10 | PNG | Power+Volume captures. Include light mode, dark mode, web pages. |
| A-19 | Android screenshots | 5 | PNG/JPEG | Power+Volume. Some Android devices save as JPEG. |
| A-20 | Browser screenshots (full-page) | 5 | PNG | Firefox/Chrome full-page screenshot tool. Very tall images. |

**Expected challenges:**
- **This is the highest-risk category for false positives.** Screenshots are PNG (lossless), lack EXIF, have perfectly uniform noise (rendered pixels), and may trigger `lsb_randomness` and `lsb_entropy_mean` -- the same features that caused the original 14% FP rate.
- Text-heavy screenshots have distinctive texture statistics (high `lbp_uniformity`).
- Retina screenshots at 2x are very large and may have unusual frequency spectra.
- Screenshots of photos (screenshot of a website showing a photo) are a compound case.

**Pass criteria:** Verdict Authentic or Inconclusive. Trust score >= 0.25. Synthetic verdict is a critical test failure -- this is the exact scenario that prompted the FP reduction work.

### 2.5 Scanned Documents and Historical Photos

| ID | Source | Count | Format | Acquisition Method |
|----|--------|-------|--------|-------------------|
| A-21 | Flatbed scanner (documents) | 10 | TIFF/PNG/JPEG | Scan at 300 DPI and 600 DPI. Include typed documents, handwritten notes, receipts. |
| A-22 | Flatbed scanner (photographs) | 10 | TIFF/JPEG | Scan printed photographs at 300-600 DPI. Include colour and B&W prints from different decades. |
| A-23 | Phone scan (document) | 10 | JPEG | Use Notes, Adobe Scan, or CamScanner app. Perspective correction applied. |
| A-24 | Book/magazine pages | 5 | JPEG/PNG | Phone photos of printed images. Moire patterns, halftone dots. |
| A-25 | Museum/archive digitisations | 10 | TIFF/JPEG | Source from Wikimedia Commons, Europeana, or Library of Congress digital collections. |

**Expected challenges:**
- Scanner artefacts (CCD noise, dust, newton rings) produce distinctive noise patterns.
- Halftone dots in scanned printed material create strong periodic frequency content that overlaps with GAN spectral signatures.
- Very old photographs (1900s-1960s) have grain patterns unlike any modern camera or AI generator.
- Phone document scans have perspective distortion and variable lighting.

**Pass criteria:** Verdict Authentic or Inconclusive. Trust score >= 0.30. These images are core to the museum/archive use case.

### 2.6 Stock Photography and Web-Distributed Images

| ID | Source | Count | Format | Acquisition Method |
|----|--------|-------|--------|-------------------|
| A-26 | Unsplash (CC0) | 20 | JPEG | Download from unsplash.com. Mix of professional and amateur. Include nature, architecture, food, portraits. |
| A-27 | Pexels (CC0) | 10 | JPEG | Download from pexels.com. Similar to Unsplash but different CDN processing. |
| A-28 | Wikimedia Commons (featured) | 10 | JPEG/PNG/TIFF | Download featured pictures. Diverse subjects, cameras, eras. |
| A-29 | Flickr (CC-licensed) | 10 | JPEG | Download from Flickr. Includes EXIF. Older camera models, diverse processing. |
| A-30 | Getty/AP wire photos | 10 | JPEG | Source from news websites (right-click save). Heavy CDN compression, metadata stripped. |

**Expected challenges:**
- CDN processing strips EXIF and re-encodes at variable quality.
- Some stock photos are heavily post-processed (colour grading, compositing, retouching) and may legitimately trigger manipulation signals.
- Wire photos from AP/Reuters are the exact use case the Guardian corpus represents, so should perform well.

**Pass criteria:** Verdict Authentic or Inconclusive. Trust score >= 0.35.

### 2.7 Edited and Post-Processed Photos

| ID | Source | Count | Format | Acquisition Method |
|----|--------|-------|--------|-------------------|
| A-31 | Photoshop edits (light) | 10 | JPEG/PSD→JPEG | Crop, levels, colour balance, sharpening. Export from PS at quality 80-95. |
| A-32 | Photoshop edits (heavy) | 10 | JPEG | Compositing, cloning, content-aware fill, sky replacement. Should trigger regional detectors. |
| A-33 | Lightroom exports | 10 | JPEG | RAW development with presets. Include HDR merge, panorama stitch. |
| A-34 | VSCO / Snapseed (mobile editing) | 10 | JPEG | Apply filters, adjust curves, add grain. Re-save. |
| A-35 | Canva / design tool output | 5 | PNG/JPEG | Design compositions with text overlays, stickers, templates. |

**Expected challenges:**
- Content-aware fill and sky replacement in Photoshop are **legitimate edits by a human using AI tools**. These should ideally score Inconclusive, not Synthetic, but may trigger regional forensics.
- Heavy compositing may trigger splice boundary detection legitimately.
- Design tool output (Canva) is rendered, not photographed -- similar noise profile to screenshots.

**Pass criteria:** Light edits: Authentic or Inconclusive. Heavy edits (A-32): Inconclusive is acceptable -- these images ARE manipulated, just not AI-generated. Synthetic verdict for light edits (A-31, A-33, A-34) is a test failure.

---

## 3. Test Matrix -- AI-Generated Content

### 3.1 Latest Commercial Generators (High Priority)

| ID | Generator | Count | Format | Acquisition Method | C2PA Expected |
|----|-----------|-------|--------|--------------------|---------------|
| S-01 | GPT-4o (image gen) | 15 | PNG | Generate via ChatGPT. Mix of photorealistic and artistic prompts. | Yes (DALL-E 3 C2PA) |
| S-02 | Midjourney v6.1 | 15 | PNG | Generate via Discord. Use `--style raw` for photorealistic subset. | No |
| S-03 | DALL-E 3 (via API) | 10 | PNG | Generate via OpenAI API. | Yes |
| S-04 | Google Gemini (image gen) | 10 | PNG | Generate via Gemini web interface. | Yes (trainedAlgorithmicMedia) |
| S-05 | Adobe Firefly 3 | 10 | JPEG/PNG | Generate via firefly.adobe.com. | Yes (Content Credentials) |

**Expected challenges:**
- GPT-4o images with embedded C2PA should be immediately flagged via provenance, bypassing forensic analysis. This tests the C2PA pathway.
- Midjourney v6 has no C2PA and very high photorealism -- relies entirely on forensic detection.
- Adobe Firefly embeds Content Credentials, which should declare AI generation.

**Pass criteria:** Verdict MUST be Synthetic or Inconclusive. Trust score <= 0.55. Any Authentic verdict is a critical test failure. Images with C2PA declaring AI generation should score <= 0.30.

### 3.2 Open-Source / Open-Weight Models

| ID | Generator | Count | Format | Acquisition Method |
|----|-----------|-------|--------|--------------------|
| S-06 | Stable Diffusion XL | 15 | PNG | Generate locally via ComfyUI or `diffusers`. Photorealistic prompts. |
| S-07 | Flux 1.1 Pro/Dev | 15 | PNG | Generate via Replicate API or local with `diffusers`. |
| S-08 | Stable Diffusion 3.5 | 10 | PNG | Generate locally. Tests latest Stability AI architecture. |
| S-09 | Playground v3 | 5 | PNG | Generate via playground.com or API. |
| S-10 | Ideogram v2 | 10 | PNG | Generate via ideogram.ai. Include text-in-image prompts. |

**Expected challenges:**
- Flux 1.1 is currently the most photorealistic open model and the hardest to detect with frequency-domain methods alone.
- SDXL at high step counts (50+) with upscaling can be very clean.
- Text-in-image generation (Ideogram, Flux) produces content that may pass text-based authenticity checks.

**Pass criteria:** Verdict Synthetic or Inconclusive. Trust score <= 0.55. Detection rate target: >= 65% flagged as Synthetic across all open-source generators.

### 3.3 Older Generators (Should Be Easy to Detect)

| ID | Generator | Count | Format | Acquisition Method |
|----|-----------|-------|--------|--------------------|
| S-11 | Stable Diffusion 1.5 | 10 | PNG | Generate locally. Baseline quality. |
| S-12 | StyleGAN2/3 (faces) | 10 | PNG | Source from thispersondoesnotexist.com or generate locally. |
| S-13 | DALL-E 2 | 5 | PNG | If API access available, otherwise source from web. |
| S-14 | Midjourney v4/v5 | 5 | PNG | Source from Midjourney community showcase (older versions). |

**Pass criteria:** Verdict Synthetic. Detection rate target: >= 90%. These older generators leave strong artefacts.

### 3.4 Video AI Content

| ID | Generator | Count | Format | Acquisition Method |
|----|-----------|-------|--------|--------------------|
| S-15 | Sora (OpenAI) | 5 | MP4 | Generate via ChatGPT or source from public Sora demos. |
| S-16 | Runway Gen-3 Alpha | 5 | MP4 | Generate via runwayml.com. |
| S-17 | Kling AI | 5 | MP4 | Generate via kling.ai. |
| S-18 | Pika Labs | 5 | MP4 | Generate via pika.art. |

**Expected challenges:**
- Video deepfake pipeline runs per-frame detection with temporal consistency signals (noise drift, spectral drift, LBP drift).
- AI videos may have very consistent temporal signals (low drift), which paradoxically differs from real video where sensor noise varies.
- Compression artefacts in MP4 can mask frame-level AI signals.

**Pass criteria:** Aggregate verdict Synthetic or Inconclusive. At least 50% of sampled frames should score > 0.40.

---

## 4. Test Matrix -- Edge Cases

These test adversarial and ambiguous scenarios that probe the boundaries of the detection pipeline.

| ID | Case | Count | Acquisition | Expected Verdict | Rationale |
|----|------|-------|-------------|-----------------|-----------|
| E-01 | AI photo + Topaz upscaling | 5 | Generate with SDXL, upscale with Topaz Photo AI | Synthetic or Inconclusive | Upscaling adds camera-like noise but does not remove all AI signatures |
| E-02 | AI photo + JPEG Q50 re-save | 5 | Generate with Flux, save as JPEG Q50 | Inconclusive | Heavy compression destroys high-frequency AI signals |
| E-03 | AI photo + 50% crop | 5 | Generate 1024x1024, crop to 512x512 (centre) | Synthetic or Inconclusive | Tests spatial robustness |
| E-04 | AI photo + crop + resize + JPEG | 5 | Generate, crop 30%, resize to 800px, save JPEG Q70 | Inconclusive | Multi-step degradation chain |
| E-05 | Screenshot of AI image | 5 | Generate image, display on screen, take screenshot | Inconclusive | Double format transformation (PNG of rendered PNG) |
| E-06 | Real photo + AI face swap | 5 | Use FaceSwap or similar on authentic video frames | Synthetic or Inconclusive | Tests face-region detection |
| E-07 | Real photo + AI sky replacement | 5 | Use Photoshop neural sky replacement | Inconclusive | Regional detectors should flag sky region |
| E-08 | Real photo + AI object removal | 5 | Use generative fill to remove an object | Inconclusive | Filled region should differ from authentic regions |
| E-09 | AI image with EXIF injection | 5 | Generate image, inject fake camera EXIF via exiftool | Synthetic | EXIF-based FP reduction should NOT override strong forensic evidence |
| E-10 | Authentic photo resaved as PNG | 10 | Take camera JPEG, open in Preview, export as PNG | Authentic | Tests that format conversion alone does not trigger LSB features |
| E-11 | GIF converted to PNG | 5 | Save animated GIF frame as PNG | Inconclusive | Limited colour palette, dithering patterns |
| E-12 | AI image with C2PA stripped | 5 | Generate with Firefly/GPT-4o, strip C2PA metadata | Synthetic or Inconclusive | Must detect via forensics alone when provenance removed |
| E-13 | HDR merge (authentic) | 5 | Merge 3-5 bracketed exposures in Lightroom | Authentic | HDR merging creates unusual noise and tone patterns |
| E-14 | Focus stack (authentic) | 5 | Stack 5-10 images in Helicon Focus | Authentic or Inconclusive | Macro focus stacking is common in museum digitisation |
| E-15 | Panorama stitch (authentic) | 5 | Stitch 5-10 images in Lightroom/PTGui | Authentic or Inconclusive | Stitching creates edge artefacts at seam boundaries |

---

## 5. Scoring Rubric

### 5.1 Verdict Classification

The pipeline produces a three-way verdict (Authentic / Inconclusive / Synthetic) and a trust score (0.0 to 1.0). Each test image is evaluated as:

| Result | Definition |
|--------|-----------|
| **PASS** | Verdict matches expected outcome per the pass criteria for that category |
| **SOFT FAIL** | Verdict is one step away from expected (e.g., Inconclusive when Authentic expected, or Inconclusive when Synthetic expected) |
| **HARD FAIL** | Verdict is opposite to expected (e.g., Synthetic when Authentic expected, or Authentic when Synthetic expected) |

### 5.2 Category-Level Thresholds

| Category | Hard Fail Threshold | Target Pass Rate | Acceptable Soft Fail Rate |
|----------|-------------------|-----------------|--------------------------|
| Phone photos (A-01 to A-05) | Any Synthetic verdict | >= 80% Authentic | <= 20% Inconclusive |
| DSLR photos (A-06 to A-10) | Any Synthetic/Inconclusive | >= 90% Authentic | <= 10% Inconclusive |
| Social media (A-11 to A-15) | Any Synthetic | >= 60% Authentic | <= 40% Inconclusive |
| Screenshots (A-16 to A-20) | Any Synthetic | >= 50% Authentic | <= 50% Inconclusive |
| Scanned content (A-21 to A-25) | Any Synthetic | >= 60% Authentic | <= 40% Inconclusive |
| Stock photos (A-26 to A-30) | Any Synthetic | >= 70% Authentic | <= 30% Inconclusive |
| Light edits (A-31, A-33, A-34) | Any Synthetic | >= 60% Authentic | <= 40% Inconclusive |
| Heavy edits (A-32, A-35) | N/A | N/A | Inconclusive acceptable |
| Latest AI (S-01 to S-05) | Any Authentic | >= 65% Synthetic | <= 35% Inconclusive |
| Open-source AI (S-06 to S-10) | Any Authentic | >= 65% Synthetic | <= 35% Inconclusive |
| Older AI (S-11 to S-14) | Any Authentic | >= 90% Synthetic | <= 10% Inconclusive |
| Video AI (S-15 to S-18) | Any Authentic | >= 50% Synthetic | <= 50% Inconclusive |
| Edge cases (E-01 to E-15) | Per-case criteria above | N/A | Inconclusive generally acceptable |

### 5.3 Per-Detector Signal Evaluation

For each test image, record whether each of the 21 heuristic signals triggered (true/false). This builds a **signal reliability matrix** showing, for each detector, its true positive rate and false positive rate across content categories.

The 21 heuristic signals in the deepfake pipeline:

| # | Signal | Feature Category | Key Features |
|---|--------|-----------------|--------------|
| 1 | Spectral decay | Frequency | `spectral_decay_beta` |
| 2 | High-frequency energy | Frequency | `hf_energy_ratio` |
| 3 | Azimuthal variance | Frequency | `az_var_band_*` |
| 4 | Spectral entropy | Frequency | `spectral_entropy` |
| 5 | Noise residual | Noise | `noise_mean_abs`, `noise_std`, `noise_kurtosis` |
| 6 | Noise structure | Noise | `noise_autocorr_*`, `noise_var_cv` |
| 7 | Colour distribution | Colour | `color_*_mean/std/skew/kurt/entropy` |
| 8 | Colour correlation | Colour | `color_corr_rg/rb/gb` |
| 9 | Saturation | Colour | `sat_mean`, `sat_std`, `sat_kurtosis` |
| 10 | LBP texture | Texture | `lbp_entropy`, `lbp_uniformity`, `lbp_block_var_*` |
| 11 | GLCM texture | Texture | `glcm_contrast/homogeneity/energy/correlation` |
| 12 | DCT / Benford | JPEG | `dct_benford_div`, `blocking_strength` |
| 13 | Edge structure | Edge | `edge_mag_*`, `edge_dir_*`, `laplacian_var` |
| 14 | Sharpness | Edge | `sharpness_cv` |
| 15 | Patch spectral | Multi-scale | `patch_spectral_cv` |
| 16 | Multiscale gradient | Multi-scale | `multiscale_gradient_ratio` |
| 17 | Noise autocorrelation | Noise (new) | `noise_autocorr_tau` |
| 18 | Cross-channel noise | Noise (new) | `cross_channel_noise_corr_*` |
| 19 | VAE grid | Artefact | `vae_grid_energy_ratio` |
| 20 | CA absence | Artefact | `ca_radial_trend` |
| 21 | Saturation/luminance | Colour (new) | `sat_lum_extreme_ratio` |

Plus conditional lossless-only signals: `lsb_randomness`, `lsb_entropy_mean`, `demosaic_peak_count`, `demosaic_peak_strength`.

Plus the regional forensic detectors (separate from deepfake pipeline):
- Segmented ELA (8x8 grid)
- Shadow consistency
- Colour temperature
- Splice boundary

---

## 6. Acquisition Toolkit

### 6.1 Authentic Content Acquisition

| Tool | Purpose | Command / URL |
|------|---------|---------------|
| **Personal devices** | Phone photos (A-01 to A-05) | AirDrop, USB transfer, Google Photos export |
| **Unsplash API** | Stock photos (A-26) | `curl "https://api.unsplash.com/photos/random?count=20" -H "Authorization: Client-ID $UNSPLASH_KEY"` |
| **Pexels API** | Stock photos (A-27) | `curl "https://api.pexels.com/v1/curated?per_page=10" -H "Authorization: $PEXELS_KEY"` |
| **COCO 2017 val** | Diverse photos | `scripts/expand_corpus_v2.py --sources coco` |
| **Wikimedia API** | Featured images (A-28) | `scripts/expand_corpus_v2.py --sources wikimedia` |
| **exiftool** | Verify EXIF presence | `exiftool -Make -Model -ExposureTime -FNumber image.jpg` |
| **ImageMagick** | Format conversion for E-10 | `convert input.jpg output.png` |
| **Screencapture** | macOS screenshots (A-16) | `screencapture -x screenshot.png` |

### 6.2 AI Content Acquisition

| Tool | Purpose | Notes |
|------|---------|-------|
| **ChatGPT** | GPT-4o images (S-01) | Use "generate an image of..." prompt. Download PNG. |
| **Midjourney Discord** | Midjourney v6 (S-02) | Use `/imagine` command. Upscale to U1-U4. Save full-size PNG. |
| **OpenAI API** | DALL-E 3 (S-03) | `openai.images.generate(model="dall-e-3", ...)` |
| **Gemini** | Google images (S-04) | gemini.google.com, image generation feature |
| **firefly.adobe.com** | Adobe Firefly (S-05) | Generate and download with Content Credentials |
| **ComfyUI** | SDXL, SD 3.5 (S-06, S-08) | Local generation. Save workflow JSON for reproducibility. |
| **Replicate API** | Flux 1.1 (S-07) | `replicate.run("black-forest-labs/flux-1.1-pro", ...)` |
| **ideogram.ai** | Ideogram v2 (S-10) | Web interface or API |
| **thispersondoesnotexist.com** | StyleGAN faces (S-12) | Direct download. Note: may be cached/older model. |
| **Runway** | Gen-3 Alpha video (S-16) | runwayml.com, generate 4-second clips |

### 6.3 Edge Case Preparation

| Tool | Purpose |
|------|---------|
| **exiftool** | EXIF injection (E-09): `exiftool -Make="Canon" -Model="EOS R5" -ExposureTime="1/250" ai_image.png` |
| **c2patool** | C2PA stripping (E-12): `c2patool strip input.png -o output.png` or hex-edit JUMBF box |
| **Topaz Photo AI** | AI upscaling (E-01): Process generated image through Topaz denoise + sharpen |
| **ImageMagick** | JPEG re-save (E-02): `convert input.png -quality 50 output.jpg` |
| **ImageMagick** | Crop (E-03): `convert input.png -gravity center -crop 50%x50%+0+0 output.png` |
| **Photoshop** | Sky replacement (E-07), object removal (E-08), face compositing |
| **FaceSwap / DeepFaceLab** | Face swap (E-06) |

---

## 7. Test Execution Automation

### 7.1 Directory Layout

```
test-corpus/
├── authentic/
│   ├── phone-iphone/         # A-01, A-02
│   ├── phone-samsung/        # A-03
│   ├── phone-pixel/          # A-04
│   ├── phone-older/          # A-05
│   ├── dslr-nikon/           # A-06
│   ├── dslr-canon/           # A-07
│   ├── dslr-sony/            # A-08
│   ├── dslr-fuji/            # A-09
│   ├── dslr-medium-format/   # A-10
│   ├── social-instagram/     # A-11
│   ├── social-twitter/       # A-12
│   ├── social-facebook/      # A-13
│   ├── social-whatsapp/      # A-14
│   ├── social-reddit/        # A-15
│   ├── screenshot-macos/     # A-16
│   ├── screenshot-windows/   # A-17
│   ├── screenshot-ios/       # A-18
│   ├── screenshot-android/   # A-19
│   ├── screenshot-browser/   # A-20
│   ├── scan-document/        # A-21
│   ├── scan-photo/           # A-22
│   ├── scan-phone/           # A-23
│   ├── scan-printed/         # A-24
│   ├── scan-archive/         # A-25
│   ├── stock-unsplash/       # A-26
│   ├── stock-pexels/         # A-27
│   ├── stock-wikimedia/      # A-28
│   ├── stock-flickr/         # A-29
│   ├── stock-wire/           # A-30
│   ├── edit-light-ps/        # A-31
│   ├── edit-heavy-ps/        # A-32
│   ├── edit-lightroom/       # A-33
│   ├── edit-mobile/          # A-34
│   └── edit-canva/           # A-35
├── ai-generated/
│   ├── gpt4o/                # S-01
│   ├── midjourney-v6/        # S-02
│   ├── dalle3/               # S-03
│   ├── gemini/               # S-04
│   ├── firefly/              # S-05
│   ├── sdxl/                 # S-06
│   ├── flux11/               # S-07
│   ├── sd35/                 # S-08
│   ├── playground-v3/        # S-09
│   ├── ideogram-v2/          # S-10
│   ├── sd15/                 # S-11
│   ├── stylegan/             # S-12
│   ├── dalle2/               # S-13
│   └── midjourney-v4/        # S-14
├── ai-video/
│   ├── sora/                 # S-15
│   ├── runway-gen3/          # S-16
│   ├── kling/                # S-17
│   └── pika/                 # S-18
├── edge-cases/
│   ├── ai-upscaled/          # E-01
│   ├── ai-jpeg-resave/       # E-02
│   ├── ai-cropped/           # E-03
│   ├── ai-degraded/          # E-04
│   ├── ai-screenshot/        # E-05
│   ├── faceswap/             # E-06
│   ├── ai-sky-replace/       # E-07
│   ├── ai-object-remove/     # E-08
│   ├── ai-fake-exif/         # E-09
│   ├── authentic-png/        # E-10
│   ├── gif-to-png/           # E-11
│   ├── ai-c2pa-stripped/     # E-12
│   ├── hdr-merge/            # E-13
│   ├── focus-stack/          # E-14
│   └── panorama-stitch/      # E-15
└── manifest.json             # Ground truth labels and metadata
```

### 7.2 Manifest Format

Each image must have a ground-truth entry in `test-corpus/manifest.json`:

```json
{
  "images": [
    {
      "path": "authentic/phone-iphone/IMG_0001.HEIC",
      "label": "authentic",
      "category": "A-01",
      "source": "iPhone 15 Pro, main camera",
      "format": "HEIC",
      "notes": "Daylight outdoor scene, portrait orientation",
      "expected_verdict": "Authentic",
      "expected_trust_min": 0.40,
      "hard_fail_verdict": "Synthetic"
    },
    {
      "path": "ai-generated/flux11/flux_landscape_001.png",
      "label": "ai_generated",
      "category": "S-07",
      "source": "Flux 1.1 Pro via Replicate",
      "format": "PNG",
      "prompt": "A photorealistic landscape of Scottish highlands at sunset",
      "notes": "1024x1024, default settings",
      "expected_verdict": "Synthetic",
      "expected_trust_max": 0.55,
      "hard_fail_verdict": "Authentic"
    }
  ]
}
```

### 7.3 Test Runner Script

Save as `scripts/run_real_world_tests.py`:

```python
#!/usr/bin/env python3
"""
Jura Trace -- Real-World Test Runner

Submits every image in the test corpus to the sidecar forensics pipeline
and records per-image results. Requires the sidecar to be running on
localhost:8200.

Usage:
    # Run all tests in Standard mode:
    python scripts/run_real_world_tests.py --corpus test-corpus/

    # Run in Deep mode:
    python scripts/run_real_world_tests.py --corpus test-corpus/ --mode deep

    # Run only a specific category:
    python scripts/run_real_world_tests.py --corpus test-corpus/ --category A-01

    # Resume from a previous run:
    python scripts/run_real_world_tests.py --corpus test-corpus/ --resume
"""

import argparse
import base64
import csv
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import HTTPError

SIDECAR_BASE = "http://127.0.0.1:8200"
API_KEY = os.environ.get("JURA_API_KEY", "")
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".tiff", ".tif", ".heic", ".heif"}
VIDEO_EXTENSIONS = {".mp4", ".mov", ".webm"}


def _post_sidecar(endpoint: str, payload: dict, timeout: int = 120) -> dict:
    """POST JSON to sidecar and return response dict."""
    url = f"{SIDECAR_BASE}{endpoint}"
    data = json.dumps(payload).encode("utf-8")
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
    }
    if API_KEY:
        headers["X-Jura-API-Key"] = API_KEY
    req = Request(url, data=data, headers=headers, method="POST")
    with urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read())


def analyse_image(file_path: Path, mode: str = "standard") -> dict:
    """Run the deepfake detection pipeline on a single image.

    Returns a dict with score, verdict, signals, classifier_score, etc.
    """
    image_bytes = file_path.read_bytes()
    b64 = base64.b64encode(image_bytes).decode("ascii")

    # Detect MIME
    ext = file_path.suffix.lower()
    mime_map = {
        ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
        ".png": "image/png", ".webp": "image/webp",
        ".tiff": "image/tiff", ".tif": "image/tiff",
        ".heic": "image/heic", ".heif": "image/heif",
    }
    mime = mime_map.get(ext, "image/jpeg")

    # Main deepfake detection
    result = _post_sidecar("/forensics/deepfake", {
        "image_base64": b64,
        "mime_type": mime,
    })

    # ELA
    try:
        ela = _post_sidecar("/forensics/ela", {"image_base64": b64})
        result["ela_score"] = ela.get("score", None)
    except Exception:
        result["ela_score"] = None

    # Noise analysis
    try:
        noise = _post_sidecar("/forensics/noise", {"image_base64": b64})
        result["noise_score"] = noise.get("score", None)
    except Exception:
        result["noise_score"] = None

    # Copy-move detection
    try:
        cm = _post_sidecar("/forensics/copy-move", {"image_base64": b64})
        result["copy_move_detected"] = cm.get("detected", False)
    except Exception:
        result["copy_move_detected"] = None

    # Regional forensics (Deep mode; archival is back-compat alias for deep)
    if mode in ("deep", "archival"):
        for endpoint in [
            "/forensics/segmented-ela",
            "/forensics/shadow-consistency",
            "/forensics/colour-temperature",
            "/forensics/splice-boundary",
        ]:
            try:
                r = _post_sidecar(endpoint, {"image_base64": b64})
                key = endpoint.split("/")[-1].replace("-", "_")
                result[f"regional_{key}"] = r
            except Exception:
                pass

    # CLIP detection (if available)
    try:
        clip = _post_sidecar("/forensics/clip-detect", {"image_base64": b64})
        result["clip_ai_prob"] = clip.get("ai_probability", None)
        result["clip_univfd_score"] = clip.get("univfd_score", None)
    except Exception:
        result["clip_ai_prob"] = None

    return result


def analyse_video(file_path: Path) -> dict:
    """Run video deepfake pipeline on a video file."""
    video_bytes = file_path.read_bytes()
    b64 = base64.b64encode(video_bytes).decode("ascii")

    result = _post_sidecar("/forensics/video/deepfake", {
        "video_base64": b64,
        "mode": "standard",
    }, timeout=180)

    return result


def evaluate_result(result: dict, manifest_entry: dict) -> dict:
    """Compare pipeline result against ground truth."""
    score = result.get("score", 0.5)

    # Derive verdict from score (matching lib.rs logic)
    if score > 0.55:
        verdict = "Synthetic"
    elif score > 0.35:
        verdict = "Inconclusive"
    else:
        verdict = "Authentic"

    expected = manifest_entry.get("expected_verdict", "Inconclusive")
    hard_fail = manifest_entry.get("hard_fail_verdict", None)

    if verdict == expected:
        outcome = "PASS"
    elif verdict == hard_fail:
        outcome = "HARD_FAIL"
    else:
        outcome = "SOFT_FAIL"

    # Trust score bounds check
    trust_min = manifest_entry.get("expected_trust_min", None)
    trust_max = manifest_entry.get("expected_trust_max", None)
    trust_ok = True
    if trust_min is not None and score < trust_min:
        trust_ok = False
    if trust_max is not None and score > trust_max:
        trust_ok = False

    # Collect triggered signals
    signals = result.get("signals", [])
    triggered = [s["name"] for s in signals if s.get("triggered", False)]

    return {
        "outcome": outcome,
        "verdict": verdict,
        "expected_verdict": expected,
        "score": round(score, 4),
        "trust_ok": trust_ok,
        "triggered_signals": triggered,
        "triggered_count": len(triggered),
        "total_signals": len(signals),
        "classifier_score": result.get("classifier_score"),
        "clip_ai_prob": result.get("clip_ai_prob"),
        "ela_score": result.get("ela_score"),
    }


def main():
    parser = argparse.ArgumentParser(description="Jura Trace Real-World Test Runner")
    parser.add_argument("--corpus", required=True, help="Path to test-corpus/ directory")
    parser.add_argument("--mode", default="standard", choices=["standard", "deep", "archival"])
    parser.add_argument("--category", default=None, help="Run only this category (e.g. A-01)")
    parser.add_argument("--resume", action="store_true", help="Skip already-tested images")
    parser.add_argument("--output", default=None, help="Output CSV path (default: results-{timestamp}.csv)")
    args = parser.parse_args()

    corpus_dir = Path(args.corpus)
    manifest_path = corpus_dir / "manifest.json"

    if not manifest_path.exists():
        print(f"ERROR: manifest.json not found in {corpus_dir}")
        sys.exit(1)

    manifest = json.loads(manifest_path.read_text())
    entries = manifest["images"]

    if args.category:
        entries = [e for e in entries if e["category"] == args.category]

    output_path = args.output or f"results-{datetime.now().strftime('%Y%m%d-%H%M%S')}.csv"

    # Load previous results for resume
    completed = set()
    if args.resume and os.path.exists(output_path):
        with open(output_path) as f:
            reader = csv.DictReader(f)
            completed = {r["path"] for r in reader}

    print(f"Jura Trace Real-World Test Runner")
    print(f"  Corpus: {corpus_dir}")
    print(f"  Mode: {args.mode}")
    print(f"  Images: {len(entries)} ({len(entries) - len(completed)} remaining)")
    print(f"  Output: {output_path}")
    print()

    fieldnames = [
        "path", "label", "category", "source", "format",
        "outcome", "verdict", "expected_verdict", "score",
        "trust_ok", "triggered_count", "total_signals",
        "triggered_signals", "classifier_score", "clip_ai_prob",
        "ela_score", "elapsed_ms", "error",
    ]

    write_header = not os.path.exists(output_path) or not args.resume
    with open(output_path, "a" if args.resume else "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        if write_header:
            writer.writeheader()

        pass_count = 0
        soft_fail_count = 0
        hard_fail_count = 0
        error_count = 0

        for i, entry in enumerate(entries):
            rel_path = entry["path"]
            if rel_path in completed:
                continue

            file_path = corpus_dir / rel_path
            if not file_path.exists():
                print(f"  [{i+1}/{len(entries)}] SKIP (not found): {rel_path}")
                continue

            print(f"  [{i+1}/{len(entries)}] {rel_path}...", end=" ", flush=True)
            start = time.monotonic()

            try:
                ext = file_path.suffix.lower()
                if ext in VIDEO_EXTENSIONS:
                    result = analyse_video(file_path)
                else:
                    result = analyse_image(file_path, mode=args.mode)

                elapsed = int((time.monotonic() - start) * 1000)
                evaluation = evaluate_result(result, entry)

                row = {
                    "path": rel_path,
                    "label": entry.get("label", ""),
                    "category": entry.get("category", ""),
                    "source": entry.get("source", ""),
                    "format": entry.get("format", ""),
                    "outcome": evaluation["outcome"],
                    "verdict": evaluation["verdict"],
                    "expected_verdict": evaluation["expected_verdict"],
                    "score": evaluation["score"],
                    "trust_ok": evaluation["trust_ok"],
                    "triggered_count": evaluation["triggered_count"],
                    "total_signals": evaluation["total_signals"],
                    "triggered_signals": "|".join(evaluation["triggered_signals"]),
                    "classifier_score": evaluation.get("classifier_score"),
                    "clip_ai_prob": evaluation.get("clip_ai_prob"),
                    "ela_score": evaluation.get("ela_score"),
                    "elapsed_ms": elapsed,
                    "error": "",
                }
                writer.writerow(row)
                f.flush()

                icon = {"PASS": "OK", "SOFT_FAIL": "SOFT", "HARD_FAIL": "FAIL"}[evaluation["outcome"]]
                print(f"{icon} (verdict={evaluation['verdict']}, score={evaluation['score']:.3f}, "
                      f"{evaluation['triggered_count']}/{evaluation['total_signals']} signals, "
                      f"{elapsed}ms)")

                if evaluation["outcome"] == "PASS":
                    pass_count += 1
                elif evaluation["outcome"] == "SOFT_FAIL":
                    soft_fail_count += 1
                else:
                    hard_fail_count += 1

            except Exception as e:
                elapsed = int((time.monotonic() - start) * 1000)
                print(f"ERROR ({e})")
                writer.writerow({
                    "path": rel_path, "label": entry.get("label", ""),
                    "category": entry.get("category", ""), "error": str(e),
                    "elapsed_ms": elapsed,
                    **{k: "" for k in fieldnames if k not in ("path", "label", "category", "error", "elapsed_ms")},
                })
                f.flush()
                error_count += 1

    total = pass_count + soft_fail_count + hard_fail_count
    print(f"\n{'='*60}")
    print(f"RESULTS: {total} tested")
    print(f"  PASS:      {pass_count} ({pass_count/total*100:.1f}%)" if total else "  No results")
    print(f"  SOFT FAIL: {soft_fail_count} ({soft_fail_count/total*100:.1f}%)" if total else "")
    print(f"  HARD FAIL: {hard_fail_count} ({hard_fail_count/total*100:.1f}%)" if total else "")
    print(f"  ERROR:     {error_count}")
    print(f"\nResults written to: {output_path}")


if __name__ == "__main__":
    main()
```

### 7.4 Analysis Script

Save as `scripts/analyse_real_world_results.py`:

```python
#!/usr/bin/env python3
"""
Analyse results from the real-world test runner.

Produces:
  - Per-category pass/fail rates
  - Per-detector signal reliability matrix
  - Score distribution by content type
  - Hard fail root cause analysis
  - Recommendations for threshold/detector tuning

Usage:
    python scripts/analyse_real_world_results.py results-20260328-1200.csv
"""

import csv
import json
import sys
from collections import defaultdict
from pathlib import Path


def load_results(path: str) -> list[dict]:
    with open(path) as f:
        return list(csv.DictReader(f))


def category_summary(results: list[dict]) -> dict:
    by_cat = defaultdict(lambda: {"pass": 0, "soft_fail": 0, "hard_fail": 0, "error": 0, "scores": []})
    for r in results:
        cat = r.get("category", "unknown")
        outcome = r.get("outcome", "error").lower()
        by_cat[cat][outcome] = by_cat[cat].get(outcome, 0) + 1
        if r.get("score"):
            try:
                by_cat[cat]["scores"].append(float(r["score"]))
            except ValueError:
                pass
    return dict(by_cat)


def signal_reliability(results: list[dict]) -> dict:
    """For each signal, compute FP rate (triggers on authentic) and TP rate (triggers on AI)."""
    signal_stats = defaultdict(lambda: {"auth_triggered": 0, "auth_total": 0,
                                         "ai_triggered": 0, "ai_total": 0})
    for r in results:
        label = r.get("label", "")
        triggered = set(r.get("triggered_signals", "").split("|")) if r.get("triggered_signals") else set()

        # We need the full signal list -- use total_signals count
        for sig in triggered:
            if not sig:
                continue
            if label == "authentic":
                signal_stats[sig]["auth_triggered"] += 1
            elif label == "ai_generated":
                signal_stats[sig]["ai_triggered"] += 1

        if label == "authentic":
            for sig in signal_stats:
                signal_stats[sig]["auth_total"] = max(
                    signal_stats[sig]["auth_total"],
                    sum(1 for rr in results if rr.get("label") == "authentic")
                )
        elif label == "ai_generated":
            for sig in signal_stats:
                signal_stats[sig]["ai_total"] = max(
                    signal_stats[sig]["ai_total"],
                    sum(1 for rr in results if rr.get("label") == "ai_generated")
                )

    return dict(signal_stats)


def hard_fail_analysis(results: list[dict]) -> list[dict]:
    """Extract and group hard failures for root cause analysis."""
    failures = [r for r in results if r.get("outcome") == "HARD_FAIL"]
    return sorted(failures, key=lambda r: float(r.get("score", 0.5)), reverse=True)


def main():
    if len(sys.argv) < 2:
        print("Usage: python scripts/analyse_real_world_results.py <results.csv>")
        sys.exit(1)

    results = load_results(sys.argv[1])
    print(f"Loaded {len(results)} results\n")

    # Category summary
    cats = category_summary(results)
    print("CATEGORY SUMMARY")
    print("-" * 80)
    print(f"{'Category':<20} {'Pass':>6} {'Soft':>6} {'Hard':>6} {'Total':>6} {'Pass%':>7} {'Mean Score':>10}")
    for cat in sorted(cats.keys()):
        c = cats[cat]
        total = c["pass"] + c["soft_fail"] + c["hard_fail"]
        pass_pct = c["pass"] / total * 100 if total else 0
        mean_score = sum(c["scores"]) / len(c["scores"]) if c["scores"] else 0
        print(f"{cat:<20} {c['pass']:>6} {c['soft_fail']:>6} {c['hard_fail']:>6} {total:>6} {pass_pct:>6.1f}% {mean_score:>10.3f}")

    # Hard failures
    failures = hard_fail_analysis(results)
    if failures:
        print(f"\nHARD FAILURES ({len(failures)})")
        print("-" * 80)
        for f in failures[:20]:
            print(f"  {f['path']}: score={f.get('score','?')}, verdict={f.get('verdict','?')}, "
                  f"expected={f.get('expected_verdict','?')}, "
                  f"signals={f.get('triggered_signals','')[:60]}")

    # Signal reliability
    sig_stats = signal_reliability(results)
    if sig_stats:
        print(f"\nSIGNAL RELIABILITY")
        print("-" * 80)
        print(f"{'Signal':<35} {'Auth FP Rate':>12} {'AI TP Rate':>12}")
        for sig in sorted(sig_stats.keys()):
            s = sig_stats[sig]
            fp_rate = s["auth_triggered"] / s["auth_total"] * 100 if s["auth_total"] else 0
            tp_rate = s["ai_triggered"] / s["ai_total"] * 100 if s["ai_total"] else 0
            print(f"{sig:<35} {fp_rate:>11.1f}% {tp_rate:>11.1f}%")


if __name__ == "__main__":
    main()
```

---

## 8. Results Recording and Analysis

### 8.1 Output Files

Each test run produces:

| File | Contents |
|------|----------|
| `results-{timestamp}.csv` | Per-image: path, label, category, verdict, score, signals, timing |
| `results-{timestamp}-summary.json` | Aggregate: per-category pass rates, overall metrics |
| `results-{timestamp}-signals.csv` | Per-signal: FP rate, TP rate, by content category |

### 8.2 Key Metrics to Track

**Pipeline-level metrics (computed from CSV):**

| Metric | Formula | Target |
|--------|---------|--------|
| Authentic FP rate | Hard fails on authentic / total authentic | < 5% |
| AI detection rate | (Synthetic + Inconclusive on AI) / total AI | > 80% |
| AI hard detection rate | Synthetic on AI / total AI | > 65% |
| Mean authentic trust score | Mean score for authentic images | < 0.35 |
| Mean AI trust score | Mean score for AI images | > 0.50 |
| Verdict separation gap | Mean AI score - Mean authentic score | > 0.25 |

**Per-detector metrics (from signal reliability analysis):**

| Metric | Good | Concerning | Bad |
|--------|------|-----------|-----|
| Signal FP rate (triggers on authentic) | < 10% | 10-25% | > 25% |
| Signal TP rate (triggers on AI) | > 50% | 25-50% | < 25% |
| Signal discriminative power (TP - FP) | > 30 pp | 10-30 pp | < 10 pp |

### 8.3 Comparative Analysis Across Modes

Run each test image through all three modes and compare:

```bash
# Standard mode (6 detectors, ~2-5 seconds per image)
python scripts/run_real_world_tests.py --corpus test-corpus/ --mode standard --output results-standard.csv

# Deep mode (all detectors + regional, ~10-15 seconds per image)
python scripts/run_real_world_tests.py --corpus test-corpus/ --mode deep --output results-deep.csv
```

Compare whether Deep mode improves detection over Standard without increasing FP rate.

---

## 9. Threshold Tuning Recommendations

Based on the results, tune thresholds for specific content types and detectors.

### 9.1 Decision Framework

After collecting results, analyse the score distributions for authentic vs AI content:

1. **Plot score histograms** for authentic and AI images separately. Look for overlap region.
2. **Compute optimal threshold** using Youden's J statistic: `threshold = argmax(TPR - FPR)`.
3. **Consider asymmetric costs**: A false positive on a museum's authentic photograph damages trust more than missing one AI image. Weight FP cost 3x-5x higher than FN cost.

### 9.2 Per-Source Threshold Adjustments

If specific content types systematically score high (e.g., screenshots), consider:

1. **Format-aware threshold tiers** -- already partially implemented via `_classify_codec()`. May need additional tiers for screenshots (uniform noise detection) and social media (extreme compression detection).
2. **EXIF-conditional scoring** -- already implemented. Verify it works for phone photos and DSLR photos.
3. **Signal gating** -- disable unreliable signals for specific content types. For example, if `lsb_randomness` has > 30% FP rate on screenshots, suppress it for PNG inputs.

### 9.3 Specific Tuning Targets

| Issue | Current Behaviour | Proposed Fix | Validation |
|-------|------------------|-------------|-----------|
| Screenshots flagged as AI | LSB features trigger on lossless PNG | Suppress LSB weight for PNG with uniform noise profile | E-10, A-16 to A-20 |
| Social media downloads Inconclusive | Missing EXIF + compression artefacts | Lower base suspicion when JPEG Q < 60 detected (indicates social media pipeline) | A-11 to A-15 |
| Night mode photos flagged | Smooth noise from computational HDR | Detect multi-frame merge signature; reduce noise_var_cv weight | A-01 night mode subset |
| Flux 1.1 not detected | Very clean frequency spectrum | Increase weight on `cross_channel_noise_corr` and `vae_grid_energy_ratio` | S-07 |
| AI with fake EXIF passes | EXIF FP reduction caps score | Only apply EXIF cap when EXIF is internally consistent (matching Make/Model/Software chain) | E-09 |

---

## 10. Per-Detector Reliability Assessment

After running the full test matrix, produce a reliability grade for each detector. This informs which detectors should have their weight increased or decreased in the heuristic scorer.

### 10.1 Grading Criteria

| Grade | Criteria | Action |
|-------|----------|--------|
| **A** | TP rate > 60%, FP rate < 10%, discriminative power > 50 pp | Increase weight |
| **B** | TP rate > 40%, FP rate < 15%, discriminative power > 25 pp | Keep current weight |
| **C** | TP rate > 25%, FP rate < 25%, discriminative power > 10 pp | Reduce weight or add conditions |
| **D** | FP rate > 25% OR discriminative power < 10 pp | Disable or restrict to specific content types |
| **F** | FP rate > TP rate (actively harmful) | Remove from pipeline |

### 10.2 Expected Outcomes (Hypothesised Before Testing)

| Detector | Expected Grade | Rationale |
|----------|---------------|-----------|
| Spectral decay | B | Robust for GAN content, less effective for modern diffusion |
| HF energy | B | Good for older generators, Flux may evade |
| Noise residual | B-C | Effective but FP-prone on computational photography |
| Noise structure | B | Good discriminator, but phone night mode may trigger |
| Colour distribution | C | Useful as ensemble member but not highly discriminative alone |
| LBP texture | B | Consistent discriminator across generators |
| GLCM texture | B | Similar to LBP, good ensemble member |
| DCT/Benford | A for JPEG, F for PNG | Only meaningful for JPEG content |
| Edge structure | C | Moderate discriminator |
| Cross-channel noise | A | Strong AI signal, low FP expected |
| VAE grid | A for latent diffusion | Specific to VAE-based generators (SD, Flux) |
| CA absence | B | Camera photos have CA; AI does not |
| LSB features | D-F for general, A for lossless | High FP on authentic PNG -- the known confound |
| Demosaicing | A for lossless | Camera sensors leave demosaicing traces; AI does not |
| CLIP detection | A (if available) | Operates on semantic features, orthogonal to pixel forensics |

### 10.3 What the Results Tell Us

After testing, produce a narrative report for each detector answering:

1. **Is this detector fit for purpose?** Does it reliably distinguish AI from authentic content in real-world conditions?
2. **What content types does it fail on?** Are the failures systematic or random?
3. **Should its weight be adjusted?** Up, down, or conditional on content type?
4. **Does it contribute unique signal?** Would removing it change any verdicts?

---

## 11. Execution Timeline

| Phase | Duration | Activities |
|-------|----------|-----------|
| **Corpus assembly** | 3-5 days | Collect 400+ images across all categories. Prepare manifest.json. |
| **Standard mode run** | 1 day | Run all images through Standard mode. Initial results CSV. |
| **Deep mode run** | 1 day | Run all images through Deep mode. Compare with Standard. |
| **Analysis and report** | 1-2 days | Run analysis script. Produce signal reliability matrix. Identify hard failures. |
| **Threshold tuning** | 1-2 days | Adjust thresholds based on results. Re-run to validate. |
| **Regression suite** | 1 day | Select 50 images as permanent regression test set. |

**Total estimated effort:** 8-12 working days.

### 11.1 Priority Order for Corpus Assembly

If time is limited, assemble in this order (highest risk of real-world failure first):

1. **Screenshots** (A-16 to A-20) -- known FP risk from LSB confound
2. **Phone photos** (A-01 to A-05) -- most common real-world input
3. **Flux 1.1 + GPT-4o** (S-01, S-07) -- hardest AI to detect
4. **Social media downloads** (A-11 to A-15) -- stripped EXIF, heavy compression
5. **Edge cases: fake EXIF, C2PA stripped** (E-09, E-12) -- adversarial scenarios
6. Everything else

---

## 12. Regression Test Set

After the initial test run, select a permanent regression set of 50 images:

- 5 images from each authentic category that are "borderline" (scored closest to the detection threshold)
- 5 images from each AI category that are hardest to detect (lowest AI scores)
- 5 edge cases that test specific pipeline behaviours

Store these in `test-corpus/regression/` with their ground-truth manifest. Run as part of CI or before any threshold/model change:

```bash
python scripts/run_real_world_tests.py --corpus test-corpus/regression/ --mode standard
```

Any change in verdicts compared to the baseline CSV indicates a regression.

---

## 13. Appendix: Full Detector Inventory

For reference, the complete set of forensic analysis endpoints available in the sidecar:

| # | Endpoint | Service | Purpose |
|---|----------|---------|---------|
| 1 | `POST /forensics/deepfake` | `deepfake.py` | 21-signal heuristic + GBM classifier + CLIP blending |
| 2 | `POST /forensics/ela` | `ela.py` | Error Level Analysis (JPEG re-save comparison) |
| 3 | `POST /forensics/noise` | `noise.py` | Noise residual analysis |
| 4 | `POST /forensics/copy-move` | `copy_move.py` | Copy-move forgery detection |
| 5 | `POST /forensics/npr` | `npr.py` | Neighbouring Pixel Relationship analysis |
| 6 | `POST /forensics/chromatic-aberration` | `chromatic_aberration.py` | Radial lens CA pattern detection |
| 7 | `POST /forensics/jpeg-ghost` | `jpeg_ghost.py` | Double compression detection |
| 8 | `POST /forensics/segmented-ela` | `segmented_ela.py` | 8x8 grid regional ELA |
| 9 | `POST /forensics/shadow-consistency` | `shadow_consistency.py` | Gradient-based light direction |
| 10 | `POST /forensics/colour-temperature` | `colour_temperature.py` | CIELAB colour space segmentation |
| 11 | `POST /forensics/splice-boundary` | `splice_boundary.py` | Three-signal edge analysis |
| 12 | `POST /forensics/clip-detect` | `clip_detector.py` | CLIP ViT-B/32 zero-shot + UnivFD probe |
| 13 | `POST /forensics/claim-check` | `claim_checker.py` | RAG claim verification (Ollama) |
| 14 | `POST /forensics/watermark/extract` | `watermark.py` | Invisible watermark extraction |
| 15 | `POST /forensics/video/deepfake` | `video_deepfake.py` | Per-frame + temporal video deepfake |
| 16 | `POST /forensics/video/metadata` | `video_metadata.py` | FFmpeg video metadata |
| 17 | `POST /forensics/audio/metadata` | `audio_metadata.py` | FFmpeg audio metadata |
| 18 | `POST /forensics/video/frames` | `video_frames.py` | Frame extraction for thumbnails |
| 19 | `POST /forensics/transcribe` | Whisper | Audio/video transcription |

Plus Rust-side analysis (not via sidecar):
- C2PA manifest verification (`c2pa.rs`)
- EXIF anomaly detection and trust scoring (`exif_anomaly.rs`)
- Perceptual fingerprinting (`fingerprint.rs`)
- Format routing and MIME detection (`format_router.rs`)
- Invisible watermark embed/extract (`watermark.rs`)

---

*This test plan is designed to validate Jura Trace's forensic pipeline against the diversity of real-world content it will encounter in production. Results should feed directly into threshold tuning, detector weight adjustment, and the ongoing FP-reduction workstream.*
