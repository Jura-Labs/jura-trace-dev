//! Trust-score computation.
//!
//! [`compute_trust`] is the AGPL reproducibility anchor cited in the methodology
//! help page (`ui/src/routes/help/methodology/+page.svelte`) and PDF reports
//! (`ui/src/lib/pdf.ts`).  Every numeric constant lives here so that the
//! public citation points to a single authoritative location.  Do not change a
//! constant without also updating the methodology page, the PDF report template,
//! and the CHANGELOG.

// ── compute_trust weights and thresholds ────────────────────────────────────
// Every numeric constant used by compute_trust is named here so that
// the public AGPL reproducibility citation (methodology help page + PDF reports)
// points to a single authoritative location. Do not change a constant without
// also updating the methodology page, the PDF report template, and the CHANGELOG.

/// Weight applied to the EXIF metadata trust component in the blended base score.
///
/// Reduced from 0.4 after a security audit found forged EXIF could inflate AI-image
/// trust by up to 14 percentage points (see methodology page §2 and Sprint 28 audit).
pub(crate) const EXIF_WEIGHT: f64 = 0.2;

/// Weight applied to the forensic analysis trust component in the blended base score.
///
/// Complement of [`EXIF_WEIGHT`]: `EXIF_WEIGHT + FORENSIC_WEIGHT == 1.0`.
pub(crate) const FORENSIC_WEIGHT: f64 = 0.8;

/// Weight applied to each regional detector (segmented ELA, colour temperature)
/// relative to the primary manipulation signals (ELA, noise, copy-move) at 1.0.
///
/// Higher weight reflects the corroborating value of spatially-localised analysis.
pub(crate) const REGIONAL_DETECTOR_WEIGHT: f64 = 1.5;

/// Trust-score cap applied when two or more regional detectors simultaneously flag
/// the same image as suspicious.
///
/// Corresponds to the "Mixed Signals — Moderate Concern" band in the UI.
/// Cited as "Composite-evidence cap" on the methodology page.
pub(crate) const COMPOSITE_EVIDENCE_CAP: f64 = 0.55;

/// Adjustment added to the base trust score when a cryptographically valid C2PA
/// manifest is present and does not declare AI generation.
///
/// Cited as "+0.10 uplift" on the methodology page.
pub(crate) const C2PA_VALID_BONUS: f64 = 0.1;

/// Adjustment applied to the base trust score when a C2PA manifest or XMP metadata
/// explicitly declares AI-generated content.
///
/// Negative value (penalty). Cited as "reduced by 0.25" on the methodology page.
pub(crate) const C2PA_AI_DECLARED_PENALTY: f64 = -0.25;

/// Verdict ceiling applied when the deepfake ensemble returns "synthetic" with
/// high confidence. Lands in the "Low Trust" band.
pub(crate) const VERDICT_CEILING_SYNTHETIC_HIGH: f64 = 0.25;

/// Verdict ceiling applied when the deepfake ensemble returns "synthetic" with
/// medium confidence.
pub(crate) const VERDICT_CEILING_SYNTHETIC_MEDIUM: f64 = 0.35;

/// Verdict ceiling applied when the deepfake ensemble returns "synthetic" with
/// low confidence.
///
/// Semantically equivalent to inconclusive at this confidence level.
pub(crate) const VERDICT_CEILING_SYNTHETIC_LOW: f64 = 0.45;

/// Verdict ceiling applied when the deepfake ensemble returns "inconclusive".
///
/// Matches [`COMPOSITE_EVIDENCE_CAP`] in numeric value but represents a distinct
/// semantic condition: epistemic uncertainty from the deepfake detector, not
/// convergence of regional manipulation evidence.
pub(crate) const VERDICT_CEILING_INCONCLUSIVE: f64 = 0.55;

/// Trust-score cap applied to low-resolution images where the deepfake ensemble
/// and regional forensics operate below their tested-resolution regime.
///
/// Matches [`COMPOSITE_EVIDENCE_CAP`] and [`VERDICT_CEILING_INCONCLUSIVE`] numerically
/// but is a separate semantic cap: low-resolution confidence bound, not a verdict.
pub(crate) const LOW_RESOLUTION_CAP: f64 = 0.55;

/// Minimum dimension (pixels) below which the low-resolution cap fires.
///
/// Corresponds to the shorter edge of the HD-ready boundary (1366 x 768).
pub(crate) const LOW_RES_MIN_DIMENSION_PX: u32 = 768;

/// Minimum total pixel count below which the low-resolution cap fires.
///
/// Lower edge of typical training-corpus images (1.0 megapixel).
pub(crate) const LOW_RES_MIN_TOTAL_PIXELS: u64 = 1_000_000;

/// Base weight for JPEG Ghost in the manipulation-signal weighted average.
///
/// Half the weight of ELA / noise / copy-move to reflect its narrower scope
/// (single-JPEG-resave splice attacks) and partial redundancy with GBM v4 features.
/// The quality-adaptive formula scales this:
/// `effective_weight = JPEG_GHOST_BASE_WEIGHT * (q/100).max(JPEG_GHOST_QUALITY_FLOOR)`.
pub(crate) const JPEG_GHOST_BASE_WEIGHT: f64 = 0.5;

/// Minimum quality scaling factor for JPEG Ghost weight.
///
/// Prevents the effective weight from collapsing to near-zero at very low quality
/// factors (Q <= 30), where the ghost signal is already attenuated by heavy platform
/// re-encoding.
pub(crate) const JPEG_GHOST_QUALITY_FLOOR: f64 = 0.3;

/// Self-declared pure AI ceiling.
///
/// Applied when a C2PA manifest or XMP metadata declares `trainedAlgorithmicMedia`.
/// Caps trust at 25% regardless of other signals.
pub(crate) const SELF_DECLARED_AI_CEILING: f64 = 0.25;

/// Self-declared composite AI ceiling.
///
/// Applied when `compositeWithTrainedAlgorithmicMedia` is declared (real photograph
/// with AI-generated regions: Pixel Zoom Enhance, Magic Editor, Adobe generative fill).
/// Caps trust at 55% — disclosure, not condemnation.
/// Matches [`COMPOSITE_EVIDENCE_CAP`] numerically but represents a distinct condition.
pub(crate) const SELF_DECLARED_COMPOSITE_CEILING: f64 = 0.55;

