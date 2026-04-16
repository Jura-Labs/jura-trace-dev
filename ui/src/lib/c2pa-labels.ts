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
 * Compressed Invalid label for tight layouts (header strips, small badges).
 * Near-verbatim compression of the Table 4 string; flag in the submission
 * letter as a layout-driven abbreviation.
 */
export const C2PA_STATUS_INVALID_SHORT = 'Invalid or unavailable';
