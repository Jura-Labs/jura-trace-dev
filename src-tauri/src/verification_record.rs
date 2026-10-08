// SPDX-License-Identifier: AGPL-3.0-or-later

//! Verification Record Export (v1.2.0 plan, item B3).
//!
//! A signed, tamper-evident record of the checks that were run on one file:
//! what was examined, which detectors ran and what each one reported. It is
//! made so that somebody other than the person who ran the check can confirm
//! it has not been altered, using a tool that is not Jura Trace.
//!
//! The format is deliberately plain. `record.json` is the record, and its
//! bytes are what is signed. `record.json.sig` is a detached ECDSA P-256
//! signature over those bytes with SHA-256, in ASN.1 DER, which is exactly
//! what `openssl dgst -sha256 -verify` checks. `signer-cert.pem` is the
//! certificate chain the signing key belongs to. Nothing is canonicalised and
//! nothing is wrapped, so there is no parser to trust between the signature
//! and the bytes. The procedure a third party follows is `docs/VERIFICATION_RECORD.md`,
//! and [`instructions`] writes the same steps into every export.
//!
//! What the signature does and does not say is part of the record, because a
//! signature is easy to over-read. This release signs with the Sovereign
//! key, which is generated on the install and vouched for by nobody. The
//! signature therefore shows the record is unchanged since that install
//! signed it. It does not show who ran the check, and the time in the record
//! is the clock of the machine that made it.
//!
//! This module does not depend on Tauri, so the headless binary can use it.

use std::path::Path;

use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::c2pa;

pub const FORMAT: &str = "jura-trace-verification-record";
pub const FORMAT_VERSION: u32 = 1;

pub const RECORD_FILE: &str = "record.json";
pub const SIGNATURE_FILE: &str = "record.json.sig";
pub const CERTIFICATE_FILE: &str = "signer-cert.pem";
pub const INSTRUCTIONS_FILE: &str = "HOW-TO-CHECK.txt";

/// The wording decided on 4 September 2026 (`docs/release/v1.1.0-plan.md`,
/// "Export copy"). The Article 50 framing was rejected, so nothing here names
/// a statute or suggests the record discharges a duty.
pub const NOTICE: &str = "This is a signed, tamper-evident record of the checks Jura Trace ran on one file. \
An organisation may find it useful as supporting evidence. It does not determine, certify, or guarantee \
compliance with any law or standard, and it is not a finding that the file is authentic or inauthentic.";

const SIGNING_MODE: &str = "sovereign";
const SIGNING_MODE_DESCRIPTION: &str = "Sovereign mode: signed with a key generated on the computer that made this record. \
No certificate authority vouches for that key. The signature shows the record has not changed since it was signed. \
It does not show who signed it.";
const SIGNING_ALGORITHM: &str = "ECDSA P-256 with SHA-256, detached, ASN.1 DER";

/// A string longer than this is not a finding, it is an embedded image or a
/// dump. It is replaced by its length and hash, which keeps the record
/// readable and still binds the record to the value that was left out.
const LONG_VALUE_CHARS: usize = 8192;

/// What a caller needs to write the export: four files and two digests.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRecordExport {
    /// The exact text that was signed. Written out byte for byte as
    /// `record.json`; re-serialising it would break the signature.
    pub record_json: String,
    /// ASN.1 DER ECDSA signature over `record_json`'s UTF-8 bytes.
    pub signature_der: Vec<u8>,
    /// End-entity certificate, then the per-install CA certificate.
    pub certificate_pem: String,
    /// `HOW-TO-CHECK.txt`, for the person the export is handed to.
    pub instructions: String,
    /// SHA-256 of `record_json`, lowercase hex.
    pub record_sha256: String,
    /// SHA-256 of the signing certificate, as `XX:XX:...`.
    pub certificate_sha256: String,
}

