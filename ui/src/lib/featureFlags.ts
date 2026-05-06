// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * V1.0 launch feature flags.
 *
 * Each flag is a single boolean that gates a UI surface. Backend code paths
 * for the gated features remain in tree — flipping a flag to `true` is the
 * only change required to re-enable the surface in a future release.
 *
 * Naming convention:  V1_SHOW_<feature>  — true = show, false = hide.
 *
 * Strategic context: the v1.0 live release (29 May 2026) deliberately ships
 * a reduced scope to make the launch achievable for a solo founder and to
 * create credible v1.1+ deliverables for the grant pipeline. Each flag here
 * has a paired plan for re-enable.
 */

/**
 * Conformant C2PA signing UI (Settings → Signing Mode section + the
 * dual-card Bedrock/Conformant selector + cert-import flow).
 *
 * v1.0:  false  — hide the entire Signing Mode section. Bedrock remains the
 *                 implicit default; users sign with the per-install local CA
 *                 and never see a mode toggle. Backend signing dispatcher
 *                 still routes via `signing_mode` state, which stays at the
 *                 'bedrock' default for new installs.
 * v1.1:  true   — re-enable when the C2PA Conformance Programme Validator
 *                 round 3 lands and a documented procurement path exists for
 *                 institution-supplied certificates.
 *
 * Edge case: an rc.x install that previously set 'conformant' will retain
 * that state after v1.0 upgrade; the existing protect-page conformant code
 * paths still execute. Forcing all installs to bedrock at v1.0 boot would
 * be more scope and is deferred unless it surfaces as an issue.
 */
export const V1_SHOW_CONFORMANT_SIGNING = false;
