# Jura Trace — Esper Machine: Advanced Image Investigation System

**Document type**: Feature vision and technical design
**Prepared**: 28 March 2026
**Status**: Proposal (pre-sprint)
**Audience**: Engineering, product, pilot partners

---

## 1. The Vision

In Ridley Scott's *Blade Runner*, the Esper Machine lets Deckard zoom into a photograph, rotate around corners, enhance impossible detail, and extract intelligence that the naked eye cannot see. It is science fiction. But the *intent* behind it — to interrogate an image until it yields every piece of information it contains — is not fiction. It is exactly what journalists, OSINT investigators, insurance adjusters, archivists, and human rights documenters need every day.

Jura Trace already answers the question **"Is this image AI-generated?"** with a 21-detector forensic pipeline. The Esper Machine extends the question to the harder, more consequential ones:

- **"Is this image what the caption claims it is?"** (geolocation, dating, contextual verification)
- **"Has this image been composited from multiple sources?"** (region-level forensic inspection)
- **"What can this image tell me that isn't visible at first glance?"** (enhanced visualisation, metadata intelligence, sensor forensics)

These are not deepfake questions. They are *verification* questions — and they are the questions that matter most in misinformation, insurance fraud, legal evidence, and cultural heritage authentication.

### The Three Scenarios That Define the Requirements

**Scenario 1 — Conflict zone re-use**: A photograph genuinely depicts a bombing, but it is from Aleppo 2016, not Kharkiv 2024. It IS a real photograph. Every deepfake detector will correctly say "authentic". The failure mode is not detection — it is *contextualisation*. The investigator needs sun angle analysis, script identification on visible signage, vegetation typing, weather cross-referencing, and reverse image search to establish provenance.

**Scenario 2 — Insurance evidence from a different property**: A claimant submits photographs of genuine structural damage. Nothing has been cloned or spliced within the image. Copy-move detection is silent. The investigator needs GPS coordinate mapping, EXIF consistency checking, camera fingerprint matching across the claim portfolio, and enhanced zoom to read serial numbers, address markers, or utility stickers in the background.

**Scenario 3 — Decontextualised protest image**: A real photograph of a genuine protest is captioned as occurring in Country A when it actually took place in Country B. The metadata has been stripped. The investigator needs OCR of visible signage, language/script identification, architectural style matching, licence plate format recognition, and historical weather data cross-referencing.

In all three cases, the existing pipeline returns "authentic" — because the image IS authentic. The Esper Machine provides the tools to verify not authenticity, but *context, provenance, and claim accuracy*.

---

## 2. System Architecture

The Esper Machine introduces a new investigation paradigm: **interactive, analyst-driven inspection** alongside the existing automated pipeline. The automated pipeline runs first and presents its verdict. The Esper Machine then provides tools for the investigator to test specific hypotheses about the image.

