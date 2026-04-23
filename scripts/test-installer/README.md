# Installer testing VMs

Virtual-machine harness for testing Jura Trace installers on Linux and
Windows from macOS, addressing the platform-testing blind spot flagged in
Claude memory (`feedback_platform_testing.md`).

**Goal:** catch Windows and Linux installer regressions *before* tagging a
release, reducing the burn rate on CI tags (~£1.10 each) and pilot-user
trust damage from broken RCs.

## Stack

| Tool | Purpose | Scope |
|---|---|---|
| UTM | Free QEMU/Apple Virtualization front-end. Hosts Ubuntu + Windows VMs. | Full installer UX, GUI testing |
| OrbStack | Lightweight Linux container/VM runtime. Near-instant launch. | Headless sidecar tests, AppImage smoke |
| VM storage | `/Volumes/MAC SSD/dev/vms/` | Keeps 80–160 GB of VM disks off the internal drive |
| Installer artefacts | `/Volumes/MAC SSD/artefacts/releases/` | Shared with VMs as a read-only mount |

---

## One-time setup

### 1. Install UTM + OrbStack

Already done via Homebrew (`brew install --cask utm orbstack`). If not,
run:

```sh
brew install --cask utm orbstack
```

### 2. Set VM storage to the SSD

Open UTM → Settings → change "VM directory" to
`/Volumes/MAC SSD/dev/vms/`.

OrbStack stores state under `~/.orbstack` by default; override with:

```sh
mkdir -p "/Volumes/MAC SSD/dev/orbstack"
# Then in OrbStack → Settings → Machines → Data location → point to
# /Volumes/MAC SSD/dev/orbstack
```

### 3. Create the Ubuntu ARM64 VM (UTM)

The ISO is at `/Volumes/MAC SSD/artefacts/iso/ubuntu-24.04.3-live-server-arm64.iso`.

1. UTM → Create new VM → Virtualize → Linux
2. ISO: select the downloaded Ubuntu 24.04 ARM64 server image
3. RAM: 4096 MB, CPU cores: 2
4. Disk: 40 GB dynamic
5. Shared directory: `/Volumes/MAC SSD/artefacts/releases` (read-only)
6. Boot, run through the Ubuntu Server installer (OpenSSH yes, snaps no)
7. After first boot:
   ```sh
   sudo apt update && sudo apt install -y xfce4 xfce4-goodies libfuse2
   sudo systemctl set-default graphical.target
   sudo reboot
   ```
   `libfuse2` is required for AppImages. XFCE is lighter than GNOME (~1 GB
   RAM idle vs ~2 GB).
8. UTM → snapshot the VM as **clean-baseline** so you can revert after
   each test.

### 4. Create the Windows 11 ARM64 VM (UTM)

Requires a Microsoft account to download the ISO. Not automatable.

1. Visit <https://www.microsoft.com/en-us/software-download/windowsinsiderpreviewARM64>
2. Sign in with your Microsoft account (Jura Labs CIC account preferred)
3. Download the Windows 11 Insider Preview ARM64 VHDX
4. Save to `/Volumes/MAC SSD/artefacts/iso/windows-11-arm64.vhdx`
5. UTM → Create new VM → Virtualize → Windows → Import VHDX
6. RAM: 8192 MB, CPU cores: 4 (Windows needs more than Linux)
7. Disk: 80 GB dynamic (or use imported VHDX as-is)
8. Boot, complete Windows OOBE
9. Install the Microsoft **Prism** x64-emulation layer (auto-detected on
   first run of an x64 MSI) — Windows 11 ARM comes with this built in
10. UTM → snapshot as **clean-baseline**

### 5. Install OrbStack Ubuntu machine (for headless tests)

```sh
orb create --arch arm64 ubuntu:24.04 jura-linux
orb -m jura-linux -- sudo apt update && sudo apt install -y libfuse2
```

OrbStack boots this in ~3 seconds once created.

---

## Running a test

**Before tagging a release**, run the pre-tag check:

```sh
scripts/test-installer/pre-tag-check.sh path/to/installer.msi
```

This stages the installer into `artefacts/releases/`, snapshots/reverts
the relevant VM, and prints next steps for the manual GUI test.

For sidecar-only changes (no Tauri shell rebuild), OrbStack is faster:

```sh
scripts/test-installer/test-linux-headless.sh
```

This runs the sidecar pytest suite inside OrbStack's Ubuntu container
against the current working tree.

---

## What this replaces

- **Per-RC CI Windows tags** — drop from 5–6 per release cycle to ~2
  (final signed validation only). Saves ~£4 per cycle.
- **Pilot-user bug reports** — catch "installer silently fails" and
  "app crashes on launch" before public RC.

## What this does NOT replace

- **Signed binary validation** — the signed MSI interacts differently
  with Windows SmartScreen than an unsigned dev build. Keep one CI run
  per tagged release for end-to-end cert validation.
- **Real hardware GPU/webview quirks** — VM display drivers differ from
  real hardware. If a user reports a rendering bug that doesn't repro in
  the VM, it's probably a real-hardware issue.

## Scheduled use

No cron/launchd job — VM tests are run manually before tagged releases,
not on every commit. Automating via guest agents is possible later if the
manual workflow gets tedious.
