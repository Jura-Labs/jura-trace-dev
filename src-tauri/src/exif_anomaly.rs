//! EXIF anomaly detection — analyse image metadata for manipulation indicators.
//!
//! Performs seven categories of checks: software detection, missing EXIF,
//! timestamp validation, dimension consistency, GPS plausibility, field
//! completeness, and metadata-injection heuristics (templated timestamps,
//! integer-degree GPS, pipeline-library software fields, compound MakerNote
//! absence on mandatory-vendor cameras, and iPhone-sRGB colour-space mismatch).
//! Returns a trust score (0.0–1.0) with findings.
//!
//! See `docs/design/exif-injection-detection.md` for the injection-detection
//! design rationale, severity justifications, and known false-positive triggers.

use crate::metadata::ImageMetadata;
use serde::{Deserialize, Serialize};

// ===== Types =====

/// Severity level for an EXIF anomaly finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Score deduction for this severity level.
    pub fn deduction(&self) -> f64 {
        match self {
            Self::Info => 0.0,
            Self::Low => 0.05,
            Self::Medium => 0.10,
            Self::High => 0.20,
            Self::Critical => 0.35,
        }
    }
}

/// A single anomaly finding from EXIF analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnomalyFinding {
    pub check_id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub category: String,
}

/// Complete EXIF anomaly analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifAnalysis {
    pub findings: Vec<AnomalyFinding>,
    pub trust_score: f64,
    pub fields_populated: u32,
    pub fields_total: u32,
    pub has_exif: bool,
    /// GPS latitude in decimal degrees (north positive), if present in EXIF.
    pub gps_latitude: Option<f64>,
    /// GPS longitude in decimal degrees (east positive), if present in EXIF.
    pub gps_longitude: Option<f64>,
    /// Camera-origin authenticity confidence in [0.0, 1.0] derived from
    /// MakerNote presence + vendor match. 1.0 means a strong positive signal
    /// that this is a genuine camera capture (not AI-generated). Used by
    /// the deepfake scoring layer to mitigate false positives on computational
    /// photography output. 0.0 means no MakerNote, no signal either way.
    pub camera_authenticity_bonus: f64,
}

// ===== Analysis =====

/// Analyse EXIF metadata for anomalies and compute a trust score.
///
/// Pass `None` for metadata if the file had no EXIF data.
/// Pass actual decoded image dimensions for dimension consistency checks.
pub fn analyse(
    metadata: Option<&ImageMetadata>,
    actual_width: Option<u32>,
    actual_height: Option<u32>,
) -> ExifAnalysis {
    let mut findings = Vec::new();

    let (fields_populated, fields_total, gps_latitude, gps_longitude, camera_authenticity_bonus) =
        match metadata {
            Some(meta) => {
                check_software(meta, &mut findings);
                check_missing_exif(meta, &mut findings);
                check_timestamps(meta, &mut findings);
                check_dimensions(meta, actual_width, actual_height, &mut findings);
                check_gps(meta, &mut findings);
                let bonus = check_maker_note_authenticity(meta, &mut findings);
                // Injection-detection heuristics — see docs/design/exif-injection-detection.md
                check_pipeline_library_software(meta, &mut findings);
                check_templated_timestamps(meta, &mut findings);
                check_integer_degree_gps(meta, &mut findings);
                check_mandatory_maker_note_missing(meta, &mut findings);
                check_iphone_colour_space_mismatch(meta, &mut findings);
                let (fp, ft) = compute_completeness(meta);
                (fp, ft, meta.gps_latitude, meta.gps_longitude, bonus)
            }
            None => {
                findings.push(AnomalyFinding {
                    check_id: "no_exif_data".into(),
                    title: "No EXIF data present".into(),
                    description: "This file contains no EXIF metadata. Authentic camera images \
                                  always have EXIF data — absence may indicate AI generation, \
                                  social media reprocessing, or deliberate stripping."
                        .into(),
                    severity: Severity::High,
                    category: "completeness".into(),
                });
                (0, 16, None, None, 0.0)
            }
        };

    // Score: start at 1.0, deduct per finding severity
    let mut score = 1.0_f64;
    for finding in &findings {
        score -= finding.severity.deduction();
    }
    let trust_score = score.max(0.0);

    // Sort by severity descending (Critical first)
    findings.sort_by(|a, b| b.severity.cmp(&a.severity));

    ExifAnalysis {
        findings,
        trust_score,
        fields_populated,
        fields_total,
        has_exif: metadata.is_some(),
        gps_latitude,
        gps_longitude,
        camera_authenticity_bonus,
    }
}

/// Check whether the image carries an authentic camera MakerNote signature.
///
/// MakerNotes are vendor-proprietary binary blobs embedded by camera firmware.
/// AI image generators do not synthesise these. A present, substantial MakerNote
/// whose `Make` tag matches a known vendor is a strong positive authenticity
/// signal.
///
/// Returns the bonus score in [0.0, 1.0]. The deepfake scoring layer applies
/// this as a negative adjustment to the AI probability.
fn check_maker_note_authenticity(
    meta: &crate::metadata::ImageMetadata,
    findings: &mut Vec<AnomalyFinding>,
) -> f64 {
    let bonus = crate::metadata::camera_authenticity_confidence(meta);
    if bonus >= 0.7 {
        let make = meta
            .camera_make
            .as_deref()
            .unwrap_or("unknown vendor")
            .to_string();
        let model = meta.camera_model.as_deref().unwrap_or("").to_string();
        let model_part = if model.is_empty() {
            String::new()
        } else {
            format!(" {model}")
        };
        findings.push(AnomalyFinding {
            check_id: "authentic_maker_note".into(),
            title: "Camera MakerNote signature detected".into(),
            description: format!(
                "This file carries a {make}{model_part} MakerNote ({} bytes), a vendor-proprietary \
                 binary signature embedded by genuine camera firmware. AI image generators do not \
                 synthesise MakerNotes — this is a positive authenticity signal.",
                meta.maker_note_length
            ),
            severity: Severity::Info,
            category: "authenticity".into(),
        });
    }
    bonus
}