```
┌─────────────────────────────────────────────────────────────────┐
│                    VERIFY PAGE — ESPER MODE                      │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Image Canvas (pan/zoom/rotate)                          │   │
│  │  ┌──────────────┐  ┌──────────────────────────────────┐  │   │
│  │  │ Loupe (deep   │  │ Overlay layer:                   │  │   │
│  │  │ zoom with     │  │  - ELA heatmap                   │  │   │
│  │  │ enhancement)  │  │  - Noise pattern                 │  │   │
│  │  └──────────────┘  │  - JPEG grid                     │  │   │
│  │                     │  - Colour channels (R/G/B)       │  │   │
│  │                     │  - Clone regions                 │  │   │
│  │                     │  - Splice boundaries             │  │   │
│  │                     │  - Shadow directions              │  │   │
│  │                     └──────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────────┐ │
│  │ Enhancement   │ │ Intelligence │ │ Context                  │ │
│  │ ─────────     │ │ ─────────    │ │ ─────────                │ │
│  │ Histogram eq. │ │ OCR / text   │ │ Reverse image search     │ │
│  │ Channel sep.  │ │ GPS → map    │ │ Sun angle calculator     │ │
│  │ Frequency sep.│ │ Camera ID    │ │ Weather cross-ref        │ │
│  │ Noise render  │ │ Edit history │ │ Earliest publication     │ │
│  │ Sharpen       │ │ Thumb vs img │ │ Script/language ID       │ │
│  │ Invert        │ │ PRNU match   │ │ Vegetation season est.   │ │
│  └──────────────┘ └──────────────┘ └──────────────────────────┘ │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Investigation Notes (timestamped, exportable)            │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Interaction Model

The Esper Machine is NOT an automated pipeline that runs all tools. It is an **analyst workbench** — tools are invoked on demand, results are overlaid on the canvas, and the investigator builds a case by combining multiple lines of evidence. This matches the GIJN SIFT methodology (Stop, Investigate, Find, Trace) and the Berkeley Protocol for digital open source investigations.

Each tool produces:
1. A **visual overlay** or **data panel** that appears on/beside the canvas
2. A **structured finding** that is logged in the investigation notes
3. An **exportable artefact** included in the case ZIP

---

## 3. Feature Catalogue

### 3.1 Interactive Image Canvas

The foundation. Every other tool builds on top of this.

| Feature | Description | Implementation | Effort |
|---------|-------------|----------------|--------|
| **Pan and zoom** | Smooth pan with trackpad/mouse, pinch-to-zoom, scroll wheel zoom. Zoom range 1x-32x. | HTML5 Canvas with CSS transforms, or a dedicated library (OpenSeadragon, Leaflet for non-map tiling, or Panzoom). SvelteKit component wrapping a `<canvas>` element. | S (3 pts) |
| **Deep zoom with enhancement** | At >8x zoom, apply Lanczos upscaling + unsharp mask to reveal sub-pixel detail. Optional wavelet-based super-resolution. | Canvas `imageSmoothingQuality: 'high'` for basic. Sidecar endpoint for server-side Lanczos + unsharp mask on a crop region. | M (5 pts) |
| **Rotation** | Arbitrary rotation (not just 90 degree increments). Useful for aligning horizon lines, reading rotated text. | CSS transform on canvas container. Pure frontend. | XS (1 pt) |
| **Measurement tool** | Click two points to measure pixel distance. With known focal length from EXIF, estimate real-world distance. | Canvas overlay with SVG line and label. Frontend only. | S (2 pts) |
| **Region selection** | Draw rectangle or polygon to select a region for targeted analysis (send selected crop to any sidecar endpoint). | Canvas overlay with SVG polygon drawing. Selection coordinates sent to sidecar as crop bounds. | S (3 pts) |
| **Side-by-side comparison** | Place two images or two views (original vs overlay) side by side with synchronised pan/zoom. | Dual canvas with linked transform state. | M (5 pts) |
| **Overlay layer system** | Toggle visibility of analysis overlays (ELA heatmap, noise map, JPEG grid, etc.) at adjustable opacity. Layers stack. | Canvas composite operations or SVG overlay. Each overlay is a base64 image from sidecar rendered as a semi-transparent layer. | M (5 pts) |

**Already exists**: The verify page currently renders ELA heatmaps, noise heatmaps, and segmented ELA grids as static images below the source image. The Esper Machine replaces this with interactive overlays on a single canvas.

### 3.2 Image Enhancement Tools

Client-side image processing applied to the canvas view. These do NOT modify the source image — they are investigative visualisations.

| Feature | Description | Implementation | Effort |
|---------|-------------|----------------|--------|
| **Histogram equalisation** | Enhance contrast to reveal detail in dark/light regions. Global (CLAHE) and local modes. | Canvas `getImageData()` → pixel manipulation → `putImageData()`. Or CSS `filter: contrast()` for simple version. Full CLAHE via sidecar. | S (2 pts) — CSS; M (5 pts) — full CLAHE |
| **Colour channel separation** | View R, G, B channels independently. Reveals information invisible in the composite. Useful for watermark detection, hidden text, and manipulation artefacts. | Frontend: `getImageData()`, zero out two channels, `putImageData()`. Pure client-side. | S (2 pts) |
| **Colour inversion** | Invert all pixel values. Reveals hidden patterns, especially in low-contrast regions. | CSS `filter: invert(1)`. Trivial. | XS (1 pt) |
| **Brightness/contrast/gamma** | Adjustable sliders for brightness, contrast, gamma correction. | CSS filters: `brightness()`, `contrast()`. Gamma via canvas pixel manipulation. | S (2 pts) |
| **Frequency separation** | Split image into low-frequency (structure/shapes) and high-frequency (detail/texture) components. High-pass reveals manipulation at fine scales; low-pass reveals structural inconsistencies. | Sidecar endpoint: Gaussian blur for low-pass, original minus low-pass for high-pass. Return both as base64 overlay images. | S (3 pts) |
| **Noise pattern visualisation** | Extract and render the image's noise residual. Uniform noise = single source. Discontinuities = splicing. Different from block-wise noise variance — this renders the actual noise pattern at pixel level. | Sidecar endpoint: median filter denoising, compute residual (original - denoised), histogram-equalise, return as base64. Related to existing `noise_analysis.py` but produces a full-resolution noise image rather than block statistics. | S (3 pts) |
| **JPEG compression grid overlay** | Render the 8x8 JPEG block grid. Misaligned grids between regions reveal double-compression or splicing. | Sidecar: detect primary JPEG grid phase via block boundary analysis (DCT coefficient discontinuities). Return grid overlay image with misaligned regions highlighted. Related to existing `splice_boundary.py` JPEG grid signal but rendered as a visual overlay. | M (5 pts) |
| **Edge enhancement** | Sobel/Canny edge detection rendered as overlay. Reveals structural artefacts, GAN checkerboard patterns, and unnatural boundary smoothing. | Sidecar: `cv2.Canny()` or Sobel filter, return as base64 overlay. | S (2 pts) |
| **Fourier spectrum visualisation** | Render the 2D FFT magnitude spectrum. GAN-generated images show characteristic spectral peaks. | Sidecar: `np.fft.fft2()`, log magnitude, return as base64 image. Already partially computed in `deepfake.py` — needs to be exposed as a returnable visualisation. | S (2 pts) |

### 3.3 Metadata Intelligence

Extracting investigative leads from non-pixel data.

| Feature | Description | Implementation | Effort |
|---------|-------------|----------------|--------|
| **GPS coordinate mapping** | If EXIF GPS tags are present, render the coordinates on an interactive map. Show accuracy radius. | Frontend: Leaflet.js or MapLibre GL JS with OpenStreetMap tiles. Coordinates already extracted by Rust `metadata.rs`. Requires no network for the base case (tile URL only). | M (5 pts) |
| **Camera fingerprint database** | Compare EXIF Make/Model/Lens/SerialNumber across multiple images to determine if they came from the same device. Store camera profiles in SQLite. | Rust: new `camera_profiles` table in `db.rs`. Compare incoming EXIF against stored profiles. Frontend: "Other images from this camera" panel. | M (5 pts) |
| **Thumbnail vs full image comparison** | Extract embedded JPEG thumbnail, display side-by-side with full image. If they differ (different crop, different content), the image was modified after the camera wrote the thumbnail. | Rust: extract thumbnail from EXIF APP1 marker (already parsed by `rexiv2` / `kamadak-exif`). Frontend: side-by-side display with difference highlighting. | S (3 pts) |
| **Edit history reconstruction** | Parse XMP `xmpMM:History` to reconstruct the software pipeline (e.g., "Camera RAW 2024 → Photoshop → Export"). Display as a timeline. | Rust: parse XMP via existing metadata extraction. XMP history actions are structured as `stEvt:action`, `stEvt:softwareAgent`, `stEvt:when`. | S (3 pts) |
| **EXIF consistency cross-check** | Compare EXIF data against known camera capabilities. E.g., claimed ISO 100 on a phone that starts at ISO 50; resolution that does not match any known sensor for that make/model. | Rust or sidecar: camera specification database (JSON/SQLite). Compare extracted EXIF against known specifications. Start with top 50 camera models. | L (8 pts) |
| **Quantisation table fingerprinting** | Extract JPEG quantisation tables and match against known software signatures (Photoshop, GIMP, social media platforms). Different from quality factor — the specific Q-table values identify the software. | Sidecar: `PIL` Q-table extraction, compare against database of known signatures (Photoshop CS6 standard, Instagram, WhatsApp, Twitter, Facebook, etc.). | M (5 pts) |
| **Steganography scan** | Check for data hidden in LSB (least significant bit) planes. Render LSB plane as image. Detect statistical anomalies (chi-square test, RS analysis). | Sidecar: extract LSB plane per channel, run chi-square test on pairs of values, return LSB visualisation and p-value. | M (5 pts) |

### 3.4 Geolocation and Temporal Analysis

The hardest and most valuable tools for the three core scenarios.

| Feature | Description | Implementation | Effort |
|---------|-------------|----------------|--------|
| **Sun angle calculator** | From shadow direction and length in the image (analyst-marked), plus claimed latitude/date, calculate whether the sun position is consistent. Or: given shadow angle, compute possible lat/lon/date combinations. | Frontend: SunCalc.js library (open source, no network needed). Analyst marks shadow tip and base → direction and relative length. Calculator shows solar azimuth/elevation for claimed time/location. Mismatch is investigative lead. | M (5 pts) |
| **Shadow direction visualisation** | Automated extraction of dominant shadow directions across the image. If shadows in different parts of the image point in different directions, the image is composited. | Already exists: `shadow_consistency.py`. Needs visual overlay showing arrows per region overlaid on the canvas, not just a score. | S (3 pts) |
| **OCR / text extraction** | Extract all visible text: signs, documents, screens, licence plates, watermarks. Identify language and script. | Already exists: `describe_image.py` has `extract_text_from_image()` via Ollama LLaVA. For non-Ollama: Tesseract OCR (`pytesseract`). For macOS: native Vision framework OCR (already referenced in MEMORY.md). Multi-script support is critical for the geolocation use case. | S (2 pts) — wire existing; M (5 pts) — add Tesseract fallback |
| **Script and language identification** | From extracted text, identify the script (Latin, Cyrillic, Arabic, Devanagari, CJK, etc.) and probable language. Narrows geographic region. | Python: `langdetect` or `langid` library on extracted text. Script identification via Unicode block analysis (no ML needed — pure Unicode range checking). | S (2 pts) |
| **Vegetation season estimation** | Classify visible vegetation as dormant/budding/full leaf/autumn. Combined with claimed date, checks seasonal consistency for the hemisphere and latitude. | Sidecar: CLIP zero-shot classification with vegetation prompts ("deciduous trees with full green leaves", "bare winter trees", "autumn foliage", "spring blossom"). Requires CLIP model (already optional dependency). | S (3 pts) |
| **Weather cross-reference** | Compare visible weather conditions (clear sky, overcast, rain, snow) with historical weather data for claimed location and date. | Two parts: (1) Weather condition extraction via CLIP zero-shot or analyst annotation. (2) Historical weather API lookup (Open-Meteo historical API — free, no key required, CC BY 4.0). **This is the one feature that requires a network call.** Must be optional and clearly disclosed to the user. | M (5 pts) |
| **Licence plate format recognition** | Identify licence plate format (not read the number) to narrow down country/region. E.g., EU blue strip, US state format, UK yellow rear plate. | Sidecar: template matching or CLIP zero-shot with plate format prompts. Or analyst-assisted: "What plate format do you see?" with a visual guide. | S (3 pts) |
| **Architectural style matching** | Identify distinctive architectural elements (minarets, pagodas, Tudor half-timbering, Soviet brutalist blocks) to narrow geographic region. | CLIP zero-shot with architectural vocabulary. Similar to vegetation but with built environment terms. | S (3 pts) |
| **Road marking / signage conventions** | Different countries use different road marking styles (white/yellow centre lines, roundabout signage, pedestrian crossing patterns). Visual guide for analyst reference. | Frontend only: reference gallery of road marking conventions by region. No automation — purely a visual aide-memoire for the investigator. | S (2 pts) |
| **Reverse image search launcher** | One-click buttons to search the image on Google Images, TinEye, and Yandex. Opens in external browser. Does NOT send the image through Jura Trace servers — opens the platform's upload page or uses their bookmarklet protocol. | Frontend: buttons that open URLs. For Google: `https://lens.google.com/uploadbyurl?url=` (requires the image to be URL-accessible, which conflicts with local-first). Alternative: copy image to clipboard with instructions, or use TinEye's API (requires key). Best approach: save a temporary reduced-resolution copy and open it in the default browser with a `file://` URL, or use platform-specific share sheet. | S (3 pts) — basic launcher; L (8 pts) — with TinEye API integration |
| **Earliest known publication** | Search for the earliest indexed version of this image online. Critical for the "this photo is from a different event" scenario. | External: TinEye "Sort by oldest" is the gold standard. Requires TinEye API key ($). Google Lens also shows dates but less reliably. This is inherently a network feature. | M (5 pts) — with TinEye API |

