//! Filename pattern heuristics for provenance signalling.
//!
//! Camera manufacturers follow predictable naming conventions (DSC, IMG,
//! DSCF, PXL, etc.).  AI generators and screenshots produce very different
//! naming patterns (UUID strings, prompt-derived names, generic "image.jpg").
//! This module classifies filenames into provenance categories and attaches a
//! confidence score and human-readable summary.
//!
//! These are provenance *signals*, not forensic detectors. They are additive
//! context that lives in the provenance section of the UI, not the trust score.

use serde::{Deserialize, Serialize};

/// Filename provenance analysis result.
///
/// All fields use camelCase for IPC serialisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilenameAnalysis {
    /// Provenance category:
    /// - `"camera"` — matches a known camera naming convention
    /// - `"screenshot"` — matches screenshot naming patterns
    /// - `"ai_generated"` — UUID, prompt-like, or AI-tool pattern
    /// - `"web_download"` — generic web download name
    /// - `"edited"` — edited/export suffix detected
    /// - `"unknown"` — no pattern matched
    pub pattern: String,
    /// Confidence in the classification (0.0–1.0).
    pub confidence: f64,
    /// Human-readable description of the matched pattern, if any.
    /// e.g. `"DSC_NNNN.JPG (Sony / Nikon)"`.
    pub matched_pattern: Option<String>,
    /// Short human-readable summary for the UI provenance panel.
    pub summary: String,
}

/// Analyse the filename component of a path and return a [`FilenameAnalysis`].
///
/// Only the file stem (name without directory or extension) is examined;
/// the extension is available as secondary context for some heuristics.
pub fn analyse_filename(path: &std::path::Path) -> FilenameAnalysis {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    classify(&file_name, &stem, &ext)
}

