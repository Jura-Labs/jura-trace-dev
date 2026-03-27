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
**Current Version**: 0.9.0-rc.3 (Phase 3 — Sprint 20 in progress, v1.0 blocked on signing certs only)

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
│   ├── src/routes/      # Page routes (protect/, verify/, settings/, help/)
│   ├── src/lib/
│   │   ├── components/  # LogoMark, VerdictSummary, OnboardingOverlay,
│   │   │                #   MethodologyPanel, InspectionChecklist,
│   │   │                #   SignalAgreement, HelpSidebar,
│   │   │                #   ContextualHelpLink
│   │   ├── types.ts     # TypeScript interfaces mirroring Rust structs
│   │   ├── api.ts       # Tauri IPC wrapper with browser mock fallback
│   │   ├── pdf.ts       # Trust report PDF generation
│   │   ├── zip.ts       # Case export ZIP generation
│   │   └── stores/      # Svelte stores (deployment profiles)
│   ├── tests/           # Playwright e2e tests (164 tests)
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
- **Error module**: `src-tauri/src/error.rs` — `AppError` enum with structured IPC serialisation `{ code, message }`
- **Help layout**: `ui/src/routes/help/+layout.svelte` — help section sidebar + content layout
- **Methodology page**: `ui/src/routes/help/methodology/+page.svelte` — all 16 detectors explained, trust scoring, limitations
- **Glossary**: `ui/src/routes/help/glossary/+page.svelte` — 34 terms A-Z with sticky alphabet jump bar
- **Persona guides**: `ui/src/routes/help/personas/+page.svelte` — museum staff, journalists, creators, researchers workflows
- **HelpSidebar**: `ui/src/lib/components/HelpSidebar.svelte` — help section navigation component
- **ContextualHelpLink**: `ui/src/lib/components/ContextualHelpLink.svelte` — inline `?` help link component
- **PyInstaller spec**: `sidecar/jura-sidecar.spec` — PyInstaller spec for frozen sidecar binary (macOS arm64)
- **PyInstaller findings**: `docs/pyinstaller-spike-findings.md` — spike results, GO verdict for Sprint 18
- **Feature scoping**: `docs/feature-scoping/online-monitoring-and-help-system.md` — online monitoring 3-layer architecture, paid tier structure
- **Sprint 17 plan**: `docs/sprint-plans/sprint-17-plan.md` — quality floor and deployment readiness
- **Sprint 18 plan**: `docs/sprint-plans/sprint-18-plan.md` — unsigned platform installers, frozen sidecar
- **Entitlements**: `src-tauri/Entitlements.plist` — macOS hardened runtime capabilities
- **Sidecar binaries**: `src-tauri/binaries/` — platform-specific sidecar stubs (replaced by CI with PyInstaller output)
- **MONITOR schema**: `src-tauri/migrations/003_monitor_tables.sql` — monitor_urls + monitor_events table design
- **Schema design doc**: `docs/monitor-schema-design.md` — MONITOR table rationale, query patterns, case management
- **Install guides**: `docs/install-guides/` — macOS unsigned, Windows unsigned, Linux requirements
- **Pilot test script**: `docs/pilot-testing/test-script.md` — 30-minute structured test session
- **Linux smoke test**: `docs/pilot-testing/linux-smoke-test.md` — AppImage verification checklist
- **SetupWizard**: `ui/src/lib/components/SetupWizard.svelte` — 5-step first-launch service check and model download flow
- **Security pen test**: `docs/security-pen-test-s19.md` — OWASP self-audit (4 HIGH, 4 MEDIUM fixed)
- **User guides**: `docs/user-guide/` — getting-started.md, protect-guide.md, verify-guide.md (standalone website docs)
- **Quick Start cards**: `docs/install-guides/quickstart-{macos,windows,linux}.md` — 1-page per platform
- **SHA-256 verification**: `docs/install-guides/verify-downloads.md` — download integrity guide
- **Windows IT deployment**: `docs/install-guides/windows-it-deployment.md` — Intune, GPO, silent install, FFmpeg
- **Tier structure**: `docs/tier-structure-decision.md` — Community/Professional/Team/Enterprise pricing and features
- **Tier comparison**: `docs/tier-comparison-wiki.md` — full feature matrix across all tiers
- **Deployment design**: `docs/deployment-experience-design.md` — FFmpeg bundling, Ollama, model downloads, setup wizard
- **Strategic pivot**: `docs/strategic-pivot-assessment.md` — verification-first messaging, post-v1.0 roadmap
- **Phase A plan**: `docs/sprint-plans/phase-a-plan.md` — FP reduction, API wrapper, reports, versioning (July-Aug 2026)

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

