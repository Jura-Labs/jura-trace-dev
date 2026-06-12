//! Input quality assessment — run before detectors to identify conditions
//! that reduce the reliability of forensic analysis.

use crate::exif_anomaly;
use crate::format_router;
use crate::metadata;
use crate::InputQualityAssessment;

/// Estimate JPEG quality from file size ratio (bytes per pixel).
///
/// This is a rough heuristic — not a precise Q-factor extraction. Empirical
/// mapping: files with very few bytes per pixel are heavily compressed and
/// will degrade ELA, noise, and JPEG ghost detector reliability.
pub(crate) fn estimate_jpeg_quality(
    path: &std::path::Path,
    width: Option<u32>,
    height: Option<u32>,
) -> Option<u8> {
    let file_size = std::fs::metadata(path).map(|m| m.len()).ok()?;
    let pixels = width? as u64 * height? as u64;
    if pixels == 0 {
        return None;
    }

    // Bytes per pixel ratio — empirical mapping to approximate Q-factor
    let bpp = file_size as f64 / pixels as f64;
    let q = if bpp > 3.0 {
        95
    } else if bpp > 2.0 {
        90
    } else if bpp > 1.0 {
        85
    } else if bpp > 0.5 {
        75
    } else if bpp > 0.3 {
        65
    } else if bpp > 0.15 {
        50
    } else if bpp > 0.08 {
        35
    } else {
        20
    };
    Some(q)
}

/// Detect whether an image is likely a screenshot based on aspect ratio
/// and EXIF characteristics.
///
/// Combines three signals: common screenshot aspect ratio, absence of camera
/// EXIF, and a recognised screenshot pixel width. All three must be true to
/// avoid false positives on letterboxed camera photos.
pub(crate) fn detect_screenshot(
    width: Option<u32>,
    height: Option<u32>,
    exif: &Option<exif_anomaly::ExifAnalysis>,
) -> bool {
    let w = width.unwrap_or(0) as f64;
    let h = height.unwrap_or(0) as f64;
    if w == 0.0 || h == 0.0 {
        return false;
    }

    // Common screenshot aspect ratios (phone portrait and desktop landscape)
    let ratio = w / h;
    let is_phone_ratio = (ratio - 9.0 / 16.0).abs() < 0.05
        || (ratio - 9.0 / 19.5).abs() < 0.05
        || (ratio - 9.0 / 20.0).abs() < 0.05;
    let is_desktop_ratio = (ratio - 16.0 / 9.0).abs() < 0.05 || (ratio - 16.0 / 10.0).abs() < 0.05;

    // No camera EXIF = likely screenshot or web-sourced image
    let no_camera = exif.as_ref().is_none_or(|e| !e.has_exif);

    // Common screenshot widths (iOS, Android, standard desktop resolutions)
    let common_width = matches!(
        w as u32,
        750 | 828 | 1080 | 1125 | 1170 | 1242 | 1284 | 1290 | 1920 | 2560 | 2880 | 3840
    );

    (is_phone_ratio || is_desktop_ratio) && no_camera && common_width
}

