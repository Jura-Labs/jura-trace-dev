---
title: "Installing Jura Trace on Linux"
description: "System requirements, package installation steps, and optional dependency setup for running Jura Trace on Ubuntu and Fedora."
last-updated: 24 March 2026
status: internal
---

# Installing Jura Trace on Linux

This guide covers installing Jura Trace on supported Linux distributions. Jura Trace is a local-first desktop application — all processing happens on your machine, with no data sent to external servers.

---

## Contents

1. [Requirements](#requirements)
2. [Installation](#installation)
3. [System dependencies](#system-dependencies)
4. [Optional: FFmpeg for video and audio analysis](#optional-ffmpeg-for-video-and-audio-analysis)
5. [Font rendering](#font-rendering)
6. [Wayland support](#wayland-support)
7. [Verifying the installation](#verifying-the-installation)
8. [Data location](#data-location)
9. [Reporting issues](#reporting-issues)

---

## Requirements

| Requirement | Details |
|---|---|
| Distribution | Ubuntu 22.04 LTS or later; Fedora 37 or later |
| Processor | x86-64 (64-bit Intel or AMD) |
| RAM | 4 GB minimum (8 GB recommended for forensic analysis) |
| Disk space | 500 MB for the application; additional space for your asset database |
| WebKit | WebKitGTK 4.1 — required; see [System dependencies](#system-dependencies) |

> **Important**: Ubuntu 20.04 and earlier are **not supported**. Those releases ship with WebKitGTK 4.0, which is not compatible with Jura Trace. Please upgrade to Ubuntu 22.04 LTS or later before installing.

Other distributions (Arch Linux, openSUSE, etc.) may work if WebKitGTK 4.1 and its dependencies are available, but are not officially supported for the pilot.

---

## Installation

### AppImage (recommended for most users)

AppImage is a self-contained format that requires no system-wide installation and bundles all application dependencies.

1. Download `Jura-Trace_x.x.x_amd64.AppImage` from the link your Juralabs contact has shared.
2. Open a terminal and make the file executable:

```bash
chmod +x Jura-Trace_x.x.x_amd64.AppImage
```

3. Run the application:

```bash
./Jura-Trace_x.x.x_amd64.AppImage
```

You can also double-click the AppImage in your file manager if your desktop environment supports it. No further installation is required.

**To create a desktop shortcut:** Most file managers will offer to integrate the AppImage when you first run it, or you can use a tool such as `AppImageLauncher`.

---

### Debian/Ubuntu package (.deb)

If you prefer system-wide installation on a Debian-based distribution:

```bash
sudo dpkg -i jura-trace_x.x.x_amd64.deb
sudo apt install -f
```

The second command installs any missing dependencies that `dpkg` cannot resolve on its own. After this completes, Jura Trace will appear in your applications menu.

**To uninstall:**

```bash
sudo apt remove jura-trace
```

---

### RPM package (.rpm)

For Fedora and other RPM-based distributions:

```bash
sudo rpm -i jura-trace_x.x.x_x86_64.rpm
```

Or using `dnf` for automatic dependency resolution:

```bash
sudo dnf install ./jura-trace_x.x.x_x86_64.rpm
```

---

## System dependencies

### Required: WebKitGTK 4.1

Jura Trace uses WebKitGTK 4.1 to render its interface. The AppImage bundles this dependency, so AppImage users can skip this step. If you installed the `.deb` or `.rpm` and the application fails to launch, install WebKitGTK manually.

**Ubuntu/Debian:**

```bash
sudo apt install libwebkit2gtk-4.1-0
```

**Fedora:**

```bash
sudo dnf install webkit2gtk4.1
```

If you are unsure which version of WebKitGTK you have installed:

```bash
# Ubuntu/Debian
dpkg -l | grep webkit

# Fedora
rpm -qa | grep webkit
```

---

## Optional: FFmpeg for video and audio analysis

Video metadata extraction, frame-by-frame forensic analysis, and audio analysis all require FFmpeg. Image verification works without it. If FFmpeg is not installed, video and audio features will be gracefully disabled in the interface — no errors will occur.

**Ubuntu/Debian:**

```bash
sudo apt install ffmpeg
```

**Fedora:**

```bash
sudo dnf install ffmpeg
```

> **Note**: On Fedora, FFmpeg is available from the RPM Fusion repository. If the command above fails, enable RPM Fusion first:
>
> ```bash
> sudo dnf install https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
> sudo dnf install ffmpeg
> ```

**To confirm FFmpeg is available:**

```bash
ffmpeg -version
```

You should see version information. If you see "command not found", the installation did not complete successfully.

---

## Font rendering

Jura Trace uses Georgia serif for headings. On Linux, this falls back to DejaVu Serif or Noto Serif if Georgia is not available. Both are usually pre-installed, but if headings appear in a sans-serif font, install DejaVu:

**Ubuntu/Debian:**

```bash
sudo apt install fonts-dejavu
```

**Fedora:**

```bash
sudo dnf install dejavu-serif-fonts
```

After installing, there is no need to restart the application — fonts are loaded at launch.

---

## Wayland support

Jura Trace works on both X11 and Wayland. Most users do not need to adjust anything.

If you experience rendering issues such as a blank window, incorrect scaling, or missing interface elements on a Wayland session, try forcing X11 compatibility mode:

**AppImage:**

```bash
GDK_BACKEND=x11 ./Jura-Trace_x.x.x_amd64.AppImage
```

**System installation:**

```bash
GDK_BACKEND=x11 jura-trace
```

To check which display server your session is using:

```bash
echo $XDG_SESSION_TYPE
```

This will output `wayland` or `x11`. If you are on Wayland and the application behaves unexpectedly, please include this information when reporting issues.

---

## Verifying the installation

Once Jura Trace is running, confirm the core components are working:

1. Launch Jura Trace from your applications menu or the terminal.
2. Navigate to **Settings** using the navigation menu.
3. Under **Analysis services**, check the status reads **Online**.
4. Under **Database location**, confirm a path is shown — it should be within `~/.local/share/jura-trace/`.
5. Navigate to **Verify** and try dropping in a test image. A result (even an inconclusive one) confirms the pipeline is working.

If **Analysis services** shows **Offline**, the ML sidecar did not start. This can happen if a required Python dependency is missing. In this case, please report the issue with the terminal output from your launch command.

---

## Data location

Jura Trace stores all data locally on your machine. Nothing is sent to external servers.

| Data | Default location |
|---|---|
| Asset database | `~/.local/share/jura-trace/jura_archive.db` |
| Application settings | `~/.local/share/jura-trace/` |
| Logs | `~/.local/share/jura-trace/` |

You can change the database location at any time in **Settings** → **Database location**. This is useful if you want to store the database on an external drive or a separate partition.

---

## Reporting issues

If you encounter problems during installation or first use, please report them to your Juralabs pilot contact with the following details:

- Your Linux distribution and version:

```bash
cat /etc/os-release
```

- Your desktop environment (GNOME, KDE Plasma, XFCE, etc.)
- Whether you are using Wayland or X11:

```bash
echo $XDG_SESSION_TYPE
```

- The exact error message or terminal output when launching Jura Trace
- Whether you used AppImage, `.deb`, or `.rpm`
- The output of `ffmpeg -version` if the issue relates to video or audio features

---

*Jura Trace is developed by Juralabs Community Interest Company (UK) and is released under the PolyForm Noncommercial 1.0.0 licence for pilot use. For queries, contact your Juralabs pilot coordinator.*
