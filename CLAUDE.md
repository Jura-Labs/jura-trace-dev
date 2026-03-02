# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

**Jura Archive** is a local-first desktop application for content protection and verification. It helps cultural institutions protect their digital assets from unauthorised AI extraction, and helps communities verify content authenticity.

**Developed by**: Juralabs Community Interest Company (UK) — https://juralabs.org
**Licence**: PolyForm Noncommercial 1.0.0
**Current Version**: 0.2.0-dev (Phase 2 active)

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
│  SQLite Database                        │
└────────────────┬────────────────────────┘
                 │ HTTP (localhost:8200)
┌────────────────▼────────────────────────┐
│      Python ML Sidecar (Port 8200)      │
│  ELA | Noise | Copy-Move | Deepfake     │
│  RAG Pipeline (Phase 2 Wk 19-20)       │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│    Ollama (Port 11434) — Optional       │
│  LLaVA (Tier 3 descriptions) | Qwen2.5 │
└─────────────────────────────────────────┘
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 (Rust) |
| Frontend | SvelteKit 5 + TailwindCSS (SPA mode, static adapter, Svelte 5 runes) |
| Core engine | Rust (c2pa-rs, rusqlite, image_hasher, reqwest) |
| Auto-catalogue | Tier 1: EXIF (Rust), Tier 2: CLIP/ONNX (optional), Tier 3: Ollama (optional) |
| ML sidecar | Python 3.13 + FastAPI (ELA, noise analysis, copy-move, deepfake detection) |
| LLM runtime | Ollama — optional (LLaVA for Tier 3 descriptions, Qwen2.5 for text) |
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

# Run Rust tests (84 tests)
cd src-tauri && cargo test

# Run Rust linter
cd src-tauri && cargo clippy -- -D warnings

# Run Python sidecar tests (35 tests)
cd sidecar && python -m pytest tests/ -v

# Run SvelteKit tests
cd ui && npm test
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
│   │   └── sidecar.rs       # HTTP client for Python ML sidecar
│   ├── Cargo.toml       # Rust dependencies
│   └── tauri.conf.json  # Tauri app configuration
├── ui/                  # SvelteKit frontend
│   ├── src/routes/      # Page routes (protect/, verify/, settings/)
│   ├── src/lib/         # Shared types, api, components
│   └── package.json     # Node dependencies
├── sidecar/             # Python ML sidecar (FastAPI, port 8200)
│   ├── app/api/         # FastAPI routers (health, forensics)
│   ├── app/services/    # ELA, noise analysis, copy-move, deepfake
│   ├── app/models/      # Pydantic schemas
│   ├── tests/           # pytest test suite
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
- **Sidecar client**: `src-tauri/src/sidecar.rs` — HTTP client for Python ML sidecar
- **Frontend types**: `ui/src/lib/types.ts` — TypeScript interfaces mirroring Rust structs
- **Frontend API**: `ui/src/lib/api.ts` — Tauri IPC wrapper with browser mock fallback
- **Verify page**: `ui/src/routes/verify/+page.svelte` — forensic analysis UI
- **Frontend entry**: `ui/src/routes/+layout.svelte` — root layout, navigation
- **Sidecar entry**: `sidecar/main.py` — FastAPI application

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

**Phase 2 (Weeks 19-20)**: Next — RAG pipeline for claim verification via Ollama.

**Test counts**: 84 Rust tests, 35 Python tests, 0 svelte-check errors, clippy clean.

## British Spelling

- **User-facing text**: British spelling (Organisation, Colour, Catalogue)
- **Code**: American spelling (organization, color, catalog) for framework consistency

## Brand Identity

- **Palette**: Cool mineral tones — Obsidian, Graphite, Lapis, Malachite, Amber, Cinnabar
- **Metaphor**: Geology — permanence, layers, provenance
- **Tagline**: "Know What's Real"
- **Design**: System fonts, no emojis, clean typography, WCAG 2.2 AA

## Relationship to ROOTED

Jura Archive is a **sibling product**, not a fork. Shares Juralabs' philosophy and some frontend patterns (SvelteKit, Tailwind, dark mode) but has its own codebase, brand, tech stack (Tauri/Rust), and release cycle.