### 3.5 Sensor Forensics (Research-Grade)

These are the most technically demanding features. They require significant R&D and may not achieve production reliability.

| Feature | Description | Implementation | Effort |
|---------|-------------|----------------|--------|
| **PRNU (Photo Response Non-Uniformity) fingerprinting** | Every camera sensor has a unique noise pattern caused by manufacturing variations in individual pixels. Extract this pattern and use it to: (a) verify that two images came from the same camera, (b) verify that an image matches a known camera. | Sidecar: extract PRNU via flat-field estimation (average multiple images from same camera, subtract scene content). Compare via peak-to-correlation energy (PCE). Requires multiple reference images from the same camera. Literature: Lukas, Fridrich & Goljan, 2006. Libraries: `prnu_python` (GitHub). | XL (13 pts) — core algorithm; XXL (21 pts) — production system with camera database |
| **Source camera identification** | Given a single image, estimate the camera model from sensor artefacts (colour filter array interpolation pattern, lens distortion profile, noise characteristics). | Sidecar: CFA pattern analysis, radial distortion estimation, noise profile extraction. Compare against database of known camera models. Research-grade — accuracy varies significantly. | XL (13 pts) |
| **Double JPEG compression detection** | Detect whether an image has been JPEG-compressed more than once (indicates editing). Analyse DCT coefficient histograms for the characteristic "comb" pattern of double quantisation. | Sidecar: DCT coefficient histogram extraction, periodicity analysis. Related to existing `jpeg_ghost.py` but more rigorous statistical test. | M (5 pts) |
| **Copy-move with feature matching** | Enhanced copy-move detection using SIFT/SURF/ORB feature descriptors instead of block matching. Finds geometrically transformed copies (scaled, rotated, reflected). | Sidecar: OpenCV `cv2.SIFT_create()` + FLANN matcher. Return matched region pairs with transformation parameters. Upgrade of existing `copy_move.py`. | M (5 pts) |
| **Splicing detection via noise level function** | Estimate the noise level function (NLF) — the relationship between pixel intensity and noise variance — across the image. Spliced regions from a different source have a different NLF. | Sidecar: block-wise noise estimation at multiple intensity levels, fit NLF curve per region, detect discontinuities. Based on Mahdian & Saic, 2009. | L (8 pts) |

