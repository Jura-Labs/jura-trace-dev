// SPDX-License-Identifier: AGPL-3.0-or-later

//! Heatmap file management for the verify pipeline.
//!
//! The Python ML sidecar returns forensic heatmaps as base64-encoded PNG strings
//! embedded in JSON. Carrying these in the IPC payload between Rust and the
//! SvelteKit frontend inflates the JS heap by 30–60 MB per verify run (>100 MB
//! for deep-mode video). This module decodes each base64 heatmap to PNG bytes,
//! writes them to a per-session subdirectory of the app's cache dir, and
//! returns the on-disk path so the frontend can load the image lazily via
//! Tauri's asset protocol (`convertFileSrc()`).
//!
//! Session directories are cleaned up at the start of the *next* verify call
//! so that heatmaps remain accessible for the lifetime of the current result
//! (including PDF / ZIP export). On app shutdown Tauri's OS temp-dir management
//! handles any leftover session dirs.

use crate::sidecar::{
    ColourTemperatureResult, CopyMoveResult, DctAnalysisResult, DeepfakeResult, ElaResult,
    FourierAnalysisResult, JpegGhostResult, NoiseResult, NprResult, SegmentedElaResult,
    ShadowConsistencyResult, SpliceBoundaryResult, VideoDeepfakeResult,
};
use base64::Engine;
use std::path::{Path, PathBuf};

/// Maximum permitted size for a single decoded heatmap. The largest legitimate
/// heatmap observed (deep-mode video frame) is well under 5 MiB; this 50 MiB
/// cap exists to bound writes if the sidecar is ever compromised or returns
/// malformed payloads.
const MAX_HEATMAP_BYTES: usize = 50 * 1024 * 1024;

/// PNG magic bytes (RFC 2083 signature).
const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// JPEG magic bytes (SOI marker prefix). Used for video frame thumbnails
/// written as `.jpg` files.
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];

/// Decode a non-empty base64 string and write it to `dest`, validating that
/// the decoded bytes match the expected image format inferred from the
/// destination's extension.
///
/// Defence-in-depth: even though the sidecar is a trusted local subprocess,
/// validating the magic bytes prevents a compromised sidecar binary or a
/// malformed response from writing arbitrary content (HTML, scripts) into
/// the cache dir which is served back to the webview via the asset protocol.
///
/// Returns `Ok(())` when the file was written successfully.
/// Returns `Err` on decode failure, magic mismatch, oversize payload, or
/// write failure; a missing / empty base64 string is a no-op and returns
/// `Ok(())`.
fn decode_and_write(b64: &str, dest: &Path) -> std::io::Result<()> {
    if b64.is_empty() {
        return Ok(());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("base64 decode error: {e}"),
            )
        })?;

    if bytes.len() > MAX_HEATMAP_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "heatmap exceeds {} MiB cap ({} bytes)",
                MAX_HEATMAP_BYTES / (1024 * 1024),
                bytes.len()
            ),
        ));
    }

    let ext = dest
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());
    let expected_ok = match ext.as_deref() {
        Some("png") => bytes.starts_with(PNG_MAGIC),
        Some("jpg") | Some("jpeg") => bytes.starts_with(JPEG_MAGIC),
        // Unknown / no extension: accept either magic so future heatmap
        // formats do not silently break.  Refuse anything else.
        _ => bytes.starts_with(PNG_MAGIC) || bytes.starts_with(JPEG_MAGIC),
    };
    if !expected_ok {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "heatmap bytes do not match expected image magic",
        ));
    }

    std::fs::write(dest, &bytes)
}

/// Shared logic: write a mandatory (non-optional) base64 PNG to disk.
///
/// Returns the path as a `String`, or an empty string when the base64 value
/// is empty or writing fails (logged as a warning; not a fatal error).
fn write_mandatory(b64: &str, dest: &Path) -> String {
    if b64.is_empty() {
        return String::new();
    }
    match decode_and_write(b64, dest) {
        Ok(()) => dest.to_string_lossy().into_owned(),
        Err(e) => {
            log::warn!("Failed to write heatmap to {}: {e}", dest.display());
            String::new()
        }
    }
}

