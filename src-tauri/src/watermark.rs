//! Invisible frequency-domain watermarking.
//!
//! Uses the DWT-DCT-SVD algorithm via the `blind_watermark` crate to embed a
//! byte payload (e.g. a 16-byte institution UUID) invisibly into image files.
//!
//! ## Algorithm overview
//!
//! 1. Convert the image to YCbCr floating-point representation.
//! 2. Apply a one-level 2-D Haar DWT to each channel.
//! 3. Divide the LL sub-band into 4×4 blocks.
//! 4. For each block, apply a DCT, then decompose via SVD.
//! 5. Modify the singular values proportionally to `strength_1` to encode one
//!    bit of the payload.
//! 6. Reconstruct: inverse SVD → inverse DCT → assemble LL → inverse DWT →
//!    remove padding → save as PNG (lossless, required for survival).
//!
//! ## Seed
//!
//! A deterministic u64 seed is derived from the payload bytes by folding them
//! with FNV-1a. This means extraction requires only the payload *length* (in
//! bytes) and the same seed, both of which can be stored in the database or
//! derived from the institution UUID.
//!
//! ## Strength mapping
//!
//! | User level | `strength_1` | Typical PSNR |
//! |-----------|-------------|-------------|
//! | 1 (low)   | 20          | ~48 dB      |
//! | 2 (medium)| 36 (default)| ~42 dB      |
//! | 3 (high)  | 56          | ~36 dB      |
//!
//! A PSNR of ≥42 dB is considered imperceptible to the human eye.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ===== Public types =====

/// Parameters for embedding a watermark into an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkOptions {
    /// 16-byte institution UUID encoded as a 32-character lowercase hex string.
    ///
    /// Example: `"550e8400e29b41d4a716446655440000"`
    pub payload_hex: String,
    /// Embedding strength: 1 = low (~48 dB PSNR), 2 = medium (~42 dB, default),
    /// 3 = high (~36 dB). Higher strength is more robust against JPEG
    /// compression and resizing but more visible on close inspection.
    pub strength: Option<u32>,
}

/// Result returned after successfully embedding a watermark.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkResult {
    /// Absolute path to the watermarked output file (always PNG).
    pub output_path: String,
    /// The payload that was embedded, as a lowercase hex string.
    pub payload_hex: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
}

/// Result returned after attempting to extract a watermark.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractResult {
    /// The extracted payload as a lowercase hex string, or `None` if extraction
    /// failed.
    pub extracted_hex: Option<String>,
    /// Whether the extracted payload matches a supplied reference payload.
    /// `None` when no reference was provided (extract-only mode).
    pub matches: Option<bool>,
    /// Extraction confidence in [0, 1].
    ///
    /// Computed as the fraction of bytes that exactly match when a reference is
    /// available, or 1.0 on a successful blind extraction (no reference).
    pub confidence: f64,
    /// Human-readable status message.
    pub message: String,
}

// ===== Helpers =====

/// Decode a lowercase hex string to a `Vec<u8>`.
///
/// Returns `Err` if the string contains non-hex characters or has odd length.
fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err(format!(
            "Hex string has odd length ({}); expected an even number of characters",
            hex.len()
        ));
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex byte '{}': {e}", &hex[i..i + 2]))
        })
        .collect()
}

/// Encode a byte slice as a lowercase hex string.
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Derive a deterministic u64 seed from a byte payload using FNV-1a.
///
/// Using the payload as a seed means the extraction caller only needs the
/// payload *length* (stored in the DB) to reproduce the same seed — no
/// separate secret is required.
fn payload_seed(payload: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;
    payload.iter().fold(FNV_OFFSET, |acc, &b| {
        acc.wrapping_mul(FNV_PRIME) ^ (b as u64)
    })
}

/// Map user-facing strength level (1/2/3) to the `blind_watermark` `strength_1`
/// parameter.
fn strength_to_param(strength: u32) -> i32 {
    match strength {
        1 => 20,
        3 => 56,
        _ => 36, // level 2 is the default; also catches any out-of-range value
    }
}

// ===== Format support =====

/// Return `true` if the MIME type supports frequency-domain watermarking.
///
/// Only raster bitmap formats are supported. The output is always saved as PNG
/// (lossless), so the input format only affects the decode step.
pub fn supports_watermarking(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "image/jpeg"
            | "image/png"
            | "image/tiff"
            | "image/webp"
            | "image/bmp"
            | "image/avif"
    )
}

