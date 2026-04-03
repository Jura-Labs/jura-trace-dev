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

/** Save the current verification session. */
export function saveVerifySession(session: VerifySession): void {
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(session));
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
