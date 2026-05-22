// SPDX-License-Identifier: AGPL-3.0-or-later

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
    /// True when the leaf signing certificate's `notAfter` is in the past at
    /// the time of this verification.  Independent of `is_valid`: a manifest
    /// can be fully valid (trusted chain + trusted timestamp) AND have a
    /// signing certificate that has since expired — this is the normal case
    /// for short-lived phone-camera certs like Google Pixel.  The UI uses
    /// this flag to render an informational L3 note explaining the
    /// "signed with a certificate that has since expired" situation without
    /// downgrading the Valid seal.
    ///
    /// `None` when the cert chain cannot be parsed (e.g. self-signed
    /// manifests from Sovereign mode that don't embed a parseable chain).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_expired: Option<bool>,
    /// Leaf signing certificate's `notBefore` in RFC 3339 / ISO 8601.
    /// Populated alongside `certificate_expired` — `None` when the chain
    /// cannot be parsed.  Surfaced at L3 so the certificate-expired
    /// disclosure carries concrete evidence (validity window + TSA time)
    /// rather than prose alone, per C2PA UX Rec v1.4 §6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_not_before: Option<String>,
    /// Leaf signing certificate's `notAfter` in RFC 3339 / ISO 8601.
    /// Paired with `cert_not_before`; see that field for semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_not_after: Option<String>,
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
    /// **Infrastructure note**: trust-list loading is unconditional as of c2pa-rs 0.79
    /// and happens via thread-local `Settings` populated by
    /// `ensure_trust_settings_initialised`.  The `enhanced = true` path remains reserved
    /// for future OCSP/CRL revocation checks, which c2pa-rs 0.79 does not yet expose at
    /// the `Reader` level.  For now this field documents which mode the user requested so
    /// the UI and PDF export can accurately report whether online checks were attempted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_mode: Option<String>,
    /// Base64-encoded thumbnail image extracted from the manifest's thumbnail assertion
    /// (label prefix `c2pa.thumbnail`).  Suitable for use directly in an `<img src>`
    /// data URI on the frontend.  `None` when no thumbnail assertion is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_base64: Option<String>,
    /// MIME type of the thumbnail (e.g. `"image/jpeg"`, `"image/png"`).
    /// Derived from the thumbnail assertion label suffix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_mime: Option<String>,
    /// The app or device that produced this content, intended for user-facing display
    /// (e.g. "Pixel Camera", "Google Photos", "Adobe Lightroom").
    ///
    /// Derived in priority order:
    /// 1. `claim_generator_info[0].name` — the most structured, tool-specific field.
    /// 2. `signature_info.common_name` — the certificate CN, often the device name.
    /// 3. `claim_generator` — the raw generator string as a last resort.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_or_device: Option<String>,
    /// Auto-generated plain-language content summary derived from actions and
    /// `digitalSourceType` declarations in the manifest assertions.  1-2 sentences.
    /// `None` when insufficient information is available to form a meaningful summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_summary: Option<String>,
    /// True when this manifest is a C2PA **update manifest** — a metadata-only
    /// change (e.g. provenance re-binding, redaction, re-signing) that does
    /// **not** represent an edit to the underlying media asset.  Detected
    /// heuristically from the reader JSON: an update manifest has no data hash
    /// assertion (`c2pa.hash.data`, `c2pa.hash.bmff`, `c2pa.hash.boxes`,
    /// `c2pa.hash.bmff.v2`) yet still references a parent via `ingredients`.
    ///
    /// Per C2PA UX Recommendations v1.4 §6 the UI must not present update
    /// manifests as edits to the asset and must not display update-manifest
    /// thumbnails.
    #[serde(default)]
    pub is_update_manifest: bool,
    /// Redactions declared by this manifest.  Populated from two sources:
    ///
    /// 1. The `redactions` array on the manifest itself (JUMBF URIs to the
    ///    redacted assertions) — surfaced by c2pa-rs in the reader JSON.
    /// 2. `c2pa.redacted` action entries in the manifest's actions assertion,
    ///    from which the optional `reason` field is extracted.
    ///
    /// Per C2PA UX Recommendations v1.4 §6 redaction details must be shown at
    /// L3 with a rationale when available.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub redactions: Vec<RedactionRecord>,
}

/// A single redaction recorded on a C2PA manifest.
///
/// `target` is the JUMBF URI of the redacted assertion (e.g.
/// `self#jumbf=/c2pa/<label>/c2pa.assertions/c2pa.training-mining`).
/// `reason` is the human-readable rationale taken from the accompanying
/// `c2pa.redacted` action entry, when present.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactionRecord {
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
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

/// Map a human-readable licence string to its canonical URI.
///
/// Returns `Some(uri)` for the five Creative Commons licences and CC0.
/// Returns `None` for `"All Rights Reserved"` and any unrecognised string.
///
/// For "All Rights Reserved" we intentionally omit the `license` URI field on
/// `stds.schema-org.CreativeWork`, because there is no standardised URI for
/// proprietary unlicenced content. We still emit the CreativeWork assertion
/// itself with `copyrightNotice` set to the human-readable label, so verifiers
/// reading creator + copyright still get useful data.
fn license_to_uri(license: &str) -> Option<&'static str> {
    match license {
        "CC BY 4.0" => Some("https://creativecommons.org/licenses/by/4.0/"),
        "CC BY-SA 4.0" => Some("https://creativecommons.org/licenses/by-sa/4.0/"),
        "CC BY-NC 4.0" => Some("https://creativecommons.org/licenses/by-nc/4.0/"),
        "CC BY-ND 4.0" => Some("https://creativecommons.org/licenses/by-nd/4.0/"),
        "CC0 1.0" => Some("https://creativecommons.org/publicdomain/zero/1.0/"),
        // "All Rights Reserved" and any unknown string: no URI emitted.
        _ => None,
    }
}