// ===== Check Functions =====

/// Known software patterns and their severity.
const AI_GENERATORS: &[&str] = &[
    "dall-e",
    "dall·e",
    "midjourney",
    "stable diffusion",
    "adobe firefly",
    "comfyui",
    "automatic1111",
    "invokeai",
    "leonardo.ai",
    "ideogram",
    "chatgpt",
    "bing image",
];

const METADATA_TOOLS: &[&str] = &["exiftool", "exiv2", "jhead", "pyexiv"];

const IMAGE_EDITORS: &[&str] = &[
    "photoshop",
    "gimp",
    "affinity photo",
    "pixelmator",
    "snapseed",
    "picsart",
    "facetune",
    "paint.net",
    "corel paintshop",
    "topaz",
    "remini",
];

const RAW_PROCESSORS: &[&str] = &[
    "lightroom",
    "capture one",
    "dxo",
    "on1 photo",
    "darktable",
    "rawtherapee",
];

fn check_software(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let Some(software) = &meta.software else {
        return;
    };
    let lower = software.to_lowercase();

    for pattern in AI_GENERATORS {
        if lower.contains(pattern) {
            findings.push(AnomalyFinding {
                check_id: "software_ai_generator".into(),
                title: "AI generation software detected".into(),
                description: format!(
                    "Software field contains '{software}', which is associated with AI image generation."
                ),
                severity: Severity::Critical,
                category: "software".into(),
            });
            return;
        }
    }

    for pattern in METADATA_TOOLS {
        if lower.contains(pattern) {
            findings.push(AnomalyFinding {
                check_id: "software_metadata_tool".into(),
                title: "Metadata manipulation tool detected".into(),
                description: format!(
                    "Software field contains '{software}'. Metadata editing tools can fabricate or alter EXIF data."
                ),
                severity: Severity::High,
                category: "software".into(),
            });
            return;
        }
    }

    for pattern in IMAGE_EDITORS {
        if lower.contains(pattern) {
            findings.push(AnomalyFinding {
                check_id: "software_editor".into(),
                title: "Image editing software detected".into(),
                description: format!(
                    "Software field contains '{software}'. This indicates the image has been \
                     post-processed, though this is common in legitimate workflows."
                ),
                severity: Severity::Low,
                category: "software".into(),
            });
            return;
        }
    }

    for pattern in RAW_PROCESSORS {
        if lower.contains(pattern) {
            findings.push(AnomalyFinding {
                check_id: "software_raw_processor".into(),
                title: "RAW processor detected".into(),
                description: format!(
                    "Software field contains '{software}'. RAW processing is a standard \
                     photographic workflow and does not indicate manipulation."
                ),
                severity: Severity::Info,
                category: "software".into(),
            });
            return;
        }
    }
}

fn check_missing_exif(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let has_camera = meta.camera_make.is_some() || meta.camera_model.is_some();
    let has_exposure = meta.iso.is_some()
        || meta.exposure_time.is_some()
        || meta.f_number.is_some()
        || meta.focal_length.is_some();

    if !has_camera && !has_exposure {
        findings.push(AnomalyFinding {
            check_id: "no_camera_info".into(),
            title: "No camera information".into(),
            description: "No camera make, model, or exposure settings found. This may indicate \
                          AI generation, screenshot capture, or metadata stripping."
                .into(),
            severity: Severity::Medium,
            category: "completeness".into(),
        });
    } else if has_camera && !has_exposure {
        findings.push(AnomalyFinding {
            check_id: "camera_no_exposure".into(),
            title: "Camera identified but no exposure settings".into(),
            description: "Camera make/model is present but ISO, exposure, and aperture data \
                          are missing. This suggests selective metadata editing."
                .into(),
            severity: Severity::Medium,
            category: "consistency".into(),
        });
    }
}

