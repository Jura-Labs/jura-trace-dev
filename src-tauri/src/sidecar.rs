//! HTTP client for the Python ML sidecar.
//!
//! Communicates with the FastAPI sidecar running on port 8200.
//! All methods use `reqwest::blocking` since Tauri commands run
//! in a thread pool — async is unnecessary complexity here.
//!
//! Designed for graceful degradation: if the sidecar is unavailable,
//! `is_available()` returns false and callers skip ML analysis.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Sidecar capability flags.
///
/// Python sidecar returns snake_case JSON; the frontend expects camelCase.
/// We use `rename_all = "camelCase"` for serialisation to the frontend and
/// `alias` on multi-word fields so deserialization accepts Python's snake_case.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(default)]
    pub ela: bool,
    #[serde(default)]
    pub noise: bool,
    #[serde(default, alias = "copy_move")]
    pub copy_move: bool,
    #[serde(default)]
    pub deepfake: bool,
    #[serde(default)]
    pub rag: bool,
    #[serde(default, alias = "jpeg_ghost")]
    pub jpeg_ghost: bool,
    #[serde(default)]
    pub npr: bool,
    #[serde(default, alias = "segmented_ela")]
    pub segmented_ela: bool,
    #[serde(default, alias = "shadow_consistency")]
    pub shadow_consistency: bool,
    #[serde(default, alias = "colour_temperature")]
    pub colour_temperature: bool,
    #[serde(default, alias = "splice_boundary")]
    pub splice_boundary: bool,
    #[serde(default)]
    pub watermark: bool,
    #[serde(default, alias = "clip_detect")]
    pub clip_detect: bool,
    #[serde(default, alias = "video_metadata")]
    pub video_metadata: bool,
    #[serde(default, alias = "audio_metadata")]
    pub audio_metadata: bool,
    #[serde(default, alias = "video_frames")]
    pub video_frames: bool,
    #[serde(default, alias = "video_deepfake")]
    pub video_deepfake: bool,
    #[serde(default)]
    pub transcription: bool,
}

/// Health check response from the sidecar.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SidecarHealth {
    pub status: String,
    pub version: String,
    pub service: String,
    pub capabilities: Capabilities,
    pub ollama: Option<String>,
}

/// Error Level Analysis result from the sidecar.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ElaResult {
    /// Raw base64 PNG received from the sidecar. Not forwarded to the
    /// frontend — the Rust pipeline writes this to disk and populates
    /// `ela_image_url` with the on-disk path instead.
    #[serde(alias = "ela_image_base64", skip_serializing)]
    pub ela_image_base64: String,
    /// Filesystem path to the decoded PNG heatmap written by the Rust
    /// pipeline. Populated after `write_heatmaps()`; empty string when
    /// the heatmap was absent or the write failed.
    #[serde(skip_deserializing, rename = "elaImageUrl")]
    pub ela_image_url: String,
    #[serde(alias = "max_difference")]
    pub max_difference: f64,
    #[serde(alias = "mean_difference")]
    pub mean_difference: f64,
    pub score: f64,
    pub suspicious: bool,
}

/// Block-wise noise variance analysis result from the sidecar.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NoiseResult {
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: String,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: String,
    #[serde(alias = "block_variances")]
    pub block_variances: Vec<f64>,
    #[serde(alias = "global_variance")]
    pub global_variance: f64,
    #[serde(alias = "anomalous_blocks")]
    pub anomalous_blocks: u32,
    #[serde(alias = "total_blocks")]
    pub total_blocks: u32,
    pub score: f64,
    pub suspicious: bool,
}

/// A detected clone region bounding box.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CloneRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub area: i32,
    #[serde(alias = "point_count")]
    pub point_count: i32,
}

/// Copy-move forgery detection result from the sidecar.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CopyMoveResult {
    #[serde(alias = "visualisation_base64", skip_serializing)]
    pub visualisation_base64: String,
    #[serde(skip_deserializing, rename = "visualisationUrl")]
    pub visualisation_url: String,
    #[serde(alias = "clone_regions")]
    pub clone_regions: Vec<CloneRegion>,
    #[serde(alias = "matched_pairs")]
    pub matched_pairs: u32,
    pub score: f64,
    pub suspicious: bool,
}

/// Decision boundaries that govern an AI-detection verdict.
///
/// Emitted by the sidecar on every `DeepfakeResult` and `ClipDetectionResult`
/// (since the sidecar threshold-emission landing on 2026-04-28 — JTV-97).
/// `Option` + `serde(default)` so older sidecar builds without the field
/// still deserialise; the UI keeps a hardcoded fallback for one release
/// cycle covering dev workflows where an older `uvicorn` runs against a
/// newer Tauri build.  Production end-users get matched sidecar+Rust+UI
/// because the sidecar is bundled into the Tauri installer via
/// PyInstaller.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VerdictThresholds {
    #[serde(alias = "synthetic_min")]
    pub synthetic_min: f64,
    #[serde(alias = "authentic_max")]
    pub authentic_max: f64,
    #[serde(alias = "model_version")]
    pub model_version: String,
    #[serde(alias = "threshold_basis")]
    pub threshold_basis: String,
}

/// A single signal from the deepfake detection ensemble.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeepfakeSignal {
    pub name: String,
    pub description: String,
    pub weight: f64,
    pub triggered: bool,
}

/// An invisible watermark detection result from the sidecar.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkDetection {
    #[serde(alias = "type", rename = "watermarkType")]
    pub watermark_type: String,
    pub detected: bool,
    pub confidence: f64,
    pub details: String,
}

/// Deepfake / AI-generated image detection result from the sidecar.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeepfakeResult {
    pub score: f64,
    pub suspicious: bool,
    pub confidence: String,
    #[serde(default, alias = "verdict_level")]
    pub verdict_level: Option<String>,
    pub signals: Vec<DeepfakeSignal>,
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: String,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: String,
    pub summary: String,
    #[serde(default)]
    pub watermarks: Vec<WatermarkDetection>,
    #[serde(alias = "classifier_score")]
    pub classifier_score: Option<f64>,
    #[serde(default, alias = "classifier_available")]
    pub classifier_available: bool,
    #[serde(default, alias = "verdict_thresholds")]
    pub verdict_thresholds: Option<VerdictThresholds>,
}

/// CLIP-based AI image detection result (UnivFD probe + zero-shot classifier).
///
/// Returned by the sidecar's `/forensics/clip-detect` endpoint. Surfaced to
/// the frontend as the "CLIP Classification" section in Expert View. Only
/// populated when the optional CLIP model is installed in the sidecar
/// environment (`open_clip` and ViT-B/32 weights).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipDetectionResult {
    pub score: f64,
    #[serde(default, alias = "verdict_level")]
    pub verdict_level: String,
    pub confidence: String,
    /// Per-class probabilities. Frontend type calls this `classProbs`.
    #[serde(rename = "classProbs", alias = "class_probabilities")]
    pub class_probs: std::collections::HashMap<String, f64>,
    pub summary: String,
    #[serde(default, alias = "model_available")]
    pub model_available: bool,
    #[serde(default, alias = "model_name")]
    pub model_name: String,
    #[serde(default, alias = "univfd_score")]
    pub univfd_score: Option<f64>,
    #[serde(default, alias = "univfd_available")]
    pub univfd_available: bool,
    #[serde(default, alias = "verdict_thresholds")]
    pub verdict_thresholds: Option<VerdictThresholds>,
}

/// NPR (Neighbouring Pixel Relationships) analysis result.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NprResult {
    pub score: f64,
    pub suspicious: bool,
    #[serde(alias = "hv_correlation")]
    pub hv_correlation: f64,
    #[serde(alias = "diff_variance_ratio")]
    pub diff_variance_ratio: f64,
    #[serde(alias = "hf_energy_ratio")]
    pub hf_energy_ratio: f64,
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: String,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: String,
    pub summary: String,
}

/// JPEG ghost detection result for splice/composite forgery analysis.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JpegGhostResult {
    pub score: f64,
    pub suspicious: bool,
    #[serde(alias = "ghost_quality")]
    pub ghost_quality: i32,
    #[serde(alias = "quality_variance")]
    pub quality_variance: f64,
    #[serde(alias = "deviating_blocks")]
    pub deviating_blocks: u32,
    #[serde(alias = "total_blocks")]
    pub total_blocks: u32,
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: String,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: String,
    pub summary: String,
}

/// A region within a segmented ELA heatmap.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElaRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    #[serde(alias = "ela_score")]
    pub ela_score: f64,
    pub anomalous: bool,
}

/// Segmented ELA result: per-region compression inconsistency analysis.
///
/// Divides the image into blocks and computes independent ELA scores for each,
/// surfacing localised splice artefacts that global ELA may miss.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentedElaResult {
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: Option<String>,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: Option<String>,
    pub regions: Vec<ElaRegion>,
    #[serde(alias = "anomalous_regions")]
    pub anomalous_regions: u32,
    #[serde(alias = "total_regions")]
    pub total_regions: u32,
    #[serde(alias = "inter_region_variance")]
    pub inter_region_variance: f64,
    pub score: f64,
    pub suspicious: bool,
    pub summary: String,
}

/// A shadow region flagged for directional inconsistency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub area: u32,
    #[serde(alias = "gradient_angle_mean")]
    pub gradient_angle_mean: f64,
    #[serde(alias = "deviation_from_global")]
    pub deviation_from_global: f64,
    pub inconsistent: bool,
}

/// Shadow consistency analysis result.
///
/// Authentic camera images exhibit a consistent global light direction;
/// composites often have shadow regions that deviate from the dominant angle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowConsistencyResult {
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: Option<String>,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: Option<String>,
    #[serde(alias = "global_light_direction")]
    pub global_light_direction: f64,
    pub regions: Vec<ShadowRegion>,
    #[serde(alias = "inconsistent_regions")]
    pub inconsistent_regions: u32,
    #[serde(alias = "total_regions")]
    pub total_regions: u32,
    pub score: f64,
    pub suspicious: bool,
    pub summary: String,
}

/// A region with anomalous colour temperature in Lab colour space.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColourTempRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Mean a* value (green–red axis) for this region.
    #[serde(alias = "mean_a")]
    pub mean_a: f64,
    /// Mean b* value (blue–yellow axis) for this region.
    #[serde(alias = "mean_b")]
    pub mean_b: f64,
    #[serde(alias = "deviation_from_global")]
    pub deviation_from_global: f64,
    pub anomalous: bool,
}

/// Colour temperature consistency analysis result.
///
/// Checks whether the colour temperature (Lab a*/b* chromaticity) is uniform
/// across the frame. Spliced regions lit under different conditions show
/// localised temperature deviations that betray composite origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColourTemperatureResult {
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: Option<String>,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: Option<String>,
    pub regions: Vec<ColourTempRegion>,
    #[serde(alias = "anomalous_regions")]
    pub anomalous_regions: u32,
    #[serde(alias = "total_regions")]
    pub total_regions: u32,
    /// Global mean a* (green–red) across the whole image.
    #[serde(alias = "global_mean_a")]
    pub global_mean_a: f64,
    /// Global mean b* (blue–yellow) across the whole image.
    #[serde(alias = "global_mean_b")]
    pub global_mean_b: f64,
    pub score: f64,
    pub suspicious: bool,
    pub summary: String,
}

/// A candidate splice boundary between two image regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpliceBoundary {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Whether the boundary aligns with the 8×8 JPEG DCT block grid.
    #[serde(alias = "jpeg_grid_aligned")]
    pub jpeg_grid_aligned: bool,
    /// Whether noise variance is asymmetric across this boundary.
    #[serde(alias = "noise_asymmetric")]
    pub noise_asymmetric: bool,
    /// Whether edge-feathering consistent with compositing was detected.
    #[serde(alias = "feathering_detected")]
    pub feathering_detected: bool,
    /// Number of individual signals that fired for this boundary.
    #[serde(alias = "signals_triggered")]
    pub signals_triggered: u32,
    pub confidence: f64,
}

