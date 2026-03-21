# Changelog

All notable changes to Jura Trace (formerly Jura Archive) are documented here, organised by development phase and sprint.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## Sprint 13 — Video Frames, Audio Metadata & Extended C2PA

### Week 25 — Frame Extraction + Audio Support (21 Mar 2026)

#### Video Frame Extraction

**Added**
- Video frame extraction service (`video_frames.py`): evenly-spaced thumbnails extracted from video as base64 JPEG via FFmpeg
- `POST /video/frames` endpoint returning frame count, timestamps, and base64-encoded thumbnail array
- Frame thumbnail strip on the verify page for visual inspection of video content
- `VideoFramesResult` Pydantic model, Rust struct, and TypeScript interface

#### Audio Metadata

**Added**
- Audio metadata extraction service (`audio_metadata.py`): codec, sample rate, channels, bitrate, duration via FFmpeg/ffprobe
- `POST /audio/metadata` endpoint
- `AudioMetadataResult` Pydantic model, Rust struct, and TypeScript interface
- C2PA signing extended to `audio/wav` and `audio/mpeg` content types
- Audio metadata display on the Protect page (codec, sample rate, channels, bitrate, duration)

#### FFmpeg Integration

**Added**
- FFmpeg availability detection with graceful degradation — audio/video metadata and frame extraction skipped cleanly when FFmpeg is not installed
- Shared `ffprobe_extract()` helper used across video and audio metadata services

**Test counts**: 180 Rust, 280 Python (+3 skipped without ffprobe), 104 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

---

## Sprint 12 — Watermark Extraction, Video Metadata & Extended C2PA

### Week 24 — Verify Pipeline Watermark + Video Basics (21 Mar 2026)

#### Watermark Detection in Verify Pipeline

**Added**
- Watermark extraction wired into the verify pipeline — runs automatically on all protected images
- Watermark detection panel on the verify page showing institution name and confidence score
- `WatermarkExtractionResult` fields on `VerificationResult` (institution, confidence, payload)

#### Video Metadata

**Added**
- Video metadata extraction service (`video_metadata.py`): codec, resolution, FPS, duration, audio track information via FFmpeg/ffprobe
- `POST /video/metadata` endpoint
- `VideoMetadataResult` Pydantic model, Rust struct, and TypeScript interface
- C2PA signing extended to `video/mp4` and `video/quicktime` content types
- Video metadata display on the Protect page (codec, resolution, FPS, duration)

#### Batch Watermarking Preparation

**Added**
- Batch watermarking infrastructure: queue management and progress tracking (UI wiring deferred to Sprint 14)

**Test counts**: 165 Rust, 258 Python, 98 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

---

## Sprint 11 — Invisible Watermarking & CI/CD

### Week 23 — DWT-DCT-SVD Watermarking + GitHub Actions (21 Mar 2026)

#### Invisible Watermarking

**Added**
- Rust `watermark.rs` module using the `blind_watermark` crate — frequency-domain DWT-DCT-SVD invisible watermarking
- Three watermark strength levels: Low (~48 dB PSNR, maximum invisibility), Medium (~42 dB, balanced), High (~36 dB, maximum robustness)
- 128-bit UUID payload embedded per asset; survives JPEG compression at Q70+, proportional resize, and up to 30% crop
- `embed_watermark_asset` and `extract_watermark_from_path` Tauri commands
- Python watermark service (`watermark.py`) using `imwatermark` library with DWT-DCT-SVD algorithm
- `POST /forensics/watermark/embed` endpoint — embeds watermark and returns protected image
- `POST /forensics/watermark/extract` endpoint — recovers payload, confidence, and strength estimate
- `WatermarkEmbedRequest`, `WatermarkExtractRequest`, `WatermarkExtractResult` Pydantic models, Rust structs, and TypeScript interfaces
- Protect page watermark UI: institution name field, strength selector (Low / Medium / High), embed button
- `watermark_payload` and `watermark_strength` columns added to assets table in SQLite

