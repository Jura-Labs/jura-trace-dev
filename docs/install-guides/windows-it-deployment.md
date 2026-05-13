---
title: "Windows IT Deployment — Jura Trace Technical Appendix"
description: "Technical reference for IT administrators deploying Jura Trace on managed Windows environments via Active Directory, Intune, or SCCM/MECM."
last-updated: 25 March 2026
status: internal
---

# Windows IT Deployment — Jura Trace Technical Appendix

This appendix is for IT administrators deploying Jura Trace on managed Windows environments. It covers silent installation, Intune and SCCM/MECM packaging, Group Policy firewall rules, FFmpeg deployment without administrative privileges, SmartScreen handling, data locations, and network requirements.

For end-user installation guidance, see [`windows-unsigned.md`](./windows-unsigned.md). For SHA-256 installer verification, see [`verify-downloads.md`](./verify-downloads.md).

---

## Contents

1. [Silent installation](#silent-installation)
2. [Installation scope](#installation-scope)
3. [Intune deployment](#intune-deployment)
4. [SCCM / MECM deployment](#sccm--mecm-deployment)
5. [Group Policy — Windows Firewall](#group-policy--windows-firewall)
6. [FFmpeg deployment](#ffmpeg-deployment)
7. [SmartScreen and code signing](#smartscreen-and-code-signing)
8. [Data locations](#data-locations)
9. [Network requirements](#network-requirements)
10. [Environment variables](#environment-variables)
11. [Uninstallation](#uninstallation)

---

## Silent installation

Jura Trace ships in two Windows installer formats. Choose the format that suits your deployment tooling.

### MSI package (recommended for managed environments)

```powershell
msiexec /i "Jura-Trace_X.X.X_x64_en-US.msi" /qn /norestart
```

| Flag | Meaning |
|---|---|
| `/qn` | No UI — fully silent |
| `/norestart` | Suppresses automatic reboot prompts |

### NSIS setup executable

```powershell
Jura-Trace_X.X.X_x64-setup.exe /S
```

The `/S` flag (capital S) runs the NSIS installer silently with default options.

### Logging the installation

To capture a verbose log for troubleshooting, append a log path to the MSI command:

```powershell
msiexec /i "Jura-Trace_X.X.X_x64_en-US.msi" /qn /norestart /l*v "C:\Logs\jura-trace-install.log"
```

---

## Installation scope

### Per-user installation (no administrative privileges required)

```powershell
msiexec /i "Jura-Trace_X.X.X_x64_en-US.msi" /qn /norestart ALLUSERS=0
```

Installs to `%LOCALAPPDATA%\Programs\Jura Trace\`. The application and its ML sidecar run entirely in user space. No administrative rights are required at runtime.

**This is the default and recommended mode for most deployments.**

### Per-machine installation (requires administrative privileges)

```powershell
msiexec /i "Jura-Trace_X.X.X_x64_en-US.msi" /qn /norestart ALLUSERS=1
```

Installs to `%ProgramFiles%\Jura Trace\`. All users on the machine share the application binary. Each user maintains their own database under `%APPDATA%\Jura Trace\`.

---

## Intune deployment

Package Jura Trace as a Win32 app using the Microsoft Win32 Content Prep Tool (`IntuneWinAppUtil.exe`).

### Package preparation

```powershell
IntuneWinAppUtil.exe -c "C:\Staging\JuraTrace" -s "Jura-Trace_X.X.X_x64_en-US.msi" -o "C:\Output"
```

### Intune app configuration

| Field | Value |
|---|---|
| App type | Windows app (Win32) |
| Install command | `msiexec /i "Jura-Trace_X.X.X_x64_en-US.msi" /qn /norestart` |
| Uninstall command | `msiexec /x {ProductCode} /qn` |
| Install behaviour | User |
| Restart behaviour | App install may force a restart — Suppress |
| Return codes | `0` (success), `3010` (success, reboot required) |

Replace `{ProductCode}` with the GUID from the MSI property table. You can retrieve it with:

```powershell
Get-WmiObject -Query "SELECT * FROM Win32_Product WHERE Name LIKE 'Jura Trace%'" | Select-Object IdentifyingNumber
```

### Detection rule

| Rule type | Path | File or folder | File name | Detection method |
|---|---|---|---|---|
| File | `%LOCALAPPDATA%\Programs\Jura Trace` | File | `jura-trace.exe` | File or folder exists |

For per-machine installs, change the path to `%ProgramFiles%\Jura Trace`.

### Requirements

| Field | Value |
|---|---|
| Operating system architecture | 64-bit |
| Minimum operating system | Windows 10 1803 |

---

## SCCM / MECM deployment

### Application model (recommended)

Create a new Application in the SCCM console with the following deployment type settings.

**General:**

| Field | Value |
|---|---|
| Name | Jura Trace X.X.X |
| Technology | Script Installer |

**Programs:**

| Field | Value |
|---|---|
| Installation program | `msiexec /i "Jura-Trace_X.X.X_x64_en-US.msi" /qn /norestart` |
| Uninstall program | `msiexec /x {ProductCode} /qn` |
| Run | Hidden |
| Installation start in | (leave blank — uses content location) |

**Detection method** — use an MSI product code rule or the file detection rule described in the Intune section above.

**Return codes:**

| Return code | Type |
|---|---|
| `0` | Success |
| `3010` | Soft reboot |
| `1641` | Hard reboot |

---

## Group Policy — Windows Firewall

Jura Trace's ML sidecar listens on a **dynamically-assigned ephemeral port** on the `127.0.0.1` loopback interface (Option C port-collision fix, May 2026). Each launch the Tauri shell asks the OS for a free port via `bind("127.0.0.1:0")` and passes that port to the sidecar binary — there is no fixed port to pre-authorise.

Because the bind address is always `127.0.0.1`, inbound traffic from other machines cannot reach the sidecar regardless of firewall configuration. **No inbound firewall rule is required.** Windows Firewall may still prompt users on first launch the first time a particular Jura Trace build binds; if you want to silence that prompt across an estate without enumerating ports, pre-authorise the executable itself rather than a port:

```powershell
# Allow the Jura Trace sidecar executable on private networks
# (binds to 127.0.0.1 only — this rule prevents the first-run prompt;
#  it does not change actual network exposure.)
New-NetFirewallRule -DisplayName "Jura Trace Sidecar" `
  -Direction Inbound -Protocol TCP `
  -Program "C:\Program Files\Jura Trace\jura-sidecar.exe" `
  -Action Allow -Profile Private `
  -Description "Jura Trace ML analysis sidecar (loopback only)"
```

The optional Ollama integration uses port `11434` on the same loopback interface. If you are deploying Ollama alongside Jura Trace:

```powershell
New-NetFirewallRule -DisplayName "Jura Trace Ollama" `
  -Direction Inbound -Protocol TCP -LocalPort 11434 `
  -Action Allow -Profile Private `
  -Description "Ollama LLM runtime for Jura Trace optional AI features (localhost only)"
```

Ollama is not required for core Jura Trace functionality. It enables optional Tier 3 image descriptions and RAG claim verification only.

---

## FFmpeg deployment

Video metadata extraction, frame-by-frame forensic analysis, and audio transcription all require FFmpeg. Jura Trace degrades gracefully when FFmpeg is absent — image verification and C2PA signing continue to work without it. Deploy FFmpeg if your users will work with video or audio files.

The script below installs FFmpeg to the user profile and adds it to the user PATH. **No administrative privileges are required.**

```powershell
# Download FFmpeg to user profile (no admin required)
$ffmpegDir = "$env:LOCALAPPDATA\ffmpeg"
New-Item -ItemType Directory -Force -Path $ffmpegDir

Invoke-WebRequest -Uri "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" `
  -OutFile "$env:TEMP\ffmpeg.zip"

Expand-Archive "$env:TEMP\ffmpeg.zip" -DestinationPath "$env:TEMP\ffmpeg-extract" -Force

Copy-Item "$env:TEMP\ffmpeg-extract\ffmpeg-*\bin\*" -Destination $ffmpegDir

# Clean up temporary files
Remove-Item "$env:TEMP\ffmpeg.zip", "$env:TEMP\ffmpeg-extract" -Recurse -Force

# Add to user PATH (persists across sessions, no admin required)
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$ffmpegDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$ffmpegDir", "User")
}

Write-Host "FFmpeg installed to $ffmpegDir — restart Jura Trace to enable video analysis."
```

To deploy this as a per-machine installation (system PATH, requires administrator):

```powershell
$ffmpegDir = "C:\Program Files\ffmpeg\bin"
New-Item -ItemType Directory -Force -Path $ffmpegDir

# (download and extract steps as above, then:)

$currentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
if ($currentPath -notlike "*$ffmpegDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$ffmpegDir", "Machine")
}
```

After FFmpeg is installed, users must restart Jura Trace for the detection to take effect. No application reconfiguration is needed — Jura Trace probes for `ffmpeg` on the PATH at startup.

---

## SmartScreen and code signing

Pre-release pilot builds are not code-signed. Windows SmartScreen will display a "Windows protected your PC" warning when users run the installer.

**For end users on unmanaged devices:**

1. Click **More info** in the SmartScreen dialog.
2. Click **Run anyway**.

**For managed devices where Group Policy blocks unsigned execution:**

The "Run anyway" button is not shown when `SmartScreen` enforcement is set to **Block** in Group Policy (`Computer Configuration → Windows Settings → Security Settings → Windows Defender SmartScreen`). In this configuration, users cannot bypass the warning themselves.

Options available to you:

- Request a temporary publisher exception for the Jura Trace installer hash.
- Add the installer to your organisation's application allow-list.
- Deploy silently via SCCM or Intune (management tools bypass SmartScreen by design).
- Wait for the signed release (targeted for v1.0 — see the release roadmap).

**To obtain the SHA-256 hash for your allow-list or exception request**, see [`verify-downloads.md`](./verify-downloads.md). Hashes are published in `SHA256SUMS.txt` on the GitHub Releases page alongside each build.

Signed, notarised builds are planned for the public v1.0 release. Pilot builds will remain unsigned.

---

## Data locations

All Jura Trace data is stored locally on the user's machine. No data is transmitted to external servers.

| Data | Default location |
|---|---|
| Asset database | `%APPDATA%\Jura Trace\jura_archive.db` |
| Application settings | `%APPDATA%\Jura Trace\` |
| ML sidecar temporary files | `%TEMP%\jura-sidecar\` (cleaned on process exit) |

`%APPDATA%` typically resolves to `C:\Users\[username]\AppData\Roaming\`.

### Redirecting the database

The database path can be changed in two ways:

1. **Settings UI**: Navigate to **Settings → Database location** and select a new path.
2. **Environment variable**: Set `JURA_DB_PATH` to the desired full path before launching the application.

```powershell
# Set via system environment variable (requires admin, persists for all users)
[Environment]::SetEnvironmentVariable("JURA_DB_PATH", "D:\TeamData\jura_archive.db", "Machine")
```

```powershell
# Set via user environment variable (no admin, applies to current user only)
[Environment]::SetEnvironmentVariable("JURA_DB_PATH", "\\server\share\jura_archive.db", "User")
```

**Shared team databases on network shares** are supported. Point `JURA_DB_PATH` to a UNC path accessible to all team members. Be aware that concurrent writes from multiple users are serialised by SQLite's locking mechanism — this is suitable for small teams (up to approximately 10 concurrent users). Larger deployments should provision individual databases per user.

---

## Network requirements

Jura Trace is a local-first application. All processing occurs on-device. The table below lists every port the application uses.

| Port | Protocol | Binding | Purpose | Required |
|---|---|---|---|---|
| 8200 | TCP | 127.0.0.1 (loopback) | ML sidecar communication | Yes (forensic features) |
| 11434 | TCP | 127.0.0.1 (loopback) | Ollama LLM runtime | No (optional AI features) |

No external network access is required for core functionality. Both ports bind exclusively to the loopback interface — traffic cannot originate from or reach external hosts.

**Optional outbound access** (not enabled by default, not required in the pilot release):

- Reverse image search APIs will be available in Professional+ licence tiers (Phase 4). These require outbound HTTPS. They are opt-in per-verification and disabled by default.

If your outbound firewall filters by destination, no rules are needed for the current pilot release. You may wish to document that future Professional+ features will require outbound HTTPS to search provider endpoints — specific domains will be published in the v1.0 network requirements reference.

---

## Environment variables

| Variable | Purpose | Example value |
|---|---|---|
| `JURA_DB_PATH` | Override the default database location | `D:\TeamData\jura_archive.db` |
| `JURA_SIDECAR_KEY` | API key for sidecar authentication (auto-generated on first run; override only if needed) | 32-character hex string |

`JURA_SIDECAR_KEY` is generated automatically and stored in `%APPDATA%\Jura Trace\` on first launch. You do not need to set it manually in standard deployments. Override it only if your security policy requires pre-provisioned secrets.

---

## Uninstallation

### Via Add or Remove Programs

Navigate to **Settings → Apps → Installed apps**, search for "Jura Trace", and select **Uninstall**.

### Silent uninstall (MSI)

```powershell
msiexec /x {ProductCode} /qn /norestart
```

Replace `{ProductCode}` with the GUID from the MSI. To retrieve it from an installed machine:

```powershell
Get-WmiObject -Query "SELECT * FROM Win32_Product WHERE Name LIKE 'Jura Trace%'" | Select-Object IdentifyingNumber, Version
```

### Silent uninstall (NSIS)

```powershell
& "$env:LOCALAPPDATA\Programs\Jura Trace\uninstall.exe" /S
```

### Data retention after uninstall

The uninstaller removes the application binary but does not delete user data. The following locations remain after uninstallation and must be removed manually if required by your data retention policy:

- `%APPDATA%\Jura Trace\` — database and settings
- `%LOCALAPPDATA%\Programs\Jura Trace\` — may contain residual files on per-user installs

To remove all user data silently:

```powershell
Remove-Item "$env:APPDATA\Jura Trace" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item "$env:LOCALAPPDATA\Programs\Jura Trace" -Recurse -Force -ErrorAction SilentlyContinue
```

Run this after the uninstaller completes. The `-ErrorAction SilentlyContinue` flag prevents the script failing if either path does not exist.

---

*Jura Trace is developed by Juralabs Community Interest Company (UK) and is released under the PolyForm Noncommercial 1.0.0 licence. For deployment queries, contact your Juralabs pilot coordinator.*