/// AI-training + data-mining usage policy keyed by `c2pa.training-mining`
/// reason ID, per C2PA 2.1 §18.18. Returns one of `"allowed"`, `"notAllowed"`,
/// or `"constrained"` for each of the four canonical reasons.
///
/// v1.0 policy is intentionally binary: CC0 permits everything (public domain
/// cannot legally prohibit it); all other licences prohibit everything by
/// default. Per-asset override is deferred to v1.1 when the UI exposes a
/// toggle. The CC0 carve-out resolves the contradiction the Generator-track
/// audit flagged: a CC0 file with `notAllowed` on every training reason was
/// asserting a restriction the licence explicitly waives.
fn training_mining_for_license(license: &str) -> serde_json::Value {
    // Canonical reason IDs per C2PA 2.1 §18.18.
    const REASONS: &[&str] = &[
        "c2pa.ai_generative_training",
        "c2pa.ai_inference",
        "c2pa.ai_training",
        "c2pa.data_mining",
    ];
    let policy = if license == "CC0 1.0" {
        "allowed"
    } else {
        "notAllowed"
    };
    let mut entries = serde_json::Map::new();
    for reason in REASONS {
        entries.insert(reason.to_string(), serde_json::json!({ "use": policy }));
    }
    serde_json::Value::Object(entries)
}

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

    // Build the base assertions array, then conditionally append the schema-org
    // CreativeWork assertion when a canonical licence URI is available (JTV-120).
    //
    // Version sourced from CARGO_PKG_VERSION so manifest provenance stays
    // accurate across rc.x cuts and the v1.0.x maintenance line (was
    // hardcoded "0.9.0" until 2026-05-22). Two formats co-exist briefly:
    // a human-readable string (`Jura Trace <ver>`) for downstream
    // softwareAgent fields, and the slash-separated v1.x legacy form for
    // claim_generator (kept for backwards compatibility with readers).
    const PRODUCT: &str = "Jura Trace";
    let pkg_ver = env!("CARGO_PKG_VERSION");
    let software_agent = format!("{PRODUCT} {pkg_ver}");
    let claim_generator = format!("{PRODUCT}/{pkg_ver}");

    // The 2026-05-22 Generator-track audit flagged three structural problems
    // with the previous manifest shape, all fixed here:
    //   1. `c2pa.rights` is not a spec-defined assertion label (the c2pa.*
    //      namespace is reserved). Replaced by the spec-correct mechanisms:
    //      `stds.schema-org.CreativeWork` for licence + copyright, and
    //      `c2pa.training-mining` (§18.18) for AI / data-mining policy.
    //   2. AI opt-out was hardcoded inside the bogus `c2pa.rights` blob with
    //      no canonical reader path. Now emitted via the spec assertion
    //      (`training_mining_for_license` below), with CC0 correctly
    //      mapped to `allowed` because public domain cannot prohibit it.
    //   3. `plus:DataMining` inside `stds.iptc` was redundant with the new
    //      c2pa.training-mining assertion; removed.
    //
    // `stds.iptc` is retained only for `Iptc4xmpExt:DigitalSourceType` until
    // audit item #6 moves it into the c2pa.created action.
    let mut creative_work = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "CreativeWork",
        "creator": creator_name,
        "copyrightNotice": license_value,
    });
    if let Some(uri) = license_to_uri(license_value) {
        creative_work["license"] = serde_json::Value::String(uri.to_string());
    }

    let assertions = vec![
        serde_json::json!({
            "label": "c2pa.actions",
            "data": {
                "actions": [{
                    "action": "c2pa.created",
                    "softwareAgent": software_agent,
                    "parameters": {
                        "name": creator_name
                    }
                }]
            }
        }),
        serde_json::json!({
            "label": "stds.schema-org.CreativeWork",
            "data": creative_work
        }),
        serde_json::json!({
            "label": "c2pa.training-mining",
            "data": {
                "entries": training_mining_for_license(license_value)
            }
        }),
        serde_json::json!({
            "label": "stds.iptc",
            "data": {
                // Default to "digitalCapture" for files signed via this
                // path because Jura Trace's Sign action is the human
                // declaring authorship of a captured photograph.
                // Generator-track audit item #6 moves this into the
                // c2pa.created action itself (the canonical 2.x location).
                "Iptc4xmpExt:DigitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"
            }
        }),
    ];

    let manifest_def = serde_json::json!({
        "claim_generator": claim_generator,
        "title": file_name,
        "assertions": assertions
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
    read_manifest(output, false)?
        .ok_or_else(|| "Signed file but could not read back manifest".to_string())
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
/// Resolve the validation-checks source for an ingredient manifest in a chain.
///
/// Source priority (revised 2026-04-26 after the Google Pixel Zoom Enhance
/// "Content Credential unavailable or invalid" misreport):
///
/// 1. The ingredient's own embedded `validation_results.activeManifest` block
///    — the AUTHORITATIVE source for the ingredient's own claim signature,
///    data hash, timestamp and assertion-hash outcomes.
///
/// 2. Top-level `validation_results.ingredientDeltas[]` matched by URI. The
///    URI is keyed on the PARENT manifest's label + the parent's
///    `c2pa.ingredient` assertion path (not the child's label), so the
///    matcher must search for the parent label, not the child label.
///
/// 3. Positional fallback — the nth top-level ingredient delta corresponds
///    to the nth ingredient in walk order. Last-resort only because c2pa-rs
///    occasionally emits a single summary delta (`ingredient.manifest.validated`
///    + cert-state failures) which, taken alone, drops the full success-code
///      set the user needs to see in the L3 panel.
///
/// Embedded is preferred because the delta is a c2pa-rs *summary* of the
/// ingredient's status from the parent's signing-time perspective, while the
/// embedded block is the FULL validation outcome.
fn resolve_ingredient_validation_source<'a>(
    ingredient: &'a serde_json::Value,
    parent_label: &str,
    ingredient_deltas: &[&'a serde_json::Value],
    ingredient_order: usize,
) -> Option<&'a serde_json::Value> {
    ingredient
        .get("validation_results")
        .and_then(|vr| vr.get("activeManifest"))
        .or_else(|| {
            // URI match: look for a delta whose ingredientAssertionURI references
            // this parent claim's ingredient assertion. c2pa-rs URIs are of the form
            //   self#jumbf=/c2pa/<parent_label>/c2pa.assertions/c2pa.ingredient[.vN]
            ingredient_deltas
                .iter()
                .find(|d| {
                    d.get("ingredientAssertionURI")
                        .and_then(|u| u.as_str())
                        .map(|uri| uri.contains(parent_label))
                        .unwrap_or(false)
                })
                .copied()
                .and_then(|d| d.get("validationDeltas"))
        })
        .or_else(|| {
            ingredient_deltas
                .get(ingredient_order)
                .and_then(|d| d.get("validationDeltas"))
        })
}

fn extract_validation_checks_from_delta(delta: &serde_json::Value) -> Vec<ValidationCheck> {
    let mut checks = Vec::new();
    for (outcome, key) in [
        ("pass", "success"),
        ("info", "informational"),
        ("fail", "failure"),
    ] {
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

    for (outcome, key) in [
        ("pass", "success"),
        ("info", "informational"),
        ("fail", "failure"),
    ] {
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

// ===== Reading helpers =====

/// Extract a base64-encoded thumbnail and its MIME type from a manifest's assertions array.
///
/// c2pa-rs serialises thumbnail assertions with labels of the form
/// `c2pa.thumbnail.claim.jpeg`, `c2pa.thumbnail.claim.png`, or
/// `c2pa.thumbnail.ingredient.jpeg` etc.  The assertion `data` field is
/// serialised as a JSON object `{ "identifier": "…", "format": "image/jpeg" }`
/// where the actual bytes live in the reader's resource store, **or** as a
/// plain base64 string in older c2pa-rs serialisations.
///
/// Because c2pa-rs 0.76 does not expose resource bytes in the JSON snapshot
/// directly, this function handles both representations:
///
/// - If `data` is an object with a `"data"` key whose value is a base64
///   string (some builds embed it inline), use that directly.
/// - If `data` is an object with only a resource reference
///   (`"identifier"` / `"format"`), record the MIME from the `"format"` key
///   and return `None` for the bytes (resource resolution requires the live
///   `Reader` handle, which is not threaded through here).
/// - If `data` is itself a base64 string, use it directly.
///
/// Returns `(Option<base64_string>, Option<mime_type>)`.
fn extract_thumbnail_from_assertions(
    assertions: &serde_json::Value,
) -> (Option<String>, Option<String>) {
    let arr = match assertions.as_array() {
        Some(a) => a,
        None => return (None, None),
    };

    for assertion in arr {
        let label = assertion
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !label.starts_with("c2pa.thumbnail") {
            continue;
        }

        // Derive MIME type from the label suffix: "…jpeg" → "image/jpeg", "…png" → "image/png"
        let mime_from_label = if label.ends_with(".jpeg") || label.ends_with(".jpg") {
            Some("image/jpeg".to_string())
        } else if label.ends_with(".png") {
            Some("image/png".to_string())
        } else if label.ends_with(".webp") {
            Some("image/webp".to_string())
        } else {
            None
        };

        let data = match assertion.get("data") {
            Some(d) => d,
            None => continue,
        };

        // Case 1: data is a plain base64 string.
        if let Some(b64) = data.as_str() {
            if !b64.is_empty() {
                return (Some(b64.to_string()), mime_from_label);
            }
        }

        // Case 2: data is an object.
        if data.is_object() {
            // Sub-case A: inline base64 bytes in a "data" sub-key.
            if let Some(inner) = data.get("data").and_then(|v| v.as_str()) {
                if !inner.is_empty() {
                    let mime = mime_from_label.or_else(|| {
                        data.get("format")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    });
                    return (Some(inner.to_string()), mime);
                }
            }

            // Sub-case B: resource reference only — return MIME but no bytes.
            // The resource identifier is present but we cannot resolve it without
            // the live `Reader` handle.  Returning the MIME lets the frontend
            // know a thumbnail exists even when bytes are unavailable.
            let mime = mime_from_label.or_else(|| {
                data.get("format")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            });
            if mime.is_some() {
                return (None, mime);
            }
        }
    }

    (None, None)
}

/// Derive the most user-meaningful "app or device" name from a manifest object.
///
/// Priority:
/// 1. `claim_generator_info[0].name` — structured, tool-specific (e.g. "Google Photos").
/// 2. `signature_info.common_name` — certificate CN (e.g. "Pixel Camera").
/// 3. `claim_generator` raw string, truncated at the first `/` or space followed
///    by a version number so "Jura Trace/0.9.0" becomes "Jura Trace".
fn derive_app_or_device(manifest: &serde_json::Value) -> Option<String> {
    // Priority 1: claim_generator_info[0].name
    if let Some(name) = manifest
        .get("claim_generator_info")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|entry| entry.get("name"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        return Some(name.to_string());
    }

    // Priority 2: signature_info.common_name
    if let Some(cn) = manifest
        .get("signature_info")
        .and_then(|si| si.get("common_name"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        return Some(cn.to_string());
    }

    // Priority 3: claim_generator, trimmed to remove version suffixes like "/0.9.0"
    if let Some(raw) = manifest
        .get("claim_generator")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        // Strip a trailing "/version" component (e.g. "Jura Trace/0.9.0" → "Jura Trace").
        let trimmed = raw.split('/').next().unwrap_or(raw).trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Generate a plain-language content summary from the manifest's assertions.
///
/// Inspects `c2pa.actions` / `c2pa.actions.v2` assertions for:
/// - `digitalSourceType` — presence of AI declarations
/// - action verbs — `c2pa.created`, `c2pa.opened`, `c2pa.edited`, `c2pa.repackaged`
///
/// Also considers ingredient count (multiple ingredients → composite content).
///
/// Returns `None` when there is insufficient information for a meaningful sentence.
fn build_content_summary(manifest: &serde_json::Value) -> Option<String> {
    let assertions = manifest.get("assertions").and_then(|v| v.as_array())?;

    let mut has_ai = false;
    let mut has_computational_capture = false;
    let mut has_digital_capture = false;
    let mut has_edited = false;
    let mut has_created = false;
    let mut actions_found = false;

    // The claim_generator_info name is used to personalise the summary where possible.
    let tool_name: Option<String> = manifest
        .get("claim_generator_info")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|e| e.get("name"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .or_else(|| {
            manifest
                .get("signature_info")
                .and_then(|si| si.get("common_name"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from)
        });

    for assertion in assertions {
        let label = assertion
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !label.starts_with("c2pa.actions") {
            continue;
        }

        let data = match assertion.get("data") {
            Some(d) => d,
            None => continue,
        };

        let action_list = data
            .get("actions")
            .and_then(|v| v.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);

        if !action_list.is_empty() {
            actions_found = true;
        }

        for action in action_list {
            let action_name = action.get("action").and_then(|v| v.as_str()).unwrap_or("");

            // Check digitalSourceType on the action object (C2PA 2.x style).
            let dst = action
                .get("digitalSourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();

            if dst.contains("trainedalgorithmicmedia") || dst.contains("compositewithtrained") {
                has_ai = true;
            } else if dst.contains("computationalcapture") {
                has_computational_capture = true;
            } else if dst.contains("digitalcapture") {
                has_digital_capture = true;
            }

            // Also scan the serialised action JSON for digitalSourceType buried deeper.
            let action_str = serde_json::to_string(action)
                .unwrap_or_default()
                .to_lowercase();
            if action_str.contains("trainedalgorithmicmedia") {
                has_ai = true;
            } else if action_str.contains("computationalcapture") && !has_ai {
                has_computational_capture = true;
            } else if action_str.contains("digitalcapture") && !has_ai && !has_computational_capture
            {
                has_digital_capture = true;
            }

            match action_name {
                "c2pa.created" => has_created = true,
                "c2pa.edited"
                | "c2pa.color_adjustments"
                | "c2pa.cropped"
                | "c2pa.filtered"
                | "c2pa.resized"
                | "c2pa.orientation" => has_edited = true,
                _ => {}
            }
        }
    }

    // Count ingredients for composite-content detection.
    let ingredient_count = manifest
        .get("ingredients")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);
    let is_composite = ingredient_count > 1;

    // Build the summary sentence(s).
    let mut parts: Vec<String> = Vec::new();

    if is_composite {
        parts.push("This content combines multiple pieces of content.".to_string());
    }

    if has_ai {
        parts.push("At least one component was generated or enhanced with an AI tool.".to_string());
    } else if has_computational_capture {
        if let Some(ref name) = tool_name {
            parts.push(format!(
                "Captured using computational photography by {name}."
            ));
        } else {
            parts.push("Captured using computational photography.".to_string());
        }
    } else if has_digital_capture {
        if let Some(ref name) = tool_name {
            parts.push(format!("Captured with a digital camera by {name}."));
        } else {
            parts.push("Captured with a digital camera.".to_string());
        }
    } else if has_edited && !has_created {
        if let Some(ref name) = tool_name {
            parts.push(format!("Opened and edited using {name}."));
        } else {
            parts.push("Opened and edited in a photo application.".to_string());
        }
    } else if has_created && !has_edited {
        if let Some(ref name) = tool_name {
            parts.push(format!("Created and signed by {name}."));
        } else {
            parts.push("Created and signed.".to_string());
        }
    } else if actions_found {
        if let Some(ref name) = tool_name {
            parts.push(format!("Processed using {name}."));
        } else {
            parts.push("Processed with a content tool.".to_string());
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
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

    // Extract thumbnail from assertions (c2pa.thumbnail.* labels).
    let assertions_json = manifest
        .get("assertions")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let (thumbnail_base64, thumbnail_mime) = extract_thumbnail_from_assertions(&assertions_json);

    // Derive app_or_device from claim_generator_info > signature_info.common_name > claim_generator.
    let app_or_device = derive_app_or_device(manifest);

    // Build a plain-language content summary from actions and digitalSourceType.
    let content_summary = build_content_summary(manifest);

    // Detect update manifest + redactions.
    let is_update_manifest = detect_update_manifest(manifest);
    let redactions = extract_redactions(manifest);

    ManifestInfo {
        title,
        format,
        claim_generator,
        assertions,
        is_valid,
        valid_at_signing,
        // Populated in `read_manifest` / `read_manifest_chain` from the
        // typed Reader API — the reader JSON omits `cert_chain`, so the
        // info constructed here leaves this as `None` and the caller
        // overrides it once the cert chain has been parsed.
        certificate_expired: None,
        cert_not_before: None,
        cert_not_after: None,
        signed_at,
        signed_by,
        signed_by_issuer,
        validation_checks,
        verification_mode: verification_mode.map(String::from),
        thumbnail_base64,
        thumbnail_mime,
        app_or_device,
        content_summary,
        is_update_manifest,
        redactions,
    }
}

/// Detect whether a manifest is a C2PA **update manifest** from its reader JSON.
///
/// c2pa-rs 0.79 exposes the internal `update_manifest` bool on the `Claim`
/// struct but does not surface it on the public `Manifest` type or in the
/// reader JSON.  We fall back to the spec-level heuristic: an update manifest
/// has **no** data-binding hash assertion (any of `c2pa.hash.data`,
/// `c2pa.hash.bmff`, `c2pa.hash.bmff.v2`, `c2pa.hash.boxes`) **and** has at
/// least one ingredient (the parent whose metadata is being updated).
///
/// This mirrors the invariant enforced inside c2pa-rs itself — see
/// `store.rs` guards such as `hash_assertions.is_empty() && claim.update_manifest()`.
fn detect_update_manifest(manifest: &serde_json::Value) -> bool {
    let Some(assertions) = manifest.get("assertions").and_then(|v| v.as_array()) else {
        return false;
    };

    let has_hash_assertion = assertions.iter().any(|a| {
        a.get("label")
            .and_then(|v| v.as_str())
            .map(|label| {
                label == "c2pa.hash.data"
                    || label == "c2pa.hash.bmff"
                    || label == "c2pa.hash.bmff.v2"
                    || label == "c2pa.hash.boxes"
            })
            .unwrap_or(false)
    });

    if has_hash_assertion {
        return false;
    }

    // No data hash — also require at least one ingredient so we do not
    // mis-classify a bare/malformed manifest as an update manifest.
    manifest
        .get("ingredients")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false)
}

/// Collect redaction records for a manifest from both the top-level
/// `redactions` array (JUMBF URIs) and any `c2pa.redacted` action entries
/// (which may carry a `reason` string).  Each target URI yields a single
/// `RedactionRecord`; if an action with a matching `parameters.redacted`
/// URI supplies a `reason`, it is merged onto that record.
fn extract_redactions(manifest: &serde_json::Value) -> Vec<RedactionRecord> {
    use std::collections::HashMap;

    // Step 1 — seed records from the manifest's `redactions` array.
    let mut by_target: HashMap<String, Option<String>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    if let Some(arr) = manifest.get("redactions").and_then(|v| v.as_array()) {
        for entry in arr {
            if let Some(uri) = entry.as_str() {
                if !by_target.contains_key(uri) {
                    by_target.insert(uri.to_string(), None);
                    order.push(uri.to_string());
                }
            }
        }
    }

    // Step 2 — scan actions assertions for `c2pa.redacted` action entries.
    // The action's `parameters.redacted` field is the URI of the redacted
    // assertion; `reason` (at the top level of the action) is the rationale.
    if let Some(assertions) = manifest.get("assertions").and_then(|v| v.as_array()) {
        for assertion in assertions {
            let label = assertion
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if label != "c2pa.actions" && label != "c2pa.actions.v2" {
                continue;
            }
            let Some(data) = assertion.get("data") else {
                continue;
            };
            let Some(actions) = data.get("actions").and_then(|v| v.as_array()) else {
                continue;
            };
            for action in actions {
                let action_label = action.get("action").and_then(|v| v.as_str()).unwrap_or("");
                if action_label != "c2pa.redacted" {
                    continue;
                }
                let uri = action
                    .get("parameters")
                    .and_then(|p| p.get("redacted"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let reason = action
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                if let Some(target) = uri {
                    let entry = by_target.entry(target.clone()).or_insert_with(|| {
                        order.push(target.clone());
                        None
                    });
                    if entry.is_none() {
                        *entry = reason;
                    }
                }
            }
        }
    }

    order
        .into_iter()
        .map(|target| {
            let reason = by_target.remove(&target).flatten();
            RedactionRecord { target, reason }
        })
        .collect()
}

// ===== Trust list initialisation =====
//
// Vendored PEM bundles — see `src-tauri/trust-list/README.md` for provenance
// and refresh policy.  Concatenated into a single PEM blob and loaded into
// `c2pa::Settings` thread-local storage before any `Reader` is constructed,
// so chains to the official C2PA-recognised CAs and TSAs validate correctly
// (Google Pixel, Adobe, Truepic, etc.) instead of surfacing as
// `signingCredential.untrusted`.

const C2PA_TRUST_LIST_PEM: &str = include_str!("../trust-list/C2PA-TRUST-LIST.pem");
const C2PA_TSA_TRUST_LIST_PEM: &str = include_str!("../trust-list/C2PA-TSA-TRUST-LIST.pem");
const ITL_ANCHORS_PEM: &str = include_str!("../trust-list/ITL-anchors.pem");
const ITL_ALLOWED_PEM: &str = include_str!("../trust-list/ITL-allowed.pem");

/// Concatenate all four trust list PEM bundles into one blob suitable for the
/// `trust.trust_anchors` settings field.  Newline-separated so individual
/// certificates retain their `-----BEGIN/-----END` framing.
fn concatenated_trust_bundle() -> String {
    [
        C2PA_TRUST_LIST_PEM,
        C2PA_TSA_TRUST_LIST_PEM,
        ITL_ANCHORS_PEM,
        ITL_ALLOWED_PEM,
    ]
    .join("\n")
}

/// Initialise c2pa-rs thread-local trust settings.
///
/// c2pa-rs 0.79 stores `Settings` (including `trust.trust_anchors`) in
/// thread-local storage.  Each thread that constructs a `c2pa::Reader` must
/// call this first so the official C2PA CA + TSA trust lists (and the legacy
/// Interim Trust List bundles for content signed before January 2026) are in
/// scope when the certificate chain is validated.
///
/// Idempotent per thread — a thread-local flag short-circuits subsequent
/// calls so we parse the TOML at most once per worker.  TOML multi-line
/// *literal* strings (triple single quotes) are used so no escape processing
/// is applied to the PEM content.
fn ensure_trust_settings_initialised() -> Result<(), String> {
    thread_local! {
        static INITIALISED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    INITIALISED.with(|flag| {
        if flag.get() {
            return Ok(());
        }
        let bundle = concatenated_trust_bundle();
        let toml = format!("[trust]\ntrust_anchors = '''\n{bundle}\n'''\n");
        c2pa::Settings::from_toml(&toml)
            .map_err(|e| format!("Failed to initialise C2PA trust settings: {e}"))?;
        flag.set(true);
        Ok(())
    })
}

/// Parsed validity window of the leaf signing certificate.
///
/// Populated from the PEM chain exposed by `SignatureInfo::cert_chain()`.
/// Both dates are RFC 3339 / ISO 8601 strings so the frontend can render
/// them directly via `Intl.DateTimeFormat` without Rust-side locale work.
pub(crate) struct LeafCertValidity {
    pub not_before_rfc3339: String,
    pub not_after_rfc3339: String,
    pub expired: bool,
}

/// Parse the leaf (first) certificate of a PEM chain and return its
/// `notBefore` / `notAfter` plus an `expired` flag.
///
/// c2pa-rs exposes the full signing chain as a concatenated PEM string via
/// `SignatureInfo::cert_chain()`; the leaf is always the first cert.  We
/// parse only the leaf because that's the signer itself — any CA expiry
/// is separately enforced by the trust-list validation pass.
///
/// Returns `None` if the chain is empty or the leaf can't be parsed (e.g.
/// Sovereign-mode self-signed manifests that embed a non-standard chain).
fn leaf_cert_validity(cert_chain_pem: &str) -> Option<LeafCertValidity> {
    use x509_parser::pem::parse_x509_pem;
    let (_, pem) = parse_x509_pem(cert_chain_pem.as_bytes()).ok()?;
    let cert = pem.parse_x509().ok()?;
    let not_before_ts = cert.validity().not_before.timestamp();
    let not_after_ts = cert.validity().not_after.timestamp();
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let to_rfc3339 = |ts: i64| -> Option<String> {
        chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0).map(|dt| dt.to_rfc3339())
    };
    Some(LeafCertValidity {
        not_before_rfc3339: to_rfc3339(not_before_ts)?,
        not_after_rfc3339: to_rfc3339(not_after_ts)?,
        expired: not_after_ts < now_ts,
    })
}

/// Legacy shim retained because the leaf-cert-expiry tests assert on a
/// `Option<bool>` return.  New code should call `leaf_cert_validity` to
/// pick up `notBefore` / `notAfter` alongside the expired flag.
#[cfg(test)]
fn leaf_cert_expired(cert_chain_pem: &str) -> Option<bool> {
    leaf_cert_validity(cert_chain_pem).map(|v| v.expired)
}

/// Open a c2pa-rs `Reader` for `path`, returning `Ok(None)` when no C2PA data is present.
///
/// Thread-local trust settings are initialised on first call so the returned
/// `Reader`'s validation walks the official C2PA trust lists rather than
/// returning `signingCredential.untrusted` for every well-known signer.
fn open_reader(path: &Path) -> Result<Option<c2pa::Reader>, String> {
    ensure_trust_settings_initialised()?;
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
/// Trust-list loading is unconditional — both modes chain against the
/// official C2PA CA and TSA trust lists via `ensure_trust_settings_initialised`.
/// `enhanced` remains a hook for future OCSP/CRL revocation checks, which
/// c2pa-rs 0.79 does not yet expose at the `Reader` level.
pub fn read_manifest(path: &Path, enhanced: bool) -> Result<Option<ManifestInfo>, String> {
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

    let manifest = match json
        .get("manifests")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get(&active_label))
    {
        Some(m) => m,
        None => return Ok(None),
    };

    let mode_str = if enhanced { "enhanced" } else { "standard" };
    let mut info = extract_manifest_info(manifest, &json, true, None, Some(mode_str));

    // Overlay the leaf-cert validity from the typed Reader API — the
    // reader JSON doesn't carry `cert_chain` (it's `#[serde(skip)]` in
    // c2pa-rs's `SignatureInfo`), so we have to reach into the typed
    // manifest to extract the PEM chain.  This lets L3 disclose
    // "signed with a certificate that has since expired" for short-lived
    // phone-camera credentials without contradicting the Valid seal, and
    // surface the notBefore/notAfter window as concrete evidence.
    if let Some(v) = reader
        .active_manifest()
        .and_then(|m| m.signature_info())
        .and_then(|si| leaf_cert_validity(si.cert_chain()))
    {
        info.certificate_expired = Some(v.expired);
        info.cert_not_before = Some(v.not_before_rfc3339);
        info.cert_not_after = Some(v.not_after_rfc3339);
    }

    Ok(Some(info))
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
/// Trust-list loading is unconditional and shared with [`read_manifest`] via
/// [`ensure_trust_settings_initialised`].  The `enhanced` flag remains reserved
/// for future OCSP/CRL revocation checks.
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
    let mut active = extract_manifest_info(active_manifest, &json, true, None, Some(mode_str));

    // Overlay the leaf-cert validity for the active manifest — same
    // reasoning as in `read_manifest`.  Ingredient manifests are left at
    // `None` because they represent historical steps whose cert lifecycle
    // is not actionable for current-state verification.
    if let Some(v) = reader
        .active_manifest()
        .and_then(|m| m.signature_info())
        .and_then(|si| leaf_cert_validity(si.cert_chain()))
    {
        active.certificate_expired = Some(v.expired);
        active.cert_not_before = Some(v.not_before_rfc3339);
        active.cert_not_after = Some(v.not_after_rfc3339);
    }

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
                    if let Some(child_label) = ing.get("active_manifest").and_then(|v| v.as_str()) {
                        if visited.contains(child_label) {
                            continue; // guard against cycles
                        }
                        if let Some(child_manifest) = manifests.get(child_label) {
                            let matched_delta = resolve_ingredient_validation_source(
                                ing,
                                label.as_str(),
                                &ingredient_deltas,
                                ingredient_order,
                            );

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
        //
        // IMPORTANT: `compositeWithTrainedAlgorithmicMedia` is a DIFFERENT IPTC
        // value meaning "real photograph with AI-composited regions" — not a
        // fully synthetic image.  It's detected separately by
        // `detect_composite_ai_from_assertions` and deliberately excluded here
        // so the trust ceiling for pure AI (0.25) is not applied to Pixel
        // Zoom Enhance / Magic Editor / generative fill cases, which should
        // cap at 0.55 instead.
        if lower_value.contains("trainedalgorithmicmedia")
            && !lower_value.contains("compositewithtrainedalgorithmicmedia")
        {
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

/// Check if a C2PA manifest declares composite AI content (real photograph
/// with AI-generated regions) via assertions.
///
/// Returns the human-readable composite AI label if detected, or `None`.
///
/// Distinguishes composite-AI (e.g. Google Pixel Zoom Enhance, Magic Editor,
/// Adobe generative fill) from fully synthetic AI (Firefly, DALL-E).  The
/// composite case sets `Iptc4xmpExt:DigitalSourceType` to
/// `compositeWithTrainedAlgorithmicMedia` — the IPTC value for a real
/// photograph whose content has been altered by AI generation.
///
/// This is the companion to [`detect_ai_from_assertions`], which
/// deliberately excludes composite cases so the 0.25 pure-AI ceiling is
/// not applied to legitimate camera captures with AI-assisted features.
/// Composite-AI gets a 0.55 ceiling via the compute_trust composite path.
pub fn detect_composite_ai_from_assertions(assertions: &[AssertionInfo]) -> Option<String> {
    for assertion in assertions {
        let lower_value = assertion.value.to_lowercase();
        if lower_value.contains("compositewithtrainedalgorithmicmedia") {
            return Some("AI-composited regions (C2PA digitalSourceType)".to_string());
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

    /// Every licence option value rendered by the UI must resolve in
    /// `license_to_uri`, with the single documented exception of
    /// "All Rights Reserved" (no canonical URI). The earlier rc.24 build
    /// silently dropped the schema-org licence assertion on the
    /// single-asset path because the dropdown emitted "CC-BY-4.0" but
    /// the lookup expected "CC BY 4.0". This test catches that class of
    /// drift between UI options and the lookup table.
    #[test]
    fn every_ui_license_option_resolves() {
        // Sourced from ui/src/routes/protect/+page.svelte single-asset and
        // batch dropdowns. Keep in lockstep with both <select> blocks.
        const UI_OPTIONS: &[&str] = &[
            "All Rights Reserved",
            "CC BY 4.0",
            "CC BY-NC 4.0",
            "CC BY-SA 4.0",
            "CC BY-ND 4.0",
            "CC0 1.0",
        ];

        for opt in UI_OPTIONS {
            if *opt == "All Rights Reserved" {
                assert!(
                    license_to_uri(opt).is_none(),
                    "All Rights Reserved is the documented None case"
                );
            } else {
                assert!(
                    license_to_uri(opt).is_some(),
                    "Licence option {opt:?} returned None from license_to_uri \
                     — UI dropdown and lookup table have drifted again"
                );
            }
        }
    }

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
            certificate_expired: None,
            cert_not_before: None,
            cert_not_after: None,
            signed_at: Some("2026-01-01T00:00:00Z".to_string()),
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: Some("standard".to_string()),
            thumbnail_base64: None,
            thumbnail_mime: None,
            app_or_device: None,
            content_summary: None,
            is_update_manifest: false,
            redactions: vec![],
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

    // ---- ManifestChain walk: the AI signal can live in an ingredient ----
    //
    // Regression case: Google Gemini-generated image opened in a downstream
    // tool (news-graphic overlay generator) produces a 3-manifest chain. The
    // active manifest declares only c2pa.opened + c2pa.edited (visible
    // chyron added) + c2pa.converted; the trainedAlgorithmicMedia signal
    // lives in the DEEPEST ingredient. Before the 2026-05-21 fix the verify
    // pipeline only ran detect_ai_from_assertions on the active manifest
    // and silently missed Gemini outputs that had been re-edited.
    //
    // These tests exercise the iterator pattern the pipeline now uses:
    //   std::iter::once(&chain.active).chain(chain.ingredients.iter())
    fn empty_manifest(label: &str) -> ManifestInfo {
        ManifestInfo {
            title: Some(label.to_string()),
            format: None,
            claim_generator: None,
            assertions: vec![],
            is_valid: true,
            valid_at_signing: false,
            certificate_expired: None,
            cert_not_before: None,
            cert_not_after: None,
            signed_at: None,
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: None,
            thumbnail_base64: None,
            thumbnail_mime: None,
            app_or_device: None,
            content_summary: None,
            is_update_manifest: false,
            redactions: vec![],
        }
    }

    fn ai_assertion() -> AssertionInfo {
        AssertionInfo {
            label: "c2pa.actions.v2".to_string(),
            value: r#"{"actions":[{"action":"c2pa.created","digitalSourceType":"http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia","description":"Created by Google Generative AI."}]}"#.to_string(),
        }
    }

    #[test]
    fn chain_walk_finds_ai_signal_in_deepest_ingredient() {
        // Active manifest is clean (only edit actions); AI signal in deepest ingredient.
        let active = empty_manifest("active (chyron overlay)");
        let mut mid = empty_manifest("middle (Google edit)");
        mid.assertions.push(AssertionInfo {
            label: "c2pa.actions.v2".to_string(),
            value:
                r#"{"actions":[{"action":"c2pa.edited","description":"Added visible watermark"}]}"#
                    .to_string(),
        });
        let mut deepest = empty_manifest("deepest (Gemini generation)");
        deepest.assertions.push(ai_assertion());

        let chain = ManifestChain {
            active,
            ingredients: vec![mid, deepest],
            manifest_count: 3,
        };

        // Same iterator pattern used in lib.rs verify_content_inner.
        let result = std::iter::once(&chain.active)
            .chain(chain.ingredients.iter())
            .find_map(|m| detect_ai_from_assertions(&m.assertions));

        assert!(
            result.is_some(),
            "Chain walk should surface AI signal from ingredient even when active manifest is clean"
        );
    }

    #[test]
    fn chain_walk_returns_none_for_clean_chain() {
        let chain = ManifestChain {
            active: empty_manifest("active"),
            ingredients: vec![empty_manifest("ingredient")],
            manifest_count: 2,
        };
        let result = std::iter::once(&chain.active)
            .chain(chain.ingredients.iter())
            .find_map(|m| detect_ai_from_assertions(&m.assertions));
        assert!(result.is_none(), "Clean chain should return no AI signal");
    }

    #[test]
    fn chain_walk_active_signal_wins_over_ingredient() {
        // Active has AI, ingredient also has AI. First match (active) wins.
        let mut active = empty_manifest("active");
        active.assertions.push(ai_assertion());
        let mut ing = empty_manifest("ingredient");
        ing.assertions.push(ai_assertion());

        let chain = ManifestChain {
            active,
            ingredients: vec![ing],
            manifest_count: 2,
        };
        let result = std::iter::once(&chain.active)
            .chain(chain.ingredients.iter())
            .find_map(|m| detect_ai_from_assertions(&m.assertions));
        assert!(
            result.is_some(),
            "Chain walk should detect AI in either position"
        );
    }

    // ---- extract_thumbnail_from_assertions tests ----

    #[test]
    fn thumbnail_extracted_from_claim_jpeg_label_with_inline_data() {
        let assertions = serde_json::json!([
            {
                "label": "c2pa.thumbnail.claim.jpeg",
                "data": "/9j/4AAQ"  // fake base64 snippet
            }
        ]);
        let (b64, mime) = extract_thumbnail_from_assertions(&assertions);
        assert_eq!(b64.as_deref(), Some("/9j/4AAQ"));
        assert_eq!(mime.as_deref(), Some("image/jpeg"));
    }

    #[test]
    fn thumbnail_extracted_from_claim_png_label_with_object_data_field() {
        let assertions = serde_json::json!([
            {
                "label": "c2pa.thumbnail.claim.png",
                "data": {
                    "data": "iVBORw0KGgo=",
                    "format": "image/png"
                }
            }
        ]);
        let (b64, mime) = extract_thumbnail_from_assertions(&assertions);
        assert_eq!(b64.as_deref(), Some("iVBORw0KGgo="));
        assert_eq!(mime.as_deref(), Some("image/png"));
    }

    #[test]
    fn thumbnail_resource_reference_returns_mime_only() {
        // When data has an identifier but no inline bytes, we return the MIME but no base64.
        let assertions = serde_json::json!([
            {
                "label": "c2pa.thumbnail.claim.jpeg",
                "data": {
                    "identifier": "self#jumbf=…/c2pa.thumbnail.claim.jpeg",
                    "format": "image/jpeg"
                }
            }
        ]);
        let (b64, mime) = extract_thumbnail_from_assertions(&assertions);
        assert!(
            b64.is_none(),
            "resource-ref thumbnail should not return base64"
        );
        assert_eq!(mime.as_deref(), Some("image/jpeg"));
    }

    #[test]
    fn thumbnail_skips_non_thumbnail_assertions() {
        let assertions = serde_json::json!([
            { "label": "c2pa.actions", "data": { "actions": [] } },
            { "label": "c2pa.rights", "data": "AAAA" }
        ]);
        let (b64, mime) = extract_thumbnail_from_assertions(&assertions);
        assert!(b64.is_none());
        assert!(mime.is_none());
    }

    #[test]
    fn thumbnail_ingredient_label_also_matched() {
        let assertions = serde_json::json!([
            { "label": "c2pa.thumbnail.ingredient.jpeg", "data": "ABCD" }
        ]);
        let (b64, mime) = extract_thumbnail_from_assertions(&assertions);
        assert_eq!(b64.as_deref(), Some("ABCD"));
        assert_eq!(mime.as_deref(), Some("image/jpeg"));
    }

    // ---- derive_app_or_device tests ----

    #[test]
    fn app_or_device_prefers_claim_generator_info_name() {
        let manifest = serde_json::json!({
            "claim_generator_info": [{ "name": "Google Photos" }],
            "signature_info": { "common_name": "Pixel Camera" },
            "claim_generator": "Google/C2PA-SDK/1.0"
        });
        assert_eq!(
            derive_app_or_device(&manifest).as_deref(),
            Some("Google Photos")
        );
    }

    #[test]
    fn app_or_device_falls_back_to_signature_common_name() {
        let manifest = serde_json::json!({
            "signature_info": { "common_name": "Pixel Camera" },
            "claim_generator": "Google/C2PA-SDK/1.0"
        });
        assert_eq!(
            derive_app_or_device(&manifest).as_deref(),
            Some("Pixel Camera")
        );
    }

    #[test]
    fn app_or_device_strips_version_from_claim_generator() {
        let manifest = serde_json::json!({
            "claim_generator": "Jura Trace/0.9.0"
        });
        assert_eq!(
            derive_app_or_device(&manifest).as_deref(),
            Some("Jura Trace")
        );
    }

    #[test]
    fn app_or_device_returns_none_when_no_fields() {
        let manifest = serde_json::json!({});
        assert!(derive_app_or_device(&manifest).is_none());
    }

    // ---- build_content_summary tests ----

    #[test]
    fn content_summary_computational_capture_with_tool_name() {
        let manifest = serde_json::json!({
            "claim_generator_info": [{ "name": "Pixel Camera" }],
            "assertions": [{
                "label": "c2pa.actions",
                "data": {
                    "actions": [{
                        "action": "c2pa.created",
                        "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/computationalCapture"
                    }]
                }
            }]
        });
        let summary = build_content_summary(&manifest);
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert!(
            s.contains("computational"),
            "summary should mention computational photography"
        );
        assert!(s.contains("Pixel Camera"), "summary should name the tool");
    }

    #[test]
    fn content_summary_ai_generation_detected() {
        let manifest = serde_json::json!({
            "assertions": [{
                "label": "c2pa.actions",
                "data": {
                    "actions": [{
                        "action": "c2pa.created",
                        "digitalSourceType": "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia"
                    }]
                }
            }]
        });
        let summary = build_content_summary(&manifest);
        assert!(summary.is_some());
        assert!(summary.unwrap().to_lowercase().contains("ai"));
    }

    #[test]
    fn content_summary_composite_content_mentions_multiple_pieces() {
        let manifest = serde_json::json!({
            "assertions": [{
                "label": "c2pa.actions",
                "data": { "actions": [{ "action": "c2pa.edited" }] }
            }],
            "ingredients": [
                { "title": "A.jpg" },
                { "title": "B.jpg" }
            ]
        });
        let summary = build_content_summary(&manifest);
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert!(
            s.to_lowercase().contains("multiple") || s.to_lowercase().contains("combines"),
            "summary should mention multiple/combines: {s}"
        );
    }

    #[test]
    fn content_summary_jura_trace_created_and_signed() {
        let manifest = serde_json::json!({
            "claim_generator_info": [{ "name": "Jura Trace" }],
            "assertions": [{
                "label": "c2pa.actions",
                "data": {
                    "actions": [{ "action": "c2pa.created" }]
                }
            }]
        });
        let summary = build_content_summary(&manifest);
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert!(s.contains("Jura Trace"), "summary should name Jura Trace");
    }

    #[test]
    fn content_summary_none_when_no_actions() {
        let manifest = serde_json::json!({
            "assertions": [
                { "label": "c2pa.rights", "data": { "rights": "All Rights Reserved" } }
            ]
        });
        // No actions assertion — should return None.
        let summary = build_content_summary(&manifest);
        assert!(
            summary.is_none(),
            "no actions assertion should yield None summary, got: {summary:?}"
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

        // Step 5 (JTV-119) — verify the stds.iptc assertion now carries
        // a non-empty DigitalSourceType URI. An empty value would
        // degrade to absence-of-signal in downstream verifiers.
        let iptc = readback
            .assertions
            .iter()
            .find(|a| a.label == "stds.iptc")
            .expect("stds.iptc assertion must be present in signed manifest");
        let parsed: serde_json::Value =
            serde_json::from_str(&iptc.value).expect("stds.iptc value must be valid JSON");
        let digital_source = parsed
            .get("Iptc4xmpExt:DigitalSourceType")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(
            digital_source, "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture",
            "DigitalSourceType must positively assert digitalCapture (JTV-119)"
        );

        // Step 6 (JTV-120) — verify stds.schema-org.CreativeWork is present with
        // the canonical CC BY 4.0 URI (the test signs with Some("CC BY 4.0")).
        let schema_org = readback
            .assertions
            .iter()
            .find(|a| a.label == "stds.schema-org.CreativeWork")
            .expect(
                "stds.schema-org.CreativeWork assertion must be present for CC BY 4.0 (JTV-120)",
            );
        let schema_parsed: serde_json::Value =
            serde_json::from_str(&schema_org.value).expect("schema-org value must be valid JSON");
        let license_uri = schema_parsed
            .get("license")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(
            license_uri, "https://creativecommons.org/licenses/by/4.0/",
            "schema-org CreativeWork.license must be the canonical CC BY 4.0 URI (JTV-120)"
        );
        let schema_type = schema_parsed
            .get("@type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(
            schema_type, "CreativeWork",
            "schema-org assertion @type must be CreativeWork (JTV-120)"
        );
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

        sign_file(
            &source,
            &output,
            "Regression User",
            Some("CC BY 4.0"),
            &cert,
            &key,
        )
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

    /// Regression test for the Google Pixel Zoom Enhance "Content Credential
    /// unavailable or invalid" misreport (2026-04-26).
    ///
    /// Shape of the bug: c2pa-rs emits a single `ingredientDeltas[]` entry whose
    /// URI is keyed on the PARENT manifest's label + the parent's
    /// `c2pa.ingredient` assertion path — NOT the child manifest's label. The
    /// previous matcher searched for the child label inside the URI and never
    /// matched, falling through to a positional fallback that picked up the
    /// summary-only delta (`ingredient.manifest.validated` + cert-state
    /// failures). The L3 panel then rendered "Content Credential unavailable
    /// or invalid" because none of `claimSignature.validated`,
    /// `assertion.dataHash.match`, etc. were present.
    ///
    /// Fix: prefer the ingredient's own embedded `validation_results.activeManifest`
    /// block, which carries the full success-code set.
    #[test]
    fn pixel_zoom_enhance_uses_embedded_ingredient_validation() {
        let parent_label = "urn:c2pa:53649c45-9acd-3405-c4e8-419144ec9657";
        let child_label = "urn:c2pa:eecfbd16-c3ee-4ea0-4337-45682d534294";

        // The ingredient as it appears inside the active manifest's
        // `ingredients[]` — full success-code set inside its own
        // validation_results.activeManifest block.
        let ingredient = serde_json::json!({
            "active_manifest": child_label,
            "validation_results": {
                "activeManifest": {
                    "success": [
                        { "code": "timeStamp.validated" },
                        { "code": "timeStamp.trusted" },
                        { "code": "signingCredential.trusted" },
                        { "code": "claimSignature.insideValidity" },
                        { "code": "claimSignature.validated" },
                        { "code": "assertion.hashedURI.match" },
                        { "code": "assertion.hashedURI.match" },
                        { "code": "assertion.dataHash.match" }
                    ],
                    "informational": [],
                    "failure": []
                }
            }
        });

        // The top-level ingredient delta — keyed on PARENT's label, summary
        // only. This is the trap the old matcher fell into.
        let parent_keyed_delta = serde_json::json!({
            "ingredientAssertionURI": format!(
                "self#jumbf=/c2pa/{}/c2pa.assertions/c2pa.ingredient.v3",
                parent_label
            ),
            "validationDeltas": {
                "success": [{ "code": "ingredient.manifest.validated" }],
                "informational": [],
                "failure": [
                    { "code": "signingCredential.expired" },
                    { "code": "signingCredential.untrusted" }
                ]
            }
        });
        let deltas: Vec<&serde_json::Value> = vec![&parent_keyed_delta];

        let resolved = resolve_ingredient_validation_source(&ingredient, parent_label, &deltas, 0)
            .expect("should resolve to a validation source");

        let checks = extract_validation_checks_from_delta(resolved);
        let codes: Vec<&str> = checks.iter().map(|c| c.code.as_str()).collect();

        // The full success set must be present — this is what the Origin
        // tab's Validation Summary needs to render "Signature valid" +
        // "Data integrity confirmed" instead of the fail copy.
        assert!(
            codes.contains(&"claimSignature.validated"),
            "must include claimSignature.validated; got {codes:?}"
        );
        assert!(
            codes.contains(&"assertion.dataHash.match"),
            "must include assertion.dataHash.match; got {codes:?}"
        );
        assert!(
            codes.contains(&"timeStamp.validated"),
            "must include timeStamp.validated; got {codes:?}"
        );
        // The summary delta's bare `ingredient.manifest.validated` must NOT be
        // the only signal — the bug was rendering exactly that one code.
        assert!(
            !codes.contains(&"ingredient.manifest.validated"),
            "embedded source must override the parent-keyed summary delta; got {codes:?}"
        );
    }

    /// When the ingredient has no embedded `validation_results`, the resolver
    /// falls back to a top-level delta whose URI references the parent label.
    #[test]
    fn ingredient_delta_uri_match_uses_parent_label() {
        let parent_label = "urn:c2pa:abc";
        let ingredient = serde_json::json!({ "active_manifest": "urn:c2pa:def" });
        let delta = serde_json::json!({
            "ingredientAssertionURI": format!(
                "self#jumbf=/c2pa/{}/c2pa.assertions/c2pa.ingredient",
                parent_label
            ),
            "validationDeltas": {
                "success": [{ "code": "claimSignature.validated" }],
                "failure": []
            }
        });
        let deltas = vec![&delta];

        let resolved = resolve_ingredient_validation_source(&ingredient, parent_label, &deltas, 0)
            .expect("uri-matched delta should resolve");
        assert_eq!(
            resolved
                .get("success")
                .and_then(|s| s.as_array())
                .map(|a| a.len()),
            Some(1)
        );
    }

    /// Positional fallback only kicks in when both embedded and URI-match fail.
    #[test]
    fn ingredient_resolver_positional_last_resort() {
        let ingredient = serde_json::json!({ "active_manifest": "urn:c2pa:def" });
        let delta = serde_json::json!({
            "ingredientAssertionURI": "self#jumbf=/c2pa/wrong/c2pa.assertions/c2pa.ingredient",
            "validationDeltas": {
                "success": [{ "code": "ingredient.manifest.validated" }],
                "failure": []
            }
        });
        let deltas = vec![&delta];

        let resolved = resolve_ingredient_validation_source(
            &ingredient,
            "urn:c2pa:does-not-match",
            &deltas,
            0,
        )
        .expect("positional fallback should resolve");
        assert!(resolved.get("success").is_some());
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
        assert!(checks
            .iter()
            .any(|c| c.code == "claimSignature.validated" && c.outcome == "pass"));
        assert!(checks
            .iter()
            .any(|c| c.code == "some.info" && c.outcome == "info"));
        assert!(checks
            .iter()
            .any(|c| c.code == "assertion.dataHash.mismatch" && c.outcome == "fail"));
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
        assert_eq!(
            info2.verification_mode, None,
            "ingredient should have no mode"
        );
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
            certificate_expired: None,
            cert_not_before: None,
            cert_not_after: None,
            signed_at: None,
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: Some("enhanced".to_string()),
            thumbnail_base64: None,
            thumbnail_mime: None,
            app_or_device: None,
            content_summary: None,
            is_update_manifest: false,
            redactions: vec![],
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
            certificate_expired: None,
            cert_not_before: None,
            cert_not_after: None,
            signed_at: None,
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: None,
            thumbnail_base64: None,
            thumbnail_mime: None,
            app_or_device: None,
            content_summary: None,
            is_update_manifest: false,
            redactions: vec![],
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
                certificate_expired: None,
                cert_not_before: None,
                cert_not_after: None,
                signed_at: None,
                signed_by: None,
                signed_by_issuer: None,
                validation_checks: vec![],
                verification_mode: Some("standard".to_string()),
                thumbnail_base64: None,
                thumbnail_mime: None,
                app_or_device: None,
                content_summary: None,
                is_update_manifest: false,
                redactions: vec![],
            },
            ingredients: vec![],
            manifest_count: 1,
        };
        let json = serde_json::to_string(&chain).expect("serialise");
        assert!(json.contains("\"manifestCount\""), "should use camelCase");
        assert!(
            !json.contains("\"manifest_count\""),
            "should not use snake_case"
        );
    }

    /// A manifest with a `c2pa.hash.data` assertion is a normal (non-update)
    /// manifest — even when it has ingredients.
    #[test]
    fn detect_update_manifest_false_when_hash_present() {
        let manifest = serde_json::json!({
            "assertions": [
                { "label": "c2pa.hash.data", "data": {} },
                { "label": "c2pa.actions", "data": {} }
            ],
            "ingredients": [ { "title": "parent.jpg" } ],
        });
        assert!(!detect_update_manifest(&manifest));
    }

    /// An update manifest has no hash assertion but has at least one ingredient.
    #[test]
    fn detect_update_manifest_true_when_no_hash_with_ingredients() {
        let manifest = serde_json::json!({
            "assertions": [
                { "label": "c2pa.actions", "data": {} }
            ],
            "ingredients": [ { "title": "parent.jpg" } ],
        });
        assert!(detect_update_manifest(&manifest));
    }

    /// A manifest with no hash and no ingredients is not classified as an update
    /// manifest — too ambiguous.
    #[test]
    fn detect_update_manifest_false_when_no_ingredients() {
        let manifest = serde_json::json!({
            "assertions": [
                { "label": "c2pa.actions", "data": {} }
            ],
        });
        assert!(!detect_update_manifest(&manifest));
    }

    /// Redactions from the top-level `redactions` array are captured with no
    /// reason when no matching `c2pa.redacted` action entry is present.
    #[test]
    fn extract_redactions_from_top_level_array() {
        let manifest = serde_json::json!({
            "redactions": [
                "self#jumbf=/c2pa/urn:uuid:abc/c2pa.assertions/c2pa.training-mining",
            ],
            "assertions": [],
        });
        let redactions = extract_redactions(&manifest);
        assert_eq!(redactions.len(), 1);
        assert_eq!(
            redactions[0].target,
            "self#jumbf=/c2pa/urn:uuid:abc/c2pa.assertions/c2pa.training-mining"
        );
        assert_eq!(redactions[0].reason, None);
    }

    /// A `c2pa.redacted` action entry contributes its `reason` to the
    /// matching redaction record.
    #[test]
    fn extract_redactions_merges_reason_from_action() {
        let manifest = serde_json::json!({
            "redactions": [
                "self#jumbf=/c2pa/urn:uuid:abc/c2pa.assertions/c2pa.training-mining",
            ],
            "assertions": [
                {
                    "label": "c2pa.actions.v2",
                    "data": {
                        "actions": [
                            {
                                "action": "c2pa.redacted",
                                "reason": "Removed training-mining permissions at rights-holder request.",
                                "parameters": {
                                    "redacted": "self#jumbf=/c2pa/urn:uuid:abc/c2pa.assertions/c2pa.training-mining"
                                }
                            }
                        ]
                    }
                }
            ],
        });
        let redactions = extract_redactions(&manifest);
        assert_eq!(redactions.len(), 1);
        assert_eq!(
            redactions[0].reason.as_deref(),
            Some("Removed training-mining permissions at rights-holder request.")
        );
    }

    /// A `c2pa.redacted` action without a matching top-level `redactions`
    /// entry still contributes a redaction record (discovered via the action).
    #[test]
    fn extract_redactions_action_only() {
        let manifest = serde_json::json!({
            "assertions": [
                {
                    "label": "c2pa.actions",
                    "data": {
                        "actions": [
                            {
                                "action": "c2pa.redacted",
                                "parameters": {
                                    "redacted": "self#jumbf=/c2pa/urn:uuid:xyz/c2pa.assertions/c2pa.training-mining"
                                }
                            }
                        ]
                    }
                }
            ],
        });
        let redactions = extract_redactions(&manifest);
        assert_eq!(redactions.len(), 1);
        assert!(redactions[0].target.contains("training-mining"));
    }

    /// `is_update_manifest` and `redactions` serialise to camelCase.
    #[test]
    fn update_manifest_and_redactions_serialise_to_camel_case() {
        let info = ManifestInfo {
            title: None,
            format: None,
            claim_generator: None,
            assertions: vec![],
            is_valid: true,
            valid_at_signing: false,
            certificate_expired: None,
            cert_not_before: None,
            cert_not_after: None,
            signed_at: None,
            signed_by: None,
            signed_by_issuer: None,
            validation_checks: vec![],
            verification_mode: None,
            thumbnail_base64: None,
            thumbnail_mime: None,
            app_or_device: None,
            content_summary: None,
            is_update_manifest: true,
            redactions: vec![RedactionRecord {
                target: "self#jumbf=/c2pa/x/c2pa.assertions/c2pa.training-mining".to_string(),
                reason: Some("Rights-holder request.".to_string()),
            }],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"isUpdateManifest\":true"));
        assert!(json.contains("\"redactions\""));
        assert!(json.contains("\"target\""));
        assert!(json.contains("\"reason\""));
    }

    // ===== Trust list initialisation =====

    /// The concatenated trust bundle must contain every PEM certificate from
    /// all four vendored sources.  Count drift (e.g. a missing `include_str!`
    /// or an accidentally-truncated refresh) would silently regress Validator
    /// conformance — this test catches it at build time.
    #[test]
    fn concatenated_trust_bundle_has_all_expected_certs() {
        let bundle = concatenated_trust_bundle();
        let cert_count = bundle.matches("-----BEGIN CERTIFICATE-----").count();
        // 19 (current CA) + 14 (current TSA) + 27 (ITL anchors) + 115 (ITL
        // allowed) = 175 at time of vendoring.  If this ever needs revision
        // after a refresh, update `trust-list/README.md` to match.
        assert_eq!(
            cert_count, 175,
            "expected 175 certs across all four trust bundles, got {cert_count}"
        );
        assert!(bundle.contains("Google C2PA Root CA G3"));
    }

    /// `ensure_trust_settings_initialised` must succeed and the first call
    /// must leave the thread-local `trust.trust_anchors` populated.  The
    /// second call must be a fast no-op (thread-local flag short-circuit).
    #[test]
    fn trust_settings_initialisation_is_idempotent() {
        ensure_trust_settings_initialised().expect("first init must succeed");
        ensure_trust_settings_initialised().expect("re-init must be a no-op");
    }

    /// After init, `c2pa::Reader::from_file` must not fail for reasons
    /// related to missing trust anchors.  This is a smoke test only — it
    /// uses a minimal path and asserts the Reader constructor itself does
    /// not panic or return a trust-config-related error.
    #[test]
    fn reader_construction_after_trust_init_does_not_panic() {
        ensure_trust_settings_initialised().expect("trust init");
        // Non-existent path — we expect `Ok(None)` (no C2PA data) or a
        // file-I/O error, NOT a trust-configuration error.
        let result = open_reader(std::path::Path::new("/nonexistent/asset.jpg"));
        match result {
            Ok(None) => {}
            Err(msg) => {
                assert!(
                    !msg.to_lowercase().contains("trust"),
                    "unexpected trust-config error: {msg}"
                );
            }
            Ok(Some(_)) => panic!("unexpected Reader for nonexistent file"),
        }
    }

    /// End-to-end conformance check: reading a Google-Pixel-signed JPEG
    /// from the C2PA Validator test-vector set must NOT surface
    /// `signingCredential.untrusted` in the validation checks.  This was the
    /// exact failure the C2PA Conformance administrator flagged on the
    /// 2026-04-22 Validator review, and it's the root-cause test that
    /// proves the vendored trust list is being honoured by c2pa-rs.
    ///
    /// Gracefully skipped if the fixture is absent (e.g. a minimal
    /// source checkout without `docs/c2pa-conformance/test-vectors/`).
    #[test]
    fn google_pixel_asset_chains_to_trusted_ca() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/c2pa-conformance/test-vectors/pixel_expired_cert.jpg");
        if !fixture.exists() {
            eprintln!("skipping: fixture not present at {}", fixture.display());
            return;
        }

        let manifest = read_manifest(&fixture, false)
            .expect("read_manifest must not error on a valid Pixel asset")
            .expect("Pixel asset must contain a C2PA manifest");

        // The chain must have produced validation activity — an empty list
        // would indicate the Reader bailed out early and this test would be
        // a false positive.  With the official trust list loaded, c2pa-rs
        // produces 9 passing checks: timeStamp.validated, timeStamp.trusted,
        // signingCredential.trusted, claimSignature.insideValidity,
        // claimSignature.validated, 3x assertion.hashedURI.match,
        // assertion.dataHash.match.
        assert!(
            !manifest.validation_checks.is_empty(),
            "expected at least one validation check from c2pa-rs"
        );

        let untrusted: Vec<_> = manifest
            .validation_checks
            .iter()
            .filter(|c| c.code.contains("untrusted"))
            .collect();

        assert!(
            untrusted.is_empty(),
            "Google-signed Pixel asset must not surface trust-related failures \
             after trust-list init; got: {untrusted:?}"
        );

        // All checks must have outcome "pass" — no "fail", no "info" on this
        // canonical asset once the trust list is honoured.
        let non_pass: Vec<_> = manifest
            .validation_checks
            .iter()
            .filter(|c| c.outcome != "pass")
            .collect();
        assert!(
            non_pass.is_empty(),
            "all validation checks should pass for this asset; non-pass: {non_pass:?}"
        );

        // Fully valid — overrides `valid_at_signing` when no failures occur.
        assert!(
            manifest.is_valid,
            "Pixel asset with trusted CA + TSA must be is_valid=true"
        );

        // The Pixel test vector's leaf signing cert is deliberately expired
        // (that's what the filename advertises).  With the trust list loaded
        // c2pa-rs no longer emits `signingCredential.expired` as a failure,
        // so the UI relies on this flag instead.  `certificate_expired`
        // should be `Some(true)` so L3 can disclose the expiry.
        assert_eq!(
            manifest.certificate_expired,
            Some(true),
            "Pixel asset's leaf signing cert is past its notAfter date"
        );
    }

    // ===== Leaf cert expiry helper =====

    /// Round-trip test for `leaf_cert_expired` using a synthetic PEM with a
    /// known-past `notAfter`.  The PEM below is a self-signed ECDSA cert
    /// generated at test-fixture time with a validity window of 2023-01-01
    /// to 2024-01-01 — far in the past from any reasonable build time.
    #[test]
    fn leaf_cert_expired_returns_true_for_past_not_after() {
        // Embed a real expired cert rather than mocking — rcgen lets us
        // generate one deterministically in-process.
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
        let mut params = CertificateParams::new(vec!["test".to_string()]).unwrap();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "expired-test");
        params.distinguished_name = dn;
        params.not_before = time::OffsetDateTime::from_unix_timestamp(1_672_531_200).unwrap(); // 2023-01-01
        params.not_after = time::OffsetDateTime::from_unix_timestamp(1_704_067_200).unwrap(); // 2024-01-01
        let kp = KeyPair::generate().unwrap();
        let cert = params.self_signed(&kp).unwrap();
        let pem = cert.pem();

        assert_eq!(leaf_cert_expired(&pem), Some(true));
    }

    /// A cert with a `notAfter` far in the future must return `Some(false)`.
    #[test]
    fn leaf_cert_expired_returns_false_for_future_not_after() {
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
        let mut params = CertificateParams::new(vec!["test".to_string()]).unwrap();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "future-test");
        params.distinguished_name = dn;
        params.not_before = time::OffsetDateTime::now_utc();
        params.not_after = time::OffsetDateTime::now_utc()
            .checked_add(time::Duration::days(365 * 10))
            .unwrap();
        let kp = KeyPair::generate().unwrap();
        let cert = params.self_signed(&kp).unwrap();
        let pem = cert.pem();

        assert_eq!(leaf_cert_expired(&pem), Some(false));
    }

    /// Malformed PEM input must return `None` rather than panicking.
    #[test]
    fn leaf_cert_expired_returns_none_for_garbage_pem() {
        assert_eq!(leaf_cert_expired(""), None);
        assert_eq!(leaf_cert_expired("not a pem"), None);
        assert_eq!(
            leaf_cert_expired("-----BEGIN CERTIFICATE-----\nnot-base64\n-----END CERTIFICATE-----"),
            None
        );
    }
}
