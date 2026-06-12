# Jura Trace — Technical Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Tauri v2 Desktop Shell (Rust)                     │
│              Cross-platform: macOS / Windows / Linux                │
│         Native window, system tray, file dialogs, auto-update       │
└────────────────────────────┬────────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────────┐
│                  SvelteKit Frontend (Port 1420)                      │
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
                             │ HTTP (127.0.0.1, ephemeral port)
┌────────────────────────────▼────────────────────────────────────────┐
│         Python ML Sidecar — bound to 127.0.0.1 loopback only        │
│         Port assigned by the OS at each launch (Option C, May 2026) │
│                                                                      │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────────────┐ │
│  │  Image          │  │  Deepfake        │  │  Knowledge         │ │
│  │  Forensics      │  │  Detector        │  │  Retrieval         │ │
│  │  (ELA, noise,   │  │  (GBM v4 +       │  │  (claim context,   │ │
│  │   copy-move…)   │  │   UnivFD probe)  │  │   no verdicts)     │ │
│  └─────────────────┘  └──────────────────┘  └────────────────────┘ │
└────────────────────────────┬────────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────────┐
│              Ollama (Port 11434) — Optional Enhancement               │
│         LLaVA (Tier 3 descriptions)  |  Qwen2.5 (text/claims)      │
└─────────────────────────────────────────────────────────────────────┘
```

## Rust Core Modules

All modules live in `src-tauri/src/`. `lib.rs` is the entry point: it registers Tauri commands and bootstraps the application. The verify pipeline, trust-score computation, shared result types, and application state each live in their own module.

| Module | Crate | Purpose |
|--------|-------|---------|
| `config` | — | `AppConfig`, config file read/write, `resolve_db_path`; persists user preferences (database path, licence tier, power-saver mode) |
| `startup` | — | Logging initialisation, sidecar spawn and readiness probing, ephemeral-port selection, MEI temporary-directory cleanup |
| `state` | — | `AppState` shared across Tauri commands; `LicenceTier` (Community / Professional / Enterprise); `SidecarStartupStatus` probe lifecycle |
| `verify/types` | — | Shared verification result types: `VerificationResult`, `ThumbnailCheck`, `Provenance`, `ModelHashes`, `MethodologyRecord`, `InputQualityAssessment` |
| `verify/trust` | — | `compute_trust` and `document_trust` — the AGPL reproducibility anchor cited in methodology docs and PDF reports; all trust-score constants |
| `verify/input_quality` | — | Pre-pipeline quality assessment: JPEG quality estimation, resolution categorisation, screenshot detection, degraded-detector list |
| `verify/pipeline` | — | End-to-end verify orchestration: `verify_content_inner`, `verify_url_inner`, parallel sidecar groups, heatmap application |
| `c2pa` | c2pa-rs | Sign, verify, and read C2PA provenance manifests (Sovereign and Conformant modes); all operations local |
| `fingerprint` | image_hasher | Perceptual hashing (aHash, dHash, pHash) with Hamming distance similarity |
| `metadata` | kamadak-exif | Extract EXIF, XMP (AI-provenance signals), ICC profile descriptions, and JPEG quantisation tables; camera-vendor authenticity confidence |
| `exif_anomaly` | — | Detect EXIF anomalies across eight check categories (software, timestamps, GPS, dimensions, injection heuristics, XMP edit-history) and compute a metadata trust score |
| `filename_analysis` | — | Filename pattern heuristics for provenance signalling (camera naming conventions vs AI/screenshot patterns); additive context, not part of the trust score |
| `format_router` | infer, mime_guess | Detect content type (magic bytes first, extension fallback) and route to the correct pipeline; per-detector MIME gates (e.g. ELA is JPEG-only) |
| `watermark` | blind_watermark | Invisible frequency-domain watermarking (DWT-DCT-SVD); feature-flagged off in v1.0 |
| `pdf_provenance` | lopdf | PDF internal provenance: Info dictionary, incremental save history, digital signatures, redaction annotations, PDF/A compliance |
| `heatmap` | base64 | Decode sidecar heatmaps to per-session PNG files in the app cache so the frontend loads them lazily instead of inflating the IPC payload |
| `sun_position` | — | NOAA solar position calculator (azimuth and elevation) for shadow geometry verification in the investigation tools |
| `sidecar` | reqwest | HTTP client for the Python ML sidecar, with graceful degradation when the sidecar is offline |
| `db` | rusqlite | SQLite database (schema v7, WAL mode): assets, fingerprints, verifications, hash-chained audit log, monitor URLs, annotations, API keys |
| `monitor_scheduler` | tokio | Background URL watchlist scheduler for the Monitor tab; tier-gated, polite fetch cadence |
| `network_mode` | — | Product-wide network access control: Enhanced (online verification features) vs Standard (fully local); the single choke point for outbound requests |
| `telemetry` | — | False-positive telemetry scaffold (Phase A, no network calls): builds privacy-stripped report payloads from local FP rows |
| `menu` | tauri | Native OS menu bar replacing Tauri's default auto-menu; custom items emit Tauri events consumed by the frontend |
| `error` | thiserror | `AppError` enum with structured IPC serialisation `{ code, message }`; raw detail goes to the process log, never the frontend |
| `api/` | axum, utoipa | Local REST API on port 8300 (binds 127.0.0.1 only): routes, API-key auth, per-key rate limiting, OpenAPI docs |

`ris.rs` is also present in the source tree but is **not compiled** into v1.0 (no `mod ris;` declaration in `lib.rs`). It is the staging file for the v1.1 BYOK reverse image search feature (JTV-98).

## Python ML Sidecar Services

All services live in `sidecar/app/services/`. They are grouped below by how they participate in the verify pipeline.

### Forensic detectors (run automatically)

These contribute to the numeric trust score. Together with the two Rust-side detectors (EXIF anomaly and C2PA), they make up the 10 automatic detectors in the v1.0 lineup.

| Service | Dependencies | Purpose |
|---------|-------------|---------|
| `ela` | Pillow, NumPy | Error Level Analysis: detect JPEG compression artefact inconsistencies (JPEG-only, gated by the format router) |
| `segmented_ela` | Pillow, OpenCV | Per-cell ELA over an 8x8 grid; flags clusters of anomalous cells where artefacts differ from the image-wide average |
| `noise_analysis` | OpenCV, NumPy | Block-wise noise variance analysis: detect splicing via Laplacian + MAD outlier detection |
| `copy_move` | OpenCV, NumPy | Copy-move forgery detection via SIFT keypoint matching (migrated from ORB, April 2026) |
| `jpeg_ghost` | Pillow, NumPy | Double-compression ghost detection for splice/composite forgeries; contributes at 0.5x weight |
| `colour_temperature` | OpenCV, NumPy | Per-region white-balance analysis in CIELAB space; flags discontinuous colour casts that suggest compositing |
| `deepfake` | NumPy, SciPy, scikit-image, scikit-learn | AI-generated image detection: GBM v4 classifier (84-feature vector, AUC 0.9868) ensembled with the UnivFD probe |
| `clip_detector` | ONNX Runtime, NumPy | CLIP ViT-B/32 embeddings with zero-shot classification plus the trained UnivFD v10 logistic-regression probe (AUC 0.9933); lazy-loaded, optional, graceful degradation |

### On-demand investigation tools

The three on-demand detectors inform investigator judgement but do not contribute to the numeric trust score.

| Service | Dependencies | Purpose |
|---------|-------------|---------|
| `npr` | OpenCV, SciPy | Neighbouring Pixel Relationships: detect AI generation via inter-pixel correlation statistics that camera sensors produce and generators do not |
| `shadow_consistency` | OpenCV, NumPy | Estimate dominant light direction per region; flag incompatible shadow/light directions that suggest compositing |
| `splice_boundary` | OpenCV, NumPy | Splice boundary detection from three orthogonal signals (JPEG block-grid alignment, noise transitions, edge structure) |

### Visualisation and supporting analysis

Investigator aids and contextual signals. None of these feed the trust score.

| Service | Dependencies | Purpose |
|---------|-------------|---------|
| `clahe` | OpenCV, Pillow | Contrast-Limited Adaptive Histogram Equalisation to reveal hidden detail in shadows, highlights, and low-contrast regions |
| `dct_analysis` | SciPy, NumPy | Per-block 8x8 DCT statistics to surface mixed compression levels that indicate splicing |
| `fourier_analysis` | NumPy, Pillow | 2D FFT periodic-pattern detection (AI upsampling artefacts, screen recapture, composites) |
| `frequency_visualisation` | OpenCV, NumPy | FFT magnitude spectrum display annotated with known artefact frequencies |
| `gan_fingerprint` | OpenCV, SciPy | GAN spectral fingerprint visualisation (transposed-convolution upsampling peaks) |
| `jpeg_grid` | OpenCV, Pillow | JPEG quantisation grid visualisation: misaligned 8x8 grids reveal spliced regions |
| `noise_visualisation` | OpenCV, NumPy | Noise residual and block-wise variance heatmaps for analyst inspection |
| `roi_analysis` | OpenCV, NumPy | Re-runs noise, ELA, and frequency analysis on a user-selected region for suspicious-vs-reference comparison |
| `platform_fingerprint` | Pillow | Identify which social platform (WhatsApp, Telegram, Facebook, Instagram, X, Signal, WeChat) re-processed an image via compression signatures and metadata stripping |
| `content_type` | OpenCV, Pillow | Heuristic classifier distinguishing screenshots and scanned documents from camera photographs |

### Knowledge retrieval and description (optional, Ollama-backed)

| Service | Dependencies | Purpose |
|---------|-------------|---------|
| `claim_checker` | httpx | Knowledge-base retrieval as an investigative aid; matches analyst-entered text against a local reference corpus. Not a fact-checker, produces no truth verdicts |
| `knowledge_retriever` | scikit-learn | TF-IDF paragraph index over the local `knowledge_base/` directory with cosine-similarity retrieval |
| `describe_image` | httpx | Natural-language image description via local Ollama LLaVA (Tier 3, optional) |
| `clip_tokenizer` | NumPy | Vendored numpy-only CLIP BPE tokeniser so the sidecar does not need torch |

### Video, audio, and watermark (not in the v1.0 verify scope)

Video deepfake and all audio analysis ship in v1.0.1; watermarking is feature-flagged off in v1.0.

| Service | Dependencies | Purpose |
|---------|-------------|---------|
| `video_metadata` | ffprobe (optional) | Video and audio stream metadata extraction; graceful degradation without FFmpeg |
| `video_frames` | FFmpeg (optional) | Evenly-spaced frame extraction as base64 JPEG for forensic analysis and thumbnails |
| `video_deepfake` | NumPy, Pillow | Per-frame image deepfake analysis aggregated into a video-level verdict |
| `audio_metadata` | ffprobe (optional) | Audio stream metadata (codec, sample rate, channels, duration, bitrate) |
| `audio_deepfake` | NumPy | Two-stage ensemble detector for AI-generated speech and voice cloning (Sprint 35 skeleton) |
| `enf_analysis` | SciPy, NumPy | Electrical Network Frequency (mains hum) analysis for temporal and geographic provenance |
| `transcription` | faster-whisper (optional) | Speech transcription from audio and video files |
| `watermark` | OpenCV, invisible-watermark | Invisible DWT-DCT-SVD watermark embed and extract |

## Detector Lineup (v1.0)

13 forensic detectors. 10 run automatically: EXIF anomaly and C2PA (Rust core), plus ELA, noise, copy-move, the deepfake GBM + UnivFD ensemble, JPEG Ghost (0.5x weight), segmented ELA, colour temperature, and CLIP detection (sidecar). 3 are on-demand investigation tools: NPR, shadow consistency, and splice boundary; they inform investigator judgement but do not contribute to the numeric trust score. The database persists the exact lineup per verification (`detectors_run`, schema v6) so report renderers can distinguish "detector ran and returned null" from "detector not run in this build or mode".

## Database Schema

The live schema is version 7 (tracked via SQLite `user_version`, migrated incrementally on open). The four core tables are shown below; later migrations added monitor URLs and events, annotations, false-positive reports, API keys, and the `detectors_run` column on verifications. See `src-tauri/src/db.rs` for the authoritative schema.

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
Immutable, hash-chained record of all actions for compliance. Each entry's hash incorporates the previous entry's hash; `verify_audit_chain()` validates the full chain.

| Column | Type | Description |
|--------|------|-------------|
| log_id | TEXT PK | UUID v4 |
| action | TEXT | sign/verify/watermark/export/fingerprint |
| target_type | TEXT | asset/verification |
| target_id | TEXT | ID of target |
| details | TEXT | JSON details |
| operator_id | TEXT | Operator identifier (default: local_user) |
| algorithm_metadata | TEXT | JSON algorithm parameters and versions |
| prev_hash | TEXT | Entry hash of the previous audit row (chain link) |
| entry_hash | TEXT | SHA-256 over this entry's fields plus prev_hash |
| hash_version | INTEGER | Hash scheme version |
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
Input (file drop / URL) → Format Router
                              │
            ┌─────────────────┼──────────────────┐
            ▼                 ▼                   ▼
       EXIF Analysis     C2PA Manifest      Sidecar Forensics
       (anomaly detect,  (read & verify     (if sidecar available)
        trust score)      credentials)           │
            │                 │        ┌─────────┼─────────┬──────────┐
            │                 │        ▼         ▼         ▼          ▼
            │                 │      ELA      Noise    Copy-Move  Deepfake
            │                 │    (JPEG     (block    (SIFT     (GBM v4 +
            │                 │     artefact  variance  keypoint  UnivFD
            │                 │     diff)     + MAD)    match)    ensemble)
            │                 │        │         │         │          │
            │                 │      + JPEG Ghost, segmented ELA,     │
            │                 │        colour temperature, CLIP       │
            │                 │        │         │         │          │
            └─────────────────┼────────┴─────────┴─────────┴──────────┘
                              ▼
                    Trust Score Computation (compute_trust)
                    (forensic primary, EXIF corroborating at 20% cap,
                     C2PA adjustment, composite cap, deepfake verdict ceiling)
                              │
                              ▼
                    Verification Result
                    (scores, heatmaps, signals, findings)
```

## Security Model

- **No network calls** except to localhost (Ollama, sidecar)
- **No telemetry** or analytics
- **No accounts** or authentication
- **No cloud storage** — all data in local SQLite + filesystem
- **CSP enforced** via Tauri configuration
- **File access** scoped via Tauri capability permissions
