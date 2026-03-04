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
    pub ela: bool,
    #[serde(default)]
    pub noise: bool,
    #[serde(default, alias = "copy_move")]
    pub copy_move: bool,
    pub deepfake: bool,
    pub rag: bool,
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
    #[serde(alias = "ela_image_base64")]
    pub ela_image_base64: String,
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
    #[serde(alias = "heatmap_base64")]
    pub heatmap_base64: String,
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
    #[serde(alias = "visualisation_base64")]
    pub visualisation_base64: String,
    #[serde(alias = "clone_regions")]
    pub clone_regions: Vec<CloneRegion>,
    #[serde(alias = "matched_pairs")]
    pub matched_pairs: u32,
    pub score: f64,
    pub suspicious: bool,
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
    pub signals: Vec<DeepfakeSignal>,
    #[serde(alias = "heatmap_base64")]
    pub heatmap_base64: String,
    pub summary: String,
    #[serde(default)]
    pub watermarks: Vec<WatermarkDetection>,
}

/// HTTP client for the Python ML sidecar.
pub struct SidecarClient {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl SidecarClient {
    /// Create a new sidecar client pointing at the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("failed to build HTTP client");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
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
    pub fn detect_deepfake(&self, image_path: &Path) -> Result<DeepfakeResult, String> {
        let form = self.build_image_form(image_path)?;

        let resp = self
            .client
            .post(format!("{}/forensics/deepfake", self.base_url))
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

    /// Build a multipart form with an image file.
    fn build_image_form(
        &self,
        image_path: &Path,
    ) -> Result<reqwest::blocking::multipart::Form, String> {
        let file_bytes = std::fs::read(image_path)
            .map_err(|e| format!("Failed to read image {}: {e}", image_path.display()))?;

        let file_name = image_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "image.bin".to_string());

        let part = reqwest::blocking::multipart::Part::bytes(file_bytes)
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
        let client = SidecarClient::new("http://127.0.0.1:8200");
        assert_eq!(client.base_url, "http://127.0.0.1:8200");
    }

    #[test]
    fn test_client_strips_trailing_slash() {
        let client = SidecarClient::new("http://127.0.0.1:8200/");
        assert_eq!(client.base_url, "http://127.0.0.1:8200");
    }

    #[test]
    fn test_is_available_when_offline() {
        // No sidecar running — should return false, not panic
        let client = SidecarClient::new("http://127.0.0.1:19999");
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
        let client = SidecarClient::new("http://127.0.0.1:19999");
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
}
