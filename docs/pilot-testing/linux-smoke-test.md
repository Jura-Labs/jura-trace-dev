# Jura Trace — Linux AppImage Smoke Test

**Version tested**: _(fill in, e.g. 0.4.0)_
**Tester**: _(name)_
**Date**: _(YYYY-MM-DD)_
**Distribution**: _(e.g. Ubuntu 22.04 LTS / Fedora 39)_
**Desktop environment**: _(e.g. GNOME 44 / KDE Plasma 5.27)_
**Display server**: _(Wayland / X11)_

---

## Prerequisites

- Ubuntu 22.04 LTS or later, OR Fedora 37 or later
- At least 4 GB RAM, 2 GB free disk space
- The AppImage file downloaded from the GitHub Release page
- Internet access is NOT required — Jura Trace is local-first

---

## 1. Download and Launch

- [ ] Download `jura-trace_{version}_amd64.AppImage` from the GitHub Releases page
- [ ] Verify the SHA-256 checksum matches the `.sha256` file published alongside the release:
  ```
  sha256sum jura-trace_{version}_amd64.AppImage
  ```
- [ ] Make the file executable:
  ```
  chmod +x jura-trace_{version}_amd64.AppImage
  ```
- [ ] Launch the AppImage:
  ```
  ./jura-trace_{version}_amd64.AppImage
  ```
- [ ] Confirm the application window opens within 10 seconds
- [ ] Confirm the window title reads "Jura Trace — Know What's Real"
- [ ] Confirm the minimum window size constraint is respected (cannot resize below 900 x 600 px)

---

## 2. Icon and Window Chrome

- [ ] The application icon (concentric-circle eye mark) appears in the taskbar / dock
- [ ] The application icon appears in the Alt+Tab switcher
- [ ] The window can be resized, minimised, maximised, and restored without visual artefacts

---

## 3. Navigation

Test each navigation link in the top menu bar. Each should load without a blank screen or console error.

- [ ] Dashboard (home / root route `/`)
- [ ] Protect (`/protect`)
- [ ] Verify (`/verify`)
- [ ] Monitor (`/monitor`)
- [ ] Settings (`/settings`)
- [ ] Help (`/help`, if present — skip if not yet implemented)

On each page, confirm:
- [ ] Headings render in Georgia or a serif fallback (not a monospaced or sans-serif font)
- [ ] Obsidian background (`#1E2128`) and Quartz text (`#EDEAE4`) are visible — not default browser white/black
- [ ] No broken layout (content overflowing viewport, overlapping elements)

---

## 4. Settings Page

- [ ] Open Settings (`/settings`)
- [ ] The "Service Status" section is visible
  - [ ] The Python ML sidecar status shows "Offline" (expected — sidecar is not bundled in this build)
  - [ ] The Ollama status shows "Offline" (expected unless Ollama is running locally)
  - [ ] No crash or error panel when services are offline — graceful degradation message shown
- [ ] The "Database Location" section shows a valid path under `~/.local/share/` or `~/.config/`
  - [ ] The path is not a Windows-style path (`C:\...`) or macOS path (`/Users/...`)
  - [ ] Example of a valid path: `/home/{username}/.local/share/org.juralabs.trace/jura-trace.db`

---

## 5. File Input — Protect Page

- [ ] Navigate to Protect (`/protect`)
- [ ] The drop zone is visible and labelled
- [ ] Drag a JPEG or PNG file onto the drop zone — the file name or thumbnail appears
- [ ] Click the "Browse" button (or equivalent) — the native Linux file chooser dialog opens
  - [ ] The dialog is a GTK file chooser, not a blank or broken native dialog
  - [ ] Selecting a file closes the dialog and populates the file name in the UI
- [ ] Confirm no crash when selecting a file larger than 10 MB

---

## 6. File Input — Verify Page

- [ ] Navigate to Verify (`/verify`)
- [ ] The drop zone is visible
- [ ] Drop a test image onto the drop zone
- [ ] The analysis pipeline starts (loading indicator visible)
- [ ] With the sidecar offline, the result shows the Rust-only analysis (C2PA, EXIF, fingerprint)
  - [ ] No unhandled error or white screen
  - [ ] The verdict panel renders with a label (Authentic / Inconclusive / Synthetic)

---

## 7. Responsive Layout

- [ ] Resize the window to approximately 900 px wide (the declared minimum)
  - [ ] Navigation does not overflow or clip
  - [ ] Content columns stack vertically if applicable
- [ ] Resize to 1400+ px wide
  - [ ] Content is centred within the 900 px max-width editorial column
  - [ ] No horizontal scroll bar appears

---

## 8. Font Rendering

- [ ] Page headings use a serif font (Georgia is the target; if not installed, the system serif fallback is acceptable)
- [ ] Body text is readable at default system DPI
- [ ] Text renders correctly at 125% and 150% system scaling (if your display uses fractional scaling)
  - To test: System Settings > Display > Scale

---

## 9. Wayland vs X11 (if both display servers are available)

Run both of the following and record results:

**Wayland:**
```
WAYLAND_DISPLAY=wayland-0 ./jura-trace_{version}_amd64.AppImage
```
or on a Wayland session, just launch normally.

**X11 / XWayland:**
```
GDK_BACKEND=x11 ./jura-trace_{version}_amd64.AppImage
```

For each:
- [ ] Window opens without crash
- [ ] Window chrome renders correctly (title bar, close/minimise/maximise buttons)
- [ ] File dialog opens and works
- [ ] No visible tearing or GPU artefacts

Record any differences between display servers in the notes section below.

---

## 10. .deb Package (optional — if testing the Debian package instead of AppImage)

If testing `jura-trace_{version}_amd64.deb`:

- [ ] Install with:
  ```
  sudo apt install ./jura-trace_{version}_amd64.deb
  ```
  Confirm `apt` resolves the declared runtime dependencies (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`) without errors.
- [ ] The application appears in the GNOME/KDE application menu under the "Utility" category
- [ ] The `.desktop` launcher entry shows:
  - Name: `Jura Trace`
  - Icon: the eye mark (not a generic placeholder)
- [ ] Launch from the application menu — window opens correctly
- [ ] Uninstall cleanly:
  ```
  sudo apt remove jura-trace
  ```
  Confirm no leftover files in `/usr/bin/` or `/opt/`

---

## 11. Accessibility Basics

- [ ] Tab key cycles through interactive elements in a logical order
- [ ] Pressing Enter or Space activates focused buttons
- [ ] Focus ring is visible on interactive elements
- [ ] Screen reader (Orca): launch with `orca &`, then open the app. Confirm the main heading ("Jura Trace") is announced when the window opens.

---

## Known Limitations (do not file as bugs)

- The Python ML sidecar (`jura-sidecar`) is not bundled in v0.4.0 builds. Forensic analysis (ELA, deepfake, noise) will show as unavailable. This is expected.
- Ollama integration requires a separately installed Ollama instance on port 11434. Absence is handled gracefully.
- The `models/deepfake_classifier.joblib` file is not bundled in the AppImage. Classifier-based deepfake scoring will fall back to the heuristic path.

---

## Notes

_(Record any unexpected behaviour, environment-specific issues, or display server differences here.)_

---

## Result

- [ ] PASS — all critical checks (sections 1–7) pass
- [ ] PASS WITH NOTES — minor issues recorded above, nothing blocking
- [ ] FAIL — blocking issue found (describe in notes)