### 3.6 Investigation Workflow Tools

Tools that support the investigative process rather than image analysis.

| Feature | Description | Implementation | Effort |
|---------|-------------|----------------|--------|
| **Investigation notes** | Timestamped free-text notes attached to the current analysis. Each tool invocation auto-generates a structured note. Analyst can add manual observations. | Frontend: text input below canvas. SQLite: new `investigation_notes` table linked to `verifications.verification_id`. | S (3 pts) |
| **Annotation layer** | Draw arrows, circles, text labels on the image to mark findings. Annotations are saved with the case and included in exports. | Frontend: SVG overlay on canvas with drawing tools (circle, arrow, text, freehand). Serialise as JSON. | M (5 pts) |
| **Case export enhancement** | Extend existing ZIP export to include: all overlay images, investigation notes, annotations, a timeline of tools used, and a narrative summary. | Extend `zip.ts`. Add sidecar endpoint to generate overlay images on demand (currently they are computed during verification but not all are persisted). | S (3 pts) |
| **Comparison portfolio** | Load multiple images into the investigation (e.g., all photos from an insurance claim) and compare metadata, camera fingerprints, noise profiles, and EXIF consistency across the set. | Frontend: multi-image panel. Rust: batch EXIF extraction and cross-comparison. New `investigation_portfolio` table in SQLite. | L (8 pts) |
| **Chain of custody log** | Every tool invocation, annotation, and note is logged with timestamp, tool name, parameters, and analyst ID. Produces an auditable record of the investigation process. This extends the existing audit log. | Extend existing `audit_log` table. Each Esper tool call writes a log entry. Include in case export. | S (3 pts) |

---

## 4. Mapping to the Existing Pipeline

The Esper Machine does not replace the automated pipeline — it extends it. Here is how each existing detector maps to Esper Machine tools:

