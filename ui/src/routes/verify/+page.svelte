<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { writable } from 'svelte/store';
  import {
    verifyFile, verifyUrl, checkSidecarHealth, markFalsePositive,
    parseAppError, getLicenceTier, getVersion,
    openBatchFileDialog, extractTextFromImage,
    getNetworkMode, getPowerSaverMode,
    runNprOnDemand, runShadowConsistencyOnDemand, runSpliceBoundaryOnDemand,
  } from '$lib/api';
  import { getTrustLevel, formatFileSize, formatDuration } from '$lib/types';
  import type {
    VerificationResult, SidecarHealth, VerifyMode, LicenceTier,
    AnomalyFinding, InputQualityAssessment, ManifestInfo,
    BatchItem, BatchItemStatus, NetworkMode,
  } from '$lib/types';
  import { createBlobTracker } from '$lib/blob';
  import {
    buildFpPayload,
    openFpMailto,
    copyFpReport,
    FP_FEEDBACK_EMAIL,
    type FpReportPayload,
  } from '$lib/fp-report';
  import {
    C2PA_ACTION_LABELS,
    C2PA_STATUS_INVALID,
    C2PA_STATUS_INVALID_SHORT,
    C2PA_STATUS_VALID_AT_SIGNING,
    C2PA_MSG_VALID_AT_SIGNING_DETAIL,
  } from '$lib/c2pa-labels';
  import LimitationBanner from '$lib/components/LimitationBanner.svelte';
  // EnhancedModeBanner removed — see comment near its previous mount point.
  import ExperimentalPill from '$lib/components/ExperimentalPill.svelte';
  import ContentCredentialsSeal from '$lib/components/ContentCredentialsSeal.svelte';
  import ContextualHelpLink from '$lib/components/ContextualHelpLink.svelte';
  import DetectorRow from '$lib/components/DetectorRow.svelte';
  import AiDetectorRow from '$lib/components/AiDetectorRow.svelte';
  import ImageZoom from '$lib/components/ImageZoom.svelte';
  import { forensicScoreClass } from '$lib/scoring';
  import SignalAgreement from '$lib/components/SignalAgreement.svelte';
  import MethodologyPanel from '$lib/components/MethodologyPanel.svelte';
  import { generateTrustReport } from '$lib/pdf';
  import type { ReportContext, ReportFormat } from '$lib/pdf';
  import { exportCaseZip } from '$lib/zip';
  import { saveVerifySession, restoreVerifySession, clearVerifySession } from '$lib/stores/verifySession';
  import { consumeVerifyHandoff } from '$lib/stores/verifyHandoff';

  // ── State ──────────────────────────────────────────────────────────
  let activeTab = $state<'file' | 'batch' | 'url'>('file');
  let filePath = $state<string | null>(null);
  let fileName = $state<string | null>(null);
  let urlInput = $state('');
  let result = $state<VerificationResult | null>(null);
  let checked = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let errorType = $state<'sidecar' | 'format' | 'network' | 'general' | null>(null);
  let dragOver = $state(false);
  let sidecarHealth = $state<SidecarHealth | null>(null);
  let verifyMode = $state<VerifyMode>('standard');
  let analysisElapsed = $state(0);
  let analysisStartTime = $state<number | null>(null);
  let powerSaverEnabled = $state(false);
  let cancelled = $state(false);
  let previewUrl = $state<string | null>(null);
  let showImageOverlay = $state(false);
  let appVersion = $state('0.9.0');
  let licenceTier = $state<LicenceTier>('community');
  let exportingReport = $state(false);
  let exportingCase = $state(false);
  let showReportModal = $state(false);
  let analystName = $state('');
  let analystOrg = $state('');
  let analystCaseRef = $state('');
  let analystDate = $state('');
  let analystNote = $state('');
  let reportFormat = $state<ReportFormat>('standard');
  let showFalsePositiveModal = $state(false);
  let fpReasonCode = $state('modern_codec');
  let fpReasonNote = $state('');
  let fpSubmitting = $state(false);
  let fpSubmitted = $state(false);
  // Tier 1 payload retained after submit so the success state can offer
  // a clipboard-copy fallback if the mailto launch didn't open the user's
  // mail client (no reliable way to detect that — always show the option).
  let fpPayload = $state<FpReportPayload | null>(null);
  let fpClipboardCopied = $state(false);

  // Forensic question card expand state
  let openCard = $state<'provenance' | 'integrity' | 'ai' | 'claims' | null>(null);

  // Raw scores toggle (persisted to localStorage)
  let showRawScores = $state(false);

  // Batch state
  let batchItems = $state<BatchItem[]>([]);
  let batchRunning = $state(false);
  let batchDragOver = $state(false);
  let expandedBatchId = $state<string | null>(null);

  // Read Text (Ollama LLaVA)
  let extractingText = $state(false);
  let extractedText = $state<string | null>(null);
  let extractTextError = $state<string | null>(null);

  // Video analysis — index of the frame whose detail accordion is expanded.
  let expandedFrameIndex = $state<number | null>(null);

  // CLIP zero-shot bars are hidden by default (per JTV-86 / agent review
  // 2026-04-28: 4 of 5 personas read them as contradicting the headline
  // probe score).  This local toggle exposes them on a per-row basis so a
  // power user can inspect without flipping the global Settings raw-scores
  // toggle.  The global toggle still reveals them automatically.
  let showClipZeroShot = $state(false);

  // On-demand detector loading state — set per detector while the
  // sidecar request is in-flight, cleared when the result merges back
  // into `result` (which then triggers the row to render in place of
  // the on-demand-tools footer).  String-keyed so a single error
  // message can be surfaced if the sidecar refuses, without bloating
  // the result shape.
  type OnDemandKey = 'npr' | 'shadow' | 'splice';
  let onDemandLoading = $state<Record<OnDemandKey, boolean>>({ npr: false, shadow: false, splice: false });
  let onDemandError = $state<Record<OnDemandKey, string | null>>({ npr: null, shadow: null, splice: null });

  async function runOnDemand(kind: OnDemandKey): Promise<void> {
    if (!filePath || !result) return;
    onDemandLoading[kind] = true;
    onDemandError[kind] = null;
    try {
      if (kind === 'npr') {
        const r = await runNprOnDemand(filePath);
        result = { ...result, nprResult: r };
      } else if (kind === 'shadow') {
        const r = await runShadowConsistencyOnDemand(filePath);
        result = { ...result, shadowConsistencyResult: r };
      } else {
        const r = await runSpliceBoundaryOnDemand(filePath);
        result = { ...result, spliceBoundaryResult: r };
      }
    } catch (err) {
      const parsed = parseAppError(err);
      onDemandError[kind] = parsed?.message ?? String(err);
    } finally {
      onDemandLoading[kind] = false;
    }
  }

  // GBM Deepfake synthetic verdict boundary fallback.
  //
  // The live value arrives on `result.deepfakeResult.verdictThresholds`
  // from the sidecar (JTV-97 landed 2026-04-28).  This constant is the
  // dev-only fallback for the rollout overlap window — only used when
  // a developer is running an older `uvicorn` sidecar against a newer
  // Tauri build.  Production end-users always get a matched
  // sidecar+Rust+UI bundle from the Tauri installer, so the field is
  // guaranteed present.  Remove the fallback in a follow-up release
  // once no developer is running pre-2026-04-28 sidecars.
  const GBM_SYNTHETIC_THRESHOLD_FALLBACK = 0.55;

  // Test hook store
  const _testResultStore = writable<VerificationResult | null>(null);
  let _testApplied = false;

  // Blob URL tracker
  const blobs = createBlobTracker();
  let _unlistenDragDrop: (() => void) | null = null;

  onDestroy(() => {
    blobs.revokeAll();
    _unlistenDragDrop?.();
  });

  const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // ── Weather Context state ──────────────────────────────────────────
  interface WeatherData {
    temperature: number;
    cloudCover: number;
    precipitation: number;
    visibility: number;
    windSpeed: number;
  }
  let weatherData = $state<WeatherData | null>(null);
  let weatherLoading = $state(false);
  let weatherError = $state<string | null>(null);
  // NetworkMode gates the Weather Context section below — Standard mode
  // hides it because the open-meteo fetch needs Enhanced.  Defaults to
  // 'standard' (the local-first default); refreshed on mount.
  let networkMode = $state<NetworkMode>('standard');

  const gpsCoords = $derived(
    result?.imageMetadata?.gpsLatitude != null && result?.imageMetadata?.gpsLongitude != null
      ? { lat: result.imageMetadata.gpsLatitude, lon: result.imageMetadata.gpsLongitude }
      : null
  );

  const exifDate = $derived(() => {
    const dt = result?.imageMetadata?.datetimeOriginal;
    if (!dt) return null;
    const m = dt.match(/(\d{4})[:\-](\d{2})[:\-](\d{2})/);
    if (!m) return null;
    return `${m[1]}-${m[2]}-${m[3]}`;
  });

  const exifHour = $derived(() => {
    const dt = result?.imageMetadata?.datetimeOriginal;
    if (!dt) return 12;
    const m = dt.match(/\s(\d{2}):\d{2}/);
    return m ? parseInt(m[1], 10) : 12;
  });

  async function handleFetchWeather() {
    if (!gpsCoords || weatherLoading) return;
    const date = exifDate();
    if (!date) { weatherError = 'No date available from EXIF metadata.'; return; }
    weatherLoading = true;
    weatherError = null;
    weatherData = null;
    try {
      const url = `https://archive-api.open-meteo.com/v1/archive?latitude=${gpsCoords.lat}&longitude=${gpsCoords.lon}&start_date=${date}&end_date=${date}&hourly=temperature_2m,cloudcover,precipitation,visibility,windspeed_10m&timezone=UTC`;
      const resp = await fetch(url);
      if (!resp.ok) throw new Error(`Weather API returned ${resp.status}`);
      const json = await resp.json();
      const hour = exifHour();
      const idx = Math.min(hour, (json.hourly?.time?.length ?? 1) - 1);
      weatherData = {
        temperature: json.hourly?.temperature_2m?.[idx] ?? 0,
        cloudCover: json.hourly?.cloudcover?.[idx] ?? 0,
        precipitation: json.hourly?.precipitation?.[idx] ?? 0,
        visibility: json.hourly?.visibility?.[idx] ?? 0,
        windSpeed: json.hourly?.windspeed_10m?.[idx] ?? 0,
      };
    } catch (e) {
      weatherError = e instanceof Error ? e.message : 'Weather lookup failed.';
    } finally {
      weatherLoading = false;
    }
  }



  // Reset weather state when result changes
  $effect(() => {
    void result;
    weatherData = null;
    weatherError = null;
  });

  // ── Derived ────────────────────────────────────────────────────────
  // Minimum automatic detectors that must run before any verdict claim is
  // honest.  Locked 2026-05-11 after a 1-of-15 verify result rendered as
  // "High Trust / Authentic / 70%" because the score arithmetic neutralised
  // unrun detectors.  Threshold is the union: EXIF + C2PA + ELA + noise +
  // (one AI head) = 5.  Below this, verdict downgrades to Insufficient
  // regardless of the numeric score.
  const MIN_DETECTORS_FOR_VERDICT = 5;

  const rawTrustLevel = $derived(result ? getTrustLevel(result.overallTrust) : null);

  const hasUncertainDeepfake = $derived(
    result?.deepfakeResult?.verdictLevel === 'inconclusive' ||
    result?.deepfakeResult?.verdictLevel === 'synthetic'
  );

  // A verdict is "insufficient signal" when too few detectors actually ran.
  // This catches the case where the analysis engine was unreachable mid-run
  // OR a file's codec/dimension gated most detectors off — both produce a
  // numeric score that is arithmetically valid but operationally meaningless.
  const insufficientSignal = $derived(() => {
    if (!result) return false;
    return detectorsRun() < MIN_DETECTORS_FOR_VERDICT;
  });

  // Positive authenticity evidence — at least ONE of:
  //   (a) Real camera detected via MakerNote (vendor-recognised binary blob,
  //       AI generators virtually never synthesise these); OR
  //   (b) Valid C2PA Content Credentials with NO AI declaration (manifest
  //       cryptographically valid AND no DigitalSourceType=trainedAlgorithmicMedia
  //       or compositeSynthetic action assertion).
  //
  // Without ONE of these signals, a "High Trust / Authentic" claim is unsafe:
  // it rests purely on the absence of negative findings, which a re-encoded
  // AI image (e.g. PNG export of a JPEG-generated synthetic) routinely
  // produces because JPEG-specific detectors codec-gate off and the AI
  // ensemble can drift on format conversion.  Cap at Moderate / Review.
  const hasPositiveAuthenticitySignal = $derived(() => {
    if (!result) return false;
    const cameraBonus = result.exifAnalysis?.cameraAuthenticityBonus ?? 0;
    if (cameraBonus > 0.5) return true;
    if (result.c2paValid === true && !c2paDigitalSourceType()) return true;
    return false;
  });

  const trustLevel = $derived(() => {
    if (!rawTrustLevel) return null;
    if (insufficientSignal()) return 'inconclusive' as const;
    // Safety cap (added 2026-05-11 after a re-encoded AI PNG passed as High
    // Trust): cannot claim "Authentic" without positive provenance evidence.
    if (rawTrustLevel === 'high' && !hasPositiveAuthenticitySignal()) return 'medium';
    if (hasUncertainDeepfake && rawTrustLevel === 'high') return 'medium';
    return rawTrustLevel;
  });

  const trustScorePercent = $derived(result ? Math.round(result.overallTrust * 100) : 0);

  const trustColorClass = $derived(() => {
    const lv = trustLevel();
    if (lv === 'high') return 'text-malachite-dark dark:text-malachite-light';
    if (lv === 'medium') return 'text-amber-dark dark:text-amber-light';
    if (lv === 'low') return 'text-cinnabar-dark dark:text-cinnabar-light';
    if (lv === 'inconclusive') return 'text-flint-dark dark:text-flint-light';
    return 'text-flint-dark dark:text-flint-light';
  });

  const trustStrokeColor = $derived(() => {
    const lv = trustLevel();
    if (lv === 'high') return '#5B8A5F';
    if (lv === 'medium') return '#D4943A';
    if (lv === 'low') return '#C45B52';
    if (lv === 'inconclusive') return '#78756D';
    return '#78756D';
  });

  const trustLabelText = $derived(() => {
    if (insufficientSignal()) return 'Inconclusive';
    if (hasUncertainDeepfake) {
      if (result?.deepfakeResult?.verdictLevel === 'synthetic') return 'Low Trust';
      return 'Uncertain';
    }
    const lv = trustLevel();
    if (lv === 'high') return 'High Trust';
    if (lv === 'medium') return 'Moderate Trust';
    if (lv === 'low') return 'Low Trust';
    return '';
  });

  const verdictBadgeClass = $derived(() => {
    const lv = trustLevel();
    if (lv === 'high') return 'bg-malachite/15 text-malachite-dark dark:text-malachite-light border-malachite/30';
    if (lv === 'medium') return 'bg-amber/15 text-amber-dark dark:text-amber-light border-amber/30';
    if (lv === 'inconclusive') return 'bg-flint/15 text-flint-dark dark:text-flint-light border-flint/30';
    return 'bg-cinnabar/15 text-cinnabar-dark dark:text-cinnabar-light border-cinnabar/30';
  });

  const aiDetectionSuppressed = $derived(
    result?.contentTypeResult != null &&
    result.contentTypeResult.aiDetectionSuitable === false
  );

  const sidecarAvailable = $derived(sidecarHealth?.status === 'ok');

  // ── Content Credentials (C2PA) derived state ───────────────────────
  // sealState drives the ContentCredentialsSeal icon colour.
  const c2paSealState = $derived(() => {
    if (!result) return 'none' as const;
    if (result.c2paValid === true)  return 'valid' as const;
    if (result.c2paValid === false) return 'invalid' as const;
    return 'none' as const;
  });

  // Signer display name: prefer signedBy (human-readable org), fall back to
  // claimGenerator (tool identifier). Both are permitted by the spec; signedBy
  // is the "Issued by" field per C2PA UX Rec v1.4 §4.2.
  const c2paSignerName = $derived(
    result?.c2paManifest?.signedBy ||
    result?.c2paManifest?.claimGenerator ||
    null
  );

  // Parse the signed date into a locale string for display.
  const c2paSignedDate = $derived(() => {
    const raw = result?.c2paManifest?.signedAt;
    if (!raw) return null;
    try {
      return new Date(raw).toLocaleDateString('en-GB', {
        day: 'numeric', month: 'long', year: 'numeric',
      });
    } catch {
      return raw;
    }
  });

  // Expand/collapse state for L2 and L3 disclosure panels.
  let c2paShowL2 = $state(false);
  let c2paShowL3 = $state(false);

  // Provenance chain timeline expand state (§5.4 collapse when >= 4 manifests).
  let c2paChainExpanded = $state(false);

  // L3 manifest selector — 0 = active manifest, 1+ = ingredients in chain order.
  let selectedManifestIndex = $state(0);

  const allManifests = $derived(() => {
    if (!result?.c2paChain) return result?.c2paManifest ? [result.c2paManifest] : [];
    return [result.c2paChain.active, ...result.c2paChain.ingredients];
  });

  const selectedManifest = $derived(() => allManifests()[selectedManifestIndex] ?? null);

  /**
   * Top-level "Valid at signing" state — the Pixel-Camera-style
   * short-lived-credential case.  True when the asset is cryptographically
   * Valid AND the active manifest's leaf signing cert has since expired
   * (trusted timestamp confirms signing-time validity).  Promotes the L2
   * seal label per C2PA UX Rec v1.4 Table 2 "Signed on <date>" pattern
   * without downgrading the Valid state.  Sovereign-mode chains that do
   * not expose a parseable leaf return `certificateExpired === undefined`
   * and collapse back to plain "Valid".
   */
  const isValidAtSigning = $derived(() =>
    result?.c2paValid === true
    && result?.c2paManifest?.certificateExpired === true
  );

  /** Format a ManifestInfo signer name for chain timeline display. */
  function chainSignerName(manifest: ManifestInfo): string {
    return manifest.signedBy || manifest.claimGenerator || 'Unknown';
  }

  /** Format a ManifestInfo date for chain timeline display (en-GB locale). */
  function chainSignedDate(manifest: ManifestInfo): string | null {
    const raw = manifest.signedAt;
    if (!raw) return null;
    try {
      return new Date(raw).toLocaleDateString('en-GB', {
        day: 'numeric', month: 'short', year: 'numeric',
      });
    } catch {
      return raw;
    }
  }

  /** Format an ISO 8601 / RFC 3339 timestamp to en-GB long form for L3. */
  function formatCertDate(raw: string | undefined | null): string | null {
    if (!raw) return null;
    try {
      return new Date(raw).toLocaleDateString('en-GB', {
        day: 'numeric', month: 'long', year: 'numeric',
      });
    } catch {
      return raw;
    }
  }

  /**
   * Parse the first action label from a ManifestInfo's c2pa.actions assertion.
   * Returns a short one-line summary suitable for chain timeline display.
   */
  function chainActionSummary(manifest: ManifestInfo): string | null {
    const actionsAssertion = manifest.assertions.find(
      (a) => a.label === 'c2pa.actions' || a.label === 'c2pa.actions.v2'
    );
    if (!actionsAssertion) return null;
    try {
      const parsed = JSON.parse(actionsAssertion.value);
      const actions: { action: string }[] = parsed?.actions ?? parsed ?? [];
      if (actions.length === 0) return null;
      const first = C2PA_ACTION_LABELS[actions[0].action] ?? actions[0].action;
      return actions.length > 1 ? `${first} +${actions.length - 1} more` : first;
    } catch {
      return null;
    }
  }

  // Action URI -> consumer label map is imported from $lib/c2pa-labels so the
  // verify page and the PDF report cannot drift. See that module for the
  // v1.4 Table 3 references.

  // Parse actions from the c2pa.actions.v2 assertion with full detail.
  const c2paActions = $derived(() => {
    const assertions = result?.c2paManifest?.assertions ?? [];
    const actionsAssertion = assertions.find(
      (a) => a.label === 'c2pa.actions' || a.label === 'c2pa.actions.v2'
    );
    if (!actionsAssertion) return [];
    try {
      const parsed = JSON.parse(actionsAssertion.value);
      const actions: { action: string; description?: string; digitalSourceType?: string; softwareAgent?: string }[] = parsed?.actions ?? parsed ?? [];
      return actions.map((a) => ({
        raw: a.action,
        label: C2PA_ACTION_LABELS[a.action] ?? a.action,
        description: a.description ?? null,
        sourceType: a.digitalSourceType ? humaniseDigitalSourceType(a.digitalSourceType) : null,
        softwareAgent: a.softwareAgent ?? null,
      }));
    } catch {
      return [];
    }
  });

  // Map digitalSourceType URIs to human-readable labels.
  function humaniseDigitalSourceType(raw: string): string {
    const MAP: Record<string, string> = {
      'computationalCapture':                 'Computational capture',
      'digitalCapture':                       'Digital capture',
      'filmCapture':                          'Film capture',
      'humanEdited':                          'Human edited',
      'algorithmicMedia':                     'Algorithmic media',
      'compositeCapture':                     'Composite capture',
      'compositeWithTrainedAlgorithmicMedia': 'Composite with trained algorithmic media',
      'trainedAlgorithmicMedia':              'Trained algorithmic media',
    };
    // Strip any URI prefix and look up the local name.
    const localName = raw.replace(/^.*[/#]/, '');
    return MAP[localName] ?? localName;
  }

  // Extract digitalSourceType from the c2pa.claim.v2 or stds.schema-org.CreativeWork assertion.
  const c2paDigitalSourceType = $derived(() => {
    const assertions = result?.c2paManifest?.assertions ?? [];
    for (const a of assertions) {
      try {
        const parsed = JSON.parse(a.value);
        const dst = parsed?.digitalSourceType ?? parsed?.schema_org?.digitalSourceType;
        if (dst && typeof dst === 'string') return humaniseDigitalSourceType(dst);
      } catch { /* skip */ }
    }
    return null;
  });

  // ── SVG ring animation ─────────────────────────────────────────────
  // circumference for r=47: 2π×47 ≈ 295.3
  const RING_CIRC = 295.3;
  const ringDashoffset = $derived(
    result ? RING_CIRC * (1 - result.overallTrust) : RING_CIRC
  );

  // ── Signal map dots ────────────────────────────────────────────────
  // Colour semantics (locked 2026-05-10):
  //   green  (pass)         — detector ran, returned positive evidence (e.g. authentic EXIF)
  //   amber  (concern)      — uncertain or honestly-disclosed AI in a valid manifest
  //   red    (suspicious)   — explicit evidence of AI / manipulation
  //   grey   (not-run/empty/suppressed) — no positive OR negative signal to surface
  // The map mirrors the overall-trust pill vocabulary so the per-detector dots
  // do not contradict the headline verdict.
  type DotState = 'pass' | 'concern' | 'suspicious' | 'suppressed' | 'not-run' | 'empty-data';

  function dotState(suspicious: boolean | undefined | null, ran: boolean, suppressed = false): DotState {
    if (suppressed) return 'suppressed';
    if (!ran) return 'not-run';
    if (suspicious === true) return 'suspicious';
    return 'pass';
  }

  // EXIF-specific dot state: distinguish "no EXIF data at all" (typical of
  // AI-stripped images) from "EXIF present and clean".  An empty-data EXIF
  // is informationally neutral — green is misleading.
  function exifDotState(): DotState {
    if (!result?.exifAnalysis) return 'not-run';
    const findings = result.exifAnalysis.findings ?? [];
    const highSeverity = findings.filter(
      (f: AnomalyFinding) => f.severity === 'high' || f.severity === 'critical'
    );
    if (highSeverity.length > 0) return 'suspicious';
    // hasExif=false (EXIF block absent or fully stripped) plus no findings
    // means we have nothing to assess — informationally neutral, not pass.
    // Firefly / Midjourney / DALL-E outputs typically strip EXIF on export;
    // a green dot in that case is misleading.
    if (result.exifAnalysis.hasExif === false && findings.length === 0) return 'empty-data';
    return 'pass';
  }

  // C2PA-specific dot state: a valid manifest that contains an AI-disclosure
  // action assertion (DigitalSourceType=trainedAlgorithmicMedia,
  // compositeSynthetic, etc.) is not the same as a valid manifest with no AI
  // declaration.  Honest AI disclosure is a CONCERN (amber) signal, not a
  // PASS (green) signal — the manifest is valid but it confesses AI involvement.
  function c2paDotState(): DotState {
    if (result?.c2paValid === false) return 'suspicious';
    if (result?.c2paValid !== true) return 'not-run';
    // Valid manifest.  Check whether it discloses AI.
    if (c2paDigitalSourceType()) return 'concern';
    return 'pass';
  }

  const signalDots = $derived(() => {
    if (!result) return { provenance: [], integrity: [], ai: [] };

    const exifHighFindings = (result.exifAnalysis?.findings ?? []).filter(
      (f: AnomalyFinding) => f.severity === 'high' || f.severity === 'critical'
    );
    const exifSuspicious = exifHighFindings.length > 0;

    const c2paAiType = c2paDigitalSourceType();
    const c2paState = c2paDotState();
    const exifState = exifDotState();

    return {
      provenance: [
        {
          id: 'card-provenance', label: 'EXIF',
          state: exifState,
          ariaDetail: exifState === 'suspicious'
            ? `${exifHighFindings.length} high-severity anomal${exifHighFindings.length === 1 ? 'y' : 'ies'}`
            : exifState === 'empty-data' ? 'No metadata — frequently seen on AI-generated images'
            : exifState === 'pass' ? 'No critical anomalies'
            : 'Not run',
        },
        {
          id: 'card-provenance', label: 'Content Credentials',
          state: c2paState,
          // Invalid label is the C2PA UX Rec Table 4 verbatim string.  "Valid at
          // signing" is the Jura Trace extension for the Pixel-Camera-style
          // short-lived-credential case (cert has since expired, but the timestamp
          // confirms signing-time validity) — flagged in the submission letter.
          // 'concern' state means the manifest is valid AND discloses AI generation —
          // honest disclosure is not penalised but is also not a green-light signal.
          ariaDetail: c2paState === 'concern' && c2paAiType
            ? `Valid manifest — discloses AI generation (${c2paAiType})`
            : result.c2paValid === true
              ? (result.c2paManifest?.certificateExpired === true ? C2PA_STATUS_VALID_AT_SIGNING : 'Valid')
              : result.c2paValid === false ? C2PA_STATUS_INVALID : 'Not attached',
        },
      ],
      integrity: [
        {
          id: 'card-integrity', label: 'ELA',
          state: dotState(result.elaResult?.suspicious, result.elaResult != null),
          ariaDetail: result.elaResult ? `Score ${Math.round((result.elaResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'Noise',
          state: dotState(result.noiseResult?.suspicious, result.noiseResult != null),
          ariaDetail: result.noiseResult ? `Score ${Math.round((result.noiseResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'Copy-Move',
          state: dotState(result.copyMoveResult?.suspicious, result.copyMoveResult != null),
          ariaDetail: result.copyMoveResult ? `Score ${Math.round((result.copyMoveResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'JPEG Ghost',
          state: dotState(result.jpegGhostResult?.suspicious, result.jpegGhostResult != null),
          ariaDetail: result.jpegGhostResult ? `Score ${Math.round((result.jpegGhostResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'Seg. ELA',
          state: dotState(result.segmentedElaResult?.suspicious, result.segmentedElaResult != null),
          ariaDetail: result.segmentedElaResult ? `Score ${Math.round((result.segmentedElaResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'Colour Temp',
          state: dotState(result.colourTemperatureResult?.suspicious, result.colourTemperatureResult != null),
          ariaDetail: result.colourTemperatureResult ? `Score ${Math.round((result.colourTemperatureResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'Shadow',
          state: dotState(result.shadowConsistencyResult?.suspicious, result.shadowConsistencyResult != null),
          ariaDetail: result.shadowConsistencyResult ? `Score ${Math.round((result.shadowConsistencyResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'Splice',
          state: dotState(result.spliceBoundaryResult?.suspicious, result.spliceBoundaryResult != null),
          ariaDetail: result.spliceBoundaryResult ? `Score ${Math.round((result.spliceBoundaryResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-integrity', label: 'NPR',
          state: dotState(result.nprResult?.suspicious, result.nprResult != null),
          ariaDetail: result.nprResult ? `Score ${Math.round((result.nprResult.score) * 100)}%` : 'Not run',
        },
      ],
      ai: [
        {
          id: 'card-ai', label: 'Deepfake',
          state: aiDetectionSuppressed ? 'suppressed' as DotState : dotState(result.deepfakeResult?.suspicious, result.deepfakeResult != null),
          ariaDetail: aiDetectionSuppressed ? 'Suppressed — content type' : result.deepfakeResult ? `Score ${Math.round((result.deepfakeResult.score) * 100)}%` : 'Not run',
        },
        // CLIP cross-check is conditional on the sidecar shipping with open-clip-torch.
        // The CI-built sidecar (requirements-ci.txt) excludes it (~2 GB install size);
        // dev builds with the full requirements.txt include it.  When the Analysis
        // Engine reports clipDetect=false we omit the chip entirely rather than
        // show a permanently-grey dot the user has no way to act on.
        ...(sidecarHealth?.capabilities?.clipDetect ? [{
          id: 'card-ai', label: 'CLIP',
          state: aiDetectionSuppressed ? 'suppressed' as DotState : dotState(result.clipResult?.verdictLevel === 'synthetic', result.clipResult != null),
          ariaDetail: aiDetectionSuppressed ? 'Suppressed — content type' : result.clipResult ? `Score ${Math.round((result.clipResult.score) * 100)}%` : 'Not run',
        }] : []),
        {
          id: 'card-ai', label: 'Watermark',
          state: dotState(result.watermarkExtractResult?.hasWatermark === false ? false : undefined, result.watermarkExtractResult != null),
          ariaDetail: result.watermarkExtractResult?.hasWatermark ? 'Watermark found' : 'No watermark',
        },
      ],
    };
  });

  // ── Forensic question pass/fail derivations ────────────────────────
  const provenancePass = $derived(() => {
    if (!result) return null;
    const noHighExif = (result.exifAnalysis?.findings ?? []).every(
      (f: AnomalyFinding) => f.severity !== 'high' && f.severity !== 'critical'
    );
    const c2paOk = result.c2paValid !== false;
    return noHighExif && c2paOk;
  });

  const integrityPass = $derived(() => {
    if (!result) return null;
    // If no integrity detector ran at all (e.g. analysis engine unavailable),
    // return null so the card shows "—" rather than "Pass" — saying "Pass"
    // when nothing actually ran is misleading-by-design.
    const ranAny = [
      result.elaResult,
      result.noiseResult,
      result.copyMoveResult,
      result.jpegGhostResult,
      result.segmentedElaResult,
      result.colourTemperatureResult,
      result.shadowConsistencyResult,
      result.spliceBoundaryResult,
    ].some(r => r != null);
    if (!ranAny) return null;
    return ![
      result.elaResult?.suspicious,
      result.noiseResult?.suspicious,
      result.copyMoveResult?.suspicious,
      result.jpegGhostResult?.suspicious,
      result.segmentedElaResult?.suspicious,
      result.colourTemperatureResult?.suspicious,
      result.shadowConsistencyResult?.suspicious,
      result.spliceBoundaryResult?.suspicious,
    ].some(Boolean);
  });

  const aiPass = $derived(() => {
    if (!result || aiDetectionSuppressed) return null;
    // If no AI-detection head ran at all, return null — the question is
    // unanswered.  Saying "Pass" when neither GBM nor UnivFD ran is
    // misleading-by-design (the user assumes the question was assessed).
    const gbmRan = result.deepfakeResult != null;
    const clipRan = result.clipResult != null;
    if (!gbmRan && !clipRan) return null;
    return !result.deepfakeResult?.suspicious && result.clipResult?.verdictLevel !== 'synthetic';
  });

  // State-label for the AI-card panel header (JTV-92).  Replaces the
  // mathematically correct but communicatively backwards "X of 2 passed"
  // — for AI detection, a "pass" means "not flagged as synthetic" and
  // pass-counting framed positive AI flags as failures.  This label
  // tells the user the *agreement state* directly.
  const aiStateLabel = $derived(() => {
    if (!result || aiDetectionSuppressed) return null;
    const gbm = result.deepfakeResult;
    const clip = result.clipResult;
    const ranCount = [gbm, clip].filter(Boolean).length;
    if (ranCount === 0) return null;
    const gbmSyn = gbm?.suspicious === true || gbm?.verdictLevel === 'synthetic';
    const clipSyn = clip?.verdictLevel === 'synthetic';
    const gbmAuth = gbm?.verdictLevel === 'authentic';
    const clipAuth = clip?.verdictLevel === 'authentic';
    if (ranCount === 1) {
      if (gbmSyn || clipSyn) return 'Detector flagged AI';
      if (gbmAuth || clipAuth) return 'Detector clear';
      return 'Detector inconclusive';
    }
    if (gbmSyn && clipSyn) return 'Both detectors flagged AI';
    if (gbmAuth && clipAuth) return 'Both detectors clear';
    if (gbmSyn !== clipSyn) return 'Detectors disagree';
    return 'Both detectors inconclusive';
  });

  // Plain-English one-liner shown as the first element inside the AI card
  // body (JTV-89, agent review 2026-04-28).  Goal: a non-technical reader
  // gets the conclusion before encountering scores, percentages, or signal
  // breakdowns.  Derived from the two detector verdicts so it never drifts
  // from what the rows below claim.
  const aiVerdictSentence = $derived(() => {
    if (!result || aiDetectionSuppressed) return null;
    const gbm = result.deepfakeResult;
    const clip = result.clipResult;
    const gbmSyn = gbm?.suspicious === true || gbm?.verdictLevel === 'synthetic';
    const clipSyn = clip?.verdictLevel === 'synthetic';
    const gbmAuth = gbm?.verdictLevel === 'authentic';
    const clipAuth = clip?.verdictLevel === 'authentic';
    const ranCount = [gbm, clip].filter(Boolean).length;
    if (ranCount === 0) return null;
    if (ranCount === 1) {
      // Only one of the two detectors ran.  Use whichever's verdict is
      // present.  GBM has `suspicious`; CLIP does not — fall back to the
      // verdictLevel comparison which both expose.
      const singleSyn = gbmSyn || clipSyn;
      const singleAuth = gbmAuth || clipAuth;
      if (singleSyn) {
        return 'One AI check ran and identified characteristics consistent with AI-generated content. Cross-checking with a second detector was unavailable.';
      }
      if (singleAuth) {
        return 'One AI check ran and indicates this is consistent with a real photograph. Cross-checking with a second detector was unavailable.';
      }
      return 'One AI check ran with an inconclusive result. Cross-checking with a second detector was unavailable.';
    }
    if (gbmSyn && clipSyn) {
      return 'Both AI checks identified characteristics consistent with AI-generated content.';
    }
    if (gbmAuth && clipAuth) {
      return 'Both AI checks indicate this is consistent with a real photograph.';
    }
    if (gbmSyn !== clipSyn) {
      return 'AI checks disagree — one detector flagged synthetic features, the other did not. Treat the result as inconclusive and inspect the per-detector evidence below.';
    }
    return 'AI checks ran with mixed or inconclusive results. Inspect the per-detector evidence below.';
  });

  const hasClaimsData = $derived(
    result != null && (result.transcriptionResult != null || result.claimCheckResult != null)
  );

  // Count of findings per card
  const provenanceFindings = $derived(
    !result ? 0 : [
      (result.exifAnalysis?.findings ?? []).filter((f: AnomalyFinding) => f.severity === 'high' || f.severity === 'critical').length > 0,
      result.c2paValid === false,
    ].filter(Boolean).length
  );

  const integrityFindings = $derived(
    !result ? 0 : [
      result.elaResult?.suspicious,
      result.noiseResult?.suspicious,
      result.copyMoveResult?.suspicious,
      result.jpegGhostResult?.suspicious,
      result.segmentedElaResult?.suspicious,
      result.colourTemperatureResult?.suspicious,
      result.shadowConsistencyResult?.suspicious,
      result.spliceBoundaryResult?.suspicious,
    ].filter(Boolean).length
  );

  const aiFindings = $derived(
    !result || aiDetectionSuppressed ? 0 : [
      result.deepfakeResult?.suspicious,
      result.clipResult?.verdictLevel === 'synthetic',
    ].filter(Boolean).length
  );

  // Total detectors that ran.  CLIP is excluded from the count when the
  // sidecar build does not ship open-clip-torch (CI builds excludes it for
  // size reasons — ~2 GB).  Counting "CLIP" as a missing detector when the
  // user has no way to install it produces a misleading "N/15" denominator.
  const detectorsRun = $derived(() => {
    if (!result) return 0;
    const slots: Array<unknown> = [
      result.exifAnalysis, result.c2paValid !== undefined && result.c2paValid !== null,
      result.elaResult, result.noiseResult, result.copyMoveResult,
      result.deepfakeResult, result.jpegGhostResult, result.segmentedElaResult,
      result.colourTemperatureResult, result.watermarkExtractResult,
      result.shadowConsistencyResult, result.spliceBoundaryResult, result.nprResult,
    ];
    if (sidecarHealth?.capabilities?.clipDetect) {
      slots.push(result.clipResult);
    }
    return slots.filter(Boolean).length;
  });

  // Total detectors the build is capable of running (denominator for "N/M").
  // Adapts to the sidecar's actual capability list — when CLIP is not bundled,
  // the total is 14 not 15.
  const detectorsAvailable = $derived(() => {
    return sidecarHealth?.capabilities?.clipDetect ? 15 : 14;
  });

  const totalFindings = $derived(provenanceFindings + integrityFindings + aiFindings);

  // Camera make/model from EXIF
  const cameraLabel = $derived(() => {
    if (!result?.exifAnalysis) return null;
    const meta = result.exifAnalysis as any;
    const make = meta.cameraMake ?? meta.make ?? null;
    const model = meta.cameraModel ?? meta.model ?? null;
    if (make && model) return `${make} ${model}`;
    if (model) return model;
    return null;
  });

  const imageDimensions = $derived(() => {
    if (!result) return null;
    const iq = result.inputQuality;
    if (iq?.width && iq?.height) {
      const mp = ((iq.width * iq.height) / 1_000_000).toFixed(1);
      return `${iq.width.toLocaleString()} × ${iq.height.toLocaleString()} · ${mp} MP`;
    }
    return null;
  });

  // Auto-expand the card with findings when result arrives
  $effect(() => {
    if (!result) { openCard = null; return; }
    if (integrityFindings > 0) { openCard = 'integrity'; return; }
    if (provenanceFindings > 0) { openCard = 'provenance'; return; }
    if (aiFindings > 0) { openCard = 'ai'; return; }
    openCard = null;
  });

  // ── Error classification ───────────────────────────────────────────
  function setError(e: unknown, context: string) {
    const { code, message } = parseAppError(e);
    if (code !== null) {
      switch (code) {
        case 'Sidecar':
          errorType = 'sidecar';
          error = 'The Analysis Engine is not running. Core checks (Content Credentials, EXIF) are still available.';
          break;
        case 'Validation':
          errorType = 'format';
          error = message;
          break;
        default:
          errorType = 'general';
          error = message;
      }
      return;
    }
    const lower = message.toLowerCase();
    if (lower.includes('unsupported') || lower.includes('format') || lower.includes('mime')) {
      errorType = 'format';
      error = 'Unsupported file format. Jura Trace v1.0 supports JPEG, PNG, TIFF, WebP, HEIC, AVIF, PDF, MP4 and MOV.';
    } else if (lower.includes('sidecar') || lower.includes('connection refused')) {
      errorType = 'sidecar';
      error = 'The Analysis Engine is not running. Core checks are still available.';
    } else if (lower.includes('fetch') || lower.includes('network')) {
      errorType = 'network';
      error = 'Could not fetch the URL. Check the address is correct and publicly accessible.';
    } else {
      errorType = 'general';
      error = `${context}: ${message}`;
    }
  }

  // ── Preview URL ────────────────────────────────────────────────────
  // Generate a Tauri asset:// URL for any media we can render in-webview.
  // Images go through <img>; mp4/mov/webm go through <video controls>;
  // unsupported codecs (mkv/avi) and non-media types fall back to a
  // text-only placeholder downstream.  We still emit the URL for video
  // so the renderer can decide; the URL itself is cheap.
  $effect(() => {
    const path = filePath;
    const res = result;
    previewUrl = null;
    if (!path || !res || !inTauri) return;
    if (res.contentType !== 'image' && res.contentType !== 'video' && res.contentType !== 'audio') return;
    import('@tauri-apps/api/core').then(({ convertFileSrc }) => {
      previewUrl = convertFileSrc(path);
    }).catch(() => {});
  });

  // ── Preview kind ───────────────────────────────────────────────────
  // Decide which renderer to use in the trust-score banner.  Webview
  // codec support varies by platform — see project memory and the
  // CLAUDE.md backlog notes — so unsupported video codecs fall back to
  // a "preview unavailable" tile rather than a broken <video> element.
  function previewKind(name: string | null, contentType: string | undefined): 'image' | 'video' | 'audio' | 'unsupported' {
    if (!name) return 'unsupported';
    if (contentType === 'image' || /\.(jpe?g|png|webp|tiff?|avif|heic|heif|gif)$/i.test(name)) return 'image';
    if (contentType === 'video' || /\.(mp4|mov|webm|m4v)$/i.test(name)) {
      // mkv and avi are rejected — neither WKWebView (macOS) nor
      // WebView2 (Windows) plays them reliably.  Sidecar still
      // analyses them; only the preview is unavailable.
      if (/\.(mkv|avi)$/i.test(name)) return 'unsupported';
      return 'video';
    }
    if (contentType === 'audio' || /\.(mp3|wav|m4a|flac|ogg|aac)$/i.test(name)) return 'audio';
    return 'unsupported';
  }
  const currentPreviewKind = $derived<'image' | 'video' | 'audio' | 'unsupported'>(
    previewKind(fileName, result?.contentType)
  );

  /**
   * Convert an on-disk heatmap path to an asset:// URL that the webview can
   * load.  Falls back to passing the value through as-is so data: URLs and
   * empty strings are never broken.
   *
   * Module-level closure (no window namespace pollution).  The dynamic
   * import below populates `_convertFileSrc`; until it resolves we
   * pass-through the path unchanged, which is fine because the heatmap
   * `<img>` tags are reactive and will re-render once the closure is set.
   */
  let _convertFileSrc: ((path: string) => string) = (p) => p;
  import('@tauri-apps/api/core').then(({ convertFileSrc }) => {
    _convertFileSrc = convertFileSrc;
  }).catch(() => {});

  function heatmapSrc(pathOrUrl: string | null | undefined): string {
    if (!pathOrUrl) return '';
    if (pathOrUrl.startsWith('data:') || pathOrUrl.startsWith('http') || pathOrUrl.startsWith('asset:')) {
      return pathOrUrl;
    }
    try {
      return _convertFileSrc(pathOrUrl);
    } catch {
      return pathOrUrl;
    }
  }

  // ── Playwright test hooks ──────────────────────────────────────────
  // Both hooks are gated behind import.meta.env.DEV so they tree-shake out of
  // production builds, closing the arbitrary-injection surface.
  if (typeof window !== 'undefined' && import.meta.env.DEV) {
    (window as any).__juraSetVerifyResult = (data: VerificationResult) => {
      _testResultStore.set(data);
    };
    (window as any).__juraSetVerifyError = (msg: string) => {
      const lower = msg.toLowerCase();
      if (lower.includes('sidecar') || lower.includes('connection refused') || lower.includes('127.0.0.1')) {
        errorType = 'sidecar';
      } else if (lower.includes('unsupported') || lower.includes('format') || lower.includes('mime')) {
        errorType = 'format';
      } else if (lower.includes('fetch') || lower.includes('network')) {
        errorType = 'network';
      } else {
        errorType = 'general';
      }
      error = msg;
    };
  }
  $effect(() => {
    const injected = $_testResultStore;
    if (injected && !_testApplied) {
      _testApplied = true;
      result = structuredClone(injected) as VerificationResult;
      checked = true;
      loading = false;
      error = null;
    }
  });

  // ── Elapsed timer ─────────────────────────────────────────────────
  $effect(() => {
    if (loading && analysisStartTime) {
      const interval = setInterval(() => {
        analysisElapsed = Math.floor((Date.now() - (analysisStartTime ?? Date.now())) / 1000);
      }, 1000);
      return () => clearInterval(interval);
    } else {
      analysisElapsed = 0;
    }
  });

  // ── Session persistence ────────────────────────────────────────────
  $effect(() => {
    if (result && fileName) {
      saveVerifySession({
        result,
        fileName,
        filePath,
        previewDataUrl: previewUrl,
        mode: verifyMode,
        verifiedAt: new Date().toISOString(),
      });
    }
  });

  // ── Lifecycle ─────────────────────────────────────────────────────
  onMount(() => {
    const savedMode = localStorage.getItem('jura-verify-mode');
    // Migrate 'archival' -> 'deep' — archival was removed 2026-04-22 because
    // it ran the identical pipeline to Deep.  Existing pilot users had
    // 'archival' persisted and must land on a still-offered mode.
    if (savedMode === 'archival') {
      verifyMode = 'deep';
      localStorage.setItem('jura-verify-mode', 'deep');
    } else if (savedMode === 'standard' || savedMode === 'deep') {
      verifyMode = savedMode;
    }
    showRawScores = localStorage.getItem('jura-raw-scores-default') === 'true';

    // Load NetworkMode so the Weather Context section can gate itself.
    // Weather requires an outbound HTTPS call to archive-api.open-meteo.com,
    // which is only allowed in Enhanced mode.  In Standard mode the section
    // is hidden entirely; the EnhancedModeBanner above offers the upgrade.
    getNetworkMode().then((m) => { networkMode = m; }).catch(() => {});
    analystName = localStorage.getItem('jura-analyst-name') ?? '';
    analystOrg = localStorage.getItem('jura-analyst-org') ?? '';
    analystNote = localStorage.getItem('jura-analyst-note') ?? '';
    analystDate = new Date().toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });

    // Check for a protect→verify handoff (JTV-125).  consumeVerifyHandoff()
    // is one-shot: it returns and clears the payload in a single call so a
    // back-navigation doesn't re-trigger the analysis.
    const handoff = consumeVerifyHandoff();
    if (handoff) {
      // Don't restore a stale session — the handoff takes priority.
      void runFileVerification(handoff.filePath, handoff.fileName);
    } else if (!result) {
      // Restore session
      const saved = restoreVerifySession();
      if (saved) {
        result = saved.result;
        fileName = saved.fileName;
        filePath = saved.filePath;
        verifyMode = (saved.mode as VerifyMode) || 'standard';
        checked = true;
        previewUrl = saved.previewDataUrl;
      }
    }

    (async () => {
      sidecarHealth = await checkSidecarHealth();
      appVersion = await getVersion();
      licenceTier = await getLicenceTier();
      powerSaverEnabled = await getPowerSaverMode();
      await setupTauriDragDrop();
    })();

    function handleEsc(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        if (showImageOverlay) showImageOverlay = false;
        else if (showFalsePositiveModal) handleFpModalClose();
        else if (showReportModal) showReportModal = false;
      }
    }
    window.addEventListener('keydown', handleEsc);
    return () => window.removeEventListener('keydown', handleEsc);
  });

  // ── Drag and drop ─────────────────────────────────────────────────
  function handleDragOver(e: DragEvent) { e.preventDefault(); dragOver = true; }
  function handleDragLeave() { dragOver = false; }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const files = e.dataTransfer?.files;
    if (!files?.length) return;
    const file = files[0];
    const path = (file as any).path || file.name;
    if (path === file.name && inTauri) return;
    await runFileVerification(path, file.name);
  }

  async function setupTauriDragDrop() {
    try {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      const webview = getCurrentWebviewWindow();
      _unlistenDragDrop = await webview.onDragDropEvent((event) => {
        if (event.payload.type === 'over') {
          dragOver = true;
        } else if (event.payload.type === 'leave') {
          dragOver = false;
        } else if (event.payload.type === 'drop') {
          dragOver = false;
          const paths = event.payload.paths;
          if (paths?.length > 0) {
            const p = paths[0];
            const n = p.split('/').pop() || p.split('\\').pop() || p;
            runFileVerification(p, n);
          }
        }
      });
    } catch {}
  }

  async function handleFileClick() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        title: 'Select File to Verify',
        filters: [
          // v1.0 = images only. MP4/MOV dropped per JTV-138 (video deepfake
          // returns in v1.0.x). PDF dropped 2026-05-11 — document-format
          // forensics are deferred to v1.1+; v1.0 focuses on image verification.
          { name: 'Supported Files', extensions: ['jpg','jpeg','png','tiff','tif','webp','avif','heic','heif'] },
        ],
      });
      if (selected && typeof selected === 'string') {
        const name = selected.split('/').pop() || selected.split('\\').pop() || selected;
        await runFileVerification(selected, name);
      }
    } catch {}
  }

  // ── Core verification ─────────────────────────────────────────────
  function cancelAnalysis() {
    cancelled = true;
    loading = false;
    error = 'Analysis cancelled.';
    errorType = 'general';
  }

  async function runFileVerification(path: string, name: string) {
    clearVerifySession();
    blobs.revokeAll();
    filePath = path;
    fileName = name;
    result = null;
    checked = false;
    error = null;
    errorType = null;
    cancelled = false;
    loading = true;
    analysisStartTime = Date.now();
    try {
      result = await verifyFile(path, verifyMode);
      if (!cancelled) checked = true;
    } catch (e) {
      if (!cancelled) setError(e, 'File verification failed');
    } finally {
      if (!cancelled) loading = false;
    }
  }

  async function runUrlVerification() {
    const url = urlInput.trim();
    if (!url) return;
    clearVerifySession();
    fileName = url.split('/').pop()?.split('?')[0] || url;
    filePath = null;
    result = null;
    checked = false;
    error = null;
    errorType = null;
    loading = true;
    analysisStartTime = Date.now();
    try {
      result = await verifyUrl(url, verifyMode);
      checked = true;
    } catch (e) {
      setError(e, 'URL verification failed');
    } finally {
      loading = false;
    }
  }

  function reset() {
    filePath = null; fileName = null; urlInput = '';
    result = null; checked = false; error = null;
    loading = false; cancelled = false;
    openCard = null; showImageOverlay = false;
    previewUrl = null;
    extractedText = null; extractTextError = null;
  }

  // ── Export helpers ────────────────────────────────────────────────
  async function triggerDownload(blob: Blob, filename: string) {
    if (inTauri) {
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const { writeFile } = await import('@tauri-apps/plugin-fs');
        const chosen = await save({ defaultPath: filename });
        if (chosen) {
          await writeFile(chosen, new Uint8Array(await blob.arrayBuffer()));
          console.log(`Saved to: ${chosen}`);
        }
        return;
      } catch (e) {
        console.warn('Tauri save dialog failed, falling back to browser download:', e);
      }
    }
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url; a.download = filename; a.click();
    URL.revokeObjectURL(url);
  }

  async function handleExportReport() {
    if (!result || exportingReport) return;
    exportingReport = true;
    try {
      if (analystName.trim()) localStorage.setItem('jura-analyst-name', analystName.trim());
      if (analystOrg.trim()) localStorage.setItem('jura-analyst-org', analystOrg.trim());
      const ctx: ReportContext = {
        analystName: analystName.trim() || undefined,
        organisation: analystOrg.trim() || undefined,
        caseReference: analystCaseRef.trim() || undefined,
        analysisDate: analystDate.trim() || undefined,
      };
      const blob = await generateTrustReport(result, {
        fileName: fileName ?? 'Unknown',
        fileSize: 0,
        analysedAt: new Date().toISOString(),
        analystNote: analystNote.trim() || undefined,
        appVersion,
      }, ctx, reportFormat);
      const ts = Math.floor(Date.now() / 1000);
      const safe = (fileName ?? 'file').replace(/[^a-zA-Z0-9._-]/g, '_');
      triggerDownload(blob, `jura-report-${safe}-${ts}.pdf`);
    } finally {
      exportingReport = false;
      showReportModal = false;
      analystCaseRef = '';
    }
  }

  async function handleExportCase() {
    if (!result || exportingCase) return;
    exportingCase = true;
    try {
      const blob = await exportCaseZip(result, {
        fileName: fileName ?? 'Unknown',
        fileSize: 0,
        exportedAt: new Date().toISOString(),
        appVersion,
      });
      const ts = Math.floor(Date.now() / 1000);
      const safe = (fileName ?? 'file').replace(/[^a-zA-Z0-9._-]/g, '_');
      triggerDownload(blob, `jura-case-${safe}-${ts}.zip`);
    } finally {
      exportingCase = false;
    }
  }

  async function handleFalsePositiveSubmit() {
    if (!result || fpSubmitting) return;
    fpSubmitting = true;
    try {
      // 1. Local SQLite write — only the API fields that are written to disk.
      //    Tier 2 fields (deepfake_score, deepfake_verdict, signalScoresJson)
      //    are intentionally NOT passed: per project_fp_report_v1_locked.md
      //    they stay absent from the v1.0 payload. The Rust API still accepts
      //    them so the contract is forward-compatible; we just stop sending.
      await markFalsePositive(
        fpReasonCode,
        fpReasonNote.trim() || undefined,
        result.contentType,
      );

      // 2. Build the Tier 1 payload for the clipboard / mailto export.
      //    reason_note is deliberately excluded — free text inherently carries
      //    PII risk and the saved-locally write above is the user's audit
      //    trail of that text. Nothing else leaves the device automatically.
      fpPayload = buildFpPayload(fpReasonCode, result.contentType, appVersion);

      // 3. Best-effort attempt to launch the user's mail client. There is no
      //    reliable way to detect whether the OS actually handled the URI, so
      //    the success state always renders the clipboard fallback as well.
      openFpMailto(fpPayload);

      fpSubmitted = true;
      // Do NOT auto-close the modal — the user needs the clipboard / email
      // fallback to remain visible.
    } finally {
      fpSubmitting = false;
    }
  }

  async function handleFpCopyToClipboard() {
    if (!fpPayload) return;
    fpClipboardCopied = await copyFpReport(fpPayload);
    if (fpClipboardCopied) {
      setTimeout(() => { fpClipboardCopied = false; }, 2000);
    }
  }

  function handleFpModalClose() {
    showFalsePositiveModal = false;
    fpSubmitted = false;
    fpPayload = null;
    fpClipboardCopied = false;
    fpReasonCode = 'modern_codec';
    fpReasonNote = '';
  }

  // ── Signal dot scroll ─────────────────────────────────────────────
  function jumpToCard(cardId: 'provenance' | 'integrity' | 'ai' | 'claims') {
    openCard = cardId;
    requestAnimationFrame(() => {
      const el = document.getElementById(`card-${cardId}`);
      if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  }

  // ── Dot colour helpers ─────────────────────────────────────────────
  // Locked 2026-05-10: green=positive, amber=concern/uncertain, red=explicit
  // suspicious/manipulation evidence, grey=no signal to surface.  Mirrors the
  // overall-trust pill colour vocabulary for consistency.
  function dotBgClass(state: DotState): string {
    if (state === 'pass') return 'bg-malachite dark:bg-malachite-light';
    if (state === 'concern') return 'bg-amber dark:bg-amber-light';
    if (state === 'suspicious') return 'bg-cinnabar dark:bg-cinnabar-light';
    if (state === 'suppressed') return 'bg-flint/40';
    if (state === 'empty-data') return 'bg-flint/30';
    return 'bg-flint/30';
  }

  function dotPulse(state: DotState): boolean {
    return state === 'suspicious' || state === 'concern';
  }

  function cardPassClass(pass: boolean | null): string {
    if (pass === null) return 'text-flint-dark dark:text-flint-light';
    if (pass) return 'text-malachite-dark dark:text-malachite-light';
    return 'text-amber-dark dark:text-amber-light';
  }

  function highestSeverityFindings(findings: AnomalyFinding[]): AnomalyFinding[] {
    const order: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
    return [...findings].sort((a, b) => (order[a.severity] ?? 5) - (order[b.severity] ?? 5));
  }

  // ── Batch helpers ──────────────────────────────────────────────────
  const batchCompleted = $derived(batchItems.filter(i => i.status === 'done' || i.status === 'error').length);
  const batchQueued = $derived(batchItems.filter(i => i.status === 'queued').length);

  function addBatchFiles(files: { filePath: string; fileName: string }[]) {
    const existing = new Set(batchItems.map(i => i.filePath));
    const newItems: BatchItem[] = files
      .filter(f => !existing.has(f.filePath))
      .map(f => ({
        id: `${Date.now()}-${Math.random()}`,
        filePath: f.filePath,
        fileName: f.fileName,
        status: 'queued' as BatchItemStatus,
        result: null,
        error: null,
        startedAt: null,
        finishedAt: null,
      }));
    batchItems = [...batchItems, ...newItems];
  }

  async function handleBatchDrop(e: DragEvent) {
    e.preventDefault();
    batchDragOver = false;
    const files = e.dataTransfer?.files;
    if (!files?.length) return;
    addBatchFiles(Array.from(files).map(f => ({ filePath: (f as any).path || f.name, fileName: f.name })));
  }

  async function handleBatchBrowse() {
    const files = await openBatchFileDialog();
    if (files.length) addBatchFiles(files);
  }

  async function runBatch() {
    if (batchRunning) return;
    batchRunning = true;

    async function processItem(item: BatchItem) {
      batchItems = batchItems.map(i => i.id === item.id ? { ...i, status: 'running' as BatchItemStatus, startedAt: Date.now() } : i);
      try {
        const res = await verifyFile(item.filePath, verifyMode);
        batchItems = batchItems.map(i => i.id === item.id ? { ...i, status: 'done' as BatchItemStatus, result: res, finishedAt: Date.now() } : i);
      } catch (err: any) {
        batchItems = batchItems.map(i => i.id === item.id ? { ...i, status: 'error' as BatchItemStatus, error: err?.message ?? 'Unknown error', finishedAt: Date.now() } : i);
      }
    }

    const queued = batchItems.filter(i => i.status === 'queued');
    for (const item of queued) {
      await processItem(item);
    }
    batchRunning = false;
  }

  function removeBatchItem(id: string) {
    batchItems = batchItems.filter(i => i.id !== id);
    if (expandedBatchId === id) expandedBatchId = null;
  }

  function clearBatch() {
    batchItems = [];
    expandedBatchId = null;
  }

  function downloadBatchReport() {
    const completed = batchItems.filter(i => i.status === 'done' && i.result);
    if (completed.length === 0) return;
    const dateStr = new Date().toISOString().slice(0, 10);
    const csvHeaders = 'Filename,Verdict,Trust Score,Mode,Date\n';
    const csvRows = completed.map(item => {
      const r = item.result!;
      const verdict = r.deepfakeResult?.verdictLevel ?? (r.overallTrust >= 0.7 ? 'authentic' : r.overallTrust >= 0.4 ? 'inconclusive' : 'synthetic');
      const trust = Math.round(r.overallTrust * 100);
      const mode = (r as any).mode ?? verifyMode;
      const date = item.finishedAt ? new Date(item.finishedAt).toISOString().slice(0, 10) : dateStr;
      const name = item.fileName.replace(/"/g, '""');
      return `"${name}","${verdict}",${trust},"${mode}","${date}"`;
    }).join('\n');
    const blob = new Blob([csvHeaders + csvRows], { type: 'text/csv;charset=utf-8;' });
    const dlUrl = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = dlUrl;
    a.download = `jura-batch-results-${dateStr}.csv`;
    a.click();
    URL.revokeObjectURL(dlUrl);
  }

  // ── Read Text (Ollama LLaVA) ───────────────────────────────────────
  async function handleExtractText() {
    if (!filePath || extractingText) return;
    extractingText = true;
    extractedText = null;
    extractTextError = null;
    try {
      const text = await extractTextFromImage(filePath);
      if (text) {
        extractedText = text;
      } else {
        // JTV-132 (2026-05-02): passive copy, not warning. Ollama is optional
        // enrichment for v1.0 — this feature only fails when the user has
        // partially configured it. Point them to Settings rather than alarm.
        extractTextError = 'Optional feature — install Ollama and the llava:7b model from Settings to enable this.';
      }
    } catch {
      extractTextError = 'Optional feature — Ollama appears offline. See Settings to install it.';
    } finally {
      extractingText = false;
    }
  }
</script>

<!-- ─── Full-size image overlay ────────────────────────────────────── -->
{#if showImageOverlay && previewUrl}
  <div
    class="fixed inset-0 z-[100] bg-black/85 flex items-center justify-center cursor-zoom-out"
    role="dialog"
    aria-label="Full-size image preview — press Escape to close"
    aria-modal="true"
    onclick={() => showImageOverlay = false}
  >
    <img
      src={previewUrl}
      alt="Full-size preview of {fileName}"
      class="max-w-[90vw] max-h-[90vh] rounded-lg shadow-2xl"
      onclick={(e) => e.stopPropagation()}
    />
    <button
      class="absolute top-4 right-4 w-10 h-10 flex items-center justify-center rounded-full bg-white/80 dark:bg-graphite/80 text-obsidian dark:text-quartz
             hover:bg-white dark:hover:bg-graphite focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
      aria-label="Close image preview"
      onclick={() => showImageOverlay = false}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>
{/if}

<!-- ─── Export report modal ────────────────────────────────────────── -->
{#if showReportModal}
  <div
    class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4"
    role="dialog"
    aria-label="Export trust report"
    aria-modal="true"
    onclick={(e) => { if (e.target === e.currentTarget) showReportModal = false; }}
  >
    <div class="bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl p-6 w-full max-w-md space-y-4">
      <h2 class="font-serif text-lg text-obsidian dark:text-quartz">Export Trust Report</h2>
      <div class="space-y-3">
        <div>
          <label for="v2-analyst-name" class="block text-xs text-flint-dark dark:text-flint-light mb-1">Analyst name (optional)</label>
          <input id="v2-analyst-name" type="text" bind:value={analystName}
            class="w-full bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded px-3 py-2 text-sm text-obsidian dark:text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light" />
        </div>
        <div>
          <label for="v2-analyst-org" class="block text-xs text-flint-dark dark:text-flint-light mb-1">Organisation (optional)</label>
          <input id="v2-analyst-org" type="text" bind:value={analystOrg}
            class="w-full bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded px-3 py-2 text-sm text-obsidian dark:text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light" />
        </div>
        <div>
          <label for="v2-case-ref" class="block text-xs text-flint-dark dark:text-flint-light mb-1">Case reference (optional)</label>
          <input id="v2-case-ref" type="text" bind:value={analystCaseRef}
            class="w-full bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded px-3 py-2 text-sm text-obsidian dark:text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light" />
        </div>
        <div>
          <label for="v2-report-format" class="block text-xs text-flint-dark dark:text-flint-light mb-1">Format</label>
          <select id="v2-report-format" bind:value={reportFormat}
            class="w-full bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded px-3 py-2 text-sm text-obsidian dark:text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light">
            <option value="standard">Standard</option>
            <option value="detailed">Detailed</option>
          </select>
        </div>
      </div>
      <div class="flex gap-3 justify-end pt-2">
        <button
          class="px-4 py-2 min-h-[44px] text-sm text-flint-dark dark:text-flint-light border border-border-light dark:border-border-dark rounded-lg hover:text-obsidian dark:hover:text-quartz transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
          onclick={() => showReportModal = false}
        >Cancel</button>
        <button
          class="px-4 py-2 min-h-[44px] text-sm bg-lapis text-white rounded-lg font-medium hover:bg-lapis-dark transition-colors disabled:opacity-50
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
          disabled={exportingReport}
          onclick={handleExportReport}
        >{exportingReport ? 'Generating…' : 'Export PDF'}</button>
      </div>
    </div>
  </div>
{/if}

<!-- ─── False positive modal ───────────────────────────────────────── -->
{#if showFalsePositiveModal}
  <div
    class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4"
    role="dialog"
    aria-label="Report false positive"
    aria-modal="true"
    onclick={(e) => { if (e.target === e.currentTarget) handleFpModalClose(); }}
  >
    <div class="bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl p-6 w-full max-w-md space-y-4">
      <h2 class="font-serif text-lg text-obsidian dark:text-quartz">Report False Positive</h2>
      {#if fpSubmitted}
        <!-- Success state — user has saved locally + a mailto launch was attempted.
             We cannot reliably detect whether the OS handled the mailto:, so the
             clipboard fallback is always offered alongside the verbal hint. -->
        <div role="status" aria-live="polite" class="space-y-3">
          <p class="text-malachite-dark dark:text-malachite-light text-sm">
            ✓ Saved locally.
          </p>
          <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
            Your email client should have opened with a pre-filled report. Review and send
            it to contribute this case to model improvement.
          </p>
          <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
            If nothing opened, you can copy the report and email it manually:
          </p>
          <div class="flex flex-wrap gap-2 items-center pt-1">
            <button
              type="button"
              class="px-3 py-2 min-h-[44px] text-xs bg-gray-100 dark:bg-obsidian border border-border-light dark:border-border-dark rounded-lg text-obsidian dark:text-quartz hover:bg-gray-200 dark:hover:bg-graphite-dark transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
              onclick={handleFpCopyToClipboard}
              aria-live="polite"
            >{fpClipboardCopied ? '✓ Copied' : 'Copy report to clipboard'}</button>
            <span class="text-xs text-flint-dark dark:text-flint-light">Send to</span>
            <a
              href="mailto:{FP_FEEDBACK_EMAIL}"
              class="text-xs text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded"
            >{FP_FEEDBACK_EMAIL}</a>
          </div>
          <p class="text-[11px] text-flint-dark dark:text-flint-light leading-relaxed pt-2 border-t border-border-light dark:border-border-dark">
            The exported report contains only the reason code, MIME type, app version,
            platform, and timestamp. Your free-text notes stay on this device.
          </p>
          <div class="flex justify-end pt-2">
            <button
              class="px-4 py-2 min-h-[44px] text-sm text-flint-dark dark:text-flint-light border border-border-light dark:border-border-dark rounded-lg hover:text-obsidian dark:hover:text-quartz transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
              onclick={handleFpModalClose}
            >Close</button>
          </div>
        </div>
      {:else}
        <div class="space-y-3">
          <div>
            <label for="v2-fp-reason" class="block text-xs text-flint-dark dark:text-flint-light mb-1">Reason</label>
            <select id="v2-fp-reason" bind:value={fpReasonCode}
              class="w-full bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded px-3 py-2 text-sm text-obsidian dark:text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light">
              <option value="modern_codec">Modern codec (AVIF/WebP)</option>
              <option value="high_iso">High ISO / grain</option>
              <option value="heavy_editing">Heavy post-processing</option>
              <option value="scanner">Scanned document</option>
              <option value="other">Other</option>
            </select>
          </div>
          <div>
            <label for="v2-fp-note" class="block text-xs text-flint-dark dark:text-flint-light mb-1">
              Additional notes (optional, max 500 characters)
            </label>
            <textarea id="v2-fp-note" bind:value={fpReasonNote} rows="3" maxlength="500"
              placeholder="Do not include personal data — notes are stored locally only and are NOT included in any emailed or copied report."
              class="w-full bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded px-3 py-2 text-sm text-obsidian dark:text-quartz resize-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"></textarea>
          </div>
          <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
            Your report is saved on this device. Clicking <strong>Save &amp; prepare email</strong>
            will also open your email client with a pre-filled draft — nothing is sent
            automatically. You choose whether to send it.
          </p>
        </div>
        <div class="flex gap-3 justify-end pt-2">
          <button
            class="px-4 py-2 min-h-[44px] text-sm text-flint-dark dark:text-flint-light border border-border-light dark:border-border-dark rounded-lg hover:text-obsidian dark:hover:text-quartz transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
            onclick={handleFpModalClose}
          >Cancel</button>
          <button
            class="px-4 py-2 min-h-[44px] text-sm bg-lapis text-white rounded-lg font-medium hover:bg-lapis-dark transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
            disabled={fpSubmitting}
            onclick={handleFalsePositiveSubmit}
          >{fpSubmitting ? 'Saving…' : 'Save & prepare email'}</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- ─── Page ───────────────────────────────────────────────────────── -->
<main id="main-content" tabindex="-1" class="outline-none max-w-[900px] mx-auto px-6 py-8 pb-16">

  <!-- Header -->
  <div class="flex items-center gap-3 mb-6">
    <h1 class="font-serif text-2xl text-obsidian dark:text-quartz">Verify</h1>
  </div>

  <!-- EnhancedModeBanner removed 2026-04-25 — Enhanced is now the
       default NetworkMode (set in src-tauri/src/network_mode.rs::Default
       and the get_network_mode fallbacks), so first-run users already
       get full validation without a nudge. -->

  <!-- Mode selector + sidecar status row -->
  <div class="flex items-center gap-3 mb-5 flex-wrap">
    <div role="radiogroup" aria-label="Verification mode" class="flex rounded-lg border border-border-light dark:border-border-dark overflow-hidden">
      {#each [
        { mode: 'standard' as VerifyMode, label: 'Standard', description: '~15s' },
        { mode: 'deep' as VerifyMode, label: 'Deep', description: '~60s' },
        // Archival mode removed 2026-04-22 — it ran the same pipeline as Deep
        // (src-tauri/src/lib.rs:1986) and the advertised 40-frame video
        // extraction was capped at 12 by the sidecar.  Re-introduce once
        // proper differentiation lands (uncapped frames, scanner-calibrated
        // tolerances).  The 'archival' VerifyMode value is retained in
        // `lib/types.ts` for API back-compat and aliases to 'deep' in Rust.
      ] as opt}
        <button
          class="px-3 py-1.5 text-xs font-medium transition-colors min-h-[36px]
                 {verifyMode === opt.mode
                   ? 'bg-lapis text-white dark:bg-lapis-light dark:text-obsidian'
                   : 'text-flint-dark dark:text-flint-light hover:text-obsidian dark:hover:text-quartz'}
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light"
          role="radio"
          aria-checked={verifyMode === opt.mode}
          onclick={() => { verifyMode = opt.mode; localStorage.setItem('jura-verify-mode', opt.mode); }}
        >{opt.label} <span class="ml-0.5 {verifyMode === opt.mode ? 'text-white dark:text-obsidian' : 'text-flint-dark dark:text-flint-light'}">{opt.description}</span></button>
      {/each}
    </div>
    <ContextualHelpLink
      href="/help/verify#investigation-modes"
      label="Learn about investigation modes"
    />

    <div
      class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-full border
             {sidecarAvailable
               ? 'bg-malachite/10 text-malachite-dark dark:text-malachite-light border-malachite/20'
               : 'bg-white dark:bg-graphite text-flint-dark dark:text-flint-light border-border-light dark:border-border-dark'}"
      role="status"
      aria-label={sidecarAvailable ? 'Analysis services connected' : 'Analysis services offline'}
    >
      <span class="w-1.5 h-1.5 rounded-full {sidecarAvailable ? 'bg-malachite' : 'bg-flint/50'}" aria-hidden="true"></span>
      {sidecarAvailable ? 'Services connected' : 'Services offline'}
    </div>

    {#if checked && result}
      <button
        class="text-xs text-flint-dark dark:text-flint-light hover:text-obsidian dark:hover:text-quartz transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded px-2 py-1"
        onclick={reset}
        aria-label="Clear result and verify a new file"
      >Clear result</button>
    {/if}
  </div>

  <!-- Error banner -->
  {#if error}
    <div
      class="mb-5 rounded-lg px-4 py-3 text-sm border
        {errorType === 'sidecar' ? 'bg-amber/10 border-amber/30 text-amber-dark dark:text-amber-light' :
         errorType === 'format'  ? 'bg-lapis/10 border-lapis/30 text-lapis dark:text-lapis-light' :
         'bg-cinnabar/10 border-cinnabar/30 text-cinnabar-dark dark:text-cinnabar-light'}"
      role="alert"
      aria-live="assertive"
      data-testid="error-banner"
      data-error-code={errorType}
    >
      <span class="font-medium">{errorType === 'sidecar' ? 'Analysis Engine offline' : errorType === 'format' ? 'Unsupported format' : errorType === 'network' ? 'Network error' : 'Error'}:</span>
      {error}
    </div>
  {/if}

  <!-- ── Input panel ─────────────────────────────────────────────── -->
  {#if !checked || !result}
    <div class="mb-6 bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl overflow-hidden">
      <!-- Tab bar -->
      <div role="tablist" aria-label="Verification input method" class="flex border-b border-border-light dark:border-border-dark">
        {#each [
          { id: 'file', label: 'File' },
          { id: 'batch', label: 'Batch', badge: batchItems.length > 0 ? batchItems.length : null },
          { id: 'url', label: 'URL' },
        ] as tab}
          <button
            class="px-5 py-3 text-sm font-medium border-b-2 -mb-px transition-colors flex items-center gap-1.5
                   {activeTab === tab.id
                     ? 'text-lapis dark:text-lapis-light border-lapis-light'
                     : 'text-flint-dark dark:text-flint-light border-transparent hover:text-obsidian dark:hover:text-quartz'}
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light"
            role="tab"
            aria-selected={activeTab === tab.id}
            aria-controls="v2-tab-{tab.id}"
            id="v2-tab-btn-{tab.id}"
            onclick={() => activeTab = tab.id as 'file' | 'batch' | 'url'}
          >
            {tab.label}
            {#if (tab as any).badge}
              <span class="text-[10px] text-flint-dark dark:text-flint-light" aria-label="{(tab as any).badge} files queued">({(tab as any).badge})</span>
            {/if}
          </button>
        {/each}
      </div>

      <!-- File tab -->
      {#if activeTab === 'file'}
        <div id="v2-tab-file" role="tabpanel" aria-labelledby="v2-tab-btn-file" class="p-4">
          <button
            class="w-full border-2 border-dashed rounded-lg p-10 text-center transition-all duration-200 cursor-pointer
                   {dragOver ? 'border-lapis-light bg-lapis/5' : 'border-border-light dark:border-border-dark hover:border-lapis/50'}
                   {loading ? 'opacity-60 pointer-events-none' : ''}
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
            ondragover={handleDragOver}
            ondragleave={handleDragLeave}
            ondrop={handleDrop}
            onclick={handleFileClick}
            aria-busy={loading}
          >
            {#if loading}
              <div class="flex flex-col items-center gap-3">
                <div class="w-6 h-6 border-2 border-lapis-light border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Analysing"></div>
                <p class="text-sm text-obsidian dark:text-quartz font-medium">
                  {#if powerSaverEnabled && analysisElapsed >= 5}
                    Restarting analysis engine…
                  {:else}
                    {verifyMode === 'deep' ? 'Running deep analysis — up to 60 seconds…' : 'Running standard analysis…'}
                  {/if}
                </p>
                {#if fileName}
                  <p class="text-xs text-flint-dark dark:text-flint-light">{fileName}</p>
                {/if}
                <!-- Power-saver disclosure shown immediately (not after 5 s) so a
                     respawn pause is recognisable as expected behaviour from the
                     start of the verify, not 5 s in.  Persona-testing 30 April
                     2026 flagged the silent first 5 s as a crash-look that
                     drives force-quits during respawn. -->
                {#if powerSaverEnabled && analysisElapsed < 5}
                  <p class="text-xs text-flint-dark dark:text-flint-light max-w-xs text-center">
                    Power-saver mode is on. If the engine was idle, the first verification may take an extra 30–90 seconds.
                  </p>
                {/if}
                {#if analysisElapsed > 2}
                  <p class="text-xs text-flint-dark dark:text-flint-light tabular-nums">{analysisElapsed}s elapsed</p>
                {/if}
              </div>
            {:else}
              <div class="flex flex-col items-center gap-2">
                <svg class="w-10 h-10 text-flint-dark dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                    d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>
                </svg>
                <p class="text-obsidian dark:text-quartz font-medium">Drop a file to verify</p>
                <p class="text-xs text-flint-dark dark:text-flint-light">or click to browse</p>
                <p class="text-xs text-flint-dark dark:text-flint-light mt-1">JPEG · PNG · TIFF · WebP · HEIC · AVIF · PDF · MP4 · MOV</p>
              </div>
            {/if}
          </button>
          {#if loading}
            <div class="mt-3 flex justify-center">
              <button
                class="text-xs text-flint-dark dark:text-flint-light hover:text-cinnabar-dark dark:text-cinnabar-light dark:hover:text-cinnabar-light transition-colors px-3 py-1.5 min-h-[36px]
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded"
                onclick={cancelAnalysis}
              >Cancel analysis</button>
            </div>
          {/if}
        </div>
      {/if}

      <!-- URL tab -->
      {#if activeTab === 'url'}
        <div id="v2-tab-url" role="tabpanel" aria-labelledby="v2-tab-btn-url" class="p-4">
          <label for="v2-url-input" class="block text-xs text-flint-dark dark:text-flint-light mb-2">
            Image or media URL
          </label>
          <div class="flex gap-2">
            <input
              id="v2-url-input"
              type="url"
              bind:value={urlInput}
              placeholder="https://example.com/image.jpg"
              disabled={loading}
              onkeydown={(e) => { if (e.key === 'Enter' && urlInput.trim() && !loading) runUrlVerification(); }}
              class="flex-1 bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded-lg px-3 py-2.5 text-sm text-obsidian dark:text-quartz placeholder:text-flint-dark dark:text-flint-light dark:placeholder:text-flint-light/50
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light disabled:opacity-50"
            />
            <button
              class="px-4 py-2.5 min-h-[44px] bg-lapis text-white text-sm font-medium rounded-lg hover:bg-lapis-dark transition-colors disabled:opacity-50
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
              disabled={!urlInput.trim() || loading}
              onclick={runUrlVerification}
            >{loading ? 'Analysing…' : 'Verify'}</button>
          </div>
        </div>
      {/if}

      <!-- Batch tab -->
      {#if activeTab === 'batch'}
        <div id="v2-tab-batch" role="tabpanel" aria-labelledby="v2-tab-btn-batch" class="p-4">
          <!-- Drop zone -->
          <button
            class="w-full border-2 border-dashed rounded-lg p-8 text-center transition-all duration-200 cursor-pointer
                   {batchDragOver ? 'border-lapis-light bg-lapis/5' : 'border-border-light dark:border-border-dark hover:border-lapis/50'}
                   {batchRunning ? 'opacity-60 pointer-events-none' : ''}
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
            ondragover={(e) => { e.preventDefault(); batchDragOver = true; }}
            ondragleave={() => { batchDragOver = false; }}
            ondrop={handleBatchDrop}
            onclick={handleBatchBrowse}
            aria-label="Drop multiple files to verify, or click to browse for batch verification"
          >
            <div class="flex flex-col items-center gap-2">
              <svg class="w-8 h-8 text-flint-dark dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                  d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
              </svg>
              <p class="text-obsidian dark:text-quartz font-medium">Drop multiple files to verify</p>
              <p class="text-xs text-flint-dark dark:text-flint-light">or click to browse — files are queued for sequential verification</p>
            </div>
          </button>

          <!-- Batch controls -->
          {#if batchItems.length > 0}
            <div class="flex items-center justify-between mt-4 flex-wrap gap-3">
              <div class="flex items-center gap-3">
                <button
                  class="px-4 py-2.5 min-h-[44px] bg-lapis text-white text-sm font-medium rounded-lg hover:bg-lapis-dark transition-colors
                         disabled:opacity-50 disabled:cursor-not-allowed
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
                  onclick={runBatch}
                  disabled={batchRunning || batchQueued === 0}
                >
                  {batchRunning ? 'Running…' : 'Run Batch'}
                </button>
                <span class="text-xs text-flint-dark dark:text-flint-light">{batchCompleted} of {batchItems.length} complete</span>
              </div>
              <div class="flex items-center gap-2">
                {#if batchCompleted > 0}
                  <button
                    class="text-xs px-3 py-2 min-h-[44px] inline-flex items-center gap-1.5 rounded border border-malachite/50
                           text-malachite-dark dark:text-malachite-light hover:bg-malachite/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
                    onclick={downloadBatchReport}
                    aria-label="Download batch verification results as a CSV spreadsheet"
                  >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 16v-8m0 8l-3-3m3 3l3-3M4 20h16" />
                    </svg>
                    Download CSV
                  </button>
                {/if}
                <button
                  class="text-xs text-flint-dark dark:text-flint-light hover:text-obsidian dark:hover:text-quartz transition-colors px-2 py-1 rounded
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
                  onclick={clearBatch}
                  disabled={batchRunning}
                >Clear all</button>
              </div>
            </div>

            <!-- Results table -->
            <div class="mt-4 bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded-lg overflow-x-auto">
              <div class="grid grid-cols-[1fr_90px_70px_70px_36px] gap-3 px-4 py-2 border-b border-border-light dark:border-border-dark
                          text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wide min-w-[480px]">
                <span>File</span>
                <span>Status</span>
                <span>Trust</span>
                <span>Duration</span>
                <span></span>
              </div>
              {#each batchItems as item (item.id)}
                <div class="border-b border-border-light dark:border-border-dark/50 last:border-0 min-w-[480px]">
                  <div
                    class="w-full grid grid-cols-[1fr_90px_70px_70px_36px] gap-3 px-4 py-2.5 text-left
                           {item.status === 'done' ? 'cursor-pointer hover:bg-white/[0.03]' : ''}
                           {expandedBatchId === item.id ? 'bg-lapis/5' : ''}"
                    onclick={() => { if (item.status === 'done') expandedBatchId = expandedBatchId === item.id ? null : item.id; }}
                    onkeydown={(e) => { if ((e.key === 'Enter' || e.key === ' ') && item.status === 'done') { e.preventDefault(); expandedBatchId = expandedBatchId === item.id ? null : item.id; } }}
                    role={item.status === 'done' ? 'button' : undefined}
                    tabindex={item.status === 'done' ? 0 : undefined}
                    aria-expanded={item.status === 'done' ? expandedBatchId === item.id : undefined}
                  >
                    <span class="text-sm text-obsidian dark:text-quartz truncate self-center" title={item.filePath}>{item.fileName}</span>
                    <span class="text-xs self-center">
                      {#if item.status === 'queued'}
                        <span class="text-flint-dark dark:text-flint-light">Queued</span>
                      {:else if item.status === 'running'}
                        <span class="flex items-center gap-1.5">
                          <span class="w-3 h-3 border-2 border-lapis-light border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Verifying"></span>
                          <span class="text-lapis dark:text-lapis-light">Running</span>
                        </span>
                      {:else if item.status === 'done'}
                        {@const lv = getTrustLevel(item.result?.overallTrust ?? 0)}
                        <span class="font-medium px-1.5 py-0.5 rounded
                          {lv === 'high' ? 'text-malachite-dark dark:text-malachite-light bg-malachite/10'
                           : lv === 'medium' ? 'text-amber-dark dark:text-amber-light bg-amber/10'
                           : 'text-cinnabar-dark dark:text-cinnabar-light bg-cinnabar/10'}">Done</span>
                      {:else}
                        <span class="text-cinnabar-dark dark:text-cinnabar-light">Error</span>
                      {/if}
                    </span>
                    <span class="text-xs tabular-nums self-center">
                      {#if item.status === 'done' && item.result}
                        {@const lv = getTrustLevel(item.result.overallTrust)}
                        <span class="{lv === 'high' ? 'text-malachite-dark dark:text-malachite-light' : lv === 'medium' ? 'text-amber-dark dark:text-amber-light' : 'text-cinnabar-dark dark:text-cinnabar-light'}">
                          {Math.round(item.result.overallTrust * 100)}%
                        </span>
                      {:else}
                        <span class="text-flint-dark dark:text-flint-light">—</span>
                      {/if}
                    </span>
                    <span class="text-xs text-flint-dark dark:text-flint-light tabular-nums self-center">
                      {#if item.startedAt && item.finishedAt}
                        {formatDuration(item.startedAt, item.finishedAt)}
                      {:else}—{/if}
                    </span>
                    <button
                      class="w-9 h-9 flex items-center justify-center rounded hover:bg-white/10 text-flint-dark dark:text-flint-light hover:text-cinnabar-light transition-colors self-center
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
                      onclick={(e) => { e.stopPropagation(); removeBatchItem(item.id); }}
                      aria-label="Remove {item.fileName} from queue"
                      disabled={batchRunning && item.status === 'running'}
                    >
                      <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
                      </svg>
                    </button>
                  </div>
                  <!-- Expanded result row -->
                  {#if expandedBatchId === item.id && item.result}
                    <div class="px-4 py-3 border-t border-border-light dark:border-border-dark/40 bg-gray-50 dark:bg-obsidian/50 text-xs space-y-1">
                      <p class="text-flint-dark dark:text-flint-light">
                        Detectors:
                        {[item.result.exifAnalysis, item.result.elaResult, item.result.noiseResult, item.result.copyMoveResult, item.result.deepfakeResult].filter(Boolean).length} ran
                        {#if item.result.exifAnalysis?.findings.some((f: AnomalyFinding) => f.severity === 'high' || f.severity === 'critical')}
                          · <span class="text-amber-dark dark:text-amber-light">EXIF anomalies</span>
                        {/if}
                        {#if item.result.elaResult?.suspicious}
                          · <span class="text-amber-dark dark:text-amber-light">ELA concern</span>
                        {/if}
                        {#if item.result.deepfakeResult?.suspicious}
                          · <span class="text-amber-dark dark:text-amber-light">AI detection concern</span>
                        {/if}
                      </p>
                    </div>
                  {/if}
                  {#if item.status === 'error' && item.error}
                    <div class="px-4 py-2 border-t border-cinnabar/20 bg-cinnabar/5 text-xs text-cinnabar-dark dark:text-cinnabar-light">
                      {item.error}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- ═══════════════════════════════════════════════════════════════ -->
  <!-- RESULTS — shown only after a successful verification            -->
  <!-- ═══════════════════════════════════════════════════════════════ -->
  {#if checked && result}

    <!-- ── Content-type suppression banner ───────────────────────── -->
    {#if aiDetectionSuppressed && result.contentTypeResult}
      <div
        class="mb-4 px-4 py-3 rounded-xl border bg-amber/10 border-amber/25"
        role="note"
        aria-label="AI detection suppressed"
        aria-live="polite"
      >
        <div class="flex items-start gap-3">
          <svg class="flex-shrink-0 mt-0.5 text-amber-light" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>
          </svg>
          <div class="flex-1 min-w-0">
            <p class="text-sm text-amber-light">
              AI detection has been suppressed — this file appears to be
              <strong class="font-medium">{result.contentTypeResult.category}</strong>
              content (confidence: {Math.round(result.contentTypeResult.confidence * 100)}%). AI-detection models are calibrated for photographs and may produce unreliable results for this content type.
            </p>
            <details class="mt-2 group">
              <summary class="list-none text-xs text-amber-dark dark:text-amber-light/70 cursor-pointer flex items-center gap-1.5 hover:text-amber-light transition-colors min-h-[24px]
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber rounded" aria-label="Why was AI detection suppressed?">
                <svg class="w-3 h-3 motion-safe:group-open:rotate-90 transition-transform duration-150 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M9 5l7 7-7 7"/>
                </svg>
                Why?
              </summary>
              <p class="mt-2 text-xs text-amber-dark dark:text-amber-light/65 leading-relaxed">{result.contentTypeResult.reasoning}</p>
            </details>
          </div>
        </div>
      </div>
    {/if}

    <!-- ── 1. Trust Score Banner ───────────────────────────────────── -->
    <section aria-label="Verification result summary" class="mb-4">
      <div class="bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl p-6 flex items-center gap-6 flex-wrap sm:flex-nowrap">

        <!-- Media preview — kind-aware: image (clickable enlarge), video
             (native controls + native fullscreen), audio (native controls),
             or a placeholder when the codec isn't webview-renderable. -->
        {#if currentPreviewKind === 'image' && previewUrl}
          <button
            class="flex-shrink-0 w-[200px] h-[150px] rounded-lg overflow-hidden border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian relative group
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
            aria-label="Image preview — click to enlarge"
            onclick={() => showImageOverlay = true}
          >
            <img
              src={previewUrl}
              alt="Preview of {fileName}"
              class="w-full h-full object-cover block"
            />
            <span
              class="absolute bottom-2 right-2 bg-black/60 text-obsidian dark:text-quartz text-[10px] px-1.5 py-0.5 rounded opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity duration-150"
              aria-hidden="true"
            >Enlarge</span>
          </button>
        {:else if currentPreviewKind === 'video' && previewUrl}
          <div
            class="flex-shrink-0 w-[200px] h-[150px] rounded-lg overflow-hidden border border-border-light dark:border-border-dark bg-black relative"
          >
            <!-- preload="metadata" only fetches the moov atom (~hundreds of
                 KB); fullscreen and seeking come from the webview controls. -->
            <!-- svelte-ignore a11y_media_has_caption — local user-supplied media,
                 captions not authored by us; transcription appears in the
                 "What does it claim?" card when available. -->
            <video
              src={previewUrl}
              controls
              preload="metadata"
              muted
              class="w-full h-full object-contain bg-black"
              aria-label="Video preview of {fileName}"
            ></video>
          </div>
        {:else if currentPreviewKind === 'audio' && previewUrl}
          <div
            class="flex-shrink-0 w-[200px] h-[150px] rounded-lg overflow-hidden border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian flex flex-col items-center justify-center gap-2 px-3"
          >
            <svg class="w-8 h-8 text-flint-dark dark:text-flint-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M9 18V5l12-2v13" />
              <circle cx="6" cy="18" r="3" />
              <circle cx="18" cy="16" r="3" />
            </svg>
            <!-- svelte-ignore a11y_media_has_caption — see video note above. -->
            <audio
              src={previewUrl}
              controls
              preload="metadata"
              class="w-full"
              aria-label="Audio preview of {fileName}"
            ></audio>
          </div>
        {:else if fileName && (currentPreviewKind === 'unsupported' || (!previewUrl && (result?.contentType === 'video' || result?.contentType === 'audio' || result?.contentType === 'image')))}
          <div
            class="flex-shrink-0 w-[200px] h-[150px] rounded-lg overflow-hidden border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian flex flex-col items-center justify-center gap-2 px-4 text-center"
            role="img"
            aria-label="Preview unavailable for this file format"
          >
            <svg class="w-7 h-7 text-flint-dark dark:text-flint-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="3" y="3" width="18" height="18" rx="2" />
              <line x1="3" y1="9" x2="21" y2="9" />
              <line x1="9" y1="21" x2="9" y2="9" />
            </svg>
            <p class="text-[11px] text-flint-dark dark:text-flint-light leading-tight">
              Preview unavailable for this codec.<br />Analysis still ran.
            </p>
          </div>
        {/if}

        <!-- Trust ring -->
        <div class="flex-shrink-0 relative w-[110px] h-[110px]" role="img" aria-label="Trust score: {trustScorePercent}%">
          <svg class="w-full h-full -rotate-90" viewBox="0 0 100 100" aria-hidden="true" focusable="false">
            <circle cx="50" cy="50" r="47" fill="none" stroke="rgba(255,255,255,0.08)" stroke-width="9"/>
            <circle
              cx="50" cy="50" r="47"
              fill="none"
              stroke={trustStrokeColor()}
              stroke-width="9"
              stroke-linecap="round"
              stroke-dasharray="{RING_CIRC}"
              stroke-dashoffset="{ringDashoffset}"
              class="motion-safe:transition-[stroke-dashoffset] motion-safe:duration-1000 motion-safe:ease-out"
            />
          </svg>
          <div class="absolute inset-0 flex flex-col items-center justify-center" aria-hidden="true">
            <span class="font-serif text-2xl leading-none {trustColorClass()}">{trustScorePercent}<span class="text-sm">%</span></span>
          </div>
        </div>

        <!-- Meta -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2 mb-1 flex-wrap">
            <h2 class="font-serif text-xl text-obsidian dark:text-quartz">{trustLabelText()}</h2>
            {#if trustLevel()}
              <span class="px-2 py-0.5 text-[10px] font-bold tracking-widest uppercase rounded-full border {verdictBadgeClass()}" role="status" aria-live="polite">
                {trustLevel() === 'high' ? 'Authentic'
                  : trustLevel() === 'medium' ? 'Review'
                  : trustLevel() === 'inconclusive' ? 'Insufficient signal'
                  : 'Suspicious'}
              </span>
            {/if}
          </div>

          <p class="text-sm text-flint-dark dark:text-flint-light mb-3">
            {fileName}{#if imageDimensions()} · {imageDimensions()}{/if}
          </p>

          {#if insufficientSignal()}
            <div role="status" aria-live="polite" class="mb-3 px-3 py-2 rounded-lg border border-flint/30 bg-flint/5 text-xs text-flint-dark dark:text-flint-light leading-relaxed">
              <strong class="text-text-light dark:text-quartz">Insufficient signal.</strong>
              Only {detectorsRun()} of the expected automatic detectors ran on this file.
              The numeric score above is not a meaningful authenticity verdict — too few
              forensic signals contributed to make any judgement. Common causes:
              the Analysis Engine was unreachable during the verify run, the sidecar
              terminated mid-pipeline, or this file's format gated most detectors off.
              Re-run with a stable Analysis Engine before treating the result as authoritative.
            </div>
          {:else if rawTrustLevel === 'high' && !hasPositiveAuthenticitySignal()}
            <div role="status" aria-live="polite" class="mb-3 px-3 py-2 rounded-lg border border-amber/30 bg-amber/5 text-xs text-amber-dark dark:text-amber-light leading-relaxed">
              <strong>No positive authenticity signal.</strong>
              The numeric score is high, but no positive provenance evidence supports an
              "Authentic" claim — no recognised camera MakerNote, no valid Content Credentials
              without AI declaration. Absence of negative findings is not the same as
              evidence of authenticity, particularly on re-encoded or format-converted files
              where JPEG-specific forensics cannot run. Verdict capped at Moderate / Review.
            </div>
          {/if}

          <!-- Breakdown row -->
          <div class="flex items-center gap-4 flex-wrap border-t border-border-light dark:border-border-dark/60 pt-3" role="list" aria-label="Verification summary">
            <div role="listitem" class="flex flex-col gap-0.5">
              <span class="text-[10px] {insufficientSignal() ? 'text-cinnabar-dark dark:text-cinnabar-light font-semibold' : 'text-flint-dark dark:text-flint-light'} uppercase tracking-wider">
                Detectors run{insufficientSignal() ? ' — partial' : ''}
              </span>
              <span class="text-sm {insufficientSignal() ? 'text-cinnabar-dark dark:text-cinnabar-light font-bold' : 'text-obsidian dark:text-quartz font-medium'}">
                {detectorsRun()} / {detectorsAvailable()}
              </span>
            </div>
            <div role="separator" aria-hidden="true" class="w-px h-6 bg-border-light dark:bg-border-dark"></div>
            <div role="listitem" class="flex flex-col gap-0.5">
              <span class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Findings</span>
              <span class="text-sm font-medium {totalFindings > 0 ? 'text-amber-dark dark:text-amber-light' : 'text-malachite-dark dark:text-malachite-light'}">
                {totalFindings === 0 ? 'None' : totalFindings === 1 ? '1 concern' : `${totalFindings} concerns`}
              </span>
            </div>
            {#if cameraLabel()}
              <div role="separator" aria-hidden="true" class="w-px h-6 bg-border-light dark:bg-border-dark"></div>
              <div role="listitem" class="flex flex-col gap-0.5">
                <span class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Camera</span>
                <span class="text-sm text-obsidian dark:text-quartz font-medium">{cameraLabel()}</span>
              </div>
            {/if}
            <div role="separator" aria-hidden="true" class="w-px h-6 bg-border-light dark:bg-border-dark"></div>
            <div role="listitem" class="flex flex-col gap-0.5">
              <span class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Content Credentials</span>
              <!--
                Status labels per C2PA UX Rec v1.4 Table 4:
                  invalid -> "Content Credential unavailable or invalid" (verbatim).
                The header strip summary compresses the label visually
                to "Invalid or unavailable" for layout reasons, but the
                full verbatim Table 4 string is exposed to screen readers
                via aria-label AND is also rendered as a direct disclosure
                in the L1 card at :2239.  Valid/Not attached are
                spec-silent — plain consumer copy used.

                "Valid at signing" is the Jura Trace extension for the
                Pixel-Camera-style case where the leaf signing cert has
                since expired but a trusted timestamp + valid claim
                signature confirm signing-time validity.  Flagged in the
                C2PA conformance submission letter as a deliberate
                nuance preserved from Table 4 Valid, not a new state.
              -->
              <span
                class="text-sm font-medium {result.c2paValid === true ? 'text-malachite-dark dark:text-malachite-light' : result.c2paValid === false ? 'text-cinnabar-dark dark:text-cinnabar-light' : 'text-flint-dark dark:text-flint-light'}"
                title={result.c2paValid === false
                  ? C2PA_STATUS_INVALID
                  : isValidAtSigning() ? C2PA_MSG_VALID_AT_SIGNING_DETAIL : undefined}
                aria-label={result.c2paValid === false
                  ? C2PA_STATUS_INVALID
                  : isValidAtSigning() ? `${C2PA_STATUS_VALID_AT_SIGNING} — ${C2PA_MSG_VALID_AT_SIGNING_DETAIL}` : undefined}
              >
                {result.c2paValid === true
                  ? (isValidAtSigning() ? C2PA_STATUS_VALID_AT_SIGNING : 'Valid')
                  : result.c2paValid === false ? C2PA_STATUS_INVALID_SHORT : 'Not attached'}
              </span>
            </div>
          </div>
        </div>

      </div>
    </section>

    <!-- ── 2. Signal Map Strip ────────────────────────────────────── -->
    <section aria-label="Signal overview — all detectors at a glance" class="mb-5">
      <div class="bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl px-5 py-4">
        <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-widest mb-3">Signal Map — click any detector to view detail</p>

        <div class="flex items-start gap-0" role="group" aria-label="Detector signals grouped by category">

          <!-- Provenance group -->
          <div class="flex-1 pr-4">
            <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider font-semibold mb-2" id="sm-prov">Provenance</p>
            <div class="flex flex-wrap gap-x-4 gap-y-2" role="list" aria-labelledby="sm-prov">
              {#each signalDots().provenance as dot}
                <button
                  class="flex items-center gap-1.5 rounded px-1 py-0.5 min-h-[28px] transition-colors hover:bg-white/5
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
                  role="listitem"
                  aria-label="{dot.label}: {dot.ariaDetail} — click to jump to provenance section"
                  onclick={() => jumpToCard('provenance')}
                >
                  <span
                    class="w-2.5 h-2.5 rounded-full flex-shrink-0 {dotBgClass(dot.state)}
                           {dotPulse(dot.state) ? 'motion-safe:animate-pulse' : ''}"
                    aria-hidden="true"
                  ></span>
                  <span class="text-xs text-flint-dark dark:text-flint-light {dot.state === 'suspicious' ? 'text-amber-light font-medium' : ''}">{dot.label}</span>
                </button>
              {/each}
            </div>
          </div>

          <div class="w-px bg-border-light dark:bg-border-dark self-stretch mx-1" role="separator" aria-hidden="true"></div>

          <!-- Integrity group -->
          <div class="flex-[2] px-4">
            <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider font-semibold mb-2" id="sm-int">Integrity</p>
            <div class="flex flex-wrap gap-x-4 gap-y-2" role="list" aria-labelledby="sm-int">
              {#each signalDots().integrity as dot}
                <button
                  class="flex items-center gap-1.5 rounded px-1 py-0.5 min-h-[28px] transition-colors hover:bg-white/5
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
                  role="listitem"
                  aria-label="{dot.label}: {dot.ariaDetail} — click to jump to integrity section"
                  onclick={() => jumpToCard('integrity')}
                >
                  <span
                    class="w-2.5 h-2.5 rounded-full flex-shrink-0 {dotBgClass(dot.state)}
                           {dotPulse(dot.state) ? 'motion-safe:animate-pulse' : ''}"
                    aria-hidden="true"
                  ></span>
                  <span class="text-xs text-flint-dark dark:text-flint-light {dot.state === 'suspicious' ? 'text-amber-light font-medium' : ''}">{dot.label}</span>
                </button>
              {/each}
            </div>
          </div>

          <div class="w-px bg-border-light dark:bg-border-dark self-stretch mx-1" role="separator" aria-hidden="true"></div>

          <!-- AI Detection group -->
          <div class="flex-1 pl-4">
            <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider font-semibold mb-2" id="sm-ai">AI Detection</p>
            <div class="flex flex-wrap gap-x-4 gap-y-2" role="list" aria-labelledby="sm-ai">
              {#each signalDots().ai as dot}
                <button
                  class="flex items-center gap-1.5 rounded px-1 py-0.5 min-h-[28px] transition-colors hover:bg-white/5
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
                  role="listitem"
                  aria-label="{dot.label}: {dot.ariaDetail} — click to jump to AI detection section"
                  onclick={() => jumpToCard('ai')}
                >
                  <span
                    class="w-2.5 h-2.5 rounded-full flex-shrink-0 {dotBgClass(dot.state)}
                           {dotPulse(dot.state) ? 'motion-safe:animate-pulse' : ''}"
                    aria-hidden="true"
                  ></span>
                  <span class="text-xs {dot.state === 'suppressed' ? 'text-flint-dark dark:text-flint-light line-through' : 'text-flint-dark dark:text-flint-light'} {dot.state === 'suspicious' ? 'text-amber-light font-medium' : ''}">{dot.label}</span>
                </button>
              {/each}
            </div>
          </div>

        </div>
      </div>
    </section>

    <!-- ── 3. Four Forensic Question Cards ────────────────────────── -->
    <section aria-label="Forensic analysis by question" class="space-y-2 mb-5">
      <div class="flex items-baseline gap-2 mb-3">
        <h2 class="font-serif text-lg text-obsidian dark:text-quartz">All Checks</h2>
        <span class="text-xs text-flint-dark dark:text-flint-light">Click a question to expand the full analysis</span>
      </div>

      <!-- Card 1: Does the provenance hold? -->
      <div
        id="card-provenance"
        class="bg-white dark:bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'provenance' ? 'border-lapis/30' : 'border-border-light dark:border-border-dark'}
               focus-within:border-lapis/25"
      >
        <button
          class="w-full flex items-center gap-3 px-5 py-4 text-left min-h-[56px] hover:bg-white/[0.02] transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light rounded-xl"
          aria-expanded={openCard === 'provenance'}
          aria-controls="card-provenance-body"
          onclick={() => openCard = openCard === 'provenance' ? null : 'provenance'}
        >
          <svg
            class="w-4 h-4 text-flint-dark dark:text-flint-light flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'provenance' ? 'rotate-90' : ''}"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true"
          ><path d="M9 5l7 7-7 7"/></svg>

          <span class="flex-1 font-serif text-base text-obsidian dark:text-quartz">Does the provenance hold?</span>

          <span class="text-xs text-flint-dark dark:text-flint-light mr-2">
            {provenanceFindings === 0 ? '2 of 2 passed' : `${2 - provenanceFindings} of 2 passed`}
          </span>

          <span class="text-xs font-semibold {cardPassClass(provenancePass())}">
            {provenancePass() === null ? '—' : provenancePass() ? 'Pass' : 'Concern'}
          </span>
        </button>

        {#if openCard === 'provenance'}
          <div id="card-provenance-body" class="border-t border-border-light dark:border-border-dark/60">
            <ul class="divide-y divide-border-light/70 dark:divide-border-dark/40" aria-label="Provenance checks">

              <!-- Filename Analysis -->
              {#if result.filenameAnalysis}
                {@const fa = result.filenameAnalysis}
                <li class="px-5 py-4">
                  <div class="flex items-start gap-3">
                    <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-lapis dark:text-lapis-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/>
                    </svg>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-1 flex-wrap">
                        <span class="text-sm font-medium text-obsidian dark:text-quartz">Filename Analysis</span>
                        <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium border
                          {fa.pattern === 'camera' ? 'bg-malachite/15 text-malachite-dark dark:text-malachite-light border-malachite/30'
                           : fa.pattern === 'ai_generated' ? 'bg-amber/15 text-amber-dark dark:text-amber-light border-amber/30'
                           : fa.pattern === 'screenshot' ? 'bg-lapis/15 text-lapis dark:text-lapis-light border-lapis/30'
                           : 'bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light border-border-light dark:border-border-dark'}">
                          {fa.pattern.replace('_', ' ')}
                        </span>
                        <span class="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light tabular-nums">
                          {Math.round(fa.confidence * 100)}% confidence
                        </span>
                      </div>
                      <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">{fa.summary}</p>
                      {#if fa.matchedPattern}
                        <p class="text-[11px] mt-1 font-mono text-flint-dark dark:text-flint-light break-all">{fa.matchedPattern}</p>
                      {/if}
                    </div>
                  </div>
                </li>
              {/if}

              <!-- EXIF -->
              <li class="px-5 py-4 {(result.exifAnalysis?.findings ?? []).some((f: AnomalyFinding) => f.severity === 'high' || f.severity === 'critical') ? 'bg-amber/[0.04]' : ''}">
                <div class="flex items-start gap-3">
                  <svg class="w-4 h-4 mt-0.5 flex-shrink-0 {forensicScoreClass(1 - (result.exifAnalysis?.trustScore ?? 1))}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    {#if (result.exifAnalysis?.findings ?? []).some((f: AnomalyFinding) => f.severity === 'high' || f.severity === 'critical')}
                      <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
                    {:else}
                      <path d="M22 11.08V12a10 10 0 11-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/>
                    {/if}
                  </svg>
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-1">
                      <span class="text-sm font-medium text-obsidian dark:text-quartz">EXIF Metadata Analysis</span>
                      {#if result.exifAnalysis}
                        <span class="text-xs px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light">
                          {result.exifAnalysis.fieldsPopulated} / {result.exifAnalysis.fieldsTotal} fields
                        </span>
                      {/if}
                    </div>
                    {#if result.exifAnalysis}
                      <!-- MakerNote camera authenticity badge -->
                      {#if (result.exifAnalysis as any).cameraAuthenticityBonus != null && (result.exifAnalysis as any).cameraAuthenticityBonus > 0.5}
                        <div
                          class="mb-2 flex items-start gap-2 rounded px-2.5 py-2 bg-malachite/10 border border-malachite/30"
                          role="note"
                          aria-label="Camera MakerNote authenticity signal"
                        >
                          <span class="flex-shrink-0 text-[10px] font-semibold px-1.5 py-0.5 rounded-full bg-malachite/15 text-malachite-dark dark:text-malachite-light border border-malachite/30">
                            Authentic
                          </span>
                          <div class="flex-1 min-w-0">
                            <p class="text-xs font-medium text-malachite-dark dark:text-malachite-light leading-snug">
                              Camera MakerNote signature verified
                            </p>
                            <p class="text-[11px] text-flint-dark dark:text-flint-light leading-relaxed mt-0.5">
                              Vendor-proprietary MakerNote blob detected — AI generators virtually never synthesise these.
                              Confidence: <span class="tabular-nums font-medium">{Math.round((result.exifAnalysis as any).cameraAuthenticityBonus * 100)}%</span>.
                            </p>
                          </div>
                        </div>
                      {/if}

                      {#if result.exifAnalysis.findings.length === 0}
                        <p class="text-xs text-malachite-dark dark:text-malachite-light">No anomalies detected</p>
                      {:else}
                        <ul class="space-y-1.5 mt-1">
                          {#each highestSeverityFindings(result.exifAnalysis.findings).slice(0, 5) as finding}
                            <li class="text-xs leading-relaxed">
                              <span class="font-medium {finding.severity === 'critical' || finding.severity === 'high' ? 'text-amber-dark dark:text-amber-light' : finding.severity === 'medium' ? 'text-amber-dark/70 dark:text-amber-light/70' : 'text-flint-dark dark:text-flint-light'}">
                                [{finding.severity.toUpperCase()}]
                              </span>
                              <span class="text-obsidian dark:text-quartz ml-1">{finding.title}</span>
                              <span class="text-flint-dark dark:text-flint-light ml-1">— {finding.description}</span>
                            </li>
                          {/each}
                          {#if result.exifAnalysis.findings.length > 5}
                            <li>
                              <details class="group">
                                <summary class="list-none text-xs text-lapis dark:text-lapis-light cursor-pointer hover:text-obsidian dark:hover:text-quartz flex items-center gap-1 min-h-[24px]
                                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded">
                                  <svg class="w-3 h-3 motion-safe:group-open:rotate-90 transition-transform duration-150" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 5l7 7-7 7"/></svg>
                                  Show {result.exifAnalysis.findings.length - 5} more finding{result.exifAnalysis.findings.length - 5 === 1 ? '' : 's'}
                                </summary>
                                <ul class="space-y-1.5 mt-1.5">
                                  {#each highestSeverityFindings(result.exifAnalysis.findings).slice(5) as finding}
                                    <li class="text-xs leading-relaxed">
                                      <span class="font-medium {finding.severity === 'critical' || finding.severity === 'high' ? 'text-amber-dark dark:text-amber-light' : finding.severity === 'medium' ? 'text-amber-dark/70 dark:text-amber-light/70' : 'text-flint-dark dark:text-flint-light'}">
                                        [{finding.severity.toUpperCase()}]
                                      </span>
                                      <span class="text-obsidian dark:text-quartz ml-1">{finding.title}</span>
                                      <span class="text-flint-dark dark:text-flint-light ml-1">— {finding.description}</span>
                                    </li>
                                  {/each}
                                </ul>
                              </details>
                            </li>
                          {/if}
                        </ul>
                      {/if}
                    {:else}
                      <p class="text-xs text-flint-dark dark:text-flint-light">EXIF data not available for this file type.</p>
                    {/if}

                    <!-- Expandable raw EXIF metadata fields -->
                    {#if result.imageMetadata}
                      {@const meta = result.imageMetadata}
                      <details class="mt-2">
                        <summary class="text-[11px] text-lapis cursor-pointer hover:text-lapis dark:text-lapis-light">
                          View EXIF metadata fields
                        </summary>
                        <div class="mt-2 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs">
                          {#if meta.cameraMake || meta.cameraModel}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Camera</p>
                              <p class="text-obsidian dark:text-quartz">{[meta.cameraMake, meta.cameraModel].filter(Boolean).join(' ')}</p>
                            </div>
                          {/if}
                          {#if meta.software}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Software</p>
                              <p class="text-obsidian dark:text-quartz">{meta.software}</p>
                            </div>
                          {/if}
                          {#if meta.datetimeOriginal}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Date taken</p>
                              <p class="text-obsidian dark:text-quartz">{meta.datetimeOriginal}</p>
                            </div>
                          {/if}
                          {#if meta.datetimeModified}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Date modified</p>
                              <p class="text-obsidian dark:text-quartz">{meta.datetimeModified}</p>
                            </div>
                          {/if}
                          {#if meta.iso}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">ISO</p>
                              <p class="text-obsidian dark:text-quartz">{meta.iso}</p>
                            </div>
                          {/if}
                          {#if meta.focalLength}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Focal length</p>
                              <p class="text-obsidian dark:text-quartz">{meta.focalLength}</p>
                            </div>
                          {/if}
                          {#if meta.exposureTime}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Exposure</p>
                              <p class="text-obsidian dark:text-quartz">{meta.exposureTime}</p>
                            </div>
                          {/if}
                          {#if meta.fNumber}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Aperture</p>
                              <p class="text-obsidian dark:text-quartz">{meta.fNumber}</p>
                            </div>
                          {/if}
                          {#if meta.colorSpace}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Colour space</p>
                              <p class="text-obsidian dark:text-quartz">{meta.colorSpace}</p>
                            </div>
                          {/if}
                          {#if meta.exifWidth && meta.exifHeight}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">EXIF dimensions</p>
                              <p class="text-obsidian dark:text-quartz">{meta.exifWidth} x {meta.exifHeight}</p>
                            </div>
                          {/if}
                          {#if meta.gpsLatitude != null && meta.gpsLongitude != null}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">GPS</p>
                              <p class="text-obsidian dark:text-quartz">
                                {meta.gpsLatitude.toFixed(6)}, {meta.gpsLongitude.toFixed(6)}
                                <a
                                  href="https://www.openstreetmap.org/?mlat={meta.gpsLatitude}&mlon={meta.gpsLongitude}#map=16/{meta.gpsLatitude}/{meta.gpsLongitude}"
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  class="ml-1.5 text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline text-[11px]"
                                >View on map</a>
                              </p>
                            </div>
                          {/if}
                          {#if meta.iccProfileDescription}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">ICC Profile</p>
                              <p class="text-obsidian dark:text-quartz">{meta.iccProfileDescription}</p>
                            </div>
                          {/if}
                          {#if meta.jpegQuantTables?.estimatedQuality != null}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">JPEG Quality</p>
                              <p class="text-obsidian dark:text-quartz tabular-nums">{meta.jpegQuantTables.estimatedQuality} / 100</p>
                            </div>
                          {/if}
                          {#if meta.jpegQuantTables?.knownSource}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Q-table encoder</p>
                              <p class="text-obsidian dark:text-quartz">{meta.jpegQuantTables.knownSource}</p>
                            </div>
                          {/if}
                          {#if meta.artist}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Artist</p>
                              <p class="text-obsidian dark:text-quartz">{meta.artist}</p>
                            </div>
                          {/if}
                          {#if meta.copyright}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Copyright</p>
                              <p class="text-obsidian dark:text-quartz">{meta.copyright}</p>
                            </div>
                          {/if}
                          {#if meta.description}
                            <div class="col-span-2">
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Description</p>
                              <p class="text-obsidian dark:text-quartz">{meta.description}</p>
                            </div>
                          {/if}
                        </div>
                      </details>
                    {/if}
                  </div>
                  {#if result.exifAnalysis}
                    <div class="flex flex-col items-end gap-0.5 flex-shrink-0">
                      <span class="text-sm font-medium tabular-nums {forensicScoreClass(1 - result.exifAnalysis.trustScore)}">
                        {Math.round(result.exifAnalysis.trustScore * 100)}%
                      </span>
                      {#if showRawScores}
                        <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">trust score</span>
                      {/if}
                    </div>
                  {/if}
                </div>
              </li>

              <!-- Historical Weather Context — Enhanced mode, GPS + date required.
                   Hidden in Standard mode entirely (no broken button surface).
                   The EnhancedModeBanner higher up the page offers the upgrade. -->
              {#if gpsCoords && exifDate() && networkMode === 'enhanced'}
                <li class="px-5 py-4">
                  <div class="flex items-start gap-3">
                    <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-lapis dark:text-lapis-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M3 15h4l3-6 4 12 3-6h4"/>
                    </svg>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-2">
                        <span class="text-sm font-medium text-obsidian dark:text-quartz">Weather Context</span>
                        <span class="text-[10px] px-1.5 py-px rounded-full bg-amber/15 text-amber-dark dark:text-amber-light border border-amber/30">Enhanced</span>
                      </div>
                      <p class="text-xs text-flint-dark dark:text-flint-light mb-3">
                        Historical weather at {gpsCoords.lat.toFixed(4)}, {gpsCoords.lon.toFixed(4)} on {exifDate()} ~{exifHour()}:00 UTC.
                        Useful for verifying visible conditions match the claimed time and location.
                      </p>

                      {#if weatherData}
                        <dl class="grid grid-cols-2 sm:grid-cols-3 gap-x-6 gap-y-2 text-xs" aria-label="Historical weather conditions">
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Temperature</dt>
                            <dd class="text-obsidian dark:text-quartz font-medium">{weatherData.temperature.toFixed(1)} °C</dd>
                          </div>
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Cloud cover</dt>
                            <dd class="text-obsidian dark:text-quartz font-medium">{weatherData.cloudCover.toFixed(0)}%
                              {#if weatherData.cloudCover > 80}
                                <span class="text-flint-dark dark:text-flint-light ml-1">(overcast — no sharp shadows expected)</span>
                              {:else if weatherData.cloudCover > 50}
                                <span class="text-flint-dark dark:text-flint-light ml-1">(partly cloudy)</span>
                              {:else}
                                <span class="text-flint-dark dark:text-flint-light ml-1">(clear)</span>
                              {/if}
                            </dd>
                          </div>
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Precipitation</dt>
                            <dd class="text-obsidian dark:text-quartz font-medium">{weatherData.precipitation.toFixed(1)} mm</dd>
                          </div>
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Visibility</dt>
                            <dd class="text-obsidian dark:text-quartz font-medium">{(weatherData.visibility / 1000).toFixed(1)} km
                              {#if weatherData.visibility < 1000}
                                <span class="text-amber-dark dark:text-amber-light ml-1">(fog/mist)</span>
                              {/if}
                            </dd>
                          </div>
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Wind speed</dt>
                            <dd class="text-obsidian dark:text-quartz font-medium">{weatherData.windSpeed.toFixed(1)} km/h</dd>
                          </div>
                        </dl>
                        {#if result.shadowConsistencyResult && weatherData.cloudCover > 80}
                          <p class="mt-2 text-xs text-amber-dark dark:text-amber-light">
                            Shadow consistency analysis may be unreliable — cloud cover was {weatherData.cloudCover.toFixed(0)}% (overcast), producing diffuse lighting without distinct shadows.
                          </p>
                        {/if}
                      {:else if weatherError}
                        <p class="text-xs text-cinnabar-dark dark:text-cinnabar-light" role="alert">{weatherError}</p>
                      {:else}
                        <button
                          type="button"
                          onclick={handleFetchWeather}
                          disabled={weatherLoading}
                          class="px-3 py-1.5 min-h-[32px] text-xs rounded bg-lapis text-white hover:bg-lapis-dark
                                 transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                                 dark:focus-visible:ring-offset-obsidian"
                        >
                          {#if weatherLoading}
                            <span class="flex items-center gap-1.5">
                              <span class="w-3 h-3 border-2 border-white border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Loading"></span>
                              Fetching weather...
                            </span>
                          {:else}
                            Fetch historical weather
                          {/if}
                        </button>
                        <p class="mt-1.5 text-[10px] text-flint-dark dark:text-flint-light">Requires internet connection. Data from Open-Meteo (CC-BY).</p>
                      {/if}
                    </div>
                  </div>
                </li>
              {/if}

              <!-- Platform Fingerprint — informational-only provenance disclosure.
                   Renders in neutral flint regardless of detection state to avoid
                   amber-as-caution misread (persona-testing 30 April 2026: 6/10
                   B2B personas read amber as a tampering flag).  Amber is reserved
                   in this product for elevated concern; platform identification is
                   provenance context, not a tampering signal. -->
              {#if result.platformFingerprintResult}
                {@const pf = result.platformFingerprintResult}
                <li class="px-5 py-4">
                  <div class="flex items-start gap-3">
                    <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-flint-dark dark:text-flint-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <rect x="5" y="2" width="14" height="20" rx="2" ry="2"/><line x1="12" y1="18" x2="12.01" y2="18"/>
                    </svg>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-1 flex-wrap">
                        <span class="text-sm font-medium text-obsidian dark:text-quartz">Platform Fingerprint</span>
                        {#if pf.detected && pf.platform}
                          <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-gray-100 dark:bg-graphite-light text-obsidian dark:text-quartz border border-border-light dark:border-border-dark">
                            {pf.platform}
                          </span>
                          {#if pf.confidence != null}
                            <span class="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light tabular-nums">
                              {Math.round(pf.confidence * 100)}% confidence
                            </span>
                          {/if}
                          <span class="text-[10px] px-1.5 py-0.5 rounded font-medium bg-lapis/10 text-lapis dark:text-lapis-light border border-lapis/20">
                            Informational
                          </span>
                        {/if}
                      </div>
                      {#if pf.detected}
                        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">{pf.summary}</p>
                      {:else}
                        <p class="text-xs text-flint-dark dark:text-flint-light">No social media processing detected.</p>
                        {#if pf.summary && pf.summary !== 'No social media processing detected.'}
                          <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5">{pf.summary}</p>
                        {/if}
                      {/if}
                      {#if pf.detected && pf.allCandidates && pf.allCandidates.length > 1}
                        <details class="mt-2 group">
                          <summary class="list-none text-[11px] text-lapis dark:text-lapis-light cursor-pointer hover:text-obsidian dark:hover:text-quartz flex items-center gap-1 min-h-[24px]
                                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded">
                            <svg class="w-3 h-3 motion-safe:group-open:rotate-90 transition-transform duration-150" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 5l7 7-7 7"/></svg>
                            Other candidates
                          </summary>
                          <ul class="mt-1.5 space-y-1" aria-label="Platform fingerprint candidates">
                            {#each pf.allCandidates as candidate}
                              <li class="flex items-center justify-between gap-3 text-[11px]">
                                <span class="text-flint-dark dark:text-flint-light">{candidate.platform}</span>
                                <span class="tabular-nums text-obsidian dark:text-quartz">{Math.round(candidate.confidence * 100)}%</span>
                              </li>
                            {/each}
                          </ul>
                        </details>
                      {/if}
                    </div>
                  </div>
                </li>
              {/if}

              <!-- Content Credentials (C2PA) — 3-tier progressive disclosure
                   per C2PA UX Recommendations v1.4. Internal identifiers
                   (c2paManifest, c2paValid, card-provenance) are unchanged.
              -->
              <li
                id="section-c2pa"
                class="px-5 py-4 {result.c2paValid === false ? 'bg-cinnabar/[0.03]' : ''}"
                aria-label="Content Credentials"
              >

                <!-- ── L1: Icon + one-line summary (always visible) ──
                     The seal is the official C2PA "cr" information pin per
                     UX Recommendations v1.4 §4.1 — circle with squared
                     lower-right corner, "cr" lettermark inside.  Invalid
                     state adds a secondary warning marker that leaves the
                     "cr" fully legible.  Colour and tooltip are driven by
                     the component itself from `c2paSealState`. -->
                <div class="flex items-start gap-3">
                  <span class="mt-0.5 flex-shrink-0">
                    <ContentCredentialsSeal state={c2paSealState()} size="sm" />
                  </span>

                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-1 flex-wrap">
                      <span class="text-sm font-medium text-obsidian dark:text-quartz">Content Credentials</span>
                    </div>

                    <!-- L1 one-line summary -->
                    {#if result.c2paValid === true && result.c2paManifest}
                      {@const manifest = result.c2paManifest}
                      <p class="text-xs text-malachite-dark dark:text-malachite-light leading-relaxed">
                        {#if c2paSignerName && c2paSignedDate()}
                          Issued by {c2paSignerName} on {c2paSignedDate()}
                        {:else if c2paSignerName}
                          Issued by {c2paSignerName}
                        {:else if c2paSignedDate()}
                          Signed on {c2paSignedDate()}
                        {:else}
                          Provenance record attached and verified.
                        {/if}
                      </p>

                      <!-- Content summary — plain-language description like Adobe's tool -->
                      {#if manifest.contentSummary}
                        <p class="text-xs text-obsidian/80 dark:text-quartz/80 mt-1 leading-relaxed">
                          {manifest.contentSummary}
                        </p>
                      {/if}

                      <!-- Certificate expiry detail deferred to L3 validation summary
                           per C2PA UX Rec v1.4: timestamped credentials are Valid, not a separate state.
                           Adobe's verify.contentauthenticity.org shows no caveat for this case. -->
                    {:else if result.c2paValid === false}
                      <p class="text-xs text-cinnabar-dark dark:text-cinnabar-light leading-relaxed">
                        Content Credential unavailable or invalid — the record may have been altered.
                      </p>
                    {:else}
                      <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
                        No Content Credentials attached.
                      </p>
                    {/if}

                    <!-- L2 / L3 triggers — only shown when manifest is valid -->
                    {#if result.c2paValid === true && result.c2paManifest}
                      <div class="mt-2 flex items-center gap-3 flex-wrap">
                        <button
                          class="text-xs text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                                 dark:focus-visible:ring-offset-obsidian rounded min-h-[32px] px-0"
                          aria-expanded={c2paShowL2}
                          aria-controls="c2pa-l2"
                          onclick={() => { c2paShowL2 = !c2paShowL2; if (!c2paShowL2) c2paShowL3 = false; }}
                        >
                          {c2paShowL2 ? 'Hide details' : 'View more'}
                        </button>
                      </div>

                      <!-- ── L2: App/device, Issued by, Edits, Digital source type ── -->
                      {#if c2paShowL2}
                        <div id="c2pa-l2" class="mt-3 space-y-3 border-t border-border-light dark:border-border-dark/40 pt-3">

                          <!-- Manifest thumbnail (when embedded in assertion).
                               Suppressed on update manifests per C2PA UX Rec
                               v1.4 §6 — an update manifest describes a metadata-
                               only change and MUST NOT display a thumbnail that
                               could be mistaken for the current asset state. -->
                          {#if result.c2paManifest.thumbnailBase64 && !result.c2paManifest.isUpdateManifest}
                            <div class="mb-1">
                              <ImageZoom
                                src="data:{result.c2paManifest.thumbnailMime ?? 'image/jpeg'};base64,{result.c2paManifest.thumbnailBase64}"
                                alt="Content Credentials thumbnail"
                                thumbClass="w-24 h-auto rounded border border-border-light dark:border-border-dark"
                              />
                            </div>
                          {/if}

                          <!-- App or device used — prominent standalone field like Adobe -->
                          {#if result.c2paManifest.appOrDevice}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-0.5">App or device used</p>
                              <p class="text-xs text-obsidian dark:text-quartz font-medium">{result.c2paManifest.appOrDevice}</p>
                            </div>
                          {/if}

                          <!-- Issued by — mandatory per C2PA UX Rec v1.4 §4.2 -->
                          <div>
                            <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-0.5">Issued by</p>
                            <p class="text-xs text-obsidian dark:text-quartz">
                              {c2paSignerName ?? 'Unknown'}
                            </p>
                            {#if result.c2paManifest.signedByIssuer}
                              <p class="text-[11px] text-flint-dark dark:text-flint-light mt-0.5">
                                via {result.c2paManifest.signedByIssuer}
                              </p>
                            {/if}
                          </div>

                          <!-- Signed date -->
                          {#if c2paSignedDate()}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-0.5">Date</p>
                              <p class="text-xs text-obsidian dark:text-quartz">{c2paSignedDate()}</p>
                            </div>
                          {/if}

                          <!-- Edits and activity — per C2PA UX Rec v1.4 §4.3 -->
                          {#if c2paActions().length > 0}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-1.5">Edits and activity</p>
                              <ul class="space-y-2" aria-label="Recorded edits and activity">
                                {#each c2paActions() as action}
                                  <li class="flex items-start gap-2">
                                    <span class="inline-flex items-center px-2 py-0.5 rounded-full text-[11px] shrink-0
                                                 bg-gray-100 dark:bg-graphite-light border border-border-light dark:border-border-dark text-obsidian dark:text-quartz">
                                      {action.label}
                                    </span>
                                    <div class="text-[11px] leading-snug">
                                      {#if action.description}
                                        <p class="text-obsidian dark:text-quartz">{action.description}</p>
                                      {/if}
                                      {#if action.sourceType}
                                        <p class="text-flint-dark dark:text-flint-light">
                                          Source: {action.sourceType}
                                        </p>
                                      {/if}
                                      {#if action.softwareAgent}
                                        <p class="text-flint-dark dark:text-flint-light">
                                          Tool: {action.softwareAgent}
                                        </p>
                                      {/if}
                                      {#if !action.description && !action.sourceType && !action.softwareAgent}
                                        <p class="text-flint-dark dark:text-flint-light italic">No additional detail recorded</p>
                                      {/if}
                                    </div>
                                  </li>
                                {/each}
                              </ul>
                            </div>
                          {/if}

                          <!-- Digital source type — per C2PA UX Rec v1.4 §4.4 -->
                          {#if c2paDigitalSourceType()}
                            <div>
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-0.5">Digital source type</p>
                              <p class="text-xs text-obsidian dark:text-quartz">{c2paDigitalSourceType()}</p>
                            </div>
                          {/if}

                          <!-- ── Provenance chain timeline (C2PA UX Rec v1.4 §5.4) ── -->
                          {#if result.c2paChain && result.c2paChain.ingredients.length > 0}
                            {@const chain = result.c2paChain}
                            {@const originManifest = chain.ingredients[chain.ingredients.length - 1]}
                            {@const middleIngredients = chain.ingredients.slice(0, -1)}
                            {@const shouldCollapse = chain.manifestCount >= 4}
                            <div class="mt-1 pt-3 border-t border-border-light dark:border-border-dark/40">
                              <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-2">Provenance chain</p>
                              <ol
                                class="relative ml-1.5"
                                aria-label="Content Credentials provenance chain"
                              >

                                <!-- Active manifest node -->
                                <li class="relative pl-5 pb-3">
                                  <span
                                    class="absolute left-0 top-1.5 w-2.5 h-2.5 rounded-full bg-malachite border-2 border-malachite/30"
                                    aria-hidden="true"
                                  ></span>
                                  <!-- Connector line down to next node -->
                                  <span
                                    class="absolute left-[4.5px] top-4 bottom-0 w-px bg-malachite/40"
                                    aria-hidden="true"
                                  ></span>
                                  <div class="flex items-start gap-2">
                                    <!-- Thumbnail suppressed on update manifests per §6. -->
                                    {#if chain.active.thumbnailBase64 && !chain.active.isUpdateManifest}
                                      <ImageZoom
                                        src="data:{chain.active.thumbnailMime ?? 'image/jpeg'};base64,{chain.active.thumbnailBase64}"
                                        alt="Active manifest thumbnail"
                                        thumbClass="w-10 h-10 rounded border border-border-light dark:border-border-dark object-cover flex-shrink-0"
                                      />
                                    {/if}
                                    <div>
                                      <!-- "Active" label per C2PA UX Rec v1.4 §5.3 and §5.4. -->
                                      <p class="text-[10px] font-semibold text-flint-dark dark:text-flint-light uppercase tracking-wider leading-none mb-0.5">Active</p>
                                      <p class="text-xs text-obsidian dark:text-quartz leading-snug">{chain.active.appOrDevice ?? chainSignerName(chain.active)}</p>
                                      {#if chainSignedDate(chain.active)}
                                        <p class="text-[11px] text-flint-dark dark:text-flint-light">{chainSignedDate(chain.active)}</p>
                                      {/if}
                                      {#if chainActionSummary(chain.active)}
                                        <p class="text-[11px] text-flint-dark dark:text-flint-light italic">{chainActionSummary(chain.active)}</p>
                                      {/if}
                                    </div>
                                  </div>
                                </li>

                                <!-- Middle ingredients (collapsed when >= 4 manifests) -->
                                {#if shouldCollapse && !c2paChainExpanded && middleIngredients.length > 0}
                                  <!-- Collapsed pill -->
                                  <li class="relative pl-5 pb-3">
                                    <span
                                      class="absolute left-[4.5px] top-0 bottom-0 w-px bg-malachite/40"
                                      aria-hidden="true"
                                    ></span>
                                    <button
                                      type="button"
                                      onclick={() => { c2paChainExpanded = true; }}
                                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[11px] font-medium
                                             bg-gray-100 dark:bg-obsidian border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light
                                             hover:text-obsidian dark:hover:text-quartz hover:border-lapis/50 transition-colors
                                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 dark:focus-visible:ring-offset-obsidian"
                                      aria-label="Show {middleIngredients.length} additional manifest{middleIngredients.length === 1 ? '' : 's'} in chain"
                                    >
                                      {middleIngredients.length} additional manifest{middleIngredients.length === 1 ? '' : 's'}
                                      <svg class="w-3 h-3" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                                        <path d="M6 3.5L11 8l-5 4.5V3.5z"/>
                                      </svg>
                                    </button>
                                  </li>
                                {:else}
                                  {#each middleIngredients as ingredient, idx}
                                    <li class="relative pl-5 pb-3">
                                      <span
                                        class="absolute left-0 top-1.5 w-2 h-2 rounded-full bg-malachite/50 border border-malachite/50"
                                        aria-hidden="true"
                                      ></span>
                                      <span
                                        class="absolute left-[4.5px] top-3.5 bottom-0 w-px bg-malachite/40"
                                        aria-hidden="true"
                                      ></span>
                                      <p class="text-xs text-obsidian dark:text-quartz leading-snug">{chainSignerName(ingredient)}</p>
                                      {#if chainSignedDate(ingredient)}
                                        <p class="text-[11px] text-flint-dark dark:text-flint-light">{chainSignedDate(ingredient)}</p>
                                      {/if}
                                      {#if chainActionSummary(ingredient)}
                                        <p class="text-[11px] text-flint-dark dark:text-flint-light italic">{chainActionSummary(ingredient)}</p>
                                      {/if}
                                    </li>
                                  {/each}
                                  {#if shouldCollapse && c2paChainExpanded}
                                    <!-- Collapse button after revealed items -->
                                    <li class="relative pl-5 pb-1">
                                      <span
                                        class="absolute left-[4.5px] top-0 bottom-0 w-px bg-malachite/40"
                                        aria-hidden="true"
                                      ></span>
                                      <button
                                        type="button"
                                        onclick={() => { c2paChainExpanded = false; }}
                                        class="text-[11px] text-lapis dark:text-lapis-light hover:no-underline underline underline-offset-2
                                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                                      >
                                        Show fewer
                                      </button>
                                    </li>
                                  {/if}
                                {/if}

                                <!-- Origin node (last ingredient) -->
                                <li class="relative pl-5">
                                  <span
                                    class="absolute left-0 top-1.5 w-2.5 h-2.5 rounded-full border-2 border-malachite bg-white dark:bg-obsidian"
                                    aria-hidden="true"
                                  ></span>
                                  <div class="flex items-start gap-2">
                                    <!-- Thumbnail suppressed on update manifests per §6. -->
                                    {#if originManifest.thumbnailBase64 && !originManifest.isUpdateManifest}
                                      <ImageZoom
                                        src="data:{originManifest.thumbnailMime ?? 'image/jpeg'};base64,{originManifest.thumbnailBase64}"
                                        alt="Origin manifest thumbnail"
                                        thumbClass="w-10 h-10 rounded border border-border-light dark:border-border-dark object-cover flex-shrink-0"
                                      />
                                    {/if}
                                    <div>
                                      <!-- "Origin" label per C2PA UX Rec v1.4 §5.4 and Figure 12. -->
                                      <p class="text-[10px] font-semibold text-flint-dark dark:text-flint-light uppercase tracking-wider leading-none mb-0.5">Origin</p>
                                      <p class="text-xs text-obsidian dark:text-quartz leading-snug">{originManifest.appOrDevice ?? chainSignerName(originManifest)}</p>
                                      {#if chainSignedDate(originManifest)}
                                        <p class="text-[11px] text-flint-dark dark:text-flint-light">{chainSignedDate(originManifest)}</p>
                                      {/if}
                                      {#if chainActionSummary(originManifest)}
                                        <p class="text-[11px] text-flint-dark dark:text-flint-light italic">{chainActionSummary(originManifest)}</p>
                                      {/if}
                                    </div>
                                  </div>
                                </li>
                              </ol>
                            </div>
                          {/if}

                          <!-- L3 trigger -->
                          <button
                            class="text-xs text-lapis dark:text-lapis-light underline underline-offset-2 hover:no-underline
                                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                                   dark:focus-visible:ring-offset-obsidian rounded min-h-[32px] px-0"
                            aria-expanded={c2paShowL3}
                            aria-controls="c2pa-l3"
                            onclick={() => { if (!c2paShowL3) selectedManifestIndex = 0; c2paShowL3 = !c2paShowL3; }}
                          >
                            {c2paShowL3 ? 'Hide full details' : 'View full details'}
                          </button>

                          <!-- ── L3: Validation checks, assertions, format ── -->
                          {#if c2paShowL3}
                            {@const manifests = allManifests()}
                            {@const hasChain = manifests.length > 1}
                            <div id="c2pa-l3" class="space-y-3 border-t border-border-light dark:border-border-dark/40 pt-3">

                              <!-- Manifest selector tabs — only shown when chain has multiple manifests -->
                              {#if hasChain}
                                <div role="tablist" aria-label="Manifest in chain" class="flex flex-wrap gap-1.5">
                                  {#each manifests as manifest, idx}
                                    <!--
                                      Chain-position labels per C2PA UX Rec v1.4:
                                        "Active" — spec-sanctioned (§5.3 and §5.4:
                                          "the active manifest at the top").
                                        "Origin" — spec-sanctioned (§5.4 and Figure
                                          12: "origin ingredients … at the bottom").
                                        Middle positions — spec is silent (it assumes
                                          they are collapsed under "N additional
                                          manifests" when chain count >= 4). We use a
                                          neutral positional label for the uncollapsed
                                          <4 case rather than invent terminology like
                                          "Intermediate".
                                    -->
                                    {@const tabLabel = idx === 0 ? 'Active'
                                      : idx === manifests.length - 1 ? 'Origin'
                                      : `Step ${idx} of ${manifests.length - 1}`}
                                    {@const isSelected = selectedManifestIndex === idx}
                                    {@const isValid = manifest.isValid}
                                    <button
                                      role="tab"
                                      aria-selected={isSelected}
                                      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded text-[11px] font-medium
                                             transition-colors duration-150
                                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                                             focus-visible:ring-offset-1 dark:focus-visible:ring-offset-obsidian
                                             {isSelected
                                               ? 'bg-lapis/20 text-lapis dark:bg-lapis/30 dark:text-lapis-light'
                                               : 'bg-gray-100 dark:bg-graphite-light/40 text-flint-dark dark:text-flint-light hover:bg-gray-200 dark:hover:bg-graphite-light/70 hover:text-obsidian dark:hover:text-quartz'}"
                                      onclick={() => { selectedManifestIndex = idx; }}
                                    >
                                      <span
                                        class="w-1.5 h-1.5 rounded-full flex-shrink-0
                                               {isValid ? 'bg-malachite dark:bg-malachite-light' : 'bg-cinnabar dark:bg-cinnabar-light'}"
                                        aria-hidden="true"
                                      ></span>
                                      {tabLabel}
                                    </button>
                                  {/each}
                                </div>
                              {/if}

                              <!-- Selected manifest detail panel -->
                              {#if selectedManifest() !== null}
                                {@const sm = selectedManifest()}
                                <div role="tabpanel" aria-label="Manifest details" class="space-y-3">

                                  <!-- Format / title -->
                                  {#if sm && (sm.format || sm.title)}
                                    <div class="flex gap-4 flex-wrap">
                                      {#if sm.title}
                                        <div>
                                          <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-0.5">Title</p>
                                          <p class="text-xs text-obsidian dark:text-quartz">{sm.title}</p>
                                        </div>
                                      {/if}
                                      {#if sm.format}
                                        <div>
                                          <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-0.5">Format</p>
                                          <p class="text-xs text-obsidian dark:text-quartz font-mono">{sm.format}</p>
                                        </div>
                                      {/if}
                                    </div>
                                  {/if}

                                  <!-- L3: Validation summary (human-readable) -->
                                  {#if sm && sm.validationChecks && sm.validationChecks.length > 0}
                                    {@const checks = sm.validationChecks}
                                    {@const sigValid = checks.some(c => c.code === 'claimSignature.validated' && c.outcome === 'pass')}
                                    {@const dataValid = checks.some(c => c.code === 'assertion.dataHash.match' && c.outcome === 'pass')}
                                    {@const tsValid = checks.some(c => (c.code === 'timeStamp.validated' || c.code === 'timeStamp.trusted') && c.outcome === 'pass')}
                                    <!-- certExpired / certUntrusted removed from L3 display (always-on noise); raw codes in L4 -->
                                    {@const hashFail = checks.some(c => c.code.includes('dataHash.mismatch') && c.outcome === 'fail')}
                                    {@const passCount = checks.filter(c => c.outcome === 'pass').length}
                                    {@const failCount = checks.filter(c => c.outcome === 'fail').length}
                                    {@const infoCount = checks.filter(c => c.outcome === 'info').length}
                                    <div>
                                      <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-2">Validation summary</p>
                                      <div class="space-y-1.5">
                                        <div class="flex items-center gap-2 text-xs">
                                          <span class="w-4 text-center {sigValid ? 'text-malachite-dark dark:text-malachite-light' : 'text-cinnabar-dark dark:text-cinnabar-light'}">{sigValid ? '\u2713' : '\u2717'}</span>
                                          <!-- Spec copy: pass is "Signature valid" (Table 2 Signer
                                               vocabulary + Table 4 valid/invalid state pair, §5.3).
                                               Fail is the verbatim Table 4 row 1 string — mandatory,
                                               not a suggestion.  "Claim signature" is core-spec JSON-LD
                                               vocabulary and must not appear at L3 per §5.2 + Table 5. -->
                                          <span class="text-obsidian dark:text-quartz">{sigValid ? 'Signature valid' : C2PA_STATUS_INVALID}</span>
                                        </div>
                                        <div class="flex items-center gap-2 text-xs">
                                          <span class="w-4 text-center {dataValid ? 'text-malachite-dark dark:text-malachite-light' : hashFail ? 'text-cinnabar-dark dark:text-cinnabar-light' : 'text-flint-dark dark:text-flint-light'}">{dataValid ? '\u2713' : hashFail ? '\u2717' : '\u2014'}</span>
                                          <span class="text-obsidian dark:text-quartz">{dataValid ? 'Data integrity confirmed — file has not been modified' : hashFail ? 'Data integrity failed — file has been modified since signing' : 'Data hash not checked'}</span>
                                        </div>
                                        {#if tsValid}
                                          <div class="flex items-center gap-2 text-xs">
                                            <span class="w-4 text-center text-malachite-dark dark:text-malachite-light">{'\u2713'}</span>
                                            <span class="text-obsidian dark:text-quartz">Timestamp verified</span>
                                          </div>
                                        {/if}
                                        <!-- Certificate-expired disclosure (informational, L3 only).
                                             Spec: C2PA UX Rec v1.4 treats timestamped-valid as a
                                             Valid state, so this must NOT be a failure, warning,
                                             or red mark.  It is an amber advisory that preserves
                                             audit-trail transparency for the Pixel-Camera-style
                                             short-lived-credential case now that c2pa-rs no
                                             longer emits `signingCredential.expired` under a
                                             trusted chain.

                                             Copy is shared via C2PA_MSG_VALID_AT_SIGNING_DETAIL
                                             so v1, v2 and the PDF renderer cannot drift.  Below
                                             the prose we render the concrete validity window and
                                             TSA-asserted signing time as field rows — §6 requires
                                             the L3 detail view to expose underlying data, not
                                             prose alone.  Trigger: signing cert's notAfter is in
                                             the past, regardless of `isValid`. -->
                                        {#if sm.certificateExpired === true}
                                          {@const certUntil = formatCertDate(sm.certNotAfter)}
                                          {@const certFrom = formatCertDate(sm.certNotBefore)}
                                          {@const tsAsserted = formatCertDate(sm.signedAt)}
                                          <div class="flex items-start gap-2 text-xs">
                                            <span class="w-4 text-center text-amber-dark dark:text-amber-light shrink-0" aria-hidden="true">{'ⓘ'}</span>
                                            <span class="text-obsidian/90 dark:text-quartz/90 leading-relaxed">
                                              {C2PA_MSG_VALID_AT_SIGNING_DETAIL}
                                            </span>
                                          </div>
                                          {#if certUntil || certFrom || tsAsserted}
                                            <dl class="ml-6 mt-1 grid grid-cols-[max-content_1fr] gap-x-3 gap-y-0.5 text-[11px]">
                                              {#if certFrom}
                                                <dt class="text-flint-dark dark:text-flint-light">Certificate valid from</dt>
                                                <dd class="text-obsidian dark:text-quartz">{certFrom}</dd>
                                              {/if}
                                              {#if certUntil}
                                                <dt class="text-flint-dark dark:text-flint-light">Certificate valid until</dt>
                                                <dd class="text-obsidian dark:text-quartz">{certUntil}</dd>
                                              {/if}
                                              {#if tsAsserted}
                                                <dt class="text-flint-dark dark:text-flint-light">Timestamp asserted</dt>
                                                <dd class="text-obsidian dark:text-quartz">{tsAsserted}</dd>
                                              {/if}
                                            </dl>
                                          {/if}
                                        {/if}
                                        <!-- Trust-list warnings remain out of L3. `certUntrusted`
                                             used to fire on every file because c2pa-rs had no default
                                             trust list — now suppressed because the official CA + TSA
                                             lists are loaded.  Raw codes remain visible in L4 for
                                             forensic users. -->
                                        <!-- Pass/fail/info counters removed — the "failed" count includes
                                             cert-expired and cert-untrusted which are suppressed noise, making
                                             the counter misleading. Raw codes in L4 show the full picture. -->
                                      </div>

                                      <!-- L4: Raw validation codes (collapsible) -->
                                      <details class="mt-2">
                                        <summary class="text-[11px] text-lapis cursor-pointer hover:text-lapis dark:text-lapis-light">
                                          Show raw validation codes
                                        </summary>
                                        <ul class="mt-1.5 space-y-1 ml-1" aria-label="Raw C2PA validation codes">
                                          {#each checks as check}
                                            <li class="flex items-start gap-2">
                                              <span
                                                class="flex-shrink-0 w-1.5 h-1.5 rounded-full mt-1.5
                                                       {check.outcome === 'pass' ? 'bg-malachite dark:bg-malachite-light'
                                                        : check.outcome === 'fail' ? 'bg-cinnabar dark:bg-cinnabar-light'
                                                        : 'bg-flint dark:bg-flint-light'}"
                                                aria-hidden="true"
                                              ></span>
                                              <div>
                                                <span class="text-[11px] font-mono text-flint-dark dark:text-flint-light">{check.code}</span>
                                                {#if check.explanation}
                                                  <p class="text-[11px] text-obsidian/70 dark:text-quartz/70 mt-0.5">{check.explanation}</p>
                                                {/if}
                                              </div>
                                            </li>
                                          {/each}
                                        </ul>
                                      </details>
                                    </div>
                                  {/if}

                                  <!-- Redactions — spec MUST surface at L3 per C2PA UX
                                       Recommendations v1.4 §6 with both target and rationale,
                                       so users can see exactly which assertions were removed
                                       and why (e.g. "Rights-holder request"). -->
                                  {#if sm && sm.redactions && sm.redactions.length > 0}
                                    <div>
                                      <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-1.5">
                                        Redactions ({sm.redactions.length})
                                      </p>
                                      <ul class="space-y-2" aria-label="Redacted assertions">
                                        {#each sm.redactions as redaction}
                                          <li class="text-[11px] border-l-2 border-amber/60 pl-2">
                                            <span class="font-mono text-flint-dark dark:text-flint-light break-all">
                                              {redaction.target}
                                            </span>
                                            {#if redaction.reason}
                                              <p class="text-obsidian/80 dark:text-quartz/80 mt-0.5 italic">{redaction.reason}</p>
                                            {:else}
                                              <p class="text-flint-dark dark:text-flint-light mt-0.5 italic">No rationale provided</p>
                                            {/if}
                                          </li>
                                        {/each}
                                      </ul>
                                    </div>
                                  {/if}

                                  <!-- Assertions -->
                                  {#if sm && sm.assertions.length > 0}
                                    <div>
                                      <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-1.5">Assertions</p>
                                      <ul class="space-y-1" aria-label="Manifest assertions">
                                        {#each sm.assertions as assertion}
                                          <li class="text-[11px]">
                                            <span class="font-mono text-flint-dark dark:text-flint-light">{assertion.label}</span>
                                          </li>
                                        {/each}
                                      </ul>
                                    </div>
                                  {/if}

                                </div>
                              {/if}

                            </div>
                          {/if}
                          <!-- /L3 -->

                        </div>
                      {/if}
                      <!-- /L2 -->
                    {/if}
                    <!-- /valid manifest gate -->

                  </div>
                  <!-- /flex-1 -->
                </div>
                <!-- /L1 -->

              </li>

              <!-- Watermark Detection — provenance signal -->
              {#if result.watermarkExtractResult}
                <li class="px-5 py-4 {result.watermarkExtractResult.hasWatermark ? 'bg-malachite/[0.03]' : ''}">
                  <div class="flex items-start gap-3">
                    <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-malachite-dark dark:text-malachite-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/>
                    </svg>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-1">
                        <span class="text-sm font-medium text-obsidian dark:text-quartz">Watermark Detection</span>
                        {#if result.watermarkExtractResult.hasWatermark}
                          <span class="text-[10px] px-1.5 py-0.5 rounded-full bg-malachite/15 text-malachite-dark dark:text-malachite-light border border-malachite/30">
                            Found
                          </span>
                        {/if}
                      </div>
                      {#if result.watermarkExtractResult.hasWatermark}
                        <p class="text-xs text-malachite-dark dark:text-malachite-light">
                          Jura Trace watermark detected
                          {#if result.watermarkExtractResult.confidence != null}
                            — confidence: {Math.round(result.watermarkExtractResult.confidence * 100)}%
                          {/if}
                        </p>
                        {#if result.watermarkExtractResult.extractedPayload}
                          <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5 font-mono break-all">{result.watermarkExtractResult.extractedPayload}</p>
                        {/if}
                      {:else}
                        <p class="text-xs text-flint-dark dark:text-flint-light">No Jura Trace watermark detected. This is normal for files not protected via Jura Trace.</p>
                      {/if}
                      {#if showRawScores && result.watermarkExtractResult.confidence != null}
                        <p class="text-[10px] text-flint-dark dark:text-flint-light mt-1 tabular-nums">
                          confidence: {result.watermarkExtractResult.confidence.toFixed(4)}
                        </p>
                      {/if}
                    </div>
                  </div>
                </li>
              {/if}

              <!-- PDF Provenance -->
              {#if result.pdfProvenance}
                {@const pdf = result.pdfProvenance}
                <li class="px-5 py-4">
                  <div class="flex items-start gap-3">
                    <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-lapis dark:text-lapis-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>
                    </svg>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-2 flex-wrap">
                        <span class="text-sm font-medium text-obsidian dark:text-quartz">PDF Provenance</span>
                        <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-lapis/15 text-lapis-dark dark:text-lapis-light border border-lapis/30">Origin metadata only</span>
                        {#if pdf.hasDigitalSignature}
                          <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-malachite/15 text-malachite-dark dark:text-malachite-light border border-malachite/30">Digitally Signed</span>
                        {/if}
                        {#if pdf.isPdfA}
                          <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-malachite/15 text-malachite-dark dark:text-malachite-light border border-malachite/30">PDF/A</span>
                        {/if}
                        {#if pdf.hasIncrementalSaves}
                          <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-amber/15 text-amber-dark dark:text-amber-light border border-amber/30">Incremental Saves</span>
                        {/if}
                        {#if pdf.hasRedactionAnnotations}
                          <span class="text-[10px] px-1.5 py-0.5 rounded-full font-medium bg-amber/15 text-amber-dark dark:text-amber-light border border-amber/30">Redactions</span>
                        {/if}
                      </div>
                      <p class="text-[11px] text-flint-dark dark:text-flint-light italic mb-1.5" data-testid="pdf-scope-note">
                        Origin metadata only — image manipulation detection is not available for PDF files.
                      </p>
                      <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-2">{pdf.summary}</p>
                      <dl class="grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs">
                        {#if pdf.producer}
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Producer</dt>
                            <dd class="text-obsidian dark:text-quartz">{pdf.producer}</dd>
                          </div>
                        {/if}
                        {#if pdf.creator}
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Creator</dt>
                            <dd class="text-obsidian dark:text-quartz">{pdf.creator}</dd>
                          </div>
                        {/if}
                        {#if pdf.creationDate}
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Created</dt>
                            <dd class="text-obsidian dark:text-quartz">{pdf.creationDate}</dd>
                          </div>
                        {/if}
                        {#if pdf.modDate}
                          <div>
                            <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Modified</dt>
                            <dd class="text-obsidian dark:text-quartz">{pdf.modDate}</dd>
                          </div>
                        {/if}
                        <div>
                          <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">Pages</dt>
                          <dd class="text-obsidian dark:text-quartz tabular-nums">{pdf.pageCount}</dd>
                        </div>
                        <div>
                          <dt class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">PDF Version</dt>
                          <dd class="text-obsidian dark:text-quartz font-mono">{pdf.pdfVersion}</dd>
                        </div>
                      </dl>
                    </div>
                  </div>
                </li>
              {/if}

              <!-- Audio ENF Analysis card removed 2026-04-25 — the verify
                   pipeline never invoked enf_analysis (zero call-sites in
                   lib.rs) so the field was always null.  Restore both this
                   block and the enfAnalysisResult field on VerificationResult
                   when JTV-85 wires the audio detection pipeline (May Week 1). -->

            </ul>
            {#if result.inputQuality}
              <LimitationBanner quality={result.inputQuality} />
            {/if}
          </div>
        {/if}
      </div>

      <!-- Card 2: Is the content intact? -->
      <div
        id="card-integrity"
        class="bg-white dark:bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'integrity' ? 'border-lapis/30' : 'border-border-light dark:border-border-dark'}
               focus-within:border-lapis/25"
      >
        <button
          class="w-full flex items-center gap-3 px-5 py-4 text-left min-h-[56px] hover:bg-white/[0.02] transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light rounded-xl"
          aria-expanded={openCard === 'integrity'}
          aria-controls="card-integrity-body"
          onclick={() => openCard = openCard === 'integrity' ? null : 'integrity'}
        >
          <svg
            class="w-4 h-4 text-flint-dark dark:text-flint-light flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'integrity' ? 'rotate-90' : ''}"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true"
          ><path d="M9 5l7 7-7 7"/></svg>

          <span class="flex-1 font-serif text-base text-obsidian dark:text-quartz">Is the content intact?</span>

          <span class="text-xs text-flint-dark dark:text-flint-light mr-2">
            {(() => {
              const total = [result.elaResult, result.noiseResult, result.copyMoveResult, result.jpegGhostResult, result.segmentedElaResult, result.colourTemperatureResult, result.shadowConsistencyResult, result.spliceBoundaryResult].filter(Boolean).length;
              return `${total - integrityFindings} of ${total} passed`;
            })()}
          </span>

          <span class="text-xs font-semibold {cardPassClass(integrityPass())}">
            {integrityPass() === null ? '—' : integrityPass() ? 'Pass' : 'Concern'}
          </span>
        </button>

        {#if openCard === 'integrity'}
          <div id="card-integrity-body" class="border-t border-border-light dark:border-border-dark/60">
            <ul class="divide-y divide-border-light/70 dark:divide-border-dark/40" aria-label="Integrity checks">

              {#if result.elaResult}
                {@const ela = result.elaResult}
                <DetectorRow
                  name="Error Level Analysis"
                  score={ela.score}
                  suspicious={ela.suspicious}
                  helpAnchor="ela"
                  helpLabel="What does Error Level Analysis check?"
                >
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {ela.score.toFixed(4)} · threshold: {(ela as any).threshold?.toFixed(4) ?? '—'}</span>
                    {/if}
                  {/snippet}
                  {#if ela.elaImageUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(ela.elaImageUrl)}
                        alt="ELA heatmap showing compression artefact distribution"
                        caption={ela.suspicious
                          ? 'Click to enlarge — bright regions indicate higher compression-error mismatch'
                          : 'Click to enlarge — no significant compression anomalies detected'}
                      />
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              {#if result.noiseResult}
                {@const noise = result.noiseResult}
                <DetectorRow
                  name="Noise Pattern Analysis"
                  score={noise.score}
                  suspicious={noise.suspicious}
                  helpAnchor="noise-pattern"
                  helpLabel="What does Noise Pattern Analysis check?"
                  alwaysVisibleHint="Measures whether noise distribution is uniform across the photo — uneven noise across regions can indicate compositing."
                >
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {noise.score.toFixed(4)}</span>
                    {/if}
                  {/snippet}
                  {#if noise.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{noise.anomalousBlocks} of {noise.totalBlocks} blocks flagged</p>
                  {/if}
                </DetectorRow>
              {/if}

              {#if result.copyMoveResult}
                {@const cm = result.copyMoveResult}
                <DetectorRow
                  name="Copy-Move Detection"
                  score={cm.score}
                  suspicious={cm.suspicious}
                  helpAnchor="copy-move"
                  helpLabel="What does Copy-Move Detection check?"
                >
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {cm.score.toFixed(4)}</span>
                    {/if}
                  {/snippet}
                  {#if cm.suspicious && cm.cloneRegions.length > 0}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{cm.cloneRegions.length} cloned region{cm.cloneRegions.length === 1 ? '' : 's'} detected</p>
                  {:else if cm.visualisationUrl}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">No cloned regions detected.</p>
                  {/if}
                  {#if cm.visualisationUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(cm.visualisationUrl)}
                        alt={cm.suspicious
                          ? 'Copy-move detection visualisation showing cloned regions'
                          : 'Copy-move analysis — no cloned regions detected'}
                        caption={cm.suspicious
                          ? 'Click to enlarge — matched coloured pairs join the cloned regions'
                          : 'Click to enlarge — no cloned regions detected, image shown unmarked'}
                      />
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              {#if result.jpegGhostResult}
                {@const jg = result.jpegGhostResult}
                <DetectorRow
                  name="JPEG Ghost"
                  score={jg.score}
                  suspicious={jg.suspicious}
                  helpAnchor="jpeg-ghost"
                  helpLabel="What does JPEG Ghost check?"
                >
                  {#snippet badges()}
                    <ExperimentalPill variant="uncalibrated" tooltip="JPEG Ghost is weighted at 0.5× in the trust score. See methodology." />
                  {/snippet}
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {jg.score.toFixed(4)} · weight: 0.5×</span>
                    {/if}
                  {/snippet}
                  {#if jg.heatmapUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(jg.heatmapUrl)}
                        alt="JPEG Ghost heatmap showing re-compression artefact regions"
                        caption={jg.suspicious
                          ? 'Click to enlarge — dark regions deviate from the dominant compression history'
                          : 'Click to enlarge — no compression-history anomalies detected'}
                      />
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              {#if result.segmentedElaResult}
                {@const sela = result.segmentedElaResult}
                <DetectorRow
                  name="Segmented ELA"
                  score={sela.score}
                  suspicious={sela.suspicious}
                  helpAnchor="segmented-ela"
                  helpLabel="What does Segmented ELA check?"
                >
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {sela.score.toFixed(4)}</span>
                    {/if}
                  {/snippet}
                  {#if sela.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">
                      {sela.anomalousRegions} of {sela.totalRegions} image blocks show unusual compression — see <span aria-hidden="true">?</span> for what this means.
                    </p>
                  {/if}
                  {#if sela.heatmapUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(sela.heatmapUrl)}
                        alt="Segmented ELA region heatmap"
                        caption={sela.suspicious
                          ? 'Click to enlarge — flagged blocks show locally anomalous compression error'
                          : 'Click to enlarge — no localised compression anomalies detected'}
                      />
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              {#if result.colourTemperatureResult}
                {@const ct = result.colourTemperatureResult}
                <DetectorRow
                  name="Colour Temperature"
                  score={ct.score}
                  suspicious={ct.suspicious}
                  helpAnchor="colour-temperature"
                  helpLabel="What does Colour Temperature analysis check?"
                >
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {ct.score.toFixed(4)}</span>
                    {/if}
                  {/snippet}
                  {#if ct.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{ct.anomalousRegions} of {ct.totalRegions} regions flagged</p>
                  {/if}
                  {#if ct.heatmapUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(ct.heatmapUrl)}
                        alt="Colour temperature heatmap showing regions deviating from the global colour balance"
                        caption={ct.suspicious
                          ? 'Click to enlarge — flagged regions deviate in colour balance from the global average'
                          : 'Click to enlarge — no colour-balance anomalies detected'}
                      />
                    </div>
                  {/if}
                  {#if showRawScores}
                    <div class="mt-1 ml-6 grid grid-cols-2 gap-x-4 gap-y-0.5 text-[10px] text-flint-dark dark:text-flint-light tabular-nums">
                      <span>Global A (green-red): {ct.globalMeanA.toFixed(2)}</span>
                      <span>Global B (blue-yellow): {ct.globalMeanB.toFixed(2)}</span>
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              <!-- Shadow Consistency — deep mode, on-demand -->
              {#if result.shadowConsistencyResult}
                {@const sh = result.shadowConsistencyResult}
                <DetectorRow
                  name="Shadow Consistency"
                  score={sh.score}
                  suspicious={sh.suspicious}
                  helpAnchor="shadow-consistency"
                  helpLabel="What does Shadow Consistency check?"
                >
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {sh.score.toFixed(4)} · light dir: {sh.globalLightDirection.toFixed(1)}&deg;</span>
                    {/if}
                  {/snippet}
                  {#if sh.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{sh.inconsistentRegions} of {sh.totalRegions} regions inconsistent · global light {sh.globalLightDirection.toFixed(0)}&deg;</p>
                  {/if}
                  {#if sh.heatmapUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(sh.heatmapUrl)}
                        alt="Shadow consistency heatmap showing regions with inconsistent light direction"
                        caption={sh.suspicious
                          ? 'Click to enlarge — flagged regions cast shadows inconsistent with the global light direction'
                          : 'Click to enlarge — shadow directions consistent across the image'}
                      />
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              <!-- Splice Boundary — deep mode, on-demand -->
              {#if result.spliceBoundaryResult}
                {@const sb = result.spliceBoundaryResult}
                <DetectorRow
                  name="Splice Boundary"
                  score={sb.score}
                  suspicious={sb.suspicious}
                  helpAnchor="splice-boundary"
                  helpLabel="What does Splice Boundary check?"
                >
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {sb.score.toFixed(4)} · {sb.suspiciousBoundaries}/{sb.totalBoundariesChecked} boundaries</span>
                    {/if}
                  {/snippet}
                  {#if sb.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{sb.suspiciousBoundaries} of {sb.totalBoundariesChecked} boundaries flagged</p>
                  {/if}
                  {#if sb.heatmapUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(sb.heatmapUrl)}
                        alt="Splice boundary heatmap showing candidate cut edges between composited regions"
                        caption={sb.suspicious
                          ? 'Click to enlarge — bright lines mark candidate composite-edge boundaries'
                          : 'Click to enlarge — no splice-boundary candidates detected'}
                      />
                    </div>
                  {/if}
                  {#if sb.boundaries.length > 0}
                    <details class="mt-2 ml-6 group">
                      <summary class="list-none text-[11px] text-lapis dark:text-lapis-light cursor-pointer hover:text-obsidian dark:hover:text-quartz flex items-center gap-1 min-h-[24px]
                                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded">
                        <svg class="w-3 h-3 motion-safe:group-open:rotate-90 transition-transform duration-150" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 5l7 7-7 7"/></svg>
                        {sb.suspiciousBoundaries} candidate {sb.suspiciousBoundaries === 1 ? 'boundary' : 'boundaries'}
                      </summary>
                      <ul class="mt-1.5 space-y-1" aria-label="Splice boundary candidates">
                        {#each sb.boundaries as boundary, i (i)}
                          <li class="text-[11px] text-flint-dark dark:text-flint-light flex items-center justify-between gap-2">
                            <span class="font-mono">({boundary.x}, {boundary.y}) {boundary.width}&times;{boundary.height}px</span>
                            <span class="flex items-center gap-1.5 shrink-0">
                              {#if boundary.jpegGridAligned}<span class="text-[10px] px-1 py-px rounded bg-amber/10 text-amber-dark dark:text-amber-light">JPEG grid</span>{/if}
                              {#if boundary.noiseAsymmetric}<span class="text-[10px] px-1 py-px rounded bg-amber/10 text-amber-dark dark:text-amber-light">Noise</span>{/if}
                              {#if boundary.featheringDetected}<span class="text-[10px] px-1 py-px rounded bg-amber/10 text-amber-dark dark:text-amber-light">Feathering</span>{/if}
                              <span class="tabular-nums">{(boundary.confidence * 100).toFixed(0)}%</span>
                            </span>
                          </li>
                        {/each}
                      </ul>
                    </details>
                  {/if}
                </DetectorRow>
              {/if}

              <!-- NPR — deep mode, on-demand -->
              {#if result.nprResult}
                {@const npr = result.nprResult}
                <DetectorRow
                  name="Neighbouring Pixel Relationships"
                  score={npr.score}
                  suspicious={npr.suspicious}
                  helpAnchor="npr"
                  helpLabel="What does Neighbouring Pixel Relationships check?"
                >
                  {#snippet badges()}
                    <span class="text-[10px] px-1.5 py-px rounded-full bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30 font-normal">On-demand</span>
                  {/snippet}
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">score: {npr.score.toFixed(4)} · threshold: 40%</span>
                    {/if}
                  {/snippet}
                  {#if npr.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{npr.summary}</p>
                  {/if}
                  {#if npr.heatmapUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(npr.heatmapUrl)}
                        alt="Neighbouring pixel relationship heatmap showing local correlation anomalies"
                        caption={npr.suspicious
                          ? 'Click to enlarge — anomalies indicate atypical local pixel correlations versus natural images'
                          : 'Click to enlarge — pixel correlations consistent with a natural photograph'}
                      />
                    </div>
                  {/if}
                  {#if showRawScores}
                    <div class="mt-1 ml-6 grid grid-cols-3 gap-x-4 gap-y-0.5 text-[10px] text-flint-dark dark:text-flint-light tabular-nums">
                      <span>H-V correlation: {npr.hvCorrelation.toFixed(4)}</span>
                      <span>Diff variance ratio: {npr.diffVarianceRatio.toFixed(4)}</span>
                      <span>HF energy ratio: {npr.hfEnergyRatio.toFixed(4)}</span>
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              <!-- DCT Analysis — deep mode -->
              {#if result.dctAnalysisResult}
                {@const dct = result.dctAnalysisResult}
                <DetectorRow
                  name="DCT Analysis"
                  score={dct.score}
                  suspicious={dct.suspicious}
                  helpAnchor="dct-analysis"
                  helpLabel="What does DCT Analysis check?"
                >
                  {#snippet badges()}
                    <span class="text-[10px] px-1.5 py-px rounded-full bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30 font-normal">Deep</span>
                  {/snippet}
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">AC CV: {dct.acCoefficientOfVariation.toFixed(4)}</span>
                    {/if}
                  {/snippet}
                  {#if dct.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{dct.summary}</p>
                  {/if}
                  {#if dct.heatmapUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(dct.heatmapUrl)}
                        alt="DCT coefficient energy heatmap showing per-block AC distribution"
                        caption={dct.suspicious
                          ? 'Click to enlarge — uneven AC energy across JPEG blocks'
                          : 'Click to enlarge — uniform compression energy across the image'}
                      />
                    </div>
                  {/if}
                  {#if showRawScores}
                    <div class="mt-1 ml-6 grid grid-cols-3 gap-x-4 gap-y-0.5 text-[10px] text-flint-dark dark:text-flint-light tabular-nums">
                      <span>DC std: {dct.dcStd.toFixed(4)}</span>
                      <span>AC mean: {dct.acMean.toFixed(4)}</span>
                      <span>AC std: {dct.acStd.toFixed(4)}</span>
                    </div>
                  {/if}
                </DetectorRow>
              {/if}

              <!-- Fourier Analysis — deep mode -->
              {#if result.fourierAnalysisResult}
                {@const fou = result.fourierAnalysisResult}
                <DetectorRow
                  name="Fourier Analysis"
                  score={fou.score}
                  suspicious={fou.suspicious}
                  helpAnchor="fourier-analysis"
                  helpLabel="What does Fourier Analysis check?"
                >
                  {#snippet badges()}
                    <span class="text-[10px] px-1.5 py-px rounded-full bg-lapis/15 text-lapis dark:text-lapis-light border border-lapis/30 font-normal">Deep</span>
                  {/snippet}
                  {#snippet rawScore()}
                    {#if showRawScores}
                      <span class="text-[10px] text-flint-dark dark:text-flint-light tabular-nums">peaks: {fou.peakCount}</span>
                    {/if}
                  {/snippet}
                  {#if fou.suspicious}
                    <p class="text-xs text-flint-dark dark:text-flint-light mt-1 ml-6">{fou.summary}</p>
                  {/if}
                  {#if fou.spectrumUrl}
                    <div class="mt-2 ml-6">
                      <ImageZoom
                        src={heatmapSrc(fou.spectrumUrl)}
                        alt="Fourier spectrum showing log-magnitude FFT with detected periodic peaks"
                        caption={fou.suspicious
                          ? 'Click to enlarge — periodic peaks reveal regular structures (e.g. demosaicing or upscaling artefacts)'
                          : 'Click to enlarge — frequency spectrum consistent with a natural photograph'}
                      />
                    </div>
                  {/if}
                  {#if showRawScores}
                    <p class="mt-1 ml-6 text-[10px] text-flint-dark dark:text-flint-light tabular-nums">peak count: {fou.peakCount}</p>
                  {/if}
                </DetectorRow>
              {/if}

              {#if !result.elaResult && !result.noiseResult && !result.copyMoveResult}
                <li class="px-5 py-4">
                  <p class="text-sm text-flint-dark dark:text-flint-light">
                    Integrity checks require the Analysis Engine. {sidecarAvailable ? 'No data returned for this file type.' : 'Start the sidecar to enable forensic analysis.'}
                    <a href="/settings" class="text-lapis dark:text-lapis-light underline hover:text-obsidian dark:hover:text-quartz ml-1">Check service status</a>
                  </p>
                </li>
              {/if}

            </ul>

            <!-- On-demand detectors footer.
                 Shadow Consistency, Splice Boundary, and NPR do NOT auto-run
                 in any verify mode (see src-tauri/src/lib.rs:1621-1627 — the
                 deep group pins their results to None).  Each button below
                 invokes the dedicated Tauri command which calls the sidecar
                 endpoint and merges the result back into the active
                 VerificationResult.  When a detector returns, its row
                 appears above and disappears from this footer (gated on
                 result.<detector>Result presence per-button).
            -->
            {#if !result.shadowConsistencyResult || !result.spliceBoundaryResult || !result.nprResult}
              <div class="px-5 py-3 border-t border-border-light dark:border-border-dark/40 bg-white/[0.01]">
                <div class="flex items-center gap-2 mb-2 flex-wrap">
                  <span class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider font-semibold">On-demand investigation tools</span>
                  <span class="text-[11px] text-flint-dark dark:text-flint-light">
                    Run individually — these detectors do not auto-run in any verify mode.
                  </span>
                </div>
                <div class="flex flex-wrap gap-2">
                  {#if !result.nprResult}
                    <button
                      type="button"
                      class="text-xs px-3 py-1.5 rounded-md border border-border-light dark:border-border-dark bg-white dark:bg-graphite hover:border-lapis hover:text-lapis dark:hover:text-lapis-light transition-colors min-h-[32px]
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light disabled:opacity-50 disabled:cursor-not-allowed"
                      disabled={!filePath || !sidecarAvailable || onDemandLoading.npr}
                      onclick={() => runOnDemand('npr')}
                    >
                      {onDemandLoading.npr ? 'Running NPR…' : 'Run NPR'}
                    </button>
                  {/if}
                  {#if !result.shadowConsistencyResult}
                    <button
                      type="button"
                      class="text-xs px-3 py-1.5 rounded-md border border-border-light dark:border-border-dark bg-white dark:bg-graphite hover:border-lapis hover:text-lapis dark:hover:text-lapis-light transition-colors min-h-[32px]
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light disabled:opacity-50 disabled:cursor-not-allowed"
                      disabled={!filePath || !sidecarAvailable || onDemandLoading.shadow}
                      onclick={() => runOnDemand('shadow')}
                    >
                      {onDemandLoading.shadow ? 'Running Shadow Consistency…' : 'Run Shadow Consistency'}
                    </button>
                  {/if}
                  {#if !result.spliceBoundaryResult}
                    <button
                      type="button"
                      class="text-xs px-3 py-1.5 rounded-md border border-border-light dark:border-border-dark bg-white dark:bg-graphite hover:border-lapis hover:text-lapis dark:hover:text-lapis-light transition-colors min-h-[32px]
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light disabled:opacity-50 disabled:cursor-not-allowed"
                      disabled={!filePath || !sidecarAvailable || onDemandLoading.splice}
                      onclick={() => runOnDemand('splice')}
                    >
                      {onDemandLoading.splice ? 'Running Splice Boundary…' : 'Run Splice Boundary'}
                    </button>
                  {/if}
                </div>
                {#if onDemandError.npr || onDemandError.shadow || onDemandError.splice}
                  <p class="mt-2 text-xs text-cinnabar-dark dark:text-cinnabar-light" role="alert">
                    {onDemandError.npr ?? onDemandError.shadow ?? onDemandError.splice}
                  </p>
                {/if}
                {#if !sidecarAvailable}
                  <p class="mt-2 text-[11px] text-flint-dark dark:text-flint-light">
                    Analysis Engine unavailable — start the sidecar to enable these tools.
                  </p>
                {/if}
                {#if !filePath}
                  <p class="mt-2 text-[11px] text-flint-dark dark:text-flint-light">
                    Original file path not retained — re-verify the file to enable on-demand analysis.
                  </p>
                {/if}
              </div>
            {/if}

            {#if result.inputQuality}
              <LimitationBanner quality={result.inputQuality} />
            {/if}
          </div>
        {/if}
      </div>

      <!-- Card 3: Is this AI-generated? -->
      <div
        id="card-ai"
        class="bg-white dark:bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'ai' ? 'border-lapis/30' : 'border-border-light dark:border-border-dark'}
               focus-within:border-lapis/25"
      >
        <div class="flex items-center pr-5">
          <button
            class="flex-1 flex items-center gap-3 px-5 py-4 text-left min-h-[56px] hover:bg-white/[0.02] transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light rounded-xl"
            aria-expanded={openCard === 'ai'}
            aria-controls="card-ai-body"
            onclick={() => openCard = openCard === 'ai' ? null : 'ai'}
          >
            <svg
              class="w-4 h-4 text-flint-dark dark:text-flint-light flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'ai' ? 'rotate-90' : ''}"
              viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
              aria-hidden="true"
            ><path d="M9 5l7 7-7 7"/></svg>

            <span class="flex-1 font-serif text-base text-obsidian dark:text-quartz">Is this AI-generated?</span>

            {#if aiDetectionSuppressed}
              <span class="text-xs text-amber-dark dark:text-amber-light mr-2">Suppressed</span>
            {:else}
              <!-- State-label (JTV-92): replaces "X of 2 passed" — the old
                   counter framed a positive AI flag as a failure to "pass". -->
              {#if aiStateLabel()}
                <span class="text-xs text-flint-dark dark:text-flint-light mr-2">{aiStateLabel()}</span>
              {/if}
              <span class="text-xs font-semibold {cardPassClass(aiPass())}">
                {aiPass() === null ? '—' : aiPass() ? 'Pass' : 'Concern'}
              </span>
            {/if}
          </button>
          <ContextualHelpLink
            href="/help/how-it-works#two-ai-checks"
            label="Why two AI checks appear — open guide"
          />
        </div>

        {#if openCard === 'ai'}
          <div id="card-ai-body" class="border-t border-border-light dark:border-border-dark/60">
            {#if aiDetectionSuppressed}
              <div class="px-5 py-4">
                <p class="text-sm text-amber-dark/80 dark:text-amber-light/80">
                  AI detection signals have been suppressed for this file. The content-type classifier determined that deepfake and CLIP models are not reliable for this content type.
                  Scores were neutralised in the trust calculation.
                </p>
                <p class="text-xs text-flint-dark dark:text-flint-light mt-2">
                  Enable raw scores in Settings to inspect the suppressed values.
                </p>
              </div>
            {:else}
              <!-- Plain-English verdict sentence — first element inside the
                   AI card body so a non-technical reader gets the conclusion
                   before any score, badge, or signal list (JTV-89). -->
              {#if aiVerdictSentence()}
                <p
                  class="px-5 pt-4 text-sm text-obsidian dark:text-quartz leading-snug"
                  data-testid="ai-verdict-sentence"
                >
                  {aiVerdictSentence()}
                </p>
              {/if}
              <ul class="divide-y divide-border-light/70 dark:divide-border-dark/40 mt-3" aria-label="AI detection checks">

                {#if result.deepfakeResult}
                  {@const gbm = result.deepfakeResult}
                  {@const gbmSignals = gbm.signals ?? []}
                  {@const gbmTriggered = gbmSignals.filter((s) => s.triggered).length}
                  {@const gbmHighScoreNoSignals = gbmTriggered === 0 && (gbm.suspicious || gbm.verdictLevel === 'synthetic' || gbm.verdictLevel === 'inconclusive')}
                  <AiDetectorRow
                    name="AI Generation (GBM Deepfake)"
                    score={gbm.score}
                    suspicious={gbm.suspicious}
                    confidence="{gbm.confidence} confidence"
                  >
                    {#snippet badges()}
                      <!-- Threshold (JTV-93 / JTV-97): live value from
                           the sidecar response when present, else the
                           version-pinned fallback.  See comment on the
                           constant above for the rollout policy. -->
                      <span class="text-xs text-flint-dark dark:text-flint-light tabular-nums" aria-label="Synthetic verdict boundary">
                        threshold {Math.round((gbm.verdictThresholds?.syntheticMin ?? GBM_SYNTHETIC_THRESHOLD_FALLBACK) * 100)}%
                      </span>
                    {/snippet}
                    <p class="text-xs text-flint-dark dark:text-flint-light">{gbm.summary}</p>
                    <!-- Verdict text is gated behind raw-scores (JTV-91):
                         the row already communicates the verdict via three
                         redundant signals (amber/cinnabar background tint,
                         coloured icon, coloured title).  A fourth duplicate
                         text label was the surface that made "Synthetic"
                         read as a separate claim from the signal accordion
                         when 0 indicators triggered. -->
                    {#if showRawScores && gbm.verdictLevel}
                      <p class="text-xs mt-1 font-medium {gbm.verdictLevel === 'synthetic' ? 'text-cinnabar-dark dark:text-cinnabar-light' : gbm.verdictLevel === 'inconclusive' ? 'text-amber-dark dark:text-amber-light' : 'text-malachite-dark dark:text-malachite-light'}">
                        Verdict: {gbm.verdictLevel.charAt(0).toUpperCase() + gbm.verdictLevel.slice(1)}
                      </p>
                    {/if}
                    <!-- Bridging sentence (JTV-88): when the headline
                         verdict is non-authentic but no per-feature signal
                         individually tripped, explain the architecture so
                         the accordion below does not read as a contradiction. -->
                    {#if gbmHighScoreNoSignals && gbmSignals.length > 0}
                      <p class="text-[11px] text-flint-dark dark:text-flint-light italic mt-1.5 leading-snug">
                        Score reflects statistical patterns across the model's 84-feature vector. None of the {gbmSignals.length} named indicators triggered individually.
                      </p>
                    {/if}
                    {#if gbmSignals.length > 0}
                      <details class="mt-2 group {gbmTriggered === 0 ? 'opacity-60' : ''}">
                        <summary class="list-none text-[11px] text-lapis dark:text-lapis-light cursor-pointer hover:text-obsidian dark:hover:text-quartz flex items-center gap-1 min-h-[24px]
                                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded">
                          <svg class="w-3 h-3 motion-safe:group-open:rotate-90 transition-transform duration-150" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 5l7 7-7 7"/></svg>
                          {#if gbmTriggered === 0}
                            {gbmSignals.length} named indicator{gbmSignals.length === 1 ? '' : 's'} (none triggered)
                          {:else}
                            {gbmTriggered} of {gbmSignals.length} indicator{gbmSignals.length === 1 ? '' : 's'} triggered
                          {/if}
                        </summary>
                        <ul class="mt-1.5 space-y-1 ml-4" aria-label="Deepfake detector signals">
                          {#each gbmSignals as sig}
                            <li class="flex items-center justify-between gap-3 text-[11px]">
                              <span class="text-flint-dark dark:text-flint-light">{sig.name}</span>
                              <span class="{sig.triggered ? 'text-amber-dark dark:text-amber-light font-medium' : 'text-malachite-dark dark:text-malachite-light'}">
                                {sig.triggered ? 'Triggered' : 'Clear'} · w={sig.weight.toFixed(2)}
                              </span>
                            </li>
                          {/each}
                        </ul>
                      </details>
                    {/if}
                    {#if showRawScores}
                      <p class="text-[10px] text-flint-dark dark:text-flint-light mt-1 tabular-nums font-mono">
                        score: {gbm.score.toFixed(6)} · threshold: {(gbm as any).threshold?.toFixed(6) ?? '—'}
                      </p>
                    {/if}
                  </AiDetectorRow>
                {/if}

                {#if result.clipResult}
                  {@const clip = result.clipResult}
                  <AiDetectorRow
                    name="CLIP / UnivFD Probe"
                    score={clip.score}
                    suspicious={clip.verdictLevel === 'synthetic'}
                  >
                    {#snippet badges()}
                      <ExperimentalPill
                        variant="informational"
                        helpHref="/help/how-it-works#two-ai-checks"
                      />
                    {/snippet}
                    <p class="text-xs text-flint-dark dark:text-flint-light">{clip.summary}</p>
                    <!-- Class probability distribution (JTV-86).
                         Hidden by default because the zero-shot bars are
                         CLIP text-similarity to label prompts and are NOT
                         arithmetically related to the probe score that
                         drives the headline percentage.  Showing them by
                         default produced the pilot complaint that "87% +
                         ~20% bars" looked self-contradictory.  Surface
                         via either the per-row toggle or the global
                         Settings raw-scores switch. -->
                    {#if clip.classProbs && Object.keys(clip.classProbs).length > 0}
                      {@const isAuxiliary = clip.univfdAvailable === true}
                      <!-- Bars are hidden by default ONLY when UnivFD is the
                           headline signal (auxiliary case).  When UnivFD is
                           unavailable, the zero-shot bars ARE the headline
                           signal and must render by default — the toggle
                           below only applies to the auxiliary case. -->
                      {@const showZeroShot = !isAuxiliary || showRawScores || showClipZeroShot}
                      {#if !showZeroShot && isAuxiliary}
                        <button
                          type="button"
                          class="mt-2 text-[11px] text-lapis dark:text-lapis-light underline underline-offset-2 hover:text-obsidian dark:hover:text-quartz min-h-[24px]
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded"
                          onclick={() => (showClipZeroShot = true)}
                        >
                          Show zero-shot label distribution (auxiliary)
                        </button>
                      {/if}
                      {#if showZeroShot}
                        <div class="mt-2 space-y-1" aria-label="CLIP class probability distribution">
                          <div class="flex items-center justify-between mb-1">
                            <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider">
                              {#if isAuxiliary}
                                Zero-shot CLIP labels (auxiliary, not used for score)
                              {:else}
                                Zero-shot class probabilities
                              {/if}
                            </p>
                            {#if showClipZeroShot && !showRawScores}
                              <button
                                type="button"
                                class="text-[10px] text-flint-dark dark:text-flint-light hover:text-obsidian dark:hover:text-quartz underline underline-offset-2 min-h-[24px]
                                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded"
                                onclick={() => (showClipZeroShot = false)}
                                aria-label="Hide zero-shot label distribution"
                              >Hide</button>
                            {/if}
                          </div>
                          {#each Object.entries(clip.classProbs) as [cls, prob]}
                            <div class="flex items-center gap-2">
                              <span class="text-[10px] text-flint-dark dark:text-flint-light w-20 shrink-0 truncate" title={cls}>{cls}</span>
                              <div class="flex-1 bg-gray-200 dark:bg-graphite-light/50 rounded-full h-1.5 overflow-hidden" role="progressbar" aria-valuenow={Math.round(prob * 100)} aria-valuemin={0} aria-valuemax={100} aria-label="{cls}: {Math.round(prob * 100)}%">
                                <div
                                  class="h-full rounded-full {prob > 0.5 ? 'bg-amber dark:bg-amber-light' : 'bg-lapis dark:bg-lapis-light'}"
                                  style="width: {Math.round(prob * 100)}%"
                                ></div>
                              </div>
                              <span class="text-[10px] tabular-nums {forensicScoreClass(prob)} w-8 text-right">{Math.round(prob * 100)}%</span>
                            </div>
                          {/each}
                        </div>
                      {/if}
                    {/if}
                    {#if clip.univfdAvailable && showRawScores && clip.univfdScore != null}
                      <p class="text-[10px] text-flint-dark dark:text-flint-light mt-1 tabular-nums font-mono">
                        UnivFD probe score: {clip.univfdScore.toFixed(6)}
                      </p>
                    {/if}
                    {#if showRawScores}
                      <p class="text-[10px] text-flint-dark dark:text-flint-light mt-1 tabular-nums font-mono">
                        score: {clip.score.toFixed(6)} · confidence: {clip.confidence}
                      </p>
                    {/if}
                  </AiDetectorRow>
                {/if}

                {#if !result.deepfakeResult && !result.clipResult}
                  <li class="px-5 py-4">
                    <p class="text-sm text-flint-dark dark:text-flint-light">
                      AI detection requires the Analysis Engine.
                      {#if !sidecarAvailable}<a href="/settings" class="text-lapis dark:text-lapis-light underline hover:text-obsidian dark:hover:text-quartz">Check service status</a>{/if}
                    </p>
                  </li>
                {/if}

              </ul>
            {/if}

            <!-- ── Video Analysis — DROPPED FROM v1.0 SCOPE (2 May 2026) ───
                 Three-agent unanimous decision (project-manager + persona-
                 testing + content-authenticity-expert) to drop video deepfake
                 analysis from v1.0.  Reasoning: FFmpeg dependency is fragile
                 in the bundled installer, the per-frame pipeline has not been
                 calibration-validated against Global Majority devices or new
                 generators (Sora/Runway/HeyGen), and shipping an
                 unvalidated forensic result undermines the still-image
                 trust narrative.  Re-add planned for v1.0.1 with calibration
                 matrix + Tecno/Infinix/Samsung-A/Xiaomi device gate per
                 JTV-139.  See `project_v1_video_audio_drop.md` agent memory
                 for the full decision context.
                 The render below is a static "Planned — v1.0.1" panel that
                 appears for any video content type.  The underlying
                 `videoDeepfakeResult` is intentionally not rendered. -->
            {#if result.contentType === 'video'}
              <section
                class="px-5 py-4 border-t border-border-light dark:border-border-dark/40"
                aria-labelledby="video-analysis-heading"
              >
                <div class="flex items-center gap-3 mb-2 flex-wrap">
                  <h3 id="video-analysis-heading" class="text-sm font-medium text-obsidian dark:text-quartz">Video Analysis</h3>
                  <span
                    class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-lapis/10 text-lapis dark:text-lapis-light border border-lapis/20"
                  >
                    Planned — v1.0.1
                  </span>
                </div>
                <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-2">
                  Video deepfake analysis is in active development for v1.0.1. It will ship with calibration data covering Global Majority devices and current-generation generators (Sora, Runway Gen-3, HeyGen, Synthesia). v1.0 verifies what we can stand behind: provenance and metadata.
                </p>
                <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed">
                  <strong>Available now for video files:</strong> C2PA content credentials, EXIF metadata extraction, and native video preview above. <strong>Coming in v1.0.1:</strong> per-frame deepfake detection, audio-visual sync analysis, transcription, and claim verification.
                </p>
              </section>
            {/if}



            {#if result.inputQuality}
              <LimitationBanner quality={result.inputQuality} />
            {/if}
          </div>
        {/if}
      </div>

      <!-- Card 4: What does it claim? -->
      <div
        id="card-claims"
        class="bg-white dark:bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'claims' ? 'border-lapis/30' : 'border-border-light dark:border-border-dark'}
               focus-within:border-lapis/25"
      >
        <button
          class="w-full flex items-center gap-3 px-5 py-4 text-left min-h-[56px] hover:bg-white/[0.02] transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light rounded-xl"
          aria-expanded={openCard === 'claims'}
          aria-controls="card-claims-body"
          onclick={() => openCard = openCard === 'claims' ? null : 'claims'}
        >
          <svg
            class="w-4 h-4 text-flint-dark dark:text-flint-light flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'claims' ? 'rotate-90' : ''}"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true"
          ><path d="M9 5l7 7-7 7"/></svg>

          <span class="flex-1 font-serif text-base text-obsidian dark:text-quartz">What does it claim?</span>

          <span class="text-xs text-flint-dark dark:text-flint-light mr-2">
            {hasClaimsData ? 'Audio / video only' : 'Not applicable'}
          </span>
        </button>

        {#if openCard === 'claims'}
          <div id="card-claims-body" class="border-t border-border-light dark:border-border-dark/60 px-5 py-4">
            {#if result.transcriptionResult}
              <div class="mb-4">
                <h3 class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wider mb-2">Transcription</h3>
                <p class="text-sm text-obsidian dark:text-quartz leading-relaxed">{result.transcriptionResult.text}</p>
              </div>
            {/if}

            {#if result.claimCheckResult}
              <div>
                <h3 class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wider mb-2">Claim Check</h3>
                <p class="text-sm text-obsidian dark:text-quartz">{result.claimCheckResult.overallVerdict}</p>
                {#if result.claimCheckResult.summary}
                  <p class="text-xs text-flint-dark dark:text-flint-light mt-1">{result.claimCheckResult.summary}</p>
                {/if}
              </div>
            {/if}

            {#if result.ragClaimResult}
              <div>
                <h3 class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wider mb-2">RAG Claim Verification</h3>
                <p class="text-sm text-obsidian dark:text-quartz leading-relaxed">{result.ragClaimResult.explanation}</p>
              </div>
            {/if}

            {#if !hasClaimsData}
              <p class="text-sm text-flint-dark dark:text-flint-light">
                Transcription and claim checking are only applicable to audio and video files.
                For images, use the EXIF and Content Credentials sections above to assess provenance claims.
              </p>
            {/if}
          </div>
        {/if}
      </div>
    </section>

    <!-- ── AI Description (Ollama LLaVA) ────────────────────────────── -->
    {#if result.aiDescription}
      <section
        class="mb-4 bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl p-5"
        aria-labelledby="v2-ai-desc-heading"
      >
        <h2 id="v2-ai-desc-heading" class="font-serif text-base text-obsidian dark:text-quartz mb-2">AI Image Description</h2>
        <p class="text-sm text-obsidian dark:text-quartz leading-relaxed italic break-words whitespace-pre-wrap">"{result.aiDescription}"</p>
        <p class="mt-2 text-xs text-flint-dark dark:text-flint-light">Generated by LLaVA 7B via Ollama. This is an AI-generated description and is not a verified fact.</p>
      </section>
    {/if}

    <!-- ── Read Text (Ollama LLaVA) ──────────────────────────────────── -->
    {#if result.contentType === 'image' && filePath && sidecarHealth?.ollama !== null}
      <section
        class="mb-4 bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl p-5"
        aria-labelledby="v2-read-text-heading"
      >
        <div class="flex items-center justify-between flex-wrap gap-3 mb-3">
          <h2 id="v2-read-text-heading" class="font-serif text-base text-obsidian dark:text-quartz">Read Text</h2>
          <button
            type="button"
            onclick={handleExtractText}
            disabled={extractingText}
            aria-busy={extractingText}
            class="inline-flex items-center gap-2 text-xs px-3 py-2 min-h-[44px] rounded border border-lapis/40 text-lapis dark:text-lapis-light
                   hover:bg-lapis/10 transition-colors disabled:opacity-50 disabled:cursor-not-allowed
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
          >
            {#if extractingText}
              <svg class="w-3.5 h-3.5 motion-safe:animate-spin" fill="none" viewBox="0 0 24 24" aria-hidden="true">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"/>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8z"/>
              </svg>
              Reading text…
            {:else}
              Read Text (Ollama)
            {/if}
          </button>
        </div>
        <p class="text-xs text-flint-dark dark:text-flint-light leading-relaxed mb-3">
          Transcribe all visible text in this image using LLaVA. Useful for screenshots, social media posts, and document images.
        </p>
        {#if extractTextError}
          <div role="alert" aria-live="assertive" class="rounded border border-cinnabar/30 bg-cinnabar/10 px-4 py-3 text-xs text-cinnabar-dark dark:text-cinnabar-light leading-relaxed">
            {extractTextError}
          </div>
        {/if}
        {#if extractedText}
          <div role="status" aria-live="polite" class="rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/50 px-4 py-3">
            <p class="text-[10px] text-flint-dark dark:text-flint-light uppercase tracking-wider mb-2">Extracted Text</p>
            <pre class="text-sm text-obsidian dark:text-quartz whitespace-pre-wrap font-mono leading-relaxed">{extractedText}</pre>
            <p class="mt-3 text-xs text-flint-dark dark:text-flint-light">Extracted by LLaVA 7B via Ollama. Review carefully — AI models can misread text in low-resolution or heavily compressed images.</p>
          </div>
        {/if}
      </section>
    {/if}

    <!-- ── Signal Agreement Table ────────────────────────────────────── -->
    <section class="mb-4" aria-label="Signal agreement across all detectors">
      <details class="group bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl overflow-hidden">
        <summary class="list-none flex items-center gap-3 px-5 py-4 cursor-pointer hover:bg-white/[0.02] transition-colors min-h-[56px]
                        focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light"
                 aria-label="Signal Agreement — cross-detector summary table">
          <svg class="w-4 h-4 text-flint-dark dark:text-flint-light flex-shrink-0 motion-safe:group-open:rotate-90 transition-transform duration-200"
               viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
               aria-hidden="true"><path d="M9 5l7 7-7 7"/></svg>
          <span class="font-serif text-base text-obsidian dark:text-quartz flex-1">Signal Agreement</span>
          <span class="text-xs text-flint-dark dark:text-flint-light">Cross-detector overview</span>
        </summary>
        <div class="border-t border-border-light dark:border-border-dark/60 px-5 py-4">
          <SignalAgreement {result} />
        </div>
      </details>
    </section>

    <!-- ── Actions footer ──────────────────────────────────────────── -->
    <div class="flex items-center gap-3 flex-wrap bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl px-5 py-4">
      <button
        class="flex items-center gap-2 px-4 py-2.5 min-h-[44px] bg-lapis text-white text-sm font-medium rounded-lg hover:bg-lapis-dark transition-colors disabled:opacity-50
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
        disabled={exportingReport}
        onclick={() => showReportModal = true}
        aria-label="Export trust report as PDF"
      >
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>
        </svg>
        Export Report
      </button>

      <button
        class="flex items-center gap-2 px-4 py-2.5 min-h-[44px] border border-border-light dark:border-border-dark text-obsidian dark:text-quartz text-sm font-medium rounded-lg hover:bg-white/5 transition-colors disabled:opacity-50
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
        disabled={exportingCase}
        onclick={handleExportCase}
        aria-label="Export case archive as ZIP"
      >
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
        {exportingCase ? 'Exporting…' : 'Export Case'}
      </button>

      <button
        class="flex items-center gap-2 px-4 py-2.5 min-h-[44px] border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light text-sm rounded-lg hover:text-obsidian dark:hover:text-quartz hover:bg-white/5 transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
        onclick={() => showFalsePositiveModal = true}
        aria-label="Report a false positive result"
      >
        Report False Positive
      </button>

    </div>

    <!-- ── Methodology Panel ────────────────────────────────────────── -->
    <div class="mt-4">
      <MethodologyPanel {result} {sidecarHealth} {appVersion} />
    </div>

  {/if}<!-- end #if checked && result -->

</main>
