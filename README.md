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

- [Architecture](docs/ARCHITECTURE.md) — system design, data flow, four-layer stack
- [Technical Architecture](docs/ARCHITECTURE.md) — system design
- [Brand Guidelines](docs/BRAND_GUIDELINES.md) — visual identity

## Licence

**AGPL-3.0-or-later** — free and open-source for anyone, including commercial use that complies with the AGPL's network-use clause and copyleft terms. See [`LICENSE`](LICENSE) for the full text.

A commercial licence is available for use cases that cannot operate under the AGPL — for example, integration into closed-source products, internal modified deployments, or cases requiring contractual indemnification. See [`COMMERCIAL.md`](COMMERCIAL.md) for the process, or email `licensing@juralabs.org`.

**AI training restriction:** use of this source code as training data for ML/AI systems requires separate written permission. See [`TRAINING.md`](TRAINING.md).

Copyright © 2025-2026 Paul Griffiths, published by Jura Labs CIC under perpetual royalty-free licence.

## On the use of Generative AI in this codebase

Jura Trace is developed by a sole maintainer (Paul Griffiths) using Anthropic Claude as a coding and documentation assistant, structured around clear lines of human responsibility.

**Architecture and design decisions are human-led.** Architectural choices (the four-layer Tauri / Rust / Python sidecar / SvelteKit structure, the Sovereign vs Conformant signing-mode split, the trust-score weighting, the local-first guarantee, the detector lineup, the licence and CIC framing) are made by the maintainer. AI may be consulted on trade-offs but does not decide.

**Tests are managed by the developer.** The Rust library tests, Rust API integration tests, Python sidecar tests, Playwright end-to-end tests, and Vitest component tests are authored and reviewed by the maintainer. Recursive and regression-test discipline is run by the maintainer. AI is not used for test sign-off, coverage decisions, or detector threshold setting.

**Sources are human-verified.** Calibration figures, per-generator recall tables, vendor specification references, trust-list certificate fingerprints, conformance-programme record identifiers, and academic citations are checked against primary sources by the maintainer. AI search and AI summarisation are starting points. They are not sufficient evidence on their own.

**Code generation is AI-assisted, human-reviewed.** Boilerplate, error-handling patterns, accessibility fixes, and refactoring are sometimes drafted with AI assistance and then reviewed, edited, and integrated by the maintainer. Generated code is treated as a draft. Nothing reaches `main` without human review.

**Documentation is AI-assisted, human-edited.** README sections, in-source comments, user-facing guides, and changelog entries frequently begin as AI drafts and are edited for accuracy by the maintainer.

**Security-sensitive code** (signing, key handling, network boundaries, file-system access, IPC permissions, certificate handling, trust evaluation) receives explicit human review regardless of how it was drafted.

**Forensic verdicts** (the runtime detection output users see) are produced by the deployed detectors and human-set thresholds. No large language model is in the verdict pipeline.

This declaration follows the [NLnet Foundation's policy on the use of Generative AI for funded projects](https://nlnet.nl/genai/) (effective 8 December 2025). The contributor-facing version of this policy, including disclosure requirements for code generation, is in [`GENAI_USE_POLICY.md`](GENAI_USE_POLICY.md).

## C2PA Validator-Conformant

Jura Trace is listed on the public [C2PA Conforming Products List](https://spec.c2pa.org/conformance-explorer/) as a Validator-Conformant implementation since 6 May 2026 (publicly searchable since 31 May 2026). Record identifier: `019d8d83-ed1c-787c-920c-8fad67b55cbe`. Spec version 2.2. Validates Content Credentials on JPEG, PNG, TIFF, and WebP.

Jura Labs CIC is a Content Authenticity Initiative member organisation.

## Credits

Developed by [Jura Labs CIC](https://juralabs.org) — building ethical, local-first AI tools for social good.
