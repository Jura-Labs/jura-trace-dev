# Changelog

All notable changes to Jura Trace (formerly Jura Archive) are documented here, organised by development phase and sprint.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## 28 April 2026 — Format truth-grid honesty pass + Protect page polish (JTV-105 + JTV-114 + JTV-129)

A two-front polish day driven by the 28 April 2026 four-agent audits. **Pilot rollout extended to Week 3 June 2026** (~2026-06-15) to absorb scope expansion (Reverse Image Search proper, ML model training, Track 3 corpus completion).

### Format support truth-grid (JTV-105 → JTV-112) — 7 items

The marketed format list outran what the pipeline can meaningfully analyse. Four agents (Explore, content-authenticity-expert, persona-testing, tech-debt-analyst) converged on a truth-grid; seven remediation items landed in commit `fa56562`:

- **HEIC dependency fix** — `pillow-heif==1.2.0` added to `sidecar/requirements.txt` + `requirements-ci.txt`; lifespan handler registers HEIF opener with CRITICAL log fallback. iPhone photos on Linux production builds no longer silently fail.
- **Video deepfake trust wiring** — `lib.rs:2486` `compute_trust` for video files now substitutes `video_deepfake_result.aggregate_score` for the always-None image GBM score. Pristine deepfake videos no longer return "Likely authentic" headlines.
- **File picker trim** — extracted `PROTECT_FILE_FILTERS` / `VERIFY_FILE_FILTERS` constants in `ui/src/lib/api.ts`; dropped WebM, MKV, AVI, DOCX, animated GIF, audio (WAV/MP3/FLAC/OGG/AAC/M4A), and 3D types. Audio deferred to JTV-113 (v1.1, AASIST + ASVspoof retraining, 10–12 weeks).
- **Codec-aware ELA + JPEG Ghost gating** — new `format_router::should_run_ela()` / `should_run_jpeg_ghost()` helpers (true only for `image/jpeg`); `lib.rs` clears scores to None for non-JPEG mimes before `compute_trust`. ELA + JPEG Ghost no longer contribute uncalibrated noise on PNG / WebP / AVIF / HEIC / TIFF / BMP / GIF.
- **PDF panel relabel** — "Origin metadata only" pill + italic scope note on the PDF Provenance section; verify-page picker filter + error copy aligned to truth-grid.
- **/help/format-support route** — user-visible reference table covering supported / partial / provenance-only / not-supported families with reasoning for each exclusion.

### Protect page v1.0 polish (JTV-114 → JTV-126) — 12 items

Four-agent Protect-page review (persona-testing, ux-frontend-designer, content-authenticity-expert, security-auditor) produced 12 actionable items. **All 12 landed for v1.0**:

- **Sovereign/Conformant signing-mode badge** — read-only badge on both batch and per-asset Sign panels showing the active mode and linking to Settings → Signing Mode. Closes the Amara persona's blocker for ICC-tribunal evidence submissions.
- **Beta notice rewrite** — drops the "Content Credentials" Adobe trademark; clarifies Validator-track was submitted 14 April 2026 and Generator-track is v1.1; names the active signing mode at the point of disclosure.
- **Watermark robustness claims rewrite** — replaces hand-wavy "near-invisible / survives cropping" with PSNR (≈48/42/36 dB) and SSIM (>0.99/0.98/0.96) numbers per Cox/Miller/Bloom 2008 ch.9. Adds explicit "Limits — common to all strengths" pip naming the failure modes (screenshots, crops >10%, JPEG <50, AI regen, adversarial removal).
- **Watermark PNG-output advisory** — both batch and per-asset panels show "Output: saved as a new PNG alongside the original" before the user clicks Begin Watermarking / Embed.
- **`Iptc4xmpExt:DigitalSourceType` URI** — `c2pa.rs:361` now writes `http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture` per C2PA spec for human-captured content.
- **schema-org.CreativeWork licence URI** — emits a `stds.schema-org.CreativeWork` assertion with `license` as URI alongside the existing `c2pa.rights` string for the 5 CC licences (Adobe Inspect interop). "All Rights Reserved" stays string-only — no invented URI.
- **Creator-name self-attestation copy** — both Sign panels now disclose that the creator name is unverified (Farid 2016 ch.3 cross-examination defence).
- **Bulk-actions section extraction** — "Add Credentials to All" + "Watermark All" buttons moved out of the `role="search"` filter bar into a dedicated `<section aria-labelledby="bulk-actions-heading">` — separates "configure my view" from "start destructive operation".
- **Drop zone collapse** — empty-state `p-14` zone retained; populated state collapses to a `p-3` "Import more files" bar.
- **Asset row chrome aligned with DetectorRow** — leading 10×10 status dot (malachite=both, lapis=partial, amber=imported only); signing-mode badge ("Sovereign" / "Conformant") when `c2paSigned`. Status column widened 170 → 220 px.
- **Verify cross-link** — "Verify this asset" CTA in the expanded asset detail panel; new `ui/src/lib/stores/verifyHandoff.ts` module-scope handoff; `/verify` `onMount` consumes the handoff and calls `runFileVerification()` immediately.
- **Batch panel UX hardening** — focus moves into panel on open via `$effect` + `bind:this`; mutual exclusion (opening Sign closes Watermark and vice versa); "Sign Remaining" / "Watermark Remaining" resume buttons after batchCancelled completion screen.

### Bug fix — Export Asset Database silently failing in Tauri (JTV-129)

The three CSV exports on the Protect page (asset database, batch watermark errors, batch sign errors) used the bare `document.createElement('a'); a.click(); URL.revokeObjectURL()` synchronous pattern, which the Tauri webview blocks. Pilot users clicked Export and nothing happened.

- Lifted the `triggerDownload(blob, filename)` helper from `verify/+page.svelte` into `$lib/blob` so both pages now use the same path: native save dialog via `@tauri-apps/plugin-dialog` + `plugin-fs` in Tauri, with a properly DOM-attached anchor and deferred URL revoke as the browser fallback.
- Closes the MEDIUM CSV formula-injection finding from the 28 April security audit by adding `escapeCsvField()` which prepends a tab character to fields starting with `=`, `+`, `-`, `@` before quote-wrapping (OWASP CSV-injection guidance).
- 14 new Vitest cases pin the escape contract and the browser-fallback DOM behaviour (anchor-attached-before-click, deferred revoke timing, formula-injection neutralisation).

### Tickets opened (Plane, JTV)

- **JTV-113** — v1.1 audio deepfake AASIST + ASVspoof retraining (Backlog, 10–12 weeks, Q3 2026)
- **JTV-127** — Track 3 corpus completion (~1,000 Global Majority handset photos, Todo, v1.0 stretch)
- **JTV-128** — UnivFD v10 retraining cycle (Todo, blocked by JTV-127)
- **JTV-130** — Settings → Backup & Restore (Todo, ~3d, full DB snapshot + CSV catalogue import)
- **JTV-98** — Reverse image search (TinEye + Google Vision, BYOK) — promoted from v1.1 to v1.0

### Tests

- Rust: 498 lib tests, 0 failures
- Python sidecar: 420 tests, 0 failures
- Vitest: 102 tests across 5 files (was 88 — added `api.filters.test.ts` 45 cases + `blob.test.ts` 14 cases)
- svelte-check: 0 errors across 464 files

---

## 26 April 2026 — Release Candidate rc.24 — Brand refresh + C2PA Origin fix + tester focus

A brand and tester-experience pass alongside a meaningful C2PA correctness fix uncovered while reproducing the Google Pixel "Zoom Enhance" verification result.

### Added

- **Brand kit in `docs/branding/`** — editable SVG masters (`logo-eye-mark.svg`, three social-card composition variants, palette swatches), an Adobe Fonts setup guide (`BRAND_KIT.md`), a self-contained interactive review page (`brand-page.html`) with live palette adjuster + light/dark toggle, and a macOS app-icon set rendered at the standard sizes (16/32/64/128/256/512/1024 PNGs).
- **Almond-form eye-mark** with malachite iris, lapis stroke and quartz catchlight — replaces the previous concentric-circles favicon. Pupil at 40% of the iris; the catchlight is sub-pixel below 32 px and drops out by design.
- **Self-hosted macOS CI fix** — `actions/setup-python@v5` is now skipped on the Mac mini self-hosted runner (its hard-coded `mkdir /Users/runner/hostedtoolcache` bootstrap pre-empts any `RUNNER_TOOL_CACHE` override and fails before any user step runs). The runner now uses the host's miniconda Python 3.13 directly. Cloud `macos-latest` fallback path retained unchanged.

### Changed

- **Top-nav header logo** doubled (32 → 64 px) and the **"Jura Trace" wordmark** beside it bumped 50% (14 → 21 px) — proportionate visual weight for the new mark in the chrome.
- **Footer logo** sized up 50% (16 → 24 px) for parity with the header.
- **Monitor surface hidden** from the top nav, dashboard chapters, and Help index for the pilot-tester build. The `/monitor` and `/help/monitor` routes remain reachable by URL; all hides are comment-tagged for trivial restoration. Same pattern as the Protect hide for the C2PA Validator evaluation build.
- **Detector-count messaging reconciled** to the post-Sprint-28 truth across all user-facing surfaces: "12 automatic forensic detectors plus 3 on-demand investigation tools" replaces the stale "21 forensic detectors" copy in Berkeley Protocol page (3 spots), Help → Settings (2 spots), and `docs/information-security-summary.md` (2 spots, plus a fuller named-list breakdown).

### Fixed

