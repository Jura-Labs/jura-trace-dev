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
 * Strategic context: the v1.0 live release (22 June 2026) deliberately ships
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

/**
 * API Keys section in Settings (key creation, revocation, rate-limit picker).
 *
 * v1.0:  false  — hide the entire section. The REST API on port 8300 ships
 *                 in v1.0 (CLI groundwork JTV-181/182 lands as additive
 *                 fields) but key-based authentication is a Pro-tier UX
 *                 surface that has no audience in the Community-only launch.
 *                 Backend (`api_keys` table, `listApiKeys` / `createApiKey`
 *                 / `revokeApiKey` IPC commands, Axum auth middleware) stays
 *                 in tree — flipping this flag re-enables the UI.
 * v1.1:  true   — re-enable alongside the Pro tier UI unhide (paired with
 *                 project_v102_pro_launch, JTV-170-176). At that point the
 *                 existing `apiKeysAvailable = currentTier === 'professional'
 *                 || currentTier === 'enterprise'` derived state takes over.
 */
export const V1_SHOW_API_KEYS = false;

/**
 * Invisible-watermark embed UI (Protect page: single-asset embed form,
 * batch-watermark panel, header trigger button, "watermarked" filter chip).
 *
 * v1.0:  false  — hide the entire watermark surface. The 2026-05-21
 *                 production diagnosis (commits c8b82bf + 9fc4a22) found two
 *                 coupled bugs: Rust blind_watermark embed + Python
 *                 imwatermark extract use incompatible bit placement, and
 *                 imwatermark's package init eagerly imports rivaGan ->
 *                 torch which PyInstaller statically analyses and pulls
 *                 torch into the bundle even with `excludes = ["torch"]`,
 *                 doubling the bundle to 1.5 GB. Three agents (persona-
 *                 testing, content-authenticity-expert, tech-debt-analyst)
 *                 agreed v1.0 should ship without the feature. 9 of 10 B2B
 *                 personas don't need it; the risk to JTV-184 onedir size
 *                 envelope is real; the manifest-strip-backstop value prop
 *                 is theoretical and not demanded by any pilot user.
 * v1.1:  true   — re-enable after vendoring imwatermark with rivaGan
 *                 lazy-import patched (content-authenticity-expert
 *                 recommendation) OR replacing with a pure-Rust DWT-DCT-SVD
 *                 implementation that lets embed + extract use the same
 *                 library without torch dependency. Backend code (Rust
 *                 blind_watermark, sidecar embed/extract service, Tauri
 *                 IPC commands, REST API routes) stays in tree behind the
 *                 flag — flipping this re-exposes the existing surface.
 */
export const V1_SHOW_WATERMARK = false;


/**
 * Read Text (Ollama LLaVA) button on the Verify result panel + the
 * "Read Text" results section.
 *
 * v1.0:  false  — hide the button + results section. Dropped per 2026-05-21
 *                 user UX review. LLaVA 7B's OCR is "good enough" but not
 *                 best-in-class (macOS Preview, Tesseract, Apple Live Text,
 *                 Google Vision all beat it). Persona evidence: only Fatima
 *                 (fact-checker) might use it occasionally, with dedicated
 *                 OCR tools as a better alternative. Backend Tauri command
 *                 extract_text_from_image + sidecar /forensics/extract-text
 *                 + extract_text_from_image service stay in tree gated by
 *                 this flag (mirrors the V1_SHOW_WATERMARK pattern).
 * v1.1:  true   — re-enable IF user demand emerges post-launch OR we swap
 *                 the OCR backend for a specialised engine.
 */
export const V1_SHOW_READ_TEXT = false;


/**
 * AI image description (Ollama LLaVA "Tier 3" feature on the Verify panel).
 *
 * v1.0:  false  — hide the AI Image Description section + the Settings
 *                 toggle that enables it. Per 2026-05-21 architecture
 *                 review: shipping Ollama + LLaVA + (separately) Qwen2.5
 *                 means recommending 8.5 GB of two competing LLMs. v1.0.1
 *                 will reintroduce a SINGLE multimodal+text model (Qwen2-VL
 *                 7B candidate, ~5 GB) that handles descriptions + future
 *                 claim verification + OCR with one download. v1.0 ships
 *                 pure-forensic + C2PA with no LLM dependency. Backend
 *                 (verify pipeline ai_description call, sidecar /describe
 *                 endpoint, describe_image service) stays in tree gated by
 *                 this flag; backend is also gated by the user preference
 *                 (which defaults to null/false) so the request is never
 *                 issued in production.
 * v1.0.1: true   — re-enable when the v1.0.1 multimodal-model decision
 *                  lands. Probable replacement: Qwen2-VL 7B Instruct
 *                  (5 GB, single download, handles descriptions + RAG +
 *                  OCR with stronger text reasoning than LLaVA).
 */
export const V1_SHOW_AI_DESCRIPTION = false;


/**
 * Watched Locations section on the Monitor tab (add-URL form, watchlist
 * rows, expanded events panel, AI-training notice, RIS roadmap callout).
 *
 * v1.0:  false  — hide the entire Watched Locations section. The feature
 *                 was originally a watermark-tracking surface; with
 *                 V1_SHOW_WATERMARK = false its primary premise is moot.
 *                 The scheduler still ships and idles for Community tier
 *                 (Pro/Enterprise gate prevents accidental polling).
 *                 Backend (monitor_scheduler.rs, 4 IPC commands,
 *                 monitor_urls + monitor_events tables, 23 tests) stays
 *                 in tree behind the flag.
 * v1.1:  true   — re-enable via JTV-206, reframed as "C2PA Manifest
 *                 Integrity Monitor" (the scheduler's c2pa_stripped /
 *                 c2pa_changed events are already implemented) plus the
 *                 "first published at URL X on date Y" provenance
 *                 breadcrumb annotation field (Option 2 from the 5-persona
 *                 review 2026-05-23). Coordinates with JTV-197 audio/video
 *                 fingerprint URL check.
 */
export const V1_SHOW_WATCHED_LOCATIONS = false;
