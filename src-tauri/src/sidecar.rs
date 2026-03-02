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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub ela: bool,
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
    pub ela_image_base64: String,
    pub max_difference: f64,
    pub mean_difference: f64,
    pub score: f64,
    pub suspicious: bool,
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

        let form = reqwest::blocking::multipart::Form::new().part("file", part);

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
    fn test_ela_result_deserialise() {
        let json = r#"{
            "elaImageBase64": "iVBOR...",
            "maxDifference": 42.5,
            "meanDifference": 8.3,
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
}