/// Splice boundary detection result.
///
/// Searches for regions where multiple low-level signals (JPEG grid alignment,
/// noise asymmetry, edge feathering) converge — a strong composite indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpliceBoundaryResult {
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: Option<String>,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: Option<String>,
    pub boundaries: Vec<SpliceBoundary>,
    #[serde(alias = "suspicious_boundaries")]
    pub suspicious_boundaries: u32,
    #[serde(alias = "total_boundaries_checked")]
    pub total_boundaries_checked: u32,
    pub score: f64,
    pub suspicious: bool,
    pub summary: String,
}

/// Watermark extraction result from the sidecar.
///
/// Attempts to extract an invisible watermark payload from an image file.
/// Returns the decoded hex payload when extraction succeeds, along with a
/// confidence score in [0, 1].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkExtractResult {
    /// The extracted payload as a UTF-8 string, if decoding succeeded.
    #[serde(default, alias = "extracted_payload")]
    pub extracted_payload: Option<String>,
    /// The extracted payload as a lowercase hex string, if decoding succeeded.
    #[serde(default, alias = "extracted_hex")]
    pub extracted_hex: Option<String>,
    /// Number of bytes decoded.
    #[serde(alias = "payload_length")]
    pub payload_length: u32,
    /// Watermark algorithm used for extraction (e.g. `"DWT-DCT-SVD"`).
    pub algorithm: String,
    /// Whether a watermark was detected in the image.
    #[serde(alias = "has_watermark")]
    pub has_watermark: bool,
    /// Extraction confidence in [0, 1].
    pub confidence: f64,
    /// Whether the extraction operation completed without error.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
}

/// Video metadata result from the sidecar.
///
/// Basic container and codec information extracted from a video file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadataResult {
    /// Duration in seconds.
    pub duration: Option<f64>,
    /// Video codec name (e.g. `"h264"`, `"vp9"`).
    pub codec: Option<String>,
    /// Frame width in pixels.
    pub width: Option<u32>,
    /// Frame height in pixels.
    pub height: Option<u32>,
    /// Frame rate in frames per second.
    pub fps: Option<f64>,
    /// Whether an audio stream is present.
    #[serde(default, alias = "has_audio")]
    pub has_audio: bool,
    /// Audio codec name when an audio stream is present (e.g. `"aac"`).
    #[serde(default, alias = "audio_codec")]
    pub audio_codec: Option<String>,
    /// File size in bytes.
    #[serde(default, alias = "file_size")]
    pub file_size: Option<u64>,
    /// Whether the operation completed without error.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
}

/// Audio metadata result from the sidecar.
///
/// Basic container and codec information extracted from an audio file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMetadataResult {
    /// Duration in seconds.
    pub duration: Option<f64>,
    /// Audio codec name (e.g. `"pcm_s16le"`, `"mp3"`, `"flac"`).
    pub codec: Option<String>,
    /// Sample rate in Hz.
    #[serde(default, alias = "sample_rate")]
    pub sample_rate: Option<u32>,
    /// Number of audio channels.
    pub channels: Option<u32>,
    /// Bit rate in bits per second.
    pub bitrate: Option<u32>,
    /// File size in bytes.
    #[serde(default, alias = "file_size")]
    pub file_size: Option<u64>,
    /// Whether the operation completed without error.
    pub success: bool,
    /// Human-readable status message.
    pub message: String,
}

/// Per-frame deepfake analysis result within a video.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDeepfakeResult {
    #[serde(alias = "frame_index")]
    pub frame_index: u32,
    pub timestamp: f64,
    pub score: f64,
    pub suspicious: bool,
    #[serde(alias = "verdict_level")]
    pub verdict_level: String,
    pub signals: Vec<DeepfakeSignal>,
    #[serde(default, alias = "classifier_score")]
    pub classifier_score: Option<f64>,
    #[serde(default, alias = "classifier_available")]
    pub classifier_available: bool,
    #[serde(default, alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: String,
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: String,
    /// Downscaled JPEG thumbnail of the analysed frame, base64-encoded.
    /// Empty when omitted by the sidecar; the v2 verify UI then renders
    /// the existing "F{n}" text label.
    #[serde(default, alias = "frame_image_base64", skip_serializing)]
    pub frame_image_base64: String,
    #[serde(skip_deserializing, rename = "frameImageUrl")]
    pub frame_image_url: String,
}

/// Video-level deepfake analysis result aggregated from per-frame scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDeepfakeResult {
    #[serde(alias = "frame_results")]
    pub frame_results: Vec<FrameDeepfakeResult>,
    #[serde(alias = "aggregate_score")]
    pub aggregate_score: f64,
    #[serde(alias = "aggregate_verdict")]
    pub aggregate_verdict: String,
    #[serde(alias = "aggregate_confidence")]
    pub aggregate_confidence: String,
    #[serde(alias = "frames_analysed")]
    pub frames_analysed: u32,
    #[serde(alias = "frames_requested")]
    pub frames_requested: u32,
    #[serde(alias = "temporal_available")]
    pub temporal_available: bool,
    #[serde(default, alias = "temporal_noise_drift")]
    pub temporal_noise_drift: Option<f64>,
    #[serde(default, alias = "temporal_spectral_drift")]
    pub temporal_spectral_drift: Option<f64>,
    #[serde(default, alias = "temporal_lbp_drift")]
    pub temporal_lbp_drift: Option<f64>,
    pub mode: String,
    #[serde(default)]
    pub duration: Option<f64>,
    pub success: bool,
    pub message: String,
}

/// A single timestamped segment from speech transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Audio/video speech transcription result from the sidecar.
///
/// Uses faster-whisper for CPU-based speech-to-text. Gracefully degrades
/// when the model is not installed (success=false, message explains why).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub language: Option<String>,
    #[serde(alias = "language_probability")]
    pub language_probability: Option<f64>,
    pub duration: Option<f64>,
    #[serde(alias = "model_size")]
    pub model_size: String,
    pub success: bool,
    pub message: String,
}

/// Audio deepfake detection result from the two-stage ensemble (Sprint 35).
///
/// While no trained probe files are deployed the sidecar returns
/// `model_loaded: false` and `verdict: "model_not_loaded"`.  All other
/// fields may still be populated from feature extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeepfakeResult {
    /// Ensemble score 0–1 (0 = authentic, 1 = synthetic). `None` when no probe is loaded.
    pub score: Option<f64>,
    /// One of: `"authentic"`, `"inconclusive"`, `"likely_synthetic"`, `"model_not_loaded"`.
    pub verdict: String,
    /// Stage 1 score from MFCC + GradientBoostingClassifier. `None` when unavailable.
    #[serde(default, alias = "stage1_score")]
    pub stage1_score: Option<f64>,
    /// Stage 2 score from Wav2Vec2-Base + LogisticRegression. `None` when unavailable.
    #[serde(default, alias = "stage2_score")]
    pub stage2_score: Option<f64>,
    /// Which stages actually ran, e.g. `["stage1"]` or `["stage1", "stage2"]`.
    #[serde(default, alias = "stages_available")]
    pub stages_available: Vec<String>,
    /// `false` until trained probe files are deployed.
    #[serde(default, alias = "model_loaded")]
    pub model_loaded: bool,
    /// Audio duration in seconds (if the file could be loaded).
    #[serde(default, alias = "duration_seconds")]
    pub duration_seconds: Option<f64>,
    /// Sample rate after resampling (16 000 Hz when librosa is available).
    #[serde(default, alias = "sample_rate")]
    pub sample_rate: Option<u32>,
    /// `true` when a 160-dim MFCC feature vector was successfully extracted.
    #[serde(default, alias = "mfcc_features_extracted")]
    pub mfcc_features_extracted: bool,
    /// `true` when a 768-dim Wav2Vec2 embedding was successfully extracted.
    #[serde(default, alias = "wav2vec2_embedding_extracted")]
    pub wav2vec2_embedding_extracted: bool,
    /// Wall-clock time for the full ensemble call in milliseconds.
    #[serde(default, alias = "processing_time_ms")]
    pub processing_time_ms: Option<f64>,
}

/// A single claim verdict from the RAG claim checker.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimVerdict {
    pub claim: String,
    pub verdict: String,
    pub explanation: String,
    pub confidence: f64,
}

/// RAG claim verification result from the sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimCheckResult {
    #[serde(alias = "overall_verdict")]
    pub overall_verdict: String,
    pub claims: Vec<ClaimVerdict>,
    #[serde(alias = "model_used")]
    pub model_used: String,
    pub methodology: String,
    pub summary: String,
}

/// AI-generated natural-language description of an image via Ollama LLaVA.
///
/// `success` is false when Ollama is unavailable or the model is not pulled —
/// callers should treat `description = None` as a graceful no-op.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDescribeResult {
    pub description: Option<String>,
    #[serde(alias = "model_used")]
    pub model_used: String,
    pub success: bool,
    pub message: String,
}

/// Content-type classification result from the sidecar.
///
/// The sidecar endpoint `/forensics/content-type` classifies an image into
/// a semantic category that the Rust pipeline uses to suppress AI-detection
/// signals on content types where the models are not reliable (e.g.
/// screenshots, documents).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentTypeResult {
    /// Semantic category: `"photograph"`, `"screenshot"`, `"document"`,
    /// `"artwork"`, or `"unknown"`.
    pub category: String,
    /// Classifier confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// When `false`, AI-detection models (deepfake GBM + CLIP probe) are
    /// unreliable for this content type and their scores must be suppressed
    /// in trust scoring.
    pub ai_detection_suitable: bool,
    /// Raw classification signals returned by the sidecar (passthrough dict).
    pub signals: serde_json::Value,
    /// Human-readable reasoning string from the classifier.
    pub reasoning: String,
}

/// Block-level DCT coefficient map analysis result.
///
/// Measures per-block AC energy variance across all 8×8 DCT blocks in the
/// image. A high coefficient of variation indicates mixed compression
/// levels — a strong indicator of splice or composite forgery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DctAnalysisResult {
    /// Base64-encoded PNG heatmap of per-block AC energy distribution.
    /// Not forwarded to the frontend — decoded to disk by `write_heatmaps()`.
    #[serde(alias = "heatmap_base64", skip_serializing)]
    pub heatmap_base64: String,
    /// Filesystem path to the decoded PNG written by `write_heatmaps()`.
    #[serde(skip_deserializing, rename = "heatmapUrl")]
    pub heatmap_url: String,
    /// Standard deviation of the DC (mean brightness) coefficient across blocks.
    #[serde(alias = "dc_std")]
    pub dc_std: f64,
    /// Mean AC energy across all blocks.
    #[serde(alias = "ac_mean")]
    pub ac_mean: f64,
    /// Standard deviation of AC energy across all blocks.
    #[serde(alias = "ac_std")]
    pub ac_std: f64,
    /// AC energy coefficient of variation (ac_std / ac_mean).
    /// Values above ~0.5 indicate compression inconsistencies.
    #[serde(alias = "ac_coefficient_of_variation")]
    pub ac_coefficient_of_variation: f64,
    /// `true` when `ac_coefficient_of_variation` exceeds the threshold.
    pub suspicious: bool,
    /// Normalised score in [0, 1].
    pub score: f64,
    /// Human-readable interpretation.
    pub summary: String,
}

