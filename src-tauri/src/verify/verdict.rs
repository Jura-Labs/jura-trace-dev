// SPDX-License-Identifier: AGPL-3.0-or-later

//! The trust band, computed once, in Rust (v1.2.0 B2a stage 2).
//!
//! Until v1.2.0 the band existed only in the frontend
//! (`ui/src/routes/verify/+page.svelte`, `trustLevel`), so the REST API, the
//! CLI and anything else reading a result had to re-implement it, and a
//! consumer that banded `overallTrust` alone would disagree with the screen
//! on exactly the files that matter most. This module is a port of that
//! frontend rule, behaviour for behaviour, and the frontend now reads the
//! field instead of deriving it.
//!
//! The rule is thresholds and caps set by people. No model output decides a
//! band except through the detector results the caps name.
//!
//! Design: `docs/design/v1.2.0-headless-api-and-cli.md` section 3.1.

use serde::{Deserialize, Serialize};

use crate::{c2pa, exif_anomaly, metadata, sidecar};

/// Scores at or above this are `trusted`, before any cap.
pub const BAND_TRUSTED_MIN: f64 = 0.7;
/// Scores at or above this, and below [`BAND_TRUSTED_MIN`], are `uncertain`.
pub const BAND_UNCERTAIN_MIN: f64 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Band {
    /// Shown as "High Trust" / "Authentic".
    Trusted,
    /// Shown as "Moderate Trust" / "Review".
    Uncertain,
    /// Shown as "Low Trust" / "Suspicious".
    Untrusted,
    /// The core image detectors did not run, so the score says little.
    /// Shown as "Inconclusive" / "Insufficient signal".
    Inconclusive,
}

/// Why the band is not simply the score's own band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ceiling {
    /// Image content where neither ELA nor the deepfake detector ran.
    InsufficientSignal,
    /// The score alone would be `trusted`, but nothing positive supports
    /// authenticity: no camera MakerNote, no valid Content Credentials, no
    /// recognised camera make and model with clean EXIF.
    NoPositiveAuthenticitySignal,
    /// The score alone would be `trusted`, but the deepfake detector
    /// returned `inconclusive`.
    DeepfakeInconclusive,
    /// The score alone would be `trusted`, but the deepfake detector
    /// returned `synthetic`.
    DeepfakeSynthetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandBoundaries {
    pub trusted: f64,
    pub uncertain: f64,
}

/// The `verdict` block on every [`super::types::VerificationResult`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub band: Band,
    /// The same number as `overallTrust`.
    pub score: f64,
    /// `null` when the band is the score's own band.
    pub ceiling_applied: Option<Ceiling>,
    pub band_boundaries: BandBoundaries,
}

/// The parts of a verification result the band depends on.
pub struct VerdictInputs<'a> {
    pub overall_trust: f64,
    pub detectors_run: &'a [&'a str],
    pub exif_analysis: Option<&'a exif_anomaly::ExifAnalysis>,
    pub image_metadata: Option<&'a metadata::ImageMetadata>,
    pub c2pa_valid: Option<bool>,
    pub c2pa_manifest: Option<&'a c2pa::ManifestInfo>,
    pub c2pa_chain: Option<&'a c2pa::ManifestChain>,
    pub deepfake_result: Option<&'a sidecar::DeepfakeResult>,
}

pub fn compute_verdict(i: &VerdictInputs<'_>) -> Verdict {
    let raw = if i.overall_trust >= BAND_TRUSTED_MIN {
        Band::Trusted
    } else if i.overall_trust >= BAND_UNCERTAIN_MIN {
        Band::Uncertain
    } else {
        Band::Untrusted
    };

    let (band, ceiling_applied) = if insufficient_signal(i) {
        (Band::Inconclusive, Some(Ceiling::InsufficientSignal))
    } else if raw == Band::Trusted && !has_positive_authenticity_signal(i) {
        (Band::Uncertain, Some(Ceiling::NoPositiveAuthenticitySignal))
    } else {
        let deepfake_verdict = i
            .deepfake_result
            .and_then(|d| d.verdict_level.as_deref())
            .unwrap_or("");
        match (raw, deepfake_verdict) {
            (Band::Trusted, "inconclusive") => {
                (Band::Uncertain, Some(Ceiling::DeepfakeInconclusive))
            }
            (Band::Trusted, "synthetic") => (Band::Uncertain, Some(Ceiling::DeepfakeSynthetic)),
            _ => (raw, None),
        }
    };

    Verdict {
        band,
        score: i.overall_trust,
        ceiling_applied,
        band_boundaries: BandBoundaries {
            trusted: BAND_TRUSTED_MIN,
            uncertain: BAND_UNCERTAIN_MIN,
        },
    }
}