fn check_timestamps(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let parse_exif_dt = |s: &str| -> Option<chrono::NaiveDateTime> {
        chrono::NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
            .ok()
    };

    let original = meta.datetime_original.as_deref().and_then(parse_exif_dt);
    let modified = meta.datetime_modified.as_deref().and_then(parse_exif_dt);
    let now = chrono::Utc::now().naive_utc();

    // Check for future timestamps
    if let Some(orig) = original {
        if orig > now + chrono::Duration::hours(24) {
            findings.push(AnomalyFinding {
                check_id: "timestamp_future".into(),
                title: "Future timestamp detected".into(),
                description: format!(
                    "Original datetime '{}' is in the future, which indicates a clock error or fabricated metadata.",
                    meta.datetime_original.as_deref().unwrap_or("unknown")
                ),
                severity: Severity::High,
                category: "timestamp".into(),
            });
        }
    }

    // Check modified before original
    if let (Some(orig), Some(modif)) = (original, modified) {
        if modif < orig {
            findings.push(AnomalyFinding {
                check_id: "timestamp_modified_before_original".into(),
                title: "Modified date precedes original date".into(),
                description: format!(
                    "File was modified ({}) before it was originally captured ({}). \
                     This is logically impossible for an unaltered image.",
                    meta.datetime_modified.as_deref().unwrap_or("unknown"),
                    meta.datetime_original.as_deref().unwrap_or("unknown"),
                ),
                severity: Severity::High,
                category: "timestamp".into(),
            });
        } else {
            let gap = modif.signed_duration_since(orig);
            if gap > chrono::Duration::days(365) {
                findings.push(AnomalyFinding {
                    check_id: "timestamp_large_gap".into(),
                    title: "Large gap between capture and modification".into(),
                    description: format!(
                        "Over {} days between original capture and last modification. \
                         This suggests the image was reopened and substantially edited.",
                        gap.num_days()
                    ),
                    severity: Severity::Medium,
                    category: "timestamp".into(),
                });
            } else if gap > chrono::Duration::days(30) {
                findings.push(AnomalyFinding {
                    check_id: "timestamp_moderate_gap".into(),
                    title: "Notable gap between capture and modification".into(),
                    description: format!(
                        "{} days between original capture and last modification.",
                        gap.num_days()
                    ),
                    severity: Severity::Low,
                    category: "timestamp".into(),
                });
            }
        }
    }
}

fn check_dimensions(
    meta: &ImageMetadata,
    actual_width: Option<u32>,
    actual_height: Option<u32>,
    findings: &mut Vec<AnomalyFinding>,
) {
    let (Some(exif_w), Some(exif_h)) = (meta.exif_width, meta.exif_height) else {
        return;
    };
    let (Some(actual_w), Some(actual_h)) = (actual_width, actual_height) else {
        return;
    };
    if exif_w == 0 || exif_h == 0 || actual_w == 0 || actual_h == 0 {
        return;
    }

    // Check both normal and orientation-swapped dimensions
    let normal_match = exif_w == actual_w && exif_h == actual_h;
    let swapped_match = exif_w == actual_h && exif_h == actual_w;

    // Orientations 5-8 swap width and height
    let orientation_swaps = meta.orientation.is_some_and(|o| (5..=8).contains(&o));

    let matches = if orientation_swaps {
        swapped_match || normal_match
    } else {
        normal_match || swapped_match
    };

    if !matches {
        findings.push(AnomalyFinding {
            check_id: "dimension_mismatch".into(),
            title: "EXIF dimensions do not match actual image".into(),
            description: format!(
                "EXIF records {exif_w}x{exif_h} but actual image is {actual_w}x{actual_h}. \
                 The image has been cropped or resized after EXIF was written."
            ),
            severity: Severity::Medium,
            category: "dimensions".into(),
        });
    }
}

