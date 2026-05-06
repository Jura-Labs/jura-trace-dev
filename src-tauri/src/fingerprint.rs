// SPDX-License-Identifier: AGPL-3.0-or-later

//! Perceptual fingerprinting — compute and compare image hashes.
//!
//! Supports three hash algorithms: aHash (average), dHash (difference),
//! and pHash (perceptual, via DoubleGradient). All hashes are 64-bit
//! values stored as 16-character hex strings.

use image_hasher::{HashAlg, HasherConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ===== Types =====

/// Supported perceptual hash algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // aHash/dHash/pHash are established algorithm names
pub enum HashAlgorithm {
    AHash,
    DHash,
    PHash,
}

impl HashAlgorithm {
    /// Database label for the hash_type column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AHash => "ahash",
            Self::DHash => "dhash",
            Self::PHash => "phash",
        }
    }
}

/// Result of computing a single perceptual hash.
#[derive(Debug, Clone)]
pub struct HashResult {
    pub algorithm: HashAlgorithm,
    pub hash_hex: String,
}

/// A fingerprint record serialised to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fingerprint {
    pub fingerprint_id: String,
    pub asset_id: String,
    pub hash_type: String,
    pub hash_value: String,
    pub created_at: String,
}

/// A match result from similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarAsset {
    pub asset_id: String,
    pub file_name: String,
    pub hash_type: String,
    pub distance: u32,
    pub similarity: f64,
}

// ===== Hashing =====

/// Compute all three perceptual hashes for an image file.
///
/// Returns an empty Vec if the image cannot be decoded (SVG, corrupt, etc.).
pub fn compute_hashes(path: &Path) -> Vec<HashResult> {
    let img = match image::open(path) {
        Ok(img) => img,
        Err(e) => {
            log::warn!("Cannot compute hashes for {}: {e}", path.display());
            return vec![];
        }
    };

    let configs = [
        (HashAlgorithm::AHash, HashAlg::Mean),
        (HashAlgorithm::DHash, HashAlg::Gradient),
        (HashAlgorithm::PHash, HashAlg::DoubleGradient),
    ];

    configs
        .iter()
        .map(|(algo, hash_alg)| {
            let hasher = HasherConfig::new()
                .hash_alg(*hash_alg)
                .hash_size(8, 8)
                .to_hasher();
            let hash = hasher.hash_image(&img);
            let raw_hex: String = hash.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
            // Zero-pad to 16 chars (64 bits) for consistent Hamming distance
            let hash_hex = format!("{raw_hex:0>16}");
            HashResult {
                algorithm: *algo,
                hash_hex,
            }
        })
        .collect()
}

/// Compute only a pHash for a file on disk.
///
/// Lighter than `compute_hashes` when only the pHash is needed (e.g.
/// thumbnail mismatch check in the verify pipeline). Still requires a
/// full image decode but skips the aHash and dHash computations.
pub fn compute_phash(path: &Path) -> Option<String> {
    let img = match image::open(path) {
        Ok(img) => img,
        Err(e) => {
            log::warn!("Cannot compute pHash for {}: {e}", path.display());
            return None;
        }
    };
    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::DoubleGradient)
        .hash_size(8, 8)
        .to_hasher();
    let hash = hasher.hash_image(&img);
    let raw_hex: String = hash.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    Some(format!("{raw_hex:0>16}"))
}

/// Compute a pHash from raw image bytes (e.g. an in-memory JPEG thumbnail).
///
/// Decodes the bytes using the `image` crate and computes a pHash
/// (`DoubleGradient`, 8×8) identical in configuration to `compute_hashes`.
///
/// Returns the hash as a 16-character zero-padded hex string, or `None` if the
/// bytes cannot be decoded as a supported image format.
pub fn compute_phash_from_bytes(image_bytes: &[u8]) -> Option<String> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| log::warn!("compute_phash_from_bytes: failed to decode image: {e}"))
        .ok()?;

    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::DoubleGradient)
        .hash_size(8, 8)
        .to_hasher();
    let hash = hasher.hash_image(&img);
    let raw_hex: String = hash.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
    Some(format!("{raw_hex:0>16}"))
}

// ===== Comparison =====

/// Compute the Hamming distance between two hex-encoded 64-bit hashes.
///
/// Lower distance = more similar. Identical images return 0.
/// Maximum distance is 64 (every bit differs).
pub fn hamming_distance(hash_a: &str, hash_b: &str) -> Result<u32, String> {
    let a =
        u64::from_str_radix(hash_a, 16).map_err(|e| format!("Invalid hex hash '{hash_a}': {e}"))?;
    let b =
        u64::from_str_radix(hash_b, 16).map_err(|e| format!("Invalid hex hash '{hash_b}': {e}"))?;
    Ok((a ^ b).count_ones())
}