/// Build, sign and self-check a Verification Record for one result.
///
/// `result` is the verification result exactly as the pipeline produced it
/// (the camelCase JSON of `VerificationResult`). It must carry the SHA-256 of
/// the file that was examined: a record that cannot be tied to a file is a
/// record of nothing, so that is an error and not a blank field.
pub fn export(
    data_dir: &Path,
    result: &Value,
    file_name: Option<&str>,
    created_at_utc: &str,
) -> Result<VerificationRecordExport, String> {
    let (cert_pem, key_pem) = c2pa::ensure_certificate(data_dir)?;
    let certificate_sha256 = c2pa::signing_cert_fingerprint_hex(data_dir)?;
    let certificate_pem = String::from_utf8(cert_pem)
        .map_err(|_| "Certificate PEM is not valid UTF-8".to_string())?;

    let record_json = build_record(result, file_name, &certificate_sha256, created_at_utc)?;
    let signature_der = sign(record_json.as_bytes(), &key_pem)?;

    // Checked here against the certificate that goes into the export, not
    // assumed. A key and a certificate that do not belong together would
    // otherwise produce an export nobody can verify, found by the recipient.
    verify(record_json.as_bytes(), &signature_der, &certificate_pem).map_err(|e| {
        format!("The record was signed but the signature does not check against this install's certificate: {e}")
    })?;

    let file_sha256 = result
        .get("inputSha256")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(VerificationRecordExport {
        record_sha256: hex(&Sha256::digest(record_json.as_bytes())),
        instructions: instructions(file_name, file_sha256, &certificate_sha256),
        record_json,
        signature_der,
        certificate_pem,
        certificate_sha256,
    })
}

/// Assemble the record's JSON text. Pure, so the shape is testable without a
/// key.
pub fn build_record(
    result: &Value,
    file_name: Option<&str>,
    certificate_sha256: &str,
    created_at_utc: &str,
) -> Result<String, String> {
    let obj = result
        .as_object()
        .ok_or("The verification result is not a JSON object")?;
    let file_sha256 = obj
        .get("inputSha256")
        .and_then(Value::as_str)
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(
            "This result carries no SHA-256 of the file that was examined, so a record of it could not be tied to a file",
        )?;
    if !obj.get("overallTrust").is_some_and(Value::is_number) {
        return Err("The verification result carries no trust score".into());
    }
    let detectors_run = obj
        .get("detectorsRun")
        .and_then(Value::as_array)
        .ok_or("The verification result does not say which detectors ran")?;

    let record = json!({
        "format": FORMAT,
        "formatVersion": FORMAT_VERSION,
        "notice": NOTICE,
        "createdAtUtc": created_at_utc,
        "createdAtNote": "The clock of the computer that made this record. It is not an independent timestamp.",
        "generator": {
            "name": "Jura Trace",
            "version": env!("CARGO_PKG_VERSION"),
            "platform": format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        },
        "signing": {
            "mode": SIGNING_MODE,
            "modeDescription": SIGNING_MODE_DESCRIPTION,
            "algorithm": SIGNING_ALGORITHM,
            "signatureFile": SIGNATURE_FILE,
            "certificateFile": CERTIFICATE_FILE,
            "certificateSha256": certificate_sha256,
        },
        "subject": {
            "fileName": file_name,
            "sha256": file_sha256,
            "contentType": obj.get("contentType"),
            "sourceType": obj.get("sourceType"),
        },
        "summary": {
            "mode": obj.get("mode"),
            "overallTrust": obj.get("overallTrust"),
            "verdict": obj.get("verdict"),
            "detectorsRun": detectors_run,
            "provenance": obj.get("provenance"),
        },
        "result": scrub(result),
    });

    let mut text = serde_json::to_string_pretty(&record)
        .map_err(|e| format!("Failed to serialise the record: {e}"))?;
    text.push('\n');
    Ok(text)
}

/// Remove what does not belong in a document handed to somebody else.
///
/// Two things. Absolute local paths, which name the user's account and
/// directory layout and say nothing about the file. And very long strings,
/// which are embedded images, replaced by their length and SHA-256.
fn scrub(value: &Value) -> Value {
    match value {
        Value::String(s) if is_local_path(s) => Value::String("[local path left out]".into()),
        Value::String(s) if s.chars().count() > LONG_VALUE_CHARS => json!({
            "leftOut": "A long value, such as an embedded image, is not carried in the record.",
            "characters": s.chars().count(),
            "sha256": hex(&Sha256::digest(s.as_bytes())),
        }),
        Value::Array(items) => Value::Array(items.iter().map(scrub).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), scrub(v)))
                .collect::<Map<String, Value>>(),
        ),
        other => other.clone(),
    }
}