| Existing Detector | Current Output | Esper Machine Enhancement |
|---|---|---|
| ELA (`ela.py`) | Heatmap image + score | Interactive overlay on canvas, adjustable opacity, region selection for targeted re-analysis at different quality levels |
| Noise analysis (`noise_analysis.py`) | Block variance heatmap + score | Full-resolution noise residual visualisation, noise level function estimation |
| Copy-move (`copy_move.py`) | Matched regions + score | Feature-matching upgrade (SIFT/ORB), geometric transform detection, interactive region highlighting |
| Deepfake ensemble (`deepfake.py`) | 21 signals + score | Fourier spectrum visualisation, individual signal overlays, feature importance display |
| NPR (`npr.py`) | Score | Pixel relationship heatmap overlay |
| Chromatic aberration (`chromatic_aberration.py`) | Score + radial fit | Interactive radial CA overlay showing fit vs actual pattern |
| JPEG ghost (`jpeg_ghost.py`) | Ghost images at multiple quality levels | Quality level slider with real-time ghost overlay update |
| Segmented ELA (`segmented_ela.py`) | Grid heatmap + clusters | Interactive grid overlay, click-to-inspect individual cells |
| Shadow consistency (`shadow_consistency.py`) | Direction vectors + score | Arrow overlay showing light direction per region on canvas |
| Colour temperature (`colour_temperature.py`) | CIELAB segmentation + score | Colour temperature map overlay, click region to see absolute values |
| Splice boundary (`splice_boundary.py`) | Boundary lines + score | Interactive boundary overlay with per-signal breakdown (JPEG grid, noise, feathering) |
| CLIP detector (`clip_detector.py`) | AI/authentic classification | Extend vocabulary for geolocation (vegetation, architecture, weather, plate formats) |
| EXIF analysis (`exif_anomaly.rs`) | Anomaly flags + score | GPS mapping, thumbnail comparison, edit history timeline, camera fingerprint matching |
| C2PA (`c2pa.rs`) | Manifest validation | Claim-generator AI signal, ingredient provenance chain visualisation |
| Watermark extraction | Institution name + confidence | Visual overlay showing watermark strength map |

---

## 5. Implementation Tiers

### Tier 0 — Foundation (Sprint scope: 1 sprint)

The interactive canvas and overlay system. Without this, nothing else works.

1. **Panzoom canvas component** — pan, zoom (1x-32x), rotation
2. **Overlay layer toggle** — render existing heatmaps (ELA, noise, segmented ELA) as semi-transparent overlays on the source image instead of separate images
3. **Region selection** — draw a rectangle to crop and send to any sidecar endpoint
4. **Basic CSS enhancements** — brightness, contrast, inversion via CSS filters (zero sidecar cost)

Effort: ~12 story points. Dependencies: none.

### Tier 1 — Enhancement Toolkit (1 sprint)

Client-side and lightweight sidecar tools that provide immediate investigative value.

1. **Colour channel separation** — pure frontend, R/G/B individual channels
2. **Histogram equalisation** — global via canvas, CLAHE via sidecar
3. **Frequency separation** — sidecar endpoint returning low-pass and high-pass images
4. **Noise pattern visualisation** — sidecar endpoint returning full-resolution noise residual
5. **JPEG grid overlay** — sidecar endpoint returning grid with misalignment highlighting
6. **Edge enhancement overlay** — Canny/Sobel via sidecar
7. **Fourier spectrum display** — expose existing FFT computation as returnable image

Effort: ~20 story points. Dependencies: Tier 0 canvas.

### Tier 2 — Metadata Intelligence (1 sprint)

Deeper EXIF exploitation and structured metadata analysis.

1. **GPS coordinate mapping** — Leaflet/MapLibre with OSM tiles
2. **Thumbnail vs full image comparison** — extract and compare
3. **Edit history timeline** — XMP history parsing and display
4. **Q-table fingerprinting** — software identification database
5. **Camera fingerprint database** — cross-image device matching
6. **Investigation notes** — timestamped notes and chain of custody log

Effort: ~22 story points. Dependencies: Tier 0 canvas (for map integration).

### Tier 3 — Geolocation and Temporal (1-2 sprints)

The highest-value investigative tools for the three core scenarios.

1. **Sun angle calculator** — SunCalc.js integration with shadow marking
2. **OCR with script identification** — Tesseract fallback + Unicode script analysis
3. **Shadow direction overlay** — visual arrows from existing shadow consistency data
4. **Vegetation and weather estimation** — CLIP zero-shot with geolocation vocabularies
5. **Reverse image search launcher** — one-click TinEye/Google Lens/Yandex
6. **Weather cross-reference** — Open-Meteo historical API (network feature, optional)
7. **Annotation layer** — drawing tools for marking findings

Effort: ~25-30 story points. Dependencies: Tier 0 canvas, CLIP model (optional).

### Tier 4 — Advanced Forensics (2 sprints, research-grade)

Sensor-level forensics for professional investigators.

1. **PRNU fingerprinting** — camera sensor identification
2. **Feature-matching copy-move** — SIFT/ORB upgrade
3. **Noise level function analysis** — per-region NLF estimation
4. **Double JPEG statistical test** — DCT histogram periodicity
5. **Steganography scanner** — LSB analysis and chi-square test
6. **Comparison portfolio** — multi-image investigation workspace
7. **EXIF specification database** — camera capability cross-checking

Effort: ~40+ story points. Dependencies: Tier 2 metadata intelligence.

---

## 6. Priority Sequence: Maximum Investigative Value Per Sprint

Based on the three scenarios and discussions with the GIJN open-source investigations methodology:

### Phase A — "See More" (Tiers 0 + 1 combined, 2 sprints)

**Rationale**: The single highest-impact change is moving from "look at separate static images" to "interactively interrogate a single canvas with overlays". This is the Esper Machine moment — the point where the tool *feels* like an investigation workbench rather than a report viewer.