/// Shared logic: write an optional base64 PNG to disk.
///
/// Returns the path when written, `None` when the input is `None` or empty,
/// and `None` (with a warning) when writing fails.
fn write_optional(b64: Option<&str>, dest: &Path) -> Option<String> {
    let b64 = b64?;
    if b64.is_empty() {
        return None;
    }
    match decode_and_write(b64, dest) {
        Ok(()) => Some(dest.to_string_lossy().into_owned()),
        Err(e) => {
            log::warn!("Failed to write heatmap to {}: {e}", dest.display());
            None
        }
    }
}

/// Create the per-session heatmap directory under `$APPCACHE/heatmaps/<id>/`.
///
/// Returns the path to the created directory, or an `Err` if creation fails.
pub fn create_session_dir(cache_dir: &Path, session_id: &str) -> std::io::Result<PathBuf> {
    let dir = cache_dir.join("heatmaps").join(session_id);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Delete the per-session heatmap directory for the given session ID.
///
/// Called at the *start* of a new verify run to reclaim the previous session's
/// files. Silently ignores missing directories.
pub fn clear_heatmap_session(cache_dir: &Path, session_id: &str) {
    let dir = cache_dir.join("heatmaps").join(session_id);
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            log::warn!(
                "Failed to remove heatmap session dir {}: {e}",
                dir.display()
            );
        }
    }
}

/// Delete all heatmap session directories inside `$APPCACHE/heatmaps/`.
///
/// Called on app shutdown to reclaim all session dirs. Silently ignores errors.
pub fn clear_all_heatmap_sessions(cache_dir: &Path) {
    let base = cache_dir.join("heatmaps");
    if base.exists() {
        let _ = std::fs::remove_dir_all(&base);
    }
}

/// Heatmap writer: holds the session directory path.
pub struct HeatmapWriter {
    session_dir: PathBuf,
}

impl HeatmapWriter {
    /// Create a new writer. Ensures the session dir exists.
    pub fn new(cache_dir: &Path, session_id: &str) -> std::io::Result<Self> {
        let session_dir = create_session_dir(cache_dir, session_id)?;
        Ok(Self { session_dir })
    }

    /// Write the ELA heatmap to disk and populate `ela_image_url`.
    pub fn apply_ela(&self, result: &mut ElaResult) {
        let dest = self.session_dir.join("ela.png");
        result.ela_image_url = write_mandatory(&result.ela_image_base64, &dest);
    }

    /// Write the noise heatmap to disk and populate `heatmap_url`.
    pub fn apply_noise(&self, result: &mut NoiseResult) {
        let dest = self.session_dir.join("noise.png");
        result.heatmap_url = write_mandatory(&result.heatmap_base64, &dest);
    }

    /// Write the copy-move visualisation to disk and populate `visualisation_url`.
    pub fn apply_copy_move(&self, result: &mut CopyMoveResult) {
        let dest = self.session_dir.join("copy_move.png");
        result.visualisation_url = write_mandatory(&result.visualisation_base64, &dest);
    }

    /// Write the deepfake heatmap to disk and populate `heatmap_url`.
    pub fn apply_deepfake(&self, result: &mut DeepfakeResult) {
        let dest = self.session_dir.join("deepfake.png");
        result.heatmap_url = write_mandatory(&result.heatmap_base64, &dest);
    }

    /// Write the NPR heatmap to disk and populate `heatmap_url`.
    pub fn apply_npr(&self, result: &mut NprResult) {
        let dest = self.session_dir.join("npr.png");
        result.heatmap_url = write_mandatory(&result.heatmap_base64, &dest);
    }

    /// Write the JPEG ghost heatmap to disk and populate `heatmap_url`.
    pub fn apply_jpeg_ghost(&self, result: &mut JpegGhostResult) {
        let dest = self.session_dir.join("jpeg_ghost.png");
        result.heatmap_url = write_mandatory(&result.heatmap_base64, &dest);
    }