/// 2D Fourier periodic pattern detection result.
///
/// Counts discrete spectral peaks in the log-magnitude FFT spectrum
/// (excluding the DC region). Authentic photographs have a smooth,
/// isotropic spectrum; GAN-generated images, screen-recaptured content,
/// and composited sources typically exhibit discrete off-centre peaks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FourierAnalysisResult {
    /// Base64-encoded PNG of the log-magnitude spectrum.
    /// Not forwarded to the frontend — decoded to disk by `write_heatmaps()`.
    #[serde(alias = "spectrum_base64", skip_serializing)]
    pub spectrum_base64: String,
    /// Filesystem path to the decoded PNG written by `write_heatmaps()`.
    #[serde(skip_deserializing, rename = "spectrumUrl")]
    pub spectrum_url: String,
    /// Number of spectral peaks above the 3-sigma detection threshold.
    #[serde(alias = "peak_count")]
    pub peak_count: u32,
    /// `true` when `peak_count` exceeds the threshold (default: 20 peaks).
    pub suspicious: bool,
    /// Normalised score in [0, 1] (peak_count / 100, capped at 1).
    pub score: f64,
    /// Human-readable interpretation.
    pub summary: String,
}

/// A single candidate platform match for the social-media fingerprint result.
/// (JTV-134, Sprint 30 — promoted from backlog #26 v1.2 to v1.0 informational-only.)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCandidate {
    /// Platform name (e.g. `"WhatsApp"`, `"Twitter/X"`, `"Facebook"`).
    pub platform: String,
    /// Confidence in this candidate, 0.0–1.0.
    pub confidence: f64,
}

/// Social-media re-upload platform fingerprinting result.
///
/// Identifies which platform processed an image from JPEG quantisation
/// signatures, characteristic resolution caps, and EXIF stripping
/// patterns. **Informational-only** — does not contribute to
/// `compute_trust`; populates `VerificationResult.platform_fingerprint_result`
/// for user awareness as a provenance disclosure.
///
/// Mirrors `PlatformFingerprintResponse` in
/// `sidecar/app/models/schemas.py` (camelCase wire format).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFingerprintResult {
    /// Whether a known platform processing signature was detected.
    pub detected: bool,
    /// Name of the most likely platform; `None` when nothing matched.
    pub platform: Option<String>,
    /// Confidence in the headline platform match, 0.0–1.0.
    pub confidence: f64,
    /// All candidates ranked by confidence (descending). Empty when nothing matched.
    pub all_candidates: Vec<PlatformCandidate>,
    /// Largest of width / height in pixels.
    pub max_dimension: Option<u32>,
    /// Estimated IJG-equivalent quality factor (1–100) for JPEG inputs.
    pub estimated_quality: Option<u32>,
    /// Whether any EXIF data is present in the file.
    pub has_exif: bool,
    /// Human-readable provenance summary, suitable for direct UI display.
    pub summary: String,
}

/// Status of the CLIP model in the Python sidecar.
///
/// Returned by `GET /forensics/clip-status` and used by the idle-watcher
/// in `lib.rs` to decide when to trigger eviction.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipStatus {
    /// Whether the CLIP ViT-B/32 model is currently resident in RAM.
    pub loaded: bool,
    /// Unix timestamp of the last CLIP detection call; 0.0 if never used.
    #[serde(alias = "last_used_ts")]
    pub last_used_ts: f64,
    /// Seconds since the last CLIP call; -1.0 if never used.
    #[serde(alias = "idle_seconds")]
    pub idle_seconds: f64,
}

/// Maximum file size eligible for the in-memory file cache.
///
/// Files larger than this threshold are not stored in the cache; sidecar calls
/// for those files re-read from disk on each request. Re-reading from an SSD
/// costs <1 ms per detector call, which is negligible compared to the sidecar
/// request time (5–60 s). The cap prevents holding up to 200 MB of raw file
/// data in RAM for the full duration of a verify run.
const SIDECAR_FILE_CACHE_MAX_BYTES: u64 = 20 * 1024 * 1024; // 20 MiB

/// HTTP client for the Python ML sidecar.
///
/// Cheaply cloneable — the inner `reqwest::blocking::Client` uses an `Arc`
/// internally, so cloning shares the connection pool rather than creating a
/// new one. This is required by the parallel verification pipeline, which
/// spawns one thread per detector and clones the client for each.
#[derive(Clone)]
pub struct SidecarClient {
    base_url: String,
    client: reqwest::blocking::Client,
    /// Optional file cache: when set, `build_image_form` uses these bytes
    /// instead of re-reading from disk when the path matches.  Set via
    /// `cache_file_bytes` / `clear_file_cache` in the verify pipeline.
    /// Wrapped in `Arc` so cloned clients share the same cache.
    #[allow(clippy::type_complexity)]
    file_cache: std::sync::Arc<std::sync::Mutex<Option<(std::path::PathBuf, Vec<u8>)>>>,
}

impl SidecarClient {
    /// Create a new sidecar client pointing at the given base URL.
    ///
    /// `api_key` is the value of the `JURA_SIDECAR_KEY` environment variable
    /// (or empty string if not set). When non-empty it is attached as the
    /// `X-Jura-API-Key` default header on every request so the sidecar
    /// authentication middleware can authorise the caller.
    pub fn new(base_url: &str, api_key: &str) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        if !api_key.is_empty() {
            match reqwest::header::HeaderValue::from_str(api_key) {
                Ok(value) => {
                    headers.insert("X-Jura-API-Key", value);
                }
                Err(_) => {
                    log::warn!(
                        "JURA_SIDECAR_KEY contains characters that cannot be used in an HTTP header                          — authentication header will not be sent"
                    );
                }
            }
        }

        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .default_headers(headers)
            .build()
            .expect("failed to build HTTP client");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
            file_cache: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Cache file bytes for the given path.  Subsequent `build_image_form`
    /// calls for this path will use the cached bytes instead of re-reading
    /// from disk.  Call `clear_file_cache` when done.
    ///
    /// Files exceeding [`SIDECAR_FILE_CACHE_MAX_BYTES`] are silently skipped;
    /// sidecar calls for those paths fall back to reading from disk.
    pub fn cache_file_bytes(&self, path: &Path, bytes: Vec<u8>) {
        if bytes.len() as u64 > SIDECAR_FILE_CACHE_MAX_BYTES {
            return;
        }
        if let Ok(mut guard) = self.file_cache.lock() {
            *guard = Some((path.to_path_buf(), bytes));
        }
    }

    /// Clear the file cache, releasing the memory.
    pub fn clear_file_cache(&self) {
        if let Ok(mut guard) = self.file_cache.lock() {
            *guard = None;
        }
    }

