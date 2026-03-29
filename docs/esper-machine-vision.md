# Esper Machine Vision — Advanced Investigation Tools

> *"Enhance 224 to 176. Give me a hard copy right there."*
> — Rick Deckard, Blade Runner (1982)

**Document type**: Technical vision and design specification
**Status**: Draft
**Author**: Jura Labs engineering
**Date**: 28 March 2026
**Version**: 0.1.0

---

## 1. Introduction

In Ridley Scott's *Blade Runner*, the Esper machine lets Deckard navigate into a photograph — zooming, panning, rotating around corners, extracting details invisible to the naked eye. The scene is science fiction, but the investigative impulse is real. Journalists, museum archivists, OSINT researchers, and forensic analysts need tools that let them *interrogate* an image — not just look at it, but ask it questions and receive structured, evidenced answers.

Jura Trace already provides a strong foundation: 16 forensic detectors, C2PA credential verification, EXIF anomaly analysis, perceptual hashing, and a visual inspection toolbar. This document defines the next generation of investigation tools that will transform Jura Trace from a verification tool into a comprehensive forensic investigation platform.

### 1.1 Design Principles

All tools in this document adhere to Jura Trace's core principles:

1. **Local-first**: Every tool runs on-device. No cloud calls for analysis. External data lookups (weather APIs, map tiles) are explicitly opt-in and clearly disclosed to the user.
2. **Layered architecture**: Lightweight computation in Rust, ML-heavy work in the Python sidecar, LLM reasoning via Ollama. Each tool specifies its layer.
3. **Graceful degradation**: If the sidecar is offline or a model is unavailable, the tool either falls back to a simpler method or clearly states it is unavailable.
4. **Multi-signal philosophy**: No single tool produces a verdict. Tools produce *signals* that feed into the existing trust scoring pipeline and the analyst's professional judgement.
5. **Honest uncertainty**: Every tool reports confidence bounds. "Inconclusive" is always a valid output.
6. **British spelling** in all user-facing text; American spelling in code identifiers.

### 1.2 Architecture Context