fn check_gps(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let lat = meta.gps_latitude;
    let lon = meta.gps_longitude;

    match (lat, lon) {
        (Some(la), Some(lo)) => {
            // Out of range
            if !(-90.0..=90.0).contains(&la) || !(-180.0..=180.0).contains(&lo) {
                findings.push(AnomalyFinding {
                    check_id: "gps_out_of_range".into(),
                    title: "GPS coordinates out of valid range".into(),
                    description: format!(
                        "Coordinates ({la:.4}, {lo:.4}) are outside valid ranges \
                         (latitude -90 to 90, longitude -180 to 180)."
                    ),
                    severity: Severity::High,
                    category: "gps".into(),
                });
            }
            // Null Island
            else if la.abs() < 0.01 && lo.abs() < 0.01 {
                findings.push(AnomalyFinding {
                    check_id: "gps_null_island".into(),
                    title: "GPS points to Null Island (0, 0)".into(),
                    description: "Coordinates are at or near (0, 0) in the Gulf of Guinea. \
                                  This is a common artefact when GPS data is unavailable."
                        .into(),
                    severity: Severity::Medium,
                    category: "gps".into(),
                });
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            findings.push(AnomalyFinding {
                check_id: "gps_partial".into(),
                title: "Partial GPS data".into(),
                description: "Only latitude or longitude is present, not both. \
                              GPS coordinates should always appear as a pair."
                    .into(),
                severity: Severity::Low,
                category: "gps".into(),
            });
        }
        (None, None) => {}
    }
}

// ===== Injection detection =====
//
// Sub-checks under the exif_anomaly detector that identify suspiciously
// templated, programmatically written, or "reconstructed" EXIF blocks.
// See docs/design/exif-injection-detection.md for rationale.

/// Imaging-library and batch-pipeline names that appear in the EXIF Software
/// field when a file has been assembled by a script rather than exported by
/// camera firmware or a GUI editor. Distinct from `METADATA_TOOLS` (which
/// rewrite metadata without touching pixels) and `IMAGE_EDITORS` (user-facing
/// desktop applications).
const PIPELINE_LIBRARIES: &[&str] = &[
    "pillow",
    "pil/",
    "pil ",
    "python imaging library",
    "imagemagick",
    "graphicsmagick",
    "magick",
    "opencv",
    "skimage",
    "scikit-image",
    "photopea",
    "libvips",
    "sharp/",
    "sharp ",
    "node-sharp",
];

/// Canonical "tutorial" / script-template timestamp literals. Values that
/// appear in `exiftool` documentation, public injection scripts, and
/// placeholder defaults.
const CANONICAL_TEMPLATE_TIMESTAMPS: &[&str] = &[
    "2000:01:01 00:00:00",
    "2020:01:01 00:00:00",
    "2021:01:01 00:00:00",
    "2022:01:01 00:00:00",
    "2023:01:01 00:00:00",
    "2024:01:01 00:00:00",
    "2024:01:01 12:00:00",
    "2025:01:01 00:00:00",
    "1970:01:01 00:00:00",
];

/// Camera vendors whose firmware has been observed to write a MakerNote on
/// every capture. Smartphone vendors (Google, Samsung) are deliberately
/// excluded because sharing-platform re-encoding routinely strips MakerNote
/// from their output. Kept as a subset of `KNOWN_CAMERA_VENDORS` in
/// `metadata.rs` to avoid drift.
const MAKERNOTE_MANDATORY_VENDORS: &[&str] = &[
    "apple",
    "canon",
    "nikon",
    "sony",
    "fujifilm",
    "olympus",
    "om digital",
    "panasonic",
    "leica",
    "hasselblad",
    "phase one",
    "ricoh",
    "pentax",
];

/// Class A — Programmatic pipeline library named in the Software field.
///
/// High severity. Genuine camera firmware and the major GUI editors write
/// their own product name, not the underlying library they bundle. A file
/// whose Software field literally says `Pillow` or `ImageMagick` was assembled
/// by a script.
fn check_pipeline_library_software(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let Some(software) = &meta.software else {
        return;
    };
    let lower = software.to_lowercase();
    for pattern in PIPELINE_LIBRARIES {
        if lower.contains(pattern) {
            findings.push(AnomalyFinding {
                check_id: "software_pipeline_library".into(),
                title: "Imaging pipeline library in Software field".into(),
                description: format!(
                    "Software field contains '{software}', a programmatic imaging library. \
                     Genuine camera firmware and desktop editors write their own product name. \
                     A library name here indicates the file was assembled or re-encoded by a script \
                     rather than produced directly by a camera."
                ),
                severity: Severity::High,
                category: "injection".into(),
            });
            return;
        }
    }
}

/// Class B — Templated or suspiciously round timestamps.
///
/// Fires on two sub-patterns:
///   - **canonical template**: either timestamp matches a known
///     tutorial / script-template literal → Medium severity
///   - **identical zero-second pair**: original == modified AND both end in
///     `:00` seconds → Low severity
fn check_templated_timestamps(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let original = meta.datetime_original.as_deref();
    let modified = meta.datetime_modified.as_deref();

    // Canonical template match — either field hitting a known literal.
    if let Some(ts) = original {
        if CANONICAL_TEMPLATE_TIMESTAMPS.contains(&ts) {
            findings.push(AnomalyFinding {
                check_id: "timestamp_canonical_template".into(),
                title: "Canonical template timestamp detected".into(),
                description: format!(
                    "Original datetime '{ts}' matches a commonly used placeholder / tutorial value. \
                     Genuine captures rarely land on these literals."
                ),
                severity: Severity::Medium,
                category: "injection".into(),
            });
            return;
        }
    }
    if let Some(ts) = modified {
        if CANONICAL_TEMPLATE_TIMESTAMPS.contains(&ts) {
            findings.push(AnomalyFinding {
                check_id: "timestamp_canonical_template".into(),
                title: "Canonical template timestamp detected".into(),
                description: format!(
                    "Modified datetime '{ts}' matches a commonly used placeholder / tutorial value."
                ),
                severity: Severity::Medium,
                category: "injection".into(),
            });
            return;
        }
    }

    // Identical zero-second pair — both fields present, equal, and ending `:00`.
    if let (Some(o), Some(m)) = (original, modified) {
        if o == m && o.ends_with(":00") && o.len() >= 19 {
            findings.push(AnomalyFinding {
                check_id: "timestamp_templated_identical".into(),
                title: "Identical zero-second original and modified timestamps".into(),
                description: format!(
                    "Both original and modified timestamps are '{o}' — identical to the second \
                     and ending on a zero-second boundary. This pattern is characteristic of a \
                     scripted EXIF write rather than a natural camera capture."
                ),
                severity: Severity::Low,
                category: "injection".into(),
            });
        }
    }
}

/// Class C — GPS coordinates at exact integer degrees on both axes.
///
/// Real consumer GPS chips produce sub-integer residuals well beyond
/// decimal place 6. Two independent integer values on the same capture is
/// characteristic of hand-written or mocked data. `(0, 0)` is already handled
/// by `check_gps` as `gps_null_island`.
fn check_integer_degree_gps(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let (Some(lat), Some(lon)) = (meta.gps_latitude, meta.gps_longitude) else {
        return;
    };
    // Skip Null Island — already flagged elsewhere at Medium severity.
    if lat.abs() < 0.01 && lon.abs() < 0.01 {
        return;
    }
    // Reject invalid coordinates — already flagged by check_gps.
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return;
    }
    let lat_int = lat.fract().abs() < 1e-6;
    let lon_int = lon.fract().abs() < 1e-6;
    if lat_int && lon_int {
        findings.push(AnomalyFinding {
            check_id: "gps_integer_degrees".into(),
            title: "GPS coordinates at exact integer degrees".into(),
            description: format!(
                "Latitude {lat:.1} and longitude {lon:.1} are both exact integer values. \
                 Consumer GPS chips produce residuals beyond the sixth decimal place; exact \
                 integers on both axes suggest manually entered or templated coordinates."
            ),
            severity: Severity::Medium,
            category: "injection".into(),
        });
    }
}

