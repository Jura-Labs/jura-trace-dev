// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Shared scoring helpers used across verify-page detector rows.
 */

/**
 * Maps a continuous detector score in [0, 1] to a Tailwind class
 * expressing the trust signal of that score.
 *
 * Scale boundaries are aligned with the verdict ceiling logic in
 * `src-tauri/src/lib.rs` and the GBM verdict mapping in
 * `sidecar/app/services/deepfake.py` (synthetic at >0.55, authentic
 * at <0.25, inconclusive between).  The class returned here is for
 * visual emphasis only — it does NOT control any verdict logic.
 */
export function forensicScoreClass(score: number): string {
  if (score < 0.3) return 'text-malachite-dark dark:text-malachite-light';
  if (score < 0.6) return 'text-amber-dark dark:text-amber-light';
  return 'text-cinnabar-dark dark:text-cinnabar-light';
}
