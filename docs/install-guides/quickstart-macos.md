# Jura Trace — Quick Start (macOS)

*Know What's Real — up and running in under five minutes.*

---

## Install (2 minutes)

1. Download the correct DMG from the releases page:
   - Apple Silicon (M1/M2/M3/M4): `Jura-Trace_X.X.X_aarch64.dmg`
   - Intel Mac: `Jura-Trace_X.X.X_x64.dmg`
2. Open the DMG and drag **Jura Trace** into the **Applications** folder.
3. Go to **Applications**, right-click **Jura Trace**, and select **Open**.
4. In the dialog, click **Open** again. (First launch only — macOS expects a signed build.)

---

## Verify your first image (1 minute)

1. Click **VERIFY** in the navigation bar.
2. Drop any JPEG or PNG onto the drop zone.
3. Wait 10–15 seconds for Standard analysis to complete.
4. Read the **verdict** (Authentic / Inconclusive / Synthetic) and **trust score**.

---

## Protect your content (1 minute)

1. Click **PROTECT**.
2. Drop files onto the import zone.
3. Select an asset, then click **Sign with C2PA** — this creates a provenance record.
4. Select an asset, then click **Embed Watermark** — the mark is invisible and survives compression.

---

## That's it

| Feature | What you need |
|---|---|
| Image and document analysis | Nothing — works immediately |
| Video and audio analysis | `brew install ffmpeg` |
| AI-assisted descriptions | Ollama — download from ollama.com |

Full documentation: click **Help** inside the app.

For installation problems, see [`macos-unsigned.md`](./macos-unsigned.md).

---

*Jura Trace — Juralabs Community Interest Company (UK). AGPL-3.0-or-later. Local-first — no data leaves your machine.*
