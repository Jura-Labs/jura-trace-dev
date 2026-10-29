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

## Verifying a download

Releases are published at <https://github.com/Jura-Labs/jura-trace/releases>. There are two checks, and the first works for every file.

**1. The checksum.** Download `SHA256SUMS.txt` from the same release into the folder that holds your download, then run:

```
shasum -a 256 -c --ignore-missing SHA256SUMS.txt      # macOS
sha256sum -c --ignore-missing SHA256SUMS.txt          # Linux
certutil -hashfile <file> SHA256                      # Windows, then compare with the file's line
```

This proves the file is the one we published. `SHA256SUMS.txt` lists every installer, the updater archives, the Linux server tarball and the SBOM files.

**2. Where it was built.** Files built by our release workflow on GitHub Actions carry a signed build provenance attestation, which records the workflow, the commit and the run that built them. With the [GitHub CLI](https://cli.github.com/) installed and signed in (`gh auth login`), run:

```
gh attestation verify <file> -R Jura-Labs/jura-trace-dev
```

The attestations are stored on `Jura-Labs/jura-trace-dev`, the source repository, not on `Jura-Labs/jura-trace` where you download the files, so `-R Jura-Labs/jura-trace-dev` is needed. A passing check names the workflow (`.github/workflows/release.yml`) and the commit. It checks the file's contents, not its name.

What is covered, from v1.3.0:

| Files | Attested |
|---|---|
| Windows `.msi` and `-setup.exe` | Yes |
| Linux `.AppImage`, `.deb` and the server `.tar.gz` | Yes |
| SBOM files (`*-sbom-*.cdx.json`) | Yes |
| macOS `.dmg` and `Jura.Trace.app.tar.gz` | Only when the release workflow built them |

Some macOS releases are built on our own Mac and attached to the release by hand. Those files have no attestation, and `gh attestation verify` reports that it found none. For them, rely on the checksum and on macOS's own check of the Apple Developer ID signature and notarisation (`spctl --assess --type install <file>.dmg`).

**Software bill of materials.** Each release from v1.3.0 carries three CycloneDX JSON files: `JuraTrace-<version>-sbom-rust.cdx.json` (the Rust app, all platforms in one file), `JuraTrace-<version>-sbom-sidecar.cdx.json` (the Python sidecar) and `JuraTrace-<version>-sbom-ui.cdx.json` (the interface, build tools included). They are made from the lock files at the release commit, which are what the build installs from.

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Older releases | No, please update first |