    /// Quick check: is the sidecar responding?
    pub fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(2))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Get full health status from the sidecar.
    pub fn check_health(&self) -> Result<SidecarHealth, String> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .map_err(|e| format!("Sidecar health request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Sidecar returned status {}", resp.status()));
        }

        resp.json::<SidecarHealth>()
            .map_err(|e| format!("Failed to parse sidecar health response: {e}"))
    }

    /// Classify the semantic content type of an image.
    ///
    /// POSTs the image as a multipart upload to `POST /forensics/content-type`.
    /// The endpoint is fast (<500 ms p95) so a 15-second timeout is used.
    ///
    /// The returned [`ContentTypeResult::ai_detection_suitable`] flag should be
    /// checked by callers before trusting deepfake / CLIP scores — when `false`,
    /// AI-detection signals should be neutralised in trust scoring.
    pub fn classify_content_type(&self, image_path: &Path) -> Result<ContentTypeResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/content-type", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(15))
            .send()
            .map_err(|e| format!("Sidecar content-type request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar content-type returned {status}: {body}"));
        }

        resp.json::<ContentTypeResult>()
            .map_err(|e| format!("Failed to parse content-type response: {e}"))
    }

    /// Run Error Level Analysis on an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/ela`.
    /// Uses a 30-second timeout for large images.
    pub fn analyse_ela(&self, image_path: &Path) -> Result<ElaResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/ela", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar ELA request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar ELA returned {status}: {body}"));
        }

        resp.json::<ElaResult>()
            .map_err(|e| format!("Failed to parse ELA response: {e}"))
    }

    /// Run block-wise noise variance analysis on an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/noise`.
    pub fn analyse_noise(&self, image_path: &Path) -> Result<NoiseResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/noise", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar noise analysis request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar noise analysis returned {status}: {body}"));
        }

        resp.json::<NoiseResult>()
            .map_err(|e| format!("Failed to parse noise analysis response: {e}"))
    }

    /// Run copy-move forgery detection on an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/copy-move`.
    pub fn detect_copy_move(&self, image_path: &Path) -> Result<CopyMoveResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/copy-move", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(60))
            .send()
            .map_err(|e| format!("Sidecar copy-move request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar copy-move returned {status}: {body}"));
        }

        resp.json::<CopyMoveResult>()
            .map_err(|e| format!("Failed to parse copy-move response: {e}"))
    }

    /// Run deepfake / AI-generated image detection on an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/deepfake`.
    /// Uses a 60-second timeout for the statistical feature ensemble.
    ///
    /// `mime_type` enables codec-aware threshold selection so that modern
    /// lossy codecs (AVIF, WebP, HEIC) do not trigger false positives.
    ///
    /// `has_camera_exif` signals to the sidecar whether the image carries
    /// camera-origin EXIF data (make, model, GPS, etc.). Images with rich
    /// camera EXIF are less likely to be AI-generated; the sidecar can use
    /// this as an additional prior when calibrating the detection threshold.
    ///
    /// `camera_authenticity_bonus` (range 0.0–1.0) is the MakerNote-derived
    /// authenticity confidence from `exif_anomaly::ExifAnalysis`. The sidecar
    /// uses this to suppress false positives on computational photography
    /// output (Pixel HDR+, iPhone Deep Fusion, drone ISPs).
    pub fn detect_deepfake(
        &self,
        image_path: &Path,
        mime_type: &str,
        has_camera_exif: bool,
        camera_authenticity_bonus: f64,
    ) -> Result<DeepfakeResult, String> {
        let form = self.build_image_form(image_path)?;

        // MIME types only contain ASCII chars (a-z, /, +, -)
        // so percent-encoding the slash is sufficient.
        let encoded_mime = mime_type.replace('/', "%2F");
        let url = format!(
            "{}/forensics/deepfake?mime_type={}&has_camera_exif={}&camera_authenticity_bonus={}",
            self.base_url, encoded_mime, has_camera_exif, camera_authenticity_bonus
        );

        let resp = self
            .client
            .post(url)
            .multipart(form)
            .timeout(Duration::from_secs(60))
            .send()
            .map_err(|e| format!("Sidecar deepfake request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar deepfake returned {status}: {body}"));
        }

        resp.json::<DeepfakeResult>()
            .map_err(|e| format!("Failed to parse deepfake response: {e}"))
    }

    /// Run CLIP-based AI image classification on an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/clip-detect`.
    /// Returns a structured result including the UnivFD probe score, the
    /// zero-shot CLIP classification, and per-class probabilities.
    ///
    /// If the optional CLIP model is not installed in the sidecar environment,
    /// the response will have `model_available=false` and a neutral score.
    /// The endpoint always returns HTTP 200 so the frontend can render a
    /// graceful degradation message.
    pub fn detect_clip(&self, image_path: &Path) -> Result<ClipDetectionResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/clip-detect", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar clip-detect request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar clip-detect returned {status}: {body}"));
        }

        resp.json::<ClipDetectionResult>()
            .map_err(|e| format!("Failed to parse clip-detect response: {e}"))
    }

    /// Run social-media platform fingerprinting on an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/platform-fingerprint`.
    /// **Informational-only** — the result populates the verify result for
    /// user awareness but does not contribute to `compute_trust`. (JTV-134.)
    pub fn analyse_platform_fingerprint(
        &self,
        image_path: &Path,
    ) -> Result<PlatformFingerprintResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/platform-fingerprint", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(15))
            .send()
            .map_err(|e| format!("Sidecar platform-fingerprint request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Sidecar platform-fingerprint returned {status}: {body}"
            ));
        }

        resp.json::<PlatformFingerprintResult>()
            .map_err(|e| format!("Failed to parse platform-fingerprint response: {e}"))
    }

    /// Run Neighbouring Pixel Relationships analysis on an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/npr`.
    pub fn analyse_npr(&self, image_path: &Path) -> Result<NprResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/npr", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar NPR analysis request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar NPR analysis returned {status}: {body}"));
        }

        resp.json::<NprResult>()
            .map_err(|e| format!("Failed to parse NPR analysis response: {e}"))
    }

    /// Run JPEG ghost detection on an image file.
    ///
    /// Detects splice/composite forgeries by analysing JPEG compression
    /// artefacts at multiple quality levels.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/jpeg-ghost`.
    /// Uses a 60-second timeout because the analysis recompresses at multiple
    /// quality levels.
    pub fn detect_jpeg_ghost(&self, image_path: &Path) -> Result<JpegGhostResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/jpeg-ghost", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(60))
            .send()
            .map_err(|e| format!("Sidecar JPEG ghost detection request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Sidecar JPEG ghost detection returned {status}: {body}"
            ));
        }

        resp.json::<JpegGhostResult>()
            .map_err(|e| format!("Failed to parse JPEG ghost detection response: {e}"))
    }

    /// Run segmented ELA on an image file.
    ///
    /// Divides the image into blocks and computes per-region ELA scores,
    /// surfacing localised compression inconsistencies that indicate splicing.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/segmented-ela`.
    pub fn check_segmented_ela(&self, image_path: &Path) -> Result<SegmentedElaResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/segmented-ela", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar segmented ELA request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar segmented ELA returned {status}: {body}"));
        }

        resp.json::<SegmentedElaResult>()
            .map_err(|e| format!("Failed to parse segmented ELA response: {e}"))
    }

    /// Run shadow consistency analysis on an image file.
    ///
    /// Checks whether shadow gradient directions are consistent across the
    /// frame. Composites often contain regions lit from incompatible angles.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/shadow-consistency`.
    pub fn check_shadow_consistency(
        &self,
        image_path: &Path,
    ) -> Result<ShadowConsistencyResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/shadow-consistency", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar shadow consistency request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Sidecar shadow consistency returned {status}: {body}"
            ));
        }

        resp.json::<ShadowConsistencyResult>()
            .map_err(|e| format!("Failed to parse shadow consistency response: {e}"))
    }

    /// Run colour temperature consistency analysis on an image file.
    ///
    /// Compares Lab chromaticity (a*/b*) across image blocks to detect
    /// regions lit under incompatible colour temperatures — a composite marker.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/colour-temperature`.
    pub fn check_colour_temperature(
        &self,
        image_path: &Path,
    ) -> Result<ColourTemperatureResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/colour-temperature", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar colour temperature request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Sidecar colour temperature returned {status}: {body}"
            ));
        }

        resp.json::<ColourTemperatureResult>()
            .map_err(|e| format!("Failed to parse colour temperature response: {e}"))
    }

    /// Run splice boundary detection on an image file.
    ///
    /// Searches for boundaries where JPEG grid alignment, noise asymmetry,
    /// and edge feathering converge — a strong indicator of compositing.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/splice-boundary`.
    pub fn check_splice_boundary(&self, image_path: &Path) -> Result<SpliceBoundaryResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/splice-boundary", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar splice boundary request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar splice boundary returned {status}: {body}"));
        }

        resp.json::<SpliceBoundaryResult>()
            .map_err(|e| format!("Failed to parse splice boundary response: {e}"))
    }

    /// Attempt to extract an invisible watermark payload from an image file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/watermark/extract`.
    /// Returns a [`WatermarkExtractResult`] indicating whether a watermark was
    /// found and what payload was decoded.
    pub fn check_watermark_extract(
        &self,
        image_path: &Path,
    ) -> Result<WatermarkExtractResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/watermark/extract", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar watermark extract request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Sidecar watermark extract returned {status}: {body}"
            ));
        }

        resp.json::<WatermarkExtractResult>()
            .map_err(|e| format!("Failed to parse watermark extract response: {e}"))
    }

    /// Extract basic metadata from a video file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/video/metadata`.
    /// Returns codec, resolution, frame rate, duration, and audio stream
    /// information.
    pub fn check_video_metadata(&self, video_path: &Path) -> Result<VideoMetadataResult, String> {
        let form = self.build_image_form(video_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/video/metadata", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar video metadata request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar video metadata returned {status}: {body}"));
        }

        resp.json::<VideoMetadataResult>()
            .map_err(|e| format!("Failed to parse video metadata response: {e}"))
    }

    /// Extract basic metadata from an audio file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/audio/metadata`.
    /// Returns codec, sample rate, channels, bit rate, and duration information.
    pub fn check_audio_metadata(&self, audio_path: &Path) -> Result<AudioMetadataResult, String> {
        let form = self.build_image_form(audio_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/audio/metadata", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar audio metadata request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar audio metadata returned {status}: {body}"));
        }

        resp.json::<AudioMetadataResult>()
            .map_err(|e| format!("Failed to parse audio metadata response: {e}"))
    }

    /// Analyse a video for AI-generated or manipulated frames.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/video/deepfake`
    /// with the analysis mode as a query parameter. Uses a 120-second timeout
    /// because video analysis processes multiple frames sequentially.
    pub fn analyse_video_deepfake(
        &self,
        video_path: &Path,
        mode: &str,
    ) -> Result<VideoDeepfakeResult, String> {
        let form = self.build_image_form(video_path)?;

        let resp = self
            .client
            .post(format!(
                "{}/forensics/video/deepfake?mode={}",
                self.base_url, mode
            ))
            .multipart(form)
            .timeout(Duration::from_secs(120))
            .send()
            .map_err(|e| format!("Sidecar video deepfake request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar video deepfake returned {status}: {body}"));
        }

        resp.json::<VideoDeepfakeResult>()
            .map_err(|e| format!("Failed to parse video deepfake response: {e}"))
    }

    /// Transcribe speech from an audio or video file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/transcribe`.
    /// Uses a 120-second timeout because transcription can be slow on CPU.
    /// Returns a `TranscriptionResult` with the full text and timestamped segments.
    pub fn transcribe(&self, media_path: &Path) -> Result<TranscriptionResult, String> {
        let form = self.build_image_form(media_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/transcribe", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(120))
            .send()
            .map_err(|e| format!("Sidecar transcription request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar transcription returned {status}: {body}"));
        }

        resp.json::<TranscriptionResult>()
            .map_err(|e| format!("Failed to parse transcription response: {e}"))
    }

    /// Detect AI-generated speech, voice cloning, and TTS in an audio file.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/audio/deepfake`.
    /// Uses a 60-second timeout (MFCC extraction ~20 ms; Wav2Vec2 ~150 ms;
    /// extra headroom for slow HDD read and sidecar startup).
    ///
    /// Returns an `AudioDeepfakeResult`.  When no trained probe files are
    /// deployed yet the sidecar returns `model_loaded: false` and
    /// `verdict: "model_not_loaded"` — this is **not** an error; the
    /// caller should surface "Audio deepfake detection available after model
    /// training" rather than a failure banner.
    pub fn detect_audio_deepfake(&self, audio_path: &Path) -> Result<AudioDeepfakeResult, String> {
        let form = self.build_image_form(audio_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/audio/deepfake", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(60))
            .send()
            .map_err(|e| format!("Sidecar audio deepfake request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar audio deepfake returned {status}: {body}"));
        }

        resp.json::<AudioDeepfakeResult>()
            .map_err(|e| format!("Failed to parse audio deepfake response: {e}"))
    }

    /// Check claims against the RAG knowledge base via the sidecar.
    ///
    /// Sends claims text as a query parameter to `POST /forensics/claim-check`.
    /// Returns a `ClaimCheckResult` with per-claim verdicts.
    pub fn check_claim(&self, claims_text: &str) -> Result<ClaimCheckResult, String> {
        let resp = self
            .client
            .post(format!("{}/forensics/claim-check", self.base_url))
            .query(&[("claims_text", claims_text)])
            .timeout(Duration::from_secs(60))
            .send()
            .map_err(|e| format!("Sidecar claim check request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar claim check returned {status}: {body}"));
        }

        resp.json::<ClaimCheckResult>()
            .map_err(|e| format!("Failed to parse claim check response: {e}"))
    }

    /// Request an AI-generated description of an image from Ollama LLaVA.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/describe`.
    /// Uses a 90-second timeout because LLaVA inference on CPU can be slow.
    ///
    /// Returns an [`ImageDescribeResult`] — `success=false` when Ollama is
    /// unavailable or the model is not pulled, so callers can degrade
    /// gracefully without treating this as an error.
    pub fn describe_image(&self, image_path: &Path) -> Result<ImageDescribeResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/describe", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(90))
            .send()
            .map_err(|e| format!("Sidecar describe request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar describe returned {status}: {body}"));
        }

        resp.json::<ImageDescribeResult>()
            .map_err(|e| format!("Failed to parse describe response: {e}"))
    }

    /// Extract and transcribe all visible text from an image via Ollama LLaVA.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/extract-text`.
    /// Uses the same 90-second timeout as `describe_image` — LLaVA inference
    /// on CPU with a text-extraction prompt can be similarly slow.
    ///
    /// Returns an [`ImageDescribeResult`] whose `description` field contains
    /// the transcribed text.  `success=false` indicates Ollama is unavailable
    /// or the model is not pulled — callers should degrade gracefully.
    pub fn extract_text(&self, image_path: &Path) -> Result<ImageDescribeResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/extract-text", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(90))
            .send()
            .map_err(|e| format!("Sidecar extract-text request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar extract-text returned {status}: {body}"));
        }

        resp.json::<ImageDescribeResult>()
            .map_err(|e| format!("Failed to parse extract-text response: {e}"))
    }

    /// Build a multipart form with an image file.
    ///
    /// Checks the internal file cache first — if the path matches, uses
    /// cached bytes instead of re-reading from disk.  The verify pipeline
    /// calls `cache_file_bytes` once before the parallel sidecar group,
    /// Run 8×8 block DCT coefficient map analysis on an image file.
    ///
    /// Computes AC energy variance across all DCT blocks and returns a
    /// heatmap plus a coefficient-of-variation score. High CV indicates
    /// mixed compression levels — a strong splice/composite marker.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/dct-analysis`.
    /// Only called in deep/archival mode.
    pub fn analyse_dct(&self, image_path: &Path) -> Result<DctAnalysisResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/dct-analysis", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar DCT analysis request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar DCT analysis returned {status}: {body}"));
        }

        resp.json::<DctAnalysisResult>()
            .map_err(|e| format!("Failed to parse DCT analysis response: {e}"))
    }

    /// Run 2D Fourier periodic pattern detection on an image file.
    ///
    /// Counts discrete spectral peaks in the log-magnitude FFT spectrum
    /// (excluding DC). A high peak count indicates periodic artefacts from
    /// GAN upsampling, screen recapture, or resampling during compositing.
    ///
    /// Sends the file as a multipart upload to `POST /forensics/fourier-analysis`.
    /// Only called in deep/archival mode.
    pub fn analyse_fourier(&self, image_path: &Path) -> Result<FourierAnalysisResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/fourier-analysis", self.base_url))
            .multipart(form)
            .timeout(Duration::from_secs(30))
            .send()
            .map_err(|e| format!("Sidecar Fourier analysis request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Sidecar Fourier analysis returned {status}: {body}"
            ));
        }

        resp.json::<FourierAnalysisResult>()
            .map_err(|e| format!("Failed to parse Fourier analysis response: {e}"))
    }

    /// Query the CLIP model load state and idle duration.
    ///
    /// Calls `GET /forensics/clip-status`. Used by the idle-watcher in
    /// `lib.rs` to decide when to trigger eviction.
    pub fn clip_status(&self) -> Result<ClipStatus, String> {
        let resp = self
            .client
            .get(format!("{}/forensics/clip-status", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .map_err(|e| format!("Sidecar clip-status request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar clip-status returned {status}: {body}"));
        }

        resp.json::<ClipStatus>()
            .map_err(|e| format!("Failed to parse clip-status response: {e}"))
    }

    /// Ask the sidecar to release the CLIP model from RAM.
    ///
    /// Calls `POST /forensics/unload-clip`. Returns `Ok(())` on success.
    /// Silently tolerated by callers — if the sidecar is unreachable the
    /// idle-watcher simply retries on the next tick.
    pub fn unload_clip(&self) -> Result<(), String> {
        let resp = self
            .client
            .post(format!("{}/forensics/unload-clip", self.base_url))
            .timeout(Duration::from_secs(10))
            .send()
            .map_err(|e| format!("Sidecar unload-clip request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("Sidecar unload-clip returned {status}: {body}"));
        }

        Ok(())
    }

    /// avoiding 4–9 redundant disk reads per image.
    fn build_image_form(
        &self,
        image_path: &Path,
    ) -> Result<reqwest::blocking::multipart::Form, String> {
        let file_name = image_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "image.bin".to_string());

        // Check the cache first — avoid re-reading the file from disk.
        let bytes = if let Ok(guard) = self.file_cache.lock() {
            if let Some((cached_path, cached_bytes)) = guard.as_ref() {
                if cached_path == image_path {
                    cached_bytes.clone()
                } else {
                    drop(guard);
                    std::fs::read(image_path).map_err(|e| {
                        format!("Failed to read image {}: {e}", image_path.display())
                    })?
                }
            } else {
                drop(guard);
                std::fs::read(image_path)
                    .map_err(|e| format!("Failed to read image {}: {e}", image_path.display()))?
            }
        } else {
            std::fs::read(image_path)
                .map_err(|e| format!("Failed to read image {}: {e}", image_path.display()))?
        };

        let part = reqwest::blocking::multipart::Part::bytes(bytes)
            .file_name(file_name)
            .mime_str("application/octet-stream")
            .map_err(|e| format!("Failed to create multipart part: {e}"))?;

        Ok(reqwest::blocking::multipart::Form::new().part("file", part))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SidecarClient::new("http://127.0.0.1:8200", "");
        assert_eq!(client.base_url, "http://127.0.0.1:8200");
    }

    #[test]
    fn test_client_strips_trailing_slash() {
        let client = SidecarClient::new("http://127.0.0.1:8200/", "");
        assert_eq!(client.base_url, "http://127.0.0.1:8200");
    }

    #[test]
    fn test_is_available_when_offline() {
        // No sidecar running — should return false, not panic
        let client = SidecarClient::new("http://127.0.0.1:19999", "");
        assert!(!client.is_available());
    }

    #[test]
    fn test_sidecar_health_deserialise() {
        let json = r#"{
            "status": "ok",
            "version": "0.2.0",
            "service": "jura-sidecar",
            "capabilities": { "ela": true, "deepfake": false, "rag": false },
            "ollama": "unavailable"
        }"#;
        let health: SidecarHealth = serde_json::from_str(json).unwrap();
        assert_eq!(health.status, "ok");
        assert!(health.capabilities.ela);
        assert!(!health.capabilities.deepfake);
        assert_eq!(health.ollama, Some("unavailable".to_string()));
    }

    #[test]
    fn test_ela_result_deserialise_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "ela_image_base64": "iVBOR...",
            "max_difference": 42.5,
            "mean_difference": 8.3,
            "score": 0.332,
            "suspicious": false
        }"#;
        let result: ElaResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.332).abs() < 0.001);
        assert!(!result.suspicious);
        assert!((result.max_difference - 42.5).abs() < 0.001);
    }

    #[test]
    fn test_health_check_fails_gracefully() {
        let client = SidecarClient::new("http://127.0.0.1:19999", "");
        let result = client.check_health();
        assert!(result.is_err());
    }

    #[test]
    fn test_noise_result_deserialise_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "heatmap_base64": "iVBOR...",
            "block_variances": [10.5, 12.3, 8.7, 45.2],
            "global_variance": 120.5,
            "anomalous_blocks": 1,
            "total_blocks": 4,
            "score": 0.25,
            "suspicious": false
        }"#;
        let result: NoiseResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.25).abs() < 0.001);
        assert_eq!(result.anomalous_blocks, 1);
        assert_eq!(result.total_blocks, 4);
        assert_eq!(result.block_variances.len(), 4);
        assert!(!result.suspicious);
    }

    #[test]
    fn test_copy_move_result_deserialise_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "visualisation_base64": "iVBOR...",
            "clone_regions": [
                { "x": 50, "y": 50, "width": 100, "height": 100, "area": 10000, "point_count": 25 }
            ],
            "matched_pairs": 42,
            "score": 0.65,
            "suspicious": true
        }"#;
        let result: CopyMoveResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.65).abs() < 0.001);
        assert!(result.suspicious);
        assert_eq!(result.matched_pairs, 42);
        assert_eq!(result.clone_regions.len(), 1);
        assert_eq!(result.clone_regions[0].x, 50);
        assert_eq!(result.clone_regions[0].area, 10000);
    }

    #[test]
    fn test_deepfake_result_deserialise_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "score": 0.72,
            "suspicious": true,
            "confidence": "high",
            "signals": [
                {
                    "name": "noise_residual",
                    "description": "Low noise residual suggests AI generation",
                    "weight": 2.0,
                    "triggered": true
                },
                {
                    "name": "frequency_energy",
                    "description": "High-frequency energy is normal",
                    "weight": 1.5,
                    "triggered": false
                }
            ],
            "heatmap_base64": "iVBOR...",
            "summary": "Image shows strong indicators of AI generation"
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.72).abs() < 0.001);
        assert!(result.suspicious);
        assert_eq!(result.confidence, "high");
        assert_eq!(result.signals.len(), 2);
        assert!(result.signals[0].triggered);
        assert!(!result.signals[1].triggered);
        assert_eq!(result.signals[0].name, "noise_residual");
        assert!((result.signals[0].weight - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_deepfake_signal_deserialise() {
        let json = r#"{
            "name": "spectral_decay",
            "description": "Spectral decay outside natural range",
            "weight": 1.0,
            "triggered": true
        }"#;
        let signal: DeepfakeSignal = serde_json::from_str(json).unwrap();
        assert_eq!(signal.name, "spectral_decay");
        assert!(signal.triggered);
        assert!((signal.weight - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_deepfake_result_with_watermarks() {
        let json = r#"{
            "score": 0.85,
            "suspicious": true,
            "confidence": "high",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "AI watermark detected",
            "watermarks": [
                {
                    "type": "stable_diffusion_v1",
                    "detected": true,
                    "confidence": 1.0,
                    "details": "Exact SD v1 watermark decoded"
                },
                {
                    "type": "sdxl",
                    "detected": false,
                    "confidence": 0.0,
                    "details": "No SDXL watermark found"
                }
            ]
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.watermarks.len(), 2);
        assert!(result.watermarks[0].detected);
        assert_eq!(result.watermarks[0].watermark_type, "stable_diffusion_v1");
        assert!((result.watermarks[0].confidence - 1.0).abs() < 0.001);
        assert!(!result.watermarks[1].detected);
    }

    #[test]
    fn test_deepfake_result_without_watermarks_field() {
        // Backwards compat: old responses without watermarks default to empty vec
        let json = r#"{
            "score": 0.72,
            "suspicious": true,
            "confidence": "high",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "Strong synthetic indicators"
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert!(result.watermarks.is_empty());
    }

    #[test]
    fn test_deepfake_result_with_verdict_level() {
        // New sidecar responses include verdict_level
        let json = r#"{
            "score": 0.15,
            "suspicious": false,
            "confidence": "high",
            "verdict_level": "authentic",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "Image appears authentic"
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.verdict_level, Some("authentic".to_string()));
    }

    #[test]
    fn test_deepfake_result_verdict_level_inconclusive() {
        let json = r#"{
            "score": 0.45,
            "suspicious": false,
            "confidence": "low",
            "verdict_level": "inconclusive",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "Mixed indicators"
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.verdict_level, Some("inconclusive".to_string()));
    }

    #[test]
    fn test_deepfake_result_verdict_level_synthetic() {
        let json = r#"{
            "score": 0.85,
            "suspicious": true,
            "confidence": "high",
            "verdict_level": "synthetic",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "Strong synthetic indicators"
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.verdict_level, Some("synthetic".to_string()));
    }

    #[test]
    fn test_deepfake_result_without_verdict_level() {
        // Backwards compat: old sidecar responses without verdict_level
        let json = r#"{
            "score": 0.72,
            "suspicious": true,
            "confidence": "high",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "Strong synthetic indicators"
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.verdict_level, None);
    }

    #[test]
    fn test_deepfake_result_with_classifier_fields() {
        // New sidecar responses include trained-classifier score and availability flag.
        let json = r#"{
            "score": 0.83,
            "suspicious": true,
            "confidence": "high",
            "verdict_level": "synthetic",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "Trained classifier confirms synthetic origin",
            "classifier_score": 0.91,
            "classifier_available": true
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.83).abs() < 0.001);
        assert_eq!(result.classifier_score, Some(0.91));
        assert!(result.classifier_available);
    }

    #[test]
    fn test_deepfake_result_without_classifier_fields() {
        // Backwards compat: old sidecar responses without classifier fields.
        // classifier_score must be None; classifier_available must default to false.
        let json = r#"{
            "score": 0.72,
            "suspicious": true,
            "confidence": "high",
            "signals": [],
            "heatmap_base64": "iVBOR...",
            "summary": "Strong synthetic indicators"
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.classifier_score, None);
        assert!(!result.classifier_available);
        // verdict_thresholds defaults to None when absent (old sidecar build).
        assert!(result.verdict_thresholds.is_none());
    }

    #[test]
    fn test_deepfake_result_with_verdict_thresholds() {
        // Sidecar emits verdict_thresholds since 2026-04-28 (JTV-97).
        let json = r#"{
            "score": 0.73,
            "suspicious": true,
            "confidence": "high",
            "signals": [],
            "heatmap_base64": "",
            "summary": "Synthetic indicators",
            "verdict_thresholds": {
                "synthetic_min": 0.55,
                "authentic_max": 0.25,
                "model_version": "gbm-v4",
                "threshold_basis": "Option C calibration on 150-image confusion matrix"
            }
        }"#;
        let result: DeepfakeResult = serde_json::from_str(json).unwrap();
        let thresholds = result
            .verdict_thresholds
            .expect("verdict_thresholds populated");
        assert!((thresholds.synthetic_min - 0.55).abs() < 1e-9);
        assert!((thresholds.authentic_max - 0.25).abs() < 1e-9);
        assert_eq!(thresholds.model_version, "gbm-v4");
        assert!(thresholds.threshold_basis.contains("confusion matrix"));
    }

    #[test]
    fn test_clip_result_with_verdict_thresholds() {
        let json = r#"{
            "score": 0.87,
            "verdict_level": "synthetic",
            "confidence": "high",
            "class_probabilities": {"photograph": 0.21, "ai_generated": 0.20},
            "model_name": "ViT-B-32",
            "model_available": true,
            "summary": "UnivFD probe flags as AI-generated",
            "univfd_score": 0.87,
            "univfd_available": true,
            "verdict_thresholds": {
                "synthetic_min": 0.60,
                "authentic_max": 0.35,
                "model_version": "univfd-probe-v9",
                "threshold_basis": "AUC 0.9933 on 39,016 samples"
            }
        }"#;
        let result: ClipDetectionResult = serde_json::from_str(json).unwrap();
        let thresholds = result
            .verdict_thresholds
            .expect("verdict_thresholds populated");
        assert!((thresholds.synthetic_min - 0.60).abs() < 1e-9);
        assert!((thresholds.authentic_max - 0.35).abs() < 1e-9);
        assert_eq!(thresholds.model_version, "univfd-probe-v9");
    }

    #[test]
    fn test_capabilities_with_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "ela": true,
            "noise": true,
            "copy_move": true,
            "deepfake": false,
            "rag": false
        }"#;
        let caps: Capabilities = serde_json::from_str(json).unwrap();
        assert!(caps.ela);
        assert!(caps.noise);
        assert!(caps.copy_move);
        assert!(!caps.deepfake);
    }

    #[test]
    fn test_platform_fingerprint_result_deserialise_camel_case() {
        // Python sidecar returns camelCase for this response (PlatformFingerprintResponse).
        let json = r#"{
            "detected": true,
            "platform": "WhatsApp",
            "confidence": 0.82,
            "allCandidates": [
                { "platform": "WhatsApp", "confidence": 0.82 },
                { "platform": "Instagram", "confidence": 0.41 }
            ],
            "maxDimension": 1280,
            "estimatedQuality": 75,
            "hasExif": false,
            "summary": "WhatsApp processing signature detected (Q≈75, max 1280, EXIF stripped)"
        }"#;
        let result: PlatformFingerprintResult = serde_json::from_str(json).unwrap();
        assert!(result.detected);
        assert_eq!(result.platform.as_deref(), Some("WhatsApp"));
        assert!((result.confidence - 0.82).abs() < 0.001);
        assert_eq!(result.all_candidates.len(), 2);
        assert_eq!(result.all_candidates[0].platform, "WhatsApp");
        assert!((result.all_candidates[0].confidence - 0.82).abs() < 0.001);
        assert_eq!(result.max_dimension, Some(1280));
        assert_eq!(result.estimated_quality, Some(75));
        assert!(!result.has_exif);
        assert!(result.summary.contains("WhatsApp"));
    }

    #[test]
    fn test_platform_fingerprint_result_deserialise_negative() {
        // No platform matched.
        let json = r#"{
            "detected": false,
            "platform": null,
            "confidence": 0.0,
            "allCandidates": [],
            "maxDimension": 4032,
            "estimatedQuality": 92,
            "hasExif": true,
            "summary": "No social media processing detected."
        }"#;
        let result: PlatformFingerprintResult = serde_json::from_str(json).unwrap();
        assert!(!result.detected);
        assert!(result.platform.is_none());
        assert!(result.all_candidates.is_empty());
        assert!(result.has_exif);
    }

    #[test]
    fn test_platform_fingerprint_result_serialises_to_camel_case() {
        // Confirm Rust → frontend wire format keeps `confidence` (not `score`)
        // and uses camelCase for the IPC payload.
        let result = PlatformFingerprintResult {
            detected: true,
            platform: Some("Twitter/X".to_string()),
            confidence: 0.71,
            all_candidates: vec![PlatformCandidate {
                platform: "Twitter/X".to_string(),
                confidence: 0.71,
            }],
            max_dimension: Some(2048),
            estimated_quality: Some(85),
            has_exif: false,
            summary: "Twitter/X re-encoding signature.".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"allCandidates\""));
        assert!(json.contains("\"maxDimension\""));
        assert!(json.contains("\"estimatedQuality\""));
        assert!(json.contains("\"hasExif\""));
        assert!(json.contains("\"confidence\":0.71"));
        // Must not regress to the historical `score` field name in the
        // candidate sub-struct (TS interface drift caught during JTV-134).
        assert!(!json.contains("\"score\""));
    }

    #[test]
    fn test_npr_result_deserialise_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "score": 0.42,
            "suspicious": false,
            "hv_correlation": 0.95,
            "diff_variance_ratio": 1.12,
            "hf_energy_ratio": 0.08,
            "heatmap_base64": "iVBOR...",
            "summary": "Pixel correlations are within expected natural range"
        }"#;
        let result: NprResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.42).abs() < 0.001);
        assert!(!result.suspicious);
        assert!((result.hv_correlation - 0.95).abs() < 0.001);
        assert!((result.diff_variance_ratio - 1.12).abs() < 0.001);
        assert!((result.hf_energy_ratio - 0.08).abs() < 0.001);
        assert_eq!(result.heatmap_base64, "iVBOR...");
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn test_jpeg_ghost_result_deserialise_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "score": 0.78,
            "suspicious": true,
            "ghost_quality": 75,
            "quality_variance": 0.34,
            "deviating_blocks": 120,
            "total_blocks": 400,
            "heatmap_base64": "iVBOR...",
            "summary": "Significant block deviations suggest splice at quality 75"
        }"#;
        let result: JpegGhostResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.78).abs() < 0.001);
        assert!(result.suspicious);
        assert_eq!(result.ghost_quality, 75);
        assert!((result.quality_variance - 0.34).abs() < 0.001);
        assert_eq!(result.deviating_blocks, 120);
        assert_eq!(result.total_blocks, 400);
        assert_eq!(result.heatmap_base64, "iVBOR...");
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn test_segmented_ela_result_deserialise_snake_case() {
        // Python sidecar returns snake_case; the alias annotations handle both
        let json = r#"{
            "heatmap_base64": "iVBOR...",
            "regions": [
                {
                    "x": 0, "y": 0, "width": 64, "height": 64,
                    "ela_score": 0.12, "anomalous": false
                },
                {
                    "x": 64, "y": 0, "width": 64, "height": 64,
                    "ela_score": 0.78, "anomalous": true
                }
            ],
            "anomalous_regions": 1,
            "total_regions": 2,
            "inter_region_variance": 0.44,
            "score": 0.61,
            "suspicious": true,
            "summary": "One region shows elevated compression inconsistency"
        }"#;
        let result: SegmentedElaResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.61).abs() < 0.001);
        assert!(result.suspicious);
        assert_eq!(result.anomalous_regions, 1);
        assert_eq!(result.total_regions, 2);
        assert!((result.inter_region_variance - 0.44).abs() < 0.001);
        assert_eq!(result.regions.len(), 2);
        assert!(result.regions[1].anomalous);
        assert!((result.regions[1].ela_score - 0.78).abs() < 0.001);
        assert_eq!(result.heatmap_base64, Some("iVBOR...".to_string()));
    }

    #[test]
    fn test_segmented_ela_result_no_heatmap() {
        // heatmap_base64 is optional — should deserialise to None when absent
        let json = r#"{
            "regions": [],
            "anomalous_regions": 0,
            "total_regions": 0,
            "inter_region_variance": 0.0,
            "score": 0.0,
            "suspicious": false,
            "summary": "No regions analysed"
        }"#;
        let result: SegmentedElaResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.heatmap_base64, None);
        assert!(!result.suspicious);
    }

    #[test]
    fn test_shadow_consistency_result_deserialise_snake_case() {
        // Python sidecar returns snake_case
        let json = r#"{
            "heatmap_base64": "iVBOR...",
            "global_light_direction": 135.0,
            "regions": [
                {
                    "x": 100, "y": 50, "width": 80, "height": 80,
                    "area": 6400,
                    "gradient_angle_mean": 42.5,
                    "deviation_from_global": 92.5,
                    "inconsistent": true
                }
            ],
            "inconsistent_regions": 1,
            "total_regions": 8,
            "score": 0.55,
            "suspicious": true,
            "summary": "One shadow region deviates significantly from the global light direction"
        }"#;
        let result: ShadowConsistencyResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.55).abs() < 0.001);
        assert!(result.suspicious);
        assert!((result.global_light_direction - 135.0).abs() < 0.001);
        assert_eq!(result.inconsistent_regions, 1);
        assert_eq!(result.total_regions, 8);
        assert_eq!(result.regions.len(), 1);
        assert!(result.regions[0].inconsistent);
        assert!((result.regions[0].deviation_from_global - 92.5).abs() < 0.001);
        assert_eq!(result.regions[0].area, 6400);
    }

    #[test]
    fn test_shadow_consistency_no_heatmap() {
        // heatmap_base64 is optional
        let json = r#"{
            "global_light_direction": 90.0,
            "regions": [],
            "inconsistent_regions": 0,
            "total_regions": 0,
            "score": 0.0,
            "suspicious": false,
            "summary": "Insufficient shadow data"
        }"#;
        let result: ShadowConsistencyResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.heatmap_base64, None);
    }

    #[test]
    fn test_colour_temperature_result_deserialise_snake_case() {
        // Python sidecar returns snake_case Lab colour values
        let json = r#"{
            "heatmap_base64": "iVBOR...",
            "regions": [
                {
                    "x": 0, "y": 0, "width": 50, "height": 50,
                    "mean_a": 2.1, "mean_b": 8.4,
                    "deviation_from_global": 0.05, "anomalous": false
                },
                {
                    "x": 200, "y": 150, "width": 50, "height": 50,
                    "mean_a": 14.7, "mean_b": -6.3,
                    "deviation_from_global": 18.2, "anomalous": true
                }
            ],
            "anomalous_regions": 1,
            "total_regions": 2,
            "global_mean_a": 2.3,
            "global_mean_b": 7.9,
            "score": 0.67,
            "suspicious": true,
            "summary": "One region shows a significant colour temperature shift"
        }"#;
        let result: ColourTemperatureResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.67).abs() < 0.001);
        assert!(result.suspicious);
        assert_eq!(result.anomalous_regions, 1);
        assert_eq!(result.total_regions, 2);
        assert!((result.global_mean_a - 2.3).abs() < 0.001);
        assert!((result.global_mean_b - 7.9).abs() < 0.001);
        assert_eq!(result.regions.len(), 2);
        assert!(result.regions[1].anomalous);
        assert!((result.regions[1].mean_a - 14.7).abs() < 0.001);
        assert!((result.regions[1].deviation_from_global - 18.2).abs() < 0.001);
    }

    #[test]
    fn test_colour_temperature_no_heatmap() {
        let json = r#"{
            "regions": [],
            "anomalous_regions": 0,
            "total_regions": 0,
            "global_mean_a": 0.0,
            "global_mean_b": 0.0,
            "score": 0.0,
            "suspicious": false,
            "summary": "No regions analysed"
        }"#;
        let result: ColourTemperatureResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.heatmap_base64, None);
    }

    #[test]
    fn test_splice_boundary_result_deserialise_snake_case() {
        // Python sidecar returns snake_case; boolean signal flags use snake_case
        let json = r#"{
            "heatmap_base64": "iVBOR...",
            "boundaries": [
                {
                    "x": 120, "y": 0, "width": 8, "height": 256,
                    "jpeg_grid_aligned": true,
                    "noise_asymmetric": true,
                    "feathering_detected": false,
                    "signals_triggered": 2,
                    "confidence": 0.74
                }
            ],
            "suspicious_boundaries": 1,
            "total_boundaries_checked": 12,
            "score": 0.74,
            "suspicious": true,
            "summary": "One boundary shows JPEG grid alignment and noise asymmetry"
        }"#;
        let result: SpliceBoundaryResult = serde_json::from_str(json).unwrap();
        assert!((result.score - 0.74).abs() < 0.001);
        assert!(result.suspicious);
        assert_eq!(result.suspicious_boundaries, 1);
        assert_eq!(result.total_boundaries_checked, 12);
        assert_eq!(result.boundaries.len(), 1);
        let b = &result.boundaries[0];
        assert!(b.jpeg_grid_aligned);
        assert!(b.noise_asymmetric);
        assert!(!b.feathering_detected);
        assert_eq!(b.signals_triggered, 2);
        assert!((b.confidence - 0.74).abs() < 0.001);
        assert_eq!(b.x, 120);
        assert_eq!(b.width, 8);
    }

    #[test]
    fn test_splice_boundary_no_suspicious_boundaries() {
        // Clean image: no boundaries, score 0
        let json = r#"{
            "boundaries": [],
            "suspicious_boundaries": 0,
            "total_boundaries_checked": 16,
            "score": 0.0,
            "suspicious": false,
            "summary": "No suspicious boundaries detected"
        }"#;
        let result: SpliceBoundaryResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.heatmap_base64, None);
        assert!(!result.suspicious);
        assert_eq!(result.boundaries.len(), 0);
        assert_eq!(result.total_boundaries_checked, 16);
    }

    // ── New Sprint 12-13 struct deserialization tests ────────────────────────

    #[test]
    fn test_watermark_extract_result_deserialise_snake_case() {
        // Python sidecar returns snake_case; alias annotations translate it
        let json = r#"{
            "extracted_payload": "juralabs-cic",
            "extracted_hex": "6a7572616c6162732d636963",
            "payload_length": 12,
            "algorithm": "DWT-DCT-SVD",
            "has_watermark": true,
            "confidence": 0.94,
            "success": true,
            "message": "Watermark extracted successfully"
        }"#;
        let result: WatermarkExtractResult = serde_json::from_str(json).unwrap();
        assert!(result.has_watermark);
        assert!(result.success);
        assert_eq!(result.payload_length, 12);
        assert_eq!(result.algorithm, "DWT-DCT-SVD");
        assert_eq!(result.extracted_payload, Some("juralabs-cic".to_string()));
        assert_eq!(
            result.extracted_hex,
            Some("6a7572616c6162732d636963".to_string())
        );
        assert!((result.confidence - 0.94).abs() < 0.001);
        assert_eq!(result.message, "Watermark extracted successfully");
    }

    #[test]
    fn test_watermark_extract_result_no_watermark() {
        // No watermark found: has_watermark=false, payload fields None
        let json = r#"{
            "payload_length": 0,
            "algorithm": "DWT-DCT-SVD",
            "has_watermark": false,
            "confidence": 0.0,
            "success": true,
            "message": "No watermark detected"
        }"#;
        let result: WatermarkExtractResult = serde_json::from_str(json).unwrap();
        assert!(!result.has_watermark);
        assert!(result.success);
        assert_eq!(result.extracted_payload, None);
        assert_eq!(result.extracted_hex, None);
        assert_eq!(result.payload_length, 0);
        assert!((result.confidence - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_watermark_extract_result_failure() {
        // Sidecar returns success=false when extraction errors
        let json = r#"{
            "payload_length": 0,
            "algorithm": "DWT-DCT-SVD",
            "has_watermark": false,
            "confidence": 0.0,
            "success": false,
            "message": "Image too small for DWT-DCT-SVD extraction"
        }"#;
        let result: WatermarkExtractResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert!(!result.has_watermark);
        assert!(result.message.contains("too small"));
    }

    #[test]
    fn test_video_metadata_result_deserialise_snake_case() {
        // Python sidecar returns snake_case for multi-word fields
        let json = r#"{
            "duration": 125.4,
            "codec": "h264",
            "width": 1920,
            "height": 1080,
            "fps": 29.97,
            "has_audio": true,
            "audio_codec": "aac",
            "file_size": 52428800,
            "success": true,
            "message": "Video metadata extracted successfully"
        }"#;
        let result: VideoMetadataResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert!((result.duration.unwrap() - 125.4).abs() < 0.001);
        assert_eq!(result.codec, Some("h264".to_string()));
        assert_eq!(result.width, Some(1920));
        assert_eq!(result.height, Some(1080));
        assert!((result.fps.unwrap() - 29.97).abs() < 0.001);
        assert!(result.has_audio);
        assert_eq!(result.audio_codec, Some("aac".to_string()));
        assert_eq!(result.file_size, Some(52_428_800));
    }

    #[test]
    fn test_video_metadata_result_no_audio() {
        // Video with no audio track
        let json = r#"{
            "duration": 10.0,
            "codec": "vp9",
            "width": 1280,
            "height": 720,
            "fps": 24.0,
            "has_audio": false,
            "file_size": 1048576,
            "success": true,
            "message": "Video metadata extracted successfully"
        }"#;
        let result: VideoMetadataResult = serde_json::from_str(json).unwrap();
        assert!(!result.has_audio);
        assert_eq!(result.audio_codec, None);
        assert_eq!(result.codec, Some("vp9".to_string()));
    }

    #[test]
    fn test_video_metadata_result_failure() {
        let json = r#"{
            "success": false,
            "message": "Unsupported container format"
        }"#;
        let result: VideoMetadataResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert!(!result.has_audio);
        assert_eq!(result.duration, None);
        assert_eq!(result.codec, None);
    }

    #[test]
    fn test_audio_metadata_result_deserialise_snake_case() {
        // Python sidecar returns snake_case for multi-word fields
        let json = r#"{
            "duration": 213.7,
            "codec": "flac",
            "sample_rate": 44100,
            "channels": 2,
            "bitrate": 1411200,
            "file_size": 37748736,
            "success": true,
            "message": "Audio metadata extracted successfully"
        }"#;
        let result: AudioMetadataResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert!((result.duration.unwrap() - 213.7).abs() < 0.001);
        assert_eq!(result.codec, Some("flac".to_string()));
        assert_eq!(result.sample_rate, Some(44_100));
        assert_eq!(result.channels, Some(2));
        assert_eq!(result.bitrate, Some(1_411_200));
        assert_eq!(result.file_size, Some(37_748_736));
    }

    #[test]
    fn test_audio_metadata_result_minimal() {
        // Minimal response: only required fields
        let json = r#"{
            "success": false,
            "message": "Could not read audio stream"
        }"#;
        let result: AudioMetadataResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert_eq!(result.duration, None);
        assert_eq!(result.codec, None);
        assert_eq!(result.sample_rate, None);
        assert_eq!(result.channels, None);
        assert_eq!(result.bitrate, None);
        assert_eq!(result.file_size, None);
    }

    #[test]
    fn test_audio_metadata_result_mono() {
        // Single-channel WAV
        let json = r#"{
            "duration": 5.2,
            "codec": "pcm_s16le",
            "sample_rate": 22050,
            "channels": 1,
            "bitrate": 352800,
            "file_size": 229376,
            "success": true,
            "message": "Audio metadata extracted successfully"
        }"#;
        let result: AudioMetadataResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.channels, Some(1));
        assert_eq!(result.sample_rate, Some(22_050));
        assert_eq!(result.codec, Some("pcm_s16le".to_string()));
    }

    #[test]
    fn test_video_deepfake_result_deserialise_snake_case() {
        let json = r#"{
            "frame_results": [
                {
                    "frame_index": 0,
                    "timestamp": 1.5,
                    "score": 0.42,
                    "suspicious": false,
                    "verdict_level": "inconclusive",
                    "signals": [
                        {
                            "name": "noise_residual",
                            "description": "Low noise residual",
                            "weight": 2.0,
                            "triggered": true
                        }
                    ],
                    "classifier_score": 0.38,
                    "classifier_available": true,
                    "heatmap_base64": ""
                },
                {
                    "frame_index": 1,
                    "timestamp": 3.0,
                    "score": 0.71,
                    "suspicious": true,
                    "verdict_level": "synthetic",
                    "signals": [],
                    "heatmap_base64": ""
                }
            ],
            "aggregate_score": 0.55,
            "aggregate_verdict": "inconclusive",
            "aggregate_confidence": "medium",
            "frames_analysed": 2,
            "frames_requested": 6,
            "temporal_available": false,
            "temporal_noise_drift": null,
            "temporal_spectral_drift": null,
            "temporal_lbp_drift": null,
            "mode": "standard",
            "duration": 10.5,
            "success": true,
            "message": "Analysed 2 frames in standard mode."
        }"#;
        let result: VideoDeepfakeResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.frames_analysed, 2);
        assert_eq!(result.frames_requested, 6);
        assert!((result.aggregate_score - 0.55).abs() < 0.001);
        assert_eq!(result.aggregate_verdict, "inconclusive");
        assert_eq!(result.aggregate_confidence, "medium");
        assert!(!result.temporal_available);
        assert_eq!(result.temporal_noise_drift, None);
        assert_eq!(result.mode, "standard");
        assert!((result.duration.unwrap() - 10.5).abs() < 0.001);
        // Check frame results
        assert_eq!(result.frame_results.len(), 2);
        assert_eq!(result.frame_results[0].frame_index, 0);
        assert!((result.frame_results[0].score - 0.42).abs() < 0.001);
        assert!(!result.frame_results[0].suspicious);
        assert_eq!(result.frame_results[0].verdict_level, "inconclusive");
        assert_eq!(result.frame_results[0].signals.len(), 1);
        assert!(result.frame_results[0].signals[0].triggered);
        assert_eq!(result.frame_results[0].classifier_score, Some(0.38));
        assert!(result.frame_results[0].classifier_available);
        assert_eq!(result.frame_results[1].frame_index, 1);
        assert!(result.frame_results[1].suspicious);
        assert_eq!(result.frame_results[1].verdict_level, "synthetic");
    }

    #[test]
    fn test_video_deepfake_result_with_temporal_signals() {
        let json = r#"{
            "frame_results": [],
            "aggregate_score": 0.35,
            "aggregate_verdict": "authentic",
            "aggregate_confidence": "high",
            "frames_analysed": 6,
            "frames_requested": 6,
            "temporal_available": true,
            "temporal_noise_drift": 0.12,
            "temporal_spectral_drift": 0.08,
            "temporal_lbp_drift": 0.15,
            "mode": "standard",
            "duration": 30.0,
            "success": true,
            "message": "Analysis complete"
        }"#;
        let result: VideoDeepfakeResult = serde_json::from_str(json).unwrap();
        assert!(result.temporal_available);
        assert!((result.temporal_noise_drift.unwrap() - 0.12).abs() < 0.001);
        assert!((result.temporal_spectral_drift.unwrap() - 0.08).abs() < 0.001);
        assert!((result.temporal_lbp_drift.unwrap() - 0.15).abs() < 0.001);
        assert_eq!(result.frames_analysed, 6);
    }

    #[test]
    fn test_video_deepfake_result_failure() {
        let json = r#"{
            "frame_results": [],
            "aggregate_score": 0.0,
            "aggregate_verdict": "inconclusive",
            "aggregate_confidence": "low",
            "frames_analysed": 0,
            "frames_requested": 6,
            "temporal_available": false,
            "mode": "standard",
            "success": false,
            "message": "FFmpeg is not installed"
        }"#;
        let result: VideoDeepfakeResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert_eq!(result.frames_analysed, 0);
        assert_eq!(result.duration, None);
    }

    #[test]
    fn test_transcription_result_deserialise_snake_case() {
        // Python sidecar returns snake_case for multi-word fields
        let json = r#"{
            "text": "Hello world, this is a transcription test.",
            "segments": [
                { "start": 0.0, "end": 2.5, "text": "Hello world," },
                { "start": 2.5, "end": 5.1, "text": "this is a transcription test." }
            ],
            "language": "en",
            "language_probability": 0.9876,
            "duration": 5.1,
            "model_size": "base",
            "success": true,
            "message": "Transcribed 5.1s of audio (en, 2 segments)"
        }"#;
        let result: TranscriptionResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.text, "Hello world, this is a transcription test.");
        assert_eq!(result.segments.len(), 2);
        assert!((result.segments[0].start - 0.0).abs() < 0.001);
        assert!((result.segments[0].end - 2.5).abs() < 0.001);
        assert_eq!(result.segments[0].text, "Hello world,");
        assert_eq!(result.language, Some("en".to_string()));
        assert!((result.language_probability.unwrap() - 0.9876).abs() < 0.0001);
        assert!((result.duration.unwrap() - 5.1).abs() < 0.001);
        assert_eq!(result.model_size, "base");
    }

    #[test]
    fn test_transcription_result_failure() {
        // Sidecar returns success=false when faster-whisper is not installed
        let json = r#"{
            "text": "",
            "segments": [],
            "language": null,
            "language_probability": null,
            "duration": null,
            "model_size": "base",
            "success": false,
            "message": "Transcription unavailable: faster-whisper is not installed."
        }"#;
        let result: TranscriptionResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert!(result.text.is_empty());
        assert!(result.segments.is_empty());
        assert_eq!(result.language, None);
        assert_eq!(result.language_probability, None);
        assert_eq!(result.duration, None);
        assert!(result.message.contains("faster-whisper"));
    }

    #[test]
    fn test_claim_check_result_deserialise_snake_case() {
        // New-vocabulary fixture. The claim checker was reframed from a
        // fact-checker to a knowledge-base retrieval aid in April 2026
        // (see tech-debt audit decisions memory). The Rust side treats
        // these strings as opaque and only checks round-tripping.
        let json = r#"{
            "overall_verdict": "consistent_with_kb",
            "claims": [
                {
                    "claim": "The photograph was taken in Edinburgh.",
                    "verdict": "consistent_with_kb",
                    "explanation": "Retrieved passages describe Edinburgh landmarks consistent with the image.",
                    "confidence": 0.85
                }
            ],
            "model_used": "qwen2.5:7b-instruct",
            "methodology": "Knowledge base retrieval match against local preliminary corpus",
            "summary": "Analysed 1 claim. 1 consistent with reference material."
        }"#;
        let result: ClaimCheckResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.overall_verdict, "consistent_with_kb");
        assert_eq!(result.claims.len(), 1);
        assert_eq!(result.claims[0].verdict, "consistent_with_kb");
        assert!((result.claims[0].confidence - 0.85).abs() < 0.001);
        assert_eq!(result.model_used, "qwen2.5:7b-instruct");
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn test_claim_check_result_deserialise_legacy_vocabulary() {
        // Backwards-compatibility: older sidecar builds may still emit
        // "supported" / "disputed" / "unverified". The Rust deserialiser
        // treats verdict/overall_verdict as opaque strings, so the old
        // tokens must still round-trip without error — they're aliases.
        let json = r#"{
            "overall_verdict": "supported",
            "claims": [
                {
                    "claim": "Legacy build test.",
                    "verdict": "supported",
                    "explanation": "Legacy token round-trip check.",
                    "confidence": 0.75
                }
            ],
            "model_used": "qwen2.5:7b-instruct",
            "methodology": "Legacy RAG fixture",
            "summary": "Legacy summary."
        }"#;
        let result: ClaimCheckResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.overall_verdict, "supported");
        assert_eq!(result.claims[0].verdict, "supported");
    }

    // ── AudioDeepfakeResult ───────────────────────────────────────────────────

    #[test]
    fn test_audio_deepfake_result_model_not_loaded() {
        // Typical response before trained probes are deployed — model_not_loaded path.
        let json = r#"{
            "score": null,
            "verdict": "model_not_loaded",
            "stage1_score": null,
            "stage2_score": null,
            "stages_available": [],
            "model_loaded": false,
            "duration_seconds": 3.712,
            "sample_rate": 16000,
            "mfcc_features_extracted": true,
            "wav2vec2_embedding_extracted": false,
            "processing_time_ms": 42.3
        }"#;
        let result: AudioDeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.verdict, "model_not_loaded");
        assert!(!result.model_loaded);
        assert!(result.score.is_none());
        assert!(result.stage1_score.is_none());
        assert!(result.stages_available.is_empty());
        assert!(result.mfcc_features_extracted);
        assert!(!result.wav2vec2_embedding_extracted);
        assert!((result.duration_seconds.unwrap() - 3.712).abs() < 0.001);
        assert_eq!(result.sample_rate, Some(16_000));
        assert!((result.processing_time_ms.unwrap() - 42.3).abs() < 0.1);
    }

    #[test]
    fn test_audio_deepfake_result_snake_case_aliases() {
        // Sidecar sends snake_case; camelCase rename_all must still accept it via alias.
        let json = r#"{
            "score": 0.78,
            "verdict": "likely_synthetic",
            "stage1_score": 0.65,
            "stage2_score": 0.83,
            "stages_available": ["stage1", "stage2"],
            "model_loaded": true,
            "duration_seconds": 5.0,
            "sample_rate": 16000,
            "mfcc_features_extracted": true,
            "wav2vec2_embedding_extracted": true,
            "processing_time_ms": 175.0
        }"#;
        let result: AudioDeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.verdict, "likely_synthetic");
        assert!(result.model_loaded);
        assert!((result.score.unwrap() - 0.78).abs() < 0.001);
        assert!((result.stage1_score.unwrap() - 0.65).abs() < 0.001);
        assert!((result.stage2_score.unwrap() - 0.83).abs() < 0.001);
        assert_eq!(result.stages_available, vec!["stage1", "stage2"]);
        assert!(result.mfcc_features_extracted);
        assert!(result.wav2vec2_embedding_extracted);
    }

    #[test]
    fn test_audio_deepfake_result_stage1_only() {
        // Only Stage 1 ran (Wav2Vec2 model not downloaded yet).
        let json = r#"{
            "score": 0.31,
            "verdict": "authentic",
            "stage1_score": 0.31,
            "stage2_score": null,
            "stages_available": ["stage1"],
            "model_loaded": true,
            "duration_seconds": 2.5,
            "sample_rate": 16000,
            "mfcc_features_extracted": true,
            "wav2vec2_embedding_extracted": false,
            "processing_time_ms": 22.1
        }"#;
        let result: AudioDeepfakeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.verdict, "authentic");
        assert!(result.model_loaded);
        assert!(result.stage2_score.is_none());
        assert_eq!(result.stages_available, vec!["stage1"]);
        assert!(!result.wav2vec2_embedding_extracted);
    }

    // ── File cache ──────────────────────────────────────────────────

    /// Cache stores bytes and clear_file_cache resets it without panicking.
    #[test]
    fn file_cache_stores_and_clears() {
        let client = SidecarClient::new("http://localhost:0", "");
        let path = std::path::PathBuf::from("/tmp/test_image.png");
        let bytes = vec![0u8, 1, 2, 3, 4, 5];

        // Store bytes in cache
        client.cache_file_bytes(&path, bytes.clone());

        // Verify the cache is populated
        {
            let guard = client.file_cache.lock().unwrap();
            assert!(
                guard.is_some(),
                "cache should be populated after cache_file_bytes"
            );
            let (cached_path, cached_bytes) = guard.as_ref().unwrap();
            assert_eq!(cached_path, &path);
            assert_eq!(cached_bytes, &bytes);
        }

        // Clear the cache — must not panic
        client.clear_file_cache();

        // Verify the cache is now empty
        let guard = client.file_cache.lock().unwrap();
        assert!(
            guard.is_none(),
            "cache should be None after clear_file_cache"
        );
    }

    /// Cloned clients share the same Arc, so clearing on one clears the other.
    #[test]
    fn file_cache_clone_shares_arc() {
        let client = SidecarClient::new("http://localhost:0", "");
        let path = std::path::PathBuf::from("/tmp/shared_cache.png");
        let bytes = vec![10u8, 20, 30];

        client.cache_file_bytes(&path, bytes.clone());

        // Clone shares the same Arc<Mutex<...>>
        let cloned = client.clone();

        // Verify clone sees the cached data
        {
            let guard = cloned.file_cache.lock().unwrap();
            assert!(guard.is_some(), "clone should see cached bytes");
            let (_, cached_bytes) = guard.as_ref().unwrap();
            assert_eq!(cached_bytes, &bytes);
        }

        // Clear on the clone
        cloned.clear_file_cache();

        // Original client's cache must also be cleared (same Arc)
        let guard = client.file_cache.lock().unwrap();
        assert!(
            guard.is_none(),
            "original client cache must be None after clearing on clone"
        );
    }

    /// build_image_form uses cached bytes when the path matches.
    #[test]
    fn build_image_form_uses_cache_on_hit() {
        // Write a small PNG to a temp file so build_image_form has a valid path
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("cached.png");
        let img = image::RgbImage::from_fn(4, 4, |_, _| image::Rgb([255u8, 0, 0]));
        img.save(&file_path).unwrap();

        // Read its bytes, then cache a different (distinctive) payload under the same path
        let sentinel_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let client = SidecarClient::new("http://localhost:0", "");
        client.cache_file_bytes(&file_path, sentinel_bytes.clone());

        // build_image_form should return Ok (it constructs the multipart from the
        // cached bytes, so no disk read failure possible here)
        let result = client.build_image_form(&file_path);
        assert!(
            result.is_ok(),
            "build_image_form with cache hit should succeed: {:?}",
            result.err()
        );

        // After clearing the cache, it falls back to disk and should still succeed
        client.clear_file_cache();
        let result_after_clear = client.build_image_form(&file_path);
        assert!(
            result_after_clear.is_ok(),
            "build_image_form fallback to disk should succeed: {:?}",
            result_after_clear.err()
        );
    }

    /// build_image_form reads from disk when the cached path does not match.
    #[test]
    fn build_image_form_falls_back_to_disk_on_cache_miss() {
        let dir = tempfile::tempdir().unwrap();

        // Path A — cached but NOT the path we call build_image_form with
        let path_a = dir.path().join("path_a.png");
        let img_a = image::RgbImage::from_fn(4, 4, |_, _| image::Rgb([0u8, 255, 0]));
        img_a.save(&path_a).unwrap();

        // Path B — a real file on disk, not in the cache
        let path_b = dir.path().join("path_b.png");
        let img_b = image::RgbImage::from_fn(4, 4, |_, _| image::Rgb([0u8, 0, 255]));
        img_b.save(&path_b).unwrap();

        let client = SidecarClient::new("http://localhost:0", "");
        // Cache bytes for path A only
        client.cache_file_bytes(&path_a, vec![1u8, 2, 3]);

        // Request form for path B — must fall back to disk, not use path A's cache
        let result = client.build_image_form(&path_b);
        assert!(
            result.is_ok(),
            "build_image_form should read path_b from disk on cache miss: {:?}",
            result.err()
        );
    }

    /// `cache_file_bytes` does NOT populate the cache when the payload exceeds
    /// `SIDECAR_FILE_CACHE_MAX_BYTES`.  Builds the form from disk instead.
    #[test]
    fn cache_file_bytes_skips_oversized_payload() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("small.png");
        let img = image::RgbImage::from_fn(4, 4, |_, _| image::Rgb([128u8, 0, 128]));
        img.save(&file_path).unwrap();

        let client = SidecarClient::new("http://localhost:0", "");

        // Attempt to cache a synthetic payload that exceeds the 20 MiB threshold.
        let oversized = vec![0u8; (SIDECAR_FILE_CACHE_MAX_BYTES + 1) as usize];
        client.cache_file_bytes(&file_path, oversized);

        // Cache must remain empty.
        let guard = client.file_cache.lock().unwrap();
        assert!(
            guard.is_none(),
            "oversized payload must not be stored in the file cache"
        );
        drop(guard);

        // build_image_form must still succeed by reading from disk.
        let result = client.build_image_form(&file_path);
        assert!(
            result.is_ok(),
            "build_image_form must read from disk when cache is empty: {:?}",
            result.err()
        );
    }

    /// `cache_file_bytes` DOES populate the cache when the payload is exactly
    /// at the limit (20 MiB — boundary condition).
    #[test]
    fn cache_file_bytes_accepts_at_limit_payload() {
        let client = SidecarClient::new("http://localhost:0", "");
        let dummy_path = std::path::PathBuf::from("/tmp/dummy.png");
        let at_limit = vec![0u8; SIDECAR_FILE_CACHE_MAX_BYTES as usize];
        client.cache_file_bytes(&dummy_path, at_limit);

        let guard = client.file_cache.lock().unwrap();
        assert!(
            guard.is_some(),
            "payload at exactly the cap should be stored in the file cache"
        );
    }
}
