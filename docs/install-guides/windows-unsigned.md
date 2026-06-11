---
title: "Installing Jura Trace on Windows"
description: "Step-by-step installation guide for pilot testers running Jura Trace on Windows 10 or Windows 11."
last-updated: 31 March 2026
status: internal
---

# Installing Jura Trace on Windows

This guide is for pilot testers installing Jura Trace. From RC4 onwards, Windows builds are signed with a Microsoft Azure Trusted Signing certificate. Most users will not see a SmartScreen warning.

> **Note**: SmartScreen reputation is built over time. During the first few signed releases, Windows may still display a brief warning until publisher reputation is fully established. See [SmartScreen](#windows-smartscreen) below if this occurs.

---

## Contents

1. [Requirements](#requirements)
2. [Downloading the installer](#downloading-the-installer)
3. [Windows SmartScreen](#windows-smartscreen)
4. [Windows Firewall prompt](#windows-firewall-prompt)
5. [WebView2 Runtime](#webview2-runtime)
6. [Optional: FFmpeg for video and audio analysis](#optional-ffmpeg-for-video-and-audio-analysis)
7. [Verifying the installation](#verifying-the-installation)
8. [Data location](#data-location)
9. [Reporting issues](#reporting-issues)

---

## Requirements

Before you begin, check that your system meets these requirements:

| Requirement | Minimum |
|---|---|
| Operating system | Windows 10 (version 1803 or later) or Windows 11 |
| Processor | x86-64 (Intel or AMD, 64-bit) |
| RAM | 4 GB (8 GB recommended for forensic analysis) |
| Disk space | 500 MB for the application; additional space for your asset database |
| WebView2 Runtime | Included with Windows 11; may need installing separately on Windows 10 |

---

## Downloading the installer

Jura Trace is distributed as either an `.exe` setup file or an `.msi` package:

- `Jura-Trace_x.x.x_x64-setup.exe` — recommended for most users
- `Jura-Trace_x.x.x_x64_en-US.msi` — for managed enterprise environments

Download the file your Juralabs contact has shared with you and save it to a location you can find easily (for example, your Downloads folder).

---

## Windows SmartScreen

From v0.9.0-rc.4, Windows installers are signed with a Microsoft Azure Trusted Signing certificate. Most users will be able to install without any SmartScreen warning.

**If SmartScreen still displays a warning**, this is because the publisher reputation has not yet been fully established with Microsoft. This is normal for newly signed applications and resolves after a small number of downloads.

**Steps if a warning appears:**

1. Double-click the installer file.
2. If the SmartScreen dialog appears, click **More info**.
3. Confirm the publisher is shown as **Juralabs Community Interest Company** (or your expected publisher name). This confirms the certificate is valid.
4. Click **Run anyway**.
5. If User Account Control (UAC) then prompts you to allow the installer to make changes to your device, click **Yes**.
6. Follow the on-screen installation steps. The default install location (`C:\Program Files\Jura Trace\`) is correct for most users.

### Managed devices

Some organisations enforce SmartScreen blocking via Group Policy. If the **Run anyway** button does not appear, contact your IT department and request one of the following:

- A temporary exception for `Jura-Trace_x.x.x_x64-setup.exe` (or the `.msi` equivalent).
- Addition of `Jura Trace` to the organisation's approved applications list.

The installer is signed — provide your IT contact with the file hash (SHA-256) from the release page to verify integrity.

---

## Windows Firewall prompt

On first launch, Windows Firewall may ask whether to allow Jura Trace's analysis service to communicate on your network. This service is the ML sidecar — a local component that performs forensic image and video analysis entirely on your machine.

When prompted:

- Select **Allow access on private networks only**.
- Do **not** select "Public networks" unless your IT policy requires it.

The analysis service communicates exclusively on your local machine (`127.0.0.1`, port `8200`). It does not connect to the internet. Jura Trace is a local-first application — no data leaves your device.

If you accidentally click **Cancel** or **Block**, forensic analysis features will be unavailable. To correct this:

1. Open **Windows Defender Firewall** from the Start menu.
2. Click **Allow an app or feature through Windows Defender Firewall**.
3. Click **Change settings**, then locate **Jura Trace** in the list.
4. Tick the **Private** checkbox and click **OK**.

---

## WebView2 Runtime

Jura Trace uses the Microsoft Edge WebView2 Runtime to display its interface. This runtime is included by default on Windows 11. On Windows 10, it may not be present.

**To check:** Launch Jura Trace after installation. If the application window appears blank or fails to open, WebView2 is likely missing.

**To install WebView2 on Windows 10:**

1. Visit [developer.microsoft.com/en-us/microsoft-edge/webview2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
2. Download the **Evergreen Standalone Installer** (the "x64" version for most machines).
3. Run the installer and follow the prompts.
4. Restart Jura Trace.

You do not need to update WebView2 manually — once installed, the Evergreen runtime updates itself automatically in the background.

---

## Optional: FFmpeg for video and audio analysis

Video metadata extraction, frame-by-frame forensic analysis, and audio analysis all require FFmpeg — a free, open-source media tool. Image verification works without it.

**To install FFmpeg:**

1. Download a Windows build from [ffmpeg.org/download.html](https://ffmpeg.org/download.html). Under "Get packages & executable files", choose **Windows builds from gyan.dev** or **BtbN/FFmpeg-Builds** on GitHub.
2. Download the **essentials** or **full** build (a `.zip` file).
3. Extract the `.zip` to a permanent location, for example `C:\ffmpeg\`.

   [Screenshot: Windows Explorer showing the extracted ffmpeg folder containing bin/, doc/, and presets/ subfolders]

4. Add `C:\ffmpeg\bin` to your system PATH:
   - Press `Windows + S` and search for **Environment Variables**.
   - Click **Edit the system environment variables**.
   - In the System Properties window, click **Environment Variables**.
   - Under "System variables", select **Path** and click **Edit**.
   - Click **New** and type `C:\ffmpeg\bin`.
   - Click **OK** on each open window to save.
5. Restart Jura Trace.

**To verify FFmpeg is installed:** Open a new Command Prompt window and type `ffmpeg -version`. If you see version information, the installation was successful.

If FFmpeg is not installed, Jura Trace will still work for image analysis. Video and audio features will appear as unavailable in the interface.

---

## Verifying the installation

Once Jura Trace is installed and running, confirm everything is working:

1. Launch Jura Trace from the Start menu or desktop shortcut.
2. Navigate to **Settings** using the navigation menu.
3. Under **Analysis services**, check the status reads **Online**. If it reads **Offline**, see the [Windows Firewall prompt](#windows-firewall-prompt) section above.
4. Under **Database location**, confirm a path is shown — it should be within `%APPDATA%\Jura Trace\`.
5. Navigate to **Verify** and try dropping in a test image. A result (even an inconclusive one) confirms the pipeline is working.

[Screenshot: Settings page showing Analysis services status "Online" and the database path field]

---

## Data location

Jura Trace stores all data locally on your machine. Nothing is sent to external servers.

| Data | Default location |
|---|---|
| Asset database | `%APPDATA%\Jura Trace\jura_archive.db` |
| Application settings | `%APPDATA%\Jura Trace\` |

You can change the database location at any time in **Settings** → **Database location**. This is useful if you want to store the database on an external drive or a shared network location.

`%APPDATA%` typically resolves to `C:\Users\[your username]\AppData\Roaming\`.

---

## Reporting issues

If you encounter problems during installation or first use, please report them to your Juralabs pilot contact with the following details:

- Your Windows version: **Settings** → **System** → **About** → "Windows specifications"
- The exact error message or a screenshot of what you see
- Whether your machine is managed by an organisation (for example, domain-joined or enrolled in Intune)
- Whether the SmartScreen "Run anyway" option was visible
- If Jura Trace launched but analysis failed, the status shown under **Settings** → **Analysis services**

---

*Jura Trace is developed by Juralabs Community Interest Company (UK) and is released under the AGPL-3.0-or-later licence. For queries, contact your Juralabs pilot coordinator.*
