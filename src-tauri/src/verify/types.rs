// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared verification result types.
//!
//! These types are the public output surface of the verify pipeline. They are
//! serialised over the Tauri IPC boundary to the SvelteKit frontend and over
//! the Axum REST API to external consumers. **Do not rename fields between
//! v1.x minor versions** — downstream automation (including the v1.0.1 `jura`
//! CLI) depends on the exact camelCase JSON keys produced by `rename_all`.

use serde::{Deserialize, Serialize};

use crate::{c2pa, exif_anomaly, filename_analysis, metadata, pdf_provenance, sidecar};

/// JTV-181 — public provenance contract for the v1.0 `/api/v1/verify` response.
///
/// Spec-aligned wrapper for the methodology data. From v1.0 onwards the
/// `provenance` block on `VerificationResult` is a public API contract:
/// no breaking changes between minor versions. The v1.0.1 `jura` CLI
/// (JTV-182) reads this block to write per-verification reproducibility
/// records into case files; consumers must be able to rely on the field
/// names and types staying stable across the v1.x series.
///
/// Field names match the spec in `project_cli_v101_locked.md` exactly:
///   engine_version / sidecar_version / model_hashes / verification_mode /
///   timestamp_utc.
///
/// JSON output is camelCase per the existing API convention (see
/// `ApiResponse` in `src-tauri/src/api/types.rs`). The legacy
/// [`MethodologyRecord`] is retained on `VerificationResult` for backward
/// compatibility with the existing PDF / ZIP exporters that read
/// `methodology.pipelineVersion` etc. — both blocks are populated from
/// the same source data; consumers should prefer `provenance` going
/// forward.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// Jura Trace desktop application version (e.g. `"0.9.0"`).
    pub engine_version: String,
    /// Python ML sidecar version (e.g. `"0.9.0"`), or `None` when the
    /// sidecar was unavailable at verify time.
    pub sidecar_version: Option<String>,
    /// SHA-256 hashes of the loaded ML model files. `None` for a hash means
    /// the corresponding model was not present at verify time (graceful
    /// degradation).
    pub model_hashes: ModelHashes,
    /// Investigation mode used for this run (`"quick"`, `"standard"`,
    /// `"deep"`). The legacy `"archival"` is normalised to `"deep"` upstream
    /// so this field never carries it.
    pub verification_mode: String,
    /// RFC 3339 / ISO 8601 UTC timestamp when verification completed.
    pub timestamp_utc: String,
}

/// SHA-256 hashes of the ML model files loaded at verify time. Per JTV-181
/// spec (`project_cli_v101_locked.md`), exposed as a nested block under
/// [`Provenance::model_hashes`] so future model additions extend the surface
/// without breaking the top-level shape.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelHashes {
    /// SHA-256 hex digest of the GBM deepfake-classifier joblib, or `None`
    /// when the model is not loaded.
    pub deepfake_classifier: Option<String>,
    /// SHA-256 hex digest of the UnivFD CLIP-LogReg probe joblib
    /// (`models/univfd_probe.joblib`), or `None` when the optional CLIP
    /// detector is not installed.
    pub univfd_probe: Option<String>,
}

