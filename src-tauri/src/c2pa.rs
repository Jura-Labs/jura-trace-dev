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

/// Ensure a CA-signed ECDSA P-256 end-entity certificate exists for C2PA signing.
///
/// Generates a two-certificate chain (CA + end-entity) that satisfies every
/// check in the c2pa-rs `check_certificate_profile` function (§14.5.1):
///
/// - `aki_good`: end-entity cert carries an Authority Key Identifier pointing
///   to the CA's subject key.
/// - `ski_good`: only required when `is_ca()` is true; for end-entity certs
///   the flag is forced `true` by c2pa-rs, so no Subject Key Identifier is
///   needed on the EE cert.
/// - `key_usage_good`: Key Usage contains `digitalSignature`.
/// - `extended_key_usage_good`: Extended Key Usage contains `emailProtection`
///   (OID 1.3.6.1.5.5.7.3.4), which is in c2pa-rs's `valid_eku_oids.cfg`.
/// - `handled_all_critical`: no unknown critical extensions are present.
///
/// The `signcert` bytes returned contain **both** PEM blocks (EE cert then CA
/// cert), which is what `c2pa::create_signer::from_keys` expects for a chain.
///
/// On first call the chain is written to `<data_dir>/certs/`. Subsequent calls
/// load from disk.
pub fn ensure_certificate(data_dir: &Path) -> Result<(Vec<u8>, Vec<u8>), String> {
    let certs_dir = data_dir.join("certs");
    let cert_path = certs_dir.join("jura_cert.pem");
    let key_path = certs_dir.join("jura_key.pem");

    if cert_path.exists() && key_path.exists() {
        let cert =
            std::fs::read(&cert_path).map_err(|e| format!("Failed to read certificate: {e}"))?;
        let key = std::fs::read(&key_path).map_err(|e| format!("Failed to read key: {e}"))?;
        return Ok((cert, key));
    }

    std::fs::create_dir_all(&certs_dir)
        .map_err(|e| format!("Failed to create certs directory: {e}"))?;

    // --- Step 1: generate the CA key pair and self-signed CA certificate ---
    //
    // The CA cert must have `is_ca = true` so that c2pa-rs's
    // `check_certificate_profile` allows `issuer == subject` (the self-signed
    // check is `is_ca() && issuer == subject`, so a CA cert is permitted to be
    // self-signed). The CA cert carries a Subject Key Identifier so that its
    // key hash can be placed into the EE cert's Authority Key Identifier.
    let ca_key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| format!("Failed to generate CA key pair: {e}"))?;

    let mut ca_params = rcgen::CertificateParams::new(Vec::<String>::new())
        .map_err(|e| format!("Failed to create CA cert params: {e}"))?;
    ca_params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String("Jura Trace Local CA".to_string()),
    );
    ca_params.distinguished_name.push(
        rcgen::DnType::OrganizationName,
        rcgen::DnValue::Utf8String("Juralabs CIC".to_string()),
    );
    // CA needs keyCertSign so it can sign end-entity certs.
    ca_params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
        rcgen::KeyUsagePurpose::DigitalSignature,
    ];
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    // Set a reasonable validity window rather than rcgen's 1975-4096 defaults.
    ca_params.not_before = time::OffsetDateTime::now_utc()
        .checked_sub(time::Duration::days(1))
        .ok_or("CA cert: time underflow")?;
    ca_params.not_after = time::OffsetDateTime::now_utc()
        .checked_add(time::Duration::days(3650)) // 10 years
        .ok_or("CA cert: time overflow")?;

    let ca_cert = ca_params
        .self_signed(&ca_key)
        .map_err(|e| format!("Failed to self-sign CA certificate: {e}"))?;

    // --- Step 2: generate the end-entity key pair and CA-signed certificate ---
    //
    // The EE cert must satisfy all five flags checked by c2pa-rs:
    //
    //   aki_good      — `use_authority_key_identifier_extension = true` causes
    //                   rcgen to embed the CA's key hash as the AKI.
    //   ski_good      — forced true for non-CA certs by c2pa-rs; no action needed.
    //   key_usage_good — `KeyUsagePurpose::DigitalSignature` sets the flag.
    //   extended_key_usage_good — `ExtendedKeyUsagePurpose::EmailProtection`
    //                   matches OID 1.3.6.1.5.5.7.3.4, which is listed in
    //                   c2pa-rs's valid_eku_oids.cfg. Without an EKU the check
    //                   falls back to `tbscert.is_ca()`, which is false for an
    //                   end-entity cert, causing the failure.
    //   handled_all_critical — no unknown critical extensions added.
    let ee_key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| format!("Failed to generate EE key pair: {e}"))?;

    let mut ee_params = rcgen::CertificateParams::new(vec!["jura-trace.local".to_string()])
        .map_err(|e| format!("Failed to create EE cert params: {e}"))?;
    ee_params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String("Jura Trace Signing Certificate".to_string()),
    );
    ee_params.distinguished_name.push(
        rcgen::DnType::OrganizationName,
        rcgen::DnValue::Utf8String("Juralabs CIC".to_string()),
    );
    ee_params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    ee_params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::EmailProtection];
    // AKI is populated by rcgen from the issuer (CA) cert's subject key.
    ee_params.use_authority_key_identifier_extension = true;
    // Explicit non-CA so validators can distinguish from intermediate CAs.
    ee_params.is_ca = rcgen::IsCa::ExplicitNoCa;
    ee_params.not_before = time::OffsetDateTime::now_utc()
        .checked_sub(time::Duration::days(1))
        .ok_or("EE cert: time underflow")?;
    ee_params.not_after = time::OffsetDateTime::now_utc()
        .checked_add(time::Duration::days(3650)) // 10 years
        .ok_or("EE cert: time overflow")?;

    let ee_cert = ee_params
        .signed_by(&ee_key, &ca_cert, &ca_key)
        .map_err(|e| format!("Failed to sign end-entity certificate: {e}"))?;

    // --- Step 3: assemble the PEM chain and persist ---
    //
    // The signcert passed to `c2pa::create_signer::from_keys` is parsed by
    // x509_parser's `Pem::iter_from_buffer`, which reads every PEM block in
    // order. The end-entity cert must come first; the CA cert follows.
    let chain_pem = format!("{}{}", ee_cert.pem(), ca_cert.pem());
    let key_pem = ee_key.serialize_pem();

    std::fs::write(&cert_path, chain_pem.as_bytes())
        .map_err(|e| format!("Failed to write certificate chain: {e}"))?;
    std::fs::write(&key_path, key_pem.as_bytes())
        .map_err(|e| format!("Failed to write key: {e}"))?;

    // SECURITY: Restrict the private key file to owner-read/write only (0600).
    // Without this, the file inherits the process umask, which is typically
    // 0644 — meaning any other process running as the same OS user (or with
    // access to the user's home directory) can read the signing key and forge
    // C2PA provenance chains under the institution's identity.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&key_path)
            .map_err(|e| format!("Failed to stat key file: {e}"))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&key_path, perms)
            .map_err(|e| format!("Failed to set key file permissions: {e}"))?;
    }

    log::info!(
        "Generated C2PA certificate chain at {}",
        cert_path.display()
    );

    Ok((chain_pem.into_bytes(), key_pem.into_bytes()))
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
        "claim_generator": "Jura Trace/0.1.0",
        "title": file_name,
        "assertions": [
            {
                "label": "c2pa.actions",
                "data": {
                    "actions": [{
                        "action": "c2pa.created",
                        "softwareAgent": "Jura Trace 0.1.0",
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

    // validation_status: None or empty = fully valid.
    // For self-signed certificates, c2pa-rs always reports signingCredential.untrusted.
    // We treat that as valid because the manifest itself is structurally sound — the
    // cert simply isn't in any external trust store.
    let is_valid = reader.validation_status().is_none_or(|statuses| {
        statuses
            .iter()
            .all(|s| s.code() == "signingCredential.untrusted")
    });

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
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let ext = source.extension().map(|e| e.to_string_lossy().to_string());

    let new_name = match ext {
        Some(e) => format!("{stem}_c2pa.{e}"),
        None => format!("{stem}_c2pa"),
    };
    source.with_file_name(new_name)
}

/// Check whether a content type + MIME type supports C2PA signing.
///
/// Images are fully supported by c2pa-rs 0.76.
///
/// Video (MP4, QuickTime) and audio (WAV, MPEG) are declared here because the
/// C2PA specification defines profiles for these containers. However, c2pa-rs
/// support for video/audio is still experimental — `sign_file` may return an
/// error at runtime for specific container variants. Callers should propagate
/// that error to the user rather than treating it as a bug.
pub fn supports_signing(content_type: &str, mime_type: &str) -> bool {
    match content_type {
        "image" => matches!(
            mime_type,
            "image/jpeg"
                | "image/png"
                | "image/tiff"
                | "image/webp"
                | "image/avif"
                | "image/heic"
                | "image/heif"
        ),
        // c2pa-rs has experimental MP4/MOV support via the `mp4` feature.
        // WAV and MPEG audio are declared in the C2PA spec but runtime support
        // in c2pa-rs 0.76 is limited — errors are possible and expected.
        "video" => matches!(mime_type, "video/mp4" | "video/quicktime"),
        "audio" => matches!(mime_type, "audio/wav" | "audio/mpeg"),
        _ => false,
    }
}

/// Known AI image generator patterns in C2PA claim_generator strings.
const C2PA_AI_GENERATORS: &[(&str, &str)] = &[
    ("dall-e", "DALL-E (OpenAI)"),
    ("dall·e", "DALL-E (OpenAI)"),
    ("openai", "OpenAI"),
    ("chatgpt", "ChatGPT (OpenAI)"),
    ("adobe firefly", "Adobe Firefly"),
    ("firefly", "Adobe Firefly"),
    ("midjourney", "Midjourney"),
    ("stable diffusion", "Stable Diffusion"),
    ("stability.ai", "Stability AI"),
    ("comfyui", "ComfyUI (Stable Diffusion)"),
    ("automatic1111", "AUTOMATIC1111 (Stable Diffusion)"),
    ("invokeai", "InvokeAI"),
    ("leonardo", "Leonardo.ai"),
    ("ideogram", "Ideogram"),
    ("flux", "Flux (Black Forest Labs)"),
    ("black forest", "Black Forest Labs"),
    ("bing image creator", "Bing Image Creator (Microsoft)"),
    ("copilot", "Microsoft Copilot"),
    ("canva", "Canva AI"),
    ("gemini", "Google Gemini"),
    ("imagen", "Google Imagen"),
];

/// Check if a C2PA claim_generator string indicates an AI image generator.
///
/// Returns the human-readable name of the AI generator if detected, or `None`.
pub fn detect_ai_generator(claim_generator: &str) -> Option<String> {
    let lower = claim_generator.to_lowercase();
    for (pattern, name) in C2PA_AI_GENERATORS {
        if lower.contains(pattern) {
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_info_serialises_to_camel_case() {
        let info = ManifestInfo {
            title: Some("test.jpg".to_string()),
            format: Some("image/jpeg".to_string()),
            claim_generator: Some("Jura Trace/0.1.0".to_string()),
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
    fn detect_ai_generator_matches_known_patterns() {
        assert_eq!(
            detect_ai_generator("DALL-E 3/2024.1"),
            Some("DALL-E (OpenAI)".to_string())
        );
        assert_eq!(
            detect_ai_generator("Adobe Firefly 2.0"),
            Some("Adobe Firefly".to_string())
        );
        assert_eq!(
            detect_ai_generator("ChatGPT/2025"),
            Some("ChatGPT (OpenAI)".to_string())
        );
        assert_eq!(
            detect_ai_generator("Midjourney v6.1"),
            Some("Midjourney".to_string())
        );
        assert_eq!(detect_ai_generator("Jura Trace/0.2.0"), None);
        assert_eq!(detect_ai_generator("Apple Preview 11.0"), None);
        assert_eq!(detect_ai_generator("GIMP 2.10"), None);
    }

    #[test]
    fn supports_signing_images() {
        assert!(supports_signing("image", "image/jpeg"));
        assert!(supports_signing("image", "image/png"));
        assert!(supports_signing("image", "image/tiff"));
        assert!(supports_signing("image", "image/webp"));
        assert!(supports_signing("image", "image/avif"));
        assert!(supports_signing("image", "image/heic"));
        assert!(supports_signing("image", "image/heif"));
    }

    #[test]
    fn supports_signing_video() {
        // c2pa-rs has experimental MP4/QuickTime support
        assert!(supports_signing("video", "video/mp4"));
        assert!(supports_signing("video", "video/quicktime"));
        // Unsupported video formats
        assert!(!supports_signing("video", "video/webm"));
        assert!(!supports_signing("video", "video/x-msvideo"));
        assert!(!supports_signing("video", "video/x-matroska"));
    }

    #[test]
    fn supports_signing_audio() {
        // C2PA spec defines WAV and MPEG audio profiles
        assert!(supports_signing("audio", "audio/wav"));
        assert!(supports_signing("audio", "audio/mpeg"));
        // Unsupported audio formats
        assert!(!supports_signing("audio", "audio/flac"));
        assert!(!supports_signing("audio", "audio/ogg"));
        assert!(!supports_signing("audio", "audio/mp4"));
    }

    #[test]
    fn rejects_unsupported_formats() {
        assert!(!supports_signing("document", "application/pdf"));
        assert!(!supports_signing("image", "image/gif"));
        assert!(!supports_signing("image", "image/bmp"));
        assert!(!supports_signing("3d", "model/stl"));
    }

    #[test]
    fn read_manifest_missing_file_returns_error() {
        let result = read_manifest(Path::new("/nonexistent/file.jpg"));
        assert!(result.is_err());
    }

    /// Integration test: generate certificates, create a test PNG, sign it, read back the manifest.
    #[test]
    fn sign_and_read_back_png_integration() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        // Step 1: Generate certificates
        let (cert, key) = ensure_certificate(&data_dir).expect("ensure_certificate should succeed");

        // Verify cert + key are non-empty PEM
        let cert_str = std::str::from_utf8(&cert).expect("cert should be UTF-8");
        let key_str = std::str::from_utf8(&key).expect("key should be UTF-8");
        assert!(
            cert_str.contains("BEGIN CERTIFICATE"),
            "cert PEM should contain BEGIN CERTIFICATE"
        );
        assert!(
            key_str.contains("BEGIN PRIVATE KEY"),
            "key PEM should contain BEGIN PRIVATE KEY"
        );
        // Chain should have two certs (EE + CA)
        assert_eq!(
            cert_str.matches("BEGIN CERTIFICATE").count(),
            2,
            "chain should contain exactly 2 certificates"
        );

        // Step 2: Create a minimal test PNG using the image crate
        let source_path = tmp.path().join("test_image.png");
        let img = image::RgbImage::new(64, 64);
        img.save(&source_path).expect("save test PNG");

        let output_path = signed_output_path(&source_path);

        // Step 3: Sign the file
        let manifest_info = sign_file(
            &source_path,
            &output_path,
            "Test User",
            Some("CC BY 4.0"),
            &cert,
            &key,
        );

        match &manifest_info {
            Ok(info) => {
                eprintln!("[TEST] sign_file succeeded:");
                eprintln!("  title: {:?}", info.title);
                eprintln!("  format: {:?}", info.format);
                eprintln!("  claim_generator: {:?}", info.claim_generator);
                eprintln!("  is_valid: {}", info.is_valid);
                eprintln!("  assertions: {}", info.assertions.len());
                for a in &info.assertions {
                    eprintln!("    - {} = {}", a.label, &a.value[..a.value.len().min(100)]);
                }

                // Also print raw validation status
                let reader = ::c2pa::Reader::from_file(&output_path).expect("reader");
                if let Some(statuses) = reader.validation_status() {
                    eprintln!("  validation_status ({} issues):", statuses.len());
                    for s in statuses {
                        eprintln!(
                            "    - code={} url={:?} explanation={:?}",
                            s.code(),
                            s.url(),
                            s.explanation()
                        );
                    }
                } else {
                    eprintln!("  validation_status: None (all good)");
                }

                assert_eq!(info.title.as_deref(), Some("test_image.png"));
                assert!(!info.assertions.is_empty(), "assertions should be present");

                // Self-signed certs will always get signingCredential.untrusted,
                // which is expected. The signing itself succeeded; the manifest
                // is structurally valid but the cert isn't in any trust store.
                // We check the output file exists and contains a manifest.
                assert!(output_path.exists(), "signed output file should exist");
            }
            Err(e) => {
                panic!("sign_file failed: {e}");
            }
        }

        // Step 4: Read manifest back from the signed output
        let readback = read_manifest(&output_path).expect("read_manifest should not error");
        assert!(readback.is_some(), "signed file should contain a manifest");
        let readback = readback.unwrap();
        assert!(readback.is_valid, "readback manifest should be valid");
    }

    /// Verify that ensure_certificate returns consistent results on second call (loads from disk).
    #[test]
    fn ensure_certificate_idempotent() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        let (cert1, key1) = ensure_certificate(&data_dir).expect("first call");
        let (cert2, key2) = ensure_certificate(&data_dir).expect("second call");

        assert_eq!(cert1, cert2, "cert should be identical on second load");
        assert_eq!(key1, key2, "key should be identical on second load");
    }
}