```
┌─────────────────────────────────────────────────────────────┐
│                    INVESTIGATION TOOLS                       │
│                                                             │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────────┐    │
│  │  Rust    │  │  Python      │  │  Ollama            │    │
│  │  Core    │  │  Sidecar     │  │  (Optional)        │    │
│  │         │  │  :8200       │  │  :11434            │    │
│  │ EXIF     │  │ Shadow geo   │  │ LLaVA scene desc   │    │
│  │ GPS calc │  │ FFT visual   │  │ Landmark ID        │    │
│  │ Hash DB  │  │ Noise maps   │  │ Text analysis      │    │
│  │ Sensor   │  │ GAN finger   │  │ Context reasoning  │    │
│  │ Thumb    │  │ Face mesh    │  │                    │    │
│  └──────────┘  └──────────────┘  └────────────────────┘    │
│                         │                                   │
│              ┌──────────▼──────────┐                        │
│              │  Trust Score Engine │                        │
│              │  (existing pipeline)│                        │
│              └─────────────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 Existing Infrastructure

The following capabilities are already built and form the foundation for the tools described in this document:

| Capability | Location | Relevance |
|---|---|---|
| Perceptual hashing (aHash, dHash, pHash) | `src-tauri/src/fingerprint.rs` | Hash DB search, lineage graphs |
| EXIF extraction + anomaly detection | `src-tauri/src/metadata.rs`, `exif_anomaly.rs` | Thumbnail check, metadata report |
| C2PA verification + AI declaration detection | `src-tauri/src/c2pa.rs` | AI watermark panel |
| Shadow consistency (gradient light direction) | `sidecar/app/services/shadow_consistency.py` | Sun angle, temporal analysis |
| 21-signal deepfake ensemble + GBM classifier | `sidecar/app/services/deepfake.py` | Frequency viz, noise viz, GAN fingerprints |
| Frequency features (FFT, DCT, spectral decay) | `sidecar/app/services/deepfake.py` | Frequency domain visualisation |
| Noise residual extraction (wavelet denoising) | `sidecar/app/services/deepfake.py` | Noise pattern viz, PRNU |
| JPEG ghost analysis | `sidecar/app/services/jpeg_ghost.py` | JPEG grid visualisation |
| Splice boundary detection | `sidecar/app/services/splice_boundary.py` | Region forensics |
| CLIP ViT-B/32 zero-shot classification | `sidecar/app/services/clip_detector.py` | Diffusion artefact detection |
| Invisible watermark extraction | `sidecar/app/services/watermark.py` | AI watermark panel |
| LLaVA / Qwen2.5 via Ollama | Existing optional integration | Scene description, text analysis |
| Visual inspection toolbar | `ui/src/routes/verify/+page.svelte` | Channel separation, histogram EQ |
| SQLite fingerprint storage | `src-tauri/src/db.rs` | Hash search, lineage |
| Audit log with hash chain | `src-tauri/src/db.rs` | Investigation timeline |

---

## 2. Geolocation Analysis

Tools that answer: *"Where was this image taken? Is the claimed location consistent with what the image shows?"*

### 2.1 Sun Angle and Shadow Geometry

| Field | Detail |
|---|---|
| **What it detects** | Calculates the sun's azimuth and elevation from shadow angles and lengths in the image. When combined with a claimed date and GPS coordinate, verifies whether the shadow geometry is physically consistent with that place and time. |
| **Question answered** | "Are the shadows in this image consistent with the claimed location and date?" |
| **Layer** | Python sidecar (shadow geometry extraction) + Rust (solar position calculation via astronomical formulae) |
| **Algorithm** | 1. Detect vertical objects and their cast shadows using edge detection and Hough line transform. 2. Compute shadow azimuth angle from the shadow's direction relative to the vertical object. 3. Estimate solar elevation from the ratio of object height to shadow length (requires at least one known-height reference or user annotation). 4. Compute expected solar position using the NOAA solar position algorithm (latitude, longitude, date, time). 5. Compare observed vs expected azimuth and elevation; report angular deviation. |
| **Implementation** | Solar position calculator in Rust (`sun_position.rs`) — the NOAA algorithm is pure trigonometry, no dependencies. Shadow extraction in Python sidecar (`geolocation.py`) using OpenCV edge detection, Hough transform, and interactive annotation overlay for user to mark object base/tip and shadow tip. |
| **Complexity** | Medium — solar calculation is straightforward; shadow detection in natural images is noisy and often requires user guidance. |
| **Dependencies** | OpenCV (existing). No new Python packages. Rust: no new crates (trigonometry only). |
| **Limitations** | Requires visible shadows with identifiable casting objects. Fails on overcast days, indoor images, or images where shadow-casting objects are not clearly delineated. Accuracy depends heavily on correct identification of shadow endpoints. |
| **Priority** | **P1** — high value for OSINT journalists, directly answers "when and where" questions. Shadow consistency detector already exists in the sidecar and provides foundational gradient analysis. |

### 2.2 Vegetation Analysis

| Field | Detail |
|---|---|
| **What it detects** | Identifies vegetation types, foliage state, and seasonal indicators to constrain geographic region and time of year. |
| **Question answered** | "Is the vegetation consistent with the claimed location and season?" |
| **Layer** | Ollama (LLaVA scene description + prompted vegetation analysis) with optional CLIP embedding for biome classification |
| **Algorithm** | 1. Extract green-channel-dominant regions via colour segmentation in LAB space. 2. Classify vegetation density (barren / sparse / moderate / dense) from NDVI-like index using visible-spectrum approximation. 3. Assess foliage state: deciduous vs evergreen, leaf-on vs leaf-off, flowering state. 4. Prompt LLaVA with structured query: "Describe the vegetation visible in this image. Identify species if possible. What season does the foliage suggest? What climate zone?" 5. Cross-reference vegetation indicators with claimed GPS coordinates using a local biome lookup table (Koeppen-Geiger climate classification, ~50 KB CSV). |
| **Implementation** | Colour segmentation in Python sidecar. LLaVA prompting via existing Ollama integration. Biome lookup table as static data in Rust or Python. |
| **Complexity** | Medium — colour segmentation is straightforward; species identification is aspirational and LLM-dependent. |
| **Dependencies** | OpenCV (existing), Ollama + LLaVA (existing optional). New static data: Koeppen-Geiger grid CSV (~50 KB). |
| **Limitations** | Visible-spectrum vegetation indices are crude approximations of true NDVI (which requires near-infrared). Species identification via LLaVA is unreliable — treat as suggestive, not authoritative. Urban environments with landscaped non-native plants will confuse geographic inference. |
| **Priority** | **P3** — interesting but low precision. Vegetation analysis is a supporting signal, rarely decisive on its own. |

### 2.3 Sky and Atmosphere Analysis

| Field | Detail |
|---|---|
| **What it detects** | Analyses sky colour gradient, cloud patterns, atmospheric haze, and visibility to infer altitude, climate zone, and weather conditions. |
| **Question answered** | "Are the atmospheric conditions consistent with the claimed location and weather at the claimed time?" |
| **Layer** | Python sidecar (image analysis) + optional external weather API lookup (user opt-in) |
| **Algorithm** | 1. Segment sky region using a simple classifier (top-of-image blue hue detection + semantic segmentation via CLIP if available). 2. Measure sky colour gradient from horizon to zenith — Rayleigh scattering produces a predictable blue gradient; deviations suggest altitude, pollution, or time of day. 3. Estimate visibility/haze level from contrast between foreground and distant objects. 4. Classify cloud type using LBP texture features on the sky region (cirrus / cumulus / stratus / overcast / clear). 5. Optionally cross-reference with historical weather data API (OpenWeatherMap, Visual Crossing) if user provides date, time, and location and explicitly opts in to the network request. |
| **Implementation** | Sky segmentation and atmospheric analysis in Python sidecar (`sky_analysis.py`). Weather API integration as a separate opt-in module with clear network disclosure in the UI. |
| **Complexity** | Medium (image analysis) / Low (weather API lookup) |
| **Dependencies** | OpenCV, scikit-image (existing). Optional: `httpx` (existing) for weather API. |
| **Limitations** | Sky segmentation fails on indoor images, images without visible sky, and heavily cropped images. Cloud classification from a single image is imprecise. Weather API requires network access, breaking local-first for that specific feature. |
| **Priority** | **P2** — weather cross-referencing is powerful for investigative journalism. The atmospheric analysis portion alone (without API) is P3. |

### 2.4 Road Markings, Signage, and Architecture

| Field | Detail |
|---|---|
| **What it detects** | Identifies culturally and geographically specific features: road marking styles (white vs yellow centre lines, dashed patterns), traffic sign conventions, driving side, architectural styles, power line configurations, number plate formats. |
| **Question answered** | "What country or region do the built-environment features suggest?" |
| **Layer** | Ollama (LLaVA structured prompting for feature identification) + Python sidecar (OCR for sign text, number plate detection) |
| **Algorithm** | 1. Detect text regions using EAST text detector or Tesseract OCR. 2. Identify script/language from detected text. 3. Prompt LLaVA: "Examine the built environment in this image. Identify: road marking style and colour, traffic sign shapes and colours, driving side (left/right), architectural style, any visible text or signage. What country or region do these features suggest?" 4. Detect number plate regions using a lightweight YOLO model or Haar cascade, classify plate format by aspect ratio and character layout. 5. Cross-reference features against a rule-based geolocation knowledge base (e.g., yellow centre lines + white edge lines = North America; white dashed centre lines = most of Europe). |
| **Implementation** | OCR via existing Tesseract or macOS Vision framework (already noted in CLAUDE.md for Tier 2). LLaVA prompting via Ollama. Rule-based knowledge base as JSON in sidecar. |
| **Complexity** | High — robust sign and road marking detection in unconstrained images is a hard computer vision problem. LLaVA provides a reasonable starting point but with limited reliability. |
| **Dependencies** | Ollama + LLaVA (existing optional). Optional: `pytesseract` or macOS Vision OCR. Optional: lightweight YOLO model (~6 MB) for plate detection. |
| **Limitations** | Highly dependent on image content — many images contain no built-environment features. LLaVA's geographic knowledge is broad but shallow. OCR quality varies dramatically with image resolution and text angle. Number plate detection requires sufficient resolution. |
| **Priority** | **P2** — high value for OSINT and journalism workflows, but dependent on Ollama being available. |

### 2.5 GPS Coordinate Mapping and Verification

| Field | Detail |
|---|---|
| **What it detects** | Visualises EXIF GPS coordinates on a map and cross-references with other image content for consistency. |
| **Question answered** | "Does the GPS metadata match what the image shows?" |
| **Layer** | Rust (GPS extraction — already implemented in `metadata.rs`) + SvelteKit frontend (map rendering) |
| **Algorithm** | 1. Extract GPS coordinates from EXIF (existing). 2. Render location on an embedded map component. 3. Display satellite/terrain view at the GPS coordinate. 4. Show GPS accuracy/precision metadata (HDOP, number of satellites if available). 5. Flag impossible GPS values (e.g., coordinates in the ocean when image shows land, or vice versa). 6. Calculate distance between GPS coordinate and any user-provided claimed location. |
| **Implementation** | Map rendering in SvelteKit using Leaflet.js with OpenStreetMap tiles (no API key required, tiles are freely available). GPS validation logic in Rust (`gps_validation.rs`). Offline fallback: display raw coordinates with UTM conversion, no map tiles. |
| **Complexity** | Low (coordinate display) / Medium (map integration with offline fallback) |
| **Dependencies** | New frontend: `leaflet` (~40 KB). Map tiles require network access — must be clearly disclosed as opt-in. |
| **Limitations** | Many images have no GPS data. GPS data is trivially spoofable — presence of GPS coordinates is not evidence of authenticity. Map tile loading requires network access. |
| **Priority** | **P1** — GPS data is already extracted; visualisation is the missing piece. Critical for journalist and OSINT workflows. |

---

## 3. Temporal Analysis

Tools that answer: *"When was this image actually taken? Is the claimed date/time consistent with what the image shows?"*

### 3.1 Shadow-Based Time-of-Day Estimation

| Field | Detail |
|---|---|
| **What it detects** | Estimates the time of day from shadow length and direction, given a known or claimed location and date. |
| **Question answered** | "Is the claimed time of day consistent with the shadows in this image?" |
| **Layer** | Python sidecar (shadow measurement) + Rust (solar position inverse calculation) |
| **Algorithm** | Uses the same shadow extraction pipeline as Section 2.1, but inverts the problem: given a known GPS coordinate and date, computes the time of day that would produce the observed shadow azimuth and elevation. Returns a time range with confidence interval. |
| **Implementation** | Shared infrastructure with 2.1. The Rust solar position module provides both forward (location + time -> sun position) and inverse (location + sun position -> time) calculations. |
| **Complexity** | Medium — the inverse solar calculation has multiple solutions per day (morning and afternoon produce symmetric shadow angles). The system reports both candidates. |
| **Dependencies** | Same as 2.1. No additional dependencies. |
| **Limitations** | Same shadow detection limitations as 2.1. The inverse problem is under-determined without additional constraints — the system can narrow to a time window but rarely to a precise minute. Requires accurate GPS coordinates. |
| **Priority** | **P1** — directly extends 2.1 with minimal additional effort. |

### 3.2 Seasonal Indicators

| Field | Detail |
|---|---|
| **What it detects** | Analyses foliage state, snow coverage, grass colour, flower blooming, and sun angle to estimate the season or month range. |
| **Question answered** | "Is the claimed date consistent with the seasonal indicators in this image?" |
| **Layer** | Python sidecar (colour analysis) + Ollama (LLaVA scene description) |
| **Algorithm** | 1. Compute vegetation greenness index from LAB colour space (shared with 2.2). 2. Detect snow coverage from high-luminance, low-saturation regions. 3. Classify grass state (dormant brown / growing green / lush). 4. Estimate sun elevation from shadow analysis (shared with 2.1). 5. Prompt LLaVA: "What season does this image appear to show? List all seasonal indicators you can identify." 6. Cross-reference sun elevation with claimed latitude to determine plausible month range. |
| **Implementation** | Colour analysis in Python sidecar. LLaVA prompting via existing Ollama integration. Sun elevation seasonal mapping in Rust. |
| **Complexity** | Low-Medium — colour analysis is straightforward; seasonal inference requires combining multiple weak signals. |
| **Dependencies** | OpenCV (existing), Ollama + LLaVA (existing optional). |
| **Limitations** | Tropical and equatorial regions show minimal seasonal variation. Indoor images provide no seasonal signals. Landscaped environments (irrigated lawns, heated greenhouses) confound analysis. Southern hemisphere seasons are inverted — the system must account for latitude. |
| **Priority** | **P2** — useful supporting signal for date verification. |

### 3.3 Weather Verification

| Field | Detail |
|---|---|
| **What it detects** | Cross-references visible weather conditions (rain, sun, overcast, fog, snow) with historical weather records for the claimed location and date. |
| **Question answered** | "Did the weather at the claimed location and date match what the image shows?" |
| **Layer** | Python sidecar (weather condition classification from image) + optional external API (historical weather lookup — user opt-in) |
| **Algorithm** | 1. Classify visible weather conditions from the image: clear/sunny, overcast, rain (wet surfaces, visible precipitation, umbrellas), fog/mist (low contrast, desaturated), snow (white coverage). Use a combination of sky segmentation (Section 2.3), surface reflectance analysis, and LLaVA scene description. 2. Query historical weather API (Visual Crossing or Open-Meteo, both have free tiers) with GPS coordinates and date. 3. Compare observed vs recorded conditions. Report match, mismatch, or inconclusive. |
| **Implementation** | Weather classification in Python sidecar (`weather_classifier.py`). API client as a separate opt-in module with explicit network disclosure. |
| **Complexity** | Low (image classification) / Low (API integration) |
| **Dependencies** | OpenCV (existing), `httpx` (existing). Optional: Open-Meteo API (free, no key required). |
| **Limitations** | Requires network access for historical weather data — breaks local-first for this specific feature. Weather classification from a single image is inherently imprecise. Microclimates mean that weather at the exact GPS coordinate may differ from the nearest weather station. The feature should always present its findings as "consistent/inconsistent with recorded weather" rather than making definitive claims. |
| **Priority** | **P2** — high value for investigative journalism. The Bellingcat, BBC Africa Eye, and New York Times Visual Investigations teams all use weather cross-referencing routinely. |

### 3.4 Celestial Verification

| Field | Detail |
|---|---|
| **What it detects** | Verifies moon phase, star positions, and celestial body locations against astronomical ephemeris data for a claimed date, time, and location. |
| **Question answered** | "Is the moon phase / star field consistent with the claimed date, time, and location?" |
| **Layer** | Rust (ephemeris calculations) + Python sidecar (moon/star detection in image) |
| **Algorithm** | 1. Detect the moon in the image using template matching and circularity analysis on high-luminance regions. 2. Estimate moon phase from the illuminated fraction and terminator orientation. 3. Compute expected moon phase, altitude, and azimuth for the claimed date/time/location using astronomical algorithms (Meeus, *Astronomical Algorithms*). 4. Compare observed vs expected phase. For night sky images: detect bright stars using blob detection, attempt constellation matching against a star catalogue. |
| **Implementation** | Moon phase computation in Rust (pure trigonometry, Meeus algorithm — no external dependencies). Moon detection in Python sidecar using OpenCV blob detection and circularity filtering. Star field matching is a stretch goal requiring a local star catalogue (~2 MB). |
| **Complexity** | Medium (moon phase) / High (star field matching) |
| **Dependencies** | OpenCV (existing). New Rust: no new crates (Meeus algorithms are pure maths). Optional: Hipparcos star catalogue subset (~2 MB CSV). |
| **Limitations** | Moon detection requires a clearly visible, non-occluded moon with sufficient resolution. Star field matching requires long-exposure or night photography with minimal light pollution — very rare in typical verification scenarios. Moon phase has only ~29.5-day periodicity, so it can only narrow the date to a ~2-day window within each lunar cycle. |
| **Priority** | **P3** — niche but impressive. Moon phase verification is occasionally decisive in high-profile investigations. Star field matching is primarily of academic interest. |

---

## 4. Enhanced Visual Inspection

Tools that answer: *"What do I see when I look at this image in ways the human eye cannot?"*

These tools extend the existing visual inspection toolbar (greyscale, invert, high contrast, saturate, edge detect, brightness/contrast sliders).

### 4.1 Colour Channel Separation

| Field | Detail |
|---|---|
| **What it detects** | Displays individual R, G, B channels and composite views (R-G, R-B, G-B difference channels). Reveals manipulation artefacts that are visible in only one channel — common in crude compositing, colour manipulation, and some GAN outputs. |
| **Question answered** | "Are there channel-specific artefacts that indicate manipulation?" |
| **Layer** | SvelteKit frontend (CSS/Canvas filters for real-time toggle) + Python sidecar (pre-rendered channel images for detailed analysis) |
| **Algorithm** | 1. Split image into R, G, B channels — display each as a greyscale image. 2. Compute difference channels: \|R-G\|, \|R-B\|, \|G-B\| — these highlight regions where colour channels are inconsistent. 3. Display in HSV and LAB colour spaces — LAB is particularly useful because the L (luminance) channel separates brightness from colour, and the A/B channels reveal colour manipulation that is invisible in RGB. 4. Apply histogram equalisation per channel to enhance subtle differences. |
| **Implementation** | Real-time channel toggle in the frontend using HTML5 Canvas `getImageData`/`putImageData` for instant feedback. Pre-rendered detailed channel analysis images from the Python sidecar for the "investigate" view. |
| **Complexity** | Low — pure pixel manipulation, no ML required. |
| **Dependencies** | None new. Canvas API in frontend, OpenCV/NumPy in sidecar (existing). |
| **Limitations** | Interpreting channel separation requires analyst expertise. Many legitimate images show channel differences (e.g., chromatic aberration, white balance correction). The tool produces visualisations, not verdicts. |
| **Priority** | **P1** — low cost, high value for trained analysts. Standard tool in image forensics workflows. Should be added to the existing visual inspection toolbar. |

### 4.2 Frequency Domain Visualisation

| Field | Detail |
|---|---|
| **What it detects** | Displays the 2D FFT magnitude spectrum, DCT coefficient distribution, and wavelet decomposition of the image. Reveals periodic artefacts (JPEG grid, GAN upsampling checkerboard), compression history, and frequency-domain inconsistencies between regions. |
| **Question answered** | "Are there frequency-domain artefacts that indicate processing, compression, or synthetic generation?" |
| **Layer** | Python sidecar (FFT/DCT/wavelet computation and visualisation) |
| **Algorithm** | 1. Compute 2D FFT of the greyscale image, display log-magnitude spectrum with DC at centre. 2. Annotate known artefact frequencies: JPEG 8x8 block grid appears as bright dots at multiples of 1/8 pixel frequency; GAN upsampling checkerboard appears as peaks at specific spatial frequencies determined by the generator's upsampling stride. 3. Compute DCT and display coefficient magnitude heatmap (8x8 block-averaged for JPEG analysis). 4. Perform multi-level Haar wavelet decomposition, display LH/HL/HH subbands at each level. 5. Provide a region-selection tool: user draws a rectangle, and the frequency analysis is computed for that region alone — enabling comparison between suspected and reference regions. |
| **Implementation** | Python sidecar (`frequency_visualisation.py`). Uses `scipy.fft.fft2` (already imported in `deepfake.py`), `pywt` for wavelet decomposition. Returns rendered visualisation images as base64. |
| **Complexity** | Low-Medium — the mathematics is standard; the value is in the interactive visualisation and annotation. |
| **Dependencies** | scipy (existing), numpy (existing). New: `PyWavelets` (`pywt`) for wavelet decomposition (~1 MB). |
| **Limitations** | Frequency domain visualisation requires expertise to interpret. Many legitimate processing operations (sharpening, denoising, resizing) alter the frequency spectrum. The tool should provide interpretive guidance alongside the visualisation but should not auto-classify. |
| **Priority** | **P1** — essential for forensic analysts. The deepfake detector already computes frequency features internally; this tool exposes them visually. |

### 4.3 JPEG Quantisation Grid Visualisation

| Field | Detail |
|---|---|
| **What it detects** | Visualises the JPEG 8x8 block grid alignment and quantisation table artefacts. Regions that were spliced from a differently-compressed JPEG source show misaligned block grids or different quantisation artefact levels. This is the visual counterpart to the existing JPEG ghost detector. |
| **Question answered** | "Are there regions with misaligned JPEG compression grids, suggesting compositing?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Compute block artefact strength at each pixel by measuring the discontinuity at 8x8 grid boundaries. 2. Render a heatmap where bright = strong block artefacts, dark = weak. Uniform brightness suggests single compression; bright/dark patches suggest double compression with different Q factors or grid misalignment. 3. Detect grid phase: for each 64x64 region, determine the 8x8 grid phase that minimises boundary discontinuity. Regions with different grid phases are highlighted. 4. Extract and display the JPEG quantisation table(s) from the file header. Compare against known camera and software Q-table databases. |
| **Implementation** | Python sidecar (`jpeg_grid.py`). Extends existing `jpeg_ghost.py` with explicit grid visualisation. Q-table extraction via Pillow's `quantization` attribute on JPEG images. |
| **Complexity** | Medium — grid phase detection across regions requires careful implementation. |
| **Dependencies** | Pillow, OpenCV, NumPy (all existing). |
| **Limitations** | Only applicable to JPEG images. Images that have been re-saved as PNG after JPEG manipulation lose grid artefacts. Multiple rounds of JPEG recompression at the same quality factor can equalise grid artefacts, masking manipulation. Images from social media platforms are typically recompressed, which introduces uniform artefacts. |
| **Priority** | **P1** — directly complements existing JPEG ghost and splice boundary detectors. |

### 4.4 Noise Pattern Visualisation

| Field | Detail |
|---|---|
| **What it detects** | Renders the image's noise residual as a visible map, enabling analysts to see noise level and texture differences between regions. Spliced regions from different cameras or AI generators have different noise characteristics. |
| **Question answered** | "Are there regions with inconsistent noise patterns?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Extract noise residual using wavelet denoising (already implemented in `deepfake.py` via `skimage.restoration.denoise_wavelet`). 2. Compute local noise standard deviation in a sliding window (e.g., 32x32 patches). 3. Render as a heatmap: uniform noise suggests single source; patchy noise suggests compositing or local processing. 4. Compute noise autocorrelation map — camera noise has short-range correlation; AI-generated noise has unnaturally smooth or periodic correlation. 5. Provide side-by-side comparison: original image | denoised image | noise residual | noise variance map. |
| **Implementation** | Python sidecar (`noise_visualisation.py`). Builds on existing noise extraction in `deepfake.py`. The noise autocorrelation map is a new visualisation of the existing noise decorrelation rate signal. |
| **Complexity** | Low — the computation already exists; this tool wraps it in a visual presentation. |
| **Dependencies** | scikit-image, scipy, OpenCV (all existing). |
| **Limitations** | Noise patterns are affected by legitimate processing (sharpening, denoising, HDR tonemapping). Regions with different detail levels naturally show different noise levels (flat sky vs textured foliage). The tool must display the variance map alongside the original to help analysts distinguish content-dependent noise variation from forensically significant inconsistency. |
| **Priority** | **P1** — almost zero incremental cost since the computation already exists. |

### 4.5 Luminance Gradient Mapping

| Field | Detail |
|---|---|
| **What it detects** | Visualises the local luminance gradient magnitude and direction across the image. Reveals lighting inconsistencies, retouching artefacts (dodging/burning), and compositing seams where lighting direction changes abruptly. |
| **Question answered** | "Is the lighting direction consistent across all regions?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Convert to greyscale, compute Sobel gradients (Gx, Gy). 2. Compute gradient magnitude sqrt(Gx^2 + Gy^2) and direction atan2(Gy, Gx). 3. Render as a direction-encoded colour map (hue = direction, saturation = magnitude). 4. Segment into regions and compute dominant gradient direction per region (extends existing shadow consistency analysis). 5. Flag regions where dominant gradient direction deviates by more than a configurable threshold from the global mode. |
| **Implementation** | Python sidecar. Shared infrastructure with `shadow_consistency.py` — this tool adds the visualisation layer. |
| **Complexity** | Low — gradient computation is trivial; the value is in the visualisation. |
| **Dependencies** | OpenCV (existing). |
| **Limitations** | Multiple light sources (e.g., indoor scenes with windows and artificial lighting) produce legitimately inconsistent gradient directions. The tool must not auto-flag multi-source lighting as suspicious. |
| **Priority** | **P2** — enhances the existing shadow consistency detector with a visual exploration interface. |

### 4.6 Per-Channel Histogram Equalisation

| Field | Detail |
|---|---|
| **What it detects** | Applies CLAHE (Contrast-Limited Adaptive Histogram Equalisation) per colour channel and globally, revealing detail in shadows, highlights, and low-contrast regions. Exposes hidden text, watermarks, and steganographic content. |
| **Question answered** | "Is there hidden detail in the shadows, highlights, or low-contrast regions?" |
| **Layer** | Python sidecar (pre-rendered) + SvelteKit frontend (real-time CLAHE via Canvas for preview) |
| **Algorithm** | 1. Apply CLAHE with configurable clip limit (default 2.0, range 0.5-8.0) and tile grid size (default 8x8). 2. Apply per-channel: CLAHE on R, G, B independently to reveal channel-specific hidden detail. 3. Apply on L channel in LAB space (preserving colour) for general shadow/highlight recovery. 4. Render level-adjusted difference: `CLAHE(image) - image` to isolate the recovered detail. |
| **Implementation** | OpenCV `cv2.createCLAHE()` in Python sidecar. Interactive clip limit slider in frontend. |
| **Complexity** | Low |
| **Dependencies** | OpenCV (existing). |
| **Limitations** | CLAHE amplifies noise in flat regions. Very high clip limits produce unnaturally harsh images. The tool should default to a conservative clip limit and let the analyst adjust. |
| **Priority** | **P1** — trivial to implement, immediately useful for recovering detail in poorly exposed images. |

---

## 5. Metadata Intelligence

Tools that answer: *"What does the file itself — beyond the visible pixels — tell us about its origin and history?"*

### 5.1 Camera Sensor Fingerprint (PRNU)

| Field | Detail |
|---|---|
| **What it detects** | Every digital camera sensor has a unique noise pattern caused by manufacturing imperfections in the photodiodes — the Photo-Response Non-Uniformity (PRNU). By extracting this fingerprint, we can: (a) verify that two images came from the same physical camera, (b) detect regions that were spliced from a different camera, (c) link an image to a known device. |
| **Question answered** | "Was this image taken by the same camera as these other images? Are there regions from a different camera?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Extract noise residual from the image using wavelet denoising (Lukas, Fridrich, Goljan, 2006). 2. Estimate camera PRNU fingerprint by averaging noise residuals from multiple images known to be from the same camera (requires 30-50 reference images for a reliable fingerprint). 3. Compute normalised cross-correlation (NCC) between the image noise residual and the camera fingerprint. 4. Threshold NCC against a per-pixel decision boundary (typically NCC > 60 for match). 5. For splice detection: compute NCC in sliding windows across the image; regions with significantly lower correlation may be from a different camera. |
| **Implementation** | Python sidecar (`sensor_fingerprint.py`). Requires a local SQLite database of camera fingerprints (separate from the main Jura Trace database, stored in the user's data directory). Fingerprints are built from the user's own reference images — no external database required. |
| **Complexity** | High — PRNU estimation requires careful noise extraction, and the fingerprint database must be built per-camera. The underlying science is well-established but the implementation has many edge cases (JPEG recompression degrades PRNU, in-camera noise reduction can weaken the signal, resizing destroys it). |
| **Dependencies** | scikit-image, scipy, numpy (all existing). New: `pywt` for wavelet denoising (shared with 4.2). |
| **Limitations** | PRNU is destroyed by resizing, heavy JPEG compression, and strong denoising. Social media images (which are always recompressed and resized) have severely degraded PRNU. Requires reference images from the same camera to build a fingerprint — cold start problem. Cannot attribute an image to a specific camera without prior reference data. |
| **Priority** | **P2** — high forensic value for institutions that control the image pipeline (museums, archives, newsrooms). Less useful for ad-hoc verification of internet images. |

### 5.2 Lens Distortion Pattern Matching

| Field | Detail |
|---|---|
| **What it detects** | Analyses radial and tangential lens distortion patterns. Camera lenses produce characteristic barrel or pincushion distortion, and each lens model has a specific distortion profile. Spliced images may contain regions with inconsistent distortion. |
| **Question answered** | "Do all regions of this image show consistent lens distortion?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Detect straight lines in the image using LSD (Line Segment Detector). 2. Measure curvature of detected lines — in an undistorted image, lines that should be straight are straight; lens distortion curves them. 3. Fit a radial distortion model (Brown-Conrady, 2 radial + 2 tangential parameters) to the detected line curvatures. 4. Compare fitted distortion parameters across image quadrants — a composited image may show different distortion in different regions. 5. Optionally match against a database of known lens profiles (lensfun database, ~200 KB). |
| **Implementation** | Python sidecar (`lens_analysis.py`). Line detection via OpenCV LSD. Distortion model fitting via least-squares optimisation (scipy.optimize). |
| **Complexity** | High — distortion fitting from detected lines is sensitive to misdetected lines and requires robust estimation (RANSAC). |
| **Dependencies** | OpenCV, scipy (existing). Optional: lensfun database subset (~200 KB JSON). |
| **Limitations** | Lens correction is routinely applied by camera firmware and RAW processing software, so many images have no measurable distortion. Wide-angle images show the strongest distortion and are most amenable to this analysis. Images with few straight lines (natural landscapes) provide insufficient data. |
| **Priority** | **P3** — high complexity, niche applicability. |

### 5.3 Device-Specific JPEG Compression Signatures

| Field | Detail |
|---|---|
| **What it detects** | Different cameras, phones, and software encode JPEG files with characteristic quantisation tables, Huffman tables, and subsampling patterns. This signature can identify the encoding software even when EXIF data has been stripped. |
| **Question answered** | "What camera or software likely produced this JPEG?" |
| **Layer** | Rust (JPEG header parsing) + Python sidecar (Q-table database matching) |
| **Algorithm** | 1. Parse JPEG file structure: extract quantisation tables (DQT markers), Huffman tables (DHT markers), colour subsampling (SOF marker). 2. Compute a fingerprint from the Q-table values. 3. Match against a database of known Q-table fingerprints (~500 entries covering major cameras, phones, and software). 4. Report: "This Q-table matches cameras from [manufacturer] using [processing engine]" or "This Q-table matches [software name] at quality [N]." 5. Flag inconsistency: if EXIF says "Canon EOS R5" but the Q-table matches "Adobe Photoshop," the image has been re-saved. |
| **Implementation** | JPEG header parsing in Rust (`jpeg_signature.rs`) — JPEG marker parsing is fast and well-suited to Rust. Q-table database as a JSON file (~50 KB). Matching logic can be in either Rust or Python sidecar. |
| **Complexity** | Medium — JPEG parsing is well-defined; building the Q-table database requires gathering reference images from many devices. |
| **Dependencies** | No new crates or Python packages. Q-table database needs to be assembled (community resources exist: Kee, Farid, 2011; the `jpegquality` project). |
| **Limitations** | JPEG re-saving overwrites the original Q-table. Social media platforms recompress with their own tables (Facebook, Instagram, Twitter each have characteristic Q-tables — which is itself useful information). |
| **Priority** | **P1** — low implementation cost, high forensic value. Already partially flagged in the EXIF anomaly detector. The Q-table extraction extends naturally from Pillow's existing `quantization` attribute. |

### 5.4 Thumbnail Consistency Checking

| Field | Detail |
|---|---|
| **What it detects** | Many cameras embed a thumbnail image in the EXIF data. If the main image has been edited but the thumbnail was not updated, the thumbnail shows the original unedited image. This is one of the oldest and most reliable manipulation indicators. |
| **Question answered** | "Does the EXIF thumbnail match the main image, or does it show an earlier unedited version?" |
| **Layer** | Rust (EXIF thumbnail extraction) + Python sidecar (perceptual comparison) |
| **Algorithm** | 1. Extract the embedded EXIF thumbnail (if present). 2. Generate a thumbnail of the main image at the same dimensions. 3. Compare using perceptual hash (pHash — already implemented in `fingerprint.rs`). 4. If Hamming distance > 5, flag as inconsistent and display both thumbnails side-by-side. 5. Additionally compare histograms, average colour, and aspect ratio for extra signals. |
| **Implementation** | Thumbnail extraction in Rust via the `kamadak-exif` crate (already used in `metadata.rs`). pHash comparison using existing `fingerprint.rs`. Side-by-side display in SvelteKit frontend. |
| **Complexity** | Low — all components already exist. |
| **Dependencies** | None new. All components already implemented. |
| **Limitations** | Not all images have EXIF thumbnails. Modern image editors (Photoshop, GIMP, Lightroom) update the thumbnail on save, so this check only catches careless editing or specialised metadata-preservation tools. PNG and WebP images rarely contain embedded thumbnails. |
| **Priority** | **P1** — trivially implementable with existing infrastructure. |

### 5.5 Comprehensive Metadata Report

| Field | Detail |
|---|---|
| **What it detects** | Deep extraction and cross-referencing of all metadata layers: EXIF, XMP, IPTC, ICC colour profile, Photoshop IRB, and file-system timestamps. |
| **Question answered** | "What is the complete provenance story told by the metadata?" |
| **Layer** | Rust (core extraction) + Python sidecar (XMP/IPTC parsing) |
| **Algorithm** | 1. Extract all EXIF tags (existing in `metadata.rs`). 2. Parse XMP sidecar or embedded XMP (using `xmp-toolkit-rs` if available, or Python `defusedxml` for XMP parsing). 3. Parse IPTC fields (caption, creator, copyright, keywords). 4. Extract ICC colour profile information (colour space, rendering intent, profile creator). 5. Cross-reference timestamps: EXIF DateTimeOriginal vs file creation date vs XMP CreateDate vs GPS timestamp. 6. Detect editing software chain: XMP `xmp:CreatorTool` + `photoshop:History` action log reveals the complete editing pipeline. 7. Present as a unified timeline with conflict highlighting. |
| **Implementation** | Extended `metadata.rs` in Rust for EXIF. Python sidecar (`metadata_deep.py`) for XMP, IPTC, and ICC parsing where Rust crate support is lacking. |
| **Complexity** | Medium — the challenge is handling the many metadata formats and their inconsistent implementations across cameras and software. |
| **Dependencies** | Rust: `kamadak-exif` (existing). Python: `Pillow` (existing) for IPTC/ICC, `defusedxml` for safe XMP parsing (~50 KB). Optional Rust: `xmp-toolkit-rs`. |
| **Limitations** | Metadata can be freely modified by anyone with the right tools. Its presence is informative, but its absence or specific content is never conclusive proof of anything. The tool should present metadata as claims to be evaluated, not as facts. |
| **Priority** | **P2** — extends existing metadata extraction with a richer analysis view. |

---

## 6. Context and Provenance

Tools that answer: *"Where has this image been before? What is its history?"*

### 6.1 Local Perceptual Hash Index

| Field | Detail |
|---|---|
| **What it detects** | Maintains a local database of perceptual hashes for all images processed by Jura Trace. Enables instant "have I seen this before?" lookup and "what other images are similar?" search without any network request. |
| **Question answered** | "Have I (or my organisation) analysed this image before? Are there similar images in our archive?" |
| **Layer** | Rust (hash computation — already implemented in `fingerprint.rs`, storage in `db.rs`) |
| **Algorithm** | 1. On every image import/verification, compute aHash, dHash, and pHash (already done). 2. Store in SQLite with file path, timestamps, and verification results (already done). 3. On new image analysis, query the hash database for near-matches (Hamming distance <= configurable threshold). 4. Display matched images with their previous verification results, enabling the analyst to trace how the image has changed over time. 5. Support "image lineage" view: construct a graph of parent-child relationships based on perceptual hash similarity. |
| **Implementation** | Extend existing `db.rs` with a hash similarity search function. The current schema already stores fingerprint hashes. Add a Tauri command `search_similar_images` that queries with a configurable Hamming distance threshold (default 10). Frontend: new "Similar Images" panel in the verify page. |
| **Complexity** | Low — all storage and computation already exists. The similarity search is a linear scan of the hash table (fast for typical institutional collections of 10K-100K images; may need a VP-tree or BK-tree index for larger collections). |
| **Dependencies** | None new. |
| **Limitations** | Only searches the local database — cannot find matches on the internet. Perceptual hashes are not robust to major transformations (heavy cropping, colour inversion, significant perspective changes). False positives increase with lower Hamming distance thresholds. |
| **Priority** | **P1** — almost free given existing infrastructure. High value for institutional users (museums, newsrooms) who repeatedly encounter the same images. |

### 6.2 Reverse Image Search Integration

| Field | Detail |
|---|---|
| **What it detects** | Enables the analyst to launch reverse image searches on external services to find where else an image appears on the internet. |
| **Question answered** | "Where else has this image been published? Is it a known stock photo, previously debunked image, or frequently recycled content?" |
| **Layer** | SvelteKit frontend (URL construction) + Rust (system browser launch) |
| **Algorithm** | This is not an analysis tool — it is a workflow integration. 1. Provide one-click buttons to open reverse image search in the system browser for: Google Images (lens.google.com), TinEye, Yandex Images, Bing Visual Search. 2. For local images: copy the file to a temporary location and open the browser upload page; or encode a thumbnail as a data URL for services that support URL-based search. 3. Display the buttons prominently in the "Investigate Further" section of the verify page. |
| **Implementation** | SvelteKit buttons that invoke a Tauri command to open the system browser with a constructed URL. For file-based images, use the Tauri shell plugin's `open` command. No image data is sent to any service automatically — the user explicitly initiates the search by clicking the button. |
| **Complexity** | Low |
| **Dependencies** | `tauri-plugin-shell` (existing). |
| **Limitations** | Requires network access. The user must understand that clicking the button sends the image to an external service. The UI must clearly disclose this: "This will open your browser and upload the image to [service name]." Does not work for images that have never been published online. |
| **Priority** | **P1** — trivial implementation, massive value for verification workflows. The GIJN verification methodology identifies reverse image search as the single most important verification step. |

### 6.3 Image Lineage Graph

| Field | Detail |
|---|---|
| **What it detects** | Visualises the relationship between images in the local database, showing which images are derived from which others (via perceptual hash similarity and C2PA ingredient relationships). |
| **Question answered** | "What is the genealogy of this image? What were its source images?" |
| **Layer** | Rust (graph construction from hash DB + C2PA ingredients) + SvelteKit frontend (graph rendering) |
| **Algorithm** | 1. Query the local hash database for all images within Hamming distance threshold of the target image. 2. Extract C2PA ingredient assertions (already implemented in `c2pa.rs`) — these provide explicit parent-child relationships. 3. Merge hash-based similarity clusters with C2PA ingredient chains. 4. Render as an interactive node-link diagram where nodes are images (with thumbnails) and edges represent relationships (hash similarity, C2PA ingredient, or temporal sequence). |
| **Implementation** | Graph construction in Rust. Frontend rendering using a lightweight graph library (e.g., `d3-force` or `cytoscape.js` — ~100 KB). The graph is small (typically < 50 nodes) so no performance concerns. |
| **Complexity** | Medium — the graph construction is straightforward; the interactive visualisation requires careful UX design. |
| **Dependencies** | New frontend: graph visualisation library (~100 KB). |
| **Limitations** | Only shows relationships within the local database. Cannot reconstruct the complete history of an image that passed through systems outside Jura Trace. C2PA ingredient relationships are only present in images that have been signed with content credentials. Hash-based similarity may produce false connections between visually similar but unrelated images. |
| **Priority** | **P3** — requires a substantial local database to be useful. Best suited for institutional deployments. |

### 6.4 Known Misinformation Cross-Reference

| Field | Detail |
|---|---|
| **What it detects** | Checks the image's perceptual hash against a local database of known misinformation images. |
| **Question answered** | "Is this a known misinformation image?" |
| **Layer** | Rust (hash comparison) + periodic database updates (opt-in download) |
| **Algorithm** | 1. Maintain a local database of perceptual hashes of known misinformation images. 2. On image verification, compare the image's pHash against this database. 3. If a match is found, display the associated context: what the image was used for, when it was debunked, and by whom. 4. The database is updated periodically via an opt-in download mechanism (user explicitly triggers the update, no automatic network requests). |
| **Implementation** | Hash database as a SQLite table or flat binary file. Update mechanism via a Tauri command that downloads a signed hash database from a Jura Labs server (opt-in, clearly disclosed). |
| **Complexity** | Low (matching) / Medium (building and maintaining the database) |
| **Dependencies** | None new for matching. Database curation is an ongoing operational effort. |
| **Limitations** | The database can only contain images that have been identified and catalogued. Novel misinformation will not be detected. The database must be carefully curated to avoid false entries. Hash matching will miss images that have been significantly modified from the known version. |
| **Priority** | **P3** — high value in principle but requires sustained curation effort. Consider partnering with existing fact-checking organisations (AFP Fact Check, Full Fact, First Draft) for database content. |

---

## 7. AI-Specific Detection

Tools that answer: *"Was this image generated by AI, and if so, what kind?"*

These tools extend the existing 21-signal deepfake detector and trained GBM classifier.

### 7.1 GAN Fingerprint Visualisation

| Field | Detail |
|---|---|
| **What it detects** | Visualises the spectral fingerprint characteristic of GAN-generated images. GANs using transposed convolution (deconvolution) layers produce periodic spectral peaks at frequencies determined by the upsampling stride. Different GAN architectures produce different spectral signatures, enabling model attribution. |
| **Question answered** | "Does this image show spectral artefacts consistent with a specific GAN architecture?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Compute 2D FFT of each colour channel independently. 2. Average the log-magnitude spectra across channels. 3. Subtract the expected natural image spectrum (modelled as 1/f^beta power law) to isolate anomalous spectral peaks. 4. Detect peaks using local maxima detection with significance testing. 5. Match peak locations and patterns against a database of known GAN spectral signatures: StyleGAN2 (characteristic ring pattern), StyleGAN3 (reduced but not eliminated spectral artefacts), ProGAN (strong checkerboard peaks). 6. Render annotated spectrum with highlighted peaks and model attribution if matched. |
| **Implementation** | Python sidecar (`gan_fingerprint.py`). Extends existing frequency analysis in `deepfake.py`. Spectral signature database as a JSON file mapping peak patterns to GAN architectures. |
| **Complexity** | Medium — FFT computation is existing; peak detection and model attribution require careful calibration. |
| **Dependencies** | scipy, numpy (existing). Spectral signature database (~10 KB JSON). |
| **Limitations** | StyleGAN3 and modern architectures specifically address checkerboard artefacts, producing weaker spectral fingerprints. Diffusion models do not produce the same periodic spectral peaks as GANs — this tool is primarily useful for GAN detection, not diffusion model detection. JPEG compression can mask or mimic spectral peaks. Post-processing (resizing, sharpening) alters the spectrum. |
| **Priority** | **P2** — extends existing frequency analysis with visual output and model attribution. |

### 7.2 Diffusion Model Artefact Detection

| Field | Detail |
|---|---|
| **What it detects** | Identifies artefacts specific to diffusion model outputs (Stable Diffusion, DALL-E, Midjourney, Flux): unnaturally smooth textures, VAE decoder banding, semantic inconsistencies, and characteristic resolution patterns. |
| **Question answered** | "Does this image show artefacts consistent with diffusion model generation?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. **Texture smoothness analysis**: Compute local texture complexity using Haralick features (GLCM contrast, entropy) in sliding windows. AI-generated images often show unnaturally uniform texture complexity. Flag regions where GLCM entropy is significantly lower than expected for the semantic content (e.g., skin, fabric, foliage). 2. **VAE banding detection**: Latent diffusion models (SD, SDXL, Flux) encode images in a latent space and decode via a VAE. The VAE decoder can introduce subtle colour banding, particularly in smooth gradients. Detect by analysing gradient smoothness in LAB colour space. 3. **Resolution fingerprint**: Check image dimensions against known generation sizes (512x512, 768x768, 1024x1024, 1344x768 for SDXL, etc.). While not conclusive, generation-resolution images with no EXIF data are a supporting signal. 4. **Prompt-style detection via CLIP**: If CLIP is available, compute embedding similarity to a set of style prompts characteristic of different generators ("cinematic lighting, 8k, ultrarealistic" for SD; "in the style of" patterns for Midjourney). |
| **Implementation** | Python sidecar (`diffusion_artefacts.py`). GLCM features via scikit-image (existing). CLIP comparison via existing `clip_detector.py` infrastructure. |
| **Complexity** | Medium-High — diffusion model artefacts are subtle and evolve rapidly with each model generation. |
| **Dependencies** | scikit-image, scipy (existing). Optional: open-clip (existing). |
| **Limitations** | Diffusion model artefacts are increasingly subtle as models improve. The texture smoothness signal has high false-positive rates on professionally retouched photographs. Resolution fingerprinting is easily defeated by resizing. This tool should always present results as probability estimates, not binary classifications. |
| **Priority** | **P1** — diffusion models are now the dominant source of synthetic images. The existing detector was calibrated for GANs; explicit diffusion artefact detection is the most important gap to fill. |

### 7.3 Upscaling Artefact Detection

| Field | Detail |
|---|---|
| **What it detects** | Identifies artefacts from AI upscaling (Real-ESRGAN, Topaz, DALL-E upscale) which produce characteristic hallucinated detail and unnaturally sharp edges at fine scales. |
| **Question answered** | "Has this image been AI-upscaled?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Analyse the relationship between image resolution and detail level — upscaled images have disproportionately little fine detail relative to their pixel count. Compute the ratio of high-frequency energy to total energy at the native resolution; compare against expected ratios for the claimed camera resolution. 2. Detect hallucinated texture: AI upscalers generate plausible-looking but fabricated fine detail. Compute patch-level self-similarity — hallucinated textures show more repetitive patterns than natural textures. 3. Analyse edge sharpness distribution — AI upscalers produce edges that are uniformly sharp, while camera optics produce edges whose sharpness varies with distance from the optical axis and depth of field. 4. Check for JPEG re-compression artefacts at a resolution that is a clean integer multiple of common generation sizes (e.g., 2048x2048 = 512x512 x 4). |
| **Implementation** | Python sidecar (`upscale_detection.py`). |
| **Complexity** | Medium |
| **Dependencies** | scipy, scikit-image (existing). |
| **Limitations** | Optical zoom at capture and digital zoom in-camera produce some similar characteristics to AI upscaling. The tool should not flag legitimate in-camera processing. Images upscaled using traditional bicubic or Lanczos interpolation are easy to distinguish from AI upscaling but may produce false positives with this detector. |
| **Priority** | **P2** — upscaling is increasingly used to make AI-generated images appear higher-resolution. |

### 7.4 Face Consistency Analysis

| Field | Detail |
|---|---|
| **What it detects** | Analyses faces in the image for internal consistency: lighting direction on the face vs background, perspective consistency between facial features, eye reflection consistency (both eyes should reflect the same scene), skin texture continuity, and bilateral symmetry anomalies. |
| **Question answered** | "Are the faces in this image internally consistent, or do they show signs of generation or manipulation?" |
| **Layer** | Python sidecar |
| **Algorithm** | 1. Detect faces using a lightweight face detector (OpenCV DNN face detector or MediaPipe FaceMesh). 2. Extract 468 facial landmarks (MediaPipe) for geometric analysis. 3. **Lighting consistency**: Compute luminance gradient direction on each side of the face; compare with the dominant lighting direction in the background (from Section 4.5). 4. **Eye reflection analysis**: Extract eye regions, detect specular highlights, compare reflection positions between left and right eyes — they should show the same light source(s). 5. **Bilateral symmetry**: Measure asymmetry in skin texture, lighting, and colour between left and right halves of the face. Natural faces have moderate asymmetry; AI faces often have either too much or too little. 6. **Skin texture analysis**: Compute Haralick features on facial skin regions; compare with expected texture for the apparent image resolution. AI-generated skin often lacks pore-level detail or shows unnaturally uniform texture. |
| **Implementation** | Python sidecar (`face_analysis.py`). Face detection via OpenCV DNN (already bundled) or MediaPipe (new dependency, ~10 MB). |
| **Complexity** | High — face analysis requires robust landmark detection and careful handling of pose variation, occlusion, and lighting. |
| **Dependencies** | OpenCV (existing). New: `mediapipe` (~10 MB) for 468-point face mesh. Alternatively, use OpenCV DNN face detector (already bundled) with a simpler landmark model. |
| **Limitations** | Requires detectable faces. Side profiles, occluded faces, and very small faces will produce unreliable results. Makeup, surgical modification, and strong artistic lighting can produce asymmetry and texture patterns that mimic AI artefacts. The tool must clearly state that face analysis is probabilistic and should not be used to make definitive claims about specific individuals. |
| **Priority** | **P2** — face manipulation is a common and high-impact form of synthetic media. However, the implementation is complex and the false-positive risk is significant. |

### 7.5 Text Rendering Quality Analysis

| Field | Detail |
|---|---|
| **What it detects** | AI image generators (particularly diffusion models up to 2025) struggle with rendering coherent text. This tool detects text regions and analyses them for AI-typical artefacts: malformed characters, inconsistent baselines, meaningless character sequences, and impossible typographic features. |
| **Question answered** | "Does the text in this image look machine-generated or naturally rendered?" |
| **Layer** | Python sidecar (text region detection) + Ollama (text coherence analysis) |
| **Algorithm** | 1. Detect text regions using EAST text detector or CRAFT (Character Region Awareness for Text). 2. Apply OCR (Tesseract or macOS Vision) to extract character sequences. 3. Analyse OCR confidence per character — AI-generated text produces systematically lower confidence than real text. 4. Check text coherence: are the detected words real words? Is the text in a recognisable language? Are character shapes consistent with a real typeface? 5. Prompt LLaVA: "Examine any text visible in this image. Is the text coherent and properly rendered, or does it show signs of AI generation such as malformed characters, nonsensical words, or inconsistent styling?" |
| **Implementation** | Python sidecar (`text_analysis.py`). OCR via `pytesseract` or macOS Vision framework. LLaVA for coherence analysis. |
| **Complexity** | Medium — text detection and OCR are well-established; the coherence analysis layer adds complexity. |
| **Dependencies** | New: `pytesseract` + Tesseract system binary (~30 MB), or macOS Vision framework (built-in, no additional dependency on macOS). Ollama + LLaVA (existing optional). |
| **Limitations** | Many AI-generated images contain no text, limiting applicability. Newer models (DALL-E 3, Flux, Ideogram) have significantly improved text rendering. Heavily compressed or low-resolution images produce OCR errors that can be confused with AI artefacts. Non-English text may produce false positives due to OCR limitations. This signal weakens with each generation of AI models. |
| **Priority** | **P2** — useful now, but diminishing value as AI text rendering improves. Best as a supporting signal, not a primary detector. |

### 7.6 AI Watermark Detection (Unified Panel)

| Field | Detail |
|---|---|
| **What it detects** | Detects embedded invisible watermarks from known AI generation platforms: Stable Diffusion (DWT-DCT watermark via `invisible-watermark`), SDXL (48-bit pattern), Flux (same library), and C2PA `claim_generator` fields indicating AI origin. |
| **Question answered** | "Does this image contain a watermark from a known AI generation platform?" |
| **Layer** | Python sidecar (watermark extraction — already partially implemented) + Rust (C2PA `claim_generator` — already implemented) |
| **Algorithm** | 1. Extract DWT-DCT watermark using `invisible-watermark` library (already in requirements). 2. Check extracted payload against known patterns: SD v1 "StableDiffusionV1" (136 bits), SDXL 48-bit binary pattern. 3. Parse C2PA assertions for `digitalSourceType: trainedAlgorithmicMedia` (already implemented in Sprint 19). 4. Check C2PA `claim_generator` for known AI tool names: "DALL-E", "Adobe Firefly", "Midjourney", "Stable Diffusion". 5. Return a structured result: watermark found (yes/no), platform attribution, confidence. |
| **Implementation** | Largely already built across the watermark service and C2PA AI declaration detection. Needs to be unified into a single "AI Origin Detection" panel in the UI with clear presentation. |
| **Complexity** | Low — components already exist. |
| **Dependencies** | `invisible-watermark` (existing), `c2pa-rs` (existing). |
| **Limitations** | Watermarks are trivially removed by re-encoding, screenshot, or print-and-scan. Many AI-generated images have no watermark (Midjourney, most open-source models). C2PA credentials can be stripped. Absence of a watermark does not mean the image is authentic. |
| **Priority** | **P1** — already 80% implemented. Unifying the presentation is the remaining work. |

---

## 8. Interactive Investigation Interface

The tools above are most powerful when combined with an interactive investigation interface that lets the analyst move fluidly between different views and annotations.

### 8.1 Region-of-Interest (ROI) Selection

| Field | Detail |
|---|---|
| **What it provides** | An interactive drawing tool allowing the analyst to select rectangular, elliptical, or freeform regions for focused analysis. All analysis tools (noise, frequency, ELA, channel separation) can be re-run on the selected region only, enabling comparison between suspicious and reference regions. |
| **Layer** | SvelteKit frontend (Canvas drawing) + Python sidecar (region-specific analysis) |
| **Complexity** | Medium |
| **Priority** | **P1** — the ability to compare regions is fundamental to forensic analysis. This is the single most impactful UX improvement for trained analysts. |

### 8.2 Annotation and Note-Taking

| Field | Detail |
|---|---|
| **What it provides** | Persistent annotations (arrows, circles, text notes) overlaid on the image, saved with the verification record. Enables the analyst to mark up findings and export annotated images for reports. |
| **Layer** | SvelteKit frontend (Canvas overlay) + Rust (annotation storage in SQLite) |
| **Complexity** | Medium |
| **Priority** | **P2** — important for institutional workflows (case files, court submissions), but the analysis tools are the higher priority. |

### 8.3 Side-by-Side Comparison

| Field | Detail |
|---|---|
| **What it provides** | A dual-pane view showing two images (or two views of the same image) with synchronised zoom and pan. Essential for comparing: original vs manipulated, thumbnail vs main image, two candidate source images, or different analysis overlays. |
| **Layer** | SvelteKit frontend |
| **Complexity** | Low-Medium |
| **Priority** | **P1** — critical for the thumbnail consistency check, EXIF thumbnail comparison, and lineage analysis. |

### 8.4 Investigation Timeline

| Field | Detail |
|---|---|
| **What it provides** | A timeline view showing all analysis events, annotations, and findings in chronological order. Supports the analyst's investigation narrative and can be exported as part of the case file. |
| **Layer** | SvelteKit frontend + Rust (audit log — already implemented) |
| **Complexity** | Low |
| **Priority** | **P3** — useful for institutional record-keeping but not critical for the analysis workflow itself. |

---

## 9. Implementation Roadmap

### Phase A: Foundation (Sprints 21-22)

**Theme**: Low-cost, high-impact tools that leverage existing infrastructure.

| Sprint | Tool | Section | Priority | Complexity | New Dependencies |
|--------|------|---------|----------|------------|------------------|
| 21 | Colour channel separation | 4.1 | P1 | Low | None |
| 21 | Noise pattern visualisation | 4.4 | P1 | Low | None |
| 21 | Per-channel histogram equalisation | 4.6 | P1 | Low | None |
| 21 | Thumbnail consistency checking | 5.4 | P1 | Low | None |
| 21 | Reverse image search buttons | 6.2 | P1 | Low | None |
| 21 | AI watermark detection panel (unified) | 7.6 | P1 | Low | None |
| 22 | Frequency domain visualisation | 4.2 | P1 | Low-Med | `pywt` |
| 22 | JPEG quantisation grid visualisation | 4.3 | P1 | Medium | None |
| 22 | Local perceptual hash search | 6.1 | P1 | Low | None |
| 22 | Side-by-side comparison view | 8.3 | P1 | Low-Med | None |
| 22 | Region-of-interest selection | 8.1 | P1 | Medium | None |

**Outcome**: 11 new tools, zero or one new dependency. The existing verify page gains a comprehensive visual inspection panel and analyst workflow tools.

### Phase B: Geolocation and Temporal (Sprints 23-24)

**Theme**: Location and time verification tools for OSINT and journalism.

| Sprint | Tool | Section | Priority | Complexity | New Dependencies |
|--------|------|---------|----------|------------|------------------|
| 23 | Sun angle / shadow geometry | 2.1 | P1 | Medium | None |
| 23 | Shadow-based time estimation | 3.1 | P1 | Medium | None (shared with 2.1) |
| 23 | GPS coordinate mapping | 2.5 | P1 | Low-Med | `leaflet` (frontend) |
| 23 | Device JPEG compression signatures | 5.3 | P1 | Medium | None (Q-table JSON) |
| 24 | Diffusion model artefact detection | 7.2 | P1 | Med-High | None |
| 24 | Sky and atmosphere analysis | 2.3 | P2 | Medium | None |
| 24 | Weather verification (opt-in API) | 3.3 | P2 | Low | None |
| 24 | Seasonal indicators | 3.2 | P2 | Low-Med | None |

**Outcome**: Jura Trace becomes a credible OSINT investigation tool. Journalists can verify claimed location, date, and time from image evidence.

### Phase C: Advanced Forensics (Sprints 25-26)

**Theme**: Deep forensic analysis and AI-specific detection.

| Sprint | Tool | Section | Priority | Complexity | New Dependencies |
|--------|------|---------|----------|------------|------------------|
| 25 | GAN fingerprint visualisation | 7.1 | P2 | Medium | None |
| 25 | Upscaling artefact detection | 7.3 | P2 | Medium | None |
| 25 | Luminance gradient mapping | 4.5 | P2 | Low | None |
| 25 | Comprehensive metadata report | 5.5 | P2 | Medium | `defusedxml` |
| 26 | Face consistency analysis | 7.4 | P2 | High | `mediapipe` (~10 MB) |
| 26 | Text rendering quality analysis | 7.5 | P2 | Medium | `pytesseract` (optional) |
| 26 | Camera sensor fingerprint (PRNU) | 5.1 | P2 | High | `pywt` (shared) |
| 26 | Annotation and note-taking | 8.2 | P2 | Medium | None |

**Outcome**: Full professional forensic toolkit. Institutional users (museums, news agencies, legal teams) have publication-grade investigation capabilities.

### Phase D: Ecosystem (Sprints 27-28)

**Theme**: Context, provenance, and advanced features.

| Sprint | Tool | Section | Priority | Complexity | New Dependencies |
|--------|------|---------|----------|------------|------------------|
| 27 | Road/signage/architecture analysis | 2.4 | P2 | High | YOLO model (optional) |
| 27 | Image lineage graph | 6.3 | P3 | Medium | Graph viz library |
| 27 | Investigation timeline | 8.4 | P3 | Low | None |
| 28 | Vegetation analysis | 2.2 | P3 | Medium | Biome CSV |
| 28 | Celestial verification | 3.4 | P3 | Med-High | Star catalogue |
| 28 | Lens distortion matching | 5.2 | P3 | High | Lensfun DB subset |
| 28 | Known misinformation database | 6.4 | P3 | Low (matching) | Database curation |

**Outcome**: Comprehensive investigation platform. Jura Trace is a full Esper machine — every pixel can be interrogated from multiple angles.

---

## 10. Dependency Summary

### New Python Dependencies

| Package | Size | Used By | Phase |
|---------|------|---------|-------|
| `PyWavelets` (`pywt`) | ~1 MB | Frequency visualisation (4.2), PRNU (5.1) | A, C |
| `defusedxml` | ~50 KB | Metadata report XMP parsing (5.5) | C |
| `mediapipe` | ~10 MB | Face consistency analysis (7.4) | C |
| `pytesseract` | ~30 KB (+ Tesseract binary ~30 MB) | Text analysis (7.5) | C (optional) |

### New Frontend Dependencies

| Package | Size | Used By | Phase |
|---------|------|---------|-------|
| `leaflet` | ~40 KB | GPS mapping (2.5) | B |
| Graph visualisation (d3-force or cytoscape.js) | ~100 KB | Image lineage (6.3) | D |

### New Static Data

| Data | Size | Used By | Phase |
|------|------|---------|-------|
| JPEG Q-table database | ~50 KB JSON | Compression signatures (5.3) | B |
| GAN spectral signature database | ~10 KB JSON | GAN fingerprinting (7.1) | C |
| Koeppen-Geiger biome grid | ~50 KB CSV | Vegetation analysis (2.2) | D |
| Hipparcos star catalogue subset | ~2 MB CSV | Celestial verification (3.4) | D |
| Lensfun database subset | ~200 KB JSON | Lens distortion (5.2) | D |

**Total new dependency footprint**: ~11 MB (Python packages) + ~140 KB (frontend) + ~2.3 MB (static data). Modest relative to the existing sidecar (~350 MB with CLIP model).

---

## 11. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **LLaVA hallucination** in geolocation/vegetation/signage analysis | High | Medium | All LLM outputs are presented as "suggestions" with explicit confidence caveats. Never auto-populate verdict fields from LLM output. |
| **False positives** from new detectors degrading user trust | Medium | High | Every new signal must pass the existing calibration pipeline (`scripts/calibrate.py`) against the 709-image corpus before integration into trust scoring. New signals start with weight 0 (visualisation only) and are promoted to scoring signals only after calibration. |
| **Scope creep** — building features nobody uses | Medium | Medium | Phase A tools (visual inspection, hash search, reverse image search) are requested by existing personas. Phase B-D tools should be validated against persona needs before implementation. |
| **External API dependency** creep | Low | High | Every network request is opt-in, clearly disclosed, and the tool must function (in degraded mode) without network access. No tool's core analysis requires a network call. |
| **Model evolution** — AI artefact detectors becoming obsolete as models improve | High | Medium | Design all AI-specific detectors to be updatable: signature databases are JSON files, not hard-coded. The trained classifier can be retrained. Invest in fundamental signals (frequency analysis, noise analysis) over model-specific heuristics. |
| **Performance** — adding 30 tools to the verify pipeline | Medium | Medium | New tools do NOT run automatically. They are available on-demand in the investigation interface. Only the existing 16 detectors run in the automated pipeline. Analysts invoke investigation tools individually as needed. |

---

## 12. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Investigation tool adoption** | 40% of verification sessions use at least one investigation tool | Audit log analysis |
| **Time to first investigative finding** | < 30 seconds from opening an image to the first non-automated insight | User testing |
| **False positive rate** (for tools integrated into scoring) | < 5% on the calibration corpus | Calibration pipeline |
| **Analyst satisfaction** | "Essential" or "Very Useful" rating from 70% of pilot testers | Post-pilot survey |
| **New dependency footprint** | < 15 MB total new Python packages | Measured at each phase gate |

---

## 13. Design Philosophy

> The Esper machine doesn't tell Deckard what to think. It shows him what's there, and he decides what it means.

Every tool in this document produces *evidence*, not *conclusions*. The trust score pipeline provides automated verdict recommendations, but the investigation tools are the analyst's eyes and hands — they reveal signals that the analyst interprets in context.

This distinction is critical for Jura Trace's credibility with institutional users. A tool that says "this is fake" can be wrong and damage trust. A tool that says "this region shows 3.2 dB lower noise standard deviation than the surrounding area, and the shadow direction deviates 47 degrees from the dominant lighting — here is the visualisation" gives the analyst the evidence to reach their own conclusion.

The Esper machine is not an oracle. It is a microscope.

---

*Document version 0.1.0 — 28 March 2026*
*Jura Labs Engineering*
