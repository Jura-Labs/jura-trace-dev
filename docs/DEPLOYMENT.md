# Deployment Guide

**Last updated**: 2 March 2026
**Phase**: 1 (MVP)

---

## Prerequisites

- macOS 13+ (Ventura), Windows 10+, or Ubuntu 22.04+
- 4 GB RAM minimum (8 GB recommended for Tier 2/3 AI features)
- 500 MB free disk space (excluding optional AI models)
- No external dependencies required for core functionality

---

## Installation

### macOS

1. Download the latest `.dmg` from the releases page.
2. Open the DMG and drag **Jura Archive** to your Applications folder.
3. On first launch, macOS may prompt you to allow the app — open System Settings > Privacy & Security and click "Open Anyway".

### Windows

> **Note**: Windows installers are planned for Phase 4 (Weeks 33-34).

### Linux

> **Note**: Linux AppImage packages are planned for Phase 4 (Weeks 33-34).

---

## First Run

1. Launch Jura Archive.
2. The application creates a local database in your system's app data directory — no configuration required.
3. Navigate to the **Protect** tab to begin importing files.
4. All core features (import, metadata extraction, C2PA signing, fingerprinting) work immediately — no additional setup needed.

---

## Auto-Catalogue Tiers

Jura Archive uses a three-tier cataloguing system. Each tier is independent and optional:

### Tier 1 — Metadata Extraction (always available)

Extracts EXIF, XMP, and IPTC metadata from imported files. Provides camera information, dates, GPS coordinates, copyright, dimensions, and software details. No additional setup required.

### Tier 2 — CLIP Subject Tagging (optional download, ~400MB)

On first use, the application offers to download the CLIP vision model (~400MB). Once downloaded, Jura Archive can automatically suggest subject tags from a curated museum vocabulary (based on ICONCLASS and Getty AAT taxonomies). Tags are generated locally in 50-200ms per image.

**Requirements**: ~600-800MB RAM during tagging, ~400MB disk space for the model.

### Tier 3 — Ollama Descriptions (optional enhancement)

For rich free-text descriptions, Jura Archive can connect to a locally running Ollama instance. This is the only feature that requires Ollama.

1. Install Ollama from https://ollama.com
2. Pull the required models:
   ```bash
   ollama pull llava
   ollama pull qwen2.5
   ```
3. Ensure Ollama is running before launching Jura Archive.
4. Jura Archive connects to Ollama on `localhost:11434` — no additional configuration is needed.

**Requirements**: 8 GB+ RAM, ~4-8GB disk space for models.

The application clearly indicates which tiers are available in the Settings page.

---

## Data Storage

All data is stored locally:

| Data | Location |
|------|----------|
| Asset database | `<app-data-dir>/jura_archive.db` |
| Configuration | `<app-data-dir>/config.json` |
| CLIP model (Tier 2) | `<app-data-dir>/models/` |
| Ollama models (Tier 3) | `~/.ollama/models/` |

No data is sent to external servers. No telemetry is collected.