/// Check if a content type supports perceptual fingerprinting.
pub fn supports_fingerprinting(content_type: &str) -> bool {
    content_type == "image"
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Hamming distance ────────────────────────────────────────────

    #[test]
    fn hamming_distance_identical() {
        assert_eq!(
            hamming_distance("0000000000000000", "0000000000000000").unwrap(),
            0
        );
        assert_eq!(
            hamming_distance("ffffffffffffffff", "ffffffffffffffff").unwrap(),
            0
        );
    }

    #[test]
    fn hamming_distance_one_bit() {
        assert_eq!(
            hamming_distance("0000000000000000", "0000000000000001").unwrap(),
            1
        );
    }

    #[test]
    fn hamming_distance_max() {
        assert_eq!(
            hamming_distance("0000000000000000", "ffffffffffffffff").unwrap(),
            64
        );
    }

    #[test]
    fn hamming_distance_invalid_hex() {
        assert!(hamming_distance("zzzzzzzzzzzzzzzz", "0000000000000000").is_err());
    }

    // ── Supports fingerprinting ─────────────────────────────────────

    #[test]
    fn supports_image() {
        assert!(supports_fingerprinting("image"));
    }

    #[test]
    fn rejects_non_image_types() {
        assert!(!supports_fingerprinting("document"));
        assert!(!supports_fingerprinting("video"));
        assert!(!supports_fingerprinting("audio"));
        assert!(!supports_fingerprinting("unknown"));
        assert!(!supports_fingerprinting("unknown"));
    }

    // ── Algorithm metadata ──────────────────────────────────────────

    #[test]
    fn algorithm_labels() {
        assert_eq!(HashAlgorithm::AHash.as_str(), "ahash");
        assert_eq!(HashAlgorithm::DHash.as_str(), "dhash");
        assert_eq!(HashAlgorithm::PHash.as_str(), "phash");
    }

    #[test]
    fn three_algorithm_variants() {
        let algos = [
            HashAlgorithm::AHash,
            HashAlgorithm::DHash,
            HashAlgorithm::PHash,
        ];
        assert_eq!(algos.len(), 3);
    }

    // ── Hash computation ────────────────────────────────────────────

    #[test]
    fn compute_hashes_missing_file() {
        let result = compute_hashes(Path::new("/nonexistent/image.jpg"));
        assert!(result.is_empty());
    }

    #[test]
    fn compute_hashes_valid_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.png");
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&path).unwrap();

        let hashes = compute_hashes(&path);
        assert_eq!(hashes.len(), 3);
        for h in &hashes {
            assert_eq!(
                h.hash_hex.len(),
                16,
                "Expected 16 hex chars, got {}",
                h.hash_hex.len()
            );
            assert!(
                u64::from_str_radix(&h.hash_hex, 16).is_ok(),
                "Invalid hex: {}",
                h.hash_hex
            );
        }
    }

    #[test]
    fn same_image_produces_identical_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.png");
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 128])
        });
        img.save(&path).unwrap();

        let h1 = compute_hashes(&path);
        let h2 = compute_hashes(&path);
        for (a, b) in h1.iter().zip(h2.iter()) {
            assert_eq!(a.hash_hex, b.hash_hex);
        }
    }

    // ── compute_phash_from_bytes ─────────────────────────────────────

    #[test]
    fn phash_from_bytes_valid_png() {
        // Build a small PNG in-memory and encode it to bytes.
        let img = image::RgbImage::from_fn(32, 32, |x, y| {
            image::Rgb([(x * 8) as u8, (y * 8) as u8, 64])
        });
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();

        let hash = compute_phash_from_bytes(&buf);
        assert!(hash.is_some());
        let h = hash.unwrap();
        assert_eq!(h.len(), 16, "Expected 16 hex chars, got {}", h.len());
        assert!(
            u64::from_str_radix(&h, 16).is_ok(),
            "Hash is not valid hex: {h}"
        );
    }

    #[test]
    fn phash_from_bytes_deterministic() {
        let img = image::RgbImage::from_fn(32, 32, |x, y| {
            image::Rgb([(x * 8) as u8, (y * 8) as u8, 64])
        });
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();

        let h1 = compute_phash_from_bytes(&buf);
        let h2 = compute_phash_from_bytes(&buf);
        assert_eq!(h1, h2, "Same bytes must produce the same hash");
    }

    #[test]
    fn phash_from_bytes_invalid_returns_none() {
        let bad_bytes = b"this is not an image";
        assert!(compute_phash_from_bytes(bad_bytes).is_none());
    }

    #[test]
    fn phash_from_bytes_matches_file_phash() {
        // Hash from file path and from in-memory bytes must agree.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("thumb.png");
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 4) as u8, (y * 4) as u8, 200])
        });
        img.save(&path).unwrap();

        let file_hashes = compute_hashes(&path);
        let phash_from_file = file_hashes
            .iter()
            .find(|h| h.algorithm == HashAlgorithm::PHash)
            .map(|h| h.hash_hex.clone())
            .unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let phash_from_bytes = compute_phash_from_bytes(&bytes).unwrap();

        assert_eq!(
            phash_from_file, phash_from_bytes,
            "pHash from file and from bytes must match"
        );
    }
}
