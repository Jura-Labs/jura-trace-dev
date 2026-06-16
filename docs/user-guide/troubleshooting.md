---
title: "Jura Trace Troubleshooting"
description: "Fixes for the most common first-run problems in Jura Trace, starting with the Analysis Engine offline banner."
last-updated: 15 June 2026
status: published
---

# Troubleshooting

Most first-run problems come down to the Analysis Engine not starting. Work through this page top to bottom.

## "Analysis Engine offline"

The Analysis Engine is a local helper process that runs the forensic and AI detectors. Jura Trace still works without it, but with fewer checks (C2PA and EXIF only). Here is how to bring it back online.

**First, give it time.** On a cold start the engine can take 30 to 90 seconds to load its models, longer on older machines or with power-saver mode on. Wait, then run one verification. If the banner clears, nothing is wrong.

**If it does not clear after two minutes:**

1. **Check it is not blocked by security software.**
   - *Windows*: Windows Defender or a third-party antivirus may quarantine the engine on first launch. Allow the Jura Trace helper process if prompted, then restart the app. See the [Windows install guide](../install-guides/quickstart-windows.md) for the exact Defender exclusion steps.
   - *macOS*: if you installed by dragging from the DMG, open the app once via right-click then **Open** to clear Gatekeeper. The engine starts after the main window.

2. **Restart the app.** Quit fully (not just close the window) and reopen. The engine is restarted with the app.

3. **Check the port is free.** The engine listens on a local port. If another copy of Jura Trace is already running, close it. Only run one instance at a time.

4. **Reboot.** A pending OS update or a stuck previous process is cleared by a restart.

5. **Still offline?** Open the **Settings** tab and check the Service Status card. It shows whether the Analysis Engine is reachable and its startup phase. Note the status shown there and report it (see below). You can keep using Verify with C2PA and EXIF checks in the meantime.

## The app will not open at all

- *macOS*: "Jura Trace cannot be opened because the developer cannot be verified" means Gatekeeper has not been cleared. Right-click the app, choose **Open**, then **Open** again. This is only needed once.
- *Windows*: a SmartScreen "Windows protected your PC" dialog can appear on first run. Choose **More info**, then **Run anyway**. The installer is signed by Jura Labs CIC.

## A genuine photo scored "Uncertain"

This is usually correct behaviour, not a fault. Re-saving, screenshotting, cropping, or sending a photo through a messaging app strips metadata and re-compresses the pixels, which reduces the signals the detectors rely on. An "Uncertain" result means Jura Trace could not find strong evidence either way. It is not a finding of "fake". Verify the original file where you can.

## Reporting a problem

Jura Trace sends no telemetry, so we only learn about issues you report. Use the in-app feedback option (it pre-fills the app version, platform, and a reason code, with no file contents), or email feedback@juralabs.org directly. Include your platform, the app version (Settings tab), and the Service Status shown if the engine is offline.