/// Image content where neither `ela` nor `deepfake` ran: the sidecar was
/// unreachable, or the mode did not ask for them. Non-image content never
/// trips this, because those detectors are not expected for it.
fn insufficient_signal(i: &VerdictInputs<'_>) -> bool {
    let ran = |id: &str| i.detectors_run.contains(&id);
    if i.detectors_run.is_empty() {
        // The frontend fell back to counting result slots here and needed
        // four; a result with no detectors recorded has none.
        return true;
    }
    let is_image_content =
        i.exif_analysis.is_some() || ran("ela") || ran("deepfake") || ran("exif_anomaly");
    is_image_content && !ran("ela") && !ran("deepfake")
}

fn has_positive_authenticity_signal(i: &VerdictInputs<'_>) -> bool {
    // (a) A vendor MakerNote, which AI generators almost never synthesise.
    if i.exif_analysis
        .is_some_and(|e| e.camera_authenticity_bonus > 0.5)
    {
        return true;
    }
    // (b) Valid Content Credentials that carry no digitalSourceType at all.
    if i.c2pa_valid == Some(true) && !declares_digital_source_type(i) {
        return true;
    }
    // (c) A recognised camera make, a model, and no high-severity EXIF
    // finding (added 22 May 2026 for phone photos whose MakerNote was
    // stripped in sharing).
    if let (Some(exif), Some(meta)) = (i.exif_analysis, i.image_metadata) {
        let has_model = meta
            .camera_model
            .as_deref()
            .is_some_and(|m| !m.trim().is_empty());
        let has_severe_finding = exif.findings.iter().any(|f| {
            matches!(
                f.severity,
                exif_anomaly::Severity::High | exif_anomaly::Severity::Critical
            )
        });
        if exif.is_known_camera_make && has_model && !has_severe_finding {
            return true;
        }
    }
    false
}

