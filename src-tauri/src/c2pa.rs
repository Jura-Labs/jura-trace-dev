//! C2PA Content Credentials — signing, reading, and verification.
//!
//! All operations are local. No network calls to external services.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ===== Types =====

/// Information about a C2PA manifest embedded in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestInfo {
    pub title: Option<String>,
    pub format: Option<String>,
    pub claim_generator: Option<String>,
    pub assertions: Vec<AssertionInfo>,
    pub is_valid: bool,
    pub signed_at: Option<String>,
}

/// A single assertion within a C2PA manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionInfo {
    pub label: String,
    pub value: String,
}

// ===== Certificate management =====

/// Ensure a self-signed ECDSA P-256 certificate exists for C2PA signing.
///
/// On first use, generates a certificate and saves it to `<data_dir>/certs/`.
/// Subsequent calls load the existing certificate.
pub fn ensure_certificate(data_dir: &Path) -> Result<(Vec<u8>, Vec<u8>), String> {
    let certs_dir = data_dir.join("certs");
    let cert_path = certs_dir.join("jura_cert.pem");
    let key_path = certs_dir.join("jura_key.pem");

    if cert_path.exists() && key_path.exists() {
        let cert = std::fs::read(&cert_path)
            .map_err(|e| format!("Failed to read certificate: {e}"))?;
        let key = std::fs::read(&key_path)
            .map_err(|e| format!("Failed to read key: {e}"))?;
        return Ok((cert, key));
    }

    std::fs::create_dir_all(&certs_dir)
        .map_err(|e| format!("Failed to create certs directory: {e}"))?;

    let key_pair = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| format!("Failed to generate key pair: {e}"))?;

    let mut params = rcgen::CertificateParams::new(vec!["Jura Archive".to_string()])
        .map_err(|e| format!("Failed to create cert params: {e}"))?;
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String("Jura Archive Self-Signed".to_string()),
    );
    params.distinguished_name.push(
        rcgen::DnType::OrganizationName,
        rcgen::DnValue::Utf8String("Juralabs CIC".to_string()),
    );

    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("Failed to self-sign certificate: {e}"))?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    std::fs::write(&cert_path, cert_pem.as_bytes())
        .map_err(|e| format!("Failed to write certificate: {e}"))?;
    std::fs::write(&key_path, key_pem.as_bytes())
        .map_err(|e| format!("Failed to write key: {e}"))?;

    log::info!(
        "Generated self-signed C2PA certificate at {}",
        cert_path.display()
    );

    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

// ===== Signing =====

/// Sign a file with C2PA Content Credentials.
///
/// Creates a new file at `output` with an embedded C2PA manifest containing
/// the specified creator information, licence, and AI training opt-out.
pub fn sign_file(
    source: &Path,
    output: &Path,
    creator_name: &str,
    license: Option<&str>,
    cert: &[u8],
    key: &[u8],
) -> Result<ManifestInfo, String> {
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let license_value = license.unwrap_or("All Rights Reserved");

    let manifest_def = serde_json::json!({
        "claim_generator": "Jura Archive/0.1.0",
        "title": file_name,
        "assertions": [
            {
                "label": "c2pa.actions",
                "data": {
                    "actions": [{
                        "action": "c2pa.created",
                        "softwareAgent": "Jura Archive 0.1.0",
                        "parameters": {
                            "name": creator_name
                        }
                    }]
                }
            },
            {
                "label": "c2pa.rights",
                "data": {
                    "rights": license_value,
                    "ai_training": "notAllowed"
                }
            }
        ]
    });

    let mut builder = c2pa::Builder::from_json(&manifest_def.to_string())
        .map_err(|e| format!("Failed to create C2PA builder: {e}"))?;

    let signer = c2pa::create_signer::from_keys(cert, key, c2pa::SigningAlg::Es256, None)
        .map_err(|e| format!("Failed to create signer: {e}"))?;

    builder
        .sign_file(&*signer, source, output)
        .map_err(|e| format!("Failed to sign file: {e}"))?;

    // Read back the manifest we just created
    read_manifest(output)?.ok_or_else(|| "Signed file but could not read back manifest".to_string())
}

// ===== Reading =====

