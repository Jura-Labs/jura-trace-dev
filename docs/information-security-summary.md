---
title: "Jura Trace — Information Security Summary"
description: "Security, data protection, and compliance summary for institutional procurement and Data Protection Officers."
last-updated: 25 March 2026
product-version: v0.9.0-rc.1
document-version: "1.0"
organisation: "Juralabs Community Interest Company (UK)"
contact: "security@juralabs.org"
---

# Jura Trace — Information Security Summary

**Document version**: 1.0
**Last updated**: 25 March 2026
**Product version**: v0.9.0-rc.1
**Organisation**: Juralabs Community Interest Company (UK)
**Contact**: security@juralabs.org

---

## Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#1-architecture-overview)
3. [Data Processing — What Happens to User Content](#2-data-processing--what-happens-to-user-content)
4. [Data at Rest](#3-data-at-rest)
5. [Data in Transit](#4-data-in-transit)
6. [Authentication and Access Control](#5-authentication-and-access-control)
7. [Security Controls](#6-security-controls)
8. [Security Audit Status](#7-security-audit-status)
9. [Update Mechanism](#8-update-mechanism)
10. [Third-Party Dependencies](#9-third-party-dependencies)
11. [Compliance Considerations](#10-compliance-considerations)
12. [Logging and Monitoring](#11-logging-and-monitoring)
13. [Incident Response](#12-incident-response)
14. [Licence and Commercial Terms](#13-licence-and-commercial-terms)

---

## Executive Summary

Jura Trace is a local-first desktop application for content verification and protection. It runs entirely on the user's device. All processing — including forensic image analysis, C2PA signing, perceptual fingerprinting, invisible watermarking, and AI detection — occurs on-device. No content, metadata, or analysis result is transmitted to external servers as part of normal operation. The application stores data in a local SQLite database under the user's application data directory. Optional features that involve external communication (a version-update check and, at Professional tier and above, an opt-in reverse image search integration) are explicit user actions with no data shared with Juralabs. The architecture is designed to give organisations complete sovereignty over their content and analysis data.

---

## 1. Architecture Overview

Jura Trace is a native desktop application built on the Tauri v2 framework (Rust backend, SvelteKit frontend). It runs on macOS 13+, Windows 10+, and Ubuntu 22.04+.

```
[User Device]
    │
    ├── Tauri App (Rust + SvelteKit)
    │       │
    │       ├── Local SQLite Database
    │       │
    │       └── Python ML Sidecar (localhost:8200)
    │               │
    │               └── Ollama LLM Runtime (localhost:11434) — optional
    │
    └── [No external network traffic for core functionality]
```

### Components

| Component | Technology | Purpose |
|---|---|---|
| Desktop shell | Tauri v2 (Rust) | Application window, IPC bridge, file system access |
| Frontend | SvelteKit 5, TailwindCSS | User interface (PROTECT, VERIFY, MONITOR, SETTINGS) |
| Core engine | Rust (c2pa-rs, rusqlite, image_hasher) | C2PA signing/verification, perceptual hashing, EXIF analysis, metadata extraction |
| ML sidecar | Python 3.13, FastAPI | 21 forensic detectors — image forensics, deepfake detection, watermark operations |
| Database | SQLite | Local storage of asset metadata, analysis results, audit log |
| LLM runtime | Ollama (optional) | Local language model for content descriptions and claim verification |

### Network exposure

The Python ML sidecar binds to `127.0.0.1:8200` — the loopback interface only. It is not accessible from other machines on the network. The Ollama runtime, if installed, binds to `127.0.0.1:11434` on the same basis. Neither service is exposed as a network service reachable from outside the device.

---

## 2. Data Processing — What Happens to User Content

### Input files

When a user submits an image, video, audio file, or PDF to Jura Trace:

1. The file is read from disk by the Tauri Rust process.
2. Metadata (EXIF, XMP, IPTC) is extracted in-memory.
3. Forensic analysis is performed locally by the Python sidecar, which receives the file contents over the loopback interface.
4. A perceptual fingerprint (aHash, dHash, pHash) is computed and stored in the local database.
5. Analysis results (trust score, detector outputs, verdict) are stored in the local database.
6. The original file is never copied, moved, or transmitted.

**The original file content is not stored in the database.** Only metadata, hashes, and analysis results are persisted.

### Forensic detectors

All 21 forensic detectors run locally via the Python sidecar. No detector sends data to an external service. The full detector list includes: Error Level Analysis (ELA), noise analysis, copy-move detection, deepfake detection (image and video), NPR (neighbouring pixel relationships), chromatic aberration analysis, JPEG ghost detection, segmented ELA, shadow consistency, colour temperature analysis, splice boundary detection, CLIP zero-shot AI/authentic classification, C2PA manifest verification, perceptual fingerprinting, EXIF anomaly detection, audio metadata extraction, video metadata extraction, video frame extraction, watermark embed/extract, audio transcription, and RAG claim verification.

### C2PA signing

C2PA (Coalition for Content Provenance and Authenticity) signing operations are performed entirely locally using the `c2pa-rs` library. Private keys do not leave the device. C2PA manifests are embedded directly into the signed file.

### Watermark embedding

Frequency-domain watermarking (DWT-DCT-SVD) is computed entirely on-device. The watermark payload (typically an institution identifier as a 128-bit UUID) is never transmitted externally.

### AI inference (optional)

When Ollama is installed and running, Jura Trace may call it on `localhost:11434` for content description (LLaVA model) and claim verification (Qwen2.5 model). Ollama runs entirely on-device. No data is sent to any cloud inference service.

---

## 3. Data at Rest

### Database location

Jura Trace stores all asset metadata, fingerprints, verification results, and audit logs in a single SQLite database file. Default locations:

| Platform | Default path |
|---|---|
| macOS | `~/Library/Application Support/Jura Trace/jura_archive.db` |
| Windows | `%APPDATA%\Jura Trace\jura_archive.db` |
| Linux | `~/.local/share/jura-trace/jura_archive.db` |

The database location can be overridden via:
- Environment variable: `JURA_DB_PATH` (highest priority)
- Configuration file: `db_path` key in `<app-data-dir>/config.json`
- Settings UI: Change Location button in SETTINGS

This allows the database to be placed on a network share, managed drive, or encrypted volume under institutional control.

### What is stored in the database

| Data type | Stored |
|---|---|
| File names, paths, sizes, content types, import dates | Yes |
| EXIF metadata summaries and anomaly findings | Yes |
| Perceptual fingerprint hashes (aHash, dHash, pHash) | Yes |
| C2PA manifest digests | Yes |
| Verification trust scores and verdict history | Yes |
| Watermark embed/extract records | Yes |
| Audio/video metadata summaries | Yes |
| Audit log of all user actions (SHA-256 hash chain) | Yes |
| Original file contents or pixel data | **No** |
| Forensic heatmap images | **No** (generated on demand, not persisted) |
| Passwords, API keys, or credentials | **No** |

### Database encryption

The SQLite database is not encrypted at the application level. It relies on OS-level disk encryption for protection at rest.

**Recommended approach**: Enable full-disk encryption on all devices running Jura Trace — FileVault (macOS), BitLocker (Windows), or LUKS (Linux). This protects the database and all other local data transparently.

**Planned enhancement**: Optional AES-256 database encryption via SQLCipher is planned for v1.1. This will add a passphrase requirement on first run and applies to deployments where OS-level encryption is not available. See the [v1.1 backlog](/docs/sprint-plans/sprint-15-to-v1.0-plan.md) for status.

### User account separation

On shared workstations, each user should operate under a separate OS account. The database is stored in the user's application data directory, which is not accessible to other standard OS accounts.

---

## 4. Data in Transit

### Loopback-only communication

All communication between the Tauri application and the Python sidecar occurs over `127.0.0.1:8200` (loopback interface). This traffic never leaves the device. The same applies to Ollama communication on `127.0.0.1:11434`.

### No external calls for core functionality

The following operations involve no external network traffic whatsoever:

- Content verification (all forensic detectors)
- C2PA signing and verification
- Watermark embedding and extraction
- Perceptual fingerprinting
- EXIF metadata extraction and anomaly detection
- Audio and video metadata extraction
- Speech transcription
- RAG claim verification (when using local Ollama)
- Database operations
- Report generation (PDF and ZIP)

### Optional external calls

Two features involve external network communication. Both require explicit user action:

**Auto-update check**: The application periodically checks `github.com/juralabs/jura-archive/releases` for new version information. This is a single HTTPS GET request. No user data, no file content, no analysis results, and no telemetry are transmitted. The request contains only the current application version number and platform identifier. This check can be disabled by organisations that manage application updates centrally.

**Reverse image search** (Professional tier and above, future): When enabled, Jura Trace can perform reverse image search via TinEye or Google Vision APIs. This feature requires the user to supply their own API credentials. Jura Trace sends a thumbnail-sized image crop (not the full file) to the configured API endpoint. Per-analysis consent is required. This feature is not enabled by default and is not available on the Community tier.

### No telemetry

Jura Trace does not collect usage analytics, crash reports, feature usage statistics, or any other telemetry. It does not phone home.

---

## 5. Authentication and Access Control

### Sidecar API key

The Python ML sidecar requires an API key on every request (`X-Jura-API-Key` header). This prevents other processes running on the same machine from calling the sidecar without authorisation.

In standard single-user installations, the key is generated automatically at startup (256-bit UUID) and passed to the sidecar subprocess via environment variable. No configuration is required.

For institutional deployments where the sidecar is started independently of the desktop application, the key can be set explicitly:

```bash
export JURA_SIDECAR_KEY="your-institution-key-here"
uvicorn main:app --host 127.0.0.1 --port 8200
```

Requests without a valid key are rejected with HTTP 401.

### No user accounts

Jura Trace has no user account system, no login screen, no cloud authentication, and no identity management. Access to the application is controlled entirely by OS-level user account permissions.

### File system access

File system access is restricted to directories selected by the user via the native OS file picker. The application does not have access to the entire file system — only to paths explicitly chosen by the user and to its own application data directory. This is enforced at the Tauri capability level, not solely in application logic.

### Path validation

All file paths supplied by the frontend are validated before processing: null bytes are rejected, paths are canonicalised via the OS resolver, and any path that cannot be resolved or is not accessible to the user account returns a generic error. Sensitive path information is not echoed in error messages.

### Database access

The SQLite database is accessed directly by the application process. It is not exposed over any network interface.

---

## 6. Security Controls

### Content Security Policy

The application's webview enforces the following CSP:

```
default-src 'self';
script-src 'self';
style-src 'self' 'unsafe-inline';
img-src 'self' blob:;
connect-src ipc: http://ipc.localhost http://127.0.0.1:8200 http://127.0.0.1:11434;
object-src 'none';
base-uri 'self';
frame-ancestors 'none';
```

Key points:
- `script-src 'self'` — inline scripts and external script sources are blocked
- `connect-src` — network access is pinned to localhost addresses only; no external HTTP connections are possible from the frontend
- `object-src 'none'` — browser plugins (Flash, Java applets) are blocked
- `frame-ancestors 'none'` — the webview cannot be embedded in an external frame

### Base64 image rendering

All forensic heatmap and thumbnail images are rendered using `blob:` URLs generated from base64 data. This avoids `data:` URIs in the CSP, which would broaden the `img-src` directive unnecessarily.

### Shell capability scoping

The application does not grant the frontend the ability to execute arbitrary system commands. The Python sidecar is launched from Rust setup code; no frontend shell execution or spawn capability is granted.

### SSRF prevention

The URL verification feature (which checks remote URLs for content authenticity) blocks:
- Loopback addresses (`127.x.x.x`, `::1`)
- Private network ranges (RFC 1918: `10.x.x.x`, `172.16–31.x.x`, `192.168.x.x`)
- Link-local addresses (`169.254.x.x`)
- Non-HTTP/HTTPS schemes

This prevents the application from being used as a proxy to reach internal network resources.

### Input validation

- All user-supplied file paths: null-byte check, canonicalisation via OS resolver
- Database path changes: extension check (`.db`, `.sqlite`, `.sqlite3`), symlink rejection, null-byte check
- Sidecar mode parameters: allowlist validation against permitted values
- Transcription file uploads: 500 MB upper size limit enforced before processing

### Audit trail integrity

Every protect and verify action is recorded in the audit log with a SHA-256 hash chain. Each entry includes the SHA-256 hash of the previous entry, making any retrospective modification detectable. The integrity of the audit chain can be verified programmatically via the `verify_audit_integrity` command.

### Dependency security

- Rust dependencies: audited via `cargo-audit` in CI
- Python dependencies: pinned to exact versions in `requirements.lock`, audited via `pip-audit` in CI
- Dependabot is configured for automated dependency update pull requests on both the Rust and Python dependency trees

---

## 7. Security Audit Status

A full OWASP-aligned security penetration test was conducted on 25 March 2026 (Sprint 19), covering the Tauri IPC boundary, sidecar API, Content Security Policy, capability configuration, data at rest, and dependency surfaces. All findings were remediated within the same sprint.

### Audit summary

| Severity | Findings | Open | Fixed |
|---|---|---|---|
| Critical | 0 | 0 | — |
| High | 4 | 0 | 4 |
| Medium | 5 | 0 | 5 |
| Low | 6 | 0 | 6 |
| Informational | 4 | 0 | 4 |

**All findings are resolved. There are no open security issues.**

### High findings (all resolved)

| ID | Description | Resolution |
|---|---|---|
| HIGH-1 | `read_manifest` and `verify_c2pa` commands accepted arbitrary file paths without canonicalisation | Path validation applied: null-byte check + OS canonicalisation on all affected commands |
| HIGH-2 | `extract_watermark_from_path` accepted arbitrary paths without validation | Same path validation applied; error messages no longer echo attacker-controlled paths |
| HIGH-3 | `analyse_video_deepfake` command did not canonicalise its path; error message echoed the raw path | Canonicalisation applied; mode parameter validated against allowlist |
| HIGH-4 | `shell:allow-execute` and `shell:allow-spawn` were granted without scope restrictions, enabling arbitrary command execution from the frontend | Both permissions removed from capability configuration; sidecar launch handled in Rust only |

### Previous audit (Sprint 14, 21 March 2026)

A prior audit identified 3 critical and 6 high findings, all of which were remediated in Sprint 15:
- Critical: SSRF in URL verification, overly broad file system capability, CSP `connect-src` too permissive
- High: sidecar unauthenticated, audit log lacking integrity protection, base64 data URIs in CSP, URL query-string logging, temp file cleanup gaps, Python dependencies unpinned

The Sprint 19 audit confirmed that all Sprint 14 findings remain remediated with no regression.

The full Sprint 19 audit report is available at `/docs/security-pen-test-s19.md`.

---

## 8. Update Mechanism

### How updates work

Jura Trace includes a Tauri auto-updater that checks the GitHub Releases page for new versions. The check is a single HTTPS GET request to `github.com/juralabs/jura-archive/releases`. No user data is transmitted.

When an update is available, the user is notified and can choose to install it. The update package is downloaded, its signature verified against Juralabs' public key, and then installed. The application does not install updates silently without user consent.

### Enterprise update management

Enterprise tier customers receive the following update control options:

- **Automatic**: install updates in the background without user prompt
- **Notify-only**: notify the user that an update is available, but require manual installation
- **Disabled**: disable the auto-updater entirely, for air-gapped deployments or environments where software updates are managed centrally via MDM

The update endpoint can be configured in the central TOML configuration file distributed as part of the Enterprise silent installer package.

---

## 9. Third-Party Dependencies

The following table summarises the major third-party components included in Jura Trace and their licence terms.

| Component | Version | Licence | Purpose |
|---|---|---|---|
| Tauri v2 | 2.x | MIT / Apache-2.0 | Desktop application framework |
| SvelteKit | 5.x | MIT | Frontend framework |
| c2pa-rs | 0.x | MIT / Apache-2.0 | C2PA content provenance signing and verification |
| rusqlite | 0.x | MIT | SQLite database access |
| image_hasher | 4.x | MIT | Perceptual fingerprinting (aHash, dHash, pHash) |
| blind_watermark (Rust) | 0.1.x | MIT | Frequency-domain watermarking |
| FastAPI | 0.x | MIT | Python sidecar HTTP framework |
| NumPy | 1.x / 2.x | BSD-3-Clause | Numerical computing for forensic detectors |
| OpenCV (Python) | 4.x | Apache-2.0 | Image processing |
| Pillow | 10.x | HPND | Image format handling |
| scikit-learn | 1.x | BSD-3-Clause | GBM deepfake classifier |
| imwatermark | 0.x | MIT | Python watermark embed/extract |
| faster-whisper | 1.x | MIT | On-device speech transcription |
| open_clip | 2.x | MIT | CLIP zero-shot AI/authentic classification (optional) |
| FFmpeg / ffprobe | 6.x | LGPL-2.1+ | Video and audio metadata extraction (optional) |
| Ollama | 0.x | MIT | Local LLM runtime (optional) |

All dependencies are audited in CI. The full dependency manifest is available in `src-tauri/Cargo.toml` (Rust) and `sidecar/requirements.lock` (Python).

---

## 10. Compliance Considerations

### UK GDPR and Data Protection Act 2018

Jura Trace's local-first architecture substantially reduces the data protection risk profile for institutional deployments:

- No personal data is transmitted to Juralabs or any third party as part of core functionality.
- No data is processed by cloud services operated by Juralabs.
- The application does not create or maintain user accounts, and Juralabs holds no record of who is using the software or what content is being processed.

**Organisations must assess whether the images, videos, or documents they process through Jura Trace contain personal data.** Where content includes images of identifiable individuals (for example, press photographs, archive footage, or identity documents), the processing organisation is the data controller for that processing activity. Jura Trace acts as a local processing tool; Juralabs is not a data processor for this activity.

A DPIA (Data Protection Impact Assessment) template is included in the Enterprise tier compliance documentation pack. This template addresses the local-first architecture, identifies residual risks (principally the unencrypted SQLite database), and provides recommended mitigations.

### EU AI Act

Jura Trace detects AI-generated content but does not itself generate AI content. The application is therefore not subject to the provider obligations under the EU AI Act applicable to general-purpose AI models or high-risk AI systems.

The optional Ollama integration (LLaVA and Qwen2.5 models) runs locally. Juralabs does not operate these models as a service; the user operates them on their own device. Article 50 transparency obligations (disclosure that content is AI-generated) apply to the organisation using these features to generate content descriptions, not to Jura Trace as a tool.

Organisations using Jura Trace verification reports in automated decision-making processes should assess their obligations under Article 14 (human oversight) of the EU AI Act. The application explicitly supports human oversight through its visual inspection checklist and signal agreement dashboard. An EU AI Act compliance report template is available from Professional tier upward.

### PolyForm Noncommercial 1.0.0

The Community tier of Jura Trace is licenced under PolyForm Noncommercial 1.0.0. This licence permits use for non-commercial purposes at no cost. **Commercial use requires a paid commercial licence.**

Organisations that use Jura Trace in a commercial context — including insurance claims processing, legal proceedings, commercial journalism, corporate communications, and any other revenue-generating activity — must hold a Professional, Team, or Enterprise commercial licence. Using the Community (free) tier for commercial purposes is a breach of the licence terms.

### ISO 27001 considerations

Jura Trace does not currently hold ISO 27001 certification. However, the following controls are in place that are relevant to an ISO 27001 assessment:

- A.8.3 (Information access restriction): File system access restricted to user-selected paths via OS capability scoping
- A.8.24 (Use of cryptography): C2PA signing uses standard cryptographic operations via c2pa-rs; audit log integrity protected by SHA-256 hash chain
- A.8.25 (Secure development lifecycle): OWASP-aligned security audit conducted pre-release; dependency scanning in CI
- A.12.4 (Logging and monitoring): Tamper-evident audit log of all protect and verify actions
- A.16.1 (Management of information security incidents): Vulnerability disclosure process and response timelines defined (see [Incident Response](#12-incident-response))

---

## 11. Logging and Monitoring

### Application logs

In development mode, Jura Trace writes diagnostic logs to stdout. In production builds, logs are written to the system log (macOS: Console.app / unified logging; Windows: Event Viewer; Linux: systemd journal). Logs do not contain file content, personal data, or analysis results.

URL query strings are redacted from logs before writing, preventing sensitive query parameters from appearing in log output.

### Audit trail

Every protect action (C2PA signing, watermark embedding, fingerprint registration) and every verify action (forensic analysis, trust score generation) is recorded in the local audit log. Audit log entries include:

- Timestamp (UTC)
- Action type
- Asset identifier (hash)
- Result summary
- SHA-256 hash of the preceding entry (hash chain)

The audit log is stored in the local SQLite database. It is not transmitted externally. The integrity of the hash chain can be verified via the `verify_audit_integrity` command, accessible from the Settings page.

### Audit log retention

| Tier | Retention period |
|---|---|
| Community | 12 months |
| Professional, Team, Enterprise | 24 months |

### No telemetry

The application collects no usage telemetry, crash reports, feature analytics, or any other data that would be transmitted to Juralabs or any third party.

---

## 12. Incident Response

### Vulnerability disclosure

Juralabs operates a responsible disclosure process. To report a security vulnerability:

- **Email**: security@juralabs.org (for sensitive disclosures)
- **GitHub Issues**: github.com/juralabs/jura-archive/issues (for lower-severity issues that can be disclosed publicly)

We ask that reporters allow a reasonable period for investigation and remediation before public disclosure.

### Response timelines

| Severity | Response target | Fix target |
|---|---|---|
| Critical | 24 hours | 48 hours |
| High | 48 hours | 7 days |
| Medium | 5 business days | Next planned sprint |
| Low | 10 business days | Next planned sprint |

### Breach risk assessment

The local-first architecture substantially limits breach risk:

- **No cloud data**: there is no Juralabs-operated database of user content or analysis results to breach.
- **No user accounts**: there are no credentials to compromise at the Juralabs infrastructure level.
- **No external transmission**: content does not leave the device during normal operation.

The principal residual risk is unauthorised access to the local SQLite database on the user's device. Mitigation is OS-level disk encryption and OS-level user account separation, as described in [Section 3](#3-data-at-rest).

---

## 13. Licence and Commercial Terms

### Licence model

Jura Trace uses a split-licence model: the same application software is available under two licence frameworks.

**PolyForm Noncommercial 1.0.0** (Community tier, free): Grants full use of the software for non-commercial purposes. No payment required. This is not a trial — it is a permanent free licence for qualifying use cases including journalism, education, research, NGO work, and cultural heritage.

**Commercial licence** (Professional, Team, Enterprise tiers): Grants permission to use Jura Trace for commercial purposes and unlocks tier-specific features. The commercial licence is solicitor-drafted and references the tier terms directly. Commercial licences are sold via Juralabs.

### Tier summary for procurement

| Tier | Price | Seats | Primary use case |
|---|---|---|---|
| Community | Free | 1 | Non-commercial (journalists, NGOs, cultural institutions, researchers) |
| Professional | £199/year | 1 | Individual commercial practitioners |
| Team | £79/seat/month (3–20 seats) | 3–20 | Small commercial teams |
| Enterprise | From £6,000/year | Unlimited | Large organisations, institutional procurement |
| Grant-Subsidised | £0 (by application) | Enterprise-equivalent | Public interest organisations with funding barriers |

### Enterprise procurement

Enterprise licences include:

- Silent installer (MSI for Windows, PKG for macOS) with ADMX/MDM templates for Group Policy or Jamf/Intune deployment
- Central TOML configuration file for organisation-wide defaults
- Compliance documentation pack (DPIA template, this Information Security Summary, PolyForm licence summary)
- Auto-update management (automatic / notify-only / disabled for air-gapped deployments)
- SLA-backed support (4-hour response for critical issues, named account contact)
- Source code access for internal security audit purposes (contractual right under Enterprise licence)

To initiate an Enterprise procurement discussion: security@juralabs.org or contact Juralabs Community Interest Company directly.

---

*Document version 1.0. Last updated 25 March 2026.*
*Product version v0.9.0-rc.1.*
*Juralabs Community Interest Company (UK). https://juralabs.org*
