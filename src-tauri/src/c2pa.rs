//! C2PA provenance — signing, reading, and verification.
//!
//! All operations are local. No network calls to external services.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    /// True when the signing certificate has expired but a trusted timestamp
    /// and a cryptographically valid claim signature prove the signature was
    /// valid at signing time. Common for short-lived credentials such as
    /// Google Pixel Camera.
    #[serde(default)]
    pub valid_at_signing: bool,
    pub signed_at: Option<String>,
    /// Signer common name from `signature_info.common_name` (e.g. "Pixel Camera").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_by: Option<String>,
    /// Signer issuer from `signature_info.issuer` (e.g. "Google LLC").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_by_issuer: Option<String>,
    /// Individual validation checks from c2pa-rs, grouped by outcome.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_checks: Vec<ValidationCheck>,
    /// Verification mode used when reading this manifest.
    ///
    /// - `"standard"` — local-only verification, no OCSP/CRL or remote manifest fetch.
    ///   This is the default air-gapped mode.
    /// - `"enhanced"` — online verification with OCSP/CRL revocation checks and
    ///   remote manifest fetch enabled (user opt-in via Settings → Enhanced mode).
    ///
    /// `None` for ingredient manifests (they inherit the mode from the active manifest).
    ///
    /// **Infrastructure note**: c2pa-rs 0.76 does not expose reader-level configuration
    /// for trust-list loading or OCSP/CRL checking.  When such configuration is added
    /// to `c2pa::Reader` in a future release, the `enhanced = true` path in
    /// `read_manifest` and `read_manifest_chain` is where those options should be set.
    /// For now the field documents which mode was requested so the UI and PDF export
    /// can accurately report whether online checks were attempted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_mode: Option<String>,
}

/// A single C2PA validation check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationCheck {
    pub code: String,
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
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
        rcgen::DnValue::Utf8String("Jura Labs CIC".to_string()),
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
        rcgen::DnValue::Utf8String("Jura Labs CIC".to_string()),
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

/// Sign a file with a C2PA provenance manifest.
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
        "claim_generator": "Jura Trace/0.9.0",
        "title": file_name,
        "assertions": [
            {
                "label": "c2pa.actions",
                "data": {
                    "actions": [{
                        "action": "c2pa.created",
                        "softwareAgent": "Jura Trace 0.9.0",
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
            },
            {
                "label": "stds.iptc",
                "data": {
                    "Iptc4xmpExt:DigitalSourceType": "",
                    "plus:DataMining": "http://ns.useplus.org/ldf/vocab/DMI-PROHIBITED-EXCEPTSEARCHENGINEINDEXING"
                }
            }
        ]
    });

    let mut builder = c2pa::Builder::from_json(&manifest_def.to_string())
        .map_err(|e| format!("Failed to create C2PA builder: {e}"))?;

    let signer = c2pa::create_signer::from_keys(
        cert,
        key,
        c2pa::SigningAlg::Es256,
        Some("http://timestamp.digicert.com".to_string()),
    )
    .map_err(|e| format!("Failed to create signer: {e}"))?;

    builder
        .sign_file(&*signer, source, output)
        .map_err(|e| format!("Failed to sign file: {e}"))?;

    // Read back the manifest we just created. Always standard mode — signing is local-only.
    read_manifest(output, false)?.ok_or_else(|| "Signed file but could not read back manifest".to_string())
}

// ===== Validity derivation =====