/// Read a C2PA manifest from a file.
///
/// Returns `None` if the file contains no C2PA manifest (not an error).
pub fn read_manifest(path: &Path) -> Result<Option<ManifestInfo>, String> {
    let reader = match c2pa::Reader::from_file(path) {
        Ok(r) => r,
        Err(c2pa::Error::JumbfNotFound) => return Ok(None),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("jumbf") || msg.contains("not found") || msg.contains("no c2pa") {
                return Ok(None);
            }
            return Err(format!("Failed to read C2PA manifest: {e}"));
        }
    };

    // Parse the JSON representation — more robust than struct methods
    let json_str = reader.json();
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse manifest JSON: {e}"))?;

    let active_label = match json.get("active_manifest").and_then(|v| v.as_str()) {
        Some(l) => l.to_string(),
        None => return Ok(None),
    };

    let manifest = json
        .get("manifests")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get(&active_label));

    let manifest = match manifest {
        Some(m) => m,
        None => return Ok(None),
    };

    let title = manifest
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from);
    let format = manifest
        .get("format")
        .and_then(|v| v.as_str())
        .map(String::from);
    let claim_generator = manifest
        .get("claim_generator")
        .and_then(|v| v.as_str())
        .map(String::from);

    let mut assertions = Vec::new();
    if let Some(arr) = manifest.get("assertions").and_then(|v| v.as_array()) {
        for a in arr {
            let label = a
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let data = a
                .get("data")
                .map(|d| serde_json::to_string_pretty(d).unwrap_or_default())
                .unwrap_or_default();
            assertions.push(AssertionInfo { label, value: data });
        }
    }

    // validation_status: None or empty = valid
    let is_valid = reader
        .validation_status()
        .is_none_or(|statuses| statuses.is_empty());

    let signed_at = manifest
        .get("signature_info")
        .and_then(|si| si.get("time"))
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Some(ManifestInfo {
        title,
        format,
        claim_generator,
        assertions,
        is_valid,
        signed_at,
    }))
}

// ===== Utilities =====

/// Determine the output path for a signed file.
///
/// Adds `_c2pa` before the file extension: `photo.jpg` -> `photo_c2pa.jpg`
pub fn signed_output_path(source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let ext = source
        .extension()
        .map(|e| e.to_string_lossy().to_string());

    let new_name = match ext {
        Some(e) => format!("{stem}_c2pa.{e}"),
        None => format!("{stem}_c2pa"),
    };
    source.with_file_name(new_name)
}

/// Check whether a content type + MIME type supports C2PA signing.
pub fn supports_signing(content_type: &str, mime_type: &str) -> bool {
    content_type == "image"
        && matches!(
            mime_type,
            "image/jpeg"
                | "image/png"
                | "image/tiff"
                | "image/webp"
                | "image/avif"
                | "image/heic"
                | "image/heif"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_info_serialises_to_camel_case() {
        let info = ManifestInfo {
            title: Some("test.jpg".to_string()),
            format: Some("image/jpeg".to_string()),
            claim_generator: Some("Jura Archive/0.1.0".to_string()),
            assertions: vec![AssertionInfo {
                label: "c2pa.actions".to_string(),
                value: "{}".to_string(),
            }],
            is_valid: true,
            signed_at: Some("2026-01-01T00:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"isValid\""));
        assert!(json.contains("\"claimGenerator\""));
        assert!(json.contains("\"signedAt\""));
        assert!(!json.contains("\"is_valid\""));
    }

    #[test]
    fn signed_output_path_adds_suffix() {
        let source = Path::new("/photos/sunset.jpg");
        let output = signed_output_path(source);
        assert_eq!(output, PathBuf::from("/photos/sunset_c2pa.jpg"));
    }

    #[test]
    fn signed_output_path_no_extension() {
        let source = Path::new("/photos/readme");
        let output = signed_output_path(source);
        assert_eq!(output, PathBuf::from("/photos/readme_c2pa"));
    }

    #[test]
    fn supports_signing_images() {
        assert!(supports_signing("image", "image/jpeg"));
        assert!(supports_signing("image", "image/png"));
        assert!(supports_signing("image", "image/tiff"));
        assert!(supports_signing("image", "image/webp"));
        assert!(supports_signing("image", "image/avif"));
    }

    #[test]
    fn rejects_unsupported_formats() {
        assert!(!supports_signing("document", "application/pdf"));
        assert!(!supports_signing("video", "video/mp4"));
        assert!(!supports_signing("image", "image/gif"));
        assert!(!supports_signing("image", "image/bmp"));
        assert!(!supports_signing("3d", "model/stl"));
    }

    #[test]
    fn read_manifest_missing_file_returns_error() {
        let result = read_manifest(Path::new("/nonexistent/file.jpg"));
        assert!(result.is_err());
    }
}
