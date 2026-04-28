/**
 * Verify handoff store — one-shot navigation bridge from Protect to Verify.
 *
 * When a user clicks "Verify this asset" in the protect page asset-detail
 * panel, the asset's file path and name are stashed here, then consumed
 * immediately on the verify page's onMount to pre-load the file for
 * verification.
 *
 * Uses Svelte 5 runes via a class-based state object so the store can be
 * imported and used in both SvelteKit route modules without the module
 * boundaries imposed by writable() stores.
 *
 * Persistence: in-memory only — not written to localStorage or
 * sessionStorage.  The handoff is valid only within the current navigation
 * cycle.
 */

export interface VerifyHandoffAsset {
  filePath: string;
  fileName: string;
}

let _handoff: VerifyHandoffAsset | null = null;

/**
 * Set the handoff payload.  Call this immediately before navigating to
 * /verify via goto().  The value is consumed once and then cleared.
 */
export function setVerifyHandoff(asset: VerifyHandoffAsset): void {
  _handoff = asset;
}

/**
 * Consume the handoff payload.  Returns the stashed asset and clears the
 * store in one operation.  Returns null if no handoff is pending.
 *
 * Call this in the verify page's onMount so it fires once per navigation.
 */
export function consumeVerifyHandoff(): VerifyHandoffAsset | null {
  const value = _handoff;
  _handoff = null;
  return value;
}