/// Methodology metadata captured at verification time for reproducibility.
///
/// Records exactly which versions of the pipeline, sidecar, and classifier
/// model were used to produce a verification result. This enables courts,
/// insurers, and analysts to confirm that results are comparable or to
/// re-run analysis when a newer methodology version is available.
///
/// **NOTE:** From v1.0, the spec-aligned [`Provenance`] block is the public
/// API contract for new consumers (CLI, downstream automation). This
/// `MethodologyRecord` is retained for backward compatibility with existing
/// PDF / ZIP exporters that read `methodology.pipelineVersion` etc. Both
/// blocks are populated from the same source data.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MethodologyRecord {
    /// Jura Trace application version (e.g. "0.9.0").
    pub pipeline_version: String,
    /// Python ML sidecar version (e.g. "0.2.0"), if available.
    pub sidecar_version: Option<String>,
    /// SHA-256 hex digest of the GBM classifier model file, if present.
    pub classifier_model_hash: Option<String>,
    /// SHA-256 hex digest of the UnivFD CLIP probe (`models/univfd_probe.joblib`),
    /// if present. Added in JTV-181 (v1.0 CLI groundwork) so downstream
    /// reproducibility tooling — including the v1.0.1 `jura` CLI — can pin
    /// the exact CLIP ensemble used to produce a verification result.
    /// `None` when the optional CLIP detector is not installed.
    pub univfd_probe_model_hash: Option<String>,
    /// Investigation mode used (`quick`, `standard`, `deep`).
    /// The legacy `archival` value is accepted by callers and normalised to
    /// `deep` for back-compat (see `verify_content_inner` mode normalisation).
    pub analysis_mode: String,
    /// ISO 8601 timestamp when the analysis was performed.
    pub analysed_at: String,
}

/// Input quality assessment — run before detectors to identify
/// conditions that reduce the reliability of forensic analysis.
/// TRIED Pillar 2: contextual limitation disclosure.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputQualityAssessment {
    /// Estimated JPEG quality factor (1–100). None for non-JPEG.
    pub jpeg_quality_estimate: Option<u8>,
    /// Resolution category: "high" (>2MP), "medium" (0.5–2MP), "low" (<0.5MP), "thumbnail" (<128px).
    pub resolution_category: String,
    /// Image dimensions (width, height). None for non-image.
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// Whether the image appears to be a screenshot (aspect ratio + border heuristics).
    pub is_screenshot_likely: bool,
    /// Whether the file is JPEG format.
    pub is_jpeg: bool,
    /// Whether EXIF GPS and timestamp data are present.
    pub has_gps: bool,
    pub has_timestamp: bool,
    /// Whether the file uses a modern lossy codec (AVIF, WebP) that destroys
    /// JPEG-specific compression artefacts and typically strips metadata in
    /// web delivery pipelines. When true, ELA, noise analysis, copy-move
    /// detection, and JPEG ghost are significantly degraded.
    pub is_modern_lossy_codec: bool,
    /// Whether the file contains no EXIF data AND no XMP data.
    /// A strong indicator of metadata stripping via social media, CDN
    /// processing, or format conversion (e.g. AVIF downloaded from the web).
    /// When true, all provenance-based checks are unavailable.
    pub metadata_completely_absent: bool,
    /// List of detector names with reduced reliability for this input.
    pub degraded_detectors: Vec<String>,
}