/// Core classification logic — separated for unit testing with synthetic inputs.
pub(crate) fn classify(file_name: &str, stem: &str, ext: &str) -> FilenameAnalysis {
    let stem_upper = stem.to_uppercase();
    let stem_lower = stem.to_lowercase();
    let name_lower = file_name.to_lowercase();

    // ── Camera patterns ───────────────────────────────────────────────────

    // Google Pixel: PXL_YYYYMMDD_HHMMSSMMM (e.g. PXL_20231015_143025123)
    if matches_regex_like(&stem_upper, "PXL_", 4, &[8, b'_'], 9, 9) {
        return camera("camera", 0.95, "PXL_YYYYMMDD_HHMMSS (Google Pixel)");
    }

    // Sony / Nikon: DSC_NNNN or DSC_NNNNN or DSCNNNNN
    if (stem_upper.starts_with("DSC_") && stem[4..].chars().all(|c| c.is_ascii_digit()))
        || (stem_upper.starts_with("DSC") && stem[3..].chars().all(|c| c.is_ascii_digit()))
    {
        return camera("camera", 0.92, "DSC_NNNN / DSCNNNNN (Sony / Nikon)");
    }

    // Fujifilm: DSCF + digits
    if stem_upper.starts_with("DSCF") && stem[4..].chars().all(|c| c.is_ascii_digit()) {
        return camera("camera", 0.92, "DSCFNNNN (Fujifilm)");
    }

    // Canon: _MG_NNNN (EOS series) or IMG_NNNN
    if stem_upper.starts_with("_MG_") && stem[4..].chars().all(|c| c.is_ascii_digit()) {
        return camera("camera", 0.92, "_MG_NNNN (Canon EOS)");
    }

    // Apple iPhone / Samsung / generic: IMG_NNNN or IMG_NNNNN
    if stem_upper.starts_with("IMG_") && stem[4..].chars().all(|c| c.is_ascii_digit()) {
        return camera("camera", 0.85, "IMG_NNNN (Apple iPhone / Samsung / generic)");
    }

    // Panasonic Lumix: P + 7 digits, or PA + 6 digits
    if stem_upper.starts_with("PA")
        && stem[2..].chars().all(|c| c.is_ascii_digit())
        && stem.len() == 8
    {
        return camera("camera", 0.88, "PANNNNNN (Panasonic / Olympus)");
    }
    if stem_upper.starts_with('P')
        && stem[1..].chars().all(|c| c.is_ascii_digit())
        && stem.len() == 8
    {
        return camera("camera", 0.85, "PNNNNNNN (Panasonic / Olympus)");
    }

    // DJI drones: DJI_NNNN or DJI_YYYYMMDD_HHMMSS
    if stem_upper.starts_with("DJI_") {
        return camera("camera", 0.90, "DJI_NNNN (DJI drone)");
    }

    // GoPro: GOPR + digits, GP + digits, GOPRONNNN
    if stem_upper.starts_with("GOPR") && stem[4..].chars().all(|c| c.is_ascii_digit()) {
        return camera("camera", 0.93, "GOPRNNNN (GoPro)");
    }

    // Nikon high-end: _DSC + digits
    if stem_upper.starts_with("_DSC") && stem[4..].chars().all(|c| c.is_ascii_digit()) {
        return camera("camera", 0.92, "_DSCNNNN (Nikon)");
    }

    // Samsung: SAM_ or SAMSUNG_ prefix
    if stem_upper.starts_with("SAM_") || stem_upper.starts_with("SAMSUNG_") {
        return camera("camera", 0.88, "SAM_NNNN (Samsung)");
    }

    // ── Screenshot patterns ───────────────────────────────────────────────

    // macOS: "Screenshot YYYY-MM-DD at HH.MM.SS" or "Screen Shot YYYY-MM-DD at..."
    if name_lower.starts_with("screenshot") || name_lower.starts_with("screen shot") {
        return FilenameAnalysis {
            pattern: "screenshot".to_string(),
            confidence: 0.97,
            matched_pattern: Some("Screenshot / Screen Shot prefix (macOS / iOS)".to_string()),
            summary: "Filename matches macOS or iOS screenshot naming convention.".to_string(),
        };
    }

    // Windows Snipping Tool / Win+Shift+S: often just "image" or "Snip" or
    // timestamp-only strings like "2023-10-15 14-30-25.png"
    if name_lower.starts_with("snip") || name_lower.starts_with("capture") {
        return FilenameAnalysis {
            pattern: "screenshot".to_string(),
            confidence: 0.88,
            matched_pattern: Some("Snip / Capture prefix (Windows screenshot tools)".to_string()),
            summary: "Filename matches Windows screenshot tool naming convention.".to_string(),
        };
    }

    // ── Social media platform filename patterns ──────────────────────────

    // Twitter/X: base62-encoded media IDs, typically 15 chars alphanumeric
    // e.g. "HDddUFtWkAAZgyM", "E9kPjK1VUAAI3Jn"
    if stem.len() >= 12
        && stem.len() <= 20
        && stem.chars().all(|c| c.is_ascii_alphanumeric())
        && stem.chars().any(|c| c.is_ascii_uppercase())
        && stem.chars().any(|c| c.is_ascii_lowercase())
        && !stem.starts_with("IMG")
        && !stem.starts_with("DSC")
        && !stem.starts_with("PXL")
    {
        return FilenameAnalysis {
            pattern: "social_media".to_string(),
            confidence: 0.70,
            matched_pattern: Some("Twitter/X base62 media ID".to_string()),
            summary:
                "Filename matches Twitter/X media ID pattern — this image was likely \
                 downloaded from or shared via Twitter/X."
                    .to_string(),
        };
    }

    // Facebook: numeric IDs, typically 15-19 digits
    // e.g. "123456789012345"
    if stem.len() >= 15
        && stem.len() <= 20
        && stem.chars().all(|c| c.is_ascii_digit())
    {
        return FilenameAnalysis {
            pattern: "social_media".to_string(),
            confidence: 0.65,
            matched_pattern: Some("Facebook numeric media ID".to_string()),
            summary:
                "Filename is a long numeric ID — typical of Facebook/Instagram media downloads."
                    .to_string(),
        };
    }

    // ── AI-generated / UUID patterns ─────────────────────────────────────

    // UUID v4: 8-4-4-4-12 hex pattern
    if is_uuid_like(stem) {
        return FilenameAnalysis {
            pattern: "ai_generated".to_string(),
            confidence: 0.75,
            matched_pattern: Some("UUID / hash filename (AI generator or web download)".to_string()),
            summary:
                "Filename is a UUID or hash string — typical of AI image generators, CDN \
                 downloads, and web content management pipelines."
                    .to_string(),
        };
    }

    // Long hex strings (32–64 chars) — common from stable diffusion outputs, DALL-E
    if stem.len() >= 32
        && stem.len() <= 64
        && stem.chars().all(|c| c.is_ascii_hexdigit() || c == '-' || c == '_')
    {
        return FilenameAnalysis {
            pattern: "ai_generated".to_string(),
            confidence: 0.70,
            matched_pattern: Some("Long hex / hash filename (AI generator output)".to_string()),
            summary: "Filename is a long hexadecimal string — common in AI generator outputs."
                .to_string(),
        };
    }

    // Known AI generator output patterns
    // Midjourney: long underscore-separated descriptive names with trailing UUID
    if stem_lower.contains("midjourney")
        || stem_lower.contains("dalle")
        || stem_lower.contains("dall-e")
        || stem_lower.contains("stable-diffusion")
        || stem_lower.contains("stablediffusion")
        || stem_lower.contains("dreamstudio")
        || stem_lower.contains("firefly")
        || stem_lower.contains("nightcafe")
        || stem_lower.contains("artbreeder")
        || stem_lower.contains("generated")
        || stem_lower.contains("ai-generated")
    {
        return FilenameAnalysis {
            pattern: "ai_generated".to_string(),
            confidence: 0.90,
            matched_pattern: Some("Known AI generator name in filename".to_string()),
            summary: "Filename contains a known AI image generator name.".to_string(),
        };
    }

    // Midjourney output format: very long, underscore-separated, ends with 4-digit index
    // e.g. "a_sunset_over_the_mountains_highly_detailed_8k_1234"
    if stem.len() > 40 && stem.contains('_') && !stem.contains(' ') && ext == "webp" {
        let last_segment = stem.rsplit('_').next().unwrap_or("");
        if last_segment.len() == 4 && last_segment.chars().all(|c| c.is_ascii_digit()) {
            return FilenameAnalysis {
                pattern: "ai_generated".to_string(),
                confidence: 0.80,
                matched_pattern: Some(
                    "Long underscore-separated name with numeric suffix + .webp (Midjourney-like)"
                        .to_string(),
                ),
                summary: "Filename pattern is consistent with Midjourney output.".to_string(),
            };
        }
    }

    // ── Edited / exported file patterns ──────────────────────────────────

    // Common edit suffixes: -edit, -edited, _edit, _copy, (1), copy, -final, -v2
    let edit_keywords = [
        "-edit", "_edit", "-edited", "_edited", "-copy", "_copy",
        "-final", "_final", "-v2", "_v2", "-export", "_export",
        "-processed", "_processed",
    ];
    for kw in &edit_keywords {
        if stem_lower.ends_with(kw)
            || stem_lower.contains(&format!("{kw}_"))
            || stem_lower.contains(&format!("{kw}-"))
        {
            return FilenameAnalysis {
                pattern: "edited".to_string(),
                confidence: 0.72,
                matched_pattern: Some(format!("Edit suffix '{kw}' in filename")),
                summary: format!(
                    "Filename contains an edit indicator ('{kw}'), suggesting this file is \
                     a derivative or exported copy."
                ),
            };
        }
    }

    // Windows copy pattern: "Filename (N)" where N is a digit
    if stem.ends_with(')')
        && stem.contains(" (")
        && stem.rsplit(" (").next().unwrap_or("").trim_end_matches(')').chars().all(|c| c.is_ascii_digit())
    {
        return FilenameAnalysis {
            pattern: "edited".to_string(),
            confidence: 0.65,
            matched_pattern: Some("Windows 'Filename (N)' copy pattern".to_string()),
            summary: "Filename matches the Windows file-copy naming pattern.".to_string(),
        };
    }

    // ── Generic / web download patterns ──────────────────────────────────

    let generic_names = [
        "image", "photo", "picture", "download", "untitled", "file",
        "img", "pic", "thumbnail", "preview",
    ];
    for gn in &generic_names {
        if stem_lower == *gn
            || stem_lower.starts_with(&format!("{gn}_"))
            || stem_lower.starts_with(&format!("{gn}-"))
            || stem_lower.starts_with(&format!("{gn} "))
        {
            return FilenameAnalysis {
                pattern: "web_download".to_string(),
                confidence: 0.55,
                matched_pattern: Some(format!("Generic filename '{gn}*'")),
                summary: format!(
                    "Generic filename starting with '{gn}' — no provenance signal."
                ),
            };
        }
    }

    // Timestamp-only filenames (from some Android phones and apps):
    // YYYYMMDD_HHMMSS or YYYYMMDDHHMMSS
    if is_timestamp_filename(stem) {
        return camera("camera", 0.75, "YYYYMMDD_HHMMSS timestamp (Android / app)");
    }

    // ── Unknown ───────────────────────────────────────────────────────────

    FilenameAnalysis {
        pattern: "unknown".to_string(),
        confidence: 0.0,
        matched_pattern: None,
        summary: "Filename does not match any known provenance pattern.".to_string(),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn camera(pattern: &str, confidence: f64, label: &str) -> FilenameAnalysis {
    FilenameAnalysis {
        pattern: pattern.to_string(),
        confidence,
        matched_pattern: Some(label.to_string()),
        summary: format!("Filename matches the {label} camera naming convention."),
    }
}

/// Rough check for UUID-like strings (lowercase hex groups separated by '-').
fn is_uuid_like(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 5 {
        let lengths = [8, 4, 4, 4, 12];
        parts
            .iter()
            .zip(lengths.iter())
            .all(|(p, &len)| p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit()))
    } else {
        false
    }
}

/// Match patterns like "PXL_YYYYMMDD_HHMMSSMMM" by checking prefix length
/// and suffix digit count without pulling in a regex dependency.
///
/// `prefix_len` is the byte length of the prefix already checked by the
/// caller. `date_len` is the expected digit count for the date segment,
/// `sep` is the expected separator byte, and `time_len` is the expected
/// digit count for the time segment.
fn matches_regex_like(
    s: &str,
    prefix: &str,
    prefix_len: usize,
    sep: &[u8; 2], // [expected_date_len_as_u8_hint, sep_char]
    _date_len: usize,
    time_len: usize,
) -> bool {
    let _ = (prefix_len, sep, time_len); // unused in simplified version
    // Simplified: just check prefix + all-digits-or-underscores remainder
    s.starts_with(prefix)
        && s[prefix.len()..].chars().all(|c| c.is_ascii_digit() || c == '_')
        && s.len() > prefix.len() + 10
}

/// Check whether a stem looks like a camera timestamp filename:
/// YYYYMMDD_HHMMSS (15 chars) or YYYYMMDDHHMMSS (14 chars) or similar.
fn is_timestamp_filename(stem: &str) -> bool {
    let digits_only: String = stem.chars().filter(|c| c.is_ascii_digit()).collect();
    let underscores: usize = stem.chars().filter(|&c| c == '_').count();
    // 14 digits = YYYYMMDDHHMMSS, or 15 chars with one underscore separator
    (digits_only.len() == 14 || digits_only.len() == 15)
        && (underscores == 0 || underscores == 1)
        && stem.len() <= 16
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_sony_nikon_dsc() {
        let r = classify("DSC_1234.jpg", "DSC_1234", "jpg");
        assert_eq!(r.pattern, "camera");
        assert!(r.confidence > 0.9);
    }

    #[test]
    fn camera_fujifilm_dscf() {
        let r = classify("DSCF0078.JPG", "DSCF0078", "jpg");
        assert_eq!(r.pattern, "camera");
    }

    #[test]
    fn camera_apple_img() {
        let r = classify("IMG_4521.heic", "IMG_4521", "heic");
        assert_eq!(r.pattern, "camera");
    }

    #[test]
    fn camera_google_pixel_pxl() {
        let r = classify(
            "PXL_20231015_143025123.jpg",
            "PXL_20231015_143025123",
            "jpg",
        );
        assert_eq!(r.pattern, "camera");
        assert!(r.confidence >= 0.9);
    }

    #[test]
    fn camera_dji_drone() {
        let r = classify("DJI_0042.JPG", "DJI_0042", "jpg");
        assert_eq!(r.pattern, "camera");
    }

    #[test]
    fn screenshot_macos() {
        let r = classify(
            "Screenshot 2023-10-15 at 14.30.25.png",
            "Screenshot 2023-10-15 at 14.30.25",
            "png",
        );
        assert_eq!(r.pattern, "screenshot");
        assert!(r.confidence > 0.9);
    }

    #[test]
    fn screenshot_screen_shot() {
        let r = classify(
            "Screen Shot 2023-10-15 at 14.30.25 PM.png",
            "Screen Shot 2023-10-15 at 14.30.25 PM",
            "png",
        );
        assert_eq!(r.pattern, "screenshot");
    }

    #[test]
    fn ai_uuid() {
        let r = classify(
            "a3f5e7b2-1c4d-4e8f-b9a0-d2e6c7f8a1b3.png",
            "a3f5e7b2-1c4d-4e8f-b9a0-d2e6c7f8a1b3",
            "png",
        );
        assert_eq!(r.pattern, "ai_generated");
    }

    #[test]
    fn ai_explicit_name() {
        let r = classify(
            "dalle-output-01.png",
            "dalle-output-01",
            "png",
        );
        assert_eq!(r.pattern, "ai_generated");
        assert!(r.confidence >= 0.9);
    }

    #[test]
    fn edited_suffix() {
        let r = classify("portrait-edited.jpg", "portrait-edited", "jpg");
        assert_eq!(r.pattern, "edited");
    }

    #[test]
    fn generic_download() {
        let r = classify("image.jpg", "image", "jpg");
        assert_eq!(r.pattern, "web_download");
    }

    #[test]
    fn unknown_arbitrary() {
        let r = classify("myholiday.jpg", "myholiday", "jpg");
        assert_eq!(r.pattern, "unknown");
    }

    #[test]
    fn twitter_media_id() {
        let r = classify("HDddUFtWkAAZgyM.jpeg", "HDddUFtWkAAZgyM", "jpeg");
        assert_eq!(r.pattern, "social_media");
        assert!(r.matched_pattern.as_deref().unwrap().contains("Twitter"));
    }

    #[test]
    fn facebook_numeric_id() {
        let r = classify("123456789012345.jpg", "123456789012345", "jpg");
        assert_eq!(r.pattern, "social_media");
        assert!(r.matched_pattern.as_deref().unwrap().contains("Facebook"));
    }
}