fn is_local_path(s: &str) -> bool {
    let b = s.as_bytes();
    let unix = s.starts_with("/Users/")
        || s.starts_with("/home/")
        || s.starts_with("/var/")
        || s.starts_with("/tmp/")
        || s.starts_with("/private/")
        || s.starts_with("/Volumes/")
        || s.starts_with("/mnt/");
    let windows = b.len() > 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b[2] == b'\\' || b[2] == b'/');
    unix || windows
        || s.starts_with("\\\\")
        || s.starts_with("file://")
        || s.starts_with("asset://")
}

/// Sign `message` with a PKCS#8 ECDSA P-256 private key in PEM.
pub fn sign(message: &[u8], key_pem: &[u8]) -> Result<Vec<u8>, String> {
    let (_, pem) = x509_parser::pem::parse_x509_pem(key_pem)
        .map_err(|e| format!("The signing key is not readable PEM: {e}"))?;
    if pem.label != "PRIVATE KEY" {
        return Err(format!(
            "The signing key is a `{}` PEM block, and a PKCS#8 `PRIVATE KEY` is needed",
            pem.label
        ));
    }
    let rng = ring::rand::SystemRandom::new();
    let key = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
        &pem.contents,
        &rng,
    )
    .map_err(|e| format!("The signing key is not an ECDSA P-256 key: {e}"))?;
    let signature = key
        .sign(&rng, message)
        .map_err(|_| "Signing the record failed".to_string())?;
    Ok(signature.as_ref().to_vec())
}

/// Check a detached signature against the first certificate in `certificate_pem`.
///
/// This is the in-process check. The one that matters to a recipient is the
/// `openssl` procedure, and a test below runs that too.
pub fn verify(message: &[u8], signature_der: &[u8], certificate_pem: &str) -> Result<(), String> {
    let (_, pem) = x509_parser::pem::parse_x509_pem(certificate_pem.as_bytes())
        .map_err(|e| format!("The certificate is not readable PEM: {e}"))?;
    let cert = pem
        .parse_x509()
        .map_err(|e| format!("The certificate does not parse: {e}"))?;
    let point = &cert.tbs_certificate.subject_pki.subject_public_key.data;
    ring::signature::UnparsedPublicKey::new(
        &ring::signature::ECDSA_P256_SHA256_ASN1,
        point.as_ref(),
    )
    .verify(message, signature_der)
    .map_err(|_| "the signature does not match the record".to_string())
}