/// Verification result from the VERIFY pipeline.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub source_type: String,
    pub content_type: String,
    /// Investigation mode used for this verification run
    /// (`"quick"`, `"standard"`, `"deep"`).  Legacy `"archival"` is accepted
    /// and normalised to `"deep"` for back-compat.
    pub mode: String,
    pub ela_score: Option<f64>,
    pub noise_score: Option<f64>,
    pub copy_move_score: Option<f64>,
    pub deepfake_score: Option<f64>,
    pub c2pa_valid: Option<bool>,
    pub metadata_flags: Vec<String>,
    pub claim_verdict: Option<String>,
    pub overall_trust: f64,
    pub exif_analysis: Option<exif_anomaly::ExifAnalysis>,
    /// Raw EXIF/image metadata fields (Make, Model, DateTime, GPS, etc.).
    /// Exposed for the v2 verify page EXIF detail panel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_metadata: Option<metadata::ImageMetadata>,
    pub c2pa_manifest: Option<c2pa::ManifestInfo>,
    /// Full C2PA provenance chain (active manifest + all ancestor ingredient manifests).
    /// `None` when the file contains no C2PA data.
    /// The `active` field mirrors `c2pa_manifest`; both are populated together.
    pub c2pa_chain: Option<c2pa::ManifestChain>,
    pub ela_result: Option<sidecar::ElaResult>,
    pub noise_result: Option<sidecar::NoiseResult>,
    pub copy_move_result: Option<sidecar::CopyMoveResult>,
    pub deepfake_result: Option<sidecar::DeepfakeResult>,
    /// CLIP-based AI classification result (UnivFD probe + zero-shot classifier).
    /// Only populated when the optional CLIP model is installed in the sidecar.
    pub clip_result: Option<sidecar::ClipDetectionResult>,
    pub npr_result: Option<sidecar::NprResult>,
    pub jpeg_ghost_result: Option<sidecar::JpegGhostResult>,
    pub segmented_ela_result: Option<sidecar::SegmentedElaResult>,
    pub shadow_consistency_result: Option<sidecar::ShadowConsistencyResult>,
    pub colour_temperature_result: Option<sidecar::ColourTemperatureResult>,
    pub splice_boundary_result: Option<sidecar::SpliceBoundaryResult>,
    pub ai_generator: Option<String>,
    /// Watermark extraction result for image files (standard/deep modes).
    pub watermark_extract_result: Option<sidecar::WatermarkExtractResult>,
    /// Video metadata for video content types.
    pub video_metadata: Option<sidecar::VideoMetadataResult>,
    /// Audio metadata for audio content types.
    pub audio_metadata: Option<sidecar::AudioMetadataResult>,
    /// Video deepfake analysis result for video content types.
    pub video_deepfake_result: Option<sidecar::VideoDeepfakeResult>,
    /// Speech transcription result for audio/video content types.
    pub transcription_result: Option<sidecar::TranscriptionResult>,
    /// RAG claim check result (fed by transcription text or other claims).
    pub claim_check_result: Option<sidecar::ClaimCheckResult>,
    /// AI-generated natural-language description via Ollama LLaVA.
    /// Only populated for image content in standard/deep modes when
    /// Ollama is running with a LLaVA model pulled.  `None` when unavailable.
    pub ai_description: Option<String>,
    /// Comparison between the EXIF-embedded thumbnail and the full image.
    /// `None` for non-image content types.
    pub thumbnail_check: Option<ThumbnailCheck>,
    /// 8×8 block DCT coefficient map analysis result.
    /// Only populated in deep mode when the sidecar is available.
    /// `None` for non-image content types.
    pub dct_analysis_result: Option<sidecar::DctAnalysisResult>,
    /// 2D Fourier periodic pattern detection result.
    /// Only populated in deep mode when the sidecar is available.
    /// `None` for non-image content types.
    pub fourier_analysis_result: Option<sidecar::FourierAnalysisResult>,
    /// SHA-256 hex digest of the input file computed at verification time.
    /// Allows the caller to confirm the file has not changed since import.
    pub input_sha256: Option<String>,
    /// Methodology metadata (pipeline version, sidecar version, classifier hash).
    /// Enables reproducibility and legal defensibility of results.
    ///
    /// **NOTE:** New consumers should prefer [`Provenance`] (the
    /// `provenance` field below). This `methodology` field is retained for
    /// backward compatibility with existing PDF / ZIP exporters.
    pub methodology: Option<MethodologyRecord>,
    /// JTV-181 spec-aligned provenance block — public API contract from v1.0.
    /// Same source data as [`MethodologyRecord`] above, with field names that
    /// match the v1.0.1 `jura` CLI contract (engine_version / sidecar_version
    /// / model_hashes / verification_mode / timestamp_utc).
    pub provenance: Option<Provenance>,
    /// Input quality assessment — identifies conditions that degrade detector reliability.
    pub input_quality: Option<InputQualityAssessment>,
    /// Semantic content-type classification from the sidecar.
    ///
    /// Populated for image content when the sidecar is available.  When
    /// `ai_detection_suitable` is `false` (e.g. category is `"screenshot"` or
    /// `"document"`), the deepfake and CLIP AI-detection contributions are
    /// neutralised to 0.5 in trust scoring.  `None` when the sidecar is
    /// offline or the content type is not an image.
    pub content_type_result: Option<sidecar::ContentTypeResult>,
    /// Social-media platform fingerprint — informational-only (no contribution
    /// to `compute_trust`).  Populated for image content when the sidecar is
    /// reachable.  See JTV-134 (Sprint 30, promoted from backlog #26 v1.2 → v1.0).
    pub platform_fingerprint_result: Option<sidecar::PlatformFingerprintResult>,
    /// Filename provenance heuristics (camera naming, screenshot, AI generator, etc.).
    /// Populated for all content types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename_analysis: Option<filename_analysis::FilenameAnalysis>,
    /// PDF internal provenance signals (producer, creator, incremental saves,
    /// digital signatures, redactions, PDF/A). Only populated for PDF documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_provenance: Option<pdf_provenance::PdfProvenance>,
    /// Stable string identifiers for every detector that produced a result
    /// for this verification. Consumers (PDF / ZIP renderers, Expert View
    /// badges) use this as an authoritative list of what ran, so that
    /// missing entries can be labelled "not run in this analysis" instead
    /// of silently dropped. Added in Sprint 28 (S28-FU1) alongside the
    /// schema v6 `detectors_run` DB column — the same list is persisted
    /// to the database at `insert_verification` time.
    ///
    /// Vocabulary: `exif_anomaly`, `c2pa`, `ela`, `noise`, `copy_move`,
    /// `deepfake`, `jpeg_ghost`, `segmented_ela`, `colour_temperature`,
    /// `clip`, `watermark`, `video_deepfake`, `transcription`, plus
    /// on-demand entries `npr`, `shadow_consistency`, `splice_boundary`
    /// when the user has triggered them. Keep in sync with the frontend
    /// renderer detector-ID list in `ui/src/lib/pdf.ts`.
    pub detectors_run: Vec<String>,
}

