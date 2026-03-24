# Deployment Guide

**Last updated**: 24 March 2026
**Phase**: Phase 3 (v0.6.0-dev)

---

## Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [First Run](#first-run)
4. [Sidecar Authentication](#sidecar-authentication)
5. [Database Location](#database-location)
6. [Auto-Catalogue Tiers](#auto-catalogue-tiers)
7. [Data Storage](#data-storage)
8. [Python ML Sidecar](#python-ml-sidecar)

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
2. Open the DMG and drag **Jura Trace** to your Applications folder.
3. On first launch, macOS may prompt you to allow the app — open System Settings > Privacy & Security and click "Open Anyway".

### Windows

> **Note**: Windows installers are in progress — targeted for Sprint 18 (May 2026).

### Linux

> **Note**: Linux AppImage packages are in progress — targeted for Sprint 18 (May 2026).

---

## First Run

1. Launch Jura Trace.
2. The application creates a local database in your system's app data directory — no configuration required.
3. Navigate to the **Protect** tab to begin importing files.
4. All core features (import, metadata extraction, C2PA signing, fingerprinting) work immediately — no additional setup needed.

---

## Sidecar Authentication

From Sprint 15, the Python ML sidecar requires an API key on all requests. This adds a layer of defence against other processes on the same machine calling the sidecar without authorisation.

### How the key works

Jura Trace generates a key automatically at startup using a UUID and passes it to the sidecar process. The sidecar checks for the `X-Jura-API-Key` header on every request. Requests without a valid key are rejected with `401 Unauthorized`.

You do not need to configure anything for standard single-user desktop use — the key is negotiated internally between the Tauri app and the sidecar.

### Scripted and institutional deployments

For managed deployments where the sidecar is started independently of the desktop app (for example, on a shared workstation or via an institution's deployment script), set the key explicitly using the `JURA_SIDECAR_KEY` environment variable:

```bash
export JURA_SIDECAR_KEY="your-secret-key-here"
uvicorn main:app --host 127.0.0.1 --port 8200
```

The Tauri app reads the same environment variable at launch and sends that key with every sidecar request. If `JURA_SIDECAR_KEY` is not set, a fresh UUID is generated each time the app starts — this is the default behaviour for single-user installations.

**Recommendation for institutional deployments**: set `JURA_SIDECAR_KEY` explicitly in your environment configuration so that the key is consistent across restarts and deployment targets.

---

## Database Location

Jura Trace stores all asset metadata, fingerprints, verification results, and audit logs in a single SQLite database file. By default, this file is placed in the standard application data directory for your operating system:

| Platform | Default path |
|----------|-------------|
| macOS    | `~/Library/Application Support/Jura Trace/jura_archive.db` |
| Windows  | `%APPDATA%\Jura Trace\jura_archive.db` |
| Linux    | `~/.local/share/jura-trace/jura_archive.db` |

### Changing the database location

Three mechanisms control the database path, applied in this priority order:

**1. Environment variable (highest priority)**

Set `JURA_DB_PATH` to an absolute file path before launching the application:

```bash
export JURA_DB_PATH="/mnt/archive/jura_trace.db"
```

This overrides all other settings. Use this approach for scripted deployments or when the database must reside on a specific volume.

**2. Configuration file**

The `config.json` file in the app data directory accepts a `db_path` key:

```json
{
  "db_path": "/mnt/archive/jura_trace.db"
}
```

The config file is created on first launch. Edit it with any text editor. Changes take effect on the next application start.

**3. Settings UI**

Open **Settings** and click **Change Location...** under the Database section. A file picker allows you to choose a new directory. The database is moved to the new location automatically.

**Priority order**: `JURA_DB_PATH` environment variable → `db_path` in `config.json` → platform default.

---

## Auto-Catalogue Tiers

Jura Trace uses a three-tier cataloguing system. Each tier is independent and optional:

### Tier 1 — Metadata Extraction (always available)

Extracts EXIF, XMP, and IPTC metadata from imported files. Provides camera information, dates, GPS coordinates, copyright, dimensions, and software details. No additional setup required.

### Tier 2 — CLIP Subject Tagging (optional download, ~400MB)

On first use, the application offers to download the CLIP vision model (~400MB). Once downloaded, Jura Trace can automatically suggest subject tags from a curated museum vocabulary (based on ICONCLASS and Getty AAT taxonomies). Tags are generated locally in 50–200ms per image.

**Requirements**: ~600–800MB RAM during tagging, ~400MB disk space for the model.

### Tier 3 — Ollama Descriptions (optional enhancement)

For rich free-text descriptions, Jura Trace can connect to a locally running Ollama instance. This is the only feature that requires Ollama.

1. Install Ollama from https://ollama.com
2. Pull the required models:
   ```bash
   ollama pull llava
   ollama pull qwen2.5
   ```
3. Ensure Ollama is running before launching Jura Trace.
4. Jura Trace connects to Ollama on `localhost:11434` — no additional configuration is needed.

**Requirements**: 8 GB+ RAM, ~4–8GB disk space for models.

The application clearly indicates which tiers are available in the Settings page.

---

## Data Storage

All data is stored locally:

| Data | Location |
|------|----------|
| Asset database | See [Database Location](#database-location) |
| Configuration | `<app-data-dir>/config.json` |
| CLIP model (Tier 2) | `<app-data-dir>/models/` |
| Ollama models (Tier 3) | `~/.ollama/models/` |

No data is sent to external servers. No telemetry is collected.

### Data-at-Rest Security

The SQLite database stores asset metadata, fingerprint hashes, verification results, and audit logs. **The database is not encrypted at rest.** This means anyone with file-system access to the device can read its contents.

**Risk assessment**: For single-user workstations this is generally acceptable — the database contains metadata about files, not the files themselves. For shared workstations or high-security environments, this warrants mitigation.

**Recommended mitigations (in order of preference)**:

1. **OS-level disk encryption** — Enable FileVault (macOS), BitLocker (Windows), or LUKS (Linux). This protects all local data transparently and is the recommended approach for v1.0.
2. **User account separation** — Ensure each user has a separate OS account. The database is stored in the user's app data directory, which is not accessible to other standard users.
3. **SQLCipher** (planned for v1.1) — A future release will offer optional AES-256 encryption of the SQLite database via SQLCipher. This will require a passphrase on first run and adds ~200 ms to startup. See the [v1.1 backlog](./sprint-plans/sprint-15-to-v1.0-plan.md#v11-backlog-post-v10) for status.

**What is stored in the database**:
- File names, paths, sizes, content types, and import dates
- EXIF metadata summaries and anomaly findings
- Perceptual fingerprint hashes (aHash, dHash, pHash)
- C2PA manifest digests (not full manifests)
- Verification trust scores and verdict history
- Watermark embed/extract records
- Audit log of all user actions (with SHA-256 hash chain)

**What is NOT stored in the database**:
- Original file contents or pixel data
- Forensic heatmap images (generated on demand, not persisted)
- Ollama API keys or credentials (there are none — local-only)

---

## Python ML Sidecar

The Python sidecar provides forensic analysis (ELA, noise, copy-move, deepfake, and more) and runs on `localhost:8200`. It is optional — the application functions without it, but forensic detectors will be unavailable.

### Starting the sidecar

```bash
cd sidecar
pip install -r requirements.txt
uvicorn main:app --host 127.0.0.1 --port 8200
```

If you are using a managed deployment with an explicit API key, set `JURA_SIDECAR_KEY` before starting — see [Sidecar Authentication](#sidecar-authentication).

### Optional dependencies

| Feature | Dependency | Approximate size |
|---------|-----------|-----------------|
| Video/audio metadata | FFmpeg + ffprobe | ~100 MB |
| Speech transcription | faster-whisper (see below) | ~500 MB |
| CLIP AI detection | open_clip ViT-B/32 | ~350 MB |

The sidecar degrades gracefully when optional dependencies are absent — affected endpoints return informative error messages rather than crashing.

### Speech transcription

Speech transcription is provided by the `faster-whisper` Python package, which is included in `requirements.txt` and installed automatically.

- The transcription model downloads automatically on first use (approximately 500 MB).
- Transcription runs entirely on-device — no audio data leaves the machine.
- Transcription results feed directly into the RAG claim checker when Ollama is running (see [Tier 3](#tier-3--ollama-descriptions-optional-enhancement) above). This allows Jura Trace to verify spoken claims in video and audio files against contextual knowledge.
- If `faster-whisper` is unavailable or the model has not yet downloaded, the verify pipeline completes without transcription — no errors are raised.

> **Note**: On first use with a video or audio file, allow additional time for the transcription model to download. Subsequent runs use the cached model and are significantly faster.
