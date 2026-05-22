// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * C2PA UX Recommendations v1.4 — shared label strings.
 *
 * Single source of truth for the action-label map and the user-facing status
 * strings that appear in the verify page (L1/L2/L3 UI) and in the PDF report.
 *
 * Spec references throughout point at:
 *   https://spec.c2pa.org/specifications/specifications/1.4/ux/UX_Recommendations.html
 *
 * Do not paraphrase these strings locally. If the spec changes, update this
 * file — the PDF and the verify page import from here so they cannot drift.
 */

// ── Action label map (C2PA UX Rec v1.4 §4.3 / Table 3) ────────────────────
//
// Maps c2pa.* action URIs to the consumer-friendly labels the spec prescribes
// in Table 3 "Recommended action labels". British spelling is used for the
// single case the spec is silent on (`c2pa.color_adjustments`) — Rule 6 of
// the Jura Trace house style overrides American spelling in user-facing copy.
// Flag this deviation in the conformance submission letter.
export const C2PA_ACTION_LABELS: Record<string, string> = {
  'c2pa.color_adjustments': 'Colour or exposure edits',
  'c2pa.converted':         'Converted',
  'c2pa.created':           'Created',
  'c2pa.cropped':           'Cropped',
  'c2pa.drawing':           'Drawing edits',
  'c2pa.edited':            'Other edits',
  'c2pa.filtered':          'Filter or style edits',
  'c2pa.opened':            'Opened',
  'c2pa.orientation':       'Changed orientation',
  'c2pa.placed':            'Imported',
  'c2pa.published':         'Published',
  'c2pa.removed':           'Removed',
  'c2pa.repackaged':        'Repackaged',
  'c2pa.resized':           'Resized',
  'c2pa.transcoded':        'Transcoded',
  'c2pa.unknown':           'Unknown edits or activity',
};

// Fallback label for any action URI not listed above.
// Per v1.4 Table 3, `c2pa.unknown` -> "Unknown edits or activity" is the
// correct bucket for unrecognised actions encountered in a real manifest.
export const C2PA_UNKNOWN_ACTION_LABEL = C2PA_ACTION_LABELS['c2pa.unknown'];

/** Look up the consumer-friendly label for a c2pa.* action URI. */
export function c2paActionLabel(action: string | undefined | null): string {
  if (!action) return C2PA_UNKNOWN_ACTION_LABEL;
  return C2PA_ACTION_LABELS[action] ?? C2PA_UNKNOWN_ACTION_LABEL;
}

// ── Status strings (C2PA UX Rec v1.4 Table 4) ─────────────────────────────
//
// Table 4 "Warnings and errors" prescribes the exact user-facing message for
// each validation-failure mode. Use these constants rather than inventing
// synonyms.

/** Table 4 row 1 — manifest tampered or inaccessible. */
export const C2PA_STATUS_INVALID = 'Content Credential unavailable or invalid';

/** Table 4 row 2 — edits outside a C2PA app. */
export const C2PA_STATUS_UNRECORDED_EDITS =
  'Some edits or activity may not have been recorded';

/** Table 4 row 3 — ingredient may contain hidden layers. */
export const C2PA_STATUS_HIDDEN_LAYERS =
  'May contain layers that are hidden or not visible';

/** Table 4 row 4 — assertion connection issues. */
export const C2PA_STATUS_ASSERTIONS_MISSING =
  'Some assertions may be temporarily missing or invalid due to connection issues';

/** Table 4 row 5 — manifest connection issues. */
export const C2PA_STATUS_MANIFEST_UNAVAILABLE =
  'Content Credential temporarily unavailable due to cloud storage connection issues';

// ── Valid-at-signing (Pixel Camera pattern) ───────────────────────────────
//
// Tri-state extension for expired certificates with trusted timestamps.
// Not in v1.4 by name, but maps to "Valid" in C2PA terms (the signature was
// valid at the time it was applied). Kept distinct from the plain "Valid"
// status so the UI can explain the short-lived-credential nuance.
export const C2PA_STATUS_VALID = 'Valid';
export const C2PA_STATUS_VALID_AT_SIGNING = 'Valid at signing';

/**
 * L3 explainer for the Valid-at-signing state.  Spec-safe wording:
 * "signing certificate has expired" is descriptive language drawn
 * directly from the certificate's notAfter attribute; "trusted
 * timestamp" matches the C2PA core spec §14 time-stamp terminology
 * without reusing the TSA-internal "time-stamp token" vocabulary.
 * Earlier drafts used the term "short-lived certificate", which is
 * not part of the UX Rec vocabulary — kept out of the user-facing
 * copy and flagged in the submission letter as a deliberate deviation
 * from any such invented label.
 */
export const C2PA_MSG_VALID_AT_SIGNING_DETAIL =
  'Signing certificate has expired, but a trusted timestamp confirms the signature was valid at the time of signing.';

/**
 * Compressed Invalid label for tight layouts (header strips, small badges).
 * Near-verbatim compression of the Table 4 string; flag in the submission
 * letter as a layout-driven abbreviation.
 */
export const C2PA_STATUS_INVALID_SHORT = 'Invalid or unavailable';

// ── Training-mining reason labels (C2PA Spec 2.1 §18.18) ───────────────────
//
// Maps the four canonical c2pa.training-mining reason IDs to consumer-friendly
// labels. UX Rec v1.4 §6 is the emerging-vocabulary anchor for AI / data-mining
// opt-out display; spec 2.1 §18.18 defines the underlying assertion. Used by
// the verify page to surface the policy a signed manifest carries.
export const C2PA_TRAINING_MINING_REASONS: Record<string, string> = {
  'c2pa.ai_generative_training': 'Generative AI training',
  'c2pa.ai_inference':           'AI inference',
  'c2pa.ai_training':            'General AI training',
  'c2pa.data_mining':            'Data mining',
};

/** Human-readable label for a training-mining policy value. */
export function c2paTrainingMiningPolicyLabel(value: string | undefined | null): string {
  switch (value) {
    case 'allowed':     return 'Allowed';
    case 'notAllowed':  return 'Not allowed';
    case 'constrained': return 'Allowed with constraints';
    default:            return 'Not declared';
  }
}

/**
 * Extract the training-mining policy from a parsed c2pa.training-mining
 * assertion `data` object. Returns a map keyed by canonical reason ID with
 * the value `"allowed" | "notAllowed" | "constrained"`. Tolerates both the
 * `data.entries` and the bare-entries shape per spec 2.1 §18.18 examples.
 */
export function parseTrainingMiningEntries(
  data: unknown,
): Record<string, string> | null {
  if (!data || typeof data !== 'object') return null;
  const obj = data as Record<string, unknown>;
  const entries = (obj.entries as Record<string, unknown> | undefined) ?? obj;
  if (!entries || typeof entries !== 'object') return null;

  const out: Record<string, string> = {};
  for (const reason of Object.keys(C2PA_TRAINING_MINING_REASONS)) {
    const entry = (entries as Record<string, unknown>)[reason];
    if (entry && typeof entry === 'object' && 'use' in entry) {
      const use = (entry as Record<string, unknown>).use;
      if (typeof use === 'string') out[reason] = use;
    }
  }
  return Object.keys(out).length > 0 ? out : null;
}