/// Result of comparing the EXIF-embedded thumbnail against the full image.
///
/// Combines two complementary signals:
///
/// 1. **pHash Hamming distance** — perceptual hash mismatch between the
///    thumbnail and the resized full image. A distance above 10 indicates
///    the thumbnail no longer represents the visible content (crop, splice,
///    AI in-painting applied after the original EXIF was written).
///
/// 2. **Pixel MSE** — mean squared error between the thumbnail pixels and
///    the corresponding region of the full image after normalisation to the
///    thumbnail's exact dimensions. MSE above ~0.02 (on a [0, 1] scale) is a
///    secondary signal that confirms perceptual deviation even when pHash
///    Hamming distance is borderline.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailCheck {
    /// Whether the file contained an EXIF-embedded thumbnail.
    pub has_thumbnail: bool,
    /// Width of the embedded thumbnail in pixels.
    /// `None` when `has_thumbnail` is `false` or decoding failed.
    pub thumbnail_width: Option<u32>,
    /// Height of the embedded thumbnail in pixels.
    /// `None` when `has_thumbnail` is `false` or decoding failed.
    pub thumbnail_height: Option<u32>,
    /// Hamming distance between thumbnail pHash and full-image pHash.
    /// `None` when `has_thumbnail` is `false` or hashing failed.
    pub hamming_distance: Option<u32>,
    /// Normalised mean squared error between thumbnail pixels and the
    /// corresponding region of the full image, in [0.0, 1.0].
    /// `None` when `has_thumbnail` is `false` or pixel comparison failed.
    /// Values above ~0.02 suggest post-capture modification.
    pub difference_score: Option<f64>,
    /// `true` when either `hamming_distance` > 10 or `difference_score` > 0.02 —
    /// the thumbnail does not match the visible content, suggesting
    /// post-capture modification.
    pub mismatch: bool,
    /// Human-readable summary of the consistency check result.
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_result_includes_mode() {
        // Verify that VerificationResult has a `mode` field and that it
        // serialises to camelCase (it is a plain string so rename_all does
        // not change the key, but the value must round-trip correctly).
        let result = VerificationResult {
            source_type: "file".to_string(),
            content_type: "image".to_string(),
            mode: "standard".to_string(),
            ela_score: None,
            noise_score: None,
            copy_move_score: None,
            deepfake_score: None,
            c2pa_valid: None,
            metadata_flags: vec![],
            claim_verdict: None,
            overall_trust: 0.0,
            exif_analysis: None,
            image_metadata: None,
            c2pa_manifest: None,
            c2pa_chain: None,
            ela_result: None,
            noise_result: None,
            copy_move_result: None,
            deepfake_result: None,
            clip_result: None,
            npr_result: None,
            jpeg_ghost_result: None,
            segmented_ela_result: None,
            shadow_consistency_result: None,
            colour_temperature_result: None,
            splice_boundary_result: None,
            ai_generator: None,
            watermark_extract_result: None,
            video_metadata: None,
            audio_metadata: None,
            video_deepfake_result: None,
            transcription_result: None,
            claim_check_result: None,
            ai_description: None,
            thumbnail_check: None,
            dct_analysis_result: None,
            fourier_analysis_result: None,
            input_sha256: None,
            methodology: None,
            provenance: None,
            input_quality: None,
            content_type_result: None,
            platform_fingerprint_result: None,
            filename_analysis: None,
            pdf_provenance: None,
            detectors_run: Vec::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(
            json.contains("\"mode\":\"standard\""),
            "mode field missing or wrong value in serialised JSON: {json}"
        );
        // JTV-134: confirm the new informational-only field serialises with
        // camelCase under the existing `rename_all = "camelCase"` rule.
        assert!(
            json.contains("\"platformFingerprintResult\""),
            "platformFingerprintResult field missing in serialised JSON: {json}"
        );
    }

    /// JTV-181 — the `provenance` block on `VerificationResult` is a public
    /// API contract from v1.0. This test asserts the exact JSON field names
    /// and shape that downstream consumers (the v1.0.1 `jura` CLI and any
    /// external automation) rely on. **Do not change these field names
    /// between v1.x minor versions** — see `project_cli_v101_locked.md`.
    #[test]
    fn provenance_serialises_with_spec_field_names() {
        let prov = Provenance {
            engine_version: "0.9.0".to_string(),
            sidecar_version: Some("0.9.0".to_string()),
            model_hashes: ModelHashes {
                deepfake_classifier: Some("aabbccdd".to_string()),
                univfd_probe: Some("11223344".to_string()),
            },
            verification_mode: "standard".to_string(),
            timestamp_utc: "2026-05-13T17:30:00Z".to_string(),
        };
        let json = serde_json::to_value(&prov).expect("Provenance must serialise");
        // Camel-case per the existing API convention. Anyone changing these
        // field names is breaking the v1.0.1 jura CLI + downstream automation.
        assert_eq!(json["engineVersion"], "0.9.0");
        assert_eq!(json["sidecarVersion"], "0.9.0");
        assert_eq!(json["modelHashes"]["deepfakeClassifier"], "aabbccdd");
        assert_eq!(json["modelHashes"]["univfdProbe"], "11223344");
        assert_eq!(json["verificationMode"], "standard");
        assert_eq!(json["timestampUtc"], "2026-05-13T17:30:00Z");
    }

    /// JTV-181 — `null` values for absent ML models must serialise as
    /// JSON `null`, not omitted. CLI consumers iterate over the
    /// `modelHashes` keys and rely on consistent presence.
    #[test]
    fn provenance_null_hashes_serialise_as_null_not_omitted() {
        let prov = Provenance {
            engine_version: "0.9.0".to_string(),
            sidecar_version: None,
            model_hashes: ModelHashes {
                deepfake_classifier: None,
                univfd_probe: None,
            },
            verification_mode: "quick".to_string(),
            timestamp_utc: "2026-05-13T17:30:00Z".to_string(),
        };
        let json = serde_json::to_value(&prov).expect("Provenance must serialise");
        // Without explicit None-handling, serde_json renders Option::None as
        // `null` — this test will fail if anyone adds
        // `#[serde(skip_serializing_if = "Option::is_none")]` which would
        // silently break the contract.
        assert!(json["sidecarVersion"].is_null());
        assert!(json["modelHashes"]["deepfakeClassifier"].is_null());
        assert!(json["modelHashes"]["univfdProbe"].is_null());
    }

    // ── ThumbnailCheck ───────────────────────────────────────────────

    #[test]
    fn thumbnail_check_serialization_no_thumbnail() {
        let tc = ThumbnailCheck {
            has_thumbnail: false,
            thumbnail_width: None,
            thumbnail_height: None,
            hamming_distance: None,
            difference_score: None,
            mismatch: false,
            summary: "No EXIF thumbnail embedded in this image.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        assert!(json.contains("\"hasThumbnail\":false"));
        assert!(json.contains("\"hammingDistance\":null"));
        assert!(json.contains("\"mismatch\":false"));
        assert!(json.contains("\"summary\""));
    }

    #[test]
    fn thumbnail_check_serialization_match() {
        let tc = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(3),
            difference_score: Some(0.005),
            mismatch: false,
            summary: "Thumbnail matches full image.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        assert!(json.contains("\"hasThumbnail\":true"));
        assert!(json.contains("\"thumbnailWidth\":160"));
        assert!(json.contains("\"thumbnailHeight\":120"));
        assert!(json.contains("\"hammingDistance\":3"));
        assert!(json.contains("\"mismatch\":false"));
    }

    #[test]
    fn thumbnail_check_serialization_mismatch() {
        let tc = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(24),
            difference_score: Some(0.08),
            mismatch: true,
            summary: "Thumbnail mismatch detected.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        assert!(json.contains("\"hasThumbnail\":true"));
        assert!(json.contains("\"hammingDistance\":24"));
        assert!(json.contains("\"mismatch\":true"));
        assert!(json.contains("\"differenceScore\":0.08"));
    }

    #[test]
    fn thumbnail_check_deserialization_round_trip() {
        let tc = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(80),
            thumbnail_height: Some(60),
            hamming_distance: Some(7),
            difference_score: Some(0.01),
            mismatch: false,
            summary: "Thumbnail matches.".to_string(),
        };
        let json = serde_json::to_string(&tc).expect("serialization must succeed");
        let decoded: ThumbnailCheck =
            serde_json::from_str(&json).expect("deserialization must succeed");
        assert_eq!(decoded.has_thumbnail, tc.has_thumbnail);
        assert_eq!(decoded.thumbnail_width, tc.thumbnail_width);
        assert_eq!(decoded.thumbnail_height, tc.thumbnail_height);
        assert_eq!(decoded.hamming_distance, tc.hamming_distance);
        assert_eq!(decoded.difference_score, tc.difference_score);
        assert_eq!(decoded.mismatch, tc.mismatch);
        assert_eq!(decoded.summary, tc.summary);
    }

    #[test]
    fn thumbnail_check_mismatch_threshold() {
        // Hamming distance == 10 is NOT a mismatch; 11 IS.
        // MSE == 0.02 is NOT a mismatch; 0.021 IS.
        let at_boundary = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(10),
            difference_score: Some(0.02),
            mismatch: false, // neither threshold exceeded
            summary: "No mismatch.".to_string(),
        };
        let over_hamming = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(11),
            difference_score: Some(0.01),
            mismatch: true, // pHash distance > 10
            summary: "pHash mismatch.".to_string(),
        };
        let over_mse = ThumbnailCheck {
            has_thumbnail: true,
            thumbnail_width: Some(160),
            thumbnail_height: Some(120),
            hamming_distance: Some(5),
            difference_score: Some(0.025),
            mismatch: true, // MSE > 0.02
            summary: "MSE mismatch.".to_string(),
        };
        assert!(!at_boundary.mismatch);
        assert!(over_hamming.mismatch);
        assert!(over_mse.mismatch);
    }
}
