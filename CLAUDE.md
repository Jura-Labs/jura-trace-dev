# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

**Jura Archive** is a local-first desktop application for content protection and verification. It helps cultural institutions protect their digital assets from unauthorised AI extraction, and helps communities verify content authenticity.

**Developed by**: Juralabs Community Interest Company (UK) — https://juralabs.org
**Licence**: PolyForm Noncommercial 1.0.0
**Current Version**: 0.1.0-dev (pre-development)

## Core Architecture

```
┌─────────────────────────────────────────┐
│        Tauri v2 Desktop Shell (Rust)    │
│        macOS / Windows / Linux          │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│     SvelteKit Frontend (Port 5173)      │
│  PROTECT | VERIFY | MONITOR | SETTINGS  │
└────────────────┬────────────────────────┘
                 │ Tauri IPC
┌────────────────▼────────────────────────┐
│          Rust Core Engine               │
│  C2PA | Hash | Metadata | Watermark     │
│  Format Router | SQLite Database        │
└────────────────┬────────────────────────┘
                 │ Sidecar (Phase 2+)
┌────────────────▼────────────────────────┐
│      Python ML Sidecar (Port 8200)      │
│  Forensics | Deepfake | RAG Pipeline    │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│         Ollama (Port 11434)             │
│    LLaVA (vision) | Qwen2.5 (text)     │
└─────────────────────────────────────────┘
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 (Rust) |
| Frontend | SvelteKit + TailwindCSS (SPA mode, static adapter) |
| Core engine | Rust (c2pa-rs, rusqlite, image crate) |
| ML sidecar | Python + FastAPI (Phase 2+) |
| LLM runtime | Ollama (LLaVA for vision, Qwen2.5 for text) |
| Database | SQLite (via rusqlite in Rust) |

## Development Commands

```bash
# Start development (Tauri + SvelteKit hot reload)
make dev
# Or manually:
cd ui && npm run dev      # Terminal 1: SvelteKit dev server
cd src-tauri && cargo tauri dev  # Terminal 2: Tauri app

# Build production app
make build

# Type check SvelteKit
cd ui && npx svelte-check

# Check Rust compilation
cd src-tauri && cargo check

# Run Rust tests
cd src-tauri && cargo test

# Run SvelteKit tests
cd ui && npm test
```

## Project Structure

```
juralabs/
├── src-tauri/           # Tauri v2 Rust backend
│   ├── src/             # Rust source (main.rs, lib.rs, modules)
│   ├── Cargo.toml       # Rust dependencies
│   └── tauri.conf.json  # Tauri app configuration
├── ui/                  # SvelteKit frontend
│   ├── src/routes/      # Page routes (protect/, verify/, settings/)
│   ├── src/lib/         # Shared components, stores, utilities
│   └── package.json     # Node dependencies
├── sidecar/             # Python ML sidecar (Phase 2+)
├── data/                # Local data (SQLite, knowledge base)
├── docs/                # Documentation
├── PROJECT_SPEC.md      # Full project specification
└── CLAUDE.md            # This file
```

## Key Files

- **Project spec**: `PROJECT_SPEC.md` — objectives, KPIs, phased delivery
- **Architecture**: `docs/ARCHITECTURE.md` — detailed technical architecture
- **Brand**: `docs/BRAND_GUIDELINES.md` — visual identity, colour palette
- **Tauri config**: `src-tauri/tauri.conf.json` — app name, window, permissions
- **Rust entry**: `src-tauri/src/lib.rs` — Tauri commands, module declarations
- **Frontend entry**: `ui/src/routes/+layout.svelte` — root layout, navigation

## Design Principles

1. **Local-first**: All processing on-device. No cloud calls. No telemetry.
2. **Rust for core**: C2PA, hashing, watermarking, file I/O in Rust for performance.
3. **Python for ML**: Image forensics, deepfake detection, RAG in Python sidecar.
4. **SvelteKit for UI**: SPA mode via static adapter for Tauri webview.

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
