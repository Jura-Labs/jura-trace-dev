// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * False-positive report — Tier 1 payload helpers for the clipboard-then-email
 * pattern locked under `project_fp_report_v1_locked.md`.
 *
 * # Why this exists
 *
 * The FP-report modal saves a structured record in local SQLite (via
 * `markFalsePositive`) so the user has their own audit trail of cases
 * they disagree with. Nothing is transmitted by the app.
 *
 * To let the user voluntarily share a report with Jura Labs (so it can
 * inform calibration of the next model), the modal also constructs a
 * mailto: URI pre-filled with the **Tier 1 payload only** — a short,
 * fixed set of fields with no PII risk surface. The user is the data
 * controller of their own outgoing email; Jura Labs receives only what
 * the user chooses to send.
 *
 * # Locked exclusions (security-auditor, 2026-05-10)
 *
 * The following are NEVER included in the clipboard / mailto export:
 * - `deepfake_score` — a float is a narrow fingerprint when combined
 *   with mime + timestamp
 * - `signalScoresJson` — content-correlated detector outputs
 * - `file_hash` — a stable identifier
 * - `reason_note` free text — saved locally only; free text inherently
 *   carries PII risk
 *
 * # Field surface
 *
 * Five fields. All auto-collected, all coarse-grained.
 *
 * - `reason_code` — the dropdown enum chosen by the user
 * - `mime_type` — the analysed file's MIME type (e.g. `image/png`)
 * - `app_version` — value from `getVersion()` Tauri IPC
 * - `platform` — coarse OS string from `userAgentData.platform` with
 *   `navigator.platform` fallback (e.g. `macOS`, `Windows`)
 * - `timestamp` — ISO 8601 UTC
 */

export interface FpReportPayload {
  reasonCode: string;
  mimeType: string;
  appVersion: string;
  platform: string;
  timestamp: string;
}

const FEEDBACK_EMAIL = 'feedback@juralabs.org';

/**
 * Resolve a coarse platform string preferring the modern
 * `navigator.userAgentData.platform` (returns `macOS` / `Windows` /
 * `Linux`) with `navigator.platform` (returns `MacIntel` / `Win32` etc.
 * — older but ubiquitous) as the fallback.
 */
export function resolvePlatform(): string {
  if (typeof navigator === 'undefined') return 'Unknown';
  const uad = navigator as Navigator & { userAgentData?: { platform?: string } };
  return uad.userAgentData?.platform ?? navigator.platform ?? 'Unknown';
}

/**
 * Build the Tier 1 payload from current state.
 *
 * `appVersion` is passed in rather than fetched here so callers can
 * resolve it once on mount and avoid an async hop on submit.
 */
export function buildFpPayload(
  reasonCode: string,
  mimeType: string | undefined,
  appVersion: string,
): FpReportPayload {
  return {
    reasonCode,
    mimeType: mimeType ?? 'unknown',
    appVersion,
    platform: resolvePlatform(),
    timestamp: new Date().toISOString(),
  };
}

/**
 * Render the payload as a plain-text block for clipboard paste or
 * embedding in a mailto: body.  Kept short on purpose — under 500 bytes
 * after URI-encoding so mailto: links work across mail clients.
 */
export function buildFpClipboardText(payload: FpReportPayload): string {
  return [
    `Jura Trace — False Positive Report`,
    ``,
    `Reason:    ${payload.reasonCode}`,
    `Type:      ${payload.mimeType}`,
    `Version:   ${payload.appVersion}`,
    `Platform:  ${payload.platform}`,
    `Timestamp: ${payload.timestamp}`,
    ``,
    `Add any extra context below (optional — please avoid personal data):`,
    ``,
  ].join('\n');
}

/**
 * Construct a `mailto:` URI pre-filled with the Tier 1 payload.
 *
 * The URI is intentionally kept under 500 bytes so it works across the
 * range of mail clients (some — notably older Outlook builds — silently
 * truncate longer URIs).  Free-form notes are NOT included; the body
 * ends with a prompt instructing the user to add context in their mail
 * client manually.
 */
export function buildFpMailtoUri(payload: FpReportPayload): string {
  const subject = `False Positive Report — Jura Trace ${payload.appVersion}`;
  const body = buildFpClipboardText(payload);
  return (
    `mailto:${FEEDBACK_EMAIL}` +
    `?subject=${encodeURIComponent(subject)}` +
    `&body=${encodeURIComponent(body)}`
  );
}

/**
 * Try to open the user's default mail client with a pre-filled draft.
 *
 * Returns `true` on a best-effort attempt — there is no reliable way to
 * detect whether the OS actually launched a handler.  Callers should
 * always render the clipboard fallback regardless.
 */
export function openFpMailto(payload: FpReportPayload): boolean {
  try {
    const uri = buildFpMailtoUri(payload);
    if (typeof window === 'undefined') return false;
    window.location.href = uri;
    return true;
  } catch {
    return false;
  }
}

/**
 * Copy the plain-text Tier 1 payload to the clipboard.  Returns `true`
 * on success, `false` if the Clipboard API is unavailable or denied.
 */
export async function copyFpReport(payload: FpReportPayload): Promise<boolean> {
  if (typeof navigator === 'undefined' || !navigator.clipboard) return false;
  try {
    await navigator.clipboard.writeText(buildFpClipboardText(payload));
    return true;
  } catch {
    return false;
  }
}

/** Public for testing / display — kept here as the single source of truth. */
export const FP_FEEDBACK_EMAIL = FEEDBACK_EMAIL;
