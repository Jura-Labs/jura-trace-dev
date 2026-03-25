# Jura Trace — Quick Start (Linux)

*Know What's Real — up and running in under five minutes.*

---

## Install (1 minute)

### AppImage (recommended — works on any modern distribution)

1. Download `Jura-Trace_X.X.X_amd64.AppImage` from the releases page.
2. Make it executable:

```bash
chmod +x Jura-Trace_X.X.X_amd64.AppImage
```

3. Run it:

```bash
./Jura-Trace_X.X.X_amd64.AppImage
```

### Debian / Ubuntu (.deb package)

```bash
sudo dpkg -i jura-trace_X.X.X_amd64.deb
sudo apt install -f
```

Requires Ubuntu 22.04 or later (or equivalent). Ubuntu 20.04 is not supported.

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
| Video and audio analysis | `sudo apt install ffmpeg` |
| AI-assisted descriptions | Ollama — download from ollama.com |

Full documentation: click **Help** inside the app.

For dependency details and troubleshooting, see [`linux-requirements.md`](./linux-requirements.md).

---

*Jura Trace — Juralabs Community Interest Company (UK). PolyForm Noncommercial 1.0.0. Local-first — no data leaves your machine.*