/// The steps a recipient follows, written into every export.
pub fn instructions(
    file_name: Option<&str>,
    file_sha256: &str,
    certificate_sha256: &str,
) -> String {
    let name = file_name.unwrap_or("the file");
    format!(
        "Jura Trace Verification Record: how to check it\n\
         ===============================================\n\
         \n\
         {NOTICE}\n\
         \n\
         What is in this folder\n\
         ----------------------\n\
         {RECORD_FILE}       The record. What was examined and what each check reported.\n\
         {SIGNATURE_FILE}   A signature over the exact bytes of {RECORD_FILE}.\n\
         {CERTIFICATE_FILE}   The certificate of the key that made the signature.\n\
         {INSTRUCTIONS_FILE}  This file. It is not signed.\n\
         \n\
         1. Check the record has not been altered\n\
         ----------------------------------------\n\
         You need OpenSSL, which is not made by Jura Labs. In this folder, run:\n\
         \n\
         \x20   openssl x509 -in {CERTIFICATE_FILE} -pubkey -noout > signer-pubkey.pem\n\
         \x20   openssl dgst -sha256 -verify signer-pubkey.pem -signature {SIGNATURE_FILE} {RECORD_FILE}\n\
         \n\
         The second command prints \"Verified OK\" if {RECORD_FILE} is byte for byte\n\
         what was signed. Anything else, including \"Verification failure\", means the\n\
         record or the signature has been changed and the record should not be relied on.\n\
         Opening {RECORD_FILE} in an editor and saving it can change its bytes.\n\
         \n\
         2. Check the record is about your file\n\
         --------------------------------------\n\
         The record says it examined {name}, with this SHA-256:\n\
         \n\
         \x20   {file_sha256}\n\
         \n\
         Compute the SHA-256 of the file you were given and compare:\n\
         \n\
         \x20   openssl dgst -sha256 <file>\n\
         \n\
         If the two differ, the record describes a different file.\n\
         \n\
         3. Check which key signed it\n\
         ----------------------------\n\
         The SHA-256 fingerprint of the signing certificate is:\n\
         \n\
         \x20   {certificate_sha256}\n\
         \n\
         \x20   openssl x509 -in {CERTIFICATE_FILE} -noout -fingerprint -sha256\n\
         \n\
         The person who made the record can read the same fingerprint out of their copy\n\
         of Jura Trace, on the Protect page before signing. If you need to know the record\n\
         came from them, ask them for it by a route you trust and compare.\n\
         \n\
         What a passing check means\n\
         --------------------------\n\
         It means the record is unchanged since it was signed by the key above.\n\
         \n\
         It does not mean the file is authentic. The record reports what automated checks\n\
         found, and those checks can be wrong in both directions.\n\
         \n\
         It does not prove who made the record. The key was generated on the computer that\n\
         made it and no certificate authority vouches for it (Sovereign mode).\n\
         \n\
         It does not prove when the record was made. The time inside it is that computer's\n\
         own clock.\n"
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    const FILE_SHA: &str = "15d3d9f5670097986c5cb5db2625befe502b32aed2c55fbaa47315514283539c";

    fn sample_result() -> Value {
        json!({
            "sourceType": "file",
            "contentType": "image/jpeg",
            "mode": "standard",
            "overallTrust": 0.55,
            "verdict": { "band": "uncertain" },
            "inputSha256": FILE_SHA,
            "detectorsRun": ["exif_anomaly", "ela", "noise"],
            "elaScore": 0.12,
            "elaResult": { "score": 0.12, "heatmapUrl": "/Users/someone/Library/heatmaps/ela.png" },
            "noiseResult": { "preview": "A".repeat(LONG_VALUE_CHARS + 1) },
            "provenance": { "engineVersion": "1.2.0" },
        })
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("jura-vr-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_export(dir: &Path) -> VerificationRecordExport {
        export(
            dir,
            &sample_result(),
            Some("photo.jpg"),
            "2026-10-08T12:00:00Z",
        )
        .unwrap()
    }

    #[test]
    fn record_says_what_was_examined_and_how_it_was_signed() {
        let text = build_record(
            &sample_result(),
            Some("photo.jpg"),
            "AA:BB",
            "2026-10-08T12:00:00Z",
        )
        .unwrap();
        let record: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(record["format"], FORMAT);
        assert_eq!(record["formatVersion"], FORMAT_VERSION);
        assert_eq!(record["notice"], NOTICE);
        assert_eq!(record["subject"]["sha256"], FILE_SHA);
        assert_eq!(record["subject"]["fileName"], "photo.jpg");
        assert_eq!(record["signing"]["mode"], "sovereign");
        assert_eq!(record["signing"]["certificateSha256"], "AA:BB");
        assert_eq!(
            record["summary"]["detectorsRun"],
            json!(["exif_anomaly", "ela", "noise"])
        );
        assert_eq!(record["result"]["elaScore"], 0.12);
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn record_carries_no_local_path_and_no_embedded_blob() {
        let text = build_record(&sample_result(), None, "AA", "2026-10-08T12:00:00Z").unwrap();
        assert!(!text.contains("/Users/someone"));
        assert!(!text.contains(&"A".repeat(200)));
        let record: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            record["result"]["elaResult"]["heatmapUrl"],
            "[local path left out]"
        );
        let left_out = &record["result"]["noiseResult"]["preview"];
        assert_eq!(left_out["characters"], LONG_VALUE_CHARS + 1);
        assert_eq!(left_out["sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn a_result_with_no_file_hash_is_refused() {
        let mut result = sample_result();
        result.as_object_mut().unwrap().remove("inputSha256");
        let err = build_record(&result, None, "AA", "2026-10-08T12:00:00Z").unwrap_err();
        assert!(err.contains("could not be tied to a file"), "{err}");

        result["inputSha256"] = json!("not-a-hash");
        assert!(build_record(&result, None, "AA", "2026-10-08T12:00:00Z").is_err());
    }

    #[test]
    fn signature_checks_and_one_changed_byte_fails() {
        let dir = temp_dir("inproc");
        let ex = sample_export(&dir);
        verify(
            ex.record_json.as_bytes(),
            &ex.signature_der,
            &ex.certificate_pem,
        )
        .unwrap();

        let mut tampered = ex.record_json.clone().into_bytes();
        let at = tampered.iter().position(|b| *b == b'5').unwrap();
        tampered[at] = b'9';
        assert!(verify(&tampered, &ex.signature_der, &ex.certificate_pem).is_err());

        let mut bad_sig = ex.signature_der.clone();
        let last = bad_sig.len() - 1;
        bad_sig[last] ^= 0x01;
        assert!(verify(ex.record_json.as_bytes(), &bad_sig, &ex.certificate_pem).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_record_signed_by_one_install_does_not_check_against_another() {
        let (a, b) = (temp_dir("a"), temp_dir("b"));
        let ex_a = sample_export(&a);
        let ex_b = sample_export(&b);
        assert!(verify(
            ex_a.record_json.as_bytes(),
            &ex_a.signature_der,
            &ex_b.certificate_pem
        )
        .is_err());
        let _ = std::fs::remove_dir_all(a);
        let _ = std::fs::remove_dir_all(b);
    }

    /// The release gate's item, automated: export, check the signature with a
    /// tool that is not ours, change one byte, and watch the check fail.
    ///
    /// The commands are the ones `instructions` gives a recipient. OpenSSL is
    /// required wherever this suite is expected to run, and a machine without
    /// it fails the test instead of skipping it, because a gate that skips
    /// quietly is the fault BL-SILENT-001 describes. Windows is the exception:
    /// no CI job runs this suite there and OpenSSL is not part of the system.
    #[test]
    fn openssl_verifies_the_export_and_rejects_a_tampered_one() {
        let have_openssl = Command::new("openssl").arg("version").output().is_ok();
        if !have_openssl {
            if cfg!(windows) {
                eprintln!(
                    "openssl is not on PATH; the independent check was NOT run on this machine"
                );
                return;
            }
            panic!("openssl is not on PATH, so the independent check of a Verification Record cannot run");
        }

        let dir = temp_dir("openssl");
        let ex = sample_export(&dir);
        let out = dir.join("export");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join(RECORD_FILE), ex.record_json.as_bytes()).unwrap();
        std::fs::write(out.join(SIGNATURE_FILE), &ex.signature_der).unwrap();
        std::fs::write(out.join(CERTIFICATE_FILE), ex.certificate_pem.as_bytes()).unwrap();

        let pubkey = Command::new("openssl")
            .current_dir(&out)
            .args(["x509", "-in", CERTIFICATE_FILE, "-pubkey", "-noout"])
            .output()
            .unwrap();
        assert!(
            pubkey.status.success(),
            "{}",
            String::from_utf8_lossy(&pubkey.stderr)
        );
        std::fs::write(out.join("signer-pubkey.pem"), &pubkey.stdout).unwrap();

        let check = || {
            Command::new("openssl")
                .current_dir(&out)
                .args([
                    "dgst",
                    "-sha256",
                    "-verify",
                    "signer-pubkey.pem",
                    "-signature",
                    SIGNATURE_FILE,
                    RECORD_FILE,
                ])
                .output()
                .unwrap()
        };

        let good = check();
        assert!(
            good.status.success() && String::from_utf8_lossy(&good.stdout).contains("Verified OK"),
            "openssl did not verify an untouched export: {} {}",
            String::from_utf8_lossy(&good.stdout),
            String::from_utf8_lossy(&good.stderr)
        );

        // One byte, in the trust score, which is the byte somebody would change.
        let mut bytes = std::fs::read(out.join(RECORD_FILE)).unwrap();
        let needle = b"\"overallTrust\": 0.55";
        let at = bytes
            .windows(needle.len())
            .position(|w| w == needle)
            .unwrap()
            + needle.len()
            - 1;
        bytes[at] = b'9';
        std::fs::write(out.join(RECORD_FILE), &bytes).unwrap();

        let bad = check();
        assert!(
            !bad.status.success() && !String::from_utf8_lossy(&bad.stdout).contains("Verified OK"),
            "openssl accepted a record with one byte changed"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn instructions_name_the_commands_and_the_limits() {
        let text = instructions(Some("photo.jpg"), FILE_SHA, "AA:BB");
        assert!(text.contains(
            "openssl dgst -sha256 -verify signer-pubkey.pem -signature record.json.sig record.json"
        ));
        assert!(text.contains(FILE_SHA));
        assert!(text.contains("AA:BB"));
        assert!(text.contains("It does not prove who made the record."));
        assert!(text.contains(NOTICE));
    }
}
