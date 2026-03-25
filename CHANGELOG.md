# Changelog

All notable changes to Jura Trace (formerly Jura Archive) are documented here, organised by development phase and sprint.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## Sprint 19 — Release Candidate (25 Mar 2026, v0.9.0-rc.1)

### Accessibility

**Fixed**
- WCAG 2.2 AA audit: 14 issues fixed across verify, protect, settings, onboarding, and root layout
- Verify page: tabpanel ARIA (`role="tabpanel"`, `aria-controls`, `aria-labelledby`), URL input label, sidecar status badge `role="status"`, focus-visible rings on expand buttons, reverse image search link targets (44px), analyst note character counter `aria-live`
- Protect page: drop zone `disabled` state (was `aria-disabled` only), batch errors toggle `aria-controls` + focus ring
- Onboarding: dialog headings h1 → h2 (avoids duplicate h1 per page)
- Root layout + Settings: external links now announce "(opens in new tab)" for screen readers
- Footer version test decoupled from hardcoded string (regex match)

### C2PA AI Detection

**Fixed**
- C2PA manifests declaring AI generation (e.g. Google Gemini `trainedAlgorithmicMedia`) now correctly penalise trust instead of rewarding it
- New `detect_ai_from_assertions()` scans C2PA assertions for IPTC `trainedAlgorithmicMedia` digitalSourceType, AI keywords ("generative ai", "ai-generated"), and known generator names (gemini, dall-e, sora, firefly, flux, etc.)
- `ai_generator` field now populated from both `claim_generator` strings AND assertion content
- Trust penalty: -0.25 for images/video (was +0.10 bonus), 0.10 for documents (was 0.82)
- Verify page: amber provenance banner when C2PA confirms AI generation — "This content carries a valid, signed C2PA provenance record which confirms it was created using AI generation"

### Help System

**Added**
- Monitor help guide: 8 sections (dashboard, audit trail, hash chain integrity, AI training limitation, URL watchlist preview, best practices)
- Settings help guide: 5 sections (Ollama configuration, deployment profiles, service status, database location, about)

### MONITOR Infrastructure

**Added**
- `monitor_urls` and `monitor_events` tables wired into `db.rs init_schema()`
- `MonitorUrl` and `MonitorEvent` Rust structs with serde camelCase
- 5 CRUD functions: `add_monitor_url`, `remove_monitor_url`, `list_monitor_urls`, `get_monitor_events`, `update_case_status`
- Partial index on `monitor_events(case_status)` for alert inbox performance

### RAG Knowledge Base

**Added**
- Expanded 4 existing knowledge base files (~2x content each): AI generators, C2PA provenance, misinformation patterns, image forensics
- 2 new domain files: `video_forensics.txt`, `digital_rights_and_cultural_heritage.txt`
- Total: 314 lines across 6 documents, ~150 passages (was 158 lines, 4 docs, ~79 passages)

### Security

**Fixed**
- OWASP self-audit: 4 HIGH (path canonicalisation in c2pa/watermark/video commands, shell permissions removed), 4 MEDIUM (CSP hardened, set_db_path extension allowlist, transcription size limit, audit timestamp precision), 2 LOW (audit chain command exposed, fs write scope narrowed)
- Report: `docs/security-pen-test-s19.md`

### File Handling

**Added**
- Corrupt/truncated file guards: zero-length and <12 byte checks in verify and import pipelines with `AppError::Validation` messages
- 3 new Rust tests for empty/tiny file rejection

### Video UX

**Added**
- Video analysis progress: phase labels ("Extracting frames..." → "Running deepfake detection..."), estimated time by mode, soft cancel button with Escape key support — video-only enhancement

### First-Launch Setup Wizard

**Added**
- 5-step `SetupWizard.svelte` component: sidecar health check (3s timeout, auto-advance), FFmpeg status with platform-specific install hints, Whisper model info (auto-downloads on first use), Ollama download button (opens ollama.com), ready summary with service availability checklist
- Runs after onboarding intro, persisted via `localStorage('jura-setup-complete')`
- Wired into `+layout.svelte` with chain: onboarding → setup wizard → app

