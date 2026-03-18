# Changelog

All notable changes to Jura Archive are documented here, organised by development phase and sprint.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
