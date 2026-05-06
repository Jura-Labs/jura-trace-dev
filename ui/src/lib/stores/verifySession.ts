// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Verify session store — persists verification results across navigation.
 *
 * When a user verifies an image and then navigates to Help or Settings,
 * the result is preserved in sessionStorage so they can return to it.
 * Session storage is cleared when the browser tab closes.
 */

import type { VerificationResult } from '$lib/types';

const STORAGE_KEY = 'jura-verify-session';

export interface VerifySession {
  /** The verification result. */
  result: VerificationResult;
  /** Original filename. */
  fileName: string;
  /** File path (Tauri) or null for URL verifications. */
  filePath: string | null;
  /** Preview image as a data URL (resized to max 200KB for storage). */
  previewDataUrl: string | null;
  /** The verification mode used. */
  mode: string;
  /** Timestamp when the verification was performed. */
  verifiedAt: string;
  /** Batch items if this was a batch verification. */
  batchItems?: unknown[];
}

/**
 * Strip block-variance arrays from a VerificationResult before persisting to
 * sessionStorage. Heatmap images are now file paths (not base64 blobs) so they
 * are cheap to store — only the large numeric arrays need stripping.
 */
function stripHeatmapsForStorage(result: VerificationResult): VerificationResult {
  const r = { ...result };

  // Block variances are the only remaining large array; all heatmap data is now
  // stored on disk and referenced by path so no further stripping is needed.
  if (r.noiseResult) {
    r.noiseResult = { ...r.noiseResult, blockVariances: [] };
  }

  return r;
}

/** Save the current verification session. Heatmap images are stripped
 *  to keep the payload under the ~5 MB sessionStorage limit. */
export function saveVerifySession(session: VerifySession): void {
  try {
    const stripped = {
      ...session,
      result: stripHeatmapsForStorage(session.result),
    };
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(stripped));
  } catch {
    // sessionStorage full or unavailable — silently ignore
  }
}

/** Restore a saved verification session, or null if none exists. */
export function restoreVerifySession(): VerifySession | null {
  try {
    const raw = sessionStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as VerifySession;
  } catch {
    return null;
  }
}

/** Clear the saved session (e.g. when starting a new verification). */
export function clearVerifySession(): void {
  try {
    sessionStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}
