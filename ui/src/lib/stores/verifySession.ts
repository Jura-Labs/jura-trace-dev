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

// ── Batch list session ────────────────────────────────────────────────
// Separate, deliberately lightweight cache: just enough to redraw the batch
// results list (filename + score + status) after the user navigates away and
// back. The heavy per-item VerificationResult is NOT persisted (it would blow
// the ~5 MB sessionStorage quota for a multi-file batch); drilling into a
// restored row re-verifies that file on demand. Kept under its own key so it
// is independent of the single-result session above.

const BATCH_STORAGE_KEY = 'jura-verify-batch-session';

/** One row of the cached batch list — summary fields only, no detector data. */
export interface BatchListCacheItem {
  id: string;
  fileName: string;
  filePath: string | null;
  status: 'done' | 'error';
  /** Overall trust (0–1) for done items, or null. */
  overallTrust: number | null;
  error: string | null;
  startedAt: number | null;
  finishedAt: number | null;
}

export interface BatchListSession {
  items: BatchListCacheItem[];
  mode: string;
  savedAt: string;
}

/** Save the lightweight batch list so it survives navigation. */
export function saveBatchSession(session: BatchListSession): void {
  try {
    sessionStorage.setItem(BATCH_STORAGE_KEY, JSON.stringify(session));
  } catch {
    // sessionStorage full or unavailable — silently ignore
  }
}

/** Restore the cached batch list, or null if none exists. */
export function restoreBatchSession(): BatchListSession | null {
  try {
    const raw = sessionStorage.getItem(BATCH_STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as BatchListSession;
  } catch {
    return null;
  }
}

/** Clear the cached batch list (e.g. when starting a new verification). */
export function clearBatchSession(): void {
  try {
    sessionStorage.removeItem(BATCH_STORAGE_KEY);
  } catch {
    // ignore
  }
}