/// True when any manifest in the chain (active first, then ingredients)
/// carries a `digitalSourceType`, on an assertion or on one of its actions.
///
/// This is any declared type, a camera capture as much as an AI one. That
/// is what the frontend rule did and this port keeps it; see the PR that
/// added this module for the note on it.
fn declares_digital_source_type(i: &VerdictInputs<'_>) -> bool {
    let manifests: Vec<&c2pa::ManifestInfo> = match i.c2pa_chain {
        Some(chain) => std::iter::once(&chain.active)
            .chain(chain.ingredients.iter())
            .collect(),
        None => i.c2pa_manifest.into_iter().collect(),
    };
    let non_empty_str =
        |v: Option<&serde_json::Value>| v.and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty());
    manifests
        .iter()
        .flat_map(|m| m.assertions.iter())
        .filter_map(|a| serde_json::from_str::<serde_json::Value>(&a.value).ok())
        .any(|parsed| {
            let direct = match parsed.get("digitalSourceType") {
                Some(v) if !v.is_null() => Some(v),
                _ => parsed
                    .get("schema_org")
                    .and_then(|s| s.get("digitalSourceType")),
            };
            non_empty_str(direct)
                || parsed
                    .get("actions")
                    .and_then(|a| a.as_array())
                    .is_some_and(|actions| {
                        actions
                            .iter()
                            .any(|action| non_empty_str(action.get("digitalSourceType")))
                    })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exif(
        bonus: f64,
        known_make: bool,
        severities: &[exif_anomaly::Severity],
    ) -> exif_anomaly::ExifAnalysis {
        let mut e = exif_anomaly::analyse(None, None, None);
        e.camera_authenticity_bonus = bonus;
        e.is_known_camera_make = known_make;
        e.findings = severities
            .iter()
            .map(|s| exif_anomaly::AnomalyFinding {
                check_id: "t".into(),
                title: "t".into(),
                description: "t".into(),
                severity: *s,
                category: "t".into(),
            })
            .collect();
        e
    }

    fn meta(model: &str) -> metadata::ImageMetadata {
        serde_json::from_value(serde_json::json!({
            "cameraModel": model, "hasMakerNote": false, "makerNoteLength": 0, "xmp": {}
        }))
        .unwrap()
    }

    fn deepfake(level: &str) -> sidecar::DeepfakeResult {
        serde_json::from_value(serde_json::json!({
            "score": 0.5, "suspicious": false, "confidence": "low",
            "verdictLevel": level, "signals": [], "heatmapBase64": "", "summary": ""
        }))
        .unwrap()
    }

    fn manifest(assertion_json: &str) -> c2pa::ManifestInfo {
        serde_json::from_value(serde_json::json!({
            "title": null, "format": null, "claimGenerator": null,
            "assertions": [{ "label": "c2pa.actions", "value": assertion_json }],
            "isValid": true
        }))
        .unwrap()
    }

    const IMAGE_RUN: &[&str] = &["exif_anomaly", "c2pa", "ela", "deepfake"];

    fn inputs<'a>(score: f64, ran: &'a [&'a str]) -> VerdictInputs<'a> {
        VerdictInputs {
            overall_trust: score,
            detectors_run: ran,
            exif_analysis: None,
            image_metadata: None,
            c2pa_valid: None,
            c2pa_manifest: None,
            c2pa_chain: None,
            deepfake_result: None,
        }
    }

    #[test]
    fn score_bands_at_the_boundaries() {
        let makernote = exif(1.0, false, &[]);
        for (score, band) in [
            (0.70, Band::Trusted),
            (0.6999, Band::Uncertain),
            (0.40, Band::Uncertain),
            (0.3999, Band::Untrusted),
            (0.0, Band::Untrusted),
        ] {
            let mut i = inputs(score, IMAGE_RUN);
            i.exif_analysis = Some(&makernote);
            let v = compute_verdict(&i);
            assert_eq!(v.band, band, "score {score}");
            assert_eq!(v.ceiling_applied, None, "score {score}");
            assert_eq!(v.score, score);
        }
    }

    #[test]
    fn high_score_without_positive_evidence_is_capped() {
        let v = compute_verdict(&inputs(0.9, IMAGE_RUN));
        assert_eq!(v.band, Band::Uncertain);
        assert_eq!(
            v.ceiling_applied,
            Some(Ceiling::NoPositiveAuthenticitySignal)
        );
    }

    #[test]
    fn the_cap_only_lowers_trusted() {
        let v = compute_verdict(&inputs(0.5, IMAGE_RUN));
        assert_eq!((v.band, v.ceiling_applied), (Band::Uncertain, None));
        let v = compute_verdict(&inputs(0.1, IMAGE_RUN));
        assert_eq!((v.band, v.ceiling_applied), (Band::Untrusted, None));
    }

    #[test]
    fn deepfake_inconclusive_or_synthetic_caps_trusted() {
        let makernote = exif(1.0, false, &[]);
        for (level, ceiling) in [
            ("inconclusive", Some(Ceiling::DeepfakeInconclusive)),
            ("synthetic", Some(Ceiling::DeepfakeSynthetic)),
        ] {
            let d = deepfake(level);
            let mut i = inputs(0.8, IMAGE_RUN);
            i.exif_analysis = Some(&makernote);
            i.deepfake_result = Some(&d);
            let v = compute_verdict(&i);
            assert_eq!((v.band, v.ceiling_applied), (Band::Uncertain, ceiling));
        }
        let d = deepfake("authentic");
        let mut i = inputs(0.8, IMAGE_RUN);
        i.exif_analysis = Some(&makernote);
        i.deepfake_result = Some(&d);
        assert_eq!(compute_verdict(&i).band, Band::Trusted);

        // A synthetic verdict does not push a low score anywhere.
        let d = deepfake("synthetic");
        let mut i = inputs(0.2, IMAGE_RUN);
        i.deepfake_result = Some(&d);
        let v = compute_verdict(&i);
        assert_eq!((v.band, v.ceiling_applied), (Band::Untrusted, None));
    }

    #[test]
    fn image_without_ela_or_deepfake_is_inconclusive_at_any_score() {
        for score in [0.95, 0.5, 0.05] {
            let v = compute_verdict(&inputs(score, &["exif_anomaly", "c2pa"]));
            assert_eq!(v.band, Band::Inconclusive, "score {score}");
            assert_eq!(v.ceiling_applied, Some(Ceiling::InsufficientSignal));
        }
        // One of the two is enough.
        let v = compute_verdict(&inputs(0.5, &["exif_anomaly", "c2pa", "ela"]));
        assert_eq!(v.band, Band::Uncertain);
    }

    #[test]
    fn non_image_content_is_never_insufficient() {
        let v = compute_verdict(&inputs(0.5, &["c2pa"]));
        assert_eq!((v.band, v.ceiling_applied), (Band::Uncertain, None));
    }

    #[test]
    fn nothing_recorded_is_inconclusive() {
        let v = compute_verdict(&inputs(0.9, &[]));
        assert_eq!(v.band, Band::Inconclusive);
    }

    #[test]
    fn known_make_and_model_count_unless_exif_is_severe() {
        let with_model = meta("SM-G900F");
        let clean = exif(0.0, true, &[exif_anomaly::Severity::Medium]);
        let mut i = inputs(0.8, IMAGE_RUN);
        i.exif_analysis = Some(&clean);
        i.image_metadata = Some(&with_model);
        assert_eq!(compute_verdict(&i).band, Band::Trusted);

        let severe = exif(0.0, true, &[exif_anomaly::Severity::High]);
        i.exif_analysis = Some(&severe);
        assert_eq!(compute_verdict(&i).band, Band::Uncertain);

        let blank = meta("  ");
        i.exif_analysis = Some(&clean);
        i.image_metadata = Some(&blank);
        assert_eq!(compute_verdict(&i).band, Band::Uncertain);
    }

    #[test]
    fn valid_credentials_count_only_without_a_digital_source_type() {
        let plain = manifest(r#"{"actions":[{"action":"c2pa.edited"}]}"#);
        let mut i = inputs(0.8, IMAGE_RUN);
        i.c2pa_valid = Some(true);
        i.c2pa_manifest = Some(&plain);
        assert_eq!(compute_verdict(&i).band, Band::Trusted);

        for declared in [
            r#"{"actions":[{"action":"c2pa.created","digitalSourceType":"http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia"}]}"#,
            r#"{"digitalSourceType":"trainedAlgorithmicMedia"}"#,
            r#"{"schema_org":{"digitalSourceType":"digitalCapture"}}"#,
        ] {
            let m = manifest(declared);
            let mut i = inputs(0.8, IMAGE_RUN);
            i.c2pa_valid = Some(true);
            i.c2pa_manifest = Some(&m);
            assert_eq!(compute_verdict(&i).band, Band::Uncertain, "{declared}");
        }

        // Invalid credentials never count.
        i.c2pa_valid = Some(false);
        i.c2pa_manifest = Some(&plain);
        assert_eq!(compute_verdict(&i).band, Band::Uncertain);
    }

    #[test]
    fn the_chain_is_walked_into_ingredients() {
        let chain = c2pa::ManifestChain {
            active: manifest(r#"{"actions":[{"action":"c2pa.edited"}]}"#),
            ingredients: vec![manifest(
                r#"{"actions":[{"action":"c2pa.created","digitalSourceType":"trainedAlgorithmicMedia"}]}"#,
            )],
            manifest_count: 2,
        };
        let mut i = inputs(0.8, IMAGE_RUN);
        i.c2pa_valid = Some(true);
        i.c2pa_chain = Some(&chain);
        assert_eq!(compute_verdict(&i).band, Band::Uncertain);
    }

    #[test]
    fn serialises_as_the_design_documents() {
        let v = compute_verdict(&inputs(0.55, IMAGE_RUN));
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            serde_json::json!({
                "band": "uncertain",
                "score": 0.55,
                "ceilingApplied": null,
                "bandBoundaries": { "trusted": 0.7, "uncertain": 0.4 }
            })
        );
        let v = compute_verdict(&inputs(0.9, &["exif_anomaly"]));
        assert_eq!(
            serde_json::to_value(&v).unwrap()["ceilingApplied"],
            "insufficientSignal"
        );
    }
}
