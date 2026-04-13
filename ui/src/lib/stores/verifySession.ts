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
 * Strip large base64 heatmap strings and block-variance arrays from a
 * VerificationResult before persisting to sessionStorage.  This reduces
 * the stored payload from ~3-5 MB (deep mode) to under 50 KB while
 * keeping all scores, flags, and text fields intact.  Heatmaps are
 * re-generated from the live result object when the user navigates back.
 */
function stripHeatmapsForStorage(result: VerificationResult): VerificationResult {
  // Shallow-clone the top level, then null out heavy fields on nested objects.
  // We only touch fields that are known large binary payloads — scores, flags,
  // text fields, and structural metadata are all preserved.
  const r = { ...result };

  // ELA heatmap
  if (r.elaResult) {
    r.elaResult = { ...r.elaResult, elaImageBase64: null as unknown as string };
  }
  // Noise heatmap + block variances
  if (r.noiseResult) {
    r.noiseResult = { ...r.noiseResult, heatmapBase64: null as unknown as string, blockVariances: [] };
  }
  // Copy-move visualisation
  if (r.copyMoveResult) {
    r.copyMoveResult = { ...r.copyMoveResult, visualisationBase64: null as unknown as string };
  }
  // Deepfake heatmap
  if (r.deepfakeResult) {
    r.deepfakeResult = { ...r.deepfakeResult, heatmapBase64: null as unknown as string };
  }
  // NPR heatmap
  if (r.nprResult) {
    r.nprResult = { ...r.nprResult, heatmapBase64: null as unknown as string };
  }
  // JPEG Ghost heatmap
  if (r.jpegGhostResult) {
    r.jpegGhostResult = { ...r.jpegGhostResult, heatmapBase64: null as unknown as string };
  }
  // Segmented ELA heatmap
  if (r.segmentedElaResult) {
    r.segmentedElaResult = { ...r.segmentedElaResult, heatmapBase64: null as unknown as string };
  }
  // Shadow consistency heatmap
  if (r.shadowConsistencyResult) {
    r.shadowConsistencyResult = { ...r.shadowConsistencyResult, heatmapBase64: null as unknown as string };
  }
  // Colour temperature heatmap
  if (r.colourTemperatureResult) {
    r.colourTemperatureResult = { ...r.colourTemperatureResult, heatmapBase64: null as unknown as string };
  }
  // Splice boundary heatmap
  if (r.spliceBoundaryResult) {
    r.spliceBoundaryResult = { ...r.spliceBoundaryResult, heatmapBase64: null as unknown as string };
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