/// Class D — MakerNote absent on a vendor that always writes one.
///
/// Compound signal: the claimed camera make is a brand whose firmware always
/// writes a MakerNote, yet the parsed MakerNote is absent or trivially small.
/// Smartphone vendors are deliberately excluded because sharing-platform
/// re-encoding strips MakerNote routinely. This honours the project-level
/// constraint "do not fire on MakerNote absence alone".
fn check_mandatory_maker_note_missing(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let Some(make) = meta.camera_make.as_deref() else {
        return;
    };
    let make_lower = make.to_lowercase();
    let mandatory = MAKERNOTE_MANDATORY_VENDORS
        .iter()
        .any(|v| make_lower.contains(v));
    if !mandatory {
        return;
    }
    if meta.has_maker_note && meta.maker_note_length >= 16 {
        return;
    }
    findings.push(AnomalyFinding {
        check_id: "maker_note_mandatory_vendor_missing".into(),
        title: "MakerNote missing on a vendor that always writes one".into(),
        description: format!(
            "The Make field declares '{make}', a camera brand whose firmware always writes a \
             proprietary MakerNote block, yet no substantial MakerNote is present. This can \
             legitimately happen when an image has been re-encoded by a sharing platform, but \
             it is also a common signature of reconstructed or injected EXIF."
        ),
        severity: Severity::Medium,
        category: "injection".into(),
    });
}

/// Class E — iPhone declaring sRGB colour space with no MakerNote.
///
/// Modern iPhones (iPhone 7 and later) default to the Display P3 profile.
/// A combination of `Make = Apple`, `Model contains iPhone`, `ColorSpace = sRGB`,
/// and no MakerNote is a narrow but specific signature of hand-rolled EXIF.
fn check_iphone_colour_space_mismatch(meta: &ImageMetadata, findings: &mut Vec<AnomalyFinding>) {
    let Some(make) = meta.camera_make.as_deref() else {
        return;
    };
    let Some(model) = meta.camera_model.as_deref() else {
        return;
    };
    let Some(cs) = meta.color_space.as_deref() else {
        return;
    };
    if !make.to_lowercase().contains("apple") {
        return;
    }
    if !model.to_lowercase().contains("iphone") {
        return;
    }
    if !cs.eq_ignore_ascii_case("srgb") {
        return;
    }
    if meta.has_maker_note && meta.maker_note_length >= 16 {
        return;
    }
    findings.push(AnomalyFinding {
        check_id: "iphone_colour_space_mismatch".into(),
        title: "iPhone declared with sRGB and no MakerNote".into(),
        description: format!(
            "Camera make is '{make}' and model is '{model}', yet the colour space is sRGB and no \
             MakerNote is present. Modern iPhones default to Display P3 and always write a \
             MakerNote. This combination is a narrow but specific signature of hand-rolled EXIF."
        ),
        severity: Severity::Low,
        category: "injection".into(),
    });
}

