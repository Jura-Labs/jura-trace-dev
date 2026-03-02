# Deployment Guide

**Last updated**: 2 March 2026
**Phase**: 1 (MVP)

---

## Prerequisites

- macOS 13+ (Ventura), Windows 10+, or Ubuntu 22.04+
- 4 GB RAM minimum (8 GB recommended for AI features)
- 500 MB free disk space (excluding Ollama models)
- Ollama installed for auto-catalogue and AI features (optional for core PROTECT/VERIFY)

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

---

## Ollama Setup

Ollama is required for the auto-catalogue feature (AI-generated descriptions and tags). Core PROTECT and VERIFY features work without it.

1. Install Ollama from https://ollama.com
2. Pull the required models:
   ```bash
   ollama pull llava
   ollama pull qwen2.5
   ```
3. Ensure Ollama is running before launching Jura Archive.
4. Jura Archive connects to Ollama on `localhost:11434` — no additional configuration is needed.

---

## Data Storage

All data is stored locally:

| Data | Location |
|------|----------|
| Asset database | `<app-data-dir>/jura_archive.db` |
| Configuration | `<app-data-dir>/config.json` |
| Ollama models | `~/.ollama/models/` |

No data is sent to external servers. No telemetry is collected.
