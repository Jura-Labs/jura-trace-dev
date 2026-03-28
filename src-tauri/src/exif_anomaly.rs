//! EXIF anomaly detection — analyse image metadata for manipulation indicators.
//!
//! Performs six categories of checks: software detection, missing EXIF,
//! timestamp validation, dimension consistency, GPS plausibility, and
//! field completeness. Returns a trust score (0.0–1.0) with findings.

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

    let (fields_populated, fields_total, gps_latitude, gps_longitude) = match metadata {
        Some(meta) => {
            check_software(meta, &mut findings);
            check_missing_exif(meta, &mut findings);
            check_timestamps(meta, &mut findings);
            check_dimensions(meta, actual_width, actual_height, &mut findings);
            check_gps(meta, &mut findings);
            let (fp, ft) = compute_completeness(meta);
            (fp, ft, meta.gps_latitude, meta.gps_longitude)
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
            (0, 16, None, None)
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
    }
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
}
