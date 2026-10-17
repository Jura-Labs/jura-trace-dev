// SPDX-License-Identifier: AGPL-3.0-or-later

//! From a verification response to an exit code, and to the text a person
//! reads. The band is read from the response's `verdict` block, which the
//! server computes; this client never bands a score itself, so it cannot
//! disagree with the app.

use serde_json::Value;

use crate::exit::{Exit, Failure};

/// `--fail-on`: the band at which a completed verification exits 20.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum FailOn {
    /// Exit 20 when the band is `untrusted`.
    Untrusted,
    /// Exit 20 when the band is `uncertain` or `untrusted`.
    Uncertain,
}

/// Decide the exit code for one successful response.
///
/// `--require-complete` turns a degraded response into 8. With `--fail-on`,
/// an `inconclusive` band is 8 too, not 20: there is no usable verdict to
/// meet a threshold (decided 3 October 2026). Without either option every
/// completed verification is 0, whatever its score.
pub fn judge(
    body: &Value,
    fail_on: Option<FailOn>,
    require_complete: bool,
) -> Result<Exit, Failure> {
    if require_complete && body.get("degraded").and_then(Value::as_bool) == Some(true) {
        return Err(Failure::new(
            Exit::Incomplete,
            "the result is degraded: some analysis services were unavailable \
             (--require-complete)",
        ));
    }
    let Some(fail_on) = fail_on else {
        return Ok(Exit::Success);
    };
    let band = body.pointer("/data/verdict/band").and_then(Value::as_str);
    match (band, fail_on) {
        (None, _) => Err(Failure::new(
            Exit::Incomplete,
            "the server's response has no verdict band (servers before v1.2.0 do not send \
             one), so --fail-on cannot be applied",
        )),
        (Some("inconclusive"), _) => Err(Failure::new(
            Exit::Incomplete,
            "the verdict is inconclusive: the core image detectors did not run, so there is \
             no band to compare with --fail-on",
        )),
        (Some("untrusted"), _) | (Some("uncertain"), FailOn::Uncertain) => Ok(Exit::Verdict),
        (Some("trusted"), _) | (Some("uncertain"), FailOn::Untrusted) => Ok(Exit::Success),
        (Some(other), _) => Err(Failure::new(
            Exit::Incomplete,
            format!("the verdict band '{other}' is not one this client knows; update jura"),
        )),
    }
}

/// What a ceiling means, in a line.
fn ceiling_note(ceiling: &str) -> String {
    match ceiling {
        "insufficientSignal" => "inconclusive: the core image detectors did not run".to_string(),
        "noPositiveAuthenticitySignal" => {
            "capped: nothing positive supports authenticity (camera MakerNote, valid \
             Content Credentials, or a known camera)"
                .to_string()
        }
        "deepfakeInconclusive" => "capped: the AI-image detector was inconclusive".to_string(),
        "deepfakeSynthetic" => {
            "capped: the AI-image detector judged the image synthetic".to_string()
        }
        other => format!("capped: {other}"),
    }
}