**Sprint A1 (22 pts)**:
- Panzoom canvas with zoom 1x-32x (3 pts)
- Overlay layer system with opacity control (5 pts)
- Existing heatmaps wired as overlays: ELA, noise, segmented ELA (3 pts)
- Region selection for targeted analysis (3 pts)
- CSS enhancement filters: brightness, contrast, inversion, gamma (2 pts)
- Colour channel separation — pure frontend (2 pts)
- Shadow direction arrow overlay from existing data (3 pts)
- Investigation notes panel with timestamped entries (1 pt — basic version)

**Sprint A2 (20 pts)**:
- Frequency separation endpoint + overlay (3 pts)
- Full-resolution noise residual visualisation endpoint + overlay (3 pts)
- JPEG grid overlay with misalignment detection (5 pts)
- Fourier spectrum visualisation (2 pts)
- Edge enhancement overlay (2 pts)
- Deep zoom with Lanczos enhancement for >8x (3 pts)
- Side-by-side comparison mode (2 pts — basic linked pan/zoom)

### Phase B — "Know More" (Tiers 2 + 3 partial, 2 sprints)

**Rationale**: Metadata intelligence and geolocation tools address Scenarios 1-3 directly. GPS mapping and sun angle analysis are the highest-ROI tools for the conflict zone and insurance scenarios.

**Sprint B1 (22 pts)**:
- GPS coordinate mapping with Leaflet/OSM (5 pts)
- Sun angle calculator with shadow marking (5 pts)
- Thumbnail vs full image comparison (3 pts)
- Edit history timeline from XMP (3 pts)
- OCR wiring — Tesseract fallback for non-Ollama users (3 pts)
- Script and language identification (2 pts)
- Chain of custody audit log extension (1 pt)

**Sprint B2 (22 pts)**:
- Q-table software fingerprinting with database of 30+ signatures (5 pts)
- Camera fingerprint database and cross-image matching (5 pts)
- Reverse image search launcher (3 pts)
- Vegetation season estimation via CLIP (3 pts)
- Annotation drawing layer (5 pts)
- Weather condition extraction via CLIP (1 pt — classification only, no API)

### Phase C — "Prove More" (Tier 3 remainder + Tier 4, 2-3 sprints)

**Rationale**: Research-grade forensics for professional and enterprise tiers.

- Weather cross-reference with Open-Meteo API (5 pts)
- PRNU fingerprinting (13 pts)
- Feature-matching copy-move upgrade (5 pts)
- Noise level function analysis (8 pts)
- Steganography scanner (5 pts)
- Comparison portfolio workspace (8 pts)
- EXIF specification database (8 pts)
- Earliest publication via TinEye API (5 pts)
- Enhanced case export with all overlays, notes, annotations (3 pts)

---

## 7. Licence Tier Allocation

| Feature | Community (Flint) | Professional (Stratum) | Team (Geode) | Enterprise (Bedrock) |
|---------|:-:|:-:|:-:|:-:|
| Panzoom canvas + basic zoom | x | x | x | x |
| CSS filters (brightness, contrast, invert) | x | x | x | x |
| Overlay system (ELA, noise heatmaps) | x | x | x | x |
| Colour channel separation | x | x | x | x |
| Investigation notes (basic) | x | x | x | x |
| Region selection + targeted analysis | | x | x | x |
| Deep zoom with enhancement | | x | x | x |
| Frequency/noise/JPEG grid overlays | | x | x | x |
| Fourier spectrum visualisation | | x | x | x |
| GPS coordinate mapping | | x | x | x |
| Thumbnail vs image comparison | | x | x | x |
| Edit history timeline | | x | x | x |
| Sun angle calculator | | x | x | x |
| OCR + script identification | | x | x | x |
| Shadow direction overlay | | x | x | x |
| Edge enhancement overlay | | x | x | x |
| Side-by-side comparison | | x | x | x |
| Reverse image search launcher | | x | x | x |
| Annotation drawing layer | | | x | x |
| Camera fingerprint database | | | x | x |
| Q-table software fingerprinting | | | x | x |
| Comparison portfolio (multi-image) | | | x | x |
| Vegetation/weather estimation (CLIP) | | | x | x |
| Chain of custody audit log | | | x | x |
| Weather cross-reference (network) | | | | x |
| PRNU fingerprinting | | | | x |
| Noise level function analysis | | | | x |
| Feature-matching copy-move | | | | x |
| Steganography scanner | | | | x |
| EXIF specification database | | | | x |
| TinEye API integration | | | | x |
| Enhanced case export (full) | | | x | x |

**Design principle**: Community tier gets the canvas and basic visualisation — enough to be genuinely useful. Professional tier gets the full enhancement toolkit and metadata intelligence — this is the journalist/OSINT tier. Team tier adds collaborative features (annotations, portfolio, audit trail). Enterprise tier adds research-grade forensics and network-dependent features.

---

## 8. Technical Considerations

### 8.1 Local-First Compliance

Every feature in Tiers 0-2 and most of Tier 3 requires zero network access. The exceptions are:

1. **Weather cross-reference** — requires Open-Meteo API call. Must be: opt-in, clearly labelled as a network feature, functional without it.
2. **Reverse image search** — opens external browser. The image leaves the device only when the user explicitly chooses to search.
3. **GPS map tiles** — Leaflet requires tile server access for the map background. Mitigation: support offline MBTiles for air-gapped environments; default to a simple lat/lon display with "Load map" button.
4. **TinEye API** — network call for earliest publication search. Enterprise tier only, opt-in.