    /// Write the segmented ELA heatmap to disk and populate `heatmap_url`.
    pub fn apply_segmented_ela(&self, result: &mut SegmentedElaResult) {
        let dest = self.session_dir.join("segmented_ela.png");
        result.heatmap_url = write_optional(result.heatmap_base64.as_deref(), &dest);
    }

    /// Write the shadow consistency heatmap to disk and populate `heatmap_url`.
    pub fn apply_shadow_consistency(&self, result: &mut ShadowConsistencyResult) {
        let dest = self.session_dir.join("shadow.png");
        result.heatmap_url = write_optional(result.heatmap_base64.as_deref(), &dest);
    }

    /// Write the colour temperature heatmap to disk and populate `heatmap_url`.
    pub fn apply_colour_temperature(&self, result: &mut ColourTemperatureResult) {
        let dest = self.session_dir.join("colour_temp.png");
        result.heatmap_url = write_optional(result.heatmap_base64.as_deref(), &dest);
    }

    /// Write the splice boundary heatmap to disk and populate `heatmap_url`.
    pub fn apply_splice_boundary(&self, result: &mut SpliceBoundaryResult) {
        let dest = self.session_dir.join("splice.png");
        result.heatmap_url = write_optional(result.heatmap_base64.as_deref(), &dest);
    }

    /// Write the DCT heatmap to disk and populate `heatmap_url`.
    pub fn apply_dct(&self, result: &mut DctAnalysisResult) {
        let dest = self.session_dir.join("dct.png");
        result.heatmap_url = write_mandatory(&result.heatmap_base64, &dest);
    }

    /// Write the Fourier spectrum to disk and populate `spectrum_url`.
    pub fn apply_fourier(&self, result: &mut FourierAnalysisResult) {
        let dest = self.session_dir.join("fourier.png");
        result.spectrum_url = write_mandatory(&result.spectrum_base64, &dest);
    }

    /// Write per-frame video deepfake heatmaps and frame thumbnails.
    ///
    /// Each frame gets its own pair of files:
    ///   `frame_{n}_heatmap.png` and `frame_{n}_thumb.jpg`.
    pub fn apply_video_deepfake(&self, result: &mut VideoDeepfakeResult) {
        for frame in &mut result.frame_results {
            let n = frame.frame_index;

            let hm_dest = self.session_dir.join(format!("frame_{n}_heatmap.png"));
            frame.heatmap_url = write_mandatory(&frame.heatmap_base64, &hm_dest);

            let thumb_dest = self.session_dir.join(format!("frame_{n}_thumb.jpg"));
            frame.frame_image_url = write_mandatory(&frame.frame_image_base64, &thumb_dest);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn tiny_png_b64() -> String {
        // 1×1 transparent PNG (smallest valid PNG)
        let bytes: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
            0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR length+type
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1×1
            0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, // depth+color+crc
            0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, // IDAT length+type
            0x78, 0x9c, 0x62, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc,
            0x33, // IDAT data+crc
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82, // IEND
        ];
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn test_write_mandatory_creates_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("test.png");
        let b64 = tiny_png_b64();
        let result = write_mandatory(&b64, &dest);
        assert!(!result.is_empty(), "should return a non-empty path");
        assert!(dest.exists(), "file should be written to disk");
    }

    #[test]
    fn test_write_mandatory_empty_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("test.png");
        let result = write_mandatory("", &dest);
        assert!(result.is_empty(), "empty base64 should return empty path");
        assert!(!dest.exists(), "no file should be created");
    }

