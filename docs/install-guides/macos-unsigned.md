# Installing Jura Trace on macOS (Unsigned Build)

This guide is for pilot testers installing a pre-release build that has not yet been
signed with an Apple Developer ID certificate. Signed, notarised builds will be
available from v1.0 onwards.

---

## Requirements

- macOS 13 (Ventura) or later — macOS 14 (Sonoma) and 15 (Sequoia) are fully supported
- Apple Silicon (M1 / M2 / M3 / M4) **or** Intel Mac (x86_64)
- ~200 MB of free disk space for the application bundle

---

## Step 1 — Download the correct DMG

| Your Mac | File to download |
|---|---|
| Apple Silicon (M1/M2/M3/M4) | `Jura-Trace_x.x.x_aarch64.dmg` |
| Intel | `Jura-Trace_x.x.x_x64.dmg` |

Not sure which you have? Choose **Apple menu** → **About This Mac**. If Chip shows
"Apple M1" (or M2/M3/M4), you have Apple Silicon. If it shows an Intel Core processor,
download the Intel DMG.

---

## Step 2 — Mount and install

1. Double-click the downloaded DMG to mount it.
2. A Finder window opens showing the **Jura Trace** application icon and an
   **Applications** folder shortcut.
3. Drag **Jura Trace** onto the **Applications** shortcut.
4. Eject the DMG (drag it to the Trash or press Cmd+E).

---

## Step 3 — First launch: bypassing Gatekeeper

Because this build is unsigned, macOS Gatekeeper will block it on first launch. Use
one of the three methods below. You only need to do this **once** — subsequent launches
work normally.

### Method 1 — Right-click open (recommended, works on all macOS versions)

1. Open **Finder** and go to **Applications**.
2. **Right-click** (or Control-click) on **Jura Trace**.
3. Select **Open** from the context menu.
4. In the dialog that appears, click **Open**.

### Method 2 — Privacy & Security settings (required on macOS Sequoia if Method 1 fails)

1. Attempt to open Jura Trace normally — it will be blocked and a notification will appear.
2. Open **System Settings** → **Privacy & Security**.
3. Scroll down to the Security section and find "Jura Trace was blocked from use because
   it is not from an identified developer."
4. Click **Open Anyway**.
5. Authenticate with your password or Touch ID when prompted.

### Method 3 — Terminal (fastest if you are comfortable with the command line)

```bash
xattr -d com.apple.quarantine /Applications/Jura\ Trace.app
```

Then open Jura Trace normally from Finder or Spotlight.

---

## Step 4 — Verify the installation

1. Launch Jura Trace.
2. Navigate to **Settings** (bottom-left navigation).
3. Confirm the following:
   - **Version** shows the expected build number.
   - **Database location** shows a path under `~/Library/Application Support/org.juralabs.trace/`.
   - **Analysis services** shows "Online" if the Python ML sidecar is bundled and started
     correctly, or "Offline" if you are running a Rust-only build (image verification still
     works; ML forensics require the sidecar).

---

## Optional: FFmpeg for video and audio analysis

Video metadata, frame extraction, audio metadata, and audio transcription all require
FFmpeg. If FFmpeg is not installed, these features are gracefully disabled — Jura Trace
will still analyse images and perform C2PA verification without it.

Install via Homebrew:

```bash
brew install ffmpeg
```

If you do not have Homebrew: https://brew.sh

---

## Optional: Ollama for AI-assisted features

RAG claim verification and Tier 3 image descriptions use Ollama running locally on
port 11434. These are optional enhancements — the core verification pipeline does not
require them.

1. Download Ollama from https://ollama.com and install it.
2. Pull the required models:

```bash
ollama pull qwen2.5      # for claim verification (~4 GB)
ollama pull llava        # for image description (~4 GB)
```

3. Ollama must be running before you open Jura Trace for these features to activate.

---

## Troubleshooting

### "Jura Trace is damaged and can't be opened"

This message appears when the quarantine flag is set but Gatekeeper rejects it
outright (common on Sequoia with strict security settings). Remove the quarantine
attribute via the terminal:

```bash
xattr -d com.apple.quarantine /Applications/Jura\ Trace.app
```

### "You don't have permission to open the application"

The binary may have lost its executable bit during download. Restore it:

```bash
chmod +x /Applications/Jura\ Trace.app/Contents/MacOS/jura-trace
```

### Analysis services shows "Offline" unexpectedly

If this is a build that bundles the sidecar, the sidecar binary may have been
quarantined separately. Clear it:

```bash
xattr -cr /Applications/Jura\ Trace.app
```

The `-r` flag applies recursively to all files inside the app bundle, including the
bundled `jura-sidecar` binary.

### The app launches but the window is blank

This can happen if the SvelteKit frontend assets are missing from the bundle. Check:

```bash
ls /Applications/Jura\ Trace.app/Contents/Resources/
```

You should see a `_up_` directory (the embedded SvelteKit build). If it is absent,
re-download the DMG — the build may have been corrupted during download.

---

## Known limitations in unsigned pre-release builds

| Limitation | Notes |
|---|---|
| Gatekeeper prompt on first launch | Expected. Use one of the three methods above. One-time action. |
| No auto-update | Auto-updater requires a signed build. Download new DMGs manually from the releases page. |
| macOS may ask for keychain access | Allow it — Jura Trace stores no credentials in the keychain; this is a Tauri webview artefact on some macOS versions. |

---

## Reporting issues

Please include the following when reporting a problem:

- Your macOS version (**Apple menu** → **About This Mac**)
- Processor type (Apple Silicon or Intel)
- Jura Trace version (**Settings** → version number)
- The exact error message, Gatekeeper dialog text, or a screenshot
- Whether the issue is reproducible consistently or intermittent

Report issues via the project issue tracker or email pilot@juralabs.org.

---

*Jura Trace is developed by Juralabs Community Interest Company (UK). Licenced under AGPL-3.0-or-later. Local-first — no data leaves your machine.*