All other features process data entirely on-device, consistent with Jura Trace's local-first architecture.

### 8.2 Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Canvas pan/zoom | 60 fps | CSS transforms, no pixel manipulation during interaction |
| Overlay toggle | <100 ms | Pre-rendered overlay images, toggle visibility only |
| CSS filter change | <16 ms | Browser-native CSS filter pipeline |
| Colour channel separation | <50 ms | Canvas `getImageData()` + channel zeroing on the resolution displayed (not full source) |
| Sidecar enhancement (crop) | <2 s | Lanczos upscale + unsharp mask on a cropped region |
| Frequency separation | <3 s | Gaussian blur + subtraction on full image |
| Noise residual | <3 s | Wavelet denoising + residual computation |
| JPEG grid analysis | <5 s | DCT coefficient analysis across full image |
| Sun angle calculation | <10 ms | Pure JavaScript trigonometry (SunCalc.js) |
| OCR (Tesseract) | <5 s | Single image, depends on text density |
| PRNU extraction | <30 s | Heavy computation, enterprise tier only |

### 8.3 New Sidecar Endpoints

Tier 0-1 requires these new endpoints:

```
POST /forensics/enhance/frequency    → {low_pass_base64, high_pass_base64}
POST /forensics/enhance/noise        → {noise_residual_base64, statistics}
POST /forensics/enhance/jpeg-grid    → {grid_overlay_base64, misaligned_regions}
POST /forensics/enhance/edges        → {edge_overlay_base64}
POST /forensics/enhance/fourier      → {spectrum_base64, peak_frequencies}
POST /forensics/enhance/histogram-eq → {equalised_base64}  (CLAHE)
POST /forensics/enhance/crop         → {enhanced_crop_base64}  (Lanczos + sharpen)
```

Tier 2-3 requires:

```
POST /forensics/metadata/qtable      → {software_match, q_tables, confidence}
POST /forensics/metadata/stego       → {lsb_plane_base64, chi_square_p, rs_score}
POST /forensics/ocr                  → {text, language, script, bounding_boxes}
POST /forensics/geolocation/vegetation → {season_estimate, confidence, vegetation_type}
POST /forensics/geolocation/weather  → {conditions, confidence}
POST /forensics/sensor/prnu          → {prnu_pattern_base64, pce_scores}
POST /forensics/copymove/features    → {matched_pairs, transformations, overlay_base64}
```

### 8.4 New Frontend Dependencies

| Library | Purpose | Size | Licence |
|---------|---------|------|---------|
| `panzoom` or `@panzoom/panzoom` | Canvas pan/zoom/pinch | ~8 KB gzipped | MIT |
| `suncalc` | Solar position calculation | ~3 KB gzipped | BSD-2-Clause |
| `leaflet` | Interactive map for GPS coordinates | ~40 KB gzipped | BSD-2-Clause |
| None needed for channel separation, CSS filters, canvas pixel manipulation | — | 0 KB | — |

### 8.5 New Sidecar Dependencies

| Library | Purpose | Size | Licence |
|---------|---------|------|---------|
| `pytesseract` + Tesseract OCR engine | OCR text extraction (non-Ollama fallback) | ~30 MB (engine) | Apache 2.0 |
| `langdetect` or `langid` | Language identification from extracted text | ~2 MB | Apache 2.0 |
| None needed for frequency separation, noise residual, JPEG grid, edge detection, Fourier — all use existing OpenCV/NumPy/SciPy | — | 0 | — |

---

## 9. Relationship to the Berkeley Protocol

The Berkeley Protocol on Digital Open Source Investigations (UC Berkeley Human Rights Center, 2022) is the international standard for using digital information in human rights accountability. The Esper Machine aligns with its principles:

1. **Methodological rigour**: Every tool invocation is logged in the chain of custody. The investigation is reproducible.
2. **Source preservation**: The original image is never modified. All enhancements are non-destructive visualisations.
3. **Corroboration**: The multi-tool approach inherently supports the Protocol's requirement for corroboration — no single tool's output is treated as conclusive.
4. **Documentation**: Investigation notes, annotations, and the case export together form the "digital evidence file" the Protocol requires.
5. **Competence**: The InspectionChecklist, MethodologyPanel, and contextual help system guide investigators through proper procedure.

The Esper Machine transforms Jura Trace from a *detection tool* into an *investigation workbench* — the form factor the Berkeley Protocol envisions.

---

## 10. What This Is NOT

Clarity on scope boundaries:

1. **Not real-time video analysis**. The Esper Machine operates on still images (including frames extracted from video via the existing pipeline). Real-time video investigation is out of scope.
2. **Not facial recognition**. We do not identify individuals. We analyse forensic artefacts. No biometric databases.
3. **Not automated geolocation**. We provide tools that *assist* human analysts in geolocation. The analyst makes the determination, not the algorithm. This is a deliberate design choice: automated geolocation has significant dual-use risks.
4. **Not an OSINT crawler**. We do not scrape social media or crawl the web. Reverse image search opens an external browser; the user decides what to do with the results.
5. **Not infallible**. Every tool has failure modes and limitations. The UI must communicate confidence levels and caveats, not binary verdicts.

---