- **C2PA Origin "Content Credential unavailable or invalid" misreport on Google Pixel images.** The L3 manifest panel was rendering an X with only `ingredient.manifest.validated` as the visible code on Pixel-shaped chains (camera-capture → Google Photos Zoom Enhance edit). Root cause: c2pa-rs emits a single `validation_results.ingredientDeltas[]` entry whose URI is keyed on the **parent** manifest's label + the parent's `c2pa.ingredient` assertion path, not the child manifest's label. The previous matcher searched for `child_label` in the URI and never matched; positional fallback then picked up the parent-keyed summary delta which contained only `ingredient.manifest.validated` plus cert-soft failures, dropping the eight success codes (`claimSignature.validated`, `assertion.dataHash.match`, `timeStamp.validated`, ×3 `assertion.hashedURI.match`, `claimSignature.insideValidity`) the panel needs to render "Signature valid" + "Data integrity confirmed".

  Fix: extracted the resolution logic into `resolve_ingredient_validation_source()` and reordered priority — embedded `ingredient.validation_results.activeManifest` first (authoritative for the ingredient's own outcomes), then URI-matched delta searching for the **parent** label, then positional fallback. Three new regression tests lock the behaviour: `pixel_zoom_enhance_uses_embedded_ingredient_validation`, `ingredient_delta_uri_match_uses_parent_label`, `ingredient_resolver_positional_last_resort`.

### Tests

490 Rust lib tests pass (3 new), clippy + fmt clean.

---

## 25 April 2026 — Release Candidate rc.23 — Accessibility + Enhanced default + audit hygiene

A four-agent codebase audit (Explore, tech-debt-analyst, project-manager, grant-writer) on 25 April produced a converged set of pilot-readiness recommendations. This RC ships the immediate fixes; structural items (e.g. `jura-core` Apache-2.0 dual-licence, audio detection wire-up) are scheduled for May Week 1 in Plane (JTV-44 bumped to urgent, JTV-85 created).

### Added

- **Lighthouse 100 across all key pages in both light and dark mode.** WCAG 2.2 AA compliance verified via Chrome DevTools MCP iteration loop. Dashboard, Verify, Settings, Monitor, Help, How-It-Works, Methodology — all 100 / 100 / 100 (Accessibility / Best Practices / SEO).

### Changed

- **NetworkMode default flipped from Standard to Enhanced.** First-run users now get full Content Credentials validation (OCSP/CRL revocation, remote manifest fetch) out of the box. The local-first USP is preserved as an explicit Settings choice rather than the default. The `EnhancedModeBanner` component is removed from Dashboard and Verify (no longer needed). Reversed the rc.21 design after the C2PA Validator evaluation showed real-world Pixel and Adobe-signed content requires network access for correct trust assessment.
- **Palette adjustments** for WCAG AA contrast on cream / graphite surfaces:
  - `flint.dark` (new) `#5C5A55` — light-mode body secondary text (~6.7:1 on `#FAFAF7`)
  - `flint.light` `#9B9890` → `#ABA8A0` — dark-mode body secondary text (~5.4:1 on `#272B34`)
  - `lapis.light` `#5A85B5` → `#7AA0CC` — dark-mode lapis accent
  - `cinnabar.light` `#D47870` → `#DD8C84` — dark-mode cinnabar accent
  - `malachite.light` `#6B8F5F` → `#7DA771` — dark-mode malachite accent
  - `amber.dark` `#B87D2E` → `#8C5F22` — light-mode amber strong text
- **Class-level mass updates** to enforce the new contrast pairs (~2,400 substitutions across ~40 files): `text-flint` → `text-flint-dark`, `text-cinnabar` → `text-cinnabar-dark`, `text-amber` → `text-amber-dark`, `text-malachite` → `text-malachite-dark`. Opacity-modified tokens (`/N`) on the same colours stripped to solid. Inline body links given `underline underline-offset-2 hover:no-underline` to satisfy link-distinguishability.
- **Verify mode tab selected state** uses `bg-lapis text-white` (was `bg-lapis/20 text-lapis` — failed contrast).
- **Settings "Switch to Enhanced" button** is `bg-lapis text-white` (was border-amber + text-amber-dark — failed contrast).
- **`/help/how-it-works`**: replaced semantically-invalid `<dl>`/`<dt>`/`<dd>` (with wrapping `<div>`s) with `<ul>`/`<li>`/`<h3>`/`<p>`.
- **macOS CI builds** now run on a self-hosted runner registered on Paul's Mac mini (M4). Cost ≈ £0 per macOS build (vs ~£0.50 cloud) and ~30% faster from warm caches. Cloud signing secrets retained for hotfix fallback to `macos-latest`.

### Removed

- **`enfAnalysisResult` field** from `VerificationResult` and the `EnfAnalysisResult` interface. The verify pipeline never invoked `enf_analysis` (zero call-sites in `lib.rs`), so the field was always null but the UI rendered an "Audio ENF Analysis" card from it — a silent disclosure problem flagged by the audit. Python service (`sidecar/app/services/enf_analysis.py`) and the Rust client `AudioDeepfakeResult` + `detect_audio_deepfake` are preserved on disk; the May Week 1 ticket (JTV-85) wires them into the verify pipeline with calibration evidence.

### Fixed

- Verify upload button: removed conflicting `aria-label` so visible text serves as the accessible name (label-content-name-mismatch).
- Monitor "Add URL" button: removed redundant aria-label.
- Monitor "Learn more" link: descriptive text ("Learn more about content monitoring limits") satisfies link-text rule.

### Tests

487 Rust lib tests pass, clippy clean, svelte-check 0 errors, Lighthouse 100 across all audited pages.

---

## 24 April 2026 — Release Candidate rc.22 — Pilot-readiness fixes

Eight fixes triggered by the rc.21 Windows VM smoke test on 23 April. The C2PA Validator approver's review build now produces well-calibrated trust scores on real-world Pixel photos, displays images correctly on Windows, surfaces the Enhanced-mode upgrade contextually, handles installer collisions cleanly, and gives users a working FFmpeg path on Windows Server SKUs.

### Added

- **First-run Enhanced-mode banner** on Dashboard and Verify. One-click upgrade from the local-first Standard default to Enhanced (online OCSP/CRL revocation checks + remote Content Credentials retrieval). Auto-hides once enabled or dismissed; respects the user's choice across sessions via `localStorage`. Preserves the local-first USP while giving evaluators and real-world users the full validation path.
- **Composite-AI trust ceiling at 0.55.** When the IPTC vocabulary declares `compositeWithTrainedAlgorithmicMedia` (Pixel Zoom Enhance, Magic Editor, Adobe generative fill — real photographs with AI-composited regions), trust caps at 0.55 ("Medium — AI components declared") rather than the 0.25 pure-AI ceiling. New `xmp_ai_composite_source` Medium-severity finding and `detect_composite_ai_from_assertions` C2PA path. Two new tests lock the behaviour.
- **Direct FFmpeg download fallback in the Setup Wizard** for Windows installations without `winget` (Server 2022, locked-down enterprise SKUs). Collapsible "No winget?" section surfaces the gyan.dev pre-built static binaries with a 4-step PATH recipe.

### Fixed

- **Image preview now renders on Windows.** The CSP `img-src` allowed `asset:` and `https://asset.localhost` but not `http://asset.localhost`, which is the URL scheme Tauri's WebView2 uses for the asset protocol on Windows. Verify-page thumbnails were showing alt-text fallback only. Added `http://asset.localhost`, `data:`, and `blob:` to the directive.
- **Chain-ingredient validation checks fallback.** The Origin tab in the L3 manifest panel rendered "Content Credential unavailable or invalid" on Pixel Zoom Enhance images, even though the parent ingredient was fully validated. Root cause: the chain builder only consulted top-level `validation_results.ingredientDeltas[]` and missed the ingredient's own embedded `validation_results.activeManifest`. Added a third fallback path that extracts validation checks from the ingredient's own block when no top-level delta matches.
- **Weather Context now respects Enhanced mode.** The button rendered regardless of NetworkMode and called `https://archive-api.open-meteo.com` directly from the WebView. CSP blocked it (open-meteo wasn't in `connect-src`), and the section had no business appearing in the local-first Standard default anyway. Now: section hides entirely in Standard mode (the EnhancedModeBanner above offers the upgrade); CSP allows the open-meteo domain so Enhanced-mode fetches succeed.
- **Windows installer no longer fails on re-install.** Two NSIS hooks (`NSIS_HOOK_PREINSTALL`, `NSIS_HOOK_PREUNINSTALL`) `taskkill` any running `jura-trace.exe` or `jura-sidecar.exe` before file operations begin, with a 500 ms sleep to let Windows release file handles. Resolves the "Error opening file for writing: jura-sidecar.exe" mid-install failure observed on rc.21 when re-installing while the app was open.
- **Windows uninstaller no longer leaves orphan processes** that locked `%LOCALAPPDATA%\Jura Trace` files. Same NSIS hook approach applied to the uninstall flow.

### Tests

487 Rust lib tests pass, clippy + fmt clean, svelte-check 0 errors.

### Known limitations

- The MSI installer variant has always handled the running-process and orphan-cleanup cases more cleanly than NSIS; the new NSIS hooks bring NSIS up to MSI parity for re-install scenarios but the MSI remains the recommended distribution channel for IT-managed deployments.

---

## 23 April 2026 — Release Candidate rc.21 — Validator evaluation build

The C2PA Validator submission is progressing through final approval. The approver requested a copy change and indicated they will soon be presented the application for review. This build consolidates the last set of UX and trust-scoring changes needed for that evaluation, re-enables macOS signing in CI, and hides the Protect workflow (a separate Generator-track submission) to keep the approver's attention on the verification pipeline.

### Added

- **Self-declared AI ceiling in the trust score.** When a file self-declares AI generation — via a C2PA manifest action with `digitalSourceType = trainedAlgorithmicMedia`, OR via the XMP IPTC vocabularies (`Iptc4xmpExt:DigitalSourceType`, `xmp:CreatorTool`) — the overall trust score is now capped at 0.25. Producer self-declaration is the gold-standard provenance signal; forensic and ensemble signals cannot override it. Fixes a calibration gap where a Firefly → Photoshop-Web → JPEG export (C2PA stripped by the editor, XMP AI flag preserved) resolved to 55% "Medium Trust / Concern" instead of the correct Low Trust.
- **macOS Apple Silicon build restored in the release matrix.** Paused since rc.18 while the Apple Developer ID Application certificate was being set up for Jura Labs CIC (Team Y82C4P9L7F). All six signing secrets now configured on `juralabs/jura-archive`; both macOS and Windows produce signed + notarised artefacts on tag.
- **PDF report heatmap transcoding.** Heatmap PNGs are downscaled to 800 px and re-encoded as JPEG (quality 0.75) before embedding. Cuts PDF report size ~5–10× — addresses the approver's round-2 evidence-bundle readability feedback where aggressive global compression had made screenshots illegible.
- **Certificate validity window in L3 disclosure.** `ManifestInfo` carries `certNotBefore` + `certNotAfter`; the expired-certificate disclosure now shows the concrete validity range rather than prose alone.

### Changed

- **V2 is canonical Verify.** The hybrid V2 layout is now the default route at `/verify`; the original V1 is archived at `/verify/classic` (still reachable by URL but not linked from the main nav; retained only for specialist ROI/annotation tooling that hasn't been ported yet). The "v2 Preview" pill and "Switch to classic view" link are removed.
- **"This is normal for most images" helper copy removed** from the No-Content-Credentials state per the C2PA approver's feedback. As more generator products pass conformance, the "normal for most" framing becomes untrue; the message now states the fact and stops.
- **Protect and Signing Modes hidden for the Validator evaluation build.** Both are omitted from the top nav, the dashboard hero, the dashboard narrative chapters, the Help sidebar, and the Help index. The `/protect` and `/help/protect` routes remain reachable by URL and carry an amber beta banner: *"Content Credentials signing is in active development and is not part of the C2PA Validator conformance submission currently under evaluation. A separate Generator-track submission will follow."* All hide edits are comment-tagged "Restore after Generator-track approval" for trivial reversal.
- **Training corpus canonical path** moved from `/Volumes/Samsung USB/Training Data` to `/Volumes/MAC SSD/Training Data` (Thunderbolt SSD — 5–10× faster than the USB mirror). Samsung USB retained as an independent offsite copy via a weekly `org.juralabs.corpus-mirror` LaunchAgent. Eighteen training scripts sed-updated.
- **Tauri permissions** renamed from deprecated `fs:allow-read` / `fs:allow-write` to `fs:allow-read-file` / `fs:allow-write-file` in `capabilities/default.json`.

### Context for reviewers

The approver's 23 April guidance confirmed the Validator and Generator submissions are independently assessed via the same intake form. The Generator-track submission will follow once this Validator approval lands; a subsequent build will un-hide the Protect workflow and include a staging-system link as the approver requested.

### Tests

485 Rust lib tests pass, clippy + fmt clean, svelte-check reports 0 errors (12 pre-existing a11y warnings unchanged).

---

## 15 April 2026 — C2PA "Valid at signing" tri-state

Real-world Google Pixel Camera photos were rendering as **Invalid** in the Verify panel despite carrying a cryptographically sound Google-issued C2PA manifest. Root cause: Pixel uses short-lived signing certificates paired with a trusted timestamp — the standard C2PA pattern for camera-capture credentials. c2pa-rs correctly reports `signingCredential.expired`, but our manifest reader treated any non-`signingCredential.untrusted` failure as outright invalid, collapsing the legitimate "signature was valid at signing time" case into the invalid bucket alongside tamper and hash-mismatch failures.

### Added

- **`ManifestInfo.valid_at_signing: bool`** (`src-tauri/src/c2pa.rs`). True when (a) all failures are cert-soft — `signingCredential.expired` and/or `signingCredential.untrusted`; (b) active-manifest success list contains `timeStamp.validated` or `timeStamp.trusted`; (c) `claimSignature.validated` is present. Serialised as `validAtSigning` in the IPC payload.
- **Amber "Valid at signing" badge** in the Verify C2PA Credentials panel with tooltip explaining short-lived credentials (e.g. Google Pixel Camera). Provenance timeline node renders in amber with the "(valid at signing — certificate has since expired)" label.
- **PDF export** Status row now reads "Valid at signing (certificate expired; trusted timestamp intact)" where applicable.
- **6 new unit tests** covering the derivation: fully valid, self-signed-untrusted-only, Pixel expired+trusted-timestamp, expired-without-timestamp, expired-without-claim-signature, hash-mismatch (419/419 Rust lib tests pass).

### Changed

- `is_valid` is now true for both the fully-valid and valid-at-signing cases; `valid_at_signing` discriminates the two for display. `SimpleVerdict`, `VerdictSummary`, `SignalAgreement`, and other downstream components that already key off `isValid === true` automatically begin treating Pixel-style manifests as valid provenance — no changes needed in those paths.
- Validity derivation extracted into a pure `derive_validity(&json)` function for testability; old inline logic in `read_manifest` removed.

### Why this matters

C2PA's short-lived-cert pattern is the dominant camera-capture model (Pixel Camera, Leica M11-P, Sony α-series firmware). Treating these as outright invalid would have shipped a major false-negative on the single strongest authenticity signal in the product. Fix landed before v1.0 pilot handout.

### Backlog additions

- **Capture Source panel** (deferred) — consolidate C2PA `signature_info.common_name` + `issuer`, EXIF `Make`/`Model` + MakerNote authenticity, and `claim_generator_info[0].name` into a single tiered "Capture Source" output above the forensics stack. Surfaced during Pixel testing when the current UI showed no source attribution despite the manifest carrying `common_name: "Pixel Camera"` (v2 schema uses `claim_generator_info[]` rather than the v1 `claim_generator` string our reader was pulling).
- **Physiological video deepfake signals (rPPG + blink/gaze)** — added as backlog item #18 per Ramanaharan et al. (2025) systematic review finding that spatial + temporal + physiological multimodal fusion is the strongest generalisation path.

---

## 8 April 2026 — Sprint 28 Tech-Debt Sweep (Post-v1.0 Cleanup)

Pre-v1.0 tech-debt audit executed across two days. The audit itself was cross-reviewed by 12 specialist agents (tech-debt-analyst, api-engineer, rust-backend-engineer, ml-data-scientist, content-authenticity-expert, legal-compliance-advisor, project-manager, persona-testing, grant-writer, security-auditor, ux-frontend-designer, rag-ollama-engineer); the cross-review reversed two of the original audit recommendations (kept JPEG Ghost in deep mode at 0.5× weight, kept seasonal/diffusion deletion confirmed) and surfaced one critical release blocker (`.backup` file pickle EoP vector) plus a TRIED Pillar 2 failure in the WITNESS capability brief (line 95 "no cloud calls" claim was already false due to weather cross-reference and reverse image search).

Full audit trail and decision log in `.claude/projects/.../memory/project_tech_debt_audit_apr2026.md`.

### Changed — Detector lineup

The deep-mode parallel detector group reduced from 9 to 5 detectors (noise, copy-move, JPEG Ghost, segmented ELA, colour temperature). Trust scoring now has 12 automatic detectors total plus 3 on-demand investigation tools that do not contribute to the numeric score.

- **Chromatic aberration — deleted entirely** (S28-1, commit `a1d3166`). Forensic audit rated accuracy 1/5 and long-term viability 1/5; content-authenticity-expert confirmed "methodologically real but empirically dead post-2018" (modern smartphones correct CA in-ISP, mirrorless bodies correct from lens profiles, AI generators produce CA-free output). 18 files, 977 deletions across `chromatic_aberration.py` + tests + `/forensics/chromatic-aberration` endpoint + PyInstaller spec + `CaResult` Rust struct and TypeScript interface + display panel in verify page + glossary + methodology page entry.
- **Diffusion artefact detector — deleted** (S28-2, commit `ba3e87d`). Superseded by the trained UnivFD v8 probe (AUC 0.9911). Content-authenticity-expert: "toy version of the real literature" — hand-tuned patch std thresholds, VAE banding via gradient peakiness, and a stale resolution fingerprint set that didn't include Flux, Imagen 4, or Midjourney v6. 8 files, 388 deletions.
- **NPR demoted to on-demand** (S28-3, commit `602fb0c`). Tan et al. AAAI 2024 uses NPR features as input to a learned classifier, not a standalone threshold; partially redundant with UnivFD v8's CLIP-level upsampling artefact detection. `Option<NprResult>` field retained on `VerificationResult` for the on-demand endpoint; sidecar service and endpoint kept.
- **Shadow consistency + splice boundary demoted to on-demand** (S28-6, commit `df88122`). Shadow: gradient-weighted light direction is noisy on textured scenes; the canonical Kee-O'Brien-Farid 2013 shadow-constraint technique requires user ROI. Splice: three-signal heuristic fusion never set `suspicious=true` in production. Both retained as on-demand investigation tools.
- **Seasonal indicators + weather cross-reference — deleted** (Sprint 27 prep, commit `d650c54`). Discovery during execution: both features' Tauri commands were never registered in Rust — the desktop app only ever showed hardcoded browser-mock data. Sidecar services + endpoints + tests + PyInstaller spec + UI state + help/verify page sections all removed. WITNESS capability brief line 95 "No cloud calls. No telemetry." claim corrected in the same session (commit `32a74b0`) to an honest three-feature network carve-out.

### Changed — Trust scoring

- **JPEG Ghost wired into `compute_trust` at 0.5× weight** (S28-4, commit `63dd511`). Reversal from the original tech-debt audit recommendation. ml-data-scientist + content-authenticity-expert cross-review consensus: Farid 2009 JPEG ghost detection remains the best CPU-only signal for single-JPEG-resave splices that UnivFD cannot see and GBM v4 only partially catches via `blocking_strength` / `dct_benford_div`. 0.5 weight (half of ELA/noise/copy-move at weight 1.0) as capped contribution. Empirical calibration backlog item (#9); plan at `docs/calibration/s28-jpeg-ghost-weight.md` blocked on splice corpus sourcing.
- **`compute_trust` signature extended** with `jpeg_ghost_score: Option<f64>` as positional-last. 29 test call sites updated.

### Added — Schema v6 `detectors_run` column

Full write-read-render pipeline for recording which detectors actually produced a result for each verification. Enables PDF / ZIP renderers to distinguish "detector ran and returned null" from "detector not run in this build / mode".

- **Migration** (S28-5, commit `a4bad31`). Schema v6 adds `detectors_run TEXT` column to `verifications`. One-line idempotent `ALTER TABLE`.
- **Writer** (S28-FU1, commit `adf84fe`). `verify_content_inner` builds a stable detector ID list from the actual `*Result` Options, JSON-serialises it, passes it to `insert_verification`, and surfaces it on `VerificationResult.detectors_run`.
- **Read path** (S28-FU6, commit `038bb1a`). `get_verification_history` SELECT extended; JSON TEXT parsed with graceful `None` on NULL / parse failure. `VerificationSummary` struct gains the field with `#[serde(skip_serializing_if = "Option::is_none")]`. New round-trip test (test count 289 → 290).
- **PDF consumer** (S28-FU2, same commit as writer). PDF metadata "Detectors Run" row now prefers the authoritative list, legacy inference fallback.
- **PDF "Not run" labels** (S28-FU4, commit `1a2b3ca`). Raw-scores table renders an explicit italic grey "Not run in this analysis" row for detectors that are in `EXPECTED_DETECTORS_BY_MODE` but absent from `detectors_run`. Pre-FU1 legacy exports get an italic footer note explaining the inferred lineup.
- **ZIP case export manifest** (S28-FU4). New `detectors-run.json` file in the ZIP with authoritative detector list, mode, content type, pipeline version, methodology record, filtered label map, and an `authoritative: true|false` flag. `DETECTOR_ID_LABELS` extracted to a shared module `ui/src/lib/detectorLabels.ts`.

### Added — Claim checker reframing

- **RAG claim checker → knowledge base retrieval aid** (commit `4e4af0d`). `_PROMPT_TEMPLATE_FALLBACK` parametric-memory fallback deleted (single biggest defamation vector per legal-compliance-advisor cross-review — Defamation Act 2013 s.1 + DE/FR equivalents). Verdict vocabulary renamed `supported/disputed/unverified` → `consistent_with_kb/inconsistent_with_kb/insufficient_context_in_kb/mixed_kb_match`. Prompt rewritten to frame as reference-retrieval assessment. Non-warranty notices on both claim display panels + new model card at `/help/model-cards#kb-retrieval`. Legacy vocabulary aliased for backwards compatibility. Full investment deferred to S29-06 multilingual testing gate.

### Changed — UI polish (UI audit batch)

Six UI audit items from the 7 April deferral closed in commit `c8520b9`:

- **UI-1** Dashboard hero: "C2PA Content Credentials" → "C2PA provenance manifest".
- **UI-2** R/G/B channel toggle removed entirely. Non-experts couldn't interpret greyscale channel output; experts use external tools; feature had a reactivity bug where the second toggle sampled the already-greyscale DOM image.
- **UI-3** InspectionChecklist component removed from verify flow. 8 manual visual inspection checks moved to the existing `/help/verify` section, expanded from paragraph to numbered list.
- **UI-4** MakerNote authenticity bonus surfaced. `cameraAuthenticityBonus?: number` added to `ExifAnalysis` TypeScript interface. Malachite info badge renders in the EXIF section when the value exceeds 0.5.
- **UI-5** Setup wizard null-guard — verified no-op. Guard has been in place since `f41c7f1` on 2 April 2026; audit item was stale.
- **UI-6** Three more "content credentials" cleanups: dashboard stat label, monitor page narrative, glossary primary index.

### Changed — Methodology page restructure

- **Automatic vs on-demand split** (S28-6 + FU3 + FU7 + FU8). Intro rewritten. Automatic detectors renumbered consecutively 01–12 (removing gaps at 07/11/13 from CA removal and demotions). On-demand section moved to the end of the list and wrapped in a distinct amber-tinted panel. Side-by-side two-column layout evaluated and rejected (440 px per column is too tight inside the 900 px editorial width for the expanded `<dl>` content).

### Fixed — Security and correctness

- **Deleted `models/deepfake_classifier_v3.joblib.backup`** (commit `87cbb58`). Pickle-deserialisation EoP vector via rename-shadow attack. Flagged as a release blocker by security-auditor cross-review. `.github/workflows/ci.yml` gained a `repo-hygiene` job that fails the build on stray `.backup`/`.bak`/`.joblib.old` files in `models/`.
- **Video deepfake features dict contract** (commit `a28da2f`). Two early-return paths in `deepfake.py` were returning incomplete feature dicts, violating the `perform_deepfake_detection_with_features()` contract: the 128×128 minimum size guard returned `{}`, and the screenshot pre-classifier bypass returned `{"screenshot_confidence": ...}` only. `video_deepfake.py` per-frame temporal drift (`noise_drift`, `spectral_drift`, LBP drift) would `KeyError` on thumbnail video frames or on frames hitting the screenshot classifier (title cards, credits, rendered graphics). Both paths now populate a full `FEATURE_NAMES` NaN dict; `_compute_drift` already filters NaN so drift calculation degrades gracefully instead of crashing. Diagnosed by qa-tester cross-review.
- **Deepfake classifier threshold clarification**. Confirmed `compute_trust` does not apply a runtime threshold on `classifier_score` — the GBM probability is blended via `0.20 × heuristic + 0.30 × classifier + 0.50 × univfd`. The `0.49` in `deepfake_classifier_v4.calibration.json` describes standalone classifier performance only.

### Changed — Models and CI

- **Models directory cleanup** (commit `87cbb58`). Deleted `v1/v2/v3` `.joblib` files + their metadata/calibration/bias_check siblings, plus `evaluation_report.json`, `fp_analysis.json`, `per_sample_results.csv`, `threshold_sweep.csv`. Kept: production `deepfake_classifier.joblib`, `v4` fallback + its metadata, `sprint29_validation.json`, `univfd_probe.joblib`, `univfd_probe_meta.json`.
- **CI `repo-hygiene` job added** to `.github/workflows/ci.yml`.
- **`tech-debt-analyst` agent registered** at `.claude/agents/tech-debt-analyst.md`.

### Docs

- **JPEG Ghost 0.5× weight calibration plan** at `docs/calibration/s28-jpeg-ghost-weight.md` (372 lines). Outcome C — no splice corpus locally. Sweep methodology, three sourcing options with licence status, implementation checklist, future work.
- **Cargo target disk pressure note** added to `CLAUDE.md`. Captures the mid-session `ENOSPC` incident when `src-tauri/target` reached 15 GB and broke `cargo check` / `svelte-check` / Claude Code's own scratch writes. Documents `cargo clean --manifest-path src-tauri/Cargo.toml` as a defensive habit.
- **WITNESS capability brief freshness pass** (commit `ca6d71d`). Training corpus numbers refreshed from stale 10,091 → 10,709 (14 generator families). GBM AUC updated from the overfit `0.945→1.0` to the current v4 0.9868. Sprint 29 "May 2026" references corrected. Pillar 1 compression test suite claim reframed from present-tense to "planned" (no such suite exists yet).

### Statistics

- **13 commits** on main across two days (`87cbb58` → `012d230`), plus 4 follow-up commits (`038bb1a`, `1a2b3ca`, `57b2092`, `012d230`) delegated to specialist agents in parallel worktrees.
- **Net −1,500 lines** of code across the tech-debt sweep (detector deletions dominate).
- **Test counts**: 290 Rust lib tests (+1 new round-trip test), 231 SvelteKit files with 0 errors 0 warnings, 423+ sidecar Python tests (3 pre-existing failures fixed in commit `a28da2f`).

---

## 7 April 2026 — Sprint 29 Progress + Corpus Expansion + Model Refresh

### Added

**Sprint 29 Track 1 — MakerNote Authenticity Bonus (commit `476f265`)**
- `KNOWN_CAMERA_VENDORS` table in `src-tauri/src/metadata.rs` covering 36 vendors including 11 Global Majority brands (Tecno, Infinix, Itel, Realme, Honor, Vivo, Oppo, Xiaomi, OnePlus, Asus, Motorola)
- `camera_authenticity_confidence()` function producing a 0.0–1.0 score based on MakerNote presence and vendor match
- `camera_authenticity_bonus` field on `ExifAnalysis`; threaded through `verify_content_inner` → `detect_deepfake` → sidecar via `?camera_authenticity_bonus=` query parameter
- Applied in Python sidecar with maximum 0.25 score reduction; bypassed when heuristic_score ≥ 0.75
- 6 new `exif_anomaly` unit tests, all passing

**Sprint 29 Track 2 — Wildlife + War/Conflict Corpus (commits `8429728`, `3c817c7`)**
- New crawler `scripts/agents/crawl_wildlife_and_conflict.py` with thumbnail URL strategy, 429 backoff, 1.5 s per-request delay
- 204 wildlife/macro images downloaded from Wikimedia Commons (insects, birds, wild mammals, underwater, reptiles, amphibians, spiders, butterflies)
- 203 war/conflict images downloaded from Wikimedia Commons conflict categories with ethical filters (no casualties, CC licences only, source logging)

**Sprint 29 Track 3 — Forensic Feature Engineering (commit `52a5690`)**
- 4 new features added to `sidecar/app/services/deepfake.py`: `noise_lf_hf_ratio`, `demosaic_inter_channel_coherence`, `noise_anisotropy_mean`, `noise_anisotropy_std`
- Feature vector grows from 80 to 84 dimensions; backwards-compatible: classifier loader trims to `clf.n_features_in_` for GBM v4
- 24 unit tests in `sidecar/tests/test_new_features.py`; all 47 pre-existing deepfake tests still pass

**Sprint 29 Track 4 — Stratified Validation Script (commit `49a7e8a`)**
- `scripts/build_validation_test_set.py` with `build` and `validate` subcommands
- 8 strata: consumer_phone, mirrorless_dslr, drone, web_jpeg_easy, global_majority_handset, wildlife_macro, war_conflict, ai_diverse
- First validation run: 339 images, overall FP 1.68%, AI recall 99.00%

**Composite border detector**
- `scripts/find_composite_borders.py` — flags authentic corpus images that are stacked composites
- Run on full 4,673-image corpus: 21 candidates flagged (0.45% rate); HTML preview at `corpus/border_candidates.html` pending user visual review

**Context7 MCP + GitHub MCP installed**
- `npx -y @upstash/context7-mcp` and `npx -y @modelcontextprotocol/server-github` added via `claude mcp add`
- Both registered in `.claude.json`; require Claude Code restart to activate

### Changed

**GBM Deepfake Classifier — v2 → v3 → v4 retrain chain**
- v2 baseline: 9,658 images (earlier session)
- v3 (commit `4fa4547`): 10,721 images — COCO train set added, audited Wikimedia restored
- v4 final (commit `0857764`): 10,709 images — Wikimedia re-audit removed 12 outliers (cartoons, microscope slide, album artwork, studio composites, underwater shots)
- v4 metrics: AUC-ROC 0.9868, authentic FP 4.54%, AI recall 92.52%, calibrated threshold 0.49, SHA-256 `2931f197cba6f376e85b1cbcfd584e6802f36e4fbf68ff00c83d61d4d655db18`
- Camera FP improvements: consumer (Pixel/iPhone) 15.6% → 8.81% (cleared 2× bias gate); high-end (DJI/DSC) 13.5% → 10.32%

**UnivFD Probe — v6 → v7 → v8 retrain chain**
- v7: 10,724 images, AUC-ROC 0.9909, FP 4.91%
- v8 final (commit `29317db`): 10,712 images, AUC-ROC 0.9911, FP 5.01%, recall 96.01%, SHA-256 `d16fb22baf3981d62888e2458733c1a4c0743a5895470d82e9de76766f776908`

**Corpus size**
- 10,775 → 11,576 images (6,571 authentic + 5,005 AI)

**Wikimedia re-audit**
- 12 outliers removed from training corpus: cartoons, microscope slide, album artwork, studio still-life, underwater shots — content whose visual characteristics confound the classifier's noise/frequency features

### Documentation

- Model cards page (`/help/model-cards`) updated to GBM v4.0 + UnivFD v8.0 with version history tables
- Agent persona files refreshed: `ml-data-scientist`, `content-authenticity-expert`, `rag-ollama-engineer`, `dpia-template.md` — stale "AUC 1.000, FP 0%, 709 images" references replaced with current v4 metrics (commit `7c60a20`)
- Help index page: "Built for Those Who Need It Most" equity paragraph added (commit `06fed51`)
- Sprint 29 TRIED roadmap entry expanded with war/conflict addition (commit `275ba38`)
- TRIED roadmap Sprint 29 reframed from "Fairness Foundation" to "Camera Reinforcement & Bias Prep" (commit `9806f6a`)

### Deferred

Six UI audit items identified by Chrome DevTools review + ux-frontend-designer and content-authenticity-expert agents; deferred to next sprint:

- UI-1: Fix remaining "Content Credentials" text on dashboard (`+page.svelte` line 186)
- UI-2: R/B/G channel toggle — decide remove or repair (root cause: sampling from already-greyscale image after first toggle)
- UI-3: Remove `InspectionChecklist` from verify flow; move content to Help → Verify Guide
- UI-4: Surface MakerNote bonus in verify UI — add `cameraAuthenticityBonus` to `ExifAnalysis` interface; malachite badge when > 0.5
- UI-5: Setup wizard null-guard bug — `+layout.svelte` line 47 fires on null `setupVersion`; add `&& setupVersion !== null`
- UI-6: Three remaining "content credentials" instances in monitor page, glossary, and dashboard stat label

Additional UX recommendations noted (not enumerated, not scheduled): sidecar-offline banner split ("what works / what's unavailable"), Monitor empty-state consolidation, "Investigate Further" panel sub-section split.

### Infrastructure

**Disk cleanup — critical incident**
- Root disk reached 99% capacity mid-session; 33 GB recovered:
  - `~/.cache/huggingface` — 17 GB
  - `src-tauri/target/debug` — ~10 GB
  - `src-tauri/target/release` — ~4 GB
  - pip cache — 1.3 GB
  - Python `__pycache__` directories — ~840 MB
  - `sidecar/build` — 229 MB
- Disk space recovered: 119 MiB → 34 GiB free
- Tauri required cold rebuild after target deletion (~1 m 50 s)

---

## 7 April 2026 — DEC-2026-04-07-001: Option C Corpus Strategy Adopted

### Decision

**Adopted:** Option C — single commercial-safe production model deployed to all Jura Trace tiers (Community, Professional, Team, Enterprise), with a separate research artefact under OpenRAIL-M for academic and human rights research use.

**Rejected:** the previously-proposed two-model strategy of maintaining separate research and commercial production model variants.

### Rationale

Synthesis of four parallel agent reviews (legal-compliance-advisor, ml-data-scientist, project-manager, grant-writer) on 7 April 2026 converged on Option C as the lowest-legal-risk, lowest-operational-cost, highest-grant-credibility approach. Annual operational cost drops from £5,000–6,500 (two-model) to £2,000–4,000 (Option C). Removes Enterprise procurement objection ("why is my paid tier weaker?"). Resolves Mozilla Democracy x AI 2027 open-source eligibility via the OpenRAIL-M research artefact. Strengthens EMIF August 2026 narrative.

### Architectural Requirements

- **Filesystem segregation:** corpus split into `production/` (commercial-safe) and `research/` (research-licensed) paths
- **Training script firewall:** denylist enforcement; refuses to train production model on any file matching a research-licensed dataset hash
- **Model card declaration:** every production model card must state the Option C commitment
- **Research artefact lifecycle:** standalone download, separate model registry path, never bundled with the application binary, OpenRAIL-M licensed with Responsible Use Statement

### Constraints Recognised

- **MLAAD CC-BY-NC ambiguity:** v9 is CC-BY-NC 4.0; commercial-tier deployment under that licence is legally ambiguous. Resolution path: write to Fraunhofer AISEC (MLAAD authors) for explicit case-by-case commercial permission. Until granted, MLAAD is treated as research-only.
- **EU GDPR Article 27 Representative:** escalated to URGENT given EU training data sources (MLAAD, ASVspoof 5)
- **Datasets dropped:** VoxCeleb (institutional friction), In-the-Wild audio (GDPR Article 9 biometric exposure on identifiable politicians and celebrities)
- **Never used:** Mtechlaw/TfGBV-Grok-NCII-Dataset (non-consensual content)

### Documents

**Added**
- `docs/decisions/option-c-corpus-strategy.md` — formal decision memo (DEC-2026-04-07-001) with full rationale, architectural requirements, financial impact, implementation plan, and Responsible Use Statement appendix
- `docs/av-corpus-revised-proposition.md` — four-agent synthesis document with revised propositions covering corpus, audio architecture (W2V2-Base + MFCC ensemble, not XLS-R-300M), audio corpus targets (22K + 20K, not 5K + 5K), face-swap limitation disclosure, DeepFake-Eval-2024 zero-shot benchmark protocol, and Plane mutations

**Updated**
- `docs/av-corpus-methodology.md` — status changed to "PARTIALLY SUPERSEDED"; sections 3, 5.3, 6, 8, 10, 11 superseded by revised proposition and decision memo
- `docs/sprint-plans/tried-compliance-roadmap.md` — sprint range extended to 27–36 (added Sprints 35 audio detector and 36 EMIF/finalisation); Option C binding statement added at top

### Cost Impact

| Item | Two-model (rejected) | Option C (adopted) | Delta |
|---|---|---|---|
| Annual operational cost | £5,000–6,500 | £2,000–4,000 | −£3,000–2,500/year |
| Documentation burden | 2× model cards, 2× help system, 2× tier logic | 1× production card, 1× research artefact (separate lifecycle) | ~50% reduction |
| QA paths per release | 12 | 6 | 50% reduction |
| Contract breach exposure | Moderate | Low | Significant reduction |

**One-off costs to enable Option C:** £700–1,800 (D&O insurance £500–800/year + solicitor bundle £200–1,000 + filesystem migration £0). Funded via UnLtd Starting Up Award budget under "pre-trading governance and legal review" line items.

### Implementation Sequence

| Sprint | Implementation step |
|---|---|
| 27 (current) | Decision memo published; methodology doc updated; D&O insurance quotes; research form submissions; Fraunhofer AISEC outreach for MLAAD commercial permission |
| 28 | Filesystem migration to production/research segregation |
| 30 | Video test set assembly using research/video/ for FF++/Celeb-DF |
| 31 | Training script firewall implementation; solicitor engagement for OpenRAIL-M drafting |
| 32 | TRIED self-assessment cites Option C as governance commitment |
| 33 | Audio corpus download split per dataset licence |
| 35 (NEW) | Audio detector v1.1 production model trained on commercial-safe audio only |
| 36 (NEW) | EMIF application cites Option C as core methodology commitment |

### Approval

- **Decided by:** Paul Griffiths, Director, Juralabs Community Interest Company
- **Decision ID:** DEC-2026-04-07-001
- **Effective:** 7 April 2026
- **Next review:** Sprint 36 (August 2026) at EMIF application finalisation

---

## 7 April 2026 — Sprint 30 Corpus Expansion: 10,091 Images

### Corpus to 10,000 Target Met

**Added**
- **+2,600 training images** across three HuggingFace sources, closing both 5,000 targets in a single session at £0 API cost
- Authentic corpus: 3,287 → **5,087** (target 5,000, ✅ met)
- AI-generated corpus: 4,204 → **5,004** (target 5,000, ✅ met)
- Combined total: 7,491 → **10,091 / 10,000**

**New authentic sources**
- `flickr30k` (lmms-lab/flickr30k) — 1,300 photographs, diverse subjects and geography
- `flickr8k` (jxie/flickr8k) — 500 amateur and semi-professional photographs

**AI corpus extension**
- `elsa` (elsaEU/ELSA1M_track1) — +800 images with per-generator labelling captured from dataset `model` field (includes `stabilityai/stable-diffusion-2` and `elsa_multimodel` variants)
- Grok Aurora target (500) already met pre-session; no new generation required

### Crawler Infrastructure

**Added**
- `JURA_CORPUS_BASE` environment variable support in `scripts/agents/config.py` — crawlers now write to any configurable location (e.g. external USB drive) without touching the repo
- `download_flickr30k()` and `download_flickr8k()` in `scripts/agents/crawl_authentic_images.py` — replace the previous `openimages` and `unsplash` sources which were removed from the HuggingFace Hub in early 2026 (dataset-script deprecation). Backward-compatible aliases preserved for existing scripts
- `_existing_image_count()` and `_existing_hashes()` helpers — resume-safe behaviour: crawlers count existing files and skip duplicate content by SHA-256, so repeated invocations accumulate rather than overwrite
- Per-generator labelling in `crawl_ai_images.py` — when a dataset provides a `model` field (ELSA does), the generator slug is stored in each manifest entry for later per-generator performance evaluation

**New file**
- `scripts/agents/generate_grok_corpus.py` — xAI Grok Aurora corpus generation agent. Uses the `grok-2-image-1212` endpoint via the xAI API, configurable via `XAI_API_KEY` env var with a `XAI_BUDGET_CAP_USD` safety cap (default $50). Includes a 500-prompt set covering portraits (diverse demographics), news/documentary scenes, landscapes, architecture, products, and composite conflict zone imagery. Supports `--dry-run` mode for prompt inspection. Not executed this session — Grok Aurora target already met at 500 images from pre-session work.

**Fixed**
- Crawler overwrite bug: previous versions of `download_flickr30k` restarted filename numbering at `00000` on each invocation, silently overwriting files from earlier runs. Now uses `start_idx + downloaded` with collision-free indexing.

### Documentation

**Added**
- `docs/corpus-state-2026-04-07.md` — authoritative corpus inventory (filesystem-sourced, since stale manifests diverged significantly from reality). Records both morning pre-expansion state (7,491) and evening post-expansion state (10,091)
- Sprint 30 status note in `docs/sprint-plans/tried-compliance-roadmap.md` updated to reflect corpus target achievement

### Storage

- Corpus now lives on external Samsung USB drive at `/Volumes/Samsung USB/Training Data/corpus/training/` (60 GB drive, 43 GB free, 29% used). Configured via `JURA_CORPUS_BASE` env var.

### Cost

- **£0 total** — all sources are free HuggingFace streaming datasets. Original Sprint 30 cost estimate was £88–128 for paid APIs (Grok, Leonardo, Recraft, Ideogram, Flux Pro). None required.

### TRIED Pillar Impact

- **Pillar 1 (Real-World Adaptability)**: corpus diversity supports claim of "trained on 10,000+ real and AI images across 15+ generator families including Grok Aurora"
- **Pillar 4 (Fairness)**: 5,087 authentic images across COCO, Flickr30k, Flickr8k, ImageNet, CelebA, Google Photos, and camera DCIM provides the volume needed for meaningful demographic subgroup analysis in Sprint 29 bias audit

### Next Steps

- GBM retrain on 10,091-image corpus (Sprint 30 remaining work, S30-03)
- Model card updates with new corpus statistics (S30-05)
- Demographic bias audit (Sprint 29, depends on expanded corpus)

---

## 7 April 2026 — GBM v4 + UnivFD v8 Production

### GBM Deepfake Classifier: v4 Promoted

**Three retrains on 7 April 2026, culminating in v4 (commit 0857764).**

| Version | Corpus | AUC-ROC | FP Rate | Recall | Notes |
|---|---|---|---|---|---|
| v2 (baseline) | 9,658 | 0.9885 | 4.81% | 93.90% | Pre-session baseline |
| v3 | 10,721 | 0.9863 | 4.67% | 92.50% | Added COCO train + audited Wikimedia restored |
| v4 (production) | 10,709 | 0.9868 | 4.54% | 92.52% | Wikimedia re-audit removed 12 outliers |

**v4 production metrics**
- Corpus: 10,709 images (5,724 authentic + 4,985 AI), 14 generator families
- AUC-ROC: 0.9868 (5-fold cross-validation)
- Authentic FP rate: 4.54%
- AI detection rate (recall): 92.52%
- Calibrated threshold: 0.49 (FP 4.79%, recall 92.68%)
- File: `models/deepfake_classifier.joblib` (~1.2 MB)
- SHA-256: `2931f197cba6f376e85b1cbcfd584e6802f36e4fbf68ff00c83d61d4d655db18`

**Wikimedia re-audit (v3 → v4):** 12 outliers removed — cartoons, microscope slides, album artwork, studio composites, and underwater photography. These are non-photographic or controlled-lighting images that distorted the authentic decision boundary.

### UnivFD Linear Probe: v8 Promoted

| Version | Corpus | AUC-ROC | FP Rate | Recall |
|---|---|---|---|---|
| v6 | 6,009 | 0.9929 | 2.7% | 95.2% |
| v7 | 10,724 | 0.9909 | 4.91% | 96.03% |
| v8 (production) | 10,712 | 0.9911 | 5.01% | 96.01% |

**v8 production metrics**
- Corpus: 10,712 images (5,727 authentic + 4,985 AI)
- AUC-ROC: 0.9911 (5-fold cross-validation)
- Authentic FP rate: 5.01%
- AI detection rate (recall): 96.01%
- File: `models/univfd_probe.joblib` (4.8 KB)
- SHA-256: `d16fb22baf3981d62888e2458733c1a4c0743a5895470d82e9de76766f776908`

### MakerNote Authenticity Bonus (commit 476f265)

**Added** to `src-tauri/src/exif_anomaly.rs`: images carrying a `MakerNote` EXIF block receive a trust score bonus during EXIF anomaly analysis. MakerNote data is written by camera firmware and is structurally difficult to fabricate — its presence is a reliable indicator of a real camera capture. This improvement operates at inference time and benefits all existing verified assets without retraining.

Real-world camera FP improvement vs v2 baseline:
- `camera_consumer` (Pixel/iPhone/PXL): 15.6% → 8.81% (-6.79pp) — cleared 2× bias gate
- `camera_high_end` (DJI/DSC): 13.5% → 10.32% — still flagged, addressed in Sprint 29 Track 2

### Bias Check Findings (GBM v4)

Top FP sources (descending):
1. `wikimedia_photos`: 24.80% (n=254) — bulk wildlife/insect macro; addressed in Sprint 29 Track 2 with 200 iNaturalist photographs
2. `camera_high_end`: 10.32% (n=126) — still flagged
3. `camera_consumer`: 8.81% (n=295) — cleared

AI generators: all families pass at ≥67% recall. 100% recall: Grok Aurora, ELSA SD, Midjourney v6, SDXL, Gemini Imagen, ArtBench, HF AI. 91.4%: DALL-E 3. 75.8%: Civitai SFW. 67.6%: DiffusionDB (weakest — older SD v1.x outputs).

---

## 7 April 2026 — Sprint 28: Input Quality Assessment

### Input Quality Assessment Engine

**Added**
- `InputQualityAssessment` struct in `src-tauri/src/lib.rs` with eight fields: `jpegQualityEstimate` (0–100), `resolutionCategory` (low / standard / high), `width`, `height`, `isScreenshotLikely`, `isJpeg`, `hasGps`, `hasTimestamp`, and `degradedDetectors` (list of detector names whose reliability is reduced)
- Three Rust helper functions: `assess_input_quality()`, `estimate_jpeg_quality()`, `detect_screenshot()`
- `VerificationResult.input_quality: Option<InputQualityAssessment>` — quality data flows through the full verify pipeline
- Assessment runs after EXIF analysis, before sidecar calls (commit 11fecde)

### Per-Analysis Limitation Banners

**Added**
- `LimitationBanner.svelte` component (`ui/src/lib/components/`) — amber warning banners rendered in Expert View when input quality reduces detector reliability
- Five warning conditions: heavy JPEG compression (estimated quality < 40), low resolution (< 0.5 MP), screenshot detected, non-JPEG format, missing GPS/timestamp
- WCAG 2.2 AA compliant: `role="status"`, `aria-label`, full dark mode support

### Detector Applicability Indicators

**Added**
- `getDetectorApplicability()` helper function on the verify page
- Four status levels: Analysed (malachite), Limited (amber), N/A (grey), Unavailable (cinnabar)
- Applicability badges added to ELA, Noise, Copy-Move, and Deepfake detector headings in Expert View
- Derives status from `degradedDetectors` (backend) and `sidecarAvailable` (frontend state)
- `applicabilityBadge` Svelte snippet for consistent badge rendering across detectors

**Test counts**: 283 Rust tests, 375+ Python tests, 164 Playwright e2e tests, 231 SvelteKit files with 0 svelte-check errors, clippy + fmt clean.

---

## 6–7 April 2026 — C2PA Rebranding, UnivFD Retrain, Corpus Expansion

### C2PA Provenance Manifest Rebranding

**Changed**
- "C2PA Content Credentials" renamed to "C2PA provenance manifest" across 38 files — "Content Credentials" is Adobe's trademarked term, not a C2PA standard term
- Files updated: UI routes (verify, protect, monitor), Rust backend (`lib.rs`, `c2pa.rs`), docs (ARCHITECTURE.md, BRAND_GUIDELINES.md, user guides, help pages, pilot testing docs), PDF/ZIP export templates
- Commits: a7e4b1d (bulk rename) and b356485 (missed occurrences)
- Plane issue JTV-64 created for website copy update (Todo, medium priority, website + branding labels)

### UnivFD Probe Retrain

**Changed**
- Corpus expanded from 834 → 6,009 images (2,873 authentic + 3,136 AI)
- AUC-ROC improved: 0.9774 → 0.9929
- Authentic FP rate reduced: 28.7% → 2.7%
- AI detection rate: 95.2%
- Regularisation tuned: C=0.5 → C=1.0
- Wikimedia art/illustrations removed from authentic corpus (38% of that source were FPs due to stylised, non-photographic content)
- Training corpus relocated to external USB for local storage management

**Added (authentic sources)**
- 300 COCO natural scene images
- 828 Google Photos

**Added (AI sources)**
- 500 DiffusionDB (Stable Diffusion v1.x)
- 500 DALL-E 3
- 500 Civitai SFW (community Stable Diffusion models)
- 300 SDXL-Turbo
- 150 Midjourney v6
- 70 Gemini Imagen 4

### Corpus Generator Improvements

**Changed**
- `generate_ai_corpus.py`: conflict and political prompts replaced with neutral scene descriptions for Imagen safety filter compliance; Gemini Flash fallback removed — Imagen 4 only
- `train_univfd_probe.py`: `--C` regularisation parameter added (default 1.0)

**Added**
- `scripts/generate_local_sd.py` — new local Stable Diffusion image generator supporting SDXL-Turbo, SD 2.1, SD 1.5, and SSD-1B; generates images offline without cloud API calls

### TRIED Compliance Roadmap

**Changed**
- Roadmap updated with current detection metrics (FP 2.7%, AUC-ROC 0.9929)
- Sprint 30 marked ~15/22 points complete
- GPU cost section updated: M1 Mac training validated, no cloud GPU required

---

## Post-RC14 — UX Redesign, Detection Calibration, UnivFD Probe (3 Apr 2026)

### Two-Tier Verify Results

**Added**
- Simple View (default): large verdict card, analysis completeness indicator ("17/19 detectors"), 3-4 plain-English bullets, actionable next steps, editing vs AI distinction
- Trust score percentage moved to Expert View only (12/15 persona consensus)
- Export Report, Export Case, Report False Positive buttons in Simple View
- Verify session persistence via sessionStorage — results survive navigation to Help/Settings
- Tauri save dialog for exports (user chooses save location)

### Detector Rebalancing

**Changed**
- ELA weight 2.0 → 1.0 (biggest false positive source on multiply-compressed images)
- Shadow consistency and splice boundary removed from trust scoring (demoted to Expert View display-only)
- Regional amplification cap now requires segmented ELA + colour temperature (not 2-of-4)

### UnivFD Probe

**Added**
- Trained LogisticRegression probe on CLIP ViT-B/32 embeddings (4.8 KB)
- AUC-ROC 0.9774, AI detection rate 99.6% (498/500), trained on 834 images
- Probe auto-loaded by sidecar `clip_detector.py` — replaces useless zero-shot classification
- Authentic FP rate 28.7% — requires expanded corpus (backlogged)

### Detection Fixes

**Fixed**
- 128px minimum image size guard in deepfake detection — images below threshold return "too small for reliable analysis" instead of misleading scores (fixes 53.9% FP on CIFAR-10 32x32 thumbnails)
- Ollama model pull proxied through sidecar (`POST /ollama/pull`) — bypasses CSP for remote Ollama instances
- Cross-platform Reveal in Finder on Protect page (Windows `\` path separator)
- Release workflow `workflow_dispatch` trigger for manual re-runs

---

## RC10–RC14 — API Hardening, Detection Calibration, Dependency Audit (2–3 Apr 2026)

### REST API (Port 8300)

**Added**
- Batch verify endpoint: `POST /api/v1/verify/batch` — multipart upload, 20-file limit, per-file results with partial failure handling
- API key management: `create_api_key`, `list_api_keys`, `revoke_api_key` Tauri IPC commands + Settings UI section (tier-gated to Team/Enterprise)
- Settings UI: create key form with name + rate limit, one-time key display banner with copy button, key table with active/revoked badges, inline revoke confirmation
- 2 new API integration tests (batch verify + empty batch 400), total 15

### Methodology Versioning

**Added**
- `MethodologyRecord` struct with DB migration (schema v5)
- Methodology metadata wired into verify pipeline and VerificationResult
- Raw signal scores in PDF trust reports: 7 core + 4 regional detector float values with thresholds and Clean/Flagged status
- Methodology metadata block in PDF (mode, version, formula, detectors run)

### AI Detection Calibration

**Changed**
- Authentic verdict threshold raised from `< 0.30` to `< 0.20` in sidecar deepfake.py — scores 0.20–0.30 now classified as "inconclusive" instead of "authentic"
- Inconclusive trust ceiling lowered from 0.60 to 0.55 in `compute_trust` — prevents borderline AI images from exceeding the 0.70 trust constraint
- Closes 7.3% false-negative gap discovered via 500-image deep corpus review (3 photorealistic AI images — camping tents, t-shirt with garbled text, pendant necklace — were scoring 0.74–0.76)

### Watermark Detection

**Fixed**
- False positive watermark detection on AI-generated images: new `_assess_watermark_confidence()` checks printable ASCII ratio, Unicode replacement char ratio, and byte diversity to distinguish genuine payloads from frequency-domain noise
- Gemini AI image: confidence dropped from 0.8 (false positive) to 0.12 (correctly rejected)

### Sidecar Dependency Audit (13 Issues)

**Fixed**
- `python-multipart` added to `requirements-ci.txt` — the CI-safe file actually used by PyInstaller builds (was only in requirements.txt/lock, causing Windows sidecar crash: "Form data requires python-multipart")
- 9 service modules from Sprints 21–26 added to PyInstaller `hiddenimports`: noise_visualisation, clahe, frequency_visualisation, jpeg_grid, weather_check, diffusion_artefacts, seasonal_indicators, roi_analysis, gan_fingerprint
- `scipy.ndimage` added to hiddenimports (used by gan_fingerprint.py)
- `certifi` added to hiddenimports + spec datas (SSL CA bundle for weather_check HTTPS)
- `h11`, `starlette`, `anyio`, `sniffio`, `certifi` pinned in requirements-ci.txt
- `collect_all(chromadb)` / `collect_all(sentence_transformers)` wrapped in try/except to prevent build abort when optional deps not installed
- GAN fingerprint endpoint routing bug fixed: `@router.post("/forensics/gan-fingerprint")` → `@router.post("/gan-fingerprint")` (was double-prefixed, unreachable)

### Setup Wizard

**Fixed**
- Ollama model pull reads configured URL from localStorage (`jura-ollama-url`) instead of hardcoding `127.0.0.1:11434` — supports remote Ollama instances

### Corpus Training Agents

**Added**
- `scripts/agents/` — 8-module CLI agent system for automated corpus management:
  - `crawl_ai_images.py`: ELSA 1M (multi-model SD/DALL-E/MJ), CIFAR-10 baseline
  - `crawl_authentic_images.py`: CIFAR-10 test, Wikimedia Commons Featured, Open Images V7
  - `apply_protections.py`: fingerprint, watermark, C2PA sign, combined (all four) via REST API
  - `verify_corpus.py`: full verify pipeline across standard/deep/archival modes
  - `validate_constraint.py`: enforces AI+protection trust < 0.70 with per-protection breakdown
  - `deep_review.py`: end-to-end orchestrator (crawl → protect → verify × 3 modes → validate)
  - `run_all.py`: lightweight orchestrator with `--quick` mode
  - `api_client.py` + `config.py`: shared REST API client and configuration

### CI/CD

**Changed**
- Linux release build disabled (not under active testing, saves ~20 min CI per release)

### Test Counts

- **Rust**: 283 tests passing (+11 API integration, methodology), clippy clean
- **Python**: 375+ tests, 61 sidecar tests verified after dependency changes
- **Frontend**: 0 svelte-check errors across 223 files

---

## Sprint 26 — Berkeley Protocol, GAN Fingerprint, Annotations (29 Mar 2026)

### Legal Evidence Reporting

**Added**
- Berkeley Protocol PDF report template: 7 structured sections (evidence documentation header, capture environment, evidence integrity with SHA-256, methodology disclosure with 5 known limitations, formal analyst declaration with 4 legal statements, tool version appendix)
- Report format selector in analyst declaration modal: "Standard Trust Report" / "Berkeley Protocol (Legal Evidence)"
- `ReportFormat` type exported from pdf.ts

### AI-Specific Detection

**Added**
- GAN fingerprint visualisation: `POST /forensics/gan-fingerprint` — multi-channel FFT with 1/f model subtraction, peak detection, ring/checkerboard pattern analysis. Model attribution (StyleGAN2, ProGAN, StyleGAN3) with confidence scoring. Annotated spectrum (INFERNO) and residual spectrum (HOT) as base64 PNG. 8 Python tests

### Annotation Layer

**Added**
- SQLite `annotations` table (schema v3 migration) with `annotation_id`, `verification_id`, `asset_id`, `annotation_type`, `data_json`, `created_at`. Two indexes
- 5 CRUD functions: `insert_annotation`, `get_annotations_for_asset`, `get_annotations_for_verification`, `delete_annotation`, `delete_annotations_for_asset`. 6 Rust tests
- 3 Tauri commands: `save_annotation`, `get_annotations`, `delete_annotation`
- Interactive canvas overlay: arrow, circle, rectangle, text tools with 5 brand-palette colour swatches (lapis, cinnabar, malachite, amber, quartz)
- SVG overlay with per-annotation arrowhead markers, ghost preview during drag, delete on hover
- Pointer capture gesture handlers with 8px minimum drag guard
- Natural-pixel coordinate system (annotations survive window resize)
- Full WCAG 2.2 AA: `aria-pressed` toggles, `focus-visible` rings, `aria-live` hints, keyboard-operable delete

### Test Counts

- **Rust**: 272 tests passing (+6 annotations), clippy clean
- **Python**: 375+ tests (+8 GAN fingerprint)
- **Frontend**: 0 svelte-check errors across 223 files

---

## Sprint 25 — Pilot Polish (29 Mar 2026)

### Geolocation Intelligence

**Added**
- Enhanced GPS location panel: DMS + decimal degrees display, "Copy coordinates" with clipboard feedback, OpenStreetMap + Google Earth buttons opening system browser
- C2PA provenance chain timeline in verify page with single-claim display and multi-claim explanation

### Asset Management

**Added**
- Sort dropdown on Protect page filter bar (date, name, size — ascending/descending)
- Wired to existing sortKey/sortDir state for consistency with column header sorting

### Trust Report Enhancements

**Added**
- Video frame thumbnails in PDF: up to 6 frames in 3x2 grid with timestamps, graceful fallback on decode failure
- Schema migration v1→v2 for sha256_hash column on existing databases

### Test Counts

- **Rust**: 266 tests, clippy clean
- **Python**: 367+ tests
- **Frontend**: 0 svelte-check errors across 223 files

---

## Sprint 24 — ROI Selection, Shadow Time, Diffusion Detection (29 Mar 2026)

### Geolocation & Temporal Investigation

**Added**
- Shadow-based time-of-day estimation: inverse sun angle calculation — given GPS, date, and observed shadow azimuth, returns up to 2 candidate UTC times. `estimate_shadow_time` Tauri command. 5 new Rust tests
- Geolocation & Temporal panel on verify page: date/time picker (auto-populated from EXIF), sun position display (azimuth, elevation, solar noon, day length), shadow time estimate with compass label
- Weather cross-reference UI: opt-in amber disclosure panel, explicit consent gate, temperature/precipitation/wind display with source attribution

### AI-Specific Detection

**Added**
- Diffusion model artefact detection: `POST /forensics/diffusion-artefacts` — texture smoothness (16x16 patch std), VAE banding (gradient histogram peakiness), resolution fingerprint (known AI generation sizes), combined score (50/30/20 weighting). 7 Python tests
- On-demand "Check Diffusion Artefacts" button on verify page with colour-coded verdict and experimental disclaimer

### Seasonal Analysis

**Added**
- Seasonal indicators: `POST /forensics/seasonal-indicators` — greenness index (HSV), snow coverage (bright low-sat), warmth index (LAB b-channel), brown/autumn detection. Season estimation with confidence. 7 Python tests
- On-demand "Seasonal Analysis" button on verify page with season badge, confidence, and indicator pills

### Interactive Investigation

**Added**
- Region-of-interest (ROI) selection: `POST /forensics/roi-analysis` — click-and-drag rectangle on image preview with SVG overlay (masked dimming, dashed lapis border, corner handles). Analyses selected region for noise, ELA, frequency energy, texture complexity. 7 Python tests
- ROI results panel with traffic-light colouring per metric

### TypeScript Types & API

**Added**
- `TimeEstimate`, `RoiAnalysisResult`, `DiffusionArtefactsResult`, `SeasonalIndicatorsResult` interfaces
- `estimateShadowTime()`, `checkHistoricalWeather()`, `analyseSeasonalIndicators()`, `analyseDiffusionArtefacts()`, `analyseRoi()` API wrappers

### Test Counts

- **Rust**: 266 tests passing (+5), clippy clean, fmt clean
- **Python**: 367+ tests (+21 diffusion/seasonal/ROI)
- **Frontend**: 0 svelte-check errors across 223 files

---

## Sprint 22-23 — Frequency Analysis, Sun Angle, Batch Verify (28 Mar 2026)

### Enhanced Visual Inspection (Sprint 22)

**Added**
- Frequency domain visualisation: `POST /forensics/frequency-visualisation` — 2D FFT magnitude spectrum (INFERNO colourmap), 8x8 block-averaged DCT heatmap (VIRIDIS), JPEG grid peak detection, dominant frequency. 8 Python tests
- JPEG quantisation grid visualisation: `POST /forensics/jpeg-grid` — block boundary artefact heatmap (HOT colourmap), Q-table extraction from JPEG headers, grid consistency score. 7 Python tests
- Side-by-side image comparison mode: load a second image alongside verification result for visual diff. Tauri dialog or browser file picker. "Original" / "Comparison" overlay labels
- Raw scores as default view preference: persistent "Technical View" toggle (localStorage). When active, signal strip shows inline percentages with thresholds

### Chain of Custody (Sprint 22)

**Added**
- Input file SHA-256 hash at import: computed via `sha2::Sha256` on file import and verification. Stored in `assets.sha256_hash` column (with migration). Returned as `inputSha256` on `VerificationResult`. 4 new Rust tests

### Geolocation & Temporal (Sprint 23)

**Added**
- NOAA solar position calculator: pure Rust trigonometry (`sun_position.rs`). `calculate_sun_position` Tauri command — azimuth, elevation, solar noon, day length from lat/lon/date/time. Validated against London summer noon, Sydney winter, equator equinox, Arctic midnight sun. 7 new Rust tests
- Weather cross-reference: `POST /forensics/weather-check` — queries Open-Meteo historical weather API (free, no key). Temperature, precipitation, wind, WMO weather codes. Opt-in network feature with clear disclosure. 6 Python tests (all HTTP mocked)
- Batch VERIFY queue: multi-file verification with progress bar, mode selector, cancel, summary table with sortable results

### TypeScript Types & API

**Added**
- `SolarPosition`, `NoiseVisualisationResult`, `ClaheResult`, `FrequencyVisualisationResult`, `JpegGridResult`, `WeatherCheckResult` interfaces
- `inputSha256` field on `VerificationResult`
- `calculateSunPosition()` API wrapper

### Test Counts

- **Rust**: 261 tests passing (+11), clippy clean, fmt clean
- **Python**: 346+ tests (+21 frequency/JPEG/weather)
- **Frontend**: 0 svelte-check errors across 223 files

---

## Sprint 21 — "See More" Foundation Investigation Tools (28 Mar 2026)

### Enhanced Visual Inspection

**Added**
- Colour channel separation: R, G, B individual channels + R-G, R-B, G-B difference channels via Canvas pixel manipulation. Toggle buttons in inspection toolbar with `aria-pressed` and screen reader announcements
- Noise pattern visualisation: `POST /forensics/noise-visualisation` endpoint exposing noise residual (greyscale) and block-wise variance heatmap (JET colourmap) as base64 PNG. 8 Python tests
- Per-channel CLAHE (Contrast-Limited Adaptive Histogram Equalisation): `POST /forensics/clahe` endpoint with configurable clip limit (0.5-10.0). LAB colour space L-channel enhancement. 9 Python tests

### Metadata Intelligence

**Added**
- EXIF thumbnail consistency check: extracts embedded JPEG thumbnail via `Tag::JPEGInterchangeFormat`, compares against main image via pHash. `ThumbnailCheck` struct with `hasThumbnail`, `hammingDistance`, `mismatch` (threshold >10). 9 new Rust tests
- GPS coordinate → OpenStreetMap link: `gpsLatitude`/`gpsLongitude` fields on `ExifAnalysis`, DMS formatting, "View on map" button opening system browser. 5 new Rust tests

### AI Origin Detection

**Added**
- Unified AI Origin Detection panel consolidating C2PA AI declaration, deepfake ensemble score, and watermark extraction into single section with status badges

### Investigation Workflow

**Added**
- Reverse image search: Bing Visual Search added (4th engine). All links now use `openExternal()` via `tauri-plugin-shell` to open system browser instead of Tauri webview
- Analyst notes persistence: textarea increased to 2000 chars, saved to localStorage automatically, no longer cleared after PDF export
- Methodology + model versions in PDF: GBM classifier version, CLIP model status, Jura Trace version (v0.9.0) in methodology metadata block

### Test Counts

- **Rust**: 250 tests passing (+14), clippy clean, fmt clean
- **Python**: 325+ tests (+17 noise viz + CLAHE)
- **Frontend**: 0 svelte-check errors across 223 files

---

## Sprint 17 — Quality Floor & Deployment Readiness (28 Mar 2026)

### IPC Error Architecture (S17-A1/A2/A3)

**Changed**
- `AppError::serialize` now returns structured JSON `{ "code": "Sidecar", "message": "..." }` instead of plain string — frontend can branch on error type without string-sniffing
- Verify page `setError()` uses `parseAppError()` to match on structured `code` field first, with graceful fallback for legacy string errors
- Tiered error banner copy: dev mode shows technical `uvicorn` command, production shows user-friendly restart message
- Error-specific messaging: Sidecar, FileSystem, Database, C2pa, Validation, Internal each have tailored user-facing copy
- Test hooks (`__juraSetVerifyResult`, `__juraSetVerifyError`) gated behind `import.meta.env.DEV` — removed from production builds
- `data-testid="error-banner"` and `data-error-code` attributes for Playwright assertions decoupled from copy strings
- 2 new Rust tests: `serialize_returns_structured_object`, `serialize_all_variants_have_correct_code`

### Database & Deployment (S17-B1/B2)

**Added**
- `resolve_db_path()` supports 3-level priority: `JURA_DB_PATH` env var > `config.json` `db_path` key > default `app_data_dir`
- Settings page shows active database path with "Change location..." button (atomic copy to new path)
- `docs/DEPLOYMENT.md` — comprehensive deployment guide covering system requirements, database location, sidecar authentication, FFmpeg, Ollama, speech transcription, network configuration, institutional deployment notes

### Video Deepfake Quality (S17-C1)

**Changed**
- `deduplicate_frames()` now uses rolling buffer (default size 5) instead of single-frame comparison — catches cyclical video duplicates
- 12 rolling buffer tests including cyclical detection, buffer size limits, and performance assertions

### Privacy & UX (S17-D1/D2)

**Added**
- Source protection privacy warning in Investigate Further panel — amber banner advising caution with reverse image search for sensitive/unpublished material
- Batch watermark and batch sign ETA now uses windowed rolling average (last 8 files) for more accurate estimates with mixed file sizes

### Platform Installer Prerequisite (S17-E1)

**Added**
- `sidecar/jura-sidecar.spec` — PyInstaller spec file for frozen sidecar binary
- `docs/pyinstaller-spike-findings.md` — spike results documenting hidden imports, data files, optional dependency exclusions (torch, open_clip), estimated binary size

### Advanced Investigation Vision

**Added**
- `docs/esper-machine-vision.md` — 556-line Blade Runner Esper Machine-inspired vision document for next-generation image investigation tools (geolocation, temporal analysis, enhanced inspection, metadata intelligence, context/provenance, AI-specific detection)
- `docs/personas/investigation-workflows.md` — detailed investigation workflows for 4 personas (journalist, BBC Verify, OSINT fact-checker, human rights documenter)

### Test Counts

- **Rust**: 236 tests passing, clippy clean
- **Python**: 308+ tests (24 video deepfake including rolling buffer)
- **Frontend**: 0 svelte-check errors across 223 files
- **Playwright**: 110+ e2e tests

---

## Architecture Decisions (27 Mar 2026)

**Assessed** proposed architecture changes against existing codebase. Key decisions:
- **REJECT** pure Rust analysis pipeline (110-160 pts risk, 308 Python tests at stake)
- **REJECT** eliminating Tauri desktop shell (27 Svelte components, 110 Playwright tests)
- **REJECT** removing Team tier (strands 3 personas in £4.7K→£18K gap)
- **REJECT** expanding v1.0 scope (would delay 6-9 sprints)
- **ADOPT** shared Rust library crate extraction in Phase A Sprint 23
- **ADOPT** 4 new consumer personas (Ravi, Sarah M, Jordan, Priya — 14 total)
- **ADOPT** `plain_english_summary` response field (Phase A, 2-3 pts)
- **DEFER** JWT/Redis/CORS to Jura Check decision gate (June 2026)
- **DEFER** async video jobs + SSE to Jura Check integration
- **Architecture**: two products (Jura Trace desktop + Jura Check hosted API) sharing a common Rust library crate + Python sidecar

Full analysis: `docs/api-architecture-assessment.md`

---

## Sprint 20 — Persona Conversion & Tier Management (25 Mar 2026)

### Licence Tier Management

**Added**
- `LicenceTier` enum in Rust (Community/Professional/Team/Enterprise) persisted in `config.json`
- `get_licence_tier` / `set_licence_tier` Tauri commands
- Settings page "Your Plan" section with coloured tier badge and pilot-mode dropdown
- Non-blocking tier hints on Verify page PDF export and Investigate Further panel
- 5 new Rust tests for tier persistence and round-trip

### Analyst Declaration in PDF Export

**Added**
- 4-field modal before PDF export: analyst name, organisation, case reference, date of analysis
- Analyst name and organisation persisted in localStorage across sessions
- `ReportContext` interface in `pdf.ts` — header block rendered in PDF with declaration fields
- Tier hint for Community users: "Professional plan includes branded reports" (non-blocking)

### Raw Signal Scores in PDF Report

**Added**
- "Forensic Signal Scores" section: 7 core detectors + 4 regional with actual float values, thresholds, Clean/Flagged/Not Run status
- "Methodology" metadata block: analysis mode, pipeline version, trust formula, C2PA adjustment, detectors run, analysis date
- Extended methodology disclosure: 7 additional detector descriptions (NPR, CA, JPEG Ghost, segmented ELA, shadow consistency, colour temperature, splice boundary)
- Colour-coded status (red=Flagged, green=Clean) with text labels (WCAG 1.4.1 compliant)

### Metadata Preservation Statement

**Added**
- Malachite confirmation panel on Protect page after C2PA signing: "Existing file metadata (EXIF, IPTC, XMP) has been preserved"
- Only shows for the just-signed asset, clears on row change

### Institutional Procurement Documents

**Added**
- Information Security Summary (`docs/information-security-summary.md`): 13-section procurement document covering architecture, data flows, security measures, compliance, incident response
- DPIA template (`docs/compliance/dpia-template.md`): 80% pre-filled Data Protection Impact Assessment following ICO guidance, 9 pre-assessed risks, data inventory
- Berkeley Protocol alignment page (`ui/src/routes/help/berkeley-protocol/`): maps Jura Trace to UN/Berkeley Protocol on Digital Open Source Investigations

### Agent Architecture Overhaul

**Added**
- 6 new professional personas: Niamh (solicitor), Elena (insurance), James (BBC Verify), Amara (human rights), Richard (corporate comms), David (Jura Check consumer)
- 3 new agents: ml-data-scientist, api-engineer, grant-writer
- Stale context fixed in 6 existing agents
- Agent memory consolidated (7 agents with duplicate locations)

### Production Error Fix

**Fixed**
- uvicorn command string removed from production sidecar error path — now uses `import.meta.env.DEV` tiering
- Setup wizard: invalid `w-4.5` Tailwind class → `w-4` (fixed giant X rendering)
- Setup wizard: Re-check button for FFmpeg detection
- Capabilities struct: added 13 missing fields (video_metadata, transcription, etc.) — fixes false "FFmpeg not installed"

### Test Counts
- Rust: 235 tests, clippy + fmt clean
- Python: 308+ tests
- Playwright: 160+ tests
- SvelteKit: 206 files, 0 svelte-check errors

---

## Sprint 19 — Release Candidate (25 Mar 2026, v0.9.0-rc.1)

### Accessibility

**Fixed**
- WCAG 2.2 AA audit: 14 issues fixed across verify, protect, settings, onboarding, and root layout
- Verify page: tabpanel ARIA (`role="tabpanel"`, `aria-controls`, `aria-labelledby`), URL input label, sidecar status badge `role="status"`, focus-visible rings on expand buttons, reverse image search link targets (44px), analyst note character counter `aria-live`
- Protect page: drop zone `disabled` state (was `aria-disabled` only), batch errors toggle `aria-controls` + focus ring
- Onboarding: dialog headings h1 → h2 (avoids duplicate h1 per page)
- Root layout + Settings: external links now announce "(opens in new tab)" for screen readers
- Footer version test decoupled from hardcoded string (regex match)

### C2PA AI Detection

**Fixed**
- C2PA manifests declaring AI generation (e.g. Google Gemini `trainedAlgorithmicMedia`) now correctly penalise trust instead of rewarding it
- New `detect_ai_from_assertions()` scans C2PA assertions for IPTC `trainedAlgorithmicMedia` digitalSourceType, AI keywords ("generative ai", "ai-generated"), and known generator names (gemini, dall-e, sora, firefly, flux, etc.)
- `ai_generator` field now populated from both `claim_generator` strings AND assertion content
- Trust penalty: -0.25 for images/video (was +0.10 bonus), 0.10 for documents (was 0.82)
- Verify page: amber provenance banner when C2PA confirms AI generation — "This content carries a valid, signed C2PA provenance record which confirms it was created using AI generation"

### Help System

**Added**
- Monitor help guide: 8 sections (dashboard, audit trail, hash chain integrity, AI training limitation, URL watchlist preview, best practices)
- Settings help guide: 5 sections (Ollama configuration, deployment profiles, service status, database location, about)

### MONITOR Infrastructure

**Added**
- `monitor_urls` and `monitor_events` tables wired into `db.rs init_schema()`
- `MonitorUrl` and `MonitorEvent` Rust structs with serde camelCase
- 5 CRUD functions: `add_monitor_url`, `remove_monitor_url`, `list_monitor_urls`, `get_monitor_events`, `update_case_status`
- Partial index on `monitor_events(case_status)` for alert inbox performance

### RAG Knowledge Base

**Added**
- Expanded 4 existing knowledge base files (~2x content each): AI generators, C2PA provenance, misinformation patterns, image forensics
- 2 new domain files: `video_forensics.txt`, `digital_rights_and_cultural_heritage.txt`
- Total: 314 lines across 6 documents, ~150 passages (was 158 lines, 4 docs, ~79 passages)

### Security

**Fixed**
- OWASP self-audit: 4 HIGH (path canonicalisation in c2pa/watermark/video commands, shell permissions removed), 4 MEDIUM (CSP hardened, set_db_path extension allowlist, transcription size limit, audit timestamp precision), 2 LOW (audit chain command exposed, fs write scope narrowed)
- Report: `docs/security-pen-test-s19.md`

### File Handling

**Added**
- Corrupt/truncated file guards: zero-length and <12 byte checks in verify and import pipelines with `AppError::Validation` messages
- 3 new Rust tests for empty/tiny file rejection

### Video UX

**Added**
- Video analysis progress: phase labels ("Extracting frames..." → "Running deepfake detection..."), estimated time by mode, soft cancel button with Escape key support — video-only enhancement

### First-Launch Setup Wizard

**Added**
- 5-step `SetupWizard.svelte` component: sidecar health check (3s timeout, auto-advance), FFmpeg status with platform-specific install hints, Whisper model info (auto-downloads on first use), Ollama download button (opens ollama.com), ready summary with service availability checklist
- Runs after onboarding intro, persisted via `localStorage('jura-setup-complete')`
- Wired into `+layout.svelte` with chain: onboarding → setup wizard → app

### External Documentation

**Added**
- User guides: `docs/user-guide/getting-started.md`, `protect-guide.md`, `verify-guide.md`
- Quick Start cards: 1-page per platform (macOS, Windows, Linux)
- SHA-256 download verification guide + `SHA256SUMS.txt.template`
- Windows IT deployment appendix: MSIEXEC silent install, Intune, GPO firewall, PowerShell FFmpeg
- Automated SHA-256 checksum generation in GitHub Actions release workflow

### Strategic & Business

**Added**
- Strategic pivot assessment: verification-first positioning, post-v1.0 Phase A/B roadmap
- Tier structure decision: Community (free) / Professional (£199/yr) / Team (£79/seat/mo) / Enterprise (£6K+/yr) with geological internal codenames
- Tier comparison wiki for Plane
- Enterprise AI API analysis: BYOK model (Mistral/Claude/OpenAI) defensible for Enterprise tier
- Deployment experience design: FFmpeg bundling, Ollama button, model downloads, setup wizard
- Phase A plan: FP reduction + API wrapper + reports + versioning (4 sprints, July-August 2026)

### MONITOR URL Watchlist

**Added**
- 5 Tauri IPC commands: `add_monitor_url`, `remove_monitor_url`, `list_monitor_urls`, `get_monitor_events`, `update_monitor_case_status`
- `MonitorUrl` and `MonitorEvent` TypeScript interfaces + API wrappers
- Monitor tab URL watchlist UI: add URL form (URL + label + frequency selector), URL list with status badges (ok/changed/missing/error), expandable event rows, case management (investigate/resolve/dismiss)

### Auto-Updater

**Added**
- `tauri-plugin-updater` v2 wired into builder chain
- `latest.json` update manifest generation in release workflow
- "Check for Updates" button in Settings About section
- `updater:default` capability permission

### RC Preparation

**Changed**
- Version bumped to 0.9.0 across `tauri.conf.json`, `Cargo.toml`, `package.json`
- `cargo fmt` applied (28 diffs in c2pa.rs, db.rs, lib.rs)

**Fixed**
- Video deepfake Playwright tests: replaced 500ms fixed wait with `waitForFunction` for test hook availability
- Audit log hash chain: ordering changed from `created_at + log_id` to `rowid` — fixes chain verification when rapid inserts share the same millisecond timestamp

### Deepfake Classifier Retrained

**Fixed**
- FP rate reduced from 14% to 0% by eliminating format confound — classifier had learned JPEG=authentic, PNG=AI because all authentic training images were JPEGs
- Corpus expanded 548 → 709 images (390 authentic including PNGs + 319 AI-generated including PNGs)
- AUC-ROC: 1.0000 (was 0.9978)
- Held-out 20% test set: 0% FP, 0% FN at threshold 0.50
- Phase A PV-A1 target (<5% FP) achieved ahead of July 2026 schedule

### Security Remediation (Final)

**Fixed**
- LOW-1: 7 commands migrated from `map_err(e.to_string())` to `AppError` — no raw OS errors cross IPC boundary
- LOW-3: `import_files` now canonicalises paths before database storage
- LOW-4: `case_notes` capped at 10,000 bytes
- LOW-6: Production builds auto-generate 256-bit sidecar API key if `JURA_SIDECAR_KEY` not set
- MEDIUM-5: 8 Python packages pinned to exact versions
- **All pen test items now FIXED — 0 open**

### Test Counts
- Rust: 230 tests, clippy + fmt clean
- Python: 308+ tests
- Playwright: 160+ tests
- SvelteKit: 204 files, 0 svelte-check errors

---

## Sprint 18 — Platform Installers & Deployment Readiness (24 Mar 2026)

### Platform Build Infrastructure

**Added**
- Platform icon generation: `.icns` (macOS), `.ico` (Windows), full PNG set; `tauri.conf.json` `bundle.icon` updated
- macOS `Entitlements.plist`: `network.client`, `files.user-selected.read-write`, `allow-unsigned-executable-memory`, `disable-library-validation`; `minimumSystemVersion: "13.0"`
- macOS DMG builds successfully: 15 MB unsigned, `Jura Trace_0.4.0_aarch64.dmg`
- Linux bundle config: `category: "Utility"`, deb depends (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`)

### Sidecar Auto-Launch (S18-A2)

**Added**
- Tauri sidecar plugin integration: `externalBin` config in `tauri.conf.json`, `shell:allow-execute` and `shell:allow-spawn` in capabilities
- Sidecar lifecycle management in `lib.rs`: spawn on app start (release builds only), exponential-backoff health polling (up to 10 attempts, ~10s), clean process kill on `RunEvent::Exit`
- `AppState.sidecar_process` field for lifecycle tracking
- Platform-specific placeholder stubs in `src-tauri/binaries/` (replaced by CI with real PyInstaller output)
- Builder pattern changed from `.run(ctx)` to `.build(ctx).run(callback)` for exit event handling

### PyInstaller Cross-Platform (S18-A1)

**Fixed**
- `deepfake.py` model path: `JURA_MODELS_DIR` env var override for frozen PyInstaller context, `__file__`-relative fallback for dev mode
- `jura-sidecar.spec`: auto-detect `target_arch` via `platform.machine()`, platform-conditional UPX excludes (`.dylib`/`.dll`/`.so`), `codesign_identity=None` placeholder

### Frontend Cross-Platform Fixes

**Fixed**
- Linux font fallback: added `'DejaVu Serif', 'Noto Serif'` to Tailwind `font-heading` config and `app.css`; removed 38 inline `font-family` declarations, replaced with CSS class
- Settings path separator: replaced `selected.includes('/')` heuristic with `@tauri-apps/api/path` `join` for cross-platform correctness

### Release Infrastructure (S18-B1)

**Added**
- GitHub Actions release workflow: 4-platform matrix (macOS ARM, macOS Intel, Windows, Linux) with PyInstaller sidecar build per platform
- Changelog extraction from `CHANGELOG.md` for release notes
- Pip cache per platform, `patchelf` for Linux AppImage, 90-min timeout
- Code signing env vars commented out (placeholder for Sprint 19)

### MONITOR Preparation

**Added**
- AI training detection disclaimer: permanent lapis info banner on Monitor tab — "Content monitoring cannot detect whether your content has been used to train AI models"
- MONITOR SQLite schema design: `monitor_urls` and `monitor_events` tables with case management (`new`/`investigating`/`resolved`/`escalated`/`dismissed`), partial indexes, denormalised last-status; migration at `src-tauri/migrations/003_monitor_tables.sql`

### Documentation

**Added**
- macOS unsigned install guide (`docs/install-guides/macos-unsigned.md`): 3 Gatekeeper bypass methods, Sequoia workaround
- Windows unsigned install guide (`docs/install-guides/windows-unsigned.md`): SmartScreen, Firewall, WebView2, enterprise Group Policy
- Linux requirements guide (`docs/install-guides/linux-requirements.md`): AppImage/deb/rpm, WebKitGTK, font rendering, Wayland
- Pilot testing script (`docs/pilot-testing/test-script.md`): 30-minute structured session, 4 persona variants
- Linux smoke test checklist (`docs/pilot-testing/linux-smoke-test.md`)
- MONITOR schema design document (`docs/monitor-schema-design.md`)
- Feature scoping: online monitoring 3-layer architecture + paid tier structure (`docs/feature-scoping/`)
- Sprint 18 plan (`docs/sprint-plans/sprint-18-plan.md`)

### Test Counts
- Rust: 211 tests, clippy clean
- Python: 308 tests (+47 deepfake passed with path fix)
- Playwright: 164 tests
- SvelteKit: 201 files, 0 svelte-check errors

---

## Sprint 17 — Quality Floor & Deployment Readiness (24 Mar 2026)

### IPC Error Architecture (S17-A1, A2, A3)

**Added**
- `AppErrorResponse` TypeScript interface in `types.ts`: `{ code: 'Database' | 'FileSystem' | 'Sidecar' | 'Validation' | 'C2pa' | 'Internal', message: string }`
- `parseAppError()` in `api.ts`: normalises structured `AppError` and plain string errors into `{ code, message }`
- `setError()` on verify page upgraded to match on `AppError.code` first, string-sniff fallback for unmigrated commands
- Tiered error banner messages: dev mode shows technical details (uvicorn command), production shows user-friendly restart instructions
- `data-testid="error-banner"` and `data-error-code={errorType}` attributes on error banner
- `__juraSetVerifyResult` and `__juraSetVerifyError` test hooks gated behind `import.meta.env.DEV` (security: removed from production builds)
- Playwright error tests migrated from `toContainText` copy matching to `toHaveAttribute('data-error-code', type)`

### Database Path Configurability (S17-B1)

**Added**
- Three-source priority resolution: `JURA_DB_PATH` env var > `config.json` `db_path` key > default `app_data_dir/jura_archive.db`
- `AppConfig` struct with `read_app_config()` / `write_app_config()` helpers
- `dir_is_writable()` probe-file check (cross-platform)
- `get_db_path` and `set_db_path` Tauri commands with atomic copy + SQLite integrity check
- `AppState.db_path` field for runtime path tracking
- Settings page: "Database Location" section with folder picker, progress spinner, success/error feedback
- `getDbPath()` and `setDbPath()` IPC wrappers in `api.ts`
- 8 new Rust unit tests for config round-trip, writable check, priority logic

### Video Deepfake Quality (S17-C1)

**Added**
- Rolling buffer frame deduplication (`deduplicate_frames()`, `buffer_size=5`) catches cyclical video repeats
- Normalised correlation similarity metric for frame comparison
- 12 new Python tests: cyclical dedup, buffer size limits, threshold sensitivity, performance (<50ms for 40 frames)

### Privacy & UX (S17-D1)

**Added**
- Source protection privacy warning in Investigate Further panel: cautions about sharing URLs with third-party search services

### PDF Trust Scoring

**Fixed**
- PDFs no longer always score 50% — `document_trust()` helper: C2PA valid = 0.82, C2PA invalid = 0.25, no C2PA = 0.50
- Limited-analysis info banner on verify page for document content types

### In-App Help Documentation System

**Added**
- `/help` route with responsive sidebar navigation (desktop sidebar, mobile tab strip)
- `HelpSidebar` component with grouped sections, `aria-current="page"` active state
- Help index page with topic card grid
- Protect guide: C2PA signing, watermarking (3 strength levels), batch, asset management, best practices
- Verify guide: investigation modes, trust scores, verdicts, video/audio, exports, document analysis
- Methodology transparency page: all 16 detectors explained with `<details>` disclosures, trust scoring formula, signal weighting, honest limitations
- Glossary: 34 terms A-Z with sticky alphabet jump bar and deep-link anchors
- Persona usage guides: museum staff, journalists, content creators, researchers — recommended workflows, modes, tips
- Monitor and Settings stub pages
- `ContextualHelpLink` component (`?` icon) added to verify (modes, trust score), protect (watermark), monitor pages
- Help link added to main navigation
- 30 new Playwright tests for contextual help links

### Deployment & Documentation (S17-B2, E1)

**Changed**
- `DEPLOYMENT.md` updated for Phase 3: sidecar API key auth, database path configurability, speech transcription setup, Windows/Linux status → Sprint 18

**Added**
- PyInstaller sidecar bundling spike: GO for Sprint 18, 315 MB binary (with CLIP/torch), all core endpoints work on macOS arm64, `jura-sidecar.spec` produced, 2 path-resolution fixes documented
- Feature scoping document: online content monitoring (3-layer architecture), in-app help system, paid tier structure (Flint/Stratum/Geode/Bedrock)
- Sprint 18 plan: unsigned platform installers, frozen sidecar production integration

### Test Counts
- Rust: 211 tests (was 190), clippy clean
- Python: 308 tests (+12 new dedup), +5 skipped without ffprobe/whisper
- Playwright: 164 tests (was 110, +30 help + 4 error + 20 other)
- SvelteKit: 200 files, 0 svelte-check errors (was 180)

---

## Sprint 16 — Performance, Error Handling & Pipeline Parallelism

**Added**
- `AppError` enum in `src-tauri/src/error.rs` with structured IPC serialisation
- Performance timing instrumentation across verify pipeline
- Error classification on verify page with `errorType` state
- LOW security remediations completed

---

## Sprint 14 — Video Deepfake Analysis, Batch Watermarking & Security Hardening

### Week 26 — Video Deepfake + Security Audit (21 Mar 2026)

#### Video Deepfake Analysis

**Added**
- Video deepfake analysis service (`video_deepfake.py`): runs the existing per-image deepfake pipeline across evenly-spaced frames extracted from a video file
- Three analysis modes: standard (6 frames, ~12 s), deep (20 frames, ~40 s), archival (40 frames, ~80 s)
- Temporal consistency signals: noise drift, spectral drift, LBP drift — detect frame-level inconsistencies that per-frame scoring alone cannot surface
- Aggregation formula: `0.5 × mean_score + 0.3 × max_score + 0.2 × temporal_score`
- `POST /forensics/video/deepfake` endpoint with 120 s timeout
- `deepfake.py` refactored: extracted `perform_deepfake_detection_with_features()` to expose raw feature vectors for internal reuse by the video pipeline
- `FrameDeepfakeResult` and `VideoDeepfakeResult` Rust structs with serde camelCase/snake_case aliases
- `SidecarClient::analyse_video_deepfake()` method
- `analyse_video_deepfake` Tauri command wired into the verify pipeline
- Verify page frame timeline: coloured score badges per frame (green/amber/red) and aggregate verdict panel
- `FrameDeepfakeResult` and `VideoDeepfakeResult` TypeScript interfaces

#### Batch Watermarking UI

**Added**
- "Watermark All Images" button on the Protect page — queues all un-watermarked image assets for batch processing
- Progress bar showing completion count against total (e.g. 12 / 47)
- Cancel button aborts the remaining queue and reports how many were completed
- Completion summary panel: assets processed, skipped (non-image), and any errors

#### Security Audit & Hardening

**Added**
- Full security audit report (`docs/security-audit-report.md`): 21 findings across four severity levels (3 critical, 6 high, 7 medium, 5 low), with remediation status for each
- `url` crate dependency added to `Cargo.toml` for structured URL parsing

**Fixed**
- **CRITICAL-1 — SSRF in `verify_url`**: added URL validation before download; blocks loopback addresses (127.0.0.1, ::1, localhost), link-local ranges (169.254.0.0/16, fe80::/10), and RFC 1918 private networks (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16). Parsing via `url` crate prevents scheme confusion and encoded bypasses.
- **CRITICAL-2 — Over-broad filesystem capability**: Tauri `fs` capability scope restricted from wildcard to user directories only (`$HOME`, `$DOCUMENT`, `$DOWNLOAD`, `$DESKTOP`, `$TEMP`)
- **CRITICAL-3 — Unpinned CSP `connect-src`**: Content Security Policy `connect-src` directive pinned to explicit origins `http://127.0.0.1:8200` and `http://127.0.0.1:11434` only; wildcard removed

**Test counts**: 183 Rust, 292 Python (+3 skipped without ffprobe, +14 CLIP skipped), 104 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

---

## Sprint 13 — Video Frames, Audio Metadata & Extended C2PA

### Week 25 — Frame Extraction + Audio Support (21 Mar 2026)

#### Video Frame Extraction

**Added**
- Video frame extraction service (`video_frames.py`): evenly-spaced thumbnails extracted from video as base64 JPEG via FFmpeg
- `POST /video/frames` endpoint returning frame count, timestamps, and base64-encoded thumbnail array
- Frame thumbnail strip on the verify page for visual inspection of video content
- `VideoFramesResult` Pydantic model, Rust struct, and TypeScript interface

#### Audio Metadata

**Added**
- Audio metadata extraction service (`audio_metadata.py`): codec, sample rate, channels, bitrate, duration via FFmpeg/ffprobe
- `POST /audio/metadata` endpoint
- `AudioMetadataResult` Pydantic model, Rust struct, and TypeScript interface
- C2PA signing extended to `audio/wav` and `audio/mpeg` content types
- Audio metadata display on the Protect page (codec, sample rate, channels, bitrate, duration)

#### FFmpeg Integration

**Added**
- FFmpeg availability detection with graceful degradation — audio/video metadata and frame extraction skipped cleanly when FFmpeg is not installed
- Shared `ffprobe_extract()` helper used across video and audio metadata services

**Test counts**: 180 Rust, 280 Python (+3 skipped without ffprobe), 104 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

---

## Sprint 12 — Watermark Extraction, Video Metadata & Extended C2PA

### Week 24 — Verify Pipeline Watermark + Video Basics (21 Mar 2026)

#### Watermark Detection in Verify Pipeline

**Added**
- Watermark extraction wired into the verify pipeline — runs automatically on all protected images
- Watermark detection panel on the verify page showing institution name and confidence score
- `WatermarkExtractionResult` fields on `VerificationResult` (institution, confidence, payload)

#### Video Metadata

**Added**
- Video metadata extraction service (`video_metadata.py`): codec, resolution, FPS, duration, audio track information via FFmpeg/ffprobe
- `POST /video/metadata` endpoint
- `VideoMetadataResult` Pydantic model, Rust struct, and TypeScript interface
- C2PA signing extended to `video/mp4` and `video/quicktime` content types
- Video metadata display on the Protect page (codec, resolution, FPS, duration)

#### Batch Watermarking Preparation

**Added**
- Batch watermarking infrastructure: queue management and progress tracking (UI wiring deferred to Sprint 14)

**Test counts**: 165 Rust, 258 Python, 98 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

---

## Sprint 11 — Invisible Watermarking & CI/CD

### Week 23 — DWT-DCT-SVD Watermarking + GitHub Actions (21 Mar 2026)

#### Invisible Watermarking

**Added**
- Rust `watermark.rs` module using the `blind_watermark` crate — frequency-domain DWT-DCT-SVD invisible watermarking
- Three watermark strength levels: Low (~48 dB PSNR, maximum invisibility), Medium (~42 dB, balanced), High (~36 dB, maximum robustness)
- 128-bit UUID payload embedded per asset; survives JPEG compression at Q70+, proportional resize, and up to 30% crop
- `embed_watermark_asset` and `extract_watermark_from_path` Tauri commands
- Python watermark service (`watermark.py`) using `imwatermark` library with DWT-DCT-SVD algorithm
- `POST /forensics/watermark/embed` endpoint — embeds watermark and returns protected image
- `POST /forensics/watermark/extract` endpoint — recovers payload, confidence, and strength estimate
- `WatermarkEmbedRequest`, `WatermarkExtractRequest`, `WatermarkExtractResult` Pydantic models, Rust structs, and TypeScript interfaces
- Protect page watermark UI: institution name field, strength selector (Low / Medium / High), embed button
- `watermark_payload` and `watermark_strength` columns added to assets table in SQLite

#### CI/CD Infrastructure

**Added**
- GitHub Actions CI workflow: Rust (`cargo test` + `cargo clippy`), Python (`pytest`), and Frontend (`svelte-check`) jobs run in parallel on push and pull request
- GitHub Actions Release workflow: 4-platform matrix build (macOS x64, macOS arm64, Windows x64, Linux x64) triggered on version tag
- Dependabot configuration for Cargo, npm, and pip dependency updates
- `scripts/release.sh`: version-bump helper that updates `tauri.conf.json`, `Cargo.toml`, and `package.json` in lockstep

**Test counts**: 150 Rust, 240 Python, 92 Playwright e2e, 177 SvelteKit files, 0 svelte-check errors

---

## Sprint 10 — Trained AI Image Classifier

### Week 22 — GBM Classifier + Corpus Pipeline (20 Mar 2026)

#### AI Detection Improvement

**Added**
- GradientBoosting classifier trained on 80-feature vector from the deepfake detection pipeline
- 7 new deepfake signal extractors: noise autocorrelation tau (wavelet-based), cross-channel noise correlation, bit-plane regularity (lossless only), VAE grid artefacts (FFT-based), chromatic aberration absence, demosaicing traces (lossless only), saturation-luminance anomaly
- Classifier blending: 35% heuristic + 65% classifier for final score
- Auto-detection of PNG/WebP mime type from file magic bytes in deepfake endpoint
- `classifier_score` and `classifier_available` fields on DeepfakeResponse (Python, Rust, TypeScript)
- `FEATURE_NAMES` constant and `extract_feature_vector()` for stable training/inference contract
- Graceful degradation: if .joblib model absent, heuristic-only mode unchanged

#### Calibration Pipeline

**Added**
- `scripts/build_corpus.py`: Guardian press photo downloader (60 days, signed CDN URLs)
- `scripts/expand_corpus.py`: HuggingFace dataset + COCO val2017 downloader
- `scripts/build_corpus_ai.py`: AI image corpus builder
- `scripts/train_classifier.py`: feature extraction + GBM training + 5-fold stratified CV
- `scripts/evaluate_classifier.py`: model evaluation with precision/recall/AUC reporting
- `scripts/calibrate.py`: batch-process corpus through all detectors with threshold recommendations

#### Detector Threshold Recalibration

**Changed**
- Chromatic aberration: rewrote scoring — low R² now scores low (real lenses), "uncanny valley" high R² flags. FP: 75% -> 0%
- Shadow consistency: deviation 45° -> 80°, require >= 2 inconsistent regions, min 3% area. FP: 83% -> 8%
- Colour temperature: threshold 8 -> 14 LAB units, require 3+ adjacent cluster regions. FP: 58% -> 17%
- Splice boundary: never flags suspicious alone (corroborating signal only). FP: 100% -> 0%
- JPEG ghost: PNG images return neutral result immediately
- Deepfake: benford_divergence weight 0.5 -> 0.25, spectral_decay 1.5 -> 0.75
- Lossless codec thresholds tightened for all signals

#### Results

- **Training corpus**: 326 authentic (Guardian + COCO + HuggingFace) + 219 AI-generated (Gemini + HuggingFace)
- **Cross-validation AUC-ROC**: 0.945
- **AI detection rate**: 68% (13/19), up from 21% — all 12 Gemini PNGs detected
- **Authentic false positive rate**: 14% (14/100), meets 15% target
- **Top discriminating features**: lsb_randomness (28%), lsb_entropy_mean (25%), lbp_block_var_cv (21%)

**Test counts**: 138 Rust, 263 Python (+ 14 CLIP skipped), 92 Playwright e2e, 175 SvelteKit files, 0 svelte-check errors

---

## Sprint 9 — Region-Based Forensics & UI Redesign

### Week 21 — Region-Based Composite Detection + Sanctuary Theme (20 Mar 2026)

#### Region-Based Forensic Analysis

**Added**
- Segmented ELA service (`segmented_ela.py`): 8x8 grid regional error level analysis with 2-sigma anomaly detection and flood-fill cluster detection
- Shadow consistency service (`shadow_consistency.py`): gradient-weighted light direction analysis per foreground component using Otsu segmentation and circular statistics
- Colour temperature service (`colour_temperature.py`): CIELAB colour space analysis on a 4x4 grid with 8 LAB unit deviation threshold and spatial cluster detection
- Splice boundary service (`splice_boundary.py`): three-signal edge analysis (JPEG grid alignment, noise asymmetry, feathering) with 2-of-3 criterion to reduce false positives
- `POST /forensics/segmented-ela`, `/shadow-consistency`, `/colour-temperature`, `/splice-boundary` endpoints
- 8 new Rust structs for regional forensic results with serde camelCase/snake_case aliases
- 4 new `SidecarClient` methods with 30-second timeouts
- `VerificationResult` extended with 4 optional regional result fields
- `compute_trust()` updated with regional weights (segmented ELA 1.5, shadow 1.0, colour temp 1.5, splice 1.0) and composite amplification cap — when 2+ regional detectors are suspicious, trust capped at 0.55
- Region Analysis collapsible section on verify page (Deep/Archival mode only)
- `VerdictSummary` gains composite-signal awareness — simultaneous splice + ELA corroboration is highest-confidence composite indicator
- 8 TypeScript interfaces for regional results
- 27 new Python tests, 16 new Rust tests
- Sprint plan: `docs/sprint-plans/sprint-region-forensics.md`

**Validated**
- Known composite image (person dropped into group photo) that passed all 7 global detectors now triggers 3/4 regional detectors: shadow consistency (174 degree deviation), colour temperature (9/16 regions anomalous), splice boundary (45 candidate boundaries)

#### UI Redesign — Sanctuary Theme

**Added**
- Rebrand from "Jura Archive" to "Jura Trace" throughout all files
- Eye logo mark (`LogoMark.svelte`): concentric circles (lapis outer, cream iris, dark pupil)
- SVG favicon using the eye mark
- Sanctuary theme: warm dark background (#1E2128), cream text (#EDEAE4), soft accent blue (#5A85B5)
- Editorial dashboard layout: Georgia serif headlines, 900px content width, earth-line gradient dividers, narrative chapters, philosophy blockquote
- Mobile hamburger menu with body scroll lock, 44px touch targets, aria-expanded
- Skip navigation link as first focusable element
- `aria-current="page"` on active navigation links
- `prefers-reduced-motion` global animation disable
- Footer philosophy: "Keep people at the heart of every decision. Use technology to support and guide, not to take over."
- Links to juralabs.org in header and footer
- Playwright e2e test harness: 92 tests across navigation, responsive, accessibility, and page smoke tests
- Three design concept mockups in `docs/design-concepts/`

**Changed**
- Flint colour darkened #9B9890 -> #78756D for light mode AA contrast (4.6:1 on white)
- Lapis colour darkened #3E6FA8 -> #376399 for light mode AA contrast (5.0:1 on white)
- All `text-flint` instances updated with `dark:text-flint-light` for proper dual-mode contrast
- All bare `text-quartz` on light surfaces changed to `text-text-light dark:text-quartz`
- Max content width reduced from 1280px to 900px for editorial breathing room
- "ML Sidecar" language replaced with "Analysis services" throughout

**Test counts**: 136 Rust, 261 Python (+ 14 CLIP skipped), 92 Playwright e2e, 175 SvelteKit files, 0 svelte-check errors, Lighthouse 97% accessibility / 100% best practices

---

## Phase 2 — Detection Improvement & RAG

### Weeks 19-20+ — Detection Improvement & RAG Claim Checker (18 Mar 2026)

#### Sprints 1-4: Detection Honesty & False Positive Reduction

**Added**
- Three-way verdict system (authentic / inconclusive / synthetic) replaces binary suspicious/clean classification
- Trust ceiling: inconclusive capped at 60%, synthetic at 25-45% — fixes cases where manipulated images scored "92% High Trust"
- Confidence badge displayed alongside verdict label in the UI
- "Mixed signals" verdict path when detectors disagree
- 14th signal: GLCM texture structure, catching dispersed AI texture patterns missed by earlier extractors
- Scene complexity metric: halves texture and sharpness weights for uniform scenes (fog, snow, overcast sky)
- EXIF-informed sigmoid midpoint: presence of camera EXIF data shifts scoring toward authentic
- Anti-correlation penalty: texture-only signal clusters reduced in weight by 40%
- False positive reporting: structured reason codes stored in SQLite for ongoing calibration

**Changed**
- Codec-aware thresholds: AVIF, WebP, and HEIC images use relaxed scoring profiles calibrated against real iPhone photographs
- Anti-correlation penalty reduces score inflation when only texture signals fire

#### Sprint 5: New Forensic Detectors

**Added**
- NPR (Neighbouring Pixel Relationships): pixel-level correlation analysis to detect statistical discontinuities at splice boundaries
- Chromatic aberration consistency: radial lens CA pattern detection — authentic lens optics produce a predictable radial signature that AI generators do not replicate faithfully
- JPEG ghost detection: double compression analysis to identify spliced or composited regions

#### Sprint 6: Pipeline Wiring & Investigation Modes

**Added**
- All three new detectors (NPR, chromatic aberration, JPEG ghost) wired into the Rust verify pipeline
- Four investigation modes: Quick (~5 s), Standard (~15 s, default), Deep (~60 s), Archival
- Investigation mode selector in the VERIFY page UI

**Changed**
- Default investigation mode changed from Deep to Standard, reducing routine verification time from ~60 s to ~15 s

#### Sprint 7: RAG Claim Checker & False Positive Marking

**Added**
- RAG claim verification service (`claim_checker.py`) — extracts claims from image context, queries Ollama Qwen2.5, returns structured verdicts
- `POST /forensics/claim-check` endpoint
- False positive marking flow in the UI with structured reason codes; reports stored in SQLite for calibration feedback

#### Sprint 8: CLIP / UnivFD AI Detection

**Added**
- CLIP ViT-B/32 zero-shot classification via open_clip (~350 MB, lazy-loaded on first use)
- `clip_detector.py` service with graceful degradation when open_clip is not installed
- 14 CLIP tests, skipped automatically when open_clip is unavailable

**Fixed**
- Calibration finding documented: generic zero-shot prompts do not discriminate reliably between AI and authentic images at this model scale — a UnivFD linear probe is required for production-grade discrimination

#### UI Components (Weeks 19-20+)

**Added**
- `VerdictSummary.svelte`: three-way verdict display with confidence badge
- `InspectionChecklist.svelte`: 8-item manual visual inspection guide for analysts
- `SignalAgreement.svelte`: per-detector agreement/disagreement dashboard showing which signals align and which conflict
- Reverse image search buttons: Google Lens, TinEye, Yandex — one-click launch from the verify page
- 4-mode investigation selector on the verify page

**Test counts**: 121 Rust, 193 Python (+ 14 CLIP skipped), 175 SvelteKit files, 0 svelte-check errors

---

## Phase 2 — ML Sidecar + Forensic Pipeline

### Weeks 17-18b — AI Watermark Detection + Signal Calibration (5 Mar 2026)

**Added**
- Invisible watermark detection for Stable Diffusion v1, SDXL, and Flux images via `invisible-watermark` library (DWT decode, no PyTorch)
- SD v1 watermark: 136-bit exact string match ("StableDiffusionV1"), zero false positive rate
- SDXL/Flux watermark: 48-bit pattern matching (≥40/48 threshold = very likely, ≥35 = possible)
- `WatermarkDetection` Pydantic model, Rust struct, and TypeScript interface
- `watermarks` field on `DeepfakeResponse` / `DeepfakeResult` (default empty, backwards-compatible)
- Watermark detection banner on verify page AI Generation Detection section (cinnabar styling)
- C2PA AI generator identification — `detect_ai_generator()` checks `claim_generator` against 21 known AI services (OpenAI, Adobe Firefly, Midjourney, Stability AI, Flux, Google Gemini, etc.)
- `ai_generator` field on `VerificationResult` with badge in C2PA Credentials UI section
- 3 new deepfake signal extractors: `noise_consistency` (weight 1.5), `patch_spectral_variance` (weight 2.0), `multiscale_gradient` (weight 1.0)
- Patch-level spectral variance: CV of per-patch HF energy across 64×64 patches (strongest new discriminator, 2.5× separation between AI and authentic)
- Multi-scale gradient ratio: Gaussian pyramid (3 levels) gradient energy comparison
- 7 new Python watermark tests, 4 new Python signal tests, 3 new Rust tests
- Dependency: `invisible-watermark>=0.2.0`

**Changed**
- Deepfake ensemble expanded from 10 to 13 weighted signals, total weight 12.5 → 17.0
- Sigmoid scoring recalibrated: midpoint 0.25 → 0.18, steepness k 10 → 12
- `benford_divergence` weight reduced from 1.0 to 0.5 (empirically weak signal)
- Copy-move detection: added RANSAC geometric verification, scaled minimum distance with image diagonal, sigmoid area scoring, raised DBSCAN min_samples to 8
- Noise analysis: sigmoid scoring with midpoint 0.30, raised MAD z-score threshold to 4.5
- Trust computation: concordance-aware weighted formula (ELA=2.0, noise=1.0, copy-move=1.0), AVIF-safe EXIF penalty adjustment for web codecs

**Fixed**
- AI-generated images scoring too low (chihuahua: 0.55 → 0.94) due to GAN-era signal thresholds missing modern diffusion model output
- AVIF images triggering false positives in noise and copy-move detectors due to compression artefacts
- Wavelet denoiser destroying AVIF noise discrimination (reverted to median blur after testing)

**Test counts**: 98 Rust, 47 Python, 0 svelte-check errors

---

### Weeks 17-18 — Deepfake / AI-Generated Image Detection (2 Mar 2026)

**Added**
- Statistical feature ensemble for detecting AI-generated images — 6 feature extractors (frequency domain FFT, noise residual, colour/texture, LBP+GLCM, JPEG DCT artefacts, edge analysis) combined via 8 weighted heuristic signals
- `POST /forensics/deepfake` endpoint returning score, confidence, signal breakdown, and frequency spectrum heatmap
- `DeepfakeSignal` and `DeepfakeResult` Rust structs + `detect_deepfake()` sidecar client method (60s timeout)
- Deepfake detection wired into verify pipeline with graceful degradation
- AI Generation Detection section on verify page with heatmap, summary, expandable signal list, confidence indicator
- Deepfake capability shown on dashboard sidecar status
- `DeepfakeSignal` and `DeepfakeResult` TypeScript interfaces
- 10 Python deepfake tests, 2 Rust deserialization tests
- Dependencies: `scikit-image>=0.24`, `scipy>=1.14`

**Changed**
- Updated `docs/ARCHITECTURE.md` — corrected port (1420), updated module tables, added sidecar services table, rewrote verify pipeline diagram

**Test counts**: 84 Rust, 35 Python, 0 svelte-check errors

---

### Weeks 15-16 — Noise Analysis + Copy-Move Detection (1 Mar 2026)

**Added**
- Block-wise noise variance analysis service (`noise_analysis.py`) — Laplacian filter + MAD-based outlier detection with JET colourmap heatmap
- Copy-move forgery detection service (`copy_move.py`) — ORB keypoints + BFMatcher self-matching + DBSCAN clustering with visualisation
- `POST /forensics/noise` and `POST /forensics/copy-move` endpoints
- `NoiseResult`, `CloneRegion`, `CopyMoveResult` Rust structs + client methods
- Noise and copy-move wired into verify pipeline; all forensic scores averaged for trust computation
- Noise Analysis and Copy-Move Detection sections on verify page
- Extracted `_read_and_validate()` shared helper in forensics.py
- Extracted `build_image_form()` shared helper in sidecar.rs
- 8 noise analysis tests, 7 copy-move tests, 3 Rust deserialization tests
- Dependency: `scikit-learn>=1.5`

**Changed**
- Refactored trust computation from ELA-only to generic `forensic_signals` vec averaging all available signals
- Renamed `elaScoreClass`/`elaScoreBgClass` to `forensicScoreClass`/`forensicScoreBgClass`

**Test counts**: 82 Rust, 25 Python, 0 svelte-check errors

---

### Weeks 13-14 — Sidecar Bridge + ELA + URL Verification (28 Feb 2026)

**Added**
- Python ML sidecar (FastAPI on port 8200) with health endpoint and ELA service
- Error Level Analysis service (`ela.py`) — JPEG recompression difference with heatmap
- `POST /forensics/ela` endpoint
- Rust `SidecarClient` with `is_available()`, `check_health()`, `analyse_ela()` methods
- ELA wired into verify pipeline with graceful degradation
- ELA result section on verify page with heatmap, statistics, suspicious warning
- URL verification via `verify_url` Tauri command — downloads content to temp file and runs through pipeline
- URL tab on verify page with input field
- Sidecar health status badge on verify page and dashboard
- `SidecarHealth`, `SidecarCapabilities`, `ElaResult` TypeScript interfaces
- Browser mock fallback in `api.ts` for `checkSidecarHealth()`
- 7 ELA tests, 3 health tests, 6 Rust sidecar tests

**Test counts**: 76 Rust, 10 Python, 0 svelte-check errors

---

## Phase 1 — Core MVP

### Weeks 1-12 — PROTECT + VERIFY Foundation (27 Feb 2026)

**Added**
- Tauri v2 desktop shell with SvelteKit frontend (SPA mode, static adapter)
- Dark-mode UI with mineral colour palette (Obsidian, Graphite, Lapis, Malachite, Amber, Cinnabar)
- Four-tab navigation: PROTECT, VERIFY, MONITOR (placeholder), SETTINGS (placeholder)
- **PROTECT pipeline**: file import via drag-and-drop + native dialog, format detection (image/document/video/audio/3D/web), EXIF metadata extraction, image dimension reading, SQLite storage, audit logging
- **C2PA Content Credentials**: sign assets with ES256 self-signed certificates, read and verify manifests, signed output path management
- **Perceptual fingerprinting**: aHash, dHash, pHash via image_hasher crate, Hamming distance similarity search, fingerprint storage
- **EXIF anomaly detection**: 12 checks (timestamps, GPS, software, dimensions), severity levels (info/low/medium/high/critical), trust score computation
- **Format router**: MIME detection via `infer` crate with extension fallback, content type classification
- **VERIFY pipeline**: file drop + file dialog, EXIF analysis, C2PA manifest reading, trust score with findings display
- **SQLite database**: assets, fingerprints, verifications, audit_log tables with operator_id and algorithm_metadata
- **Dashboard**: stats grid (assets, signed, fingerprints, verifications), recent assets list, quick action cards
- **PROTECT page**: asset table with filtering (content type, signed status, search), bulk import, C2PA signing dialog, fingerprint viewer, similarity search, asset deletion
- Tiered cataloguing architecture (Tier 1: EXIF, Tier 2: CLIP/ONNX optional, Tier 3: Ollama optional)
- Three-tier AI design with no mandatory external dependencies
- Docker Compose setup and GitHub Actions CI/CD workflows

**Test counts**: 70 Rust, 0 svelte-check errors
