# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Jura Trace, **do not open a public issue.**

Email **licensing@juralabs.org** with:

- A description of the issue and the affected component (desktop app, Rust core, Python sidecar, REST API, build/release pipeline)
- Steps to reproduce, or a proof of concept if you have one
- The version you tested (Help → About, or the installer filename)
- How you would like to be credited, if at all

We will acknowledge your report within 7 days. We aim to ship a fix or mitigation within 30 days for medium-severity issues, sooner for high-severity ones. Coordinated disclosure is preferred: please give us the opportunity to release a fix before publishing details.

## Scope

In scope:

- The Jura Trace desktop application (Tauri shell, Rust core engine, SvelteKit frontend)
- The bundled Python ML sidecar (port 8200) and local REST API (port 8300)
- Release artefacts and the auto-update mechanism
- C2PA signing and verification behaviour that could mislead a user about content authenticity

Out of scope:

- Detection accuracy disagreements (a detector giving a verdict you disagree with is a calibration discussion, not a vulnerability; see `docs/methodology` and open a normal issue)
- Issues requiring physical access to an unlocked machine
- Vulnerabilities in third-party dependencies with no demonstrated impact on Jura Trace (though heads-up reports are welcome)
- The optional Ollama runtime, which is installed and operated separately

## Design Notes for Researchers

Jura Trace is local-first: there is no cloud backend, no telemetry, and both bundled services bind to 127.0.0.1 only. The threat model centres on malicious input files (crafted images and metadata), the local IPC/HTTP surfaces, and the integrity of the signing and update pipelines. Reports in those areas are especially valuable.

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Older releases | No, please update first |