**Sprint 16 (Phase 3)**: Complete — "Fast & Stable". Performance optimisation, pipeline parallelism, error handling improvements. `AppError` enum with structured IPC serialisation (`{ code, message }`) in `src-tauri/src/error.rs`. LOW security remediations completed.

**Sprint 17 (Phase 3)**: Complete — "Quality Floor & Deployment Readiness". IPC error propagation: `parseAppError` in `api.ts`, `AppErrorResponse` in `types.ts`, `setError()` upgraded to use structured error codes with string-sniff fallback. Tiered sidecar error banners: dev mode shows technical details, production shows user-friendly messages. Test hooks (`__juraSetVerifyResult`, `__juraSetVerifyError`) gated behind `import.meta.env.DEV`. Database path configurability: three-source priority resolution (`JURA_DB_PATH` env > `config.json` > default), `get_db_path`/`set_db_path` Tauri commands with atomic copy + SQLite integrity check, Settings page folder picker. Frame dedup rolling buffer (buffer_size=5) in video deepfake pipeline. Source protection privacy warning in Investigate Further panel. PDF trust scoring: `document_trust()` helper (C2PA valid 0.82, invalid 0.25, none 0.50) with limited-analysis info banner. DEPLOYMENT.md updated for Phase 3. PyInstaller sidecar bundling spike: GO for Sprint 18, 315 MB binary, all core endpoints work, 2 path fixes needed. In-app help documentation system: `/help` route with sidebar navigation, 7 content pages (Protect guide, Verify guide, Methodology transparency with 16 detectors, Glossary with 34 terms, Persona guides for 4 user types, Monitor and Settings stubs), `HelpSidebar` and `ContextualHelpLink` components, contextual `?` links on verify/protect/monitor pages.

**Sprint 18 (Phase 3)**: Complete — "Platform Installers & Deployment Readiness". PyInstaller cross-platform sidecar: `JURA_MODELS_DIR` env var override in `deepfake.py`, spec parameterised for macOS/Windows/Linux (auto-detect target_arch, platform-conditional UPX excludes). Tauri sidecar auto-launch: `externalBin` config, `tauri-plugin-shell` spawn with exponential-backoff health polling, clean process kill on `RunEvent::Exit`, `AppState.sidecar_process` field. Platform icons: `.icns` (macOS), `.ico` (Windows), full PNG set. macOS `Entitlements.plist` with network.client, files.user-selected, allow-unsigned-executable-memory, disable-library-validation. macOS DMG builds successfully (15 MB unsigned, `minimumSystemVersion: "13.0"`). Linux font fallback: DejaVu Serif + Noto Serif added to all font stacks, 38 inline font-family declarations refactored to CSS class. Settings path separator: `@tauri-apps/api/path` join replaces fragile heuristic. Linux bundle: `category: "Utility"`, deb depends (libwebkit2gtk-4.1-0, libgtk-3-0, libayatana). GitHub Actions release workflow: 4-platform matrix with PyInstaller sidecar build per platform, changelog extraction, pip cache. Monitor AI training disclaimer: permanent lapis info banner on Monitor tab. MONITOR SQLite schema spike: `monitor_urls` and `monitor_events` tables designed with case management fields. Three platform install guides (macOS/Windows/Linux unsigned). Pilot testing script (30-min structured session, 4 persona variants). Linux smoke test checklist. Feature scoping: online monitoring 3-layer architecture, paid tier structure (Flint/Stratum/Geode/Bedrock), 11 backlog items created in Plane.

**Sprint 19 (Phase 3)**: In progress — "Release Candidate". WCAG 2.2 AA accessibility audit: 14 issues fixed across 5 files (tabpanel ARIA, URL input label, focus-visible rings, external link new-tab announcements, dialog heading hierarchy). Monitor and Settings help guides: full content replacing stubs (8 sections each). MONITOR CRUD wired into `db.rs`: `monitor_urls` and `monitor_events` tables in `init_schema()`, `MonitorUrl`/`MonitorEvent` structs, 5 CRUD functions (`add_monitor_url`, `remove_monitor_url`, `list_monitor_urls`, `get_monitor_events`, `update_case_status`), 4 new tests. RAG knowledge base expanded: 6 documents, ~314 lines, ~150 passages (was 4 docs, 158 lines) — added video forensics, digital rights/cultural heritage domains. C2PA AI declaration detection: `detect_ai_from_assertions()` scans C2PA assertions for `trainedAlgorithmicMedia` digitalSourceType, AI keywords, and known generator names. Trust scoring: C2PA declaring AI generation penalised -0.25 (images/video) or 0.10 trust (documents) — was previously rewarded. Verify page: amber provenance banner when C2PA confirms AI generation. Security pen test (OWASP self-audit): 4 HIGH fixed (path canonicalisation in c2pa/watermark/video commands, shell permissions removed), 4 MEDIUM fixed (CSP hardened, set_db_path extension allowlist, transcription size limit, audit timestamp precision). Corrupt/truncated file handling: zero-length and <12 byte guards in verify and import pipelines. Video analysis progress: phase labels, estimated time by mode, soft cancel button with Escape key. External user docs: getting-started.md, protect-guide.md, verify-guide.md. First-launch setup wizard: 5-step flow (sidecar health → FFmpeg → Whisper model → Ollama → Ready summary), persisted via localStorage. Deployment experience design: FFmpeg bundled, Ollama download button, Whisper/CLIP download on first use. Quick Start cards for all 3 platforms. SHA-256 verification guide + automated checksums in release workflow. Windows IT deployment appendix (Intune, GPO, silent install, PowerShell FFmpeg). Strategic pivot assessment: verification-first messaging, post-v1.0 Phase A/B roadmap. Tier structure decision: Community/Professional/Team/Enterprise with geological internal codenames. Enterprise AI API analysis: BYOK model defensible for Enterprise with Mistral/Claude/OpenAI. Phase A plan: FP reduction, API wrapper, report customisation, methodology versioning (4 sprints, July-August 2026).