#### CI/CD Infrastructure

**Added**
- GitHub Actions CI workflow: Rust (`cargo test` + `cargo clippy`), Python (`pytest`), and Frontend (`svelte-check`) jobs run in parallel on push and pull request
- GitHub Actions Release workflow: 4-platform matrix build (macOS x64, macOS arm64, Windows x64, Linux x64) triggered on version tag
- Dependabot configuration for Cargo, npm, and pip dependency updates
- `scripts/release.sh`: version-bump helper that updates `tauri.conf.json`, `Cargo.toml`, and `package.json` in lockstep

**Test counts**: 150 Rust, 240 Python, 92 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

---

## Sprint 10 — Trained AI Image Classifier

### Week 22 — GBM Classifier + Corpus Pipeline (20 Mar 2026)

#### AI Detection Improvement

**Added**
- GradientBoosting classifier trained on 80-feature vector from the deepfake detection pipeline
- 7 new deepfake signal extractors: noise autocorrelation tau (wavelet-based), cross-channel noise correlation, bit-plane regularity (lossless only), VAE grid artefacts (FFT-based), chromatic aberration absence, demosaicing traces (lossless only), saturation-luminance anomaly
- Classifier blending: 35% heuristic + 65% classifier for final score
- Auto-detection of PNG/WebP mime type from file magic bytes in deepfake endpoint
- `classifier_score` and `classifier_available` fields on DeepfakeResponse (Python, Rust, TypeScript)
- `FEATURE_NAMES` constant and `extract_feature_vector()` for stable training/inference contract
- Graceful degradation: if .joblib model absent, heuristic-only mode unchanged

#### Calibration Pipeline

**Added**
- `scripts/build_corpus.py`: Guardian press photo downloader (60 days, signed CDN URLs)
- `scripts/expand_corpus.py`: HuggingFace dataset + COCO val2017 downloader
- `scripts/build_corpus_ai.py`: AI image corpus builder
- `scripts/train_classifier.py`: feature extraction + GBM training + 5-fold stratified CV
- `scripts/evaluate_classifier.py`: model evaluation with precision/recall/AUC reporting
- `scripts/calibrate.py`: batch-process corpus through all detectors with threshold recommendations

#### Detector Threshold Recalibration

**Changed**
- Chromatic aberration: rewrote scoring — low R² now scores low (real lenses), "uncanny valley" high R² flags. FP: 75% -> 0%
- Shadow consistency: deviation 45° -> 80°, require >= 2 inconsistent regions, min 3% area. FP: 83% -> 8%
- Colour temperature: threshold 8 -> 14 LAB units, require 3+ adjacent cluster regions. FP: 58% -> 17%
- Splice boundary: never flags suspicious alone (corroborating signal only). FP: 100% -> 0%
- JPEG ghost: PNG images return neutral result immediately
- Deepfake: benford_divergence weight 0.5 -> 0.25, spectral_decay 1.5 -> 0.75
- Lossless codec thresholds tightened for all signals

#### Results

- **Training corpus**: 326 authentic (Guardian + COCO + HuggingFace) + 219 AI-generated (Gemini + HuggingFace)
- **Cross-validation AUC-ROC**: 0.945
- **AI detection rate**: 68% (13/19), up from 21% — all 12 Gemini PNGs detected
- **Authentic false positive rate**: 14% (14/100), meets 15% target
- **Top discriminating features**: lsb_randomness (28%), lsb_entropy_mean (25%), lbp_block_var_cv (21%)

**Test counts**: 138 Rust, 263 Python (+ 14 CLIP skipped), 92 Playwright e2e, 175 SvelteKit files, 0 svelte-check errors

---

## Sprint 9 — Region-Based Forensics & UI Redesign

### Week 21 — Region-Based Composite Detection + Sanctuary Theme (20 Mar 2026)

#### Region-Based Forensic Analysis