/// Compute overall trust from individual forensic scores.
///
/// Uses a concordance-aware formula:
/// - Manipulation signals (ELA, noise, copy-move) are weighted (ELA=2.0,
///   others=1.0) since ELA is the most established forensic technique.
/// - When both ELA and deepfake agree the image is clean but other signals
///   disagree, a concordance boost dampens the outlier scores — this handles
///   codec false positives (AVIF, WebP) without affecting genuine detections.
/// - AI-generation (deepfake) is kept separate so it can't be diluted by
///   manipulation detectors that see AI-generated images as "clean".
/// - The final forensic trust uses the minimum of manipulation and deepfake
///   categories, ensuring either can lower trust.
/// - Regional detectors (segmented ELA, shadow consistency, colour temperature,
///   splice boundary) each contribute their own weighted score. When 2 or more
///   of the four regional detectors are suspicious (score > 0.5) simultaneously,
///   total trust is capped at 0.55 to reflect the convergence of evidence.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_trust(
    ela_score: Option<f64>,
    noise_score: Option<f64>,
    copy_move_score: Option<f64>,
    deepfake_score: Option<f64>,
    deepfake_confidence: Option<&str>,
    deepfake_verdict: Option<&str>,
    exif_trust: f64,
    c2pa_valid: Option<bool>,
    segmented_ela_score: Option<f64>,
    // Shadow consistency and splice boundary were demoted to on-demand
    // investigation tools (April 2026). Callers still pass them positionally
    // for signature stability across RC builds, but they are not used in
    // trust scoring. See the regional-detector comment below.
    _shadow_consistency_score: Option<f64>,
    colour_temperature_score: Option<f64>,
    _splice_boundary_score: Option<f64>,
    ai_declared_by_c2pa: bool,
    // JPEG Ghost was added to scoring in Sprint 28 (S28-4, April 2026).
    // ml-data-scientist + content-authenticity-expert cross-review consensus:
    // Farid 2009 JPEG ghost detection remains the best CPU-only splice /
    // composite signal for single-JPEG-resave attacks that the UnivFD v8
    // probe (CLIP semantic) cannot see and the GBM v4 classifier only
    // partially catches via blocking_strength / dct_benford_div. Wired in
    // at 0.5 base weight (half of ELA/noise/copy-move) as a capped
    // contribution; the effective weight is adjusted downward for
    // heavily-compressed inputs — see backlog #15 and
    // docs/calibration/s28-jpeg-ghost-weight.md §6.3.
    // Parameters are positional-last to keep signature growth backwards-
    // compatible for tests that don't exercise JPEG Ghost scoring.
    jpeg_ghost_score: Option<f64>,
    // Estimated JPEG quality factor (1–100). Used to compute the
    // quality-adaptive effective weight for JPEG Ghost scoring.
    // `None` for non-JPEG inputs — falls back to the 0.5 base weight.
    jpeg_quality_estimate: Option<u8>,
    // Content-type category from the sidecar classifier.
    // When `None` or `Some("photograph")` | `Some("artwork")`, AI-detection
    // signals are used normally.  When the category indicates AI models are
    // unreliable (`"screenshot"`, `"document"`, `"unknown"` with
    // ai_detection_suitable=false), deepfake and CLIP contributions are
    // neutralised to 0.5 (mid-scale) so they do not inflate or deflate the
    // trust score.
    //
    // The `ai_detection_suitable` flag is the authoritative gate; callers
    // that pass `Some("screenshot")` directly should also pass `false` for
    // `ai_detection_suitable` — the two are always consistent.
    content_type_category: Option<&str>,
    ai_detection_suitable: bool,
    // Self-declared AI provenance via XMP IPTC vocabularies. True when
    // exif_anomaly emits a HIGH-severity `xmp_ai_digital_source` or
    // `xmp_ai_creator_tool` finding. Treated equivalently to
    // `ai_declared_by_c2pa` — see `ai_declared` derivation and the
    // self-declared ceiling below.
    ai_declared_by_xmp: bool,
    // Self-declared COMPOSITE AI (real photograph with AI-generated
    // regions composited in). True when:
    //   - exif_anomaly emits a MEDIUM-severity `xmp_ai_composite_source`
    //     finding (IPTC `compositeWithTrainedAlgorithmicMedia`), OR
    //   - the C2PA manifest contains a `compositeWithTrainedAlgorithmicMedia`
    //     digitalSourceType via `detect_composite_ai_from_assertions`.
    //
    // Distinguished from pure AI because the base capture is real (Pixel
    // Zoom Enhance, Magic Editor, generative fill).  Caps trust at 0.55
    // instead of 0.25 — honest disclosure of AI involvement without
    // labelling every camera-with-AI-feature photo as deepfake-class.
    ai_declared_composite: bool,
    // Image dimensions for the low-resolution trust cap. When the image is
    // below the model's tested resolution regime, deepfake detection,
    // copy-move analysis, and regional forensics are less reliable; the cap
    // prevents the trust score from claiming high confidence the underlying
    // analyses cannot support. Added rc.31 (2026-06-09) after a 700x438 AI
    // image returned 77% trust despite the deepfake ensemble being
    // inconclusive — symptom of detector confidence not flowing into the
    // composite when many regional detectors silently return 0 on
    // low-resolution input.  `None` for non-image content (videos, audio,
    // documents) — cap is skipped.
    image_width: Option<u32>,
    image_height: Option<u32>,
) -> f64 {
    // ── AI-detection suppression ─────────────────────────────────────
    // Screenshots and documents cause systematic false positives in the
    // deepfake GBM and CLIP probe because both models were trained
    // exclusively on photographic content.  When the content-type
    // classifier marks a file as unsuitable for AI detection, we pin those
    // signals to 0.5 (neutral — no opinion) so they contribute neither
    // positively nor negatively to the trust score.
    let effective_deepfake_score = if !ai_detection_suitable {
        log::info!(
            "Content type: {} (ai_detection_suitable=false), suppressing AI detection contribution",
            content_type_category.unwrap_or("unknown")
        );
        deepfake_score.map(|_| 0.5) // neutral
    } else {
        deepfake_score
    };
    // The effective deepfake verdict and confidence should also be suppressed
    // so the verdict ceiling in compute_trust is not triggered.
    let effective_deepfake_confidence = if ai_detection_suitable {
        deepfake_confidence
    } else {
        None
    };
    let effective_deepfake_verdict = if ai_detection_suitable {
        deepfake_verdict
    } else {
        None
    };

    // Self-declared AI provenance — either from a C2PA manifest action or
    // from XMP IPTC vocabularies (DigitalSourceType / CreatorTool). Either
    // source is a producer-asserted "this file is AI-generated" signal, so
    // they're treated equivalently. A valid C2PA manifest without an AI
    // declaration is still a positive provenance signal.
    let ai_declared = ai_declared_by_c2pa || ai_declared_by_xmp;
    let c2pa_bonus = if ai_declared {
        C2PA_AI_DECLARED_PENALTY // Penalty: manifest or XMP explicitly declares AI-generated content
    } else if c2pa_valid == Some(true) {
        C2PA_VALID_BONUS // Bonus: valid provenance, not declared AI
    } else {
        0.0
    };

    // Use the effective deepfake score for forensic trust. When AI-detection
    // has been suppressed (e.g. screenshot/document), effective_deepfake_score
    // is pinned to Some(0.5) which yields a neutral deepfake_trust of 0.5.
    // Confidence is expressed via the verdict ceiling below, not by scaling
    // the score down. The old confidence_weight multiplier (low=0.3) nearly
    // eliminated the signal, causing a fake image to show 92% "High Trust"
    // alongside "Inconclusive".
    let deepfake_trust = effective_deepfake_score.map(|s| 1.0 - s);

    // Weighted manipulation signals: ELA, noise, and copy-move at weight 1.0.
    // ELA was previously 2.0 but forensic audit found it generates too many
    // false positives on multiply-compressed images — demoted to match others.
    // JPEG Ghost added in Sprint 28 (S28-4) at weight 0.5 as a capped
    // contribution — half the influence of ELA/noise/copy-move. The half
    // weight reflects its narrower scope (single-JPEG-resave splice attacks)
    // and its partial redundancy with GBM v4 features (blocking_strength,
    // dct_benford_div). It cannot dominate the verdict even when highly
    // suspicious, but it can meaningfully shift trust when the other
    // manipulation signals are ambiguous.
    let mut manipulation_signals: Vec<(f64, f64)> = Vec::new(); // (trust, weight)
    if let Some(s) = ela_score {
        manipulation_signals.push((1.0 - s, 1.0));
    }
    if let Some(s) = noise_score {
        manipulation_signals.push((1.0 - s, 1.0));
    }
    if let Some(s) = copy_move_score {
        manipulation_signals.push((1.0 - s, 1.0));
    }
    if let Some(s) = jpeg_ghost_score {
        // Quality-adaptive weight for JPEG Ghost (backlog #15).
        // Formula: 0.5 × (q / 100).max(0.3), where q is the estimated JPEG
        // quality factor from the input quality assessment.
        //
        // Rationale (v3 corpus evidence, 2026-04-11):
        //   • Q ≈ 95 (direct upload from camera): weight ≈ 0.475 — near full
        //   • Q ≈ 75 (Twitter / WhatsApp re-encode): weight ≈ 0.375 — reduced
        //   • Q ≤ 60 (heavy compression): weight = 0.30 (floor) — heavily
        //     attenuated because platform re-encoding wipes differential ghost
        //     signatures; p95 score for authentic heavily-compressed images
        //     (0.349) exceeds many spliced subtypes, making the signal unreliable
        //   • Non-JPEG / unknown quality: weight = 0.5 (no penalty — cannot judge)
        //
        // The 0.5 base constant is unchanged; only the quality factor scales it.
        // See docs/calibration/s28-jpeg-ghost-weight.md §6.3.
        let quality_factor = jpeg_quality_estimate
            .map(|q| (f64::from(q) / 100.0).max(JPEG_GHOST_QUALITY_FLOOR))
            .unwrap_or(1.0); // non-JPEG: full base weight (JPEG_GHOST_BASE_WEIGHT × 1.0)
        let effective_weight = JPEG_GHOST_BASE_WEIGHT * quality_factor;
        manipulation_signals.push((1.0 - s, effective_weight));
    }

    let manipulation_trust = if manipulation_signals.is_empty() {
        None
    } else if manipulation_signals.len() == 1 {
        Some(manipulation_signals[0].0)
    } else {
        // Weighted average as baseline
        let total_weight: f64 = manipulation_signals.iter().map(|(_, w)| w).sum();
        let weighted_avg: f64 =
            manipulation_signals.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight;

        // Concordance check: when ELA and deepfake both say "clean"
        // (trust > 0.7) but other manipulation signals disagree (trust < 0.3),
        // the disagreement likely reflects codec artefacts rather than
        // real manipulation.
        let ela_trust = ela_score.map(|s| 1.0 - s);

        let concordance_boost = match (ela_trust, deepfake_trust) {
            (Some(ela_t), Some(df_t)) if ela_t > 0.7 && df_t > 0.7 => {
                let disagreeing_count = manipulation_signals
                    .iter()
                    .filter(|(v, _)| *v < 0.3)
                    .count();

                if disagreeing_count > 0 {
                    let agreement_strength = (ela_t + df_t) / 2.0;
                    let disagreement_ratio =
                        disagreeing_count as f64 / manipulation_signals.len() as f64;
                    ((agreement_strength - weighted_avg) * disagreement_ratio * 0.5).max(0.0)
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        Some((weighted_avg + concordance_boost).min(1.0))
    };

    // Use the worst-case forensic signal
    let forensic_trust = match (manipulation_trust, deepfake_trust) {
        (Some(m), Some(d)) => Some(m.min(d)),
        (Some(m), None) => Some(m),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    };

    // ── Regional detector scores ─────────────────────────────────────
    // Segmented ELA (weight 1.5) and colour temperature (weight 1.5) are
    // the remaining auto-pipeline regional detectors. Shadow consistency
    // and splice boundary were demoted to on-demand investigation tools
    // after forensic audit found they add scoring noise without reliable
    // discrimination (shadow: noisy gradient analysis; splice: never sets
    // suspicious=true).
    let regional_signals: Vec<(f64, f64)> = [
        (segmented_ela_score, REGIONAL_DETECTOR_WEIGHT),
        (colour_temperature_score, REGIONAL_DETECTOR_WEIGHT),
    ]
    .iter()
    .filter_map(|(score_opt, weight)| score_opt.map(|s| (1.0 - s, *weight)))
    .collect();

    let regional_trust = if regional_signals.is_empty() {
        None
    } else {
        let total_weight: f64 = regional_signals.iter().map(|(_, w)| w).sum();
        let weighted_avg: f64 =
            regional_signals.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight;
        Some(weighted_avg)
    };

    // Merge regional trust into the overall forensic trust as the worst case.
    let forensic_trust = match (forensic_trust, regional_trust) {
        (Some(f), Some(r)) => Some(f.min(r)),
        (Some(f), None) => Some(f),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    };

    let base_trust = if let Some(ft) = forensic_trust {
        // Weight: EXIF_WEIGHT (20%) corroborating, FORENSIC_WEIGHT (80%) primary.
        // EXIF is trivially forgeable and absent from most social media images.
        // Reduced from 40% after security audit found forged EXIF could boost
        // AI images to 54% trust, bypassing the inconclusive threshold.
        (exif_trust * EXIF_WEIGHT + ft * FORENSIC_WEIGHT + c2pa_bonus).min(1.0)
    } else {
        (exif_trust + c2pa_bonus).min(1.0)
    };

    // ── Composite regional amplification cap ────────────────────────
    // When both remaining regional detectors (segmented ELA + colour temp)
    // simultaneously flag the image as suspicious (score > 0.5), the
    // convergence of evidence warrants a hard cap at 0.55.
    let suspicious_regional_count = [segmented_ela_score, colour_temperature_score]
        .iter()
        .filter(|s| s.map(|v| v > 0.5).unwrap_or(false))
        .count();

    let regional_cap = if suspicious_regional_count >= 2 {
        COMPOSITE_EVIDENCE_CAP
    } else {
        1.0
    };

    // ── Verdict ceiling ─────────────────────────────────────────────
    // Prevents high trust scores when the deepfake detector is uncertain
    // or positive. This replaces the old confidence_weight multiplier
    // which nearly eliminated the signal at low confidence.
    //
    // An "inconclusive" verdict means the system cannot determine whether
    // the image is authentic — trust must reflect that epistemic gap.
    // A "synthetic" verdict with low confidence is semantically equivalent
    // to "inconclusive" — cap in the medium range.
    // Use effective_deepfake_verdict / effective_deepfake_confidence here so
    // that suppressed AI-detection (screenshot/document) does not trigger the
    // verdict ceiling (both will be None when ai_detection_suitable=false).
    let verdict_ceiling = match effective_deepfake_verdict {
        Some("synthetic") => match effective_deepfake_confidence {
            Some("high") => VERDICT_CEILING_SYNTHETIC_HIGH,
            Some("medium") => VERDICT_CEILING_SYNTHETIC_MEDIUM,
            _ => VERDICT_CEILING_SYNTHETIC_LOW, // low confidence synthetic ≈ inconclusive
        },
        Some("inconclusive") => VERDICT_CEILING_INCONCLUSIVE,
        _ => 1.0, // no ceiling for authentic or sidecar offline
    };

    // ── Low-resolution cap ──────────────────────────────────────────
    // When the image is smaller than the model's tested-resolution regime,
    // the deepfake ensemble + regional forensics produce systematically
    // weaker signals (per the model card's "training corpus is JPEG-heavy
    // and weaker on rare formats" disclosure and the v10 platform-forwarded
    // augmentation that targets web/social-media-sized images). The
    // composite trust score therefore cannot honestly claim high confidence.
    // Cap at 0.55 (same value as the regional-amplification cap; lands in
    // the verdict UI's "Mixed Signals — Moderate Concern" band).
    //
    // Triggers if either: min(width, height) < 768 px (below HD-ready
    // boundary 1366x768), OR total pixels < 1.0 megapixel (the lower edge
    // of typical training-corpus images).
    let low_resolution_cap = match (image_width, image_height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            let min_dim = w.min(h);
            let total_px = (w as u64) * (h as u64);
            if min_dim < LOW_RES_MIN_DIMENSION_PX || total_px < LOW_RES_MIN_TOTAL_PIXELS {
                LOW_RESOLUTION_CAP
            } else {
                1.0
            }
        }
        _ => 1.0, // dimensions unknown (non-image or decode failure) — skip
    };

    let result = base_trust
        .min(verdict_ceiling)
        .min(regional_cap)
        .min(low_resolution_cap);

    // ── Self-declared AI ceilings ───────────────────────────────────
    // Pure AI declared (C2PA manifest action `c2pa.created` +
    // `digitalSourceType: trainedAlgorithmicMedia`, OR XMP IPTC
    // `Iptc4xmpExt:DigitalSourceType` / `xmp:CreatorTool`): cap trust
    // at 0.25 regardless of other signals.  Producer self-declaration
    // of pure synthetic AI is the gold-standard provenance signal.
    //
    // Composite AI declared (`compositeWithTrainedAlgorithmicMedia` —
    // real photograph with AI-generated regions: Pixel Zoom Enhance,
    // Magic Editor, Adobe generative fill): cap at 0.55 ("Medium —
    // AI components declared").  The base capture is real and the
    // provenance is intact; the cap is disclosure, not condemnation.
    //
    // If both flags are somehow set (pure AI with composite elements
    // declared), pure AI wins — the stricter ceiling applies.
    let self_declared_ceiling = if ai_declared {
        SELF_DECLARED_AI_CEILING
    } else if ai_declared_composite {
        SELF_DECLARED_COMPOSITE_CEILING
    } else {
        1.0
    };
    result.min(self_declared_ceiling)
}

/// Trust score for non-analysable content types (PDFs, documents).
///
/// Forensic image/video detectors do not apply — trust is based solely on C2PA provenance.
///
/// | `ai_declared` | `c2pa_valid`   | Score | Rationale                                        |
/// |---------------|----------------|-------|--------------------------------------------------|
/// | `true`        | any            | 0.15  | Manifest confirms AI generation — very low trust |
/// | `false`       | `Some(true)`   | 0.82  | Valid manifest — strong provenance signal        |
/// | `false`       | `Some(false)`  | 0.25  | Manifest present but invalid/tampered — suspect  |
/// | `false`       | `None`         | 0.50  | No provenance data — genuinely inconclusive      |
pub(crate) fn document_trust(c2pa_valid: Option<bool>, ai_declared: bool) -> f64 {
    if ai_declared {
        return 0.10; // C2PA explicitly confirms AI generation — very low trust
    }
    match c2pa_valid {
        Some(true) => 0.82,
        Some(false) => 0.25,
        None => 0.50,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_clean_image_with_exif() {
        // All signals clean, full EXIF, authentic verdict → high trust
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(trust > 0.85, "Expected >0.85, got {trust:.3}");
    }

    #[test]
    fn trust_manipulated_image() {
        // ELA, noise, and copy-move all suspicious → low trust
        let trust = compute_trust(
            Some(0.7),
            Some(0.8),
            Some(0.6),
            Some(0.2),
            Some("high"),
            Some("authentic"),
            0.5,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(trust < 0.5, "Expected <0.5, got {trust:.3}");
    }

    #[test]
    fn trust_concordance_dampens_false_positives() {
        // ELA clean, deepfake clean, but noise+copymove maxed (codec false positive)
        let trust = compute_trust(
            Some(0.04),
            Some(1.0),
            Some(1.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.80,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(trust > 0.50, "Expected >0.50, got {trust:.3}");
    }

    #[test]
    fn trust_genuine_manipulation_not_boosted() {
        // ELA is suspicious → concordance boost should NOT fire
        let trust = compute_trust(
            Some(0.7),
            Some(0.8),
            Some(0.5),
            Some(0.2),
            Some("high"),
            Some("authentic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(trust < 0.55, "Expected <0.55, got {trust:.3}");
    }

    #[test]
    fn trust_ela_weighted_higher() {
        // ELA clean but noise suspicious — ELA's 2.0 weight should pull up
        let trust_weighted = compute_trust(
            Some(0.1),
            Some(0.8),
            Some(0.5),
            Some(0.3),
            Some("high"),
            Some("authentic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust_weighted > 0.55,
            "Expected >0.55, got {trust_weighted:.3}"
        );
    }

    #[test]
    fn trust_c2pa_bonus_applied() {
        let trust_without = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_with = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust_with > trust_without,
            "C2PA bonus not applied: {trust_with:.3} vs {trust_without:.3}"
        );
    }

    #[test]
    fn trust_no_forensics_falls_back_to_exif() {
        let trust = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false, None,
            None, None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None, None,
        );
        assert!((trust - 0.8).abs() < 0.01, "Expected ~0.8, got {trust:.3}");
    }

    #[test]
    fn trust_avif_news_image_regression() {
        let trust = compute_trust(
            Some(0.04),
            Some(0.76),
            Some(0.35),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust > 0.65,
            "AVIF news image should score >65%, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_score_bounded() {
        let trust = compute_trust(
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some("high"),
            Some("authentic"),
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(trust <= 1.0, "Trust exceeded 1.0: {trust:.3}");
    }

    // ── Verdict ceiling tests ─────────────────────────────────────────

    #[test]
    fn trust_inconclusive_verdict_caps_trust() {
        // Fake wedding image scenario: deepfake score 0.31, inconclusive verdict.
        // Previously scored 92% "High Trust" — now capped at 55%.
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.31),
            Some("low"),
            Some("inconclusive"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.55,
            "Inconclusive verdict should cap trust at 0.55, got {trust:.3}"
        );
        assert!(
            trust >= 0.30,
            "Trust should still be in medium range, got {trust:.3}"
        );
    }

    #[test]
    fn trust_synthetic_high_confidence_very_low() {
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.85),
            Some("high"),
            Some("synthetic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.25,
            "Synthetic+high should cap at 0.25, got {trust:.3}"
        );
    }

    #[test]
    fn trust_synthetic_low_confidence_capped() {
        // Synthetic with low confidence ≈ inconclusive, caps at 0.45
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.7),
            Some("low"),
            Some("synthetic"),
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.45,
            "Synthetic+low should cap at 0.45, got {trust:.3}"
        );
    }

    /// Item 2 of JTV-105 truth-grid pass — pins the video-trust wiring fix.
    /// `compute_trust` for video files now receives `aggregate_score` from
    /// the per-frame deepfake aggregate (not the always-None image GBM
    /// score). A high-deepfake video must produce a low trust headline.
    #[test]
    fn trust_video_high_deepfake_score() {
        // Pristine deepfake video: aggregate_score 0.92 + synthetic + high
        // confidence. Image manipulation signals are all None for video.
        let trust = compute_trust(
            None, // ela_score (video — None)
            None, // noise_score
            None, // copy_move_score
            Some(0.92),
            Some("high"),
            Some("synthetic"),
            0.5,  // exif_trust (video EXIF)
            None, // c2pa_valid
            None, // segmented_ela_score
            None, // shadow_consistency_score
            None, // colour_temperature_score
            None, // splice_boundary_score
            false,
            None, // jpeg_ghost_score (video — None)
            None,
            None,
            true,
            false,
            false,
            None,
            None,
        );
        assert!(
            trust < 0.25,
            "High video deepfake (0.92, synthetic, high) should cap trust < 0.25, got {trust:.3}"
        );
    }

    /// Item 2 of JTV-105 — clean video with low aggregate_score must
    /// produce a high trust headline (no false synthetic verdict).
    #[test]
    fn trust_video_clean_deepfake_score() {
        let trust = compute_trust(
            None,
            None,
            None,
            Some(0.08),
            Some("high"),
            Some("authentic"),
            0.5,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            false,
            false,
            None,
            None,
        );
        assert!(
            trust > 0.75,
            "Clean video deepfake (0.08, authentic, high) should yield trust > 0.75, got {trust:.3}"
        );
    }

    #[test]
    fn trust_authentic_verdict_no_ceiling() {
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust > 0.85,
            "Authentic verdict should allow high trust, got {trust:.3}"
        );
    }

    #[test]
    fn trust_no_verdict_no_ceiling() {
        // Sidecar offline — no verdict available, should not impose ceiling
        let trust = compute_trust(
            Some(0.04),
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust > 0.70,
            "No sidecar should fall back to EXIF, got {trust:.3}"
        );
    }

    #[test]
    fn trust_verdict_ceiling_overrides_high_base_trust() {
        // The key regression test: a fake image with clean ELA/noise/copy-move
        // but inconclusive deepfake should NOT show "High Trust".
        // Previously: trust = 0.92 (92% High Trust) — dangerously misleading.
        // Now: capped at 0.55 by inconclusive ceiling.
        let trust = compute_trust(
            Some(0.05), // ELA clean
            Some(0.06), // Noise clean
            Some(0.0),  // Copy-move clean
            Some(0.31), // Deepfake borderline
            Some("low"),
            Some("inconclusive"),
            0.95, // Good EXIF (web image with some data)
            None,
            None,
            None,
            None,
            None, // no regional detectors
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.55,
            "Inconclusive should cap trust at 55% max, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_xmp_ai_digital_source_capped_at_25() {
        // Firefly→Photoshop Web→JPEG round-trip pattern: C2PA stripped, but
        // XMP IPTC DigitalSourceType: TrainedAlgorithmicMedia survives.
        // GBM inconclusive, CLIP/UnivFD lukewarm, no tampering. Without
        // the self-declared ceiling, trust ≈ 0.55 (Concern). With it,
        // capped at 0.25 — the file says it's AI, so we believe the file.
        let trust = compute_trust(
            Some(0.05), // ELA clean
            Some(0.06), // Noise clean
            Some(0.0),  // Copy-move clean
            Some(0.41), // Deepfake borderline
            Some("low"),
            Some("inconclusive"),
            0.85, // EXIF still has useful data
            None,
            None,
            None,
            None,
            None,
            false, // ai_declared_by_c2pa: false (manifest stripped)
            None,
            None,
            None,
            true,
            true,  // ai_declared_by_xmp: TRUE (the new behaviour)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.25,
            "XMP self-declared AI must cap trust at 25% max, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_composite_ai_capped_at_055_not_025() {
        // Pixel Zoom Enhance / Magic Editor pattern: real photo with
        // AI-composited regions.  XMP DigitalSourceType =
        // compositeWithTrainedAlgorithmicMedia.  Should cap at 0.55
        // (Medium — AI components declared), NOT 0.25 (pure-AI ceiling).
        // The base photograph is real; the AI-touched regions warrant
        // disclosure but not deepfake-class trust.
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.20),
            Some("medium"),
            Some("authentic"),
            0.95,
            Some(true), // c2pa_valid: Pixel manifest is fully valid
            None,
            None,
            None,
            None,
            false, // ai_declared_by_c2pa: false (composite is separate)
            None,
            None,
            None,
            true,
            false, // ai_declared_by_xmp: false (composite is separate)
            true,  // ai_declared_composite: TRUE (the new behaviour)
            None,
            None,
        );
        assert!(
            trust > 0.25,
            "Composite AI must NOT trigger the 0.25 pure-AI ceiling, got {:.1}%",
            trust * 100.0
        );
        assert!(
            trust <= 0.55,
            "Composite AI must cap at 0.55, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_pure_ai_overrides_composite_when_both_set() {
        // Defensive: if both flags are somehow set, pure AI wins (stricter
        // ceiling applies). Real cases shouldn't have both, but the code
        // path must be deterministic.
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            true, // pure-AI declared
            true, // composite-AI also declared
            None,
            None,
        );
        assert!(
            trust <= 0.25,
            "Pure-AI ceiling must dominate when both flags set, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn trust_xmp_declaration_overrides_high_base_trust() {
        // Even with all forensics clean and a strongly authentic deepfake
        // verdict (which would otherwise yield > 0.85), an XMP self-declared
        // AI provenance signal still caps the score at 0.25.
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            1.0,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            true,  // ai_declared_by_xmp: TRUE
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.25,
            "XMP self-declared AI must override even high base trust, got {:.1}%",
            trust * 100.0
        );
    }

    #[test]
    fn platform_fingerprint_is_informational_only() {
        // JTV-134 informational-only contract: the platform fingerprint
        // result does NOT contribute to `compute_trust`.  This test guards
        // against future regressions where someone wires the result into
        // the trust formula.  Two clean calls with identical args MUST
        // produce identical scores.  If a future change adds a platform
        // fingerprint parameter to `compute_trust`, this call site fails
        // to compile — that is the tripwire.
        let baseline = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            false,
            false,
            None,
            None,
        );
        let again = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            true,
            false,
            false,
            None,
            None,
        );
        assert!((baseline - again).abs() < f64::EPSILON);
    }

    // ── Regional detector trust tests ─────────────────────────────────

    #[test]
    fn trust_single_regional_detector_lowers_trust() {
        // Segmented ELA alone (score 0.7) should lower trust below a clean baseline
        let trust_with = compute_trust(
            Some(0.05),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.7),
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_without = compute_trust(
            Some(0.05),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust_with < trust_without,
            "Regional segmented ELA should lower trust: {trust_with:.3} vs {trust_without:.3}"
        );
    }

    #[test]
    fn trust_two_suspicious_regional_detectors_cap_at_055() {
        // Two regional detectors both > 0.5 → composite amplification cap applies
        let trust = compute_trust(
            Some(0.05),
            Some(0.05),
            Some(0.0),
            Some(0.10),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.7), // segmented ELA suspicious
            None,
            Some(0.65), // colour temperature suspicious
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.55,
            "Two suspicious regional detectors should cap trust at 0.55, got {trust:.3}"
        );
    }

    #[test]
    fn trust_three_suspicious_regional_detectors_still_capped() {
        // Both active regional detectors suspicious + shadow (ignored) — cap holds
        // Shadow consistency score is passed but no longer participates in
        // regional signals after the forensic audit demotion.
        let trust = compute_trust(
            Some(0.05),
            None,
            None,
            Some(0.10),
            Some("high"),
            Some("authentic"),
            0.9,
            None,
            Some(0.8),  // segmented ELA (active)
            Some(0.6),  // shadow consistency (ignored in scoring)
            Some(0.75), // colour temperature (active)
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.55,
            "Two active suspicious regional detectors should cap at 0.55, got {trust:.3}"
        );
    }

    #[test]
    fn trust_one_suspicious_regional_detector_no_cap() {
        // Only one regional detector suspicious (score > 0.5) — cap should NOT fire
        let trust = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.6), // segmented ELA suspicious
            None,      // shadow — absent
            Some(0.3), // colour temperature clean
            None,      // splice boundary — absent
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust > 0.55,
            "Single suspicious regional detector should not trigger the 0.55 cap, got {trust:.3}"
        );
    }

    #[test]
    fn trust_regional_detectors_all_clean_no_penalty() {
        // All four regional detectors clean — trust should match no-regional baseline
        let trust_regional = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            Some(0.05),
            Some(0.04),
            Some(0.06),
            Some(0.03),
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_no_regional = compute_trust(
            Some(0.04),
            Some(0.05),
            Some(0.0),
            Some(0.15),
            Some("high"),
            Some("authentic"),
            0.95,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        // With all regional detectors clean the trust should be close to the
        // no-regional baseline (regional scores ≈ 0 contribute ~1.0 trust).
        assert!(
            (trust_regional - trust_no_regional).abs() < 0.05,
            "Clean regional detectors should not significantly alter trust: \
             regional={trust_regional:.3} vs baseline={trust_no_regional:.3}"
        );
    }

    #[test]
    fn trust_regional_cap_overrides_verdict_ceiling() {
        // Regional cap (0.55) equals the inconclusive verdict ceiling (0.55)
        // — the minimum of both must apply.
        let trust = compute_trust(
            Some(0.05),
            None,
            None,
            Some(0.31),
            Some("low"),
            Some("inconclusive"),
            0.8,
            None,
            Some(0.7), // two regional detectors suspicious → cap 0.55
            None,
            Some(0.65),
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust <= 0.55,
            "Regional cap should be binding when stricter than verdict ceiling, got {trust:.3}"
        );
    }

    // ── JPEG Ghost quality-adaptive weight tests (backlog #15) ───────────
    // Validates the effective_weight = 0.5 × (q/100).max(0.3) formula.
    // See docs/calibration/s28-jpeg-ghost-weight.md §6.3.
    //
    // IMPORTANT: the weight only affects trust when JPEG Ghost is combined
    // with other manipulation signals (ELA / noise / copy-move) — the weighted
    // average only fires when manipulation_signals.len() > 1. Tests that
    // exercise the weight effect therefore include at least one other signal.

    #[test]
    fn trust_jpeg_ghost_quality_95_near_full_weight() {
        // Q=95: quality_factor=0.95, effective_weight=0.475 vs base 0.5.
        // With ELA also present (multi-signal path), the ghost weight matters.
        // Expect trust at Q=95 to be very close to base (within 3pp).
        let trust_q95 = compute_trust(
            Some(0.1), // ELA clean — provides second signal so weighted-avg fires
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.6), // JPEG Ghost suspicious
            Some(95),  // jpeg_quality_estimate → effective_weight=0.475
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_base = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.6), // same ghost score
            None,      // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        // At Q=95, effective_weight=0.475 vs base=0.5 — small difference (< 3pp)
        assert!(
            (trust_q95 - trust_base).abs() < 0.03,
            "Q=95 weight (0.475) should be near base 0.5 weight: q95={trust_q95:.3} base={trust_base:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_quality_75_reduced_weight() {
        // Q=75 (Twitter/WhatsApp re-encode): quality_factor=0.75, effective_weight=0.375.
        // With ELA also present, ghost at Q=75 pulls less on the weighted average.
        // Trust should be higher than base (quality_factor=1.0) case.
        let trust_q75 = compute_trust(
            Some(0.1), // ELA clean — second signal so weighted-avg fires
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // JPEG Ghost very suspicious
            Some(75),  // jpeg_quality_estimate → effective_weight=0.375
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_base = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // same ghost score
            None,      // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        // Q=75 → effective_weight=0.375 < 0.5 → ghost penalises less → higher trust
        assert!(
            trust_q75 > trust_base,
            "Q=75 should produce higher trust (attenuated weight) than base: q75={trust_q75:.3} base={trust_base:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_quality_floor_at_30() {
        // Floor: quality_factor = (q/100).max(0.3).
        // Q=30 → 30/100 = 0.30, max(0.30, 0.30) = 0.30 → floor exactly engaged.
        // Q=20 → 20/100 = 0.20, max(0.20, 0.30) = 0.30 → floor also engaged.
        // Both produce identical effective_weight (0.15) → identical trust.
        let trust_q30 = compute_trust(
            Some(0.1), // ELA clean — second signal so weighted-avg fires
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.8), // JPEG Ghost suspicious
            Some(30),  // jpeg_quality_estimate — floor exactly engaged
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_q20 = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.8), // same ghost score
            Some(20),  // jpeg_quality_estimate — floor also engaged
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        // Both floor at quality_factor=0.30 → effective_weight=0.15 → same trust
        assert!(
            (trust_q30 - trust_q20).abs() < 0.001,
            "Q=30 and Q=20 both floor at quality_factor=0.30 → same trust: q30={trust_q30:.3} q20={trust_q20:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_quality_none_uses_full_base_weight() {
        // Non-JPEG input (PNG, AVIF): jpeg_quality_estimate=None.
        // quality_factor=1.0 → effective_weight=0.5 — unchanged from pre-#15 behaviour.
        // Verify ghost still lowers trust vs no ghost (weight is active).
        let trust_with_ghost = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.7), // JPEG Ghost suspicious
            None,      // jpeg_quality_estimate: None → quality_factor=1.0
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_no_ghost = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false,
            None,  // no JPEG Ghost score at all
            None,  // jpeg_quality_estimate: None uses 0.5 base weight
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None, None,
        );
        assert!(
            trust_with_ghost < trust_no_ghost,
            "Unknown quality should still apply 0.5 weight (ghost lowers trust):              with={trust_with_ghost:.3} without={trust_no_ghost:.3}"
        );
    }

    #[test]
    fn trust_jpeg_ghost_higher_quality_lower_penalty() {
        // Snapshot: same ghost score at Q=95 vs Q=20 (floor engaged).
        // Q=95: effective_weight=0.475. Q=20: effective_weight=0.15 (floor).
        // With ELA present, the weight difference is visible in the trust score.
        // Q=20 (less penalty) → higher trust than Q=95.
        // Demonstrates Elena Vasquez's direct-upload (Q≈95) images receive near-full
        // JPEG Ghost signal; platform-forwarded (Q≤30) images are heavily attenuated.
        let trust_high_q = compute_trust(
            Some(0.1), // ELA clean — multi-signal path
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // highly suspicious ghost
            Some(95),  // direct camera upload → effective_weight=0.475
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_low_q = compute_trust(
            Some(0.1), // same ELA
            None,
            None,
            None,
            None,
            None,
            0.8,
            None,
            None,
            None,
            None,
            None,
            false,
            Some(0.9), // same ghost score
            Some(20),  // heavy compression → effective_weight=0.15 (floor at q/100=0.30)
            None,      // content_type_category: None → no suppression
            true,      // ai_detection_suitable: true → no suppression
            false,     // ai_declared_by_xmp: false (default)
            false,     // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust_low_q > trust_high_q,
            "Heavy-compression input should receive less JPEG Ghost penalty:              q20={trust_low_q:.3} q95={trust_high_q:.3}"
        );
        // Difference should be noticeable — Q=20 weight=0.15 vs Q=95 weight=0.475
        assert!(
            trust_low_q - trust_high_q > 0.02,
            "Quality-adaptive weight difference should be noticeable (>2pp): delta={:.3}",
            trust_low_q - trust_high_q
        );
    }

    // ── AI-declared C2PA trust tests ──────────────────────────────────────────

    #[test]
    fn trust_c2pa_ai_declared_penalises_compute_trust() {
        // When C2PA declares AI generation, trust must be LOWER than the same
        // content without the AI declaration — not rewarded with a C2PA bonus.
        let trust_ai_declared = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true), // valid C2PA
            None,
            None,
            None,
            None,
            true, // AI declared
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_no_ai = compute_trust(
            Some(0.1),
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true), // valid C2PA, no AI declaration
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        assert!(
            trust_ai_declared < trust_no_ai,
            "AI-declared C2PA should produce lower trust than non-AI C2PA: \
             ai={trust_ai_declared:.3} vs clean={trust_no_ai:.3}"
        );
    }

    #[test]
    fn trust_c2pa_ai_declared_lower_than_no_c2pa() {
        // AI-declared content must score lower than content with NO C2PA at all.
        // A manifest that confirms AI generation is worse than no manifest.
        let trust_ai = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            0.8,
            Some(true),
            None,
            None,
            None,
            None,
            true,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_none = compute_trust(
            None, None, None, None, None, None, 0.8, None, None, None, None, None, false, None,
            None, None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None, None,
        );
        assert!(
            trust_ai < trust_none,
            "AI-declared should score below no-C2PA: ai={trust_ai:.3} vs none={trust_none:.3}"
        );
    }

    #[test]
    fn trust_c2pa_ai_declared_penalty_value() {
        // Verify the penalty is -0.25 relative to valid non-AI C2PA (+0.10 bonus).
        // With exif_trust 1.0 and no forensics: valid C2PA → 1.0, AI C2PA → 0.75.
        let trust_valid = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            false,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        let trust_ai = compute_trust(
            None,
            None,
            None,
            None,
            None,
            None,
            1.0,
            Some(true),
            None,
            None,
            None,
            None,
            true,
            None,
            None,  // jpeg_quality_estimate: None → quality_factor=1.0, effective_weight=0.5
            None,  // content_type_category: None → no suppression
            true,  // ai_detection_suitable: true → no suppression
            false, // ai_declared_by_xmp: false (default)
            false, // ai_declared_composite: false (default)
            None,
            None,
        );
        // valid: 1.0 + 0.10 capped at 1.0 = 1.0
        assert!(
            (trust_valid - 1.0).abs() < 0.001,
            "Valid C2PA + perfect EXIF should reach 1.0: got {trust_valid:.3}"
        );
        // AI declared: -0.25 penalty AND self-declared AI ceiling → ≤0.25.
        // Updated 2026-04-23: previously asserted 0.75 (penalty only). The
        // self-declared AI ceiling, added alongside the equivalent XMP
        // detection, now caps any self-declared synthetic asset at 0.25
        // regardless of other signals. Producer self-declaration is the
        // gold-standard provenance signal — forensic disagreement cannot
        // override it.
        assert!(
            trust_ai <= 0.25,
            "AI-declared C2PA must cap trust at 0.25 (self-declared ceiling): got {trust_ai:.3}"
        );
    }

    // ── document_trust ────────────────────────────────────────────────────────

    #[test]
    fn trust_document_ai_declared_very_low() {
        // A document whose C2PA manifest declares AI generation must score very low.
        assert_eq!(
            document_trust(Some(true), true),
            0.10,
            "AI-declared document should yield 0.10 regardless of C2PA validity"
        );
        assert_eq!(
            document_trust(Some(false), true),
            0.10,
            "AI-declared document (invalid C2PA) should still yield 0.10"
        );
        assert_eq!(
            document_trust(None, true),
            0.10,
            "AI-declared document (no C2PA) should still yield 0.10"
        );
    }

    #[test]
    fn trust_document_ai_declared_lower_than_valid_c2pa() {
        let ai_trust = document_trust(Some(true), true);
        let clean_trust = document_trust(Some(true), false);
        assert!(
            ai_trust < clean_trust,
            "AI-declared document trust {ai_trust:.2} must be below valid C2PA trust {clean_trust:.2}"
        );
    }

    #[test]
    fn trust_document_with_valid_c2pa() {
        // A PDF with a valid C2PA manifest should receive a high-confidence score.
        assert_eq!(
            document_trust(Some(true), false),
            0.82,
            "Valid C2PA on a document should yield 0.82"
        );
    }

    #[test]
    fn trust_document_with_invalid_c2pa() {
        // A PDF whose C2PA manifest fails validation is actively suspicious.
        assert_eq!(
            document_trust(Some(false), false),
            0.25,
            "Invalid C2PA on a document should yield 0.25"
        );
    }

    #[test]
    fn trust_document_without_c2pa() {
        // A PDF with no C2PA data at all is genuinely inconclusive — not suspicious.
        assert_eq!(
            document_trust(None, false),
            0.50,
            "Document with no C2PA data should yield 0.50"
        );
    }
}
