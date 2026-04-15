# Jura Trace

**Know What's Real.**

In a world of synthetic media, verification matters. Jura Trace helps cultural institutions protect their digital assets from unauthorised AI extraction and helps communities verify the authenticity of media they encounter.

All processing happens locally on your machine. No data is uploaded to external servers.

## Status

Pre-development (v0.1.0-dev)

## What It Does

**PROTECT** — Safeguard digital assets
- Auto-catalogue: AI-generated descriptions, tags, and metadata for bulk archives
- C2PA signing: Embed provenance manifests proving origin and rights
- Fingerprinting: Perceptual hashes that survive crop, resize, and recompression
- Watermarking: Invisible watermarks for scrape detection

**VERIFY** — Check content authenticity
- Image forensics: Error Level Analysis, noise patterns, metadata anomalies
- Deepfake detection: Local AI model scoring for synthetic media
- Claim checking: "Is this claim credible?" with sourced answers
- Provenance reading: Verify C2PA provenance manifests on any file

## Prerequisites

- macOS 12+, Windows 10+, or Linux (Ubuntu 22.04+)
- [Ollama](https://ollama.com/) with `llava:7b` model (for auto-catalogue)
- [Rust](https://rustup.rs/) 1.88+ (for development)
- [Node.js](https://nodejs.org/) 20+ (for development)

## Quick Start (Development)

```bash
# Install dependencies
make install

# Start development
cd ui && npm run dev          # Terminal 1
cd src-tauri && cargo tauri dev  # Terminal 2
```

## Documentation

- [Project Specification](PROJECT_SPEC.md) — objectives, KPIs, delivery plan
- [Technical Architecture](docs/ARCHITECTURE.md) — system design
- [Brand Guidelines](docs/BRAND_GUIDELINES.md) — visual identity

## Licence

PolyForm Noncommercial 1.0.0 — Free for museums, archives, journalists, educators, charities, and community organisations. Commercial use requires licence from licensing@juralabs.org.

## Credits

Developed by [Jura Labs CIC](https://juralabs.org) — building ethical, local-first AI tools for social good.
