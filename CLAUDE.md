# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Jura Labs Ecosystem

Jura Trace is one of two products built by **Jura Labs** (UK Social Enterprise — CIC registration pending). The full technology stack, infrastructure, and strategic context are documented in:

- **Tech stack reference**: `../jura-labs-docs/JURA-LABS-TECH-STACK-UPDATED.md`
- **Project management**: [Plane workspace](https://app.plane.so/jura-labs/) — connected to Claude Code via MCP (user scope)
- **Sister product**: ROOTED (carbon emissions guidance) — see `../ecoadvisor/`
- **Shared docs**: `../jura-labs-docs/`

## Project Overview

**Jura Trace** is a local-first desktop application for content verification and protection. In a world of synthetic media, verification matters. It helps cultural institutions protect their digital assets from unauthorised AI extraction, and helps communities verify content authenticity.

**Developed by**: Juralabs Community Interest Company (UK) — https://juralabs.org
**Licence**: PolyForm Noncommercial 1.0.0
**Current Version**: 0.5.0-dev (Phase 3 active — Sprint 15 complete, Sprint 16 next)

## Core Architecture

```
┌─────────────────────────────────────────┐
│        Tauri v2 Desktop Shell (Rust)    │
│        macOS / Windows / Linux          │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│     SvelteKit Frontend (Port 1420)      │
│  PROTECT | VERIFY | MONITOR | SETTINGS  │
└────────────────┬────────────────────────┘
                 │ Tauri IPC
┌────────────────▼────────────────────────┐
│          Rust Core Engine               │
│  C2PA | Fingerprint | EXIF Anomaly      │
│  Metadata | Format Router | Sidecar     │
│  Watermark | Video/Audio Metadata       │
│  SQLite Database                        │
└────────────────┬────────────────────────┘
                 │ HTTP (localhost:8200)
┌────────────────▼────────────────────────┐
│      Python ML Sidecar (Port 8200)      │
│  ELA | Noise | Copy-Move | Deepfake     │
│  NPR | Chrom. Aberration | JPEG Ghost   │
│  Segmented ELA | Shadow Consistency     │
│  Colour Temperature | Splice Boundary   │
│  CLIP Detector | RAG Claim Checker      │
│  Watermark Embed/Extract                │
│  Video Metadata | Audio Metadata        │
│  Video Frame Extraction                 │
│  Video Deepfake (per-frame + temporal)  │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│    Ollama (Port 11434) — Optional       │
│  LLaVA (Tier 3 descriptions)           │
│  Qwen2.5 (RAG claim verification)      │
└─────────────────────────────────────────┘
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 (Rust) |
| Frontend | SvelteKit 5 + TailwindCSS (SPA mode, static adapter, Svelte 5 runes) |
| Core engine | Rust (c2pa-rs, rusqlite, image_hasher, reqwest) |
| Auto-catalogue | Tier 1: EXIF (Rust), Tier 2: CLIP/ONNX (optional), Tier 3: Ollama (optional) |
| ML sidecar | Python 3.13 + FastAPI (ELA, noise, copy-move, deepfake, NPR, chromatic aberration, JPEG ghost, segmented ELA, shadow consistency, colour temperature, splice boundary, CLIP detection, watermark embed/extract, video metadata, audio metadata, video frame extraction, video deepfake per-frame analysis) |
| CLIP detection | open_clip ViT-B/32 — optional (~350 MB, lazy-loaded, graceful degradation) |
| LLM runtime | Ollama — optional (LLaVA for Tier 3 descriptions, Qwen2.5 for RAG claim verification) |
| Database | SQLite (via rusqlite in Rust) |

## Development Commands

```bash
# Start development (Tauri + SvelteKit hot reload)
make dev
# Or manually:
cd ui && npm run dev      # Terminal 1: SvelteKit dev server (port 1420)
cd src-tauri && cargo tauri dev  # Terminal 2: Tauri app

# Start the Python ML sidecar (port 8200)
cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200 --reload

# Build production app
make build

# Type check SvelteKit
cd ui && npx svelte-check

# Check Rust compilation
cd src-tauri && cargo check

# Run Rust tests (183 tests)
cd src-tauri && cargo test

# Run Rust linter
cd src-tauri && cargo clippy -- -D warnings

# Run Python sidecar tests (292 tests; 3 skipped without ffprobe, 14 CLIP skipped when open_clip unavailable)
cd sidecar && python -m pytest tests/ -v

# Run Playwright e2e tests (104 tests)
cd ui && npx playwright test

# Run SvelteKit type check
cd ui && npx svelte-check
```

## Project Structure

```
juralabs/
├── src-tauri/           # Tauri v2 Rust backend
│   ├── src/
│   │   ├── lib.rs       # Tauri commands, VerificationResult, verify pipeline
│   │   ├── c2pa.rs      # C2PA signing, verification, manifest reading
│   │   ├── db.rs        # SQLite database (assets, fingerprints, verifications, audit)
│   │   ├── exif_anomaly.rs  # EXIF anomaly detection + trust scoring
│   │   ├── fingerprint.rs   # Perceptual hashing (aHash, dHash, pHash)
│   │   ├── format_router.rs # MIME detection + content type routing
│   │   ├── metadata.rs      # EXIF metadata extraction
│   │   ├── watermark.rs     # DWT-DCT-SVD invisible watermarking (embed + extract)
│   │   └── sidecar.rs       # HTTP client for Python ML sidecar
│   ├── Cargo.toml       # Rust dependencies
│   └── tauri.conf.json  # Tauri app configuration
├── ui/                  # SvelteKit frontend
│   ├── src/routes/      # Page routes (protect/, verify/, settings/)
│   ├── src/lib/
│   │   ├── components/  # LogoMark, VerdictSummary, OnboardingOverlay,
│   │   │                #   MethodologyPanel, InspectionChecklist,
│   │   │                #   SignalAgreement
│   │   ├── types.ts     # TypeScript interfaces mirroring Rust structs
│   │   ├── api.ts       # Tauri IPC wrapper with browser mock fallback
│   │   ├── pdf.ts       # Trust report PDF generation
│   │   ├── zip.ts       # Case export ZIP generation
│   │   └── stores/      # Svelte stores (deployment profiles)
│   ├── tests/           # Playwright e2e tests (110 tests)
│   └── package.json     # Node dependencies
├── sidecar/             # Python ML sidecar (FastAPI, port 8200)
│   ├── app/api/         # FastAPI routers (health, forensics)
│   ├── app/services/    # ELA, noise, copy-move, deepfake, NPR,
│   │                    #   chromatic_aberration, jpeg_ghost,
│   │                    #   segmented_ela, shadow_consistency,
│   │                    #   colour_temperature, splice_boundary,
│   │                    #   clip_detector, claim_checker,
│   │                    #   watermark, video_metadata,
│   │                    #   audio_metadata, video_frames,
│   │                    #   video_deepfake
│   ├── app/api/         # FastAPI routers (health, forensics, video, audio)
│   ├── app/models/      # Pydantic schemas
│   ├── tests/           # pytest test suite (308 tests; 5 skipped without ffprobe/whisper)
│   ├── main.py          # FastAPI app entry point
│   └── requirements.txt # Python dependencies
├── docs/                # Documentation
├── PROJECT_SPEC.md      # Full project specification
├── CHANGELOG.md         # Development progress log
└── CLAUDE.md            # This file
```

## Key Files

- **Project spec**: `PROJECT_SPEC.md` — objectives, KPIs, phased delivery
- **Changelog**: `CHANGELOG.md` — development progress by phase and sprint
- **Architecture**: `docs/ARCHITECTURE.md` — detailed technical architecture
- **Brand**: `docs/BRAND_GUIDELINES.md` — visual identity, colour palette
- **Tauri config**: `src-tauri/tauri.conf.json` — app name, window, permissions
- **Rust entry**: `src-tauri/src/lib.rs` — Tauri commands, verify pipeline, VerificationResult
- **Watermark module**: `src-tauri/src/watermark.rs` — DWT-DCT-SVD invisible watermarking; `embed_watermark_asset` and `extract_watermark_from_path` Tauri commands
- **Sidecar client**: `src-tauri/src/sidecar.rs` — HTTP client for Python ML sidecar
- **Frontend types**: `ui/src/lib/types.ts` — TypeScript interfaces mirroring Rust structs
- **Frontend API**: `ui/src/lib/api.ts` — Tauri IPC wrapper with browser mock fallback
- **Verify page**: `ui/src/routes/verify/+page.svelte` — forensic analysis UI
- **Frontend entry**: `ui/src/routes/+layout.svelte` — root layout, navigation
- **Verdict component**: `ui/src/lib/components/VerdictSummary.svelte` — three-way verdict with confidence badge
- **Checklist component**: `ui/src/lib/components/InspectionChecklist.svelte` — 8-item manual visual inspection guide
- **Signal component**: `ui/src/lib/components/SignalAgreement.svelte` — per-detector agreement/disagreement dashboard
- **Sidecar entry**: `sidecar/main.py` — FastAPI application
- **NPR service**: `sidecar/app/services/npr.py` — neighbouring pixel relationship analysis
- **Chromatic aberration**: `sidecar/app/services/chromatic_aberration.py` — radial lens CA pattern detection
- **JPEG ghost**: `sidecar/app/services/jpeg_ghost.py` — double compression splice/composite analysis
- **CLIP detector**: `sidecar/app/services/clip_detector.py` — zero-shot AI/authentic classification via open_clip ViT-B/32
- **Claim checker**: `sidecar/app/services/claim_checker.py` — RAG claim verification via Ollama Qwen2.5
- **Segmented ELA**: `sidecar/app/services/segmented_ela.py` — 8x8 grid regional ELA with cluster detection
- **Shadow consistency**: `sidecar/app/services/shadow_consistency.py` — gradient-based light direction per region
- **Colour temperature**: `sidecar/app/services/colour_temperature.py` — CIELAB colour space segmentation
- **Splice boundary**: `sidecar/app/services/splice_boundary.py` — three-signal edge analysis (JPEG grid + noise + feathering)
- **Trained classifier**: `models/deepfake_classifier.joblib` — GBM classifier on 80-feature vector (AUC 0.945)
- **Training pipeline**: `scripts/train_classifier.py` — feature extraction + GBM training + CV evaluation
- **Corpus builder**: `scripts/build_corpus.py` — authentic press photo downloader (Guardian)
- **Corpus expander**: `scripts/expand_corpus.py` — HuggingFace + COCO dataset downloader
- **Calibration pipeline**: `scripts/calibrate.py` — batch detector evaluation + threshold recommendations
- **Logo component**: `ui/src/lib/components/LogoMark.svelte` — eye logo mark SVG
- **Sprint plan**: `docs/sprint-plans/sprint-region-forensics.md` — Sprint 9 region-based forensics plan
- **Watermark service**: `sidecar/app/services/watermark.py` — Python DWT-DCT-SVD watermark embed/extract via imwatermark
- **Video metadata service**: `sidecar/app/services/video_metadata.py` — FFmpeg/ffprobe video codec, resolution, FPS, duration, audio track info
- **Audio metadata service**: `sidecar/app/services/audio_metadata.py` — FFmpeg/ffprobe audio codec, sample rate, channels, bitrate
- **Video frames service**: `sidecar/app/services/video_frames.py` — evenly-spaced frame thumbnail extraction as base64 JPEG
- **Video deepfake service**: `sidecar/app/services/video_deepfake.py` — per-frame AI detection with temporal consistency signals; `POST /forensics/video/deepfake` endpoint (120 s timeout)
- **Security audit report**: `docs/security-audit-report.md` — full audit findings (3 critical, 6 high, 7 medium, 5 low) and remediation status
- **Blob utility**: `ui/src/lib/blob.ts` — base64-to-blob URL converter with `createBlobTracker` for CSP-safe image rendering and memory management
- **Sprint 15-to-v1.0 plan**: `docs/sprint-plans/sprint-15-to-v1.0-plan.md` — 6-sprint roadmap to v1.0 (Sprints 15–20, targeting 27 Jun 2026)
- **CI/CD workflows**: `.github/workflows/` — CI (Rust + Python + Frontend with pip-audit), Release (4-platform matrix), Dependabot

## Design Principles

1. **Local-first**: All processing on-device. No cloud calls. No telemetry.
2. **Rust for core**: C2PA, hashing, EXIF analysis, file I/O in Rust for performance.
3. **Python for ML**: Image forensics, deepfake detection, RAG in Python sidecar.
4. **Graceful degradation**: Sidecar offline = forensics skipped, app still works.
5. **Tiered AI**: Core features work without AI. CLIP tagging is optional download. Ollama is optional enhancement.
6. **SvelteKit for UI**: SPA mode via static adapter for Tauri webview. Svelte 5 runes ($state, $derived).

## Current Status

**Phase 1 (Weeks 1-12)**: Complete — PROTECT + VERIFY MVP with C2PA signing, perceptual fingerprinting, EXIF anomaly detection, format routing, SQLite database, dashboard, dark-mode UI.

**Phase 2 (Weeks 13-18)**: Complete — Python ML sidecar with ELA, noise analysis, copy-move detection, deepfake detection. Rust sidecar client. URL verification. Full verify pipeline with trust scoring.

**Phase 2 (Weeks 19-20+)**: Complete — Eight sprints of detection improvement work. Three-way verdict (authentic/inconclusive/synthetic), codec-aware thresholds, scene complexity weighting, EXIF-informed scoring. Three new forensic detectors: NPR, chromatic aberration, JPEG ghost. Four investigation modes (Quick/Standard/Deep/Archival). RAG claim checker via Ollama Qwen2.5. CLIP ViT-B/32 zero-shot detector (optional). False positive reporting with SQLite storage. New UI components: VerdictSummary, InspectionChecklist, SignalAgreement.

**Sprint 9 (Week 21)**: Complete — Region-based forensic analysis for composite image detection. Four new detectors: segmented ELA (8x8 grid), shadow consistency (gradient-weighted light direction), colour temperature (CIELAB segmentation), splice boundary (three-signal edge analysis). Trust scoring with composite amplification cap (0.55 when 2+ regional detectors fire). Region Analysis section on verify page. Validated against known composite image (3/4 detectors flagged).

**UI Redesign (Week 21)**: Complete — Rebrand to "Jura Trace" with Sanctuary theme. Warm colour palette (#1E2128 bg, #EDEAE4 text, #5A85B5 accent). Eye logo mark. Editorial layout with Georgia serif headings, 900px content width, earth-line dividers, narrative chapters. Mobile hamburger menu, skip navigation, 44px touch targets. Responsive asset table. Playwright e2e test harness (92 tests). Lighthouse: 97% accessibility, 100% best practices.

**Sprint 10 (Week 22)**: Complete — Trained GBM classifier for AI image detection. 80-feature vector extracted from existing deepfake pipeline, trained on 545-image corpus (326 authentic + 219 AI-generated). Cross-validation AUC-ROC 0.945. Detection rate: 68% (13/19 AI images flagged, all 12 Gemini PNGs caught). Authentic FP rate: 14%. Classifier blends with heuristic score (35/65 split). Graceful degradation when model absent. Calibration pipeline and corpus builder scripts.

**Sprint 11 (Phase 3)**: Complete — Invisible frequency-domain watermarking via DWT-DCT-SVD. Rust `watermark.rs` with `blind_watermark` crate. Three strength levels (Low ~48 dB / Medium ~42 dB / High ~36 dB). 128-bit UUID payload survives JPEG Q70+, resize, and 30% crop. Python sidecar watermark service (`imwatermark`). `POST /forensics/watermark/embed` and `/extract` endpoints. Protect page watermark UI with institution name and strength selector. CI/CD: GitHub Actions CI, Release workflow (4-platform matrix), Dependabot.

**Sprint 12 (Phase 3)**: Complete — Watermark extraction wired into verify pipeline with detection panel (institution name, confidence). Video metadata via FFmpeg/ffprobe — codec, resolution, FPS, duration, audio info. `POST /video/metadata` endpoint. C2PA signing extended to `video/mp4` and `video/quicktime`. Batch watermarking preparation.

**Sprint 13 (Phase 3)**: Complete — Video frame extraction (evenly-spaced thumbnails as base64 JPEG). `POST /video/frames` endpoint. Frame thumbnail strip on verify page. Audio metadata via FFmpeg/ffprobe — codec, sample rate, channels, bitrate. `POST /audio/metadata` endpoint. C2PA signing extended to `audio/wav` and `audio/mpeg`. Protect page video/audio metadata display. FFmpeg availability detection with graceful degradation.

**Sprint 14 (Phase 3)**: Complete — Video deepfake analysis: per-frame AI detection using the existing deepfake pipeline across three analysis modes (standard 6 frames ~12 s, deep 20 frames ~40 s, archival 40 frames ~80 s). Temporal consistency signals: noise drift, spectral drift, LBP drift. Aggregation formula: 0.5×mean + 0.3×max + 0.2×temporal. `deepfake.py` refactored to expose `perform_deepfake_detection_with_features()` for internal feature access. `POST /forensics/video/deepfake` endpoint with 120 s timeout. `FrameDeepfakeResult` and `VideoDeepfakeResult` Rust structs with full sidecar client wiring and Tauri command. Verify page frame timeline with coloured score badges and aggregate verdict. Batch watermarking UI on the Protect page: "Watermark All Images" button with progress bar, cancel, and completion summary.

**Security fixes (Sprint 14)**: Full security audit (`docs/security-audit-report.md`, 3 critical / 6 high / 7 medium / 5 low). Three critical issues remediated: SSRF prevention in `verify_url` (URL validation, loopback and private network blocking via `url` crate); scoped filesystem capability restricted to user directories only; CSP `connect-src` pinned to `127.0.0.1:8200` and `127.0.0.1:11434` only.

**Sprint 15 (Phase 3)**: Complete — "Hardened & Heard". All 7 MEDIUM security issues resolved: sidecar API key authentication via `X-Jura-API-Key` header (MEDIUM-1); audit log SHA-256 hash chain with `verify_audit_chain` integrity check (MEDIUM-2); CSP `data:` removal — all base64 image src attributes converted to `blob:` URLs via `createBlobTracker` utility (MEDIUM-3); URL query-string redaction in logging (MEDIUM-5); sidecar temp file cleanup on exception paths (MEDIUM-6); Python dependencies pinned with `requirements.lock` and CI updated with `pip-audit` (MEDIUM-7). Two LOW items cleared: `withGlobalTauri: false` in tauri.conf.json (LOW-1); FastAPI `/docs` and `/redoc` disabled in production (LOW-3). Audio/video transcription via faster-whisper wired into verify pipeline with `TranscriptionResult` struct and transcript panel in UI. Transcription text fed into RAG claim checker. Parallel sidecar calls via `tokio::join!` (ELA + deepfake + watermark concurrent). Frame accordion expand/collapse on video deepfake timeline with per-frame signals, classifier score, and heatmap. Sprint 15-to-v1.0 release plan document (`docs/sprint-plans/sprint-15-to-v1.0-plan.md`).

**Test counts**: 190 Rust tests, 308 Python tests (+5 skipped without ffprobe/whisper, +14 CLIP skipped when open_clip unavailable), 110 Playwright e2e tests, 179 SvelteKit files with 0 svelte-check errors, clippy clean.

## British Spelling

- **User-facing text**: British spelling (Organisation, Colour, Catalogue)
- **Code**: American spelling (organization, color, catalog) for framework consistency

## Brand Identity — Sanctuary Theme

- **Name**: Jura Trace (formerly Jura Archive)
- **Logo**: Eye mark — concentric circles (lapis outer, cream iris, dark pupil)
- **Palette**: Warm mineral tones — Obsidian (#1E2128), Graphite (#272B34), Quartz (#EDEAE4), Flint (#78756D / #9B9890), Lapis (#376399 / #5A85B5), Malachite (#5B8A5F), Amber, Cinnabar
- **Metaphor**: Geology — permanence, layers, provenance — with human-centred warmth
- **Tagline**: "Know What's Real"
- **Philosophy**: "Keep people at the heart of every decision. Use technology to support and guide, not to take over."
- **Theme**: Sanctuary — warm dark backgrounds, cream text, Georgia serif headings, editorial layout (900px width), earth-line gradient dividers, generous whitespace
- **Design**: System fonts, no emojis, WCAG 2.2 AA, Lighthouse 97% accessibility

## Relationship to ROOTED

Jura Trace is a **sibling product**, not a fork. Shares Juralabs' philosophy and some frontend patterns (SvelteKit, Tailwind, dark mode) but has its own codebase, brand, tech stack (Tauri/Rust), and release cycle. Both use nature-inspired design — ROOTED through organic/growth imagery, Jura Trace through geological/mineral imagery with a warm, editorial aesthetic.