    #[test]
    fn test_write_optional_some_creates_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("opt.png");
        let b64 = tiny_png_b64();
        let result = write_optional(Some(&b64), &dest);
        assert!(result.is_some());
        assert!(dest.exists());
    }

    #[test]
    fn test_write_optional_none_is_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("none.png");
        let result = write_optional(None, &dest);
        assert!(result.is_none());
        assert!(!dest.exists());
    }

    #[test]
    fn test_create_session_dir_creates_nested_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let session_dir = create_session_dir(tmp.path(), "test-session-id").unwrap();
        assert!(session_dir.exists());
        assert!(session_dir.ends_with("heatmaps/test-session-id"));
    }

    #[test]
    fn test_clear_heatmap_session_removes_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join("heatmaps").join("sess-abc");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(dir.exists());
        clear_heatmap_session(tmp.path(), "sess-abc");
        assert!(!dir.exists());
    }

    #[test]
    fn test_clear_heatmap_session_missing_dir_is_noop() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Should not panic when directory does not exist
        clear_heatmap_session(tmp.path(), "nonexistent-session");
    }

    #[test]
    fn test_heatmap_writer_apply_ela() {
        let tmp = tempfile::TempDir::new().unwrap();
        let writer = HeatmapWriter::new(tmp.path(), "ela-session").unwrap();
        let b64 = tiny_png_b64();
        let mut ela = ElaResult {
            ela_image_base64: b64,
            ela_image_url: String::new(),
            max_difference: 10.0,
            mean_difference: 5.0,
            score: 0.3,
            suspicious: false,
        };
        writer.apply_ela(&mut ela);
        assert!(!ela.ela_image_url.is_empty());
        assert!(std::path::Path::new(&ela.ela_image_url).exists());
    }

    #[test]
    fn test_heatmap_writer_apply_segmented_ela_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        let writer = HeatmapWriter::new(tmp.path(), "seg-session").unwrap();
        let mut seg = SegmentedElaResult {
            heatmap_base64: None,
            heatmap_url: None,
            regions: vec![],
            anomalous_regions: 0,
            total_regions: 0,
            inter_region_variance: 0.0,
            score: 0.0,
            suspicious: false,
            summary: String::new(),
        };
        writer.apply_segmented_ela(&mut seg);
        assert!(seg.heatmap_url.is_none());
    }

    #[test]
    fn test_decode_and_write_rejects_non_png_for_png_dest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("not-a-png.png");
        // Base64-encoded plain text, no PNG magic.
        let b64 = base64::engine::general_purpose::STANDARD.encode(b"<html>hi</html>");
        let result = decode_and_write(&b64, &dest);
        assert!(result.is_err(), "non-PNG bytes must be rejected for .png");
        assert!(
            !dest.exists(),
            "no file should be written on validation fail"
        );
    }

    #[test]
    fn test_decode_and_write_rejects_oversized_payload() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("big.png");
        // Build > 50 MiB of arbitrary bytes prefixed with PNG magic.
        let mut bytes = vec![0u8; MAX_HEATMAP_BYTES + 1];
        bytes[..PNG_MAGIC.len()].copy_from_slice(PNG_MAGIC);
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let result = decode_and_write(&b64, &dest);
        assert!(result.is_err(), "oversized payload must be rejected");
        assert!(!dest.exists());
    }

    #[test]
    fn test_decode_and_write_accepts_jpeg_for_jpg_dest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("frame.jpg");
        // Minimum JPEG: SOI + APP0 + EOI is enough to pass magic check.
        let bytes: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0xFF, 0xD9];
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        let result = decode_and_write(&b64, &dest);
        assert!(result.is_ok(), "valid JPEG bytes must be accepted for .jpg");
        assert!(dest.exists());
    }

    #[test]
    fn test_decode_and_write_rejects_png_for_jpg_dest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("wrong-ext.jpg");
        let b64 = tiny_png_b64();
        let result = decode_and_write(&b64, &dest);
        assert!(result.is_err(), "PNG bytes must be rejected for .jpg dest");
        assert!(!dest.exists());
    }

    #[test]
    fn test_non_image_fields_untouched() {
        // Score and summary fields must survive apply_ela() unchanged.
        let tmp = tempfile::TempDir::new().unwrap();
        let writer = HeatmapWriter::new(tmp.path(), "fields-session").unwrap();
        let mut ela = ElaResult {
            ela_image_base64: String::new(),
            ela_image_url: String::new(),
            max_difference: 42.5,
            mean_difference: 8.3,
            score: 0.332,
            suspicious: false,
        };
        writer.apply_ela(&mut ela);
        // Non-image fields unchanged
        assert!((ela.score - 0.332).abs() < 0.001);
        assert!((ela.max_difference - 42.5).abs() < 0.001);
        assert!(!ela.suspicious);
    }
}