MONITOR URL watchlist: 5 Tauri IPC commands (`add_monitor_url`, `remove_monitor_url`, `list_monitor_urls`, `get_monitor_events`, `update_monitor_case_status`), TypeScript types and API wrappers, full Monitor tab UI (add URL form, status badges, expandable event rows, case management). Auto-updater: `tauri-plugin-updater` wired with `latest.json` generation in release workflow, "Check for Updates" button in Settings. RC prep: version bumped to 0.9.0 across all configs, `cargo fmt` cleaned, video deepfake test flake fixed, audit log hash chain ordering fixed (rowid). RC readiness report produced.

Deepfake classifier retrained: AUC-ROC 1.0000 (was 0.945), FP rate 0% (was 14%). Format confound eliminated — classifier learned JPEG vs PNG, fixed by adding authentic PNGs to training corpus. Corpus expanded 548 → 709 images. Security pen test: all items remediated (0 open). Python dependencies pinned to exact versions.

**Sprint 20 (Phase 3)**: In progress — persona conversion features. Licence tier management: `LicenceTier` enum (Community/Professional/Team/Enterprise) persisted in config.json, Settings page "Your Plan" section with tier badge + pilot dropdown, non-blocking tier hints on Verify page. Analyst declaration modal: 4-field export modal (name, org, case ref, date) with localStorage persistence, header block in PDF reports. Raw signal scores in PDF: 7 core + 4 regional detector scores with float values, thresholds, Clean/Flagged status; methodology metadata block (mode, version, formula, detectors run). Metadata preservation statement: malachite confirmation panel on Protect page after C2PA signing. 10-persona comprehensive review conducted. Agent architecture overhaul: 10 personas (6 new professional), 3 new agents (ml-data-scientist, api-engineer, grant-writer), 6 agents updated. IS summary document, DPIA template, Berkeley Protocol help page for institutional procurement.

**Test counts**: 235 Rust tests, 308+ Python tests, 160+ Playwright e2e tests, 206 SvelteKit files with 0 svelte-check errors, clippy + fmt clean.

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

## Product Architecture (Post-v1.0)

Two products sharing a common detection engine:

```
                    SHARED CORE
                    (Rust library crate — extracted Phase A Sprint 23)
                   /                     \
    Jura Trace Desktop              Jura Check API (post-June gate)
    (Tauri v2 + SvelteKit)          (Axum, cloud-hosted)
    - Local-first, offline           - JWT + Redis
    - Python sidecar (local 8200)    - Python sidecar (container)
    - B2B tiers (5-tier)             - Consumer tiers (4-tier)
    - Port 8300 (local API)          - verify.juralabs.org
```

**Architecture decisions (27 March 2026):**
- Python sidecar stays — rewriting 21 ML services in Rust rejected (110-160 pts risk)
- Tauri desktop shell stays — the product ships in April/May 2026
- Two separate APIs: local desktop (API key) + hosted consumer (JWT/Redis) — decided at June gate
- Team tier retained — £79/seat/mo fills the £199→£6K gap
- Consumer tiers (Free/Personal/Family/Creator) scoped under Jura Check, not Jura Trace
- 14 personas total: 10 existing B2B + 4 new consumer (Ravi, Sarah M, Jordan, Priya)

## Relationship to ROOTED

Jura Trace is a **sibling product**, not a fork. Shares Juralabs' philosophy and some frontend patterns (SvelteKit, Tailwind, dark mode) but has its own codebase, brand, tech stack (Tauri/Rust), and release cycle. Both use nature-inspired design — ROOTED through organic/growth imagery, Jura Trace through geological/mineral imagery with a warm, editorial aesthetic.