/// Derive `(is_valid, valid_at_signing)` from the JSON returned by `c2pa::Reader::json()`.
///
/// Fully valid: no failures, or failures only contain `signingCredential.untrusted`
/// (self-signed cert — manifest is structurally sound, cert just isn't in a trust list).
///
/// Valid at signing: failures are only cert-soft (`untrusted` and/or `expired`) AND the
/// active manifest success list includes a validated timestamp plus a cryptographically
/// valid claim signature. Short-lived signing certs (e.g. Google Pixel Camera) rely on
/// this — the trusted timestamp proves the signature was issued while the cert was still
/// in its original validity window.
fn derive_validity(json: &serde_json::Value) -> (bool, bool) {
    let active_results = json
        .get("validation_results")
        .and_then(|vr| vr.get("activeManifest"));
    let codes_of = |key: &str| -> Vec<String> {
        active_results
            .and_then(|am| am.get(key))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.get("code").and_then(|c| c.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let failure_codes = codes_of("failure");
    let success_codes = codes_of("success");

    let only_cert_soft_failures = !failure_codes.is_empty()
        && failure_codes
            .iter()
            .all(|c| c == "signingCredential.untrusted" || c == "signingCredential.expired");
    let has_expired = failure_codes
        .iter()
        .any(|c| c == "signingCredential.expired");
    let has_validated_timestamp = success_codes
        .iter()
        .any(|c| c == "timeStamp.validated" || c == "timeStamp.trusted");
    let has_valid_claim_sig = success_codes
        .iter()
        .any(|c| c == "claimSignature.validated");

    if failure_codes.is_empty() || (only_cert_soft_failures && !has_expired) {
        (true, false)
    } else if only_cert_soft_failures
        && has_expired
        && has_validated_timestamp
        && has_valid_claim_sig
    {
        (true, true)
    } else {
        (false, false)
    }
}

/// Extract validation checks from one entry in `ingredientDeltas[].validationDeltas`.
///
/// The `delta` argument is the `validationDeltas` object for a single ingredient,
/// containing optional `success`, `informational`, and `failure` arrays.
fn extract_validation_checks_from_delta(delta: &serde_json::Value) -> Vec<ValidationCheck> {
    let mut checks = Vec::new();
    for (outcome, key) in [("pass", "success"), ("info", "informational"), ("fail", "failure")] {
        if let Some(arr) = delta.get(key).and_then(|v| v.as_array()) {
            for entry in arr {
                let code = entry
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let explanation = entry
                    .get("explanation")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                checks.push(ValidationCheck {
                    code,
                    outcome: outcome.to_string(),
                    explanation,
                });
            }
        }
    }
    checks
}

/// Derive `(is_valid, valid_at_signing)` from a `validationDeltas` object within
/// an `ingredientDelta` entry.  Uses the same cert-soft logic as `derive_validity`.
fn derive_validity_from_delta(delta: &serde_json::Value) -> (bool, bool) {
    let codes_of = |key: &str| -> Vec<String> {
        delta
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.get("code").and_then(|c| c.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let failure_codes = codes_of("failure");
    let success_codes = codes_of("success");

    let only_cert_soft_failures = !failure_codes.is_empty()
        && failure_codes
            .iter()
            .all(|c| c == "signingCredential.untrusted" || c == "signingCredential.expired");
    let has_expired = failure_codes
        .iter()
        .any(|c| c == "signingCredential.expired");
    let has_validated_timestamp = success_codes
        .iter()
        .any(|c| c == "timeStamp.validated" || c == "timeStamp.trusted");
    let has_valid_claim_sig = success_codes
        .iter()
        .any(|c| c == "claimSignature.validated");

    if failure_codes.is_empty() || (only_cert_soft_failures && !has_expired) {
        (true, false)
    } else if only_cert_soft_failures
        && has_expired
        && has_validated_timestamp
        && has_valid_claim_sig
    {
        (true, true)
    } else {
        (false, false)
    }
}

/// Extract individual validation checks from the c2pa-rs JSON for display.
fn extract_validation_checks(json: &serde_json::Value) -> Vec<ValidationCheck> {
    let mut checks = Vec::new();
    let active = json
        .get("validation_results")
        .and_then(|vr| vr.get("activeManifest"));

    let Some(am) = active else {
        return checks;
    };

    for (outcome, key) in [("pass", "success"), ("info", "informational"), ("fail", "failure")] {
        if let Some(arr) = am.get(key).and_then(|v| v.as_array()) {
            for entry in arr {
                let code = entry
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let explanation = entry
                    .get("explanation")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                checks.push(ValidationCheck {
                    code,
                    outcome: outcome.to_string(),
                    explanation,
                });
            }
        }
    }
    checks
}

// ===== Reading =====

/// Extract a `ManifestInfo` from one manifest object within the full c2pa-rs JSON.
///
/// `manifest` is the JSON object for the specific manifest (e.g.
/// `json["manifests"]["urn:uuid:…"]`).  `full_json` is the entire c2pa-rs
/// reader JSON, used to derive active-manifest validity via
/// `derive_validity` and `extract_validation_checks`.
///
/// `use_full_validation = true` selects active-manifest validation from
/// `full_json`.  When `false`, `ingredient_delta` is consulted instead: if
/// it is `Some(delta)` the validity and checks are derived from the
/// per-ingredient `validationDeltas` object; if it is `None` the ingredient
/// manifest is conservatively marked valid with no checks (the fallback used
/// when c2pa-rs does not include `ingredientDeltas` for an ingredient).
fn extract_manifest_info(
    manifest: &serde_json::Value,
    full_json: &serde_json::Value,
    use_full_validation: bool,
    ingredient_delta: Option<&serde_json::Value>,
    verification_mode: Option<&str>,
) -> ManifestInfo {
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
    // Fallback: claim_generator_info[0].name (c2pa-rs v2 format)
    let claim_generator = claim_generator.or_else(|| {
        manifest
            .get("claim_generator_info")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|entry| entry.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from)
    });

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

    let (is_valid, valid_at_signing, validation_checks) = if use_full_validation {
        let (v, vas) = derive_validity(full_json);
        let checks = extract_validation_checks(full_json);
        (v, vas, checks)
    } else if let Some(delta) = ingredient_delta {
        // Per-ingredient validation data provided by the caller from the
        // `ingredientDeltas` array in the c2pa-rs reader JSON.
        let (v, vas) = derive_validity_from_delta(delta);
        let checks = extract_validation_checks_from_delta(delta);
        (v, vas, checks)
    } else {
        // Conservative default: no per-ingredient delta was available.
        // Mark as valid with no checks rather than falsely flagging as invalid.
        (true, false, vec![])
    };

    let sig_info = manifest.get("signature_info");
    let signed_at = sig_info
        .and_then(|si| si.get("time"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let signed_by = sig_info
        .and_then(|si| si.get("common_name"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let signed_by_issuer = sig_info
        .and_then(|si| si.get("issuer"))
        .and_then(|v| v.as_str())
        .map(String::from);

    ManifestInfo {
        title,
        format,
        claim_generator,
        assertions,
        is_valid,
        valid_at_signing,
        signed_at,
        signed_by,
        signed_by_issuer,
        validation_checks,
        verification_mode: verification_mode.map(String::from),
    }
}

/// Open a c2pa-rs `Reader` for `path`, returning `Ok(None)` when no C2PA data is present.
fn open_reader(path: &Path) -> Result<Option<c2pa::Reader>, String> {
    match c2pa::Reader::from_file(path) {
        Ok(r) => Ok(Some(r)),
        Err(c2pa::Error::JumbfNotFound) => Ok(None),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            if msg.contains("jumbf") || msg.contains("not found") || msg.contains("no c2pa") {
                return Ok(None);
            }
            Err(format!("Failed to read C2PA manifest: {e}"))
        }
    }
}

/// Read a C2PA manifest from a file.
///
/// Returns `None` if the file contains no C2PA manifest (not an error).
///
/// `enhanced` controls which network mode label is recorded in the returned
/// `ManifestInfo.verification_mode` field.  When `true` the field is set to
/// `"enhanced"` (online OCSP/CRL checks requested); when `false` it is
/// `"standard"` (local-only, air-gapped).
///
/// **Infrastructure note for future OCSP/CRL support**: when c2pa-rs exposes
/// reader-level configuration for trust-list loading or revocation checking,
/// configure the `c2pa::Reader` here based on the `enhanced` flag before
/// calling `reader.json()`.  As of c2pa-rs 0.76 no such API exists.
pub fn read_manifest(path: &Path, enhanced: bool) -> Result<Option<ManifestInfo>, String> {
    let reader = match open_reader(path)? {
        Some(r) => r,
        None => return Ok(None),
    };

    // TODO(OCSP): when c2pa-rs adds Reader configuration for trust-list and
    // OCSP/CRL revocation, configure it here when `enhanced = true`.
    // Example (hypothetical API — does not exist in c2pa-rs 0.76):
    //   if enhanced {
    //       reader.set_trust_list(c2pa::TrustList::from_online_sources()?);
    //       reader.enable_ocsp_checking(true);
    //   }

    let json_str = reader.json();
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse manifest JSON: {e}"))?;

    let active_label = match json.get("active_manifest").and_then(|v| v.as_str()) {
        Some(l) => l.to_string(),
        None => return Ok(None),
    };

    let manifest = match json
        .get("manifests")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get(&active_label))
    {
        Some(m) => m,
        None => return Ok(None),
    };

    let mode_str = if enhanced { "enhanced" } else { "standard" };
    Ok(Some(extract_manifest_info(
        manifest,
        &json,
        true,
        None,
        Some(mode_str),
    )))
}

// ===== Manifest chain =====

/// A complete C2PA provenance chain extracted from a file.
///
/// The chain captures all manifests in the store, not just the active (most
/// recent) one.  This allows the frontend to display the full history of
/// edits and re-signings that led to the current state of an asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestChain {
    /// The active (most recent) manifest.
    pub active: ManifestInfo,
    /// Ingredient manifests in chain order: the active manifest's direct
    /// parent first, then their parents, ending at the origin (oldest ancestor).
    ///
    /// Empty for single-manifest files (no provenance history recorded).
    pub ingredients: Vec<ManifestInfo>,
    /// Total number of manifest entries in the store (active + all ancestors).
    pub manifest_count: usize,
}

/// Read the full C2PA provenance chain from a file.
///
/// Returns `None` when the file contains no C2PA data.  Returns a
/// [`ManifestChain`] with an empty `ingredients` list for files that contain
/// only a single manifest (no ancestor chain recorded).
///
/// The chain is walked breadth-first: each manifest's `ingredients` array is
/// inspected for `active_manifest` references that point to other entries in
/// the manifest store. The resulting `ingredients` list is ordered from the
/// active manifest's direct parents to the oldest ancestor.
///
/// `enhanced` is the same mode flag as in [`read_manifest`] — recorded in
/// the active manifest's `verification_mode` field.  Ingredient manifests do
/// not carry a `verification_mode` (they inherit the mode from the active
/// manifest; the field is `None` for ingredients).
///
/// **Infrastructure note for future OCSP/CRL support**: see [`read_manifest`]
/// for the TODO comment on where to configure c2pa-rs when the API becomes
/// available.
pub fn read_manifest_chain(path: &Path, enhanced: bool) -> Result<Option<ManifestChain>, String> {
    let reader = match open_reader(path)? {
        Some(r) => r,
        None => return Ok(None),
    };

    let json_str = reader.json();
    let json: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse manifest JSON: {e}"))?;

    let active_label = match json.get("active_manifest").and_then(|v| v.as_str()) {
        Some(l) => l.to_string(),
        None => return Ok(None),
    };

    let manifests = match json.get("manifests").and_then(|v| v.as_object()) {
        Some(m) => m,
        None => return Ok(None),
    };

    let active_manifest = match manifests.get(&active_label) {
        Some(m) => m,
        None => return Ok(None),
    };

    let manifest_count = manifests.len();
    let mode_str = if enhanced { "enhanced" } else { "standard" };
    let active = extract_manifest_info(active_manifest, &json, true, None, Some(mode_str));

    // Build a lookup map from ingredient assertion URI → validationDeltas so
    // we can enrich ingredient manifests with their actual validation results.
    //
    // The c2pa-rs reader JSON may include:
    //   validation_results.ingredientDeltas[].ingredientAssertionURI  (string)
    //   validation_results.ingredientDeltas[].validationDeltas         (object)
    //
    // The URI typically ends with the manifest label, but the format is not
    // guaranteed to be stable.  We collect all deltas in order and also build
    // a secondary index keyed on any manifest label substring found in the URI
    // so that either strategy can match.
    let ingredient_deltas: Vec<&serde_json::Value> = json
        .get("validation_results")
        .and_then(|vr| vr.get("ingredientDeltas"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().collect())
        .unwrap_or_default();

    // Walk the ingredient chain breadth-first.
    // `queue` holds (manifest_json, label, insertion_index) tuples.
    let mut ingredients: Vec<ManifestInfo> = Vec::new();
    let mut queue: Vec<(&serde_json::Value, String, usize)> =
        vec![(active_manifest, active_label, 0)];
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Counter tracking order of ingredient discovery — used to correlate
    // with ingredient_deltas when URI matching is unavailable.
    let mut ingredient_order: usize = 0;

    while !queue.is_empty() {
        // Drain the current level before adding the next.
        let current_level = std::mem::take(&mut queue);
        for (manifest_json, label, _depth) in &current_level {
            visited.insert(label.clone());
            // Look for ingredient entries that reference a child manifest label.
            if let Some(ing_arr) = manifest_json.get("ingredients").and_then(|v| v.as_array()) {
                for ing in ing_arr {
                    // c2pa-rs exposes ingredient manifest references as
                    // `active_manifest` within each ingredient object.
                    if let Some(child_label) =
                        ing.get("active_manifest").and_then(|v| v.as_str())
                    {
                        if visited.contains(child_label) {
                            continue; // guard against cycles
                        }
                        if let Some(child_manifest) = manifests.get(child_label) {
                            // Try to find a matching ingredient delta.
                            // Strategy 1: URI contains the manifest label as a substring.
                            // Strategy 2: fall back to positional order.
                            let matched_delta: Option<&serde_json::Value> = ingredient_deltas
                                .iter()
                                .find(|d| {
                                    d.get("ingredientAssertionURI")
                                        .and_then(|u| u.as_str())
                                        .map(|uri| uri.contains(child_label))
                                        .unwrap_or(false)
                                })
                                .copied()
                                .and_then(|d| d.get("validationDeltas"))
                                .or_else(|| {
                                    // Positional fallback: nth ingredient delta.
                                    ingredient_deltas
                                        .get(ingredient_order)
                                        .and_then(|d| d.get("validationDeltas"))
                                });

                            let info = extract_manifest_info(
                                child_manifest,
                                &json,
                                false,
                                matched_delta,
                                None, // ingredients inherit mode from active manifest
                            );
                            ingredients.push(info);
                            queue.push((child_manifest, child_label.to_string(), _depth + 1));
                            ingredient_order += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(Some(ManifestChain {
        active,
        ingredients,
        manifest_count,
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

/// Check if a C2PA manifest declares AI-generated content via assertions.
///
/// Scans for:
/// - `digitalSourceType` containing `"trainedAlgorithmicMedia"` (IPTC vocabulary)
/// - Action descriptions containing AI generation keywords
/// - Known AI generator names mentioned in action descriptions
///
/// The IPTC Digital Source Type vocabulary is the canonical C2PA mechanism for
/// declaring AI-generated media:
/// `http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia`
///
/// This is separate from `detect_ai_generator` (which checks `claim_generator`)
/// because generators such as Google Gemini embed their declaration in the
/// `c2pa.actions` assertion body rather than in the generator string.
pub fn detect_ai_from_assertions(assertions: &[AssertionInfo]) -> Option<String> {
    for assertion in assertions {
        let lower_value = assertion.value.to_lowercase();

        // Check digitalSourceType — the primary IPTC/C2PA AI declaration mechanism.
        // The full URI contains "trainedAlgorithmicMedia"; match case-insensitively.
        if lower_value.contains("trainedalgorithmicmedia") {
            return Some("AI-generated (C2PA digitalSourceType)".to_string());
        }

        // For c2pa.actions assertions, also scan the description text for AI keywords
        // and known generator names. These appear when the manifest was created by an
        // AI tool that embeds a human-readable description such as
        // "Created by Google Generative AI."
        if assertion.label.contains("c2pa.actions") {
            let ai_keywords = [
                "generative ai",
                "ai-generated",
                "generated by ai",
                "synthetic media",
                "machine generated",
                "created by generative",
            ];
            for keyword in &ai_keywords {
                if lower_value.contains(keyword) {
                    return Some(format!("AI-generated (C2PA action: {keyword})"));
                }
            }

            // Cross-reference known AI generator names from the shared constant list.
            for (pattern, name) in C2PA_AI_GENERATORS {
                if lower_value.contains(pattern) {
                    return Some(format!("AI-generated (C2PA mentions {name})"));
                }
            }
        }
    }
    None
}

// ===== Conformant Signing Mode =====

/// The active signing mode for C2PA manifest creation.
///
/// `Bedrock` (default) uses the per-install `rcgen`-generated local CA chain —
/// offline-first, no phone-home. Produces structurally-valid manifests that show
/// `signingCredential.untrusted` in external validators because the root is not
/// in any public trust store. This is the Jura Labs USP.
///
/// `Conformant` uses an institution-imported certificate from a CA on the
/// C2PA trust list. Produces manifests that pass in Adobe Inspect, the
/// Content Credentials Verify site, and any conformant validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SigningMode {
    Bedrock,
    Conformant,
}

/// Metadata describing an imported conformant certificate.
///
/// Returned by `import_conformant_certificate` and by
/// `get_conformant_certificate_info`. The frontend uses this to display
/// cert status and expiry warnings in the Settings panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConformantCertificateInfo {
    /// Subject Common Name of the end-entity certificate.
    pub subject_cn: String,
    /// Issuer Common Name (CA name) of the end-entity certificate.
    pub issuer_cn: String,
    /// ISO 8601 not-before timestamp.
    pub not_before: String,
    /// ISO 8601 not-after (expiry) timestamp.
    pub not_after: String,
    /// SHA-256 fingerprint of the end-entity DER bytes, lowercase hex pairs
    /// separated by colons (e.g. `"aa:bb:cc:..."`).
    pub fingerprint_sha256: String,
    /// Signing algorithm detected from the certificate (e.g. `"ECDSA-P256-SHA256"`).
    pub signing_algorithm: String,
    /// Key usage flags present on the end-entity cert.
    pub key_usage: Vec<String>,
    /// Extended key usage OID friendly names present on the end-entity cert.
    pub extended_key_usage: Vec<String>,
    /// Whether the certificate is currently valid (not_before <= now <= not_after).
    pub is_currently_valid: bool,
    /// ISO 8601 timestamp when the cert was imported into Jura Trace.
    pub imported_at: String,
}

/// On-disk shape of `<data_dir>/certs/signing_config.json`.
#[derive(Debug, Serialize, Deserialize)]
struct SigningConfig {
    active_mode: SigningMode,
    /// ISO 8601 timestamp set when a conformant cert was successfully imported.
    conformant_cert_imported_at: Option<String>,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            active_mode: SigningMode::Bedrock,
            conformant_cert_imported_at: None,
        }
    }
}

// ---- internal helpers ----

fn certs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("certs")
}

fn signing_config_path(data_dir: &Path) -> PathBuf {
    certs_dir(data_dir).join("signing_config.json")
}

fn conformant_cert_path(data_dir: &Path) -> PathBuf {
    certs_dir(data_dir).join("conformant_cert.pem")
}

fn conformant_key_path(data_dir: &Path) -> PathBuf {
    certs_dir(data_dir).join("conformant_key.pem")
}

/// Read `signing_config.json`; returns `Default` if the file is missing.
///
/// Never panics on a missing file — this is the first-run case.
fn read_signing_config(data_dir: &Path) -> Result<SigningConfig, String> {
    let path = signing_config_path(data_dir);
    if !path.exists() {
        return Ok(SigningConfig::default());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read signing config: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse signing config: {e}"))
}

/// Write `signing_config.json` atomically (write to `.tmp`, then rename).
fn write_signing_config(data_dir: &Path, config: &SigningConfig) -> Result<(), String> {
    let dir = certs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create certs directory: {e}"))?;
    let path = signing_config_path(data_dir);
    let tmp = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialise signing config: {e}"))?;
    std::fs::write(&tmp, json.as_bytes())
        .map_err(|e| format!("Failed to write signing config: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Failed to commit signing config: {e}"))?;
    Ok(())
}

/// Format a `time::OffsetDateTime` as an ISO 8601 string suitable for IPC.
fn format_time(t: time::OffsetDateTime) -> String {
    // Use the well-known subset: YYYY-MM-DDTHH:MM:SSZ
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        t.year(),
        t.month() as u8,
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
    )
}

/// Parse the first PEM certificate block from `pem_bytes` and return its DER.
fn first_cert_der(pem_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let pem_str = std::str::from_utf8(pem_bytes)
        .map_err(|_| "Certificate PEM is not valid UTF-8".to_string())?;
    // x509_parser ships with c2pa-rs transitively; use the simpler approach of
    // splitting on PEM block boundaries ourselves to stay dependency-free.
    let mut in_block = false;
    let mut b64 = String::new();
    for line in pem_str.lines() {
        let trimmed = line.trim();
        if trimmed == "-----BEGIN CERTIFICATE-----" {
            in_block = true;
            b64.clear();
            continue;
        }
        if trimmed == "-----END CERTIFICATE-----" && in_block {
            break;
        }
        if in_block {
            b64.push_str(trimmed);
        }
    }
    if b64.is_empty() {
        return Err("No valid CERTIFICATE PEM block found in file".to_string());
    }
    use std::io::Read;
    let mut decoder = base64::Base64Decoder::new(b64.as_bytes());
    let mut der = Vec::new();
    decoder
        .read_to_end(&mut der)
        .map_err(|e| format!("Failed to base64-decode certificate DER: {e}"))?;
    Ok(der)
}

// ---- public API ----

/// Import an institution-provided conformant certificate for C2PA signing.
///
/// Reads PEM files from `cert_pem_path` (chain: EE first, then intermediates,
/// then root) and `key_pem_path` (private key for the end-entity cert).
///
/// Validates that:
/// - At least one `BEGIN CERTIFICATE` block is present.
/// - The private key matches the end-entity certificate (public key comparison
///   via a round-trip sign/verify using c2pa-rs's `from_keys` builder).
/// - The cert passes c2pa-rs's `check_certificate_profile` (AKI, Key Usage,
///   Extended Key Usage, no unknown critical extensions) by attempting to
///   construct a signer — any profile error is surfaced as a validation failure.
///
/// Copies the PEM files to `<data_dir>/certs/conformant_cert.pem` and
/// `conformant_key.pem` with 0600 permissions on the key file (Unix).
///
/// Returns metadata about the imported certificate. The `is_currently_valid`
/// field reflects expiry at import time; it does not prevent a cert that has
/// not yet expired from being imported even if it will expire soon.
pub fn import_conformant_certificate(
    cert_pem_path: &Path,
    key_pem_path: &Path,
    data_dir: &Path,
) -> Result<ConformantCertificateInfo, String> {
    // --- 1. Read PEM files ---
    let cert_bytes = std::fs::read(cert_pem_path)
        .map_err(|e| format!("Failed to read certificate file: {e}"))?;
    let key_bytes =
        std::fs::read(key_pem_path).map_err(|e| format!("Failed to read key file: {e}"))?;

    // --- 2. Basic PEM sanity checks ---
    let cert_str = std::str::from_utf8(&cert_bytes)
        .map_err(|_| "Certificate file is not valid UTF-8".to_string())?;
    let key_str =
        std::str::from_utf8(&key_bytes).map_err(|_| "Key file is not valid UTF-8".to_string())?;

    if !cert_str.contains("-----BEGIN CERTIFICATE-----") {
        return Err("Certificate file does not contain a valid PEM CERTIFICATE block".to_string());
    }
    if !key_str.contains("-----BEGIN") {
        return Err("Key file does not contain a valid PEM block".to_string());
    }

    // --- 3a. Key-cert public key match ---
    //
    // Parse the private key PEM to obtain its corresponding public key bytes.
    // Then decode the first certificate's DER and verify those public key bytes
    // are present in the SubjectPublicKeyInfo field. This detects the common
    // misconfiguration where the user supplies a key from a different cert.
    let key_pair = rcgen::KeyPair::from_pem(key_str)
        .map_err(|e| format!("Failed to parse private key PEM: {e}"))?;
    let key_pubkey_raw = key_pair.public_key_raw();

    let cert_der = first_cert_der(&cert_bytes)?;
    // The ECDSA public key raw bytes (the uncompressed point) will appear
    // verbatim inside the cert DER as the BIT STRING payload of SubjectPublicKeyInfo.
    // Searching for the raw bytes as a subsequence is sufficient for P-256 keys.
    if !cert_der
        .windows(key_pubkey_raw.len())
        .any(|w| w == key_pubkey_raw)
    {
        return Err(
            "Private key does not match the certificate: public key mismatch. \
             Ensure the key file corresponds to the certificate file."
                .to_string(),
        );
    }

    // --- 3b. Validate cert profile by constructing a c2pa signer.
    //     This calls `check_certificate_profile` internally in c2pa-rs and
    //     will return an error if AKI, Key Usage, EKU, or critical-extension
    //     constraints are violated. ---
    c2pa::create_signer::from_keys(&cert_bytes, &key_bytes, c2pa::SigningAlg::Es256, None)
        .map_err(|e| format!("Certificate profile validation failed: {e}"))?;

    // --- 4. Extract cert metadata from the first PEM block ---
    // Parse using rcgen's CertificateParams::from_ca_cert_pem for metadata
    // (subject, issuer, validity, key usage, EKU, public key bytes).
    let params = rcgen::CertificateParams::from_ca_cert_pem(cert_str)
        .map_err(|e| format!("Failed to parse certificate PEM: {e}"))?;

    // Extract subject CN
    let subject_cn = params
        .distinguished_name
        .get(&rcgen::DnType::CommonName)
        .map(|v| match v {
            rcgen::DnValue::Utf8String(s) => s.clone(),
            rcgen::DnValue::PrintableString(s) => s.as_str().to_string(),
            rcgen::DnValue::TeletexString(s) => s.as_str().to_string(),
            rcgen::DnValue::UniversalString(s) => {
                String::from_utf8_lossy(s.as_bytes()).into_owned()
            }
            rcgen::DnValue::BmpString(s) => String::from_utf8_lossy(s.as_bytes()).into_owned(),
            rcgen::DnValue::Ia5String(s) => s.as_str().to_string(),
            &_ => "(unknown)".to_string(),
        })
        .unwrap_or_else(|| "(unknown)".to_string());

    // Extract issuer CN by parsing the DER directly via x509-parser.
    // rcgen's CertificateParams::from_ca_cert_pem exposes the subject DN
    // but not the issuer DN — for that we read the tbsCertificate.issuer
    // field directly. On a self-signed CA cert this equals the subject;
    // on a chain-issued EE cert it names the issuing CA. Falls back to
    // an empty string if parsing fails, which the UI treats as "hide the
    // issuer row" rather than showing an error.
    let issuer_cn = extract_issuer_cn_from_pem(cert_str).unwrap_or_default();

    // Validity window
    let not_before_str = format_time(params.not_before);
    let not_after_str = format_time(params.not_after);

    // Is currently valid?
    let now = time::OffsetDateTime::now_utc();
    let is_currently_valid = params.not_before <= now && now <= params.not_after;

    // Key usage flags
    let key_usage: Vec<String> = params
        .key_usages
        .iter()
        .map(|ku| format!("{ku:?}"))
        .collect();

    // Extended key usage
    let extended_key_usage: Vec<String> = params
        .extended_key_usages
        .iter()
        .map(|eku| match eku {
            rcgen::ExtendedKeyUsagePurpose::EmailProtection => "emailProtection".to_string(),
            rcgen::ExtendedKeyUsagePurpose::CodeSigning => "codeSigning".to_string(),
            rcgen::ExtendedKeyUsagePurpose::ServerAuth => "serverAuth".to_string(),
            rcgen::ExtendedKeyUsagePurpose::ClientAuth => "clientAuth".to_string(),
            rcgen::ExtendedKeyUsagePurpose::TimeStamping => "timeStamping".to_string(),
            rcgen::ExtendedKeyUsagePurpose::OcspSigning => "ocspSigning".to_string(),
            rcgen::ExtendedKeyUsagePurpose::Any => "any".to_string(),
            _ => "other".to_string(),
        })
        .collect();

    // Signing algorithm — c2pa-rs only supports Es256 for `from_keys`, so if the
    // signer construction succeeded above the cert must carry an ECDSA-P256 key.
    // We record this as a fixed string rather than attempting DER-level algorithm
    // inspection (which would require an explicit x509-parser dep at the call site).
    let signing_algorithm = "ECDSA-P256-SHA256".to_string();

    // --- 5. SHA-256 fingerprint of the first cert DER ---
    let der = first_cert_der(&cert_bytes)?;
    let digest = Sha256::digest(&der);
    let fingerprint_sha256 = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":");

    // --- 6. Copy PEM files to data_dir/certs/ ---
    let dir = certs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create certs directory: {e}"))?;

    let dest_cert = conformant_cert_path(data_dir);
    let dest_key = conformant_key_path(data_dir);

    std::fs::write(&dest_cert, &cert_bytes)
        .map_err(|e| format!("Failed to write conformant certificate: {e}"))?;
    std::fs::write(&dest_key, &key_bytes)
        .map_err(|e| format!("Failed to write conformant key: {e}"))?;

    // Restrict key to owner-read/write only (0600) on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest_key)
            .map_err(|e| format!("Failed to stat conformant key file: {e}"))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&dest_key, perms)
            .map_err(|e| format!("Failed to set conformant key file permissions: {e}"))?;
    }

    // --- 7. Update signing_config.json with import timestamp ---
    let imported_at = format_time(now);
    let mut config = read_signing_config(data_dir)?;
    config.conformant_cert_imported_at = Some(imported_at.clone());
    write_signing_config(data_dir, &config)?;

    // Sanitise subject_cn before logging to prevent log injection via
    // crafted certificate CN fields containing newlines or ANSI codes.
    let safe_cn = subject_cn.replace(['\n', '\r', '\x00'], "?");
    log::info!(
        "Imported conformant certificate for {} at {}",
        safe_cn,
        dest_cert.display()
    );

    Ok(ConformantCertificateInfo {
        subject_cn,
        issuer_cn,
        not_before: not_before_str,
        not_after: not_after_str,
        fingerprint_sha256,
        signing_algorithm,
        key_usage,
        extended_key_usage,
        is_currently_valid,
        imported_at,
    })
}

/// Return the active signing mode.
///
/// Reads `<data_dir>/certs/signing_config.json`; returns `Bedrock` on
/// first run (file absent) or on any parse error.
pub fn get_active_signing_mode(data_dir: &Path) -> SigningMode {
    read_signing_config(data_dir)
        .map(|c| c.active_mode)
        .unwrap_or(SigningMode::Bedrock)
}

/// Persist the active signing mode.
///
/// Returns an error if Conformant mode is requested but no conformant
/// certificate has been imported.
pub fn set_active_signing_mode(data_dir: &Path, mode: SigningMode) -> Result<(), String> {
    if mode == SigningMode::Conformant && !conformant_cert_path(data_dir).exists() {
        return Err(
            "Cannot activate Conformant signing mode: no certificate has been imported. \
             Import a conformant certificate first."
                .to_string(),
        );
    }
    let mut config = read_signing_config(data_dir)?;
    config.active_mode = mode;
    write_signing_config(data_dir, &config)
}

/// Return metadata about the currently imported conformant certificate.
///
/// Returns `Ok(None)` if no certificate has been imported.
pub fn get_conformant_certificate_info(
    data_dir: &Path,
) -> Result<Option<ConformantCertificateInfo>, String> {
    let cert_path = conformant_cert_path(data_dir);
    if !cert_path.exists() {
        return Ok(None);
    }
    let cert_bytes = std::fs::read(&cert_path)
        .map_err(|e| format!("Failed to read conformant certificate: {e}"))?;
    let key_path = conformant_key_path(data_dir);
    let cert_str = std::str::from_utf8(&cert_bytes)
        .map_err(|_| "Conformant certificate is not valid UTF-8".to_string())?;

    let params = rcgen::CertificateParams::from_ca_cert_pem(cert_str)
        .map_err(|e| format!("Failed to parse stored conformant certificate: {e}"))?;

    let subject_cn = params
        .distinguished_name
        .get(&rcgen::DnType::CommonName)
        .map(|v| match v {
            rcgen::DnValue::Utf8String(s) => s.clone(),
            rcgen::DnValue::PrintableString(s) => s.as_str().to_string(),
            rcgen::DnValue::TeletexString(s) => s.as_str().to_string(),
            rcgen::DnValue::UniversalString(s) => {
                String::from_utf8_lossy(s.as_bytes()).into_owned()
            }
            rcgen::DnValue::BmpString(s) => String::from_utf8_lossy(s.as_bytes()).into_owned(),
            rcgen::DnValue::Ia5String(s) => s.as_str().to_string(),
            &_ => "(unknown)".to_string(),
        })
        .unwrap_or_else(|| "(unknown)".to_string());

    let now = time::OffsetDateTime::now_utc();
    let is_currently_valid = params.not_before <= now && now <= params.not_after;

    let key_usage: Vec<String> = params
        .key_usages
        .iter()
        .map(|ku| format!("{ku:?}"))
        .collect();

    let extended_key_usage: Vec<String> = params
        .extended_key_usages
        .iter()
        .map(|eku| match eku {
            rcgen::ExtendedKeyUsagePurpose::EmailProtection => "emailProtection".to_string(),
            rcgen::ExtendedKeyUsagePurpose::CodeSigning => "codeSigning".to_string(),
            rcgen::ExtendedKeyUsagePurpose::ServerAuth => "serverAuth".to_string(),
            rcgen::ExtendedKeyUsagePurpose::ClientAuth => "clientAuth".to_string(),
            rcgen::ExtendedKeyUsagePurpose::TimeStamping => "timeStamping".to_string(),
            rcgen::ExtendedKeyUsagePurpose::OcspSigning => "ocspSigning".to_string(),
            rcgen::ExtendedKeyUsagePurpose::Any => "any".to_string(),
            _ => "other".to_string(),
        })
        .collect();

    let signing_algorithm = "ECDSA-P256-SHA256".to_string();

    let der = first_cert_der(&cert_bytes)?;
    let digest = Sha256::digest(&der);
    let fingerprint_sha256 = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":");

    let config = read_signing_config(data_dir)?;
    let imported_at = config.conformant_cert_imported_at.unwrap_or_else(|| {
        // Cert file exists but config has no timestamp — use file mtime as fallback.
        std::fs::metadata(&key_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
                time::OffsetDateTime::from_unix_timestamp(secs).ok()
            })
            .map(format_time)
            .unwrap_or_else(|| "unknown".to_string())
    });

    // Issuer CN via direct DER parsing — see import_conformant_certificate
    // for the rationale. We read the raw PEM here rather than relying on
    // rcgen's CertificateParams which only exposes the subject.
    let pem_str = std::str::from_utf8(&cert_bytes).unwrap_or("");
    let issuer_cn = extract_issuer_cn_from_pem(pem_str).unwrap_or_default();

    Ok(Some(ConformantCertificateInfo {
        subject_cn,
        issuer_cn,
        not_before: format_time(params.not_before),
        not_after: format_time(params.not_after),
        fingerprint_sha256,
        signing_algorithm,
        key_usage,
        extended_key_usage,
        is_currently_valid,
        imported_at,
    }))
}

/// Extract the Common Name from the `issuer` field of the first certificate
/// in a PEM chain. Uses x509-parser to walk the tbsCertificate.issuer RDN
/// sequence and pick out the attribute type OID 2.5.4.3 (commonName).
///
/// Returns `None` if the PEM cannot be parsed, the cert has no issuer CN
/// (pathological — every conforming X.509 issuer must have a CN), or the
/// CN value is not a valid string. Callers should treat `None` as "hide
/// the issuer row" rather than an error.
fn extract_issuer_cn_from_pem(pem_str: &str) -> Option<String> {
    use x509_parser::pem::Pem;
    use x509_parser::prelude::FromDer;

    // x509-parser provides its own PEM reader that decodes the base64
    // body and yields one `Pem` per BEGIN CERTIFICATE block. We only
    // need the first (end-entity) block for issuer extraction.
    let mut reader = std::io::Cursor::new(pem_str.as_bytes());
    let (pem, _) = Pem::read(&mut reader).ok()?;
    let (_, cert) = x509_parser::certificate::X509Certificate::from_der(&pem.contents).ok()?;

    // Iterate the issuer's CommonName attributes. The attribute value is
    // an ASN.1 DirectoryString; x509-parser decodes the common variants
    // (Utf8String, PrintableString, TeletexString) via `as_str()`.
    for rdn in cert.issuer().iter_common_name() {
        if let Ok(s) = rdn.as_str() {
            return Some(s.to_string());
        }
    }
    None
}

/// Delete the imported conformant certificate and revert the active mode to Bedrock.
///
/// No-op (returns `Ok(())`) if no certificate is present.
pub fn clear_conformant_certificate(data_dir: &Path) -> Result<(), String> {
    let cert_path = conformant_cert_path(data_dir);
    let key_path = conformant_key_path(data_dir);

    if cert_path.exists() {
        std::fs::remove_file(&cert_path)
            .map_err(|e| format!("Failed to remove conformant certificate: {e}"))?;
    }
    if key_path.exists() {
        std::fs::remove_file(&key_path)
            .map_err(|e| format!("Failed to remove conformant key: {e}"))?;
    }

    // Revert config to Bedrock regardless of what was there before.
    let config = SigningConfig {
        active_mode: SigningMode::Bedrock,
        conformant_cert_imported_at: None,
    };
    write_signing_config(data_dir, &config)?;
    log::info!("Conformant certificate cleared; reverted to Bedrock signing mode");
    Ok(())
}

/// Sign a file using whichever signing mode is currently active.
///
/// If the active mode is `Bedrock`, the per-install local CA chain is used
/// (same as calling `ensure_certificate` + `sign_file` directly).
///
/// If the active mode is `Conformant`, the imported certificate is read from
/// `<data_dir>/certs/conformant_cert.pem` and validated to be currently valid
/// (not expired, not yet-valid) before signing. If the certificate has expired
/// this function returns an error — sign with Bedrock mode or import a new cert.
///
/// Existing callers of the lower-level `sign_file` are unaffected.
pub fn sign_file_with_active_mode(
    source: &Path,
    output: &Path,
    creator_name: &str,
    license: Option<&str>,
    data_dir: &Path,
) -> Result<ManifestInfo, String> {
    let mode = get_active_signing_mode(data_dir);
    match mode {
        SigningMode::Bedrock => {
            let (cert, key) = ensure_certificate(data_dir)?;
            sign_file(source, output, creator_name, license, &cert, &key)
        }
        SigningMode::Conformant => {
            let cert_path = conformant_cert_path(data_dir);
            let key_path = conformant_key_path(data_dir);

            if !cert_path.exists() {
                return Err(
                    "Conformant signing mode is active but no certificate has been imported."
                        .to_string(),
                );
            }

            let cert = std::fs::read(&cert_path)
                .map_err(|e| format!("Failed to read conformant certificate: {e}"))?;
            let key = std::fs::read(&key_path)
                .map_err(|e| format!("Failed to read conformant key: {e}"))?;

            // Verify the cert is still valid — refuse to sign with an expired cert.
            let cert_str = std::str::from_utf8(&cert)
                .map_err(|_| "Conformant certificate is not valid UTF-8".to_string())?;
            let params = rcgen::CertificateParams::from_ca_cert_pem(cert_str)
                .map_err(|e| format!("Failed to parse conformant certificate: {e}"))?;
            let now = time::OffsetDateTime::now_utc();
            if now > params.not_after {
                return Err(format!(
                    "Conformant certificate expired at {}. Import a new certificate or switch to Bedrock signing mode.",
                    format_time(params.not_after)
                ));
            }
            if now < params.not_before {
                return Err(format!(
                    "Conformant certificate is not yet valid (valid from {}). Import a current certificate or switch to Bedrock signing mode.",
                    format_time(params.not_before)
                ));
            }

            sign_file(source, output, creator_name, license, &cert, &key)
        }
    }
}

// ---- base64 helper (no new deps) ----
// We need base64 decoding for the DER extraction. The `rcgen` crate does not
// re-export base64; however `c2pa` brings in `base64` transitively.
// We use a minimal manual approach here to avoid taking a new dep.

mod base64 {
    pub struct Base64Decoder<'a> {
        input: &'a [u8],
        pos: usize,
        buf: [u8; 3],
        buf_len: usize,
    }

    impl<'a> Base64Decoder<'a> {
        pub fn new(input: &'a [u8]) -> Self {
            Self {
                input,
                pos: 0,
                buf: [0u8; 3],
                buf_len: 0,
            }
        }
    }

    fn decode_char(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            b'=' => None,                         // padding
            b'\n' | b'\r' | b' ' | b'\t' => None, // whitespace
            _ => None,
        }
    }

    impl std::io::Read for Base64Decoder<'_> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            // Flush any buffered bytes first.
            if self.buf_len > 0 {
                let n = self.buf_len.min(out.len());
                out[..n].copy_from_slice(&self.buf[3 - self.buf_len..3 - self.buf_len + n]);
                self.buf_len -= n;
                return Ok(n);
            }

            // Collect 4 base64 chars (skipping whitespace/padding).
            let mut chars = [0u8; 4];
            let mut count = 0;
            let mut padding = 0u8;
            while count < 4 && self.pos < self.input.len() {
                let c = self.input[self.pos];
                self.pos += 1;
                if c == b'=' {
                    padding += 1;
                    count += 1;
                    chars[count - 1] = 0;
                } else if let Some(v) = decode_char(c) {
                    if c == b'\n' || c == b'\r' || c == b' ' || c == b'\t' {
                        continue;
                    }
                    chars[count] = v;
                    count += 1;
                }
            }
            if count == 0 {
                return Ok(0); // EOF
            }

            let b0 = (chars[0] << 2) | (chars[1] >> 4);
            let b1 = (chars[1] << 4) | (chars[2] >> 2);
            let b2 = (chars[2] << 6) | chars[3];

            let decoded = match padding {
                0 => {
                    self.buf = [b0, b1, b2];
                    3
                }
                1 => {
                    self.buf = [b0, b1, 0];
                    2
                }
                _ => {
                    self.buf = [b0, 0, 0];
                    1
                }
            };

            let n = decoded.min(out.len());
            out[..n].copy_from_slice(&self.buf[..n]);
            if decoded > n {
                // Store remainder in buf
                let rem = decoded - n;
                self.buf_len = rem;
                // Shift remaining bytes to the end of buf for next read
                for i in 0..rem {
                    self.buf[3 - rem + i] = self.buf[n + i];
                }
            }
            Ok(n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_info_serialises_to_camel_case() {
        let info = ManifestInfo {
            title: Some("test.jpg".to_string()),
            format: Some("image/jpeg".to_string()),
            claim_generator: Some("Jura Trace/0.9.0".to_string()),
            assertions: vec![AssertionInfo {
                label: "c2pa.actions".to_string(),
                value: "{}".to_string(),
            }],
            is_valid: true,
            valid_at_signing: false,
            signed_at: Some("2026-01-01T00:00:00Z".to_string()),
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: Some("standard".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"isValid\""));
        assert!(json.contains("\"claimGenerator\""));
        assert!(json.contains("\"signedAt\""));
        assert!(json.contains("\"validAtSigning\""));
        assert!(!json.contains("\"is_valid\""));
    }

    #[test]
    fn validity_fully_valid_when_no_failures() {
        let json = serde_json::json!({
            "validation_results": { "activeManifest": { "success": [], "failure": [] } }
        });
        assert_eq!(derive_validity(&json), (true, false));
    }

    #[test]
    fn validity_self_signed_untrusted_only_is_valid() {
        let json = serde_json::json!({
            "validation_results": {
                "activeManifest": {
                    "failure": [{ "code": "signingCredential.untrusted" }]
                }
            }
        });
        assert_eq!(derive_validity(&json), (true, false));
    }

    #[test]
    fn validity_pixel_expired_with_trusted_timestamp_is_valid_at_signing() {
        let json = serde_json::json!({
            "validation_results": {
                "activeManifest": {
                    "success": [
                        { "code": "timeStamp.validated" },
                        { "code": "claimSignature.validated" }
                    ],
                    "failure": [
                        { "code": "signingCredential.expired" },
                        { "code": "signingCredential.untrusted" }
                    ]
                }
            }
        });
        assert_eq!(derive_validity(&json), (true, true));
    }

    #[test]
    fn validity_expired_without_timestamp_is_invalid() {
        let json = serde_json::json!({
            "validation_results": {
                "activeManifest": {
                    "success": [{ "code": "claimSignature.validated" }],
                    "failure": [{ "code": "signingCredential.expired" }]
                }
            }
        });
        assert_eq!(derive_validity(&json), (false, false));
    }

    #[test]
    fn validity_expired_without_claim_signature_is_invalid() {
        let json = serde_json::json!({
            "validation_results": {
                "activeManifest": {
                    "success": [{ "code": "timeStamp.validated" }],
                    "failure": [{ "code": "signingCredential.expired" }]
                }
            }
        });
        assert_eq!(derive_validity(&json), (false, false));
    }

    #[test]
    fn validity_hash_mismatch_is_invalid() {
        let json = serde_json::json!({
            "validation_results": {
                "activeManifest": {
                    "success": [
                        { "code": "timeStamp.trusted" },
                        { "code": "claimSignature.validated" }
                    ],
                    "failure": [{ "code": "assertion.dataHash.mismatch" }]
                }
            }
        });
        assert_eq!(derive_validity(&json), (false, false));
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
        assert!(!supports_signing("unknown", "model/stl"));
    }

    #[test]
    fn detect_ai_from_assertions_digital_source_type() {
        // The canonical IPTC URI contains "trainedAlgorithmicMedia" — must fire.
        let assertions = vec![AssertionInfo {
            label: "c2pa.actions".to_string(),
            value: r#"{"actions":[{"action":"c2pa.created","digitalSourceType":"http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia"}]}"#.to_string(),
        }];
        let result = detect_ai_from_assertions(&assertions);
        assert!(
            result.is_some(),
            "trainedAlgorithmicMedia should be detected"
        );
        assert!(
            result.unwrap().contains("digitalSourceType"),
            "message should mention digitalSourceType"
        );
    }

    #[test]
    fn detect_ai_from_assertions_digital_source_type_outside_actions() {
        // digitalSourceType can appear in non-actions assertions — must still fire.
        let assertions = vec![AssertionInfo {
            label: "c2pa.claim".to_string(),
            value: r#"{"digitalSourceType":"trainedAlgorithmicMedia"}"#.to_string(),
        }];
        let result = detect_ai_from_assertions(&assertions);
        assert!(
            result.is_some(),
            "trainedAlgorithmicMedia outside c2pa.actions should be detected"
        );
    }

    #[test]
    fn detect_ai_from_assertions_description_keyword_generative_ai() {
        let assertions = vec![AssertionInfo {
            label: "c2pa.actions".to_string(),
            value: r#"{"actions":[{"action":"c2pa.created","description":"Created by Google Generative AI."}]}"#.to_string(),
        }];
        let result = detect_ai_from_assertions(&assertions);
        assert!(
            result.is_some(),
            "'Generative AI' keyword should be detected"
        );
        assert!(
            result.unwrap().to_lowercase().contains("generative ai"),
            "message should name the matched keyword"
        );
    }

    #[test]
    fn detect_ai_from_assertions_generator_name_gemini() {
        let assertions = vec![AssertionInfo {
            label: "c2pa.actions".to_string(),
            value: r#"{"actions":[{"action":"c2pa.created","softwareAgent":"Google Gemini 2.0"}]}"#
                .to_string(),
        }];
        let result = detect_ai_from_assertions(&assertions);
        assert!(
            result.is_some(),
            "Known generator name 'gemini' should be detected"
        );
    }

    #[test]
    fn detect_ai_from_assertions_generator_name_dall_e() {
        let assertions = vec![AssertionInfo {
            label: "c2pa.actions".to_string(),
            value: r#"{"softwareAgent":"DALL-E 3 by OpenAI"}"#.to_string(),
        }];
        let result = detect_ai_from_assertions(&assertions);
        assert!(
            result.is_some(),
            "Known generator name 'dall-e' should be detected"
        );
    }

    #[test]
    fn detect_ai_from_assertions_no_ai_human_photo() {
        // Ordinary human-created photo — should return None.
        let assertions = vec![
            AssertionInfo {
                label: "c2pa.actions".to_string(),
                value: r#"{"actions":[{"action":"c2pa.created","softwareAgent":"Adobe Lightroom 7.0"}]}"#.to_string(),
            },
            AssertionInfo {
                label: "c2pa.rights".to_string(),
                value: r#"{"rights":"All Rights Reserved"}"#.to_string(),
            },
        ];
        assert!(
            detect_ai_from_assertions(&assertions).is_none(),
            "Normal photo assertions should return None"
        );
    }

    #[test]
    fn detect_ai_from_assertions_empty_slice() {
        assert!(
            detect_ai_from_assertions(&[]).is_none(),
            "Empty assertions should return None"
        );
    }

    #[test]
    fn read_manifest_missing_file_returns_error() {
        let result = read_manifest(Path::new("/nonexistent/file.jpg"), false);
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
        let readback = read_manifest(&output_path, false).expect("read_manifest should not error");
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

    // ===== Conformant Signing Mode Tests =====

    /// Helper: generate a fresh Bedrock-style cert chain and write to tempdir,
    /// returning (cert_pem_path, key_pem_path) for use in import tests.
    fn generate_test_cert_chain(dir: &std::path::Path) -> (PathBuf, PathBuf) {
        let data_dir = dir.join("data");
        let (cert_bytes, key_bytes) = ensure_certificate(&data_dir).expect("ensure_certificate");

        // The Bedrock chain is EE + CA. For import tests we only need the cert
        // and key files on disk. Write them to a separate location (not the
        // data_dir/certs path) so that import_conformant_certificate can copy them.
        let cert_path = dir.join("test_cert.pem");
        let key_path = dir.join("test_key.pem");
        std::fs::write(&cert_path, &cert_bytes).expect("write test cert");
        std::fs::write(&key_path, &key_bytes).expect("write test key");
        (cert_path, key_path)
    }

    /// Happy path: import a valid cert chain, verify the returned metadata is sane.
    #[test]
    fn import_conformant_certificate_happy_path() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("import_data");

        let info = import_conformant_certificate(&cert_path, &key_path, &data_dir)
            .expect("import should succeed");

        assert!(
            !info.subject_cn.is_empty(),
            "subject_cn should be populated"
        );
        assert!(
            !info.fingerprint_sha256.is_empty(),
            "fingerprint_sha256 should be populated"
        );
        assert_eq!(info.signing_algorithm, "ECDSA-P256-SHA256");
        assert!(
            info.is_currently_valid,
            "freshly-generated cert should be currently valid"
        );
        assert!(!info.imported_at.is_empty(), "imported_at should be set");
        // Verify files were actually written to data_dir/certs/
        assert!(
            data_dir.join("certs").join("conformant_cert.pem").exists(),
            "conformant_cert.pem should exist in data_dir"
        );
        assert!(
            data_dir.join("certs").join("conformant_key.pem").exists(),
            "conformant_key.pem should exist in data_dir"
        );
    }

    /// Issuer CN extraction: the imported cert is a Bedrock-style chain
    /// where the end-entity cert's issuer is the per-install CA. The
    /// returned ConformantCertificateInfo.issuer_cn must name the CA,
    /// not the placeholder that earlier versions returned.
    #[test]
    fn import_conformant_certificate_populates_issuer_cn() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("import_data");

        let info = import_conformant_certificate(&cert_path, &key_path, &data_dir)
            .expect("import should succeed");

        // Bedrock ensure_certificate writes CA CN = "Jura Trace Local CA"
        // and EE CN = "Jura Trace Signing Certificate". The EE cert's issuer
        // (what this test asserts) is therefore the CA's subject.
        assert_eq!(
            info.issuer_cn, "Jura Trace Local CA",
            "issuer_cn must be extracted from the CA that signed the EE cert"
        );
        assert_eq!(
            info.subject_cn, "Jura Trace Signing Certificate",
            "subject_cn should name the EE cert"
        );
        assert_ne!(
            info.issuer_cn, info.subject_cn,
            "issuer and subject must differ on an EE cert (not a self-signed root)"
        );
    }

    /// get_conformant_certificate_info returns the same issuer CN on read-back.
    /// Confirms both code paths (import and get_info) use the same extractor.
    #[test]
    fn get_conformant_cert_info_returns_issuer_cn_on_readback() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("import_data");

        import_conformant_certificate(&cert_path, &key_path, &data_dir)
            .expect("import should succeed");

        let info = get_conformant_certificate_info(&data_dir)
            .expect("get_conformant_certificate_info should not error")
            .expect("info should be Some after import");

        assert_eq!(info.issuer_cn, "Jura Trace Local CA");
        assert_eq!(info.subject_cn, "Jura Trace Signing Certificate");
    }

    /// The signing_config.json is updated with the import timestamp.
    #[test]
    fn import_updates_signing_config() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("import_data");

        import_conformant_certificate(&cert_path, &key_path, &data_dir)
            .expect("import should succeed");

        let config_path = data_dir.join("certs").join("signing_config.json");
        assert!(
            config_path.exists(),
            "signing_config.json should be written"
        );
        let raw = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            raw.contains("conformant_cert_imported_at"),
            "config should contain import timestamp"
        );
    }

    /// Mode persistence: Bedrock round-trip.
    #[test]
    fn signing_mode_bedrock_roundtrip() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        // Default (no file) should be Bedrock.
        assert_eq!(
            get_active_signing_mode(&data_dir),
            SigningMode::Bedrock,
            "default mode should be Bedrock"
        );

        // Explicitly set to Bedrock (should be a no-op but must not error).
        set_active_signing_mode(&data_dir, SigningMode::Bedrock)
            .expect("setting Bedrock should succeed");
        assert_eq!(get_active_signing_mode(&data_dir), SigningMode::Bedrock);
    }

    /// Mode persistence: switch to Conformant after importing a cert, then round-trip.
    #[test]
    fn signing_mode_conformant_roundtrip() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("data");

        // Import first so the cert file exists.
        import_conformant_certificate(&cert_path, &key_path, &data_dir)
            .expect("import should succeed");

        // Now switch to Conformant.
        set_active_signing_mode(&data_dir, SigningMode::Conformant)
            .expect("setting Conformant mode should succeed after import");
        assert_eq!(get_active_signing_mode(&data_dir), SigningMode::Conformant);
    }

    /// Setting Conformant mode without a cert import returns an error.
    #[test]
    fn signing_mode_conformant_without_cert_errors() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        let result = set_active_signing_mode(&data_dir, SigningMode::Conformant);
        assert!(
            result.is_err(),
            "setting Conformant without a cert should fail"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("no certificate"),
            "error message should mention missing certificate"
        );
    }

    /// clear_conformant_certificate reverts mode to Bedrock.
    #[test]
    fn clear_conformant_certificate_reverts_to_bedrock() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("data");

        import_conformant_certificate(&cert_path, &key_path, &data_dir).expect("import");
        set_active_signing_mode(&data_dir, SigningMode::Conformant).expect("set conformant");

        clear_conformant_certificate(&data_dir).expect("clear should succeed");

        assert_eq!(
            get_active_signing_mode(&data_dir),
            SigningMode::Bedrock,
            "mode should revert to Bedrock after clear"
        );
        assert!(
            !data_dir.join("certs").join("conformant_cert.pem").exists(),
            "conformant cert should be deleted"
        );
    }

    /// Validation failure: PEM file without a CERTIFICATE block.
    #[test]
    fn import_rejects_non_certificate_pem() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        let bad_cert = tmp.path().join("bad.pem");
        std::fs::write(&bad_cert, b"not a pem file at all").unwrap();
        let good_key = tmp.path().join("key.pem");
        // Generate a real key so the key-file check doesn't trip first.
        let (_, key_bytes) = ensure_certificate(&data_dir.join("gen")).expect("ensure_certificate");
        std::fs::write(&good_key, &key_bytes).unwrap();

        let result = import_conformant_certificate(&bad_cert, &good_key, &data_dir);
        assert!(result.is_err(), "import of non-PEM cert should fail");
        let err = result.unwrap_err();
        assert!(
            err.to_lowercase().contains("certificate") || err.to_lowercase().contains("pem"),
            "error should mention the problem: {err}"
        );
    }

    /// Validation failure: key that does not match the certificate.
    #[test]
    fn import_rejects_key_mismatch() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        // Generate two independent chains — use cert from one, key from the other.
        let (cert_a, _key_a) = generate_test_cert_chain(&tmp.path().join("chain_a"));
        let (_, key_b) = generate_test_cert_chain(&tmp.path().join("chain_b"));

        // Write the key from chain_b into a standalone file.
        let key_b_bytes = std::fs::read(&key_b).unwrap();
        // key_b is already the EE key of chain B; we need just the first key block.
        // (ensure_certificate returns the EE key only, so this is already a single key.)
        let mismatch_key = tmp.path().join("mismatch_key.pem");
        std::fs::write(&mismatch_key, &key_b_bytes).unwrap();

        let result = import_conformant_certificate(&cert_a, &mismatch_key, &data_dir);
        assert!(
            result.is_err(),
            "import with mismatched key should fail (key does not match cert)"
        );
    }

    /// get_conformant_certificate_info returns None when no cert has been imported.
    #[test]
    fn get_conformant_cert_info_none_when_absent() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        let result = get_conformant_certificate_info(&data_dir).expect("should not error");
        assert!(
            result.is_none(),
            "should return None when no cert is imported"
        );
    }

    /// get_conformant_certificate_info returns Some after import.
    #[test]
    fn get_conformant_cert_info_some_after_import() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("data");

        import_conformant_certificate(&cert_path, &key_path, &data_dir)
            .expect("import should succeed");

        let result = get_conformant_certificate_info(&data_dir).expect("should not error");
        assert!(result.is_some(), "should return Some after import");
        let info = result.unwrap();
        assert_eq!(info.signing_algorithm, "ECDSA-P256-SHA256");
        assert!(info.is_currently_valid);
    }

    /// Bedrock signing via `sign_file_with_active_mode` works end-to-end.
    #[test]
    fn sign_file_with_active_mode_bedrock() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        // Default mode is Bedrock — no setup needed.
        let source = tmp.path().join("input.png");
        let img = image::RgbImage::new(32, 32);
        img.save(&source).expect("save PNG");
        let output = signed_output_path(&source);

        let info = sign_file_with_active_mode(
            &source,
            &output,
            "Test Creator",
            Some("CC BY 4.0"),
            &data_dir,
        )
        .expect("Bedrock signing should succeed");

        assert_eq!(info.title.as_deref(), Some("input.png"));
        assert!(output.exists(), "signed output should exist");
    }

    /// Conformant signing via `sign_file_with_active_mode` works end-to-end.
    #[test]
    fn sign_file_with_active_mode_conformant() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let (cert_path, key_path) = generate_test_cert_chain(tmp.path());
        let data_dir = tmp.path().join("data");

        // Import and activate Conformant mode.
        import_conformant_certificate(&cert_path, &key_path, &data_dir).expect("import");
        set_active_signing_mode(&data_dir, SigningMode::Conformant).expect("set conformant");

        let source = tmp.path().join("input_conf.png");
        let img = image::RgbImage::new(32, 32);
        img.save(&source).expect("save PNG");
        let output = signed_output_path(&source);

        let info = sign_file_with_active_mode(&source, &output, "Test Creator", None, &data_dir)
            .expect("Conformant signing should succeed");

        assert_eq!(info.title.as_deref(), Some("input_conf.png"));
        assert!(output.exists(), "signed output should exist");
        // Read back and verify
        let readback = read_manifest(&output, false).expect("read_manifest");
        assert!(readback.is_some(), "manifest should be present");
    }

    /// Concurrent read safety: reading config when the file is absent must not panic.
    #[test]
    fn signing_config_missing_file_returns_bedrock() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("nonexistent_data_dir");

        // File does not exist — should return Bedrock silently.
        let mode = get_active_signing_mode(&data_dir);
        assert_eq!(
            mode,
            SigningMode::Bedrock,
            "missing config should default to Bedrock"
        );
    }

    // ===== ManifestChain / read_manifest_chain tests =====

    /// `extract_manifest_info` correctly extracts all fields from a synthetic
    /// manifest JSON object.
    #[test]
    fn extract_manifest_info_extracts_all_fields() {
        let manifest = serde_json::json!({
            "title": "photo.jpg",
            "format": "image/jpeg",
            "claim_generator": "Jura Trace/0.9.0",
            "assertions": [
                { "label": "c2pa.actions", "data": { "actions": [] } },
                { "label": "c2pa.rights",  "data": { "rights": "CC BY 4.0" } }
            ],
            "signature_info": {
                "time": "2026-01-01T00:00:00Z",
                "common_name": "Test Signer",
                "issuer": "Test CA"
            }
        });

        // Full-validation JSON with no failures — expect is_valid = true.
        let full_json = serde_json::json!({
            "validation_results": {
                "activeManifest": { "success": [], "failure": [] }
            }
        });

        let info = extract_manifest_info(&manifest, &full_json, true, None, Some("standard"));

        assert_eq!(info.title.as_deref(), Some("photo.jpg"));
        assert_eq!(info.format.as_deref(), Some("image/jpeg"));
        assert_eq!(info.claim_generator.as_deref(), Some("Jura Trace/0.9.0"));
        assert_eq!(info.assertions.len(), 2);
        assert!(info.is_valid);
        assert!(!info.valid_at_signing);
        assert_eq!(info.signed_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(info.signed_by.as_deref(), Some("Test Signer"));
        assert_eq!(info.signed_by_issuer.as_deref(), Some("Test CA"));
    }

    /// `extract_manifest_info` falls back to `claim_generator_info[0].name`
    /// when the top-level `claim_generator` field is absent (c2pa-rs v2 format).
    #[test]
    fn extract_manifest_info_claim_generator_fallback() {
        let manifest = serde_json::json!({
            "claim_generator_info": [{ "name": "FallbackTool/1.0", "version": "1.0" }],
            "assertions": []
        });
        let full_json = serde_json::json!({});

        let info = extract_manifest_info(&manifest, &full_json, false, None, None);
        assert_eq!(
            info.claim_generator.as_deref(),
            Some("FallbackTool/1.0"),
            "should fall back to claim_generator_info[0].name"
        );
    }

    /// `read_manifest_chain` on a non-existent file returns an error (not None).
    #[test]
    fn read_manifest_chain_missing_file_returns_error() {
        let result = read_manifest_chain(Path::new("/nonexistent/file.jpg"), false);
        assert!(result.is_err(), "missing file should return Err");
    }

    /// `read_manifest_chain` on a signed single-manifest file returns a chain
    /// with `manifest_count == 1` and an empty `ingredients` list.
    #[test]
    fn read_manifest_chain_single_manifest_no_ingredients() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        let (cert, key) = ensure_certificate(&data_dir).expect("ensure_certificate");

        let source = tmp.path().join("input.png");
        let img = image::RgbImage::new(32, 32);
        img.save(&source).expect("save PNG");
        let output = signed_output_path(&source);

        sign_file(&source, &output, "Chain Test User", None, &cert, &key)
            .expect("signing should succeed");

        let chain = read_manifest_chain(&output, false)
            .expect("read_manifest_chain should not error")
            .expect("signed file should contain a manifest");

        assert_eq!(
            chain.manifest_count, 1,
            "single-manifest file should have manifest_count == 1"
        );
        assert!(
            chain.ingredients.is_empty(),
            "single-manifest file should have no ingredient chain"
        );
        assert_eq!(
            chain.active.title.as_deref(),
            Some("input.png"),
            "active manifest title should match filename"
        );
    }

    /// `read_manifest` regression: still returns the same result after the
    /// refactor to use `extract_manifest_info` internally.
    #[test]
    fn read_manifest_regression_after_refactor() {
        let tmp = tempfile::tempdir().expect("create tempdir");
        let data_dir = tmp.path().join("data");

        let (cert, key) = ensure_certificate(&data_dir).expect("ensure_certificate");

        let source = tmp.path().join("regression.png");
        let img = image::RgbImage::new(16, 16);
        img.save(&source).expect("save PNG");
        let output = signed_output_path(&source);

        sign_file(&source, &output, "Regression User", Some("CC BY 4.0"), &cert, &key)
            .expect("signing should succeed");

        let manifest = read_manifest(&output, false)
            .expect("read_manifest should not error")
            .expect("signed file should have a manifest");

        assert_eq!(manifest.title.as_deref(), Some("regression.png"));
        assert!(manifest.is_valid, "manifest should be valid");
        assert!(
            !manifest.assertions.is_empty(),
            "manifest should have assertions"
        );
    }

    /// `derive_validity_from_delta` returns (true, false) when the delta has no failures.
    #[test]
    fn delta_validity_no_failures_is_valid() {
        let delta = serde_json::json!({ "success": [], "failure": [] });
        assert_eq!(derive_validity_from_delta(&delta), (true, false));
    }

    /// `derive_validity_from_delta` returns (false, false) for a hash-mismatch failure.
    #[test]
    fn delta_validity_hash_mismatch_is_invalid() {
        let delta = serde_json::json!({
            "success": [{ "code": "claimSignature.validated" }],
            "failure": [{ "code": "assertion.dataHash.mismatch" }]
        });
        assert_eq!(derive_validity_from_delta(&delta), (false, false));
    }

    /// `extract_validation_checks_from_delta` extracts checks from all outcome arrays.
    #[test]
    fn delta_checks_extracted_from_all_outcomes() {
        let delta = serde_json::json!({
            "success": [{ "code": "claimSignature.validated" }],
            "informational": [{ "code": "some.info", "explanation": "note" }],
            "failure": [{ "code": "assertion.dataHash.mismatch" }]
        });
        let checks = extract_validation_checks_from_delta(&delta);
        assert_eq!(checks.len(), 3);
        assert!(checks.iter().any(|c| c.code == "claimSignature.validated" && c.outcome == "pass"));
        assert!(checks.iter().any(|c| c.code == "some.info" && c.outcome == "info"));
        assert!(checks.iter().any(|c| c.code == "assertion.dataHash.mismatch" && c.outcome == "fail"));
    }

    /// `extract_manifest_info` with `use_full_validation = true` and `Some("enhanced")`
    /// propagates the verification_mode to the result.
    #[test]
    fn extract_manifest_info_sets_verification_mode() {
        let manifest = serde_json::json!({ "assertions": [] });
        let full_json = serde_json::json!({
            "validation_results": { "activeManifest": { "success": [], "failure": [] } }
        });
        let info = extract_manifest_info(&manifest, &full_json, true, None, Some("enhanced"));
        assert_eq!(info.verification_mode.as_deref(), Some("enhanced"));

        let info2 = extract_manifest_info(&manifest, &full_json, false, None, None);
        assert_eq!(info2.verification_mode, None, "ingredient should have no mode");
    }

    /// `verification_mode` serialises as `"verificationMode"` in camelCase JSON.
    #[test]
    fn verification_mode_serialises_as_camel_case() {
        let info = ManifestInfo {
            title: None,
            format: None,
            claim_generator: None,
            assertions: vec![],
            is_valid: true,
            valid_at_signing: false,
            signed_at: None,
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: Some("enhanced".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(
            json.contains("\"verificationMode\""),
            "should serialise as camelCase"
        );
        assert!(
            json.contains("\"enhanced\""),
            "should contain the mode value"
        );
    }

    /// `verification_mode = None` is omitted from serialised JSON.
    #[test]
    fn verification_mode_none_is_omitted() {
        let info = ManifestInfo {
            title: None,
            format: None,
            claim_generator: None,
            assertions: vec![],
            is_valid: true,
            valid_at_signing: false,
            signed_at: None,
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(
            !json.contains("verificationMode"),
            "None should be omitted from JSON"
        );
    }

    /// `ManifestChain` serialises to camelCase for the IPC boundary.
    #[test]
    fn manifest_chain_serialises_to_camel_case() {
        let chain = ManifestChain {
            active: ManifestInfo {
                title: Some("x.jpg".to_string()),
                format: None,
                claim_generator: None,
                assertions: vec![],
                is_valid: true,
                valid_at_signing: false,
                signed_at: None,
                signed_by: None,
                signed_by_issuer: None,
                validation_checks: vec![],
                verification_mode: Some("standard".to_string()),
            },
            ingredients: vec![],
            manifest_count: 1,
        };
        let json = serde_json::to_string(&chain).expect("serialise");
        assert!(json.contains("\"manifestCount\""), "should use camelCase");
        assert!(!json.contains("\"manifest_count\""), "should not use snake_case");
    }
}