## 11. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Canvas interaction framerate | 60 fps sustained | Lighthouse performance audit |
| Overlay toggle latency | <100 ms p95 | Frontend performance monitoring |
| Investigation time reduction | 30% faster vs manual process | Pilot partner time-motion study |
| False geolocation rate | <5% on controlled test set | Internal QA with known-location images |
| Investigator confidence (NPS) | >50 among pilot users | Post-investigation survey |
| Case export completeness | 100% of tools used appear in export | Automated test |
| Tool discoverability | 80% of investigators use 3+ Esper tools per session | Usage analytics (local only) |

---

## 12. Open Questions

1. **Canvas library choice**: Panzoom is lightweight but limited. OpenSeadragon supports tiled deep zoom (useful for very large images — 50+ megapixel archival scans). Leaflet can be repurposed for non-geographic image viewing with custom tile layers. Which best serves the 1x-32x zoom range on images up to 100 MP?

2. **Offline map tiles**: For air-gapped deployments (government, military), should we bundle a minimal offline tile set, support MBTiles, or fall back to a simple coordinate display with no map background?

3. **PRNU feasibility**: PRNU fingerprinting requires multiple reference images from the same camera to build a reliable fingerprint. In practice, investigators rarely have this. Is the feature valuable enough for the edge cases where they do (e.g., comparing all photos in an insurance claim portfolio)?

4. **Tesseract vs macOS Vision**: On macOS, the native Vision framework provides excellent OCR without any additional dependency. Should we use platform-native OCR where available and fall back to Tesseract on Linux/Windows? This adds platform-specific code paths but reduces dependency weight.

5. **Network feature disclosure**: How prominently should we flag features that make network calls (weather API, map tiles, reverse image search)? A simple icon? A confirmation dialog? A separate "Network Tools" section?

6. **Dual-use considerations**: Sun angle calculation and geolocation tools are valuable for journalists and human rights investigators — but they could also be used by hostile actors. Is there a responsible disclosure framework we should apply? (The tools themselves are well-established in the OSINT community; we are not creating new capability, only consolidating existing open-source techniques.)

---

## Appendix A: Reference Tools and Inspiration

| Tool | What It Does | How It Relates |
|------|-------------|----------------|
| Forensically (29a.ch) | Browser-based image forensics: ELA, noise, clone detection, EXIF | Closest existing equivalent to the Esper Enhancement Toolkit. We go further with the overlay system and investigation workflow. |
| FotoForensics | Online ELA and metadata analysis | Popularised ELA analysis. Their interface is the baseline we must exceed. |
| InVID/WeVerify | Browser extension for video/image verification | The standard OSINT verification toolkit. Reverse image search, metadata, keyframe extraction. Esper Machine provides the forensic depth InVID lacks. |
| Bellingcat toolkit | Collection of OSINT tools including SunCalc, Google Earth | Inspiration for the geolocation tools. Our advantage is integration into a single workbench. |
| SunCalc.org | Solar position calculator | Direct integration candidate for sun angle analysis. Open source (BSD-2-Clause). |
| Open-Meteo | Historical weather API | Free, no-key-required weather data source for temporal cross-referencing. |
| Ghiro | Open-source image forensics framework | Python-based image forensics with EXIF analysis, ELA, hash comparison. Our forensic pipeline already exceeds Ghiro's capabilities. |

## Appendix B: Scenario Walkthrough — Conflict Zone Image

An image is shared on social media claiming to show the aftermath of an airstrike on a hospital in City X on 15 March 2026.

**Step 1 — Automated pipeline** (existing): Jura Trace runs the 21-detector pipeline. Verdict: "Authentic" (high confidence). The image is a real photograph.

**Step 2 — Esper Mode: Metadata Intelligence**:
- EXIF shows camera model: Canon EOS R5. Date: 12 September 2023. Location: GPS coordinates point to a different city.
- Investigator opens GPS mapping → coordinates are in City Y, not City X.
- Thumbnail comparison → thumbnail matches full image (no post-capture crop).
- Edit history → no XMP history (image has not been through editing software).
- **Finding**: metadata contradicts claim (different date, different location).

**Step 3 — Esper Mode: Geolocation Verification**:
- OCR detects Arabic script on a visible shop sign → language identified as Arabic.
- Sun angle calculator → analyst marks two shadows. Solar azimuth computed for claimed date/location does not match. Computed for the metadata date/location it DOES match.
- Vegetation estimation → visible trees are in full leaf, consistent with September, inconsistent with March in the claimed latitude.
- **Finding**: three independent geolocation signals contradict the claim.

**Step 4 — Esper Mode: Reverse Image Search**:
- Analyst clicks "Search TinEye" → image was first indexed in October 2023 in a news report about City Y.
- **Finding**: image predates the claimed event by 2.5 years.

**Step 5 — Case Export**: Analyst exports a case ZIP containing:
- Original image
- All overlay images (ELA, noise, shadow directions)
- GPS map screenshot
- Sun angle calculation with parameters
- OCR results with bounding boxes
- TinEye search result screenshot (added manually)
- Investigation notes with timestamped findings
- Chain of custody log showing every tool invocation

**Conclusion**: The image is authentic but decontextualised — a real photograph of a real event, repackaged with a false caption. The Esper Machine provided the tools to prove this. The standard deepfake pipeline, correctly, had nothing to say.

---

*"Enhance. Track right. Stop. Enhance 34 to 46. Pull back. Stop. Enhance 15 to 23. Give me a hard copy right there."*

*We cannot zoom around corners. But we can see what the image truly contains — and what the caption claims it contains — and tell the difference.*
