# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Jura Labs Ecosystem

Jura Trace is one of two products built by **Jura Labs** (UK Community Interest Company — Companies House 17117467, registered 25 March 2026). The full technology stack, infrastructure, and strategic context are documented in:

- **Tech stack reference**: `../jura-labs-docs/JURA-LABS-TECH-STACK-UPDATED.md`
- **Project management**: [Plane workspace](https://app.plane.so/jura-labs/) — connected to Claude Code via MCP (user scope)
- **Sister product**: ROOTED (carbon emissions guidance) — see `../ecoadvisor/`
- **Shared docs**: `../jura-labs-docs/`

## Project Overview

**Jura Trace** is a local-first desktop application for content verification and protection. In a world of synthetic media, verification matters. It helps cultural institutions protect their digital assets from unauthorised AI extraction, and helps communities verify content authenticity.

**Developed by**: Juralabs Community Interest Company (UK) — https://juralabs.org
**Licence**: PolyForm Noncommercial 1.0.0
**Current Version**: 0.9.0-rc.9 (Phase A — v1.0 blocked on pilot testing feedback only)
**Source repo**: `juralabs/jura-archive` (private)
**Release repo**: `juralabs/jura-trace` (public — installers only, no source)
**Windows signing**: Azure Trusted Signing (certificate ID a7e35def-628b-4980-8785-2e535f709418)

## Core Architecture

Four-layer local-first stack. See `docs/ARCHITECTURE.md` for the full diagram and data flow.

1. **Tauri v2 desktop shell** (Rust) — macOS / Windows / Linux.
2. **SvelteKit 5 frontend** on port 1420 — tabs: Protect, Verify, Monitor, Settings, Help. Communicates with Rust via Tauri IPC.
3. **Rust core engine** — C2PA signing/verification, perceptual fingerprinting, EXIF anomaly + MakerNote authenticity, format routing, DWT-DCT-SVD watermarking, video/audio metadata, SQLite database, Axum REST API on port 8300.
4. **Python ML sidecar** (FastAPI) on port 8200 — image forensics (ELA, noise, copy-move, NPR, chromatic aberration, JPEG ghost, segmented ELA, shadow consistency, colour temperature, splice boundary), GBM deepfake classifier, CLIP ViT-B/32 + UnivFD zero-shot AI detection, RAG claim checker, watermark embed/extract, video frame extraction, video deepfake (per-frame + temporal), audio/video transcription via faster-whisper.
5. **Ollama** on port 11434 (optional) — LLaVA for Tier 3 descriptions, Qwen2.5 for RAG claim verification.

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

### Disk pressure — periodic cargo clean

`src-tauri/target/` grows steadily during active Rust development and can
reach 15 GB or more with incremental build artefacts. On laptops with
tight disk budgets this will fill the root filesystem and break subsequent
`cargo check` / `cargo test` / `cargo tauri dev` runs with `ENOSPC` (no
space left on device). It can also fail non-cargo writes — mid-session
disk exhaustion has previously taken out Claude Code's own scratch
directory and the editor's staging area for in-flight file edits.

**Practice**: run `cargo clean --manifest-path src-tauri/Cargo.toml`
whenever free space on `/` drops below ~5 GB. It frees the target dir
cleanly; the next `cargo check` takes ~1–2 minutes to rehydrate caches
and ~10 minutes for a cold full rebuild. Also delete `sidecar/build/`
(PyInstaller output, not tracked) which can add another 200–300 MB.

Check disk usage with `df -h /` and target size with
`du -sh src-tauri/target`.

## Project Structure

Top-level layout (browse directories directly for file listings):

- `src-tauri/` — Tauri v2 Rust backend. Core modules in `src/`: `lib.rs` (entry), `c2pa.rs`, `db.rs`, `exif_anomaly.rs`, `fingerprint.rs`, `format_router.rs`, `metadata.rs`, `watermark.rs`, `sidecar.rs`, `error.rs`, `sun_position.rs`, `api/` (REST).
- `ui/` — SvelteKit 5 frontend (SPA/static adapter). Routes in `src/routes/` (protect, verify, monitor, settings, help); shared code in `src/lib/` (`types.ts`, `api.ts`, `pdf.ts`, `zip.ts`, `blob.ts`, `components/`, `stores/`); Playwright e2e in `tests/`.
- `sidecar/` — Python 3.13 FastAPI ML sidecar on port 8200. Routers in `app/api/`, detection/forensics logic in `app/services/`, Pydantic schemas in `app/models/`, pytest suite in `tests/`. `main.py` entry, `requirements.txt` / `requirements-ci.txt` / `requirements.lock`, `jura-sidecar.spec` for PyInstaller.
- `scripts/` — training, corpus, and calibration scripts; `scripts/agents/` for corpus crawl/protect/verify pipeline.
- `models/` — trained model artefacts (on external USB during development).
- `docs/` — all documentation (sprint plans, install guides, pilot testing, user guides, decisions, methodology, tier/strategy).
- `.github/workflows/` — CI, Release (4-platform matrix), Dependabot.
- Root: `PROJECT_SPEC.md`, `CHANGELOG.md`, `CLAUDE.md`, `Makefile`.

## Two-Repo Release Architecture

Jura Trace uses a split-repo model to keep source code private while distributing installers publicly:

| Repo | Visibility | Purpose |
|------|-----------|---------|
| `juralabs/jura-archive` | Private | Source code, CI, development, all branches and tags |
| `juralabs/jura-trace` | Public | Release installers only — no source code |

**How it works:**
1. Tags are pushed to `juralabs/jura-archive` (the source repo where the workflow lives).
2. The release workflow runs in `jura-archive` using the default `GITHUB_TOKEN` only for checkout.
3. All GitHub Releases API calls (create, upload assets, publish) target `juralabs/jura-trace` via the `RELEASE_PAT` secret (a PAT with `repo` scope on `jura-trace`).
4. `tauri-action` is called without `releaseId` on all platforms — this prevents it from uploading to `jura-archive`. Assets are uploaded manually via `gh release upload --repo juralabs/jura-trace` instead.
5. The Tauri auto-updater endpoint in `tauri.conf.json` also points to `juralabs/jura-trace` so end-user update checks resolve against the public repo.

**Required secrets in `juralabs/jura-archive`:**
- `RELEASE_PAT` — GitHub PAT with `repo` scope on `juralabs/jura-trace`
- `GITHUB_TOKEN` — standard Actions token (used only for source checkout)

**CI/CD workflows**: `.github/workflows/` — CI (Rust + Python + Frontend with pip-audit), Release (4-platform matrix), Dependabot

## Key Files

Top-level entry points and non-obvious files. Sidecar services live under `sidecar/app/services/`, UI components under `ui/src/lib/components/`, help pages under `ui/src/routes/help/` — browse those directories directly rather than tracking individual files here.

- **Reference docs**: `PROJECT_SPEC.md`, `CHANGELOG.md`, `docs/ARCHITECTURE.md`, `docs/BRAND_GUIDELINES.md`
- **Rust entry**: `src-tauri/src/lib.rs` — Tauri commands, verify pipeline, `VerificationResult`
- **Sidecar client**: `src-tauri/src/sidecar.rs` — HTTP client for Python sidecar
- **REST API module**: `src-tauri/src/api/` — Axum REST API on port 8300 (routes, types, auth, rate_limit, error)
- **Error module**: `src-tauri/src/error.rs` — `AppError` enum with structured IPC serialisation `{ code, message }`
- **Tauri config**: `src-tauri/tauri.conf.json` — app name, window, permissions, auto-updater endpoint
- **Sidecar entry**: `sidecar/main.py` — FastAPI application
- **CI requirements** (non-obvious): `sidecar/requirements-ci.txt` — the file CI actually uses for PyInstaller builds (excludes torch/whisper/chromadb). Separate from `requirements.txt`.
- **PyInstaller spec**: `sidecar/jura-sidecar.spec` — parameterised for macOS/Windows/Linux
- **Frontend contract**: `ui/src/lib/types.ts` (mirrors Rust structs) and `ui/src/lib/api.ts` (Tauri IPC wrapper with browser mock fallback)
- **Main UI**: `ui/src/routes/verify/+page.svelte`, `ui/src/routes/protect/+page.svelte`, `ui/src/routes/+layout.svelte`
- **Models** (on external USB; paths are where sidecar loads them from):
  - `models/deepfake_classifier.joblib` — GBM v4, 84 features, AUC 0.9868, FP 4.54%
  - `models/univfd_probe.joblib` — UnivFD v9, LogReg on CLIP ViT-B/32, AUC 0.9933, FP 4.12%
- **Training scripts**: `scripts/train_classifier.py`, `scripts/train_univfd_probe.py` (`--C` for regularisation), `scripts/agents/` (corpus crawl/protect/verify/validate pipeline)
- **CI/CD**: `.github/workflows/` — CI (Rust + Python + Frontend with pip-audit), Release (4-platform matrix, workflow_dispatch enabled, Linux paused), Dependabot
- **macOS entitlements**: `src-tauri/Entitlements.plist`
- **Sprint plans / strategy**: `docs/sprint-plans/phase-a-plan.md`, `docs/tried-compliance-roadmap.md`, `docs/strategic-pivot-assessment.md`, `docs/tier-structure-decision.md`
- **User/install docs**: `docs/user-guide/`, `docs/install-guides/`, `docs/pilot-testing/`

## Design Principles

1. **Local-first**: All processing on-device. No cloud calls. No telemetry.
2. **Rust for core**: C2PA, hashing, EXIF analysis, file I/O in Rust for performance.
3. **Python for ML**: Image forensics, deepfake detection, RAG in Python sidecar.
4. **Graceful degradation**: Sidecar offline = forensics skipped, app still works.
5. **Tiered AI**: Core features work without AI. CLIP tagging is optional download. Ollama is optional enhancement.
6. **SvelteKit for UI**: SPA mode via static adapter for Tauri webview. Svelte 5 runes ($state, $derived).

## Current Status

**Version**: 0.9.0-rc14 (Phase A — v1.0 blocked on pilot testing feedback only). Phases 1–3 complete. Sprint 28 tech-debt sweep closed 8 April 2026; Sprint 29 shipped 7 April 2026; Sprint 30 backlog sweep 9 April 2026 (SIFT copy-move, EXIF injection detection, FP telemetry Phase B review bundle, URL watchlist scheduler, Tauri race-condition fix, Experimental UI tag rollout, XMP AI-provenance detection). For full sprint-by-sprint history see `CHANGELOG.md` and git log.

**Models in production** (as of 7 April 2026):
- **GBM Deepfake Classifier v4** — 10,709 images (5,724 authentic + 4,985 AI), 84-feature vector, AUC-ROC 0.9868, authentic FP 4.54%, AI recall 92.52%, threshold 0.49. SHA-256 `2931f197cba6f376e85b1cbcfd584e6802f36e4fbf68ff00c83d61d4d655db18`.
- **UnivFD probe v9** — LogisticRegression on CLIP ViT-B/32 embeddings, 39,016 training samples (10,712 original + 32,142 platform-forwarded augmentation via Q=75/85/2× re-saves), AUC-ROC 0.9933, FP 4.12%, recall 95.70%. Platform-forwarded-specific AUC: plt75 0.9947, plt85 0.9949, plt2x 0.9937. DiffusionDB recall 67.6% → 97.3%, Civitai SFW 75.8% → 98.7%. Known trade-off: flux_dev 88.9% (−11.1 pp), sdxl_turbo 91.1% (−8.9 pp). SHA-256 `ed691b45cbe2903a7e0530fd0ec78ab91eef9f15133af4c1a5c8cf172086dacd`. Promoted 2026-04-12 from candidate trained 2026-04-11. Full validation in `docs/calibration/univfd-v9-platform-augmentation.md`.
- Training corpus: 6,571 authentic + 5,005 AI = 11,576 images total (stored on external USB).
- Camera FP after MakerNote authenticity bonus + `KNOWN_CAMERA_VENDORS` (36 vendors, 11 Global Majority brands): consumer (Pixel/iPhone) 8.81%, high-end (DJI/DSC) 10.32%. All AI generator families pass 100% recall except DALL-E 3 (91.4%), Civitai SFW (75.8%), DiffusionDB (67.6% — weakest, older SD).

**Detector lineup post-Sprint-28**: 12 automatic detectors (EXIF anomaly, C2PA, ELA, noise, copy-move, deepfake GBM+UnivFD ensemble, JPEG Ghost at 0.5× weight, segmented ELA, colour temperature, CLIP, watermark, video deepfake) plus 3 on-demand investigation tools (NPR, shadow consistency, splice boundary) that do not contribute to trust scoring. Chromatic aberration removed entirely (forensic audit 1/5). Diffusion artefact detector removed (superseded by UnivFD v8). Schema v6 `detectors_run` column persists the exact lineup per verification so PDF / ZIP renderers can distinguish "detector ran and returned null" from "detector not run in this build / mode". Full Sprint 28 audit trail in `docs/backlog.md` and project memory.

**EXIF anomaly detector (as of Sprint 30)**: the `exif_anomaly` detector now includes both an injection-detection suite (5 sub-checks: programmatic pipeline library in Software field, templated timestamps, integer-degree GPS, MakerNote absent on mandatory-vendor camera, iPhone sRGB mismatch — all landed in commit `d03f338`) and an XMP provenance suite (2 sub-checks: `xmp_ai_digital_source` for `Iptc4xmpExt:DigitalSourceType` values like `trainedAlgorithmicMedia`, and `xmp_ai_creator_tool` for known AI generators in `xmp:CreatorTool` — landed in commit `ce06f7f`). Both are High severity and flow through the existing `exif_anomaly` scoring path with no new detector ID.

**Known issues / carry-over from Sprint 29**:
- Track 2 partial: ~1,000 of target 1,200–1,400 Global Majority handset photos still outstanding.
- Track 4 validation set samples from training corpus — not a true held-out set.
- `demosaic_inter_channel_coherence` feature has DC-dominated sampling, needs iteration.
- Composite border detector: 21 candidates flagged on full corpus, HTML preview pending user review.
- **JPEG Ghost 0.5× weight calibration closed 2026-04-07** — Option 3 (synthetic CC-BY generator) executed; weight retained at 0.5, no code change. CASIA v2 **rejected 2026-04-11** as non-commercial-licensed and unusable for Jura Trace (commercial product); a commercial-cleared splice benchmark remains an open future need per `docs/calibration/s28-jpeg-ghost-weight.md` Section 6.

**Test counts**: 334 Rust lib tests, 375+ Python (47 sidecar deepfake), 164 Playwright e2e, 233 SvelteKit files with 0 svelte-check errors, clippy + fmt clean. Pre-commit hook now runs `cargo fmt`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `svelte-check` — hardened in commit `0912b14` after two Sprint 30 test-fixture regressions slipped past the lighter `cargo check` alone.

## Backlog

See `docs/backlog.md` for open items, UI audit deferrals, and resolved-archive. Track in Plane; that file is a lightweight mirror.

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