/// Derive the output path for a watermarked image.
///
/// Appends `_wm` to the file stem and forces `.png` as the extension:
/// `photo.jpg` → `photo_wm.png`
pub fn watermark_output_path(source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    source.with_file_name(format!("{stem}_wm.png"))
}

// ===== Core operations =====

/// Embed a watermark into an image file.
///
/// Reads `input_path`, embeds `options.payload_hex` using the DWT-DCT-SVD
/// algorithm, and writes the result to `output_path` (which should be a `.png`
/// file — use [`watermark_output_path`] to derive it).
///
/// # Errors
///
/// Returns `Err(String)` if:
/// - `payload_hex` is not valid hex or is empty.
/// - `input_path` cannot be read or decoded as an image.
/// - `output_path` cannot be written.
pub fn embed_watermark(
    input_path: &Path,
    output_path: &Path,
    options: &WatermarkOptions,
) -> Result<WatermarkResult, String> {
    // Validate and decode the payload
    let payload_bytes = decode_hex(&options.payload_hex)
        .map_err(|e| format!("Invalid payload_hex: {e}"))?;

    if payload_bytes.is_empty() {
        return Err("payload_hex must not be empty".to_string());
    }

    let strength = options.strength.unwrap_or(2);
    let strength_param = strength_to_param(strength);
    let seed = payload_seed(&payload_bytes);

    log::info!(
        "Embedding watermark: input={}, output={}, payload_len={}, strength={} (param={}), seed={}",
        input_path.display(),
        output_path.display(),
        payload_bytes.len(),
        strength,
        strength_param,
        seed
    );

    // Build the custom config with the requested strength
    let config = blind_watermark::config::WatermarkConfigBuilder::default()
        .strength_1(strength_param)
        .mode(blind_watermark::config::WatermarkMode::Strategy(seed))
        .build()
        .map_err(|e| format!("Failed to build watermark config: {e}"))?;

    // Load image and embed
    let img = image::ImageReader::open(input_path)
        .map_err(|e| format!("Cannot open input image '{}': {e}", input_path.display()))?
        .decode()
        .map_err(|e| format!("Cannot decode image '{}': {e}", input_path.display()))?
        .into_rgba32f();

    let ycbcr: blind_watermark::YCrBrAMat = img.into();
    let processed = ycbcr
        .add_padding()
        .dwt()
        .cut()
        .embed_watermark_bits(
            bitvec::slice::BitSlice::from_slice(&payload_bytes),
            &config,
        )
        .assemble()
        .idwt()
        .remove_padding();

    // Convert back to RGB8 (RGBA32F → DynamicImage → RGB8 for PNG output)
    let rgba32f: image::Rgba32FImage = processed.into();
    let dynamic: image::DynamicImage = rgba32f.into();
    dynamic
        .to_rgb8()
        .save(output_path)
        .map_err(|e| format!("Cannot save watermarked image '{}': {e}", output_path.display()))?;

    log::info!(
        "Watermark embedded successfully: {}",
        output_path.display()
    );

    Ok(WatermarkResult {
        output_path: output_path.to_string_lossy().to_string(),
        payload_hex: options.payload_hex.to_lowercase(),
        success: true,
        message: format!(
            "Watermark embedded successfully. Output: {}",
            output_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ),
    })
}

