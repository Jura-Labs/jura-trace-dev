---
title: "Jura Trace — Deployment Guide"
description: "System requirements, installation, configuration, and institutional deployment guidance for IT administrators and advanced users."
last-updated: 28 March 2026
phase: Phase 3 (v0.9.0-rc)
---

# Jura Trace — Deployment Guide

Jura Trace is a local-first desktop application for content verification and protection. All processing runs on-device. No data leaves your machine. This guide covers system requirements, installation, optional component setup, and institutional deployment configuration.

**Developed by**: Juralabs Community Interest Company (UK) — https://juralabs.org
**Licence**: PolyForm Noncommercial 1.0.0
**Current version**: 0.9.0-rc.3 (Phase 3)

---

## Contents

1. [System Requirements](#system-requirements)
2. [Installation](#installation)
3. [First Launch](#first-launch)
4. [Database Location](#database-location)
5. [Analysis Engine (Python ML Sidecar)](#analysis-engine-python-ml-sidecar)
6. [Sidecar Authentication](#sidecar-authentication)
7. [Optional: Speech Transcription](#optional-speech-transcription)
8. [Optional: Ollama LLM Integration](#optional-ollama-llm-integration)
9. [Optional: FFmpeg](#optional-ffmpeg)
10. [Network Configuration](#network-configuration)
11. [Institutional Deployment](#institutional-deployment)
12. [Data Storage](#data-storage)
13. [Troubleshooting](#troubleshooting)

---

## System Requirements

| Component        | Minimum                                  | Recommended                          |
|------------------|------------------------------------------|--------------------------------------|
| Operating system | macOS 13.0+, Windows 10+ (64-bit), Ubuntu 22.04+ / Fedora 38+ | macOS 14+, Windows 11, Ubuntu 24.04+ |
| Architecture     | x86-64 or Apple Silicon (ARM64)          | Apple Silicon M1 or later            |
| RAM              | 4 GB                                     | 8 GB (required for video deepfake analysis and Ollama) |
| Disk (app)       | 500 MB                                   | 500 MB                               |
| Disk (ML models) | — (optional)                             | 2 GB (Whisper base ~1.5 GB + CLIP ~350 MB) |
| Disk (Ollama)    | — (optional)                             | 10 GB (LLaVA ~4.7 GB + Qwen2.5 ~4.4 GB) |

### Platform notes

**macOS**: macOS 13 (Ventura) is the minimum supported version, matching the `minimumSystemVersion` in the app bundle. Both Apple Silicon (arm64) and Intel (x86-64) builds are provided — download the correct DMG for your architecture.

**Windows**: Windows 10 version 1803 or later. Windows 11 is fully supported. The Microsoft Edge WebView2 Runtime is required — it ships with Windows 11 by default. Windows 10 users may need to install it separately (see [Installation — Windows](#windows)).

**Linux**: Ubuntu 22.04 LTS or later and Fedora 38 or later are the supported distributions. The application requires WebKitGTK 4.1. Ubuntu 20.04 and earlier are not supported — they ship with WebKitGTK 4.0.

---

## Installation

> **Note**: Code-signed builds require an Apple Developer ID certificate and a Windows EV code signing certificate. These are pending for the v1.0 public release. Current builds are unsigned — follow the platform-specific guidance below to install safely.

### macOS

Two DMG files are available for each release:

| Your Mac                        | File to download                            |
|---------------------------------|---------------------------------------------|
| Apple Silicon (M1/M2/M3/M4)    | `Jura-Trace_x.x.x_aarch64.dmg`             |
| Intel                           | `Jura-Trace_x.x.x_x64.dmg`                 |

**Installation steps:**

1. Download the correct DMG from the releases page.
2. Double-click the DMG to mount it.
3. Drag **Jura Trace** onto the **Applications** shortcut in the Finder window.
4. Eject the DMG.

**First launch (Gatekeeper bypass — required once only):**

Because this build is unsigned, macOS Gatekeeper will block it on first launch. Use one of the following methods:

- **Right-click method (all macOS versions)**: In Finder, right-click Jura Trace and select **Open**, then click **Open** in the confirmation dialog.
- **Privacy & Security method**: Attempt a normal open, then go to **System Settings → Privacy & Security** and click **Open Anyway**.
- **Terminal method**: `xattr -d com.apple.quarantine /Applications/Jura\ Trace.app`

If the sidecar binary is also quarantined (Analysis services shows "Offline" unexpectedly), clear the quarantine flag recursively:

```bash
xattr -cr /Applications/Jura\ Trace.app
```

For a full walkthrough, see [`docs/install-guides/macos-unsigned.md`](./install-guides/macos-unsigned.md).

---

### Windows

Two installer formats are available:

| Format | File | Use case |
|--------|------|----------|
| NSIS setup | `Jura-Trace_x.x.x_x64-setup.exe` | Most users |
| MSI package | `Jura-Trace_x.x.x_x64_en-US.msi` | Managed/enterprise environments |

**Installation steps:**

1. Download the installer.
2. Double-click to run it.
3. When Windows SmartScreen displays "Windows protected your PC", click **More info**, then **Run anyway**.
4. Follow the on-screen steps. The default installation path (`C:\Program Files\Jura Trace\`) is correct for most users.

If the **Run anyway** button is absent, your device is managed and SmartScreen blocking is enforced by Group Policy. Contact your IT department — provide the SHA-256 hash of the installer (available at [`docs/install-guides/verify-downloads.md`](./install-guides/verify-downloads.md)).

**WebView2 Runtime (Windows 10 only)**: If the application window appears blank after installation, install the WebView2 Evergreen Standalone Installer from [developer.microsoft.com/microsoft-edge/webview2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

For silent installation and enterprise deployment, see [`docs/install-guides/windows-it-deployment.md`](./install-guides/windows-it-deployment.md).

---

### Linux

Three package formats are available:

| Format | File | Use case |
|--------|------|----------|
| AppImage | `Jura-Trace_x.x.x_amd64.AppImage` | Most users — no system install required |
| Debian package | `jura-trace_x.x.x_amd64.deb` | Ubuntu, Debian |
| RPM package | `jura-trace_x.x.x_x86_64.rpm` | Fedora, openSUSE |

**AppImage (recommended):**

```bash
chmod +x Jura-Trace_x.x.x_amd64.AppImage
./Jura-Trace_x.x.x_amd64.AppImage
```

**Debian package:**

```bash
sudo dpkg -i jura-trace_x.x.x_amd64.deb
sudo apt install -f   # resolve any missing dependencies
```

**System dependencies**: The `.deb` and `.rpm` packages require WebKitGTK 4.1. The AppImage bundles this dependency.

```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.1-0

# Fedora
sudo dnf install webkit2gtk4.1
```

**Font rendering**: Jura Trace uses Georgia serif for headings, falling back to DejaVu Serif or Noto Serif on Linux. If headings appear in a sans-serif font:

```bash
# Ubuntu/Debian
sudo apt install fonts-dejavu

# Fedora
sudo dnf install dejavu-serif-fonts
```

For Wayland rendering issues, see [`docs/install-guides/linux-requirements.md`](./install-guides/linux-requirements.md).

---

### Building from source

Development builds can be run directly from the repository while signed installers are pending.

**Prerequisites**: Rust toolchain (stable), Node.js 18+, Python 3.13+.

```bash
# Clone the repository
git clone https://github.com/juralabs/jura-trace.git
cd jura-trace

# Start the SvelteKit frontend (Terminal 1)
cd ui && npm install && npm run dev

# Start the Tauri application (Terminal 2)
cd src-tauri && cargo tauri dev

# Start the Python ML sidecar (Terminal 3, optional)
cd sidecar && pip install -r requirements.txt
uvicorn main:app --host 127.0.0.1 --port 8200 --reload
```

Or use the convenience target:

```bash
make dev
```

---

## First Launch

On first launch, the **Setup Wizard** runs automatically. It checks for optional services and guides you through any required downloads:

1. **Sidecar health** — verifies the bundled ML sidecar started correctly.
2. **FFmpeg** — checks whether FFmpeg is available on your PATH. Video and audio features require it.
3. **Whisper model** — offers to download the speech transcription model (~1.5 GB base) for audio/video transcript analysis.
4. **Ollama** — checks whether Ollama is running on port 11434. Provides a download link if not installed.
5. **Ready** — summarises available capabilities.

All five steps are optional — you can skip any of them and proceed. Skipped capabilities are indicated in the Settings page and can be configured later.

After the wizard, Jura Trace creates a local SQLite database in your platform's application data directory (see [Database Location](#database-location)) and navigates to the **Protect** tab.

---

## Database Location

Jura Trace stores all asset metadata, fingerprints, verification results, and audit logs in a single SQLite file. By default, this file is placed in the standard application data directory for your operating system:

| Platform | Default path                                              |
|----------|-----------------------------------------------------------|
| macOS    | `~/Library/Application Support/org.juralabs.trace/`      |
| Windows  | `%APPDATA%\org.juralabs.trace\`                           |
| Linux    | `~/.local/share/org.juralabs.trace/`                     |

The database file is named `jura_archive.db` within that directory.

### Changing the database location

Three mechanisms control the database path, applied in this priority order:

**1. Environment variable (highest priority)**

Set `JURA_DB_PATH` to an absolute file path before launching the application:

```bash
# macOS / Linux
export JURA_DB_PATH="/mnt/archive/jura_trace.db"

# Windows (Command Prompt)
set JURA_DB_PATH=D:\Archive\jura_trace.db

# Windows (PowerShell)
$env:JURA_DB_PATH = "D:\Archive\jura_trace.db"
```

This overrides all other settings. Use this approach for scripted deployments or when the database must reside on a specific volume.

**2. Configuration file**

The `config.json` file in the application data directory accepts a `db_path` key:

```json
{
  "db_path": "/mnt/shared-archive/jura_trace.db",
  "licence_tier": "professional"
}
```

The file is created on first launch. Edit it with any text editor. Changes take effect on the next application start.

**3. Settings UI**

Open **Settings** and click **Change Location...** under the Database section. A folder picker appears. Jura Trace copies the existing database to the new location atomically (an SQLite integrity check is run before the old file is removed), then restarts the database connection.

**Priority order**: `JURA_DB_PATH` environment variable → `db_path` in `config.json` → platform default.

---

## Analysis Engine (Python ML Sidecar)

The Python ML sidecar provides forensic analysis — error level analysis (ELA), noise analysis, copy-move detection, deepfake scoring, NPR, chromatic aberration, JPEG ghost, segmented ELA, shadow consistency, colour temperature, splice boundary detection, CLIP zero-shot classification, watermark embed/extract, video frame extraction, and video deepfake analysis.

It runs on the `127.0.0.1` loopback interface, on an **OS-assigned ephemeral port** picked by the Tauri shell at each launch (Option C port-collision fix, May 2026). It is local-only — the bind address is `127.0.0.1`, so the port is not exposed to the network. The dynamic port is stored in `AppState.sidecar_port` and used by every Rust HTTP client; the frontend never speaks HTTP to the sidecar directly (all interaction goes through Rust IPC).

**Graceful degradation**: If the sidecar is unavailable, C2PA signing and verification, EXIF anomaly detection, perceptual fingerprinting, and metadata extraction all continue to work. Forensic detectors are skipped and the verify result notes the reduced analysis scope.

### Production builds

In production installers, the sidecar is bundled as a self-contained binary (`jura-sidecar`) and launched automatically by Tauri on startup. No separate Python installation is required.

The auto-launch process:

1. The Rust shell calls `TcpListener::bind("127.0.0.1:0")` so the OS allocates a free port, then drops the listener.
2. Tauri spawns `jura-sidecar` via `tauri-plugin-shell` with `--port $PORT`.
3. The app polls `http://127.0.0.1:$PORT/health/ready` with exponential backoff.
4. Once healthy, the sidecar is ready and forensic analysis becomes available.
5. On application exit, the sidecar process is cleanly terminated.

### Development builds

For development, start the sidecar manually:

```bash
cd sidecar
pip install -r requirements.txt
uvicorn main:app --host 127.0.0.1 --port 8200 --reload
```

### Optional sidecar dependencies

| Feature                        | Dependency                        | Approximate size |
|--------------------------------|-----------------------------------|-----------------|
| Video/audio metadata           | FFmpeg + ffprobe                  | ~100 MB         |
| Speech transcription           | faster-whisper (base model)       | ~1.5 GB         |
| CLIP zero-shot AI detection    | open_clip ViT-B/32 (lazy-loaded)  | ~350 MB         |

CLIP loads on first use and is ~350 MB. If unavailable, the CLIP detector is skipped gracefully — all other detectors continue to run.

---

## Sidecar Authentication

From Sprint 15, the sidecar requires an API key on every request. This provides a layer of defence against other processes on the same machine sending unauthorised requests to port 8200.

The key is sent as the `X-Jura-API-Key` HTTP header on all requests from the Rust core to the sidecar.

### Single-user installations

No configuration is required. Jura Trace generates a 256-bit session key automatically at startup and negotiates it with the sidecar process internally. The key changes on each application launch.

### Managed and institutional deployments

For deployments where the sidecar is started independently of the desktop application (for example, via a system service or deployment script), set a fixed key using the `JURA_SIDECAR_KEY` environment variable:

```bash
# macOS / Linux
export JURA_SIDECAR_KEY="your-256-bit-key-here"
uvicorn main:app --host 127.0.0.1 --port 8200

# Windows (PowerShell)
$env:JURA_SIDECAR_KEY = "your-256-bit-key-here"
```

The Tauri application reads the same environment variable at launch and uses that key for all sidecar requests. If `JURA_SIDECAR_KEY` is not set, a fresh key is generated on each start.

**Recommendation**: For institutional deployments, set `JURA_SIDECAR_KEY` explicitly in your environment configuration so the key is consistent across restarts.

**Development builds**: In development mode, the sidecar accepts requests with an empty key by default. No configuration is needed.

---

## Optional: Speech Transcription

Audio and video transcription is provided by `faster-whisper`. Transcription feeds directly into the RAG claim checker when Ollama is running, allowing Jura Trace to verify spoken claims in video and audio files against contextual knowledge.

Transcription runs entirely on-device. No audio data leaves the machine.

### Automatic download

The Whisper model (~1.5 GB for the base model) downloads automatically on first use with a video or audio file. The Setup Wizard prompts you to pre-download it if preferred.

### Manual pre-download

To pre-download the model before first use:

```bash
python -c "from faster_whisper import WhisperModel; WhisperModel('base')"
```

This caches the model to `~/.cache/huggingface/hub/` (or the equivalent HuggingFace cache directory on your platform).

### Graceful degradation

If `faster-whisper` is unavailable or the model has not yet downloaded, the verify pipeline completes without transcription. No errors are raised. The transcript panel in the verify results will be absent.

---

## Optional: Ollama LLM Integration

Ollama provides two optional capabilities within Jura Trace:

- **LLaVA** — rich natural-language descriptions of images (Tier 3 auto-catalogue).
- **Qwen2.5** — RAG claim verification. Cross-references spoken or written claims in media against Jura Trace's knowledge base of forensic methodology, digital rights, and cultural heritage guidance.

Both capabilities are fully optional. All core verification features work without Ollama.

### Installation

1. Download and install Ollama from https://ollama.com.
2. Pull the required models:

```bash
ollama pull llava:7b      # image descriptions and text extraction (~4.7 GB)
ollama pull qwen2.5:7b    # RAG claim verification (~4.4 GB)
```

3. Ensure Ollama is running before launching Jura Trace:

```bash
ollama serve   # if not already running as a background service
```

Jura Trace connects to Ollama on `localhost:11434` — no additional configuration is required.

### Resource requirements

| Model       | Disk  | RAM (approximate) |
|-------------|-------|-------------------|
| llava:7b    | 4.7 GB | 6–8 GB            |
| qwen2.5:7b  | 4.4 GB | 6–8 GB            |

8 GB RAM is the practical minimum for running either model. Running both simultaneously requires 16 GB or a machine with unified memory (Apple Silicon M2 Pro or equivalent).

### Network behaviour

Ollama operates entirely locally. No data is sent to Anthropic, Ollama, or any external service. The connection is `http://127.0.0.1:11434` — loopback only.

---

## Optional: FFmpeg

FFmpeg is required for the following features:

- Video metadata extraction (codec, resolution, FPS, duration, audio track info)
- Video frame extraction and thumbnail generation
- Audio metadata extraction (codec, sample rate, channels, bitrate)
- Speech transcription (ffprobe is used to demux audio from video containers)

Image verification, C2PA signing, perceptual fingerprinting, and EXIF analysis all work without FFmpeg.

### macOS

```bash
brew install ffmpeg
```

If Homebrew is not installed: https://brew.sh

### Linux (Ubuntu / Debian)

```bash
sudo apt install ffmpeg
```

### Linux (Fedora)

```bash
sudo dnf install ffmpeg
```

> **Note**: On Fedora, FFmpeg is available from the RPM Fusion repository. If the command above fails, enable RPM Fusion first:
>
> ```bash
> sudo dnf install https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
> sudo dnf install ffmpeg
> ```

### Windows

1. Download a Windows build from [ffmpeg.org/download.html](https://ffmpeg.org/download.html). Under "Get packages & executable files", choose **Windows builds from gyan.dev**.
2. Extract the `.zip` to a permanent location, for example `C:\ffmpeg\`.
3. Add `C:\ffmpeg\bin` to your system PATH:
   - Search for **Environment Variables** in the Start menu.
   - Under "System variables", select **Path** → **Edit** → **New**.
   - Enter `C:\ffmpeg\bin` and click **OK** on each window.
4. Restart Jura Trace.

**Verify the installation**: Open a terminal or Command Prompt and run `ffmpeg -version`. Version information confirms a successful installation.

For IT administrators managing FFmpeg deployment across a Windows estate, see the FFmpeg section of [`docs/install-guides/windows-it-deployment.md`](./install-guides/windows-it-deployment.md).

---

## Network Configuration

Jura Trace is designed for fully offline operation. No internet connection is required for normal use.

### Local ports

| Port  | Service                  | Required | Notes                                      |
|-------|--------------------------|----------|--------------------------------------------|
| 1420  | SvelteKit dev server     | Dev only | Not present in production builds           |
| 8200  | Python ML sidecar        | Optional | Loopback only (`127.0.0.1`) — not exposed to the network |
| 11434 | Ollama LLM runtime       | Optional | Loopback only — default Ollama port        |

All three ports listen on `127.0.0.1` only. None are bound to `0.0.0.0`. No inbound connections from the network are accepted.

### Outbound connections

| Destination                                                              | Purpose                    | Configurable          |
|--------------------------------------------------------------------------|----------------------------|-----------------------|
| `https://github.com/juralabs/jura-archive/releases/latest/download/latest.json` | Auto-update check | Disable in Settings → Updates |
| `https://ollama.com` (Ollama install only)                               | Model download             | One-time, manual only |
| `https://huggingface.co` (first use only)                                | Whisper model download     | One-time, automatic   |

The auto-updater checks the GitHub releases endpoint once per session. It does not transmit any device or usage data — it only fetches the `latest.json` manifest. You can disable it in **Settings → Updates**.

No telemetry is collected. No usage data, file metadata, or verification results are ever transmitted.

### Firewall rules

For managed Windows environments, port 8200 must be accessible on the loopback interface. On Windows, the Jura Trace sidecar service should be allowed to communicate on **private networks only**. See [`docs/install-guides/windows-it-deployment.md`](./install-guides/windows-it-deployment.md) for Group Policy firewall rule configuration.

---

## Institutional Deployment

### Shared workstations

To share a single database across multiple users on a workstation, or to store the database on a network volume accessible to a team:

```bash
# Point all users to a shared network location
export JURA_DB_PATH="/mnt/shared-archive/jura_trace.db"
```

Or set `db_path` in `config.json` in each user's application data directory:

```json
{
  "db_path": "\\\\server\\archive\\jura_trace.db"
}
```

> **Note**: SQLite is designed for single-writer access. For concurrent multi-user write scenarios, use separate database files per user and combine records periodically, or contact Juralabs to discuss Team or Enterprise tier options.

### Enterprise image pre-configuration

To pre-configure Jura Trace as part of a desktop image or deployment package, place a `config.json` file in the application data directory before first launch:

```json
{
  "db_path": "D:\\JuraTrace\\jura_archive.db",
  "licence_tier": "professional"
}
```

Valid `licence_tier` values: `community`, `professional`, `team`, `enterprise`.

### Silent Windows installation

```powershell
# MSI — fully silent, no reboot
msiexec /i "Jura-Trace_x.x.x_x64_en-US.msi" /qn /norestart

# NSIS — silent with default options
Jura-Trace_x.x.x_x64-setup.exe /S
```

For Intune, SCCM/MECM, and Group Policy deployment, see [`docs/install-guides/windows-it-deployment.md`](./install-guides/windows-it-deployment.md).

### Licence tiers

Jura Trace operates under a tiered licence structure:

| Tier         | Codename | Use case                                      |
|--------------|----------|-----------------------------------------------|
| Community    | Flint    | Individual, non-commercial use — free         |
| Professional | Stratum  | Individual professionals and researchers      |
| Team         | Geode    | Small teams and cultural organisations        |
| Enterprise   | Bedrock  | Large institutions, multi-site deployments    |

The Community tier is free and covered by the PolyForm Noncommercial 1.0.0 licence. Professional, Team, and Enterprise tiers include additional capabilities and support. Contact sales@juralabs.org for pricing.

### Disconnected environments

Jura Trace operates without internet access for all core functionality. To deploy in a fully disconnected environment:

1. Disable the auto-updater by setting `"updates_enabled": false` in `config.json`.
2. Pre-download and bundle any required ML models before deployment.
3. Ensure the bundled sidecar binary is included in the installer (production builds include it automatically).

---

## Data Storage

All data is stored locally on the user's machine. The application stores no data in the cloud.

| Data                        | Location                                                      |
|-----------------------------|---------------------------------------------------------------|
| Asset database              | See [Database Location](#database-location)                   |
| Application configuration   | `<app-data-dir>/config.json`                                  |
| CLIP model                  | `<app-data-dir>/models/`                                      |
| Ollama models               | `~/.ollama/models/`                                           |
| Whisper model cache         | `~/.cache/huggingface/hub/` (or platform equivalent)          |

### Database contents

The SQLite database stores:

- File names, paths, sizes, content types, and import dates
- EXIF metadata summaries and anomaly findings
- Perceptual fingerprint hashes (aHash, dHash, pHash)
- C2PA manifest digests (not full manifests)
- Verification trust scores and verdict history
- Watermark embed and extract records
- Monitor watchlist URLs and events
- Audit log of all user actions, with a SHA-256 hash chain for tamper detection

The database does **not** store:

- Original file contents or pixel data
- Forensic heatmap images (generated on demand, not persisted)
- API keys or credentials

### Data-at-rest security

The SQLite database is not encrypted at the application level. The data-at-rest control is OS-level full-disk encryption, which is GDPR Art 32 compliant and is the default on modern macOS and Windows. Required mitigations:

1. **OS-level full-disk encryption** — FileVault (macOS, default since Catalina), BitLocker (Windows, default since 11 24H2), or LUKS or equivalent on Linux. Protects all local data transparently. Institutional deployments must verify FDE is enabled.
2. **User account separation** — Ensure each user has a separate OS account. The database resides in the user's application data directory and is not accessible to other standard users.

Application-level encryption (SQLCipher and similar) was evaluated and not adopted. Rationale: on a local-first application running on a modern OS where FDE is default-on, application-level encryption adds passphrase-management UX cost without meaningfully reducing the realistic threat surface — malware running as the user can read either the database or the keychain that would hold the passphrase. If a deployment has a specific requirement that OS-level FDE cannot satisfy, contact us to discuss as a Custom Engineering deliverable.

---

## Troubleshooting

### Sidecar not starting

**Symptom**: Analysis services shows "Offline" in Settings after launch.

Since the Option C port-collision fix (May 2026) the sidecar binds an OS-assigned ephemeral port, so a fixed-port collision is no longer a likely cause. Diagnostic steps:

1. Find the port the running sidecar is actually using:
   ```bash
   # macOS / Linux
   ps aux | grep jura-sidecar      # the `--port NNNNN` arg is visible here
   lsof -p $(pgrep -f jura-sidecar | head -1) -i TCP

   # Windows (PowerShell)
   Get-Process jura-sidecar | ForEach-Object {
       Get-NetTCPConnection -OwningProcess $_.Id -State Listen
   }
   ```
2. If no `jura-sidecar` process is running at all, the spawn failed — check the application log (path printed at startup; typical locations are `~/Library/Logs/com.juralabs.jura-trace/` on macOS and `%APPDATA%\com.juralabs.jura-trace\logs\` on Windows) for `Failed to (re)spawn sidecar` or `[Errno NN]` messages.
3. On macOS, check whether the sidecar binary inside the app bundle is quarantined:
   ```bash
   xattr -cr /Applications/Jura\ Trace.app
   ```
4. **Orphaned sidecar from a previous launch** (Windows is most affected because it lacks SIGTERM): a previous crash may have left a `jura-sidecar` process running but no longer associated with any Jura Trace window. With Option C this no longer blocks new launches (each launch picks its own port), but the orphan still consumes ~300-500 MB RAM. Kill the orphan and relaunch:
   ```bash
   # macOS / Linux
   pkill -f jura-sidecar

   # Windows (PowerShell)
   Stop-Process -Name jura-sidecar -Force
   ```

---

### Database locked

**Symptom**: An error appears stating the database is locked or in use.

- Ensure only one instance of Jura Trace is running.
- On shared volumes, verify no other process holds the database file open.
- SQLite WAL mode (enabled by default) tolerates concurrent readers — a lock error indicates a writer conflict. Close all Jura Trace windows and reopen.

---

### FFmpeg not found

**Symptom**: Video and audio features show as unavailable even after installing FFmpeg.

1. Verify FFmpeg is on your PATH: open a **new** terminal window and run `ffmpeg -version`.
   - If you see "command not found", the installation did not complete or the PATH was not updated.
2. Restart Jura Trace after installing or updating FFmpeg. The Setup Wizard re-checks availability on launch.
3. On macOS, if you installed via Homebrew and Jura Trace still does not detect it, verify that `/usr/local/bin` (Intel) or `/opt/homebrew/bin` (Apple Silicon) is in your shell's PATH.

---

### Ollama not responding

**Symptom**: RAG claim verification and Tier 3 image descriptions are unavailable.

1. Check whether Ollama is running:
   ```bash
   ollama list
   ```
   If this returns an error, Ollama is not running.
2. Start Ollama:
   ```bash
   ollama serve
   ```
3. Verify the required models are downloaded:
   ```bash
   ollama list
   # Should show llava:7b and qwen2.5:7b
   ```
4. Confirm Ollama is listening on port 11434:
   ```bash
   curl http://127.0.0.1:11434/
   ```

---

### Transcription model downloading slowly

The Whisper base model is approximately 1.5 GB. Download time depends on your connection. Progress is shown in the Setup Wizard and in the verify panel on first use with audio or video.

Once downloaded, the model is cached locally and does not download again. Subsequent transcription runs are fast.

---

### Windows SmartScreen blocks installation

See [Installation — Windows](#windows) above. If the **Run anyway** option is absent on a managed device, contact your IT department. Provide the SHA-256 hash of the installer from [`docs/install-guides/verify-downloads.md`](./install-guides/verify-downloads.md).

---

## Related Documents

- [`docs/install-guides/macos-unsigned.md`](./install-guides/macos-unsigned.md) — detailed macOS installation walkthrough
- [`docs/install-guides/windows-unsigned.md`](./install-guides/windows-unsigned.md) — detailed Windows installation walkthrough
- [`docs/install-guides/linux-requirements.md`](./install-guides/linux-requirements.md) — Linux system dependencies and Wayland guidance
- [`docs/install-guides/windows-it-deployment.md`](./install-guides/windows-it-deployment.md) — Intune, GPO, silent install, FFmpeg deployment for IT administrators
- [`docs/install-guides/verify-downloads.md`](./install-guides/verify-downloads.md) — SHA-256 installer verification
- [`docs/install-guides/quickstart-macos.md`](./install-guides/quickstart-macos.md) — one-page quick start for macOS
- [`docs/install-guides/quickstart-windows.md`](./install-guides/quickstart-windows.md) — one-page quick start for Windows
- [`docs/install-guides/quickstart-linux.md`](./install-guides/quickstart-linux.md) — one-page quick start for Linux
- [`docs/ARCHITECTURE.md`](./ARCHITECTURE.md) — technical architecture reference
- [`docs/sprint-plans/sprint-15-to-v1.0-plan.md`](./sprint-plans/sprint-15-to-v1.0-plan.md) — release roadmap and post-v1.0 backlog

---

*Jura Trace is developed by Juralabs Community Interest Company (UK). Licenced under PolyForm Noncommercial 1.0.0. Local-first — no data leaves your machine.*