fn compute_completeness(meta: &ImageMetadata) -> (u32, u32) {
    let mut count = 0u32;
    if meta.camera_make.is_some() {
        count += 1;
    }
    if meta.camera_model.is_some() {
        count += 1;
    }
    if meta.software.is_some() {
        count += 1;
    }
    if meta.datetime_original.is_some() {
        count += 1;
    }
    if meta.datetime_modified.is_some() {
        count += 1;
    }
    if meta.exif_width.is_some() {
        count += 1;
    }
    if meta.exif_height.is_some() {
        count += 1;
    }
    if meta.color_space.is_some() {
        count += 1;
    }
    if meta.gps_latitude.is_some() {
        count += 1;
    }
    if meta.gps_longitude.is_some() {
        count += 1;
    }
    if meta.iso.is_some() {
        count += 1;
    }
    if meta.focal_length.is_some() {
        count += 1;
    }
    if meta.exposure_time.is_some() {
        count += 1;
    }
    if meta.f_number.is_some() {
        count += 1;
    }
    if meta.copyright.is_some() {
        count += 1;
    }
    if meta.artist.is_some() {
        count += 1;
    }
    (count, 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_meta() -> ImageMetadata {
        ImageMetadata {
            camera_make: None,
            camera_model: None,
            software: None,
            datetime_original: None,
            datetime_modified: None,
            exif_width: None,
            exif_height: None,
            color_space: None,
            gps_latitude: None,
            gps_longitude: None,
            iso: None,
            focal_length: None,
            exposure_time: None,
            f_number: None,
            copyright: None,
            artist: None,
            description: None,
            orientation: None,
            has_maker_note: false,
            maker_note_length: 0,
        }
    }

    fn camera_meta() -> ImageMetadata {
        ImageMetadata {
            camera_make: Some("Canon".into()),
            camera_model: Some("EOS R5".into()),
            software: Some("Canon EOS R5 Firmware 1.8.1".into()),
            datetime_original: Some("2026:01:15 10:30:00".into()),
            datetime_modified: Some("2026:01:15 10:30:00".into()),
            exif_width: Some(8192),
            exif_height: Some(5464),
            color_space: Some("sRGB".into()),
            gps_latitude: Some(51.5074),
            gps_longitude: Some(-0.1278),
            iso: Some(400),
            focal_length: Some("85 mm".into()),
            exposure_time: Some("1/250".into()),
            f_number: Some("f/2.8".into()),
            copyright: Some("(C) Photographer".into()),
            artist: Some("Jane Doe".into()),
            description: None,
            orientation: Some(1),
            has_maker_note: true,
            maker_note_length: 2048,
        }
    }

    // ── No metadata ─────────────────────────────────────────────────

    #[test]
    fn no_metadata_returns_high_finding() {
        let result = analyse(None, None, None);
        assert!(!result.has_exif);
        assert_eq!(result.fields_populated, 0);
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].severity, Severity::High);
        assert!(result.trust_score < 1.0);
    }

    // ── Clean camera metadata ───────────────────────────────────────

    #[test]
    fn clean_camera_metadata_high_score() {
        let meta = camera_meta();
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert!(result.has_exif);
        assert!(
            result.trust_score >= 0.9,
            "Expected high score, got {}",
            result.trust_score
        );
        assert_eq!(result.fields_populated, 16); // description is None but all others present
    }

    // ── Software detection ──────────────────────────────────────────

    #[test]
    fn ai_software_critical_severity() {
        let mut meta = empty_meta();
        meta.software = Some("DALL-E 3".into());
        let result = analyse(Some(&meta), None, None);
        let ai_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "software_ai_generator");
        assert!(ai_finding.is_some());
        assert_eq!(ai_finding.unwrap().severity, Severity::Critical);
    }

    #[test]
    fn editor_software_low_severity() {
        let mut meta = camera_meta();
        meta.software = Some("Adobe Photoshop 25.0".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let ed_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "software_editor");
        assert!(ed_finding.is_some());
        assert_eq!(ed_finding.unwrap().severity, Severity::Low);
    }

    #[test]
    fn metadata_tool_high_severity() {
        let mut meta = camera_meta();
        meta.software = Some("ExifTool 12.70".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let tool_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "software_metadata_tool");
        assert!(tool_finding.is_some());
        assert_eq!(tool_finding.unwrap().severity, Severity::High);
    }

    #[test]
    fn lightroom_info_only() {
        let mut meta = camera_meta();
        meta.software = Some("Adobe Lightroom Classic 13.0".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let lr_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "software_raw_processor");
        assert!(lr_finding.is_some());
        assert_eq!(lr_finding.unwrap().severity, Severity::Info);
    }

    // ── Timestamp checks ────────────────────────────────────────────

    #[test]
    fn modified_before_original() {
        let mut meta = camera_meta();
        meta.datetime_original = Some("2026:06:15 10:00:00".into());
        meta.datetime_modified = Some("2025:01:01 10:00:00".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let ts_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "timestamp_modified_before_original");
        assert!(ts_finding.is_some());
        assert_eq!(ts_finding.unwrap().severity, Severity::High);
    }

    #[test]
    fn future_timestamp() {
        let mut meta = camera_meta();
        meta.datetime_original = Some("2030:01:01 00:00:00".into());
        meta.datetime_modified = Some("2030:01:01 00:00:00".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let ts_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "timestamp_future");
        assert!(ts_finding.is_some());
        assert_eq!(ts_finding.unwrap().severity, Severity::High);
    }

    #[test]
    fn large_datetime_gap() {
        let mut meta = camera_meta();
        meta.datetime_original = Some("2023:01:01 10:00:00".into());
        meta.datetime_modified = Some("2026:01:15 10:30:00".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let gap_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "timestamp_large_gap");
        assert!(gap_finding.is_some());
        assert_eq!(gap_finding.unwrap().severity, Severity::Medium);
    }

    // ── Dimension checks ────────────────────────────────────────────

    #[test]
    fn dimension_mismatch() {
        let meta = camera_meta(); // EXIF: 8192x5464
        let result = analyse(Some(&meta), Some(800), Some(600));
        let dim_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "dimension_mismatch");
        assert!(dim_finding.is_some());
        assert_eq!(dim_finding.unwrap().severity, Severity::Medium);
    }

    #[test]
    fn orientation_swap_no_finding() {
        let mut meta = camera_meta();
        meta.orientation = Some(6); // 90° rotation swaps w/h
                                    // EXIF says 8192x5464, actual is 5464x8192 (swapped)
        let result = analyse(Some(&meta), Some(5464), Some(8192));
        let dim_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "dimension_mismatch");
        assert!(dim_finding.is_none());
    }

    // ── GPS checks ──────────────────────────────────────────────────

    #[test]
    fn null_island() {
        let mut meta = camera_meta();
        meta.gps_latitude = Some(0.001);
        meta.gps_longitude = Some(-0.002);
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let gps_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "gps_null_island");
        assert!(gps_finding.is_some());
        assert_eq!(gps_finding.unwrap().severity, Severity::Medium);
    }

    #[test]
    fn out_of_range_gps() {
        let mut meta = camera_meta();
        meta.gps_latitude = Some(200.0);
        meta.gps_longitude = Some(0.0);
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let gps_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "gps_out_of_range");
        assert!(gps_finding.is_some());
        assert_eq!(gps_finding.unwrap().severity, Severity::High);
    }

    #[test]
    fn partial_gps() {
        let mut meta = camera_meta();
        meta.gps_latitude = Some(51.5);
        meta.gps_longitude = None;
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let gps_finding = result.findings.iter().find(|f| f.check_id == "gps_partial");
        assert!(gps_finding.is_some());
        assert_eq!(gps_finding.unwrap().severity, Severity::Low);
    }

    // ── Completeness ────────────────────────────────────────────────

    #[test]
    fn completeness_score() {
        let meta = empty_meta();
        let result = analyse(Some(&meta), None, None);
        assert_eq!(result.fields_populated, 0);
        assert_eq!(result.fields_total, 16);

        let full_meta = camera_meta();
        let result2 = analyse(Some(&full_meta), Some(8192), Some(5464));
        assert_eq!(result2.fields_populated, 16);
    }

    // ── GPS fields on ExifAnalysis ──────────────────────────────────

    #[test]
    fn gps_fields_propagated_from_metadata() {
        let meta = camera_meta(); // has lat=51.5074, lon=-0.1278
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert_eq!(result.gps_latitude, Some(51.5074));
        assert_eq!(result.gps_longitude, Some(-0.1278));
    }

    #[test]
    fn gps_fields_none_when_absent() {
        let meta = empty_meta(); // no GPS
        let result = analyse(Some(&meta), None, None);
        assert!(result.gps_latitude.is_none());
        assert!(result.gps_longitude.is_none());
    }

    #[test]
    fn gps_fields_none_when_no_exif() {
        let result = analyse(None, None, None);
        assert!(result.gps_latitude.is_none());
        assert!(result.gps_longitude.is_none());
    }

    #[test]
    fn exif_analysis_gps_serializes_correctly() {
        let meta = camera_meta();
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let json = serde_json::to_string(&result).expect("serialization must succeed");
        assert!(json.contains("\"gpsLatitude\""));
        assert!(json.contains("\"gpsLongitude\""));
        assert!(json.contains("51.5074"));
        assert!(json.contains("-0.1278"));
    }

    #[test]
    fn exif_analysis_gps_null_when_absent() {
        let meta = empty_meta();
        let result = analyse(Some(&meta), None, None);
        let json = serde_json::to_string(&result).expect("serialization must succeed");
        // camelCase keys must appear even when null
        assert!(json.contains("\"gpsLatitude\":null"));
        assert!(json.contains("\"gpsLongitude\":null"));
    }

    // ── MakerNote authenticity bonus ──────────────────────────────────

    #[test]
    fn maker_note_present_with_known_vendor_gives_full_bonus() {
        let mut meta = camera_meta(); // Canon, has_maker_note=true, length=2048
        meta.has_maker_note = true;
        meta.maker_note_length = 2048;
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert_eq!(result.camera_authenticity_bonus, 1.0);
        // Authentic finding should be present at Info severity (no deduction)
        let auth_finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "authentic_maker_note");
        assert!(auth_finding.is_some());
        assert_eq!(auth_finding.unwrap().severity, Severity::Info);
    }

    #[test]
    fn maker_note_absent_zero_bonus() {
        let mut meta = camera_meta();
        meta.has_maker_note = false;
        meta.maker_note_length = 0;
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert_eq!(result.camera_authenticity_bonus, 0.0);
    }

    #[test]
    fn maker_note_unknown_vendor_partial_bonus() {
        let mut meta = empty_meta();
        meta.camera_make = Some("ObscureBrand".into());
        meta.has_maker_note = true;
        meta.maker_note_length = 1024;
        let result = analyse(Some(&meta), None, None);
        // Vendor not in known table → 0.4
        assert!((result.camera_authenticity_bonus - 0.4).abs() < 1e-9);
    }

    #[test]
    fn maker_note_global_majority_vendors_recognised() {
        for vendor in &["Tecno", "Infinix", "Itel", "Realme", "Xiaomi"] {
            let mut meta = empty_meta();
            meta.camera_make = Some((*vendor).into());
            meta.has_maker_note = true;
            meta.maker_note_length = 4096;
            let result = analyse(Some(&meta), None, None);
            assert_eq!(
                result.camera_authenticity_bonus, 1.0,
                "Vendor {vendor} should be recognised as a known camera vendor"
            );
        }
    }

    #[test]
    fn maker_note_too_small_zero_bonus() {
        let mut meta = camera_meta();
        meta.has_maker_note = true;
        meta.maker_note_length = 8; // suspiciously small
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert_eq!(result.camera_authenticity_bonus, 0.0);
    }

    #[test]
    fn camera_authenticity_bonus_serializes_camelcase() {
        let meta = camera_meta();
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let json = serde_json::to_string(&result).expect("serialization must succeed");
        assert!(json.contains("\"cameraAuthenticityBonus\""));
    }

    // ── Injection: pipeline-library software field (Class A) ──────────

    #[test]
    fn pipeline_library_software_flagged_high() {
        let mut meta = camera_meta();
        meta.software = Some("Pillow 10.2.0".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "software_pipeline_library");
        assert!(
            finding.is_some(),
            "Pillow should be flagged as pipeline library"
        );
        assert_eq!(finding.unwrap().severity, Severity::High);
        assert_eq!(finding.unwrap().category, "injection");
    }

    #[test]
    fn pipeline_library_imagemagick_flagged() {
        let mut meta = camera_meta();
        meta.software = Some("ImageMagick 7.1.1-11 Q16".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "software_pipeline_library"));
    }

    #[test]
    fn pipeline_library_clean_camera_software_no_finding() {
        // Edge case: genuine camera firmware should never trigger this rule.
        let meta = camera_meta(); // "Canon EOS R5 Firmware 1.8.1"
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "software_pipeline_library"));
    }

    // ── Injection: templated timestamps (Class B) ─────────────────────

    #[test]
    fn canonical_template_timestamp_flagged_medium() {
        let mut meta = camera_meta();
        meta.datetime_original = Some("2024:01:01 12:00:00".into());
        meta.datetime_modified = Some("2024:01:01 12:00:00".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "timestamp_canonical_template");
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().severity, Severity::Medium);
    }

    #[test]
    fn identical_zero_second_pair_flagged_low() {
        let mut meta = camera_meta();
        meta.datetime_original = Some("2026:03:15 14:22:00".into());
        meta.datetime_modified = Some("2026:03:15 14:22:00".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "timestamp_templated_identical");
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().severity, Severity::Low);
    }

    #[test]
    fn genuine_timestamps_no_injection_finding() {
        // Edge case: original and modified identical but with non-zero seconds → no flag.
        let mut meta = camera_meta();
        meta.datetime_original = Some("2026:03:15 14:22:37".into());
        meta.datetime_modified = Some("2026:03:15 14:22:37".into());
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "timestamp_templated_identical"
                || f.check_id == "timestamp_canonical_template"));
    }

    // ── Injection: integer-degree GPS (Class C) ───────────────────────

    #[test]
    fn integer_degree_gps_flagged() {
        let mut meta = camera_meta();
        meta.gps_latitude = Some(51.0);
        meta.gps_longitude = Some(-1.0);
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "gps_integer_degrees");
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().severity, Severity::Medium);
    }

    #[test]
    fn realistic_gps_no_integer_finding() {
        let meta = camera_meta(); // 51.5074, -0.1278
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "gps_integer_degrees"));
    }

    #[test]
    fn integer_gps_null_island_not_double_flagged() {
        // Edge case: (0, 0) should hit gps_null_island but not gps_integer_degrees.
        let mut meta = camera_meta();
        meta.gps_latitude = Some(0.0);
        meta.gps_longitude = Some(0.0);
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert!(result
            .findings
            .iter()
            .any(|f| f.check_id == "gps_null_island"));
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "gps_integer_degrees"));
    }

    // ── Injection: MakerNote missing on mandatory vendor (Class D) ────

    #[test]
    fn canon_without_maker_note_flagged() {
        let mut meta = camera_meta();
        meta.camera_make = Some("Canon".into());
        meta.has_maker_note = false;
        meta.maker_note_length = 0;
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        let finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "maker_note_mandatory_vendor_missing");
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().severity, Severity::Medium);
    }

    #[test]
    fn canon_with_maker_note_no_injection_finding() {
        let meta = camera_meta(); // has_maker_note=true, length=2048
        let result = analyse(Some(&meta), Some(8192), Some(5464));
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "maker_note_mandatory_vendor_missing"));
    }

    #[test]
    fn pixel_without_maker_note_not_flagged() {
        // Edge case: smartphone vendors (Google Pixel) are deliberately excluded
        // from the mandatory-vendor list because sharing platforms strip them.
        let mut meta = empty_meta();
        meta.camera_make = Some("Google".into());
        meta.camera_model = Some("Pixel 8".into());
        meta.has_maker_note = false;
        let result = analyse(Some(&meta), None, None);
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "maker_note_mandatory_vendor_missing"));
    }

    // ── Injection: iPhone colour space mismatch (Class E) ─────────────

    #[test]
    fn iphone_srgb_no_makernote_flagged() {
        let mut meta = empty_meta();
        meta.camera_make = Some("Apple".into());
        meta.camera_model = Some("iPhone 15 Pro".into());
        meta.color_space = Some("sRGB".into());
        meta.has_maker_note = false;
        let result = analyse(Some(&meta), None, None);
        let finding = result
            .findings
            .iter()
            .find(|f| f.check_id == "iphone_colour_space_mismatch");
        assert!(finding.is_some());
        assert_eq!(finding.unwrap().severity, Severity::Low);
    }

    #[test]
    fn iphone_with_makernote_no_finding() {
        let mut meta = empty_meta();
        meta.camera_make = Some("Apple".into());
        meta.camera_model = Some("iPhone 15 Pro".into());
        meta.color_space = Some("sRGB".into());
        meta.has_maker_note = true;
        meta.maker_note_length = 4096;
        let result = analyse(Some(&meta), None, None);
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "iphone_colour_space_mismatch"));
    }

    #[test]
    fn canon_srgb_no_iphone_finding() {
        // Edge case: rule is iPhone-specific; a Canon with sRGB should not fire.
        let mut meta = empty_meta();
        meta.camera_make = Some("Canon".into());
        meta.camera_model = Some("EOS R5".into());
        meta.color_space = Some("sRGB".into());
        meta.has_maker_note = false;
        let result = analyse(Some(&meta), None, None);
        assert!(!result
            .findings
            .iter()
            .any(|f| f.check_id == "iphone_colour_space_mismatch"));
    }
}