/// The human-readable summary for one input. Not a compatibility surface:
/// scripts read `--format json`.
pub fn text(label: &str, body: &Value) -> String {
    let data = body.get("data").unwrap_or(&Value::Null);
    let s = |p: &str| data.pointer(p).and_then(Value::as_str);
    let mut out = format!("{label}\n");
    let mut line = |k: &str, v: String| out.push_str(&format!("  {k:<12} {v}\n"));

    let score = data.get("overallTrust").and_then(Value::as_f64);
    match (s("/verdict/band"), score) {
        (Some(band), Some(score)) => line("verdict", format!("{band} ({score:.2})")),
        (None, Some(score)) => line(
            "score",
            format!("{score:.2} (no verdict band from this server)"),
        ),
        _ => line("verdict", "none in the response".into()),
    }
    if let Some(mode) = s("/mode") {
        line("mode", mode.into());
    }
    if let Some(ran) = data.get("detectorsRun").and_then(Value::as_array) {
        line("detectors", format!("{} run", ran.len()));
    }
    if let Some(sha) = s("/inputSha256") {
        line("sha256", sha.into());
    }
    if data.get("provenance").is_some_and(|p| !p.is_null()) {
        line(
            "engine",
            format!(
                "{}   sidecar {}   {}   {}",
                s("/provenance/engineVersion").unwrap_or("?"),
                s("/provenance/sidecarVersion").unwrap_or("unavailable"),
                s("/provenance/verificationMode").unwrap_or("?"),
                s("/provenance/timestampUtc").unwrap_or("?"),
            ),
        );
    }
    if let Some(c) = s("/verdict/ceilingApplied") {
        line("note", ceiling_note(c));
    }
    if body.get("degraded").and_then(Value::as_bool) == Some(true) {
        line("degraded", "some analysis services were unavailable".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn body(band: Option<&str>, degraded: bool) -> Value {
        let verdict = band.map(|b| json!({ "band": b, "score": 0.5, "ceilingApplied": null }));
        json!({ "data": { "overallTrust": 0.5, "verdict": verdict }, "apiVersion": "1.0", "degraded": degraded })
    }

    fn exit(r: Result<Exit, Failure>) -> Exit {
        r.unwrap_or_else(|f| f.exit)
    }

    #[test]
    fn without_options_every_verdict_is_success() {
        for band in ["trusted", "uncertain", "untrusted", "inconclusive"] {
            assert_eq!(
                exit(judge(&body(Some(band), false), None, false)),
                Exit::Success,
                "{band}"
            );
        }
        assert_eq!(
            exit(judge(&body(Some("untrusted"), true), None, false)),
            Exit::Success
        );
    }

    #[test]
    fn fail_on_thresholds() {
        let cases = [
            ("trusted", FailOn::Untrusted, Exit::Success),
            ("uncertain", FailOn::Untrusted, Exit::Success),
            ("untrusted", FailOn::Untrusted, Exit::Verdict),
            ("trusted", FailOn::Uncertain, Exit::Success),
            ("uncertain", FailOn::Uncertain, Exit::Verdict),
            ("untrusted", FailOn::Uncertain, Exit::Verdict),
            ("inconclusive", FailOn::Untrusted, Exit::Incomplete),
            ("inconclusive", FailOn::Uncertain, Exit::Incomplete),
            ("somethingNew", FailOn::Uncertain, Exit::Incomplete),
        ];
        for (band, fail_on, want) in cases {
            assert_eq!(
                exit(judge(&body(Some(band), false), Some(fail_on), false)),
                want,
                "{band} {fail_on:?}"
            );
        }
    }

    #[test]
    fn no_band_from_an_old_server_is_incomplete_only_with_fail_on() {
        assert_eq!(exit(judge(&body(None, false), None, false)), Exit::Success);
        assert_eq!(
            exit(judge(&body(None, false), Some(FailOn::Untrusted), false)),
            Exit::Incomplete
        );
    }

    #[test]
    fn require_complete_refuses_degraded_first() {
        assert_eq!(
            exit(judge(&body(Some("trusted"), true), None, true)),
            Exit::Incomplete
        );
        assert_eq!(
            exit(judge(
                &body(Some("untrusted"), true),
                Some(FailOn::Untrusted),
                true
            )),
            Exit::Incomplete
        );
        assert_eq!(
            exit(judge(&body(Some("trusted"), false), None, true)),
            Exit::Success
        );
    }

    #[test]
    fn text_leads_with_the_verdict_and_names_the_ceiling() {
        let b = json!({
            "data": {
                "overallTrust": 0.55, "mode": "deep",
                "verdict": { "band": "uncertain", "score": 0.55, "ceilingApplied": "deepfakeInconclusive" },
                "detectorsRun": ["exif_anomaly", "c2pa", "ela"],
                "inputSha256": "9f2c",
                "provenance": { "engineVersion": "1.2.0", "sidecarVersion": "1.2.0",
                                "verificationMode": "deep", "timestampUtc": "2026-11-02T09:15:44Z" }
            },
            "apiVersion": "1.0", "degraded": false
        });
        let t = text("photo.jpg", &b);
        assert!(
            t.starts_with("photo.jpg\n  verdict      uncertain (0.55)\n"),
            "{t}"
        );
        assert!(t.contains("detectors    3 run"), "{t}");
        assert!(
            t.contains("engine       1.2.0   sidecar 1.2.0   deep   2026-11-02T09:15:44Z"),
            "{t}"
        );
        assert!(t.contains("AI-image detector was inconclusive"), "{t}");
        assert!(!t.contains("degraded"), "{t}");
    }
}