**Added**
- Segmented ELA service (`segmented_ela.py`): 8x8 grid regional error level analysis with 2-sigma anomaly detection and flood-fill cluster detection
- Shadow consistency service (`shadow_consistency.py`): gradient-weighted light direction analysis per foreground component using Otsu segmentation and circular statistics
- Colour temperature service (`colour_temperature.py`): CIELAB colour space analysis on a 4x4 grid with 8 LAB unit deviation threshold and spatial cluster detection
- Splice boundary service (`splice_boundary.py`): three-signal edge analysis (JPEG grid alignment, noise asymmetry, feathering) with 2-of-3 criterion to reduce false positives
- `POST /forensics/segmented-ela`, `/shadow-consistency`, `/colour-temperature`, `/splice-boundary` endpoints
- 8 new Rust structs for regional forensic results with serde camelCase/snake_case aliases
- 4 new `SidecarClient` methods with 30-second timeouts
- `VerificationResult` extended with 4 optional regional result fields
- `compute_trust()` updated with regional weights (segmented ELA 1.5, shadow 1.0, colour temp 1.5, splice 1.0) and composite amplification cap — when 2+ regional detectors are suspicious, trust capped at 0.55
- Region Analysis collapsible section on verify page (Deep/Archival mode only)
- `VerdictSummary` gains composite-signal awareness — simultaneous splice + ELA corroboration is highest-confidence composite indicator
- 8 TypeScript interfaces for regional results
- 27 new Python tests, 16 new Rust tests
- Sprint plan: `docs/sprint-plans/sprint-region-forensics.md`

**Validated**
- Known composite image (person dropped into group photo) that passed all 7 global detectors now triggers 3/4 regional detectors: shadow consistency (174 degree deviation), colour temperature (9/16 regions anomalous), splice boundary (45 candidate boundaries)

#### UI Redesign — Sanctuary Theme