### External Documentation

**Added**
- User guides: `docs/user-guide/getting-started.md`, `protect-guide.md`, `verify-guide.md`
- Quick Start cards: 1-page per platform (macOS, Windows, Linux)
- SHA-256 download verification guide + `SHA256SUMS.txt.template`
- Windows IT deployment appendix: MSIEXEC silent install, Intune, GPO firewall, PowerShell FFmpeg
- Automated SHA-256 checksum generation in GitHub Actions release workflow

### Strategic & Business

**Added**
- Strategic pivot assessment: verification-first positioning, post-v1.0 Phase A/B roadmap
- Tier structure decision: Community (free) / Professional (£199/yr) / Team (£79/seat/mo) / Enterprise (£6K+/yr) with geological internal codenames
- Tier comparison wiki for Plane
- Enterprise AI API analysis: BYOK model (Mistral/Claude/OpenAI) defensible for Enterprise tier
- Deployment experience design: FFmpeg bundling, Ollama button, model downloads, setup wizard
- Phase A plan: FP reduction + API wrapper + reports + versioning (4 sprints, July-August 2026)

### MONITOR URL Watchlist

**Added**
- 5 Tauri IPC commands: `add_monitor_url`, `remove_monitor_url`, `list_monitor_urls`, `get_monitor_events`, `update_monitor_case_status`
- `MonitorUrl` and `MonitorEvent` TypeScript interfaces + API wrappers
- Monitor tab URL watchlist UI: add URL form (URL + label + frequency selector), URL list with status badges (ok/changed/missing/error), expandable event rows, case management (investigate/resolve/dismiss)

### Auto-Updater

**Added**
- `tauri-plugin-updater` v2 wired into builder chain
- `latest.json` update manifest generation in release workflow
- "Check for Updates" button in Settings About section
- `updater:default` capability permission

### RC Preparation

**Changed**
- Version bumped to 0.9.0 across `tauri.conf.json`, `Cargo.toml`, `package.json`
- `cargo fmt` applied (28 diffs in c2pa.rs, db.rs, lib.rs)

**Fixed**
- Video deepfake Playwright tests: replaced 500ms fixed wait with `waitForFunction` for test hook availability
- Audit log hash chain: ordering changed from `created_at + log_id` to `rowid` — fixes chain verification when rapid inserts share the same millisecond timestamp

### Deepfake Classifier Retrained

**Fixed**
- FP rate reduced from 14% to 0% by eliminating format confound — classifier had learned JPEG=authentic, PNG=AI because all authentic training images were JPEGs
- Corpus expanded 548 → 709 images (390 authentic including PNGs + 319 AI-generated including PNGs)
- AUC-ROC: 1.0000 (was 0.9978)
- Held-out 20% test set: 0% FP, 0% FN at threshold 0.50
- Phase A PV-A1 target (<5% FP) achieved ahead of July 2026 schedule

### Security Remediation (Final)

**Fixed**
- LOW-1: 7 commands migrated from `map_err(e.to_string())` to `AppError` — no raw OS errors cross IPC boundary
- LOW-3: `import_files` now canonicalises paths before database storage
- LOW-4: `case_notes` capped at 10,000 bytes
- LOW-6: Production builds auto-generate 256-bit sidecar API key if `JURA_SIDECAR_KEY` not set
- MEDIUM-5: 8 Python packages pinned to exact versions
- **All pen test items now FIXED — 0 open**

### Test Counts
- Rust: 230 tests, clippy + fmt clean
- Python: 308+ tests
- Playwright: 160+ tests
- SvelteKit: 204 files, 0 svelte-check errors

---

## Sprint 18 — Platform Installers & Deployment Readiness (24 Mar 2026)

### Platform Build Infrastructure