/// Extract a watermark from a watermarked image file.
///
/// `payload_len_bytes` is the number of bytes expected in the payload (e.g. 16
/// for a UUID). If `reference_hex` is supplied, the result includes a
/// `matches` field indicating whether the extracted payload matches.
///
/// The same deterministic seed strategy used during embedding is re-derived
/// from the reference payload when `reference_hex` is provided. When
/// extracting blindly (no reference), a seed of `None` is used, which falls
/// back to `WatermarkMode::Normal` in the crate.
pub fn extract_watermark(
    input_path: &Path,
    payload_len_bytes: usize,
    reference_hex: Option<&str>,
) -> Result<ExtractResult, String> {
    if payload_len_bytes == 0 {
        return Err("payload_len_bytes must be greater than 0".to_string());
    }

    // Parse the optional reference to derive a matching seed
    let reference_bytes: Option<Vec<u8>> = match reference_hex {
        Some(hex) => Some(
            decode_hex(hex).map_err(|e| format!("Invalid reference_hex: {e}"))?,
        ),
        None => None,
    };

    let seed: Option<u64> = reference_bytes.as_deref().map(payload_seed);
    let wm_len_bits = payload_len_bytes * 8;

    log::info!(
        "Extracting watermark: input={}, payload_len_bytes={}, seed={:?}",
        input_path.display(),
        payload_len_bytes,
        seed
    );

    // Use the crate's high-level extraction function
    let extracted_bytes =
        blind_watermark::utils::extract_watermark_bytes(input_path, wm_len_bits, seed)
            .map_err(|e| format!("Watermark extraction failed: {e}"))?;

    let extracted_hex = encode_hex(&extracted_bytes);

    let (matches, confidence, message) = match &reference_bytes {
        Some(reference) => {
            let matching_bytes = extracted_bytes
                .iter()
                .zip(reference.iter())
                .filter(|(a, b)| a == b)
                .count();
            let total = reference.len().max(1);
            let conf = matching_bytes as f64 / total as f64;
            let exact_match = extracted_bytes == *reference;
            (
                Some(exact_match),
                conf,
                if exact_match {
                    format!(
                        "Watermark matched: {}/{} bytes identical",
                        matching_bytes, total
                    )
                } else {
                    format!(
                        "Watermark mismatch: {}/{} bytes matched ({:.0}%)",
                        matching_bytes,
                        total,
                        conf * 100.0
                    )
                },
            )
        }
        None => (
            None,
            1.0,
            format!(
                "Watermark extracted ({} bytes, blind extraction)",
                extracted_bytes.len()
            ),
        ),
    };

    log::info!(
        "Watermark extraction complete: extracted={}, matches={:?}, confidence={:.2}",
        extracted_hex,
        matches,
        confidence
    );

    Ok(ExtractResult {
        extracted_hex: Some(extracted_hex),
        matches,
        confidence,
        message,
    })
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    // ── Format support ──────────────────────────────────────────────

    #[test]
    fn supports_watermarking_true_for_image_types() {
        assert!(supports_watermarking("image/jpeg"));
        assert!(supports_watermarking("image/png"));
        assert!(supports_watermarking("image/tiff"));
        assert!(supports_watermarking("image/webp"));
        assert!(supports_watermarking("image/bmp"));
        assert!(supports_watermarking("image/avif"));
    }

    #[test]
    fn supports_watermarking_false_for_non_images() {
        assert!(!supports_watermarking("application/pdf"));
        assert!(!supports_watermarking("video/mp4"));
        assert!(!supports_watermarking("audio/wav"));
        assert!(!supports_watermarking("text/plain"));
        assert!(!supports_watermarking("image/gif")); // animated — not in allow-list
        assert!(!supports_watermarking("image/svg+xml")); // vector — not raster
    }

    // ── Output path generation ──────────────────────────────────────

    #[test]
    fn watermark_output_path_jpeg_to_png() {
        let source = Path::new("/photos/sunset.jpg");
        let output = watermark_output_path(source);
        assert_eq!(output, PathBuf::from("/photos/sunset_wm.png"));
    }

    #[test]
    fn watermark_output_path_png_to_png() {
        let source = Path::new("/assets/logo.png");
        let output = watermark_output_path(source);
        assert_eq!(output, PathBuf::from("/assets/logo_wm.png"));
    }

    #[test]
    fn watermark_output_path_tiff_to_png() {
        let source = Path::new("/scans/document.tiff");
        let output = watermark_output_path(source);
        assert_eq!(output, PathBuf::from("/scans/document_wm.png"));
    }

    #[test]
    fn watermark_output_path_no_extension() {
        let source = Path::new("/data/rawfile");
        let output = watermark_output_path(source);
        assert_eq!(output, PathBuf::from("/data/rawfile_wm.png"));
    }

    // ── Hex encoding helpers ────────────────────────────────────────

    #[test]
    fn decode_hex_valid_roundtrip() {
        let bytes: Vec<u8> = (0u8..16).collect();
        let hex = encode_hex(&bytes);
        let decoded = decode_hex(&hex).unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn decode_hex_invalid_chars_returns_err() {
        assert!(decode_hex("gggggggggggggggg").is_err());
    }

    #[test]
    fn decode_hex_odd_length_returns_err() {
        assert!(decode_hex("abc").is_err());
    }

    #[test]
    fn decode_hex_empty_string() {
        let result = decode_hex("").unwrap();
        assert!(result.is_empty());
    }

    // ── Seed determinism ────────────────────────────────────────────

    #[test]
    fn payload_seed_is_deterministic() {
        let payload = b"juralabs-uuid-01";
        assert_eq!(payload_seed(payload), payload_seed(payload));
    }

    #[test]
    fn payload_seed_differs_for_different_payloads() {
        let a = b"juralabs-uuid-01";
        let b = b"juralabs-uuid-02";
        assert_ne!(payload_seed(a), payload_seed(b));
    }

    // ── Strength mapping ────────────────────────────────────────────

    #[test]
    fn strength_mapping_levels() {
        assert_eq!(strength_to_param(1), 20);
        assert_eq!(strength_to_param(2), 36);
        assert_eq!(strength_to_param(3), 56);
        // Out-of-range falls back to medium
        assert_eq!(strength_to_param(0), 36);
        assert_eq!(strength_to_param(99), 36);
    }

    // ── Embed validation ────────────────────────────────────────────

    #[test]
    fn embed_watermark_rejects_empty_payload() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("out.png");

        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&input).unwrap();

        let result = embed_watermark(
            &input,
            &output,
            &WatermarkOptions {
                payload_hex: String::new(),
                strength: None,
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn embed_watermark_rejects_invalid_hex() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.png");
        let output = dir.path().join("out.png");

        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&input).unwrap();

        let result = embed_watermark(
            &input,
            &output,
            &WatermarkOptions {
                payload_hex: "ZZZZZZZZ".to_string(),
                strength: None,
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid payload_hex"));
    }

    #[test]
    fn embed_watermark_rejects_missing_input() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("nonexistent.png");
        let output = dir.path().join("out.png");

        let result = embed_watermark(
            &input,
            &output,
            &WatermarkOptions {
                payload_hex: "550e8400e29b41d4a71644665544000f".to_string(),
                strength: None,
            },
        );
        assert!(result.is_err());
    }

    // ── Extract validation ──────────────────────────────────────────

    #[test]
    fn extract_watermark_rejects_zero_length() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("img.png");
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&input).unwrap();

        let result = extract_watermark(&input, 0, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("greater than 0"));
    }

    // ── Embed-extract round-trip ────────────────────────────────────

    /// Full round-trip: embed a 16-byte UUID payload, then extract and verify.
    ///
    /// Uses a 256×256 synthetic image — the minimum recommended size for the
    /// DWT-DCT-SVD algorithm to have enough blocks for a 128-bit payload.
    #[test]
    fn embed_extract_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("original.png");
        let output_path = dir.path().join("watermarked.png");

        // Create a 256×256 test image with varied pixel values so DWT has
        // meaningful frequency content to work with.
        let img = image::RgbImage::from_fn(256, 256, |x, y| {
            image::Rgb([
                ((x.wrapping_add(y)) % 256) as u8,
                ((x.wrapping_mul(2).wrapping_add(y)) % 256) as u8,
                ((y.wrapping_mul(3)) % 256) as u8,
            ])
        });
        img.save(&input_path).unwrap();

        // 16-byte UUID as hex
        let payload_hex = "550e8400e29b41d4a71644665544000f".to_string();

        let embed_result = embed_watermark(
            &input_path,
            &output_path,
            &WatermarkOptions {
                payload_hex: payload_hex.clone(),
                strength: Some(2), // medium
            },
        );

        assert!(embed_result.is_ok(), "embed failed: {:?}", embed_result.err());
        let embed_result = embed_result.unwrap();
        assert!(embed_result.success);
        assert!(output_path.exists(), "output file was not created");

        // Extract with reference
        let extract_result = extract_watermark(&output_path, 16, Some(&payload_hex));

        assert!(
            extract_result.is_ok(),
            "extract failed: {:?}",
            extract_result.err()
        );
        let extract_result = extract_result.unwrap();
        assert!(
            extract_result.extracted_hex.is_some(),
            "extracted_hex should be present"
        );
        assert_eq!(
            extract_result.matches,
            Some(true),
            "extracted payload should match the embedded payload (got extracted={:?})",
            extract_result.extracted_hex
        );
        assert!(
            extract_result.confidence > 0.9,
            "confidence should be high, got {:.2}",
            extract_result.confidence
        );
    }
}