**Added**
- Rebrand from "Jura Archive" to "Jura Trace" throughout all files
- Eye logo mark (`LogoMark.svelte`): concentric circles (lapis outer, cream iris, dark pupil)
- SVG favicon using the eye mark
- Sanctuary theme: warm dark background (#1E2128), cream text (#EDEAE4), soft accent blue (#5A85B5)
- Editorial dashboard layout: Georgia serif headlines, 900px content width, earth-line gradient dividers, narrative chapters, philosophy blockquote
- Mobile hamburger menu with body scroll lock, 44px touch targets, aria-expanded
- Skip navigation link as first focusable element
- `aria-current="page"` on active navigation links
- `prefers-reduced-motion` global animation disable
- Footer philosophy: "Keep people at the heart of every decision. Use technology to support and guide, not to take over."
- Links to juralabs.org in header and footer
- Playwright e2e test harness: 92 tests across navigation, responsive, accessibility, and page smoke tests
- Three design concept mockups in `docs/design-concepts/`

**Changed**
- Flint colour darkened #9B9890 -> #78756D for light mode AA contrast (4.6:1 on white)
- Lapis colour darkened #3E6FA8 -> #376399 for light mode AA contrast (5.0:1 on white)
- All `text-flint` instances updated with `dark:text-flint-light` for proper dual-mode contrast
- All bare `text-quartz` on light surfaces changed to `text-text-light dark:text-quartz`
- Max content width reduced from 1280px to 900px for editorial breathing room
- "ML Sidecar" language replaced with "Analysis services" throughout

**Test counts**: 136 Rust, 261 Python (+ 14 CLIP skipped), 92 Playwright e2e, 175 SvelteKit files, 0 svelte-check errors, Lighthouse 97% accessibility / 100% best practices

---

## Phase 2 — Detection Improvement & RAG

### Weeks 19-20+ — Detection Improvement & RAG Claim Checker (18 Mar 2026)

#### Sprints 1-4: Detection Honesty & False Positive Reduction

**Added**
- Three-way verdict system (authentic / inconclusive / synthetic) replaces binary suspicious/clean classification
- Trust ceiling: inconclusive capped at 60%, synthetic at 25-45% — fixes cases where manipulated images scored "92% High Trust"
- Confidence badge displayed alongside verdict label in the UI
- "Mixed signals" verdict path when detectors disagree
- 14th signal: GLCM texture structure, catching dispersed AI texture patterns missed by earlier extractors
- Scene complexity metric: halves texture and sharpness weights for uniform scenes (fog, snow, overcast sky)
- EXIF-informed sigmoid midpoint: presence of camera EXIF data shifts scoring toward authentic
- Anti-correlation penalty: texture-only signal clusters reduced in weight by 40%
- False positive reporting: structured reason codes stored in SQLite for ongoing calibration

**Changed**
- Codec-aware thresholds: AVIF, WebP, and HEIC images use relaxed scoring profiles calibrated against real iPhone photographs
- Anti-correlation penalty reduces score inflation when only texture signals fire

#### Sprint 5: New Forensic Detectors

**Added**
- NPR (Neighbouring Pixel Relationships): pixel-level correlation analysis to detect statistical discontinuities at splice boundaries
- Chromatic aberration consistency: radial lens CA pattern detection — authentic lens optics produce a predictable radial signature that AI generators do not replicate faithfully
- JPEG ghost detection: double compression analysis to identify spliced or composited regions

#### Sprint 6: Pipeline Wiring & Investigation Modes

**Added**
- All three new detectors (NPR, chromatic aberration, JPEG ghost) wired into the Rust verify pipeline
- Four investigation modes: Quick (~5 s), Standard (~15 s, default), Deep (~60 s), Archival
- Investigation mode selector in the VERIFY page UI

**Changed**
- Default investigation mode changed from Deep to Standard, reducing routine verification time from ~60 s to ~15 s

#### Sprint 7: RAG Claim Checker & False Positive Marking

**Added**
- RAG claim verification service (`claim_checker.py`) — extracts claims from image context, queries Ollama Qwen2.5, returns structured verdicts
- `POST /forensics/claim-check` endpoint
- False positive marking flow in the UI with structured reason codes; reports stored in SQLite for calibration feedback

#### Sprint 8: CLIP / UnivFD AI Detection

**Added**
- CLIP ViT-B/32 zero-shot classification via open_clip (~350 MB, lazy-loaded on first use)
- `clip_detector.py` service with graceful degradation when open_clip is not installed
- 14 CLIP tests, skipped automatically when open_clip is unavailable

**Fixed**
- Calibration finding documented: generic zero-shot prompts do not discriminate reliably between AI and authentic images at this model scale — a UnivFD linear probe is required for production-grade discrimination

#### UI Components (Weeks 19-20+)

**Added**
- `VerdictSummary.svelte`: three-way verdict display with confidence badge
- `InspectionChecklist.svelte`: 8-item manual visual inspection guide for analysts
- `SignalAgreement.svelte`: per-detector agreement/disagreement dashboard showing which signals align and which conflict
- Reverse image search buttons: Google Lens, TinEye, Yandex — one-click launch from the verify page
- 4-mode investigation selector on the verify page

**Test counts**: 121 Rust, 193 Python (+ 14 CLIP skipped), 175 SvelteKit files, 0 svelte-check errors

---

## Phase 2 — ML Sidecar + Forensic Pipeline

### Weeks 17-18b — AI Watermark Detection + Signal Calibration (5 Mar 2026)

**Added**
- Invisible watermark detection for Stable Diffusion v1, SDXL, and Flux images via `invisible-watermark` library (DWT decode, no PyTorch)
- SD v1 watermark: 136-bit exact string match ("StableDiffusionV1"), zero false positive rate
- SDXL/Flux watermark: 48-bit pattern matching (≥40/48 threshold = very likely, ≥35 = possible)
- `WatermarkDetection` Pydantic model, Rust struct, and TypeScript interface
- `watermarks` field on `DeepfakeResponse` / `DeepfakeResult` (default empty, backwards-compatible)
- Watermark detection banner on verify page AI Generation Detection section (cinnabar styling)
- C2PA AI generator identification — `detect_ai_generator()` checks `claim_generator` against 21 known AI services (OpenAI, Adobe Firefly, Midjourney, Stability AI, Flux, Google Gemini, etc.)
- `ai_generator` field on `VerificationResult` with badge in C2PA Credentials UI section
- 3 new deepfake signal extractors: `noise_consistency` (weight 1.5), `patch_spectral_variance` (weight 2.0), `multiscale_gradient` (weight 1.0)
- Patch-level spectral variance: CV of per-patch HF energy across 64×64 patches (strongest new discriminator, 2.5× separation between AI and authentic)
- Multi-scale gradient ratio: Gaussian pyramid (3 levels) gradient energy comparison
- 7 new Python watermark tests, 4 new Python signal tests, 3 new Rust tests
- Dependency: `invisible-watermark>=0.2.0`

**Changed**
- Deepfake ensemble expanded from 10 to 13 weighted signals, total weight 12.5 → 17.0
- Sigmoid scoring recalibrated: midpoint 0.25 → 0.18, steepness k 10 → 12
- `benford_divergence` weight reduced from 1.0 to 0.5 (empirically weak signal)
- Copy-move detection: added RANSAC geometric verification, scaled minimum distance with image diagonal, sigmoid area scoring, raised DBSCAN min_samples to 8
- Noise analysis: sigmoid scoring with midpoint 0.30, raised MAD z-score threshold to 4.5
- Trust computation: concordance-aware weighted formula (ELA=2.0, noise=1.0, copy-move=1.0), AVIF-safe EXIF penalty adjustment for web codecs

**Fixed**
- AI-generated images scoring too low (chihuahua: 0.55 → 0.94) due to GAN-era signal thresholds missing modern diffusion model output
- AVIF images triggering false positives in noise and copy-move detectors due to compression artefacts
- Wavelet denoiser destroying AVIF noise discrimination (reverted to median blur after testing)

**Test counts**: 98 Rust, 47 Python, 0 svelte-check errors

---

### Weeks 17-18 — Deepfake / AI-Generated Image Detection (2 Mar 2026)

**Added**
- Statistical feature ensemble for detecting AI-generated images — 6 feature extractors (frequency domain FFT, noise residual, colour/texture, LBP+GLCM, JPEG DCT artefacts, edge analysis) combined via 8 weighted heuristic signals
- `POST /forensics/deepfake` endpoint returning score, confidence, signal breakdown, and frequency spectrum heatmap
- `DeepfakeSignal` and `DeepfakeResult` Rust structs + `detect_deepfake()` sidecar client method (60s timeout)
- Deepfake detection wired into verify pipeline with graceful degradation
- AI Generation Detection section on verify page with heatmap, summary, expandable signal list, confidence indicator
- Deepfake capability shown on dashboard sidecar status
- `DeepfakeSignal` and `DeepfakeResult` TypeScript interfaces
- 10 Python deepfake tests, 2 Rust deserialization tests
- Dependencies: `scikit-image>=0.24`, `scipy>=1.14`

**Changed**
- Updated `docs/ARCHITECTURE.md` — corrected port (1420), updated module tables, added sidecar services table, rewrote verify pipeline diagram

**Test counts**: 84 Rust, 35 Python, 0 svelte-check errors

---

### Weeks 15-16 — Noise Analysis + Copy-Move Detection (1 Mar 2026)

**Added**
- Block-wise noise variance analysis service (`noise_analysis.py`) — Laplacian filter + MAD-based outlier detection with JET colourmap heatmap
- Copy-move forgery detection service (`copy_move.py`) — ORB keypoints + BFMatcher self-matching + DBSCAN clustering with visualisation
- `POST /forensics/noise` and `POST /forensics/copy-move` endpoints
- `NoiseResult`, `CloneRegion`, `CopyMoveResult` Rust structs + client methods
- Noise and copy-move wired into verify pipeline; all forensic scores averaged for trust computation
- Noise Analysis and Copy-Move Detection sections on verify page
- Extracted `_read_and_validate()` shared helper in forensics.py
- Extracted `build_image_form()` shared helper in sidecar.rs
- 8 noise analysis tests, 7 copy-move tests, 3 Rust deserialization tests
- Dependency: `scikit-learn>=1.5`

**Changed**
- Refactored trust computation from ELA-only to generic `forensic_signals` vec averaging all available signals
- Renamed `elaScoreClass`/`elaScoreBgClass` to `forensicScoreClass`/`forensicScoreBgClass`

**Test counts**: 82 Rust, 25 Python, 0 svelte-check errors

---

### Weeks 13-14 — Sidecar Bridge + ELA + URL Verification (28 Feb 2026)

**Added**
- Python ML sidecar (FastAPI on port 8200) with health endpoint and ELA service
- Error Level Analysis service (`ela.py`) — JPEG recompression difference with heatmap
- `POST /forensics/ela` endpoint
- Rust `SidecarClient` with `is_available()`, `check_health()`, `analyse_ela()` methods
- ELA wired into verify pipeline with graceful degradation
- ELA result section on verify page with heatmap, statistics, suspicious warning
- URL verification via `verify_url` Tauri command — downloads content to temp file and runs through pipeline
- URL tab on verify page with input field
- Sidecar health status badge on verify page and dashboard
- `SidecarHealth`, `SidecarCapabilities`, `ElaResult` TypeScript interfaces
- Browser mock fallback in `api.ts` for `checkSidecarHealth()`
- 7 ELA tests, 3 health tests, 6 Rust sidecar tests

**Test counts**: 76 Rust, 10 Python, 0 svelte-check errors

---

## Phase 1 — Core MVP

### Weeks 1-12 — PROTECT + VERIFY Foundation (27 Feb 2026)

**Added**
- Tauri v2 desktop shell with SvelteKit frontend (SPA mode, static adapter)
- Dark-mode UI with mineral colour palette (Obsidian, Graphite, Lapis, Malachite, Amber, Cinnabar)
- Four-tab navigation: PROTECT, VERIFY, MONITOR (placeholder), SETTINGS (placeholder)
- **PROTECT pipeline**: file import via drag-and-drop + native dialog, format detection (image/document/video/audio/3D/web), EXIF metadata extraction, image dimension reading, SQLite storage, audit logging
- **C2PA Content Credentials**: sign assets with ES256 self-signed certificates, read and verify manifests, signed output path management
- **Perceptual fingerprinting**: aHash, dHash, pHash via image_hasher crate, Hamming distance similarity search, fingerprint storage
- **EXIF anomaly detection**: 12 checks (timestamps, GPS, software, dimensions), severity levels (info/low/medium/high/critical), trust score computation
- **Format router**: MIME detection via `infer` crate with extension fallback, content type classification
- **VERIFY pipeline**: file drop + file dialog, EXIF analysis, C2PA manifest reading, trust score with findings display
- **SQLite database**: assets, fingerprints, verifications, audit_log tables with operator_id and algorithm_metadata
- **Dashboard**: stats grid (assets, signed, fingerprints, verifications), recent assets list, quick action cards
- **PROTECT page**: asset table with filtering (content type, signed status, search), bulk import, C2PA signing dialog, fingerprint viewer, similarity search, asset deletion
- Tiered cataloguing architecture (Tier 1: EXIF, Tier 2: CLIP/ONNX optional, Tier 3: Ollama optional)
- Three-tier AI design with no mandatory external dependencies
- Docker Compose setup and GitHub Actions CI/CD workflows

**Test counts**: 70 Rust, 0 svelte-check errors