**Added**
- Platform icon generation: `.icns` (macOS), `.ico` (Windows), full PNG set; `tauri.conf.json` `bundle.icon` updated
- macOS `Entitlements.plist`: `network.client`, `files.user-selected.read-write`, `allow-unsigned-executable-memory`, `disable-library-validation`; `minimumSystemVersion: "13.0"`
- macOS DMG builds successfully: 15 MB unsigned, `Jura Trace_0.4.0_aarch64.dmg`
- Linux bundle config: `category: "Utility"`, deb depends (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`)

### Sidecar Auto-Launch (S18-A2)

**Added**
- Tauri sidecar plugin integration: `externalBin` config in `tauri.conf.json`, `shell:allow-execute` and `shell:allow-spawn` in capabilities
- Sidecar lifecycle management in `lib.rs`: spawn on app start (release builds only), exponential-backoff health polling (up to 10 attempts, ~10s), clean process kill on `RunEvent::Exit`
- `AppState.sidecar_process` field for lifecycle tracking
- Platform-specific placeholder stubs in `src-tauri/binaries/` (replaced by CI with real PyInstaller output)
- Builder pattern changed from `.run(ctx)` to `.build(ctx).run(callback)` for exit event handling

### PyInstaller Cross-Platform (S18-A1)

**Fixed**
- `deepfake.py` model path: `JURA_MODELS_DIR` env var override for frozen PyInstaller context, `__file__`-relative fallback for dev mode
- `jura-sidecar.spec`: auto-detect `target_arch` via `platform.machine()`, platform-conditional UPX excludes (`.dylib`/`.dll`/`.so`), `codesign_identity=None` placeholder

### Frontend Cross-Platform Fixes

**Fixed**
- Linux font fallback: added `'DejaVu Serif', 'Noto Serif'` to Tailwind `font-heading` config and `app.css`; removed 38 inline `font-family` declarations, replaced with CSS class
- Settings path separator: replaced `selected.includes('/')` heuristic with `@tauri-apps/api/path` `join` for cross-platform correctness

### Release Infrastructure (S18-B1)

**Added**
- GitHub Actions release workflow: 4-platform matrix (macOS ARM, macOS Intel, Windows, Linux) with PyInstaller sidecar build per platform
- Changelog extraction from `CHANGELOG.md` for release notes
- Pip cache per platform, `patchelf` for Linux AppImage, 90-min timeout
- Code signing env vars commented out (placeholder for Sprint 19)

### MONITOR Preparation

**Added**
- AI training detection disclaimer: permanent lapis info banner on Monitor tab — "Content monitoring cannot detect whether your content has been used to train AI models"
- MONITOR SQLite schema design: `monitor_urls` and `monitor_events` tables with case management (`new`/`investigating`/`resolved`/`escalated`/`dismissed`), partial indexes, denormalised last-status; migration at `src-tauri/migrations/003_monitor_tables.sql`

### Documentation

**Added**
- macOS unsigned install guide (`docs/install-guides/macos-unsigned.md`): 3 Gatekeeper bypass methods, Sequoia workaround
- Windows unsigned install guide (`docs/install-guides/windows-unsigned.md`): SmartScreen, Firewall, WebView2, enterprise Group Policy
- Linux requirements guide (`docs/install-guides/linux-requirements.md`): AppImage/deb/rpm, WebKitGTK, font rendering, Wayland
- Pilot testing script (`docs/pilot-testing/test-script.md`): 30-minute structured session, 4 persona variants
- Linux smoke test checklist (`docs/pilot-testing/linux-smoke-test.md`)
- MONITOR schema design document (`docs/monitor-schema-design.md`)
- Feature scoping: online monitoring 3-layer architecture + paid tier structure (`docs/feature-scoping/`)
- Sprint 18 plan (`docs/sprint-plans/sprint-18-plan.md`)

### Test Counts
- Rust: 211 tests, clippy clean
- Python: 308 tests (+47 deepfake passed with path fix)
- Playwright: 164 tests
- SvelteKit: 201 files, 0 svelte-check errors

---

## Sprint 17 — Quality Floor & Deployment Readiness (24 Mar 2026)

### IPC Error Architecture (S17-A1, A2, A3)

**Added**
- `AppErrorResponse` TypeScript interface in `types.ts`: `{ code: 'Database' | 'FileSystem' | 'Sidecar' | 'Validation' | 'C2pa' | 'Internal', message: string }`
- `parseAppError()` in `api.ts`: normalises structured `AppError` and plain string errors into `{ code, message }`
- `setError()` on verify page upgraded to match on `AppError.code` first, string-sniff fallback for unmigrated commands
- Tiered error banner messages: dev mode shows technical details (uvicorn command), production shows user-friendly restart instructions
- `data-testid="error-banner"` and `data-error-code={errorType}` attributes on error banner
- `__juraSetVerifyResult` and `__juraSetVerifyError` test hooks gated behind `import.meta.env.DEV` (security: removed from production builds)
- Playwright error tests migrated from `toContainText` copy matching to `toHaveAttribute('data-error-code', type)`

### Database Path Configurability (S17-B1)

**Added**
- Three-source priority resolution: `JURA_DB_PATH` env var > `config.json` `db_path` key > default `app_data_dir/jura_archive.db`
- `AppConfig` struct with `read_app_config()` / `write_app_config()` helpers
- `dir_is_writable()` probe-file check (cross-platform)
- `get_db_path` and `set_db_path` Tauri commands with atomic copy + SQLite integrity check
- `AppState.db_path` field for runtime path tracking
- Settings page: "Database Location" section with folder picker, progress spinner, success/error feedback
- `getDbPath()` and `setDbPath()` IPC wrappers in `api.ts`
- 8 new Rust unit tests for config round-trip, writable check, priority logic

### Video Deepfake Quality (S17-C1)

**Added**
- Rolling buffer frame deduplication (`deduplicate_frames()`, `buffer_size=5`) catches cyclical video repeats
- Normalised correlation similarity metric for frame comparison
- 12 new Python tests: cyclical dedup, buffer size limits, threshold sensitivity, performance (<50ms for 40 frames)

### Privacy & UX (S17-D1)

**Added**
- Source protection privacy warning in Investigate Further panel: cautions about sharing URLs with third-party search services

### PDF Trust Scoring

**Fixed**
- PDFs no longer always score 50% — `document_trust()` helper: C2PA valid = 0.82, C2PA invalid = 0.25, no C2PA = 0.50
- Limited-analysis info banner on verify page for document content types

### In-App Help Documentation System

**Added**
- `/help` route with responsive sidebar navigation (desktop sidebar, mobile tab strip)
- `HelpSidebar` component with grouped sections, `aria-current="page"` active state
- Help index page with topic card grid
- Protect guide: C2PA signing, watermarking (3 strength levels), batch, asset management, best practices
- Verify guide: investigation modes, trust scores, verdicts, video/audio, exports, document analysis
- Methodology transparency page: all 16 detectors explained with `<details>` disclosures, trust scoring formula, signal weighting, honest limitations
- Glossary: 34 terms A-Z with sticky alphabet jump bar and deep-link anchors
- Persona usage guides: museum staff, journalists, content creators, researchers — recommended workflows, modes, tips
- Monitor and Settings stub pages
- `ContextualHelpLink` component (`?` icon) added to verify (modes, trust score), protect (watermark), monitor pages
- Help link added to main navigation
- 30 new Playwright tests for contextual help links

### Deployment & Documentation (S17-B2, E1)

**Changed**
- `DEPLOYMENT.md` updated for Phase 3: sidecar API key auth, database path configurability, speech transcription setup, Windows/Linux status → Sprint 18

**Added**
- PyInstaller sidecar bundling spike: GO for Sprint 18, 315 MB binary (with CLIP/torch), all core endpoints work on macOS arm64, `jura-sidecar.spec` produced, 2 path-resolution fixes documented
- Feature scoping document: online content monitoring (3-layer architecture), in-app help system, paid tier structure (Flint/Stratum/Geode/Bedrock)
- Sprint 18 plan: unsigned platform installers, frozen sidecar production integration

### Test Counts
- Rust: 211 tests (was 190), clippy clean
- Python: 308 tests (+12 new dedup), +5 skipped without ffprobe/whisper
- Playwright: 164 tests (was 110, +30 help + 4 error + 20 other)
- SvelteKit: 200 files, 0 svelte-check errors (was 180)

---

## Sprint 16 — Performance, Error Handling & Pipeline Parallelism

**Added**
- `AppError` enum in `src-tauri/src/error.rs` with structured IPC serialisation
- Performance timing instrumentation across verify pipeline
- Error classification on verify page with `errorType` state
- LOW security remediations completed

---

## Sprint 14 — Video Deepfake Analysis, Batch Watermarking & Security Hardening

### Week 26 — Video Deepfake + Security Audit (21 Mar 2026)

#### Video Deepfake Analysis

**Added**
- Video deepfake analysis service (`video_deepfake.py`): runs the existing per-image deepfake pipeline across evenly-spaced frames extracted from a video file
- Three analysis modes: standard (6 frames, ~12 s), deep (20 frames, ~40 s), archival (40 frames, ~80 s)
- Temporal consistency signals: noise drift, spectral drift, LBP drift — detect frame-level inconsistencies that per-frame scoring alone cannot surface
- Aggregation formula: `0.5 × mean_score + 0.3 × max_score + 0.2 × temporal_score`
- `POST /forensics/video/deepfake` endpoint with 120 s timeout
- `deepfake.py` refactored: extracted `perform_deepfake_detection_with_features()` to expose raw feature vectors for internal reuse by the video pipeline
- `FrameDeepfakeResult` and `VideoDeepfakeResult` Rust structs with serde camelCase/snake_case aliases
- `SidecarClient::analyse_video_deepfake()` method
- `analyse_video_deepfake` Tauri command wired into the verify pipeline
- Verify page frame timeline: coloured score badges per frame (green/amber/red) and aggregate verdict panel
- `FrameDeepfakeResult` and `VideoDeepfakeResult` TypeScript interfaces

#### Batch Watermarking UI

**Added**
- "Watermark All Images" button on the Protect page — queues all un-watermarked image assets for batch processing
- Progress bar showing completion count against total (e.g. 12 / 47)
- Cancel button aborts the remaining queue and reports how many were completed
- Completion summary panel: assets processed, skipped (non-image), and any errors

#### Security Audit & Hardening

**Added**
- Full security audit report (`docs/security-audit-report.md`): 21 findings across four severity levels (3 critical, 6 high, 7 medium, 5 low), with remediation status for each
- `url` crate dependency added to `Cargo.toml` for structured URL parsing

**Fixed**
- **CRITICAL-1 — SSRF in `verify_url`**: added URL validation before download; blocks loopback addresses (127.0.0.1, ::1, localhost), link-local ranges (169.254.0.0/16, fe80::/10), and RFC 1918 private networks (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16). Parsing via `url` crate prevents scheme confusion and encoded bypasses.
- **CRITICAL-2 — Over-broad filesystem capability**: Tauri `fs` capability scope restricted from wildcard to user directories only (`$HOME`, `$DOCUMENT`, `$DOWNLOAD`, `$DESKTOP`, `$TEMP`)
- **CRITICAL-3 — Unpinned CSP `connect-src`**: Content Security Policy `connect-src` directive pinned to explicit origins `http://127.0.0.1:8200` and `http://127.0.0.1:11434` only; wildcard removed

**Test counts**: 183 Rust, 292 Python (+3 skipped without ffprobe, +14 CLIP skipped), 104 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

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
