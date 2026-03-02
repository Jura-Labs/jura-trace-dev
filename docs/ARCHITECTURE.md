# Jura Archive — Technical Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Tauri v2 Desktop Shell (Rust)                     │
│              Cross-platform: macOS / Windows / Linux                │
│         Native window, system tray, file dialogs, auto-update       │
└────────────────────────────┬────────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────────┐
│                  SvelteKit Frontend (Port 5173)                      │
│                                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  ┌───────────┐ │
│  │  PROTECT     │  │  VERIFY      │  │  MONITOR   │  │  SETTINGS │ │
│  │  Dashboard   │  │  Workspace   │  │  Alerts    │  │  Profiles │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬─────┘  └─────┬─────┘ │
│         └──────────────────┴────────────────┴───────────────┘       │
└────────────────────────────┬────────────────────────────────────────┘
                             │ Tauri IPC (invoke)
┌────────────────────────────▼────────────────────────────────────────┐
│                     Rust Core Engine                                 │
│                                                                      │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐ │
│  │  C2PA Module    │  │  Hash Engine     │  │  Format Router     │ │
│  │  (c2pa-rs)      │  │  (perceptual     │  │  (MIME detect,     │ │
│  │  Sign, verify,  │  │   hashing)       │  │   pipeline route)  │ │
│  │  read manifests │  │                  │  │                    │ │
│  └─────────────────┘  └──────────────────┘  └────────────────────┘ │
│                                                                      │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐ │
│  │  Metadata       │  │  Watermark       │  │  Database          │ │
│  │  Extractor      │  │  Engine          │  │  (rusqlite)        │ │
│  │  (EXIF, XMP,    │  │  (invisible +    │  │  assets, prints,   │ │
│  │   IPTC, ICC)    │  │   visible)       │  │  verifications     │ │
│  └─────────────────┘  └──────────────────┘  └────────────────────┘ │
└────────────────────────────┬────────────────────────────────────────┘
                             │ HTTP (localhost:8200)
┌────────────────────────────▼────────────────────────────────────────┐
│              Python ML Sidecar — Phase 2+ (Port 8200)               │
│                                                                      │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐ │
│  │  Image          │  │  Deepfake        │  │  RAG Pipeline      │ │
│  │  Forensics      │  │  Detector        │  │  (claim checking)  │ │
│  │  (ELA, noise)   │  │  (DeepSafe)      │  │                    │ │
│  └─────────────────┘  └──────────────────┘  └────────────────────┘ │
└────────────────────────────┬────────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────────┐
│              Ollama (Port 11434) — Optional Enhancement               │
│         LLaVA (Tier 3 descriptions)  |  Qwen2.5 (text/claims)      │
└─────────────────────────────────────────────────────────────────────┘
```

## Rust Core Modules

| Module | Crate | Purpose |
|--------|-------|---------|
| `c2pa` | c2pa-rs | Sign, verify, and read C2PA Content Credentials |
| `hash` | image, sha2 | Perceptual hashing (pHash, aHash, dHash, wHash) and cryptographic hashing |
| `metadata` | kamadak-exif, xmp-toolkit | Extract and inspect EXIF, XMP, IPTC metadata (Tier 1 cataloguing) |
| `catalogue` | ort (ONNX Runtime) | CLIP-based subject tagging via museum vocabulary (Tier 2, optional download) |
| `watermark` | custom | Invisible frequency-domain watermarking |
| `format_router` | infer, mime_guess | Detect content type and route to correct pipeline |
| `db` | rusqlite | SQLite database for assets, fingerprints, verifications, audit log |

## Database Schema

### assets
Primary registry of all imported files.

| Column | Type | Description |
|--------|------|-------------|
| asset_id | TEXT PK | UUID v4 |
| file_path | TEXT | Original file path |
| file_name | TEXT | File name |
| content_type | TEXT | image/document/video/audio/3d |
| mime_type | TEXT | MIME type |
| file_size | INTEGER | Bytes |
| ai_description | TEXT | AI-generated description (Tier 3 Ollama, optional) |
| ai_tags | TEXT | JSON array of tags (Tier 2 CLIP or Tier 3 Ollama) |
| c2pa_signed | BOOLEAN | Whether C2PA manifest embedded |
| watermarked | BOOLEAN | Whether invisible watermark applied |
| collection_id | TEXT | Optional grouping |
| created_at | DATETIME | Import timestamp |

### fingerprints
Perceptual hashes for content tracking.

| Column | Type | Description |
|--------|------|-------------|
| fingerprint_id | TEXT PK | UUID v4 |
| asset_id | TEXT FK | References assets |
| hash_type | TEXT | phash/ahash/dhash/whash/audio_fp/mesh_fp |
| hash_value | TEXT | Hex-encoded hash |
| created_at | DATETIME | Creation timestamp |

### verifications
Results from the VERIFY pipeline.

| Column | Type | Description |
|--------|------|-------------|
| verification_id | TEXT PK | UUID v4 |
| source_type | TEXT | upload/url/clipboard |
| content_type | TEXT | Detected content type |
| ela_score | REAL | 0.0 (clean) to 1.0 (manipulated) |
| deepfake_score | REAL | 0.0 (authentic) to 1.0 (synthetic) |
| c2pa_valid | BOOLEAN | C2PA verification result |
| metadata_flags | TEXT | JSON array of anomalies |
| claim_verdict | TEXT | supported/disputed/unverified/mixed |
| overall_trust | REAL | Composite score 0.0-1.0 |
| created_at | DATETIME | Verification timestamp |

### audit_log
Immutable record of all actions for compliance.

| Column | Type | Description |
|--------|------|-------------|
| log_id | TEXT PK | UUID v4 |
| action | TEXT | sign/verify/watermark/export/fingerprint |
| target_type | TEXT | asset/verification |
| target_id | TEXT | ID of target |
| details | TEXT | JSON details |
| operator_id | TEXT | Operator identifier (default: local_user) |
| algorithm_metadata | TEXT | JSON algorithm parameters and versions |
| created_at | DATETIME | Action timestamp |

## Data Flow: PROTECT Pipeline

```
File Drop → Format Router → [Image|Document|Video|Audio|3D] Pipeline
                                      │
                            ┌─────────┼──────────┐
                            ▼         ▼          ▼
                        Metadata   Catalogue   Hash
                        Extract    (tiered)    Compute
                        (Tier 1)       │          │
                            │    ┌─────┴─────┐    │
                            │    ▼           ▼    │
                            │  CLIP tags   Ollama │
                            │  (Tier 2)  describe │
                            │  optional  (Tier 3) │
                            │    │       optional  │
                            ▼    ▼           ▼    ▼
                        C2PA Sign  Watermark   Store
                            │         │       Fingerprint
                            └─────────┼──────────┘
                                      ▼
                              SQLite Registry
                                      │
                                      ▼
                              Export (files + CSV)
```

## Data Flow: VERIFY Pipeline

```
Input (file/URL/text) → Intent Classifier
                              │
                ┌─────────────┼──────────────┐
                ▼             ▼              ▼
           Media Forensic  Text Claim    Source Trace
           (ELA, deepfake, (RAG search,  (URL fetch,
            EXIF, C2PA)    LLM assess)   archive compare)
                │             │              │
                └─────────────┼──────────────┘
                              ▼
                    Trust Report Generator
                    (composite score, evidence, sources)
```

## Security Model

- **No network calls** except to localhost (Ollama, sidecar)
- **No telemetry** or analytics
- **No accounts** or authentication
- **No cloud storage** — all data in local SQLite + filesystem
- **CSP enforced** via Tauri configuration
- **File access** scoped via Tauri capability permissions