/// Assess input quality to identify conditions that degrade detector reliability.
///
/// Runs before detector dispatch — adds <5 ms to the pipeline. Returns an
/// `InputQualityAssessment` containing the resolution category, JPEG quality
/// estimate, screenshot likelihood, modern-codec flag, and a list of detectors
/// whose results should be treated with reduced confidence for this input.
///
/// `raw_meta` is the raw EXIF/XMP extraction result — passed through so that
/// the XMP packet can be inspected for the `metadata_completely_absent` check
/// even when kamadak-exif found no EXIF fields.
pub(crate) fn assess_input_quality(
    path: &std::path::Path,
    info: &format_router::FormatInfo,
    exif: &Option<exif_anomaly::ExifAnalysis>,
    raw_meta: Option<&metadata::ImageMetadata>,
    width: Option<u32>,
    height: Option<u32>,
) -> InputQualityAssessment {
    let is_jpeg = info.mime_type == "image/jpeg";
    let is_image = info.content_type == format_router::ContentType::Image;

    // Modern lossy codec detection — AVIF (AV1 intra-frame) and WebP (VP8/VP8L)
    // re-quantise uniformly on encode, destroying differential ELA/noise signal.
    // Both formats aggressively strip metadata in typical web delivery pipelines.
    let is_modern_lossy_codec = matches!(info.mime_type.as_str(), "image/avif" | "image/webp");

    // JPEG quality estimation from file size heuristic
    let jpeg_quality_estimate = if is_jpeg {
        estimate_jpeg_quality(path, width, height)
    } else {
        None
    };

    // Resolution category
    let pixels = width.unwrap_or(0) as u64 * height.unwrap_or(0) as u64;
    let resolution_category = if !is_image {
        "n/a".to_string()
    } else if width.unwrap_or(0) < 128 || height.unwrap_or(0) < 128 {
        "thumbnail".to_string()
    } else if pixels < 500_000 {
        "low".to_string()
    } else if pixels < 2_000_000 {
        "medium".to_string()
    } else {
        "high".to_string()
    };

    // Screenshot detection heuristic
    let is_screenshot_likely = if is_image {
        detect_screenshot(width, height, exif)
    } else {
        false
    };

    // EXIF GPS/timestamp presence
    let has_exif = exif.as_ref().is_some_and(|e| e.has_exif);
    let has_gps = exif
        .as_ref()
        .is_some_and(|e| e.gps_latitude.is_some() && e.gps_longitude.is_some());
    let has_timestamp = has_exif;

    // Metadata-completely-absent: no EXIF AND no XMP data.
    // Delegates to exif_anomaly for XMP emptiness logic.
    let metadata_completely_absent =
        exif_anomaly::check_metadata_completely_absent(has_exif, raw_meta.map(|m| &m.xmp))
            .is_some();

    // Build degraded detectors list
    let mut degraded = Vec::new();

    if let Some(q) = jpeg_quality_estimate {
        if q < 40 {
            degraded.push("ELA".to_string());
            degraded.push("Noise Analysis".to_string());
            degraded.push("JPEG Ghost".to_string());
        }
    }

    if resolution_category == "low" || resolution_category == "thumbnail" {
        degraded.push("Deepfake Detection".to_string());
        degraded.push("Copy-Move Detection".to_string());
        degraded.push("Segmented ELA".to_string());
    }

    if is_screenshot_likely {
        degraded.push("EXIF Anomaly".to_string());
        degraded.push("JPEG Ghost".to_string());
    }

    if !is_jpeg {
        degraded.push("JPEG Ghost".to_string());
    }

    if is_modern_lossy_codec {
        degraded.push("ELA".to_string());
        degraded.push("Noise Analysis".to_string());
        degraded.push("Copy-Move Detection".to_string());
        degraded.push("JPEG Ghost".to_string());
    }

    if !has_gps || !has_timestamp {
        degraded.push("Sun Position".to_string());
        degraded.push("Shadow Time Estimation".to_string());
    }

    // Deduplicate
    degraded.sort();
    degraded.dedup();

    InputQualityAssessment {
        jpeg_quality_estimate,
        resolution_category,
        width,
        height,
        is_screenshot_likely,
        is_jpeg,
        has_gps,
        has_timestamp,
        is_modern_lossy_codec,
        metadata_completely_absent,
        degraded_detectors: degraded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AVIF files must set `is_modern_lossy_codec = true` and add the four
    /// codec-degraded detectors to `degraded_detectors`.
    #[test]
    fn input_quality_avif_sets_modern_lossy_codec() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // A minimal placeholder file — assess_input_quality only reads the
        // MIME type from FormatInfo, not the actual file bytes.
        let fake_avif = tmp.path().join("test.avif");
        std::fs::write(&fake_avif, b"fake avif bytes for size heuristic only")
            .expect("write fake avif");

        let info = format_router::FormatInfo {
            mime_type: "image/avif".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif: Option<exif_anomaly::ExifAnalysis> = None;
        let quality = assess_input_quality(&fake_avif, &info, &exif, None, Some(1000), Some(800));

        assert!(
            quality.is_modern_lossy_codec,
            "AVIF must set is_modern_lossy_codec = true"
        );
        assert!(
            quality.degraded_detectors.contains(&"ELA".to_string()),
            "ELA must be degraded for AVIF"
        );
        assert!(
            quality
                .degraded_detectors
                .contains(&"JPEG Ghost".to_string()),
            "JPEG Ghost must be degraded for AVIF"
        );
    }

    /// JPEG files must NOT set `is_modern_lossy_codec`.
    #[test]
    fn input_quality_jpeg_not_modern_lossy_codec() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_jpeg = tmp.path().join("test.jpg");
        std::fs::write(&fake_jpeg, b"fake jpeg bytes for size heuristic only")
            .expect("write fake jpeg");

        let info = format_router::FormatInfo {
            mime_type: "image/jpeg".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif: Option<exif_anomaly::ExifAnalysis> = None;
        let quality = assess_input_quality(&fake_jpeg, &info, &exif, None, Some(2000), Some(1500));

        assert!(
            !quality.is_modern_lossy_codec,
            "JPEG must not set is_modern_lossy_codec"
        );
    }

    /// WebP files must also set `is_modern_lossy_codec = true`.
    #[test]
    fn input_quality_webp_sets_modern_lossy_codec() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_webp = tmp.path().join("test.webp");
        std::fs::write(&fake_webp, b"RIFF\x00\x00\x00\x00WEBPVP8 ").expect("write fake webp");

        let info = format_router::FormatInfo {
            mime_type: "image/webp".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif: Option<exif_anomaly::ExifAnalysis> = None;
        let quality = assess_input_quality(&fake_webp, &info, &exif, None, Some(800), Some(600));

        assert!(
            quality.is_modern_lossy_codec,
            "WebP must set is_modern_lossy_codec = true"
        );
    }

    // ── assess_input_quality — metadata_completely_absent ────────────────

    /// When has_exif = false and no raw_meta is provided, metadata_completely_absent
    /// must be true.
    #[test]
    fn input_quality_metadata_absent_when_no_exif_no_raw_meta() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_avif = tmp.path().join("stripped.avif");
        std::fs::write(&fake_avif, b"fake avif content").expect("write");

        let info = format_router::FormatInfo {
            mime_type: "image/avif".to_string(),
            content_type: format_router::ContentType::Image,
        };
        // has_exif = false (ExifAnalysis.has_exif driven by has_exif field)
        let exif_analysis = Some(exif_anomaly::ExifAnalysis {
            findings: vec![],
            trust_score: 0.8,
            fields_populated: 0,
            fields_total: 16,
            has_exif: false,
            gps_latitude: None,
            gps_longitude: None,
            camera_authenticity_bonus: 0.0,
            is_known_camera_make: false,
        });
        // No raw_meta → XMP treated as absent
        let quality = assess_input_quality(
            &fake_avif,
            &info,
            &exif_analysis,
            None,
            Some(800),
            Some(600),
        );

        assert!(
            quality.metadata_completely_absent,
            "metadata_completely_absent must be true when EXIF and XMP are both absent"
        );
    }

    /// When EXIF is present, metadata_completely_absent must be false.
    #[test]
    fn input_quality_metadata_not_absent_when_exif_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fake_jpeg = tmp.path().join("with_exif.jpg");
        std::fs::write(&fake_jpeg, b"fake jpeg content").expect("write");

        let info = format_router::FormatInfo {
            mime_type: "image/jpeg".to_string(),
            content_type: format_router::ContentType::Image,
        };
        let exif_analysis = Some(exif_anomaly::ExifAnalysis {
            findings: vec![],
            trust_score: 0.9,
            fields_populated: 12,
            fields_total: 16,
            has_exif: true,
            gps_latitude: Some(51.5),
            gps_longitude: Some(-0.1),
            camera_authenticity_bonus: 0.8,
            is_known_camera_make: true,
        });
        let quality = assess_input_quality(
            &fake_jpeg,
            &info,
            &exif_analysis,
            None,
            Some(4000),
            Some(3000),
        );

        assert!(
            !quality.metadata_completely_absent,
            "metadata_completely_absent must be false when EXIF is present"
        );
    }
}
