<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { writable } from 'svelte/store';
  import {
    verifyFile, verifyUrl, checkSidecarHealth, markFalsePositive,
    parseAppError, getLicenceTier, getVersion,
  } from '$lib/api';
  import { getTrustLevel, formatFileSize } from '$lib/types';
  import { createBlobTracker } from '$lib/blob';
  import type {
    VerificationResult, SidecarHealth, VerifyMode, LicenceTier,
    AnomalyFinding, InputQualityAssessment, ManifestInfo,
  } from '$lib/types';
  import LimitationBanner from '$lib/components/LimitationBanner.svelte';
  import ExperimentalPill from '$lib/components/ExperimentalPill.svelte';
  import ContentCredentialsSeal from '$lib/components/ContentCredentialsSeal.svelte';
  import ContextualHelpLink from '$lib/components/ContextualHelpLink.svelte';
  import { generateTrustReport } from '$lib/pdf';
  import type { ReportContext, ReportFormat } from '$lib/pdf';
  import { exportCaseZip } from '$lib/zip';
  import { saveVerifySession, restoreVerifySession, clearVerifySession } from '$lib/stores/verifySession';

  // ── State ──────────────────────────────────────────────────────────
  let activeTab = $state<'file' | 'url'>('file');
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

  // Forensic question card expand state
  let openCard = $state<'provenance' | 'integrity' | 'ai' | 'claims' | null>(null);

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

  // ── Derived ────────────────────────────────────────────────────────
  const rawTrustLevel = $derived(result ? getTrustLevel(result.overallTrust) : null);

  const hasUncertainDeepfake = $derived(
    result?.deepfakeResult?.verdictLevel === 'inconclusive' ||
    result?.deepfakeResult?.verdictLevel === 'synthetic'
  );

  const trustLevel = $derived(() => {
    if (!rawTrustLevel) return null;
    if (hasUncertainDeepfake && rawTrustLevel === 'high') return 'medium';
    return rawTrustLevel;
  });

  const trustScorePercent = $derived(result ? Math.round(result.overallTrust * 100) : 0);

  const trustColorClass = $derived(() => {
    const lv = trustLevel();
    if (lv === 'high') return 'text-malachite dark:text-malachite-light';
    if (lv === 'medium') return 'text-amber dark:text-amber-light';
    if (lv === 'low') return 'text-cinnabar dark:text-cinnabar-light';
    return 'text-flint dark:text-flint-light';
  });

  const trustStrokeColor = $derived(() => {
    const lv = trustLevel();
    if (lv === 'high') return '#5B8A5F';
    if (lv === 'medium') return '#D4943A';
    if (lv === 'low') return '#C45B52';
    return '#78756D';
  });

  const trustLabelText = $derived(() => {
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
    if (lv === 'high') return 'bg-malachite/15 text-malachite-light border-malachite/30';
    if (lv === 'medium') return 'bg-amber/15 text-amber-light border-amber/30';
    return 'bg-cinnabar/15 text-cinnabar-light border-cinnabar/30';
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

  // validAtSigning: certificate may have expired but the timestamp proves the
  // signature was valid when made — spec requires a malachite seal + amber note.
  const c2paValidAtSigning = $derived(
    result?.c2paManifest?.validAtSigning === true
  );

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

  // Map C2PA action URIs to C2PA UX Rec v1.4 recommended labels.
  const C2PA_ACTION_LABELS: Record<string, string> = {
    'c2pa.created':           'Created',
    'c2pa.edited':            'Other edits',
    'c2pa.cropped':           'Cropped',
    'c2pa.filtered':          'Filter or style edits',
    'c2pa.resized':           'Resized',
    'c2pa.published':         'Published',
    'c2pa.opened':            'Opened',
    'c2pa.placed':            'Imported',
    'c2pa.orientation':       'Changed orientation',
    'c2pa.color_adjustments': 'Colour or exposure edits',
    'c2pa.drawing':           'Drawing edits',
    'c2pa.converted':         'Converted',
    'c2pa.transcoded':        'Transcoded',
    'c2pa.removed':           'Removed',
    'c2pa.repackaged':        'Repackaged',
    'c2pa.unknown':           'Unknown edits or activity',
  };

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
  type DotState = 'pass' | 'concern' | 'suspicious' | 'suppressed' | 'not-run';

  function dotState(suspicious: boolean | undefined | null, ran: boolean, suppressed = false): DotState {
    if (suppressed) return 'suppressed';
    if (!ran) return 'not-run';
    if (suspicious === true) return 'suspicious';
    return 'pass';
  }

  const signalDots = $derived(() => {
    if (!result) return { provenance: [], integrity: [], ai: [] };

    const exifHighFindings = (result.exifAnalysis?.findings ?? []).filter(
      (f: AnomalyFinding) => f.severity === 'high' || f.severity === 'critical'
    );
    const exifSuspicious = exifHighFindings.length > 0;

    return {
      provenance: [
        {
          id: 'card-provenance', label: 'EXIF',
          state: dotState(exifSuspicious, result.exifAnalysis != null),
          ariaDetail: exifSuspicious ? `${exifHighFindings.length} high-severity anomal${exifHighFindings.length === 1 ? 'y' : 'ies'}` : 'No critical anomalies',
        },
        {
          id: 'card-provenance', label: 'Content Credentials',
          state: result.c2paValid === false ? 'suspicious' as DotState
               : result.c2paValid === true  ? 'pass' as DotState
               : 'not-run' as DotState,
          ariaDetail: result.c2paValid === true ? 'Valid' : result.c2paValid === false ? 'Invalid signature' : 'Not attached',
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
      ],
      ai: [
        {
          id: 'card-ai', label: 'Deepfake',
          state: aiDetectionSuppressed ? 'suppressed' as DotState : dotState(result.deepfakeResult?.suspicious, result.deepfakeResult != null),
          ariaDetail: aiDetectionSuppressed ? 'Suppressed — content type' : result.deepfakeResult ? `Score ${Math.round((result.deepfakeResult.score) * 100)}%` : 'Not run',
        },
        {
          id: 'card-ai', label: 'CLIP',
          state: aiDetectionSuppressed ? 'suppressed' as DotState : dotState(result.clipResult?.verdictLevel === 'synthetic', result.clipResult != null),
          ariaDetail: aiDetectionSuppressed ? 'Suppressed — content type' : result.clipResult ? `Score ${Math.round((result.clipResult.score) * 100)}%` : 'Not run',
        },
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
    return ![
      result.elaResult?.suspicious,
      result.noiseResult?.suspicious,
      result.copyMoveResult?.suspicious,
      result.jpegGhostResult?.suspicious,
      result.segmentedElaResult?.suspicious,
      result.colourTemperatureResult?.suspicious,
    ].some(Boolean);
  });

  const aiPass = $derived(() => {
    if (!result || aiDetectionSuppressed) return null;
    return !result.deepfakeResult?.suspicious && result.clipResult?.verdictLevel !== 'synthetic';
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
    ].filter(Boolean).length
  );

  const aiFindings = $derived(
    !result || aiDetectionSuppressed ? 0 : [
      result.deepfakeResult?.suspicious,
      result.clipResult?.verdictLevel === 'synthetic',
    ].filter(Boolean).length
  );

  // Total detectors that ran
  const detectorsRun = $derived(() => {
    if (!result) return 0;
    return [
      result.exifAnalysis, result.c2paValid !== undefined && result.c2paValid !== null,
      result.elaResult, result.noiseResult, result.copyMoveResult,
      result.deepfakeResult, result.jpegGhostResult, result.segmentedElaResult,
      result.colourTemperatureResult, result.clipResult, result.watermarkExtractResult,
    ].filter(Boolean).length;
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
      error = 'Unsupported file format. Jura Trace supports JPEG, PNG, TIFF, WebP, PDF, MP4, MOV, WAV and MP3.';
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
  $effect(() => {
    const path = filePath;
    const res = result;
    previewUrl = null;
    if (!path || !res || res.contentType !== 'image' || !inTauri) return;
    import('@tauri-apps/api/core').then(({ convertFileSrc }) => {
      previewUrl = convertFileSrc(path);
    }).catch(() => {});
  });

  // ── Playwright test hook ───────────────────────────────────────────
  if (typeof window !== 'undefined' && import.meta.env.DEV) {
    (window as any).__juraSetVerifyResult = (data: VerificationResult) => {
      _testResultStore.set(data);
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
    if (savedMode === 'standard' || savedMode === 'deep' || savedMode === 'archival') {
      verifyMode = savedMode;
    }
    analystName = localStorage.getItem('jura-analyst-name') ?? '';
    analystOrg = localStorage.getItem('jura-analyst-org') ?? '';
    analystNote = localStorage.getItem('jura-analyst-note') ?? '';
    analystDate = new Date().toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' });

    // Restore session
    if (!result) {
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
      await setupTauriDragDrop();
    })();

    function handleEsc(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        if (showImageOverlay) showImageOverlay = false;
        else if (showFalsePositiveModal) showFalsePositiveModal = false;
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
          { name: 'Supported Files', extensions: ['jpg','jpeg','png','tiff','tif','webp','avif','heic','heif','pdf','docx','mp4','mov','webm'] },
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
        }
        return;
      } catch {}
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
      const blob = generateTrustReport(result, {
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
      await markFalsePositive(
        fpReasonCode,
        fpReasonNote.trim() || undefined,
        result.contentType,
        result.deepfakeResult?.score,
        result.deepfakeResult?.verdictLevel,
      );
      fpSubmitted = true;
      setTimeout(() => {
        showFalsePositiveModal = false;
        fpSubmitted = false;
        fpReasonCode = 'modern_codec';
        fpReasonNote = '';
      }, 2000);
    } finally {
      fpSubmitting = false;
    }
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
  function dotBgClass(state: DotState): string {
    if (state === 'pass') return 'bg-malachite dark:bg-malachite-light';
    if (state === 'concern') return 'bg-amber dark:bg-amber-light';
    if (state === 'suspicious') return 'bg-amber dark:bg-amber-light';
    if (state === 'suppressed') return 'bg-flint/40';
    return 'bg-flint/30';
  }

  function dotPulse(state: DotState): boolean {
    return state === 'suspicious' || state === 'concern';
  }

  function forensicScoreClass(score: number): string {
    if (score < 0.3) return 'text-malachite dark:text-malachite-light';
    if (score < 0.6) return 'text-amber dark:text-amber-light';
    return 'text-cinnabar dark:text-cinnabar-light';
  }

  function cardPassClass(pass: boolean | null): string {
    if (pass === null) return 'text-flint dark:text-flint-light';
    if (pass) return 'text-malachite dark:text-malachite-light';
    return 'text-amber dark:text-amber-light';
  }

  function highestSeverityFindings(findings: AnomalyFinding[]): AnomalyFinding[] {
    const order: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
    return [...findings].sort((a, b) => (order[a.severity] ?? 5) - (order[b.severity] ?? 5));
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
      class="absolute top-4 right-4 w-10 h-10 flex items-center justify-center rounded-full bg-graphite/80 text-quartz
             hover:bg-graphite focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
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
    <div class="bg-graphite border border-border-dark rounded-xl p-6 w-full max-w-md space-y-4">
      <h2 class="font-serif text-lg text-quartz">Export Trust Report</h2>
      <div class="space-y-3">
        <div>
          <label for="v2-analyst-name" class="block text-xs text-flint dark:text-flint-light mb-1">Analyst name (optional)</label>
          <input id="v2-analyst-name" type="text" bind:value={analystName}
            class="w-full bg-obsidian border border-border-dark rounded px-3 py-2 text-sm text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light" />
        </div>
        <div>
          <label for="v2-analyst-org" class="block text-xs text-flint dark:text-flint-light mb-1">Organisation (optional)</label>
          <input id="v2-analyst-org" type="text" bind:value={analystOrg}
            class="w-full bg-obsidian border border-border-dark rounded px-3 py-2 text-sm text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light" />
        </div>
        <div>
          <label for="v2-case-ref" class="block text-xs text-flint dark:text-flint-light mb-1">Case reference (optional)</label>
          <input id="v2-case-ref" type="text" bind:value={analystCaseRef}
            class="w-full bg-obsidian border border-border-dark rounded px-3 py-2 text-sm text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light" />
        </div>
        <div>
          <label for="v2-report-format" class="block text-xs text-flint dark:text-flint-light mb-1">Format</label>
          <select id="v2-report-format" bind:value={reportFormat}
            class="w-full bg-obsidian border border-border-dark rounded px-3 py-2 text-sm text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light">
            <option value="standard">Standard</option>
            <option value="detailed">Detailed</option>
          </select>
        </div>
      </div>
      <div class="flex gap-3 justify-end pt-2">
        <button
          class="px-4 py-2 min-h-[44px] text-sm text-flint dark:text-flint-light border border-border-dark rounded-lg hover:text-quartz transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
          onclick={() => showReportModal = false}
        >Cancel</button>
        <button
          class="px-4 py-2 min-h-[44px] text-sm bg-lapis text-quartz rounded-lg font-medium hover:bg-lapis-dark transition-colors disabled:opacity-50
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
    onclick={(e) => { if (e.target === e.currentTarget) showFalsePositiveModal = false; }}
  >
    <div class="bg-graphite border border-border-dark rounded-xl p-6 w-full max-w-md space-y-4">
      <h2 class="font-serif text-lg text-quartz">Report False Positive</h2>
      {#if fpSubmitted}
        <p class="text-malachite dark:text-malachite-light text-sm" role="status" aria-live="polite">Thank you — your feedback has been recorded locally.</p>
      {:else}
        <div class="space-y-3">
          <div>
            <label for="v2-fp-reason" class="block text-xs text-flint dark:text-flint-light mb-1">Reason</label>
            <select id="v2-fp-reason" bind:value={fpReasonCode}
              class="w-full bg-obsidian border border-border-dark rounded px-3 py-2 text-sm text-quartz focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light">
              <option value="modern_codec">Modern codec (AVIF/WebP)</option>
              <option value="high_iso">High ISO / grain</option>
              <option value="heavy_editing">Heavy post-processing</option>
              <option value="scanner">Scanned document</option>
              <option value="other">Other</option>
            </select>
          </div>
          <div>
            <label for="v2-fp-note" class="block text-xs text-flint dark:text-flint-light mb-1">Additional notes (optional)</label>
            <textarea id="v2-fp-note" bind:value={fpReasonNote} rows="3"
              class="w-full bg-obsidian border border-border-dark rounded px-3 py-2 text-sm text-quartz resize-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"></textarea>
          </div>
        </div>
        <div class="flex gap-3 justify-end pt-2">
          <button
            class="px-4 py-2 min-h-[44px] text-sm text-flint dark:text-flint-light border border-border-dark rounded-lg hover:text-quartz transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
            onclick={() => showFalsePositiveModal = false}
          >Cancel</button>
          <button
            class="px-4 py-2 min-h-[44px] text-sm bg-lapis text-quartz rounded-lg font-medium hover:bg-lapis-dark transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
            disabled={fpSubmitting}
            onclick={handleFalsePositiveSubmit}
          >{fpSubmitting ? 'Submitting…' : 'Submit'}</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- ─── Page ───────────────────────────────────────────────────────── -->
<main id="main-content" tabindex="-1" class="outline-none max-w-[900px] mx-auto px-6 py-8 pb-16">

  <!-- Header -->
  <div class="flex items-center gap-3 mb-6">
    <h1 class="font-serif text-2xl text-quartz">Verify</h1>
    <span class="px-2 py-0.5 text-[10px] font-semibold tracking-widest uppercase rounded-full bg-lapis/15 text-lapis-light border border-lapis/25">
      v2 Preview
    </span>
    <a
      href="/verify"
      class="ml-auto text-xs text-flint dark:text-flint-light hover:text-quartz transition-colors underline underline-offset-2
             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded"
    >Switch to classic view</a>
  </div>

  <!-- Mode selector + sidecar status row -->
  <div class="flex items-center gap-3 mb-5 flex-wrap">
    <div role="radiogroup" aria-label="Verification mode" class="flex rounded-lg border border-border-dark overflow-hidden">
      {#each [
        { mode: 'standard' as VerifyMode, label: 'Standard', description: '~15s' },
        { mode: 'deep' as VerifyMode, label: 'Deep', description: '~60s' },
        { mode: 'archival' as VerifyMode, label: 'Archival', description: '~2min' },
      ] as opt}
        <button
          class="px-3 py-1.5 text-xs font-medium transition-colors min-h-[36px]
                 {verifyMode === opt.mode
                   ? 'bg-lapis/20 text-lapis-light'
                   : 'text-flint dark:text-flint-light hover:text-quartz'}
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light"
          role="radio"
          aria-checked={verifyMode === opt.mode}
          onclick={() => { verifyMode = opt.mode; localStorage.setItem('jura-verify-mode', opt.mode); }}
        >{opt.label} <span class="opacity-50 ml-0.5">{opt.description}</span></button>
      {/each}
    </div>

    <div
      class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-full border
             {sidecarAvailable
               ? 'bg-malachite/10 text-malachite dark:text-malachite-light border-malachite/20'
               : 'bg-graphite text-flint dark:text-flint-light border-border-dark'}"
      role="status"
      aria-label={sidecarAvailable ? 'Analysis services connected' : 'Analysis services offline'}
    >
      <span class="w-1.5 h-1.5 rounded-full {sidecarAvailable ? 'bg-malachite' : 'bg-flint/50'}" aria-hidden="true"></span>
      {sidecarAvailable ? 'Services connected' : 'Services offline'}
    </div>

    {#if checked && result}
      <button
        class="ml-auto text-xs text-flint dark:text-flint-light hover:text-quartz transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded px-2 py-1"
        onclick={reset}
        aria-label="Clear result and verify a new file"
      >Clear result</button>
    {/if}
  </div>

  <!-- Error banner -->
  {#if error}
    <div
      class="mb-5 rounded-lg px-4 py-3 text-sm border
        {errorType === 'sidecar' ? 'bg-amber/10 border-amber/30 text-amber dark:text-amber-light' :
         errorType === 'format'  ? 'bg-lapis/10 border-lapis/30 text-lapis dark:text-lapis-light' :
         'bg-cinnabar/10 border-cinnabar/30 text-cinnabar dark:text-cinnabar-light'}"
      role="alert"
      aria-live="assertive"
    >
      <span class="font-medium">{errorType === 'sidecar' ? 'Analysis Engine offline' : errorType === 'format' ? 'Unsupported format' : 'Error'}:</span>
      {error}
    </div>
  {/if}

  <!-- ── Input panel ─────────────────────────────────────────────── -->
  {#if !checked || !result}
    <div class="mb-6 bg-graphite border border-border-dark rounded-xl overflow-hidden">
      <!-- Tab bar -->
      <div role="tablist" aria-label="Verification input method" class="flex border-b border-border-dark">
        {#each [{ id: 'file', label: 'File' }, { id: 'url', label: 'URL' }] as tab}
          <button
            class="px-5 py-3 text-sm font-medium border-b-2 -mb-px transition-colors
                   {activeTab === tab.id
                     ? 'text-lapis-light border-lapis-light'
                     : 'text-flint dark:text-flint-light border-transparent hover:text-quartz'}
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light"
            role="tab"
            aria-selected={activeTab === tab.id}
            aria-controls="v2-tab-{tab.id}"
            id="v2-tab-btn-{tab.id}"
            onclick={() => activeTab = tab.id as 'file' | 'url'}
          >{tab.label}</button>
        {/each}
      </div>

      <!-- File tab -->
      {#if activeTab === 'file'}
        <div id="v2-tab-file" role="tabpanel" aria-labelledby="v2-tab-btn-file" class="p-4">
          <button
            class="w-full border-2 border-dashed rounded-lg p-10 text-center transition-all duration-200 cursor-pointer
                   {dragOver ? 'border-lapis-light bg-lapis/5' : 'border-border-dark hover:border-lapis/50'}
                   {loading ? 'opacity-60 pointer-events-none' : ''}
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light focus-visible:ring-offset-2 focus-visible:ring-offset-graphite"
            ondragover={handleDragOver}
            ondragleave={handleDragLeave}
            ondrop={handleDrop}
            onclick={handleFileClick}
            aria-label="Drop a file here or click to browse — supported formats: JPEG, PNG, TIFF, WebP, PDF, MP4, MOV"
            aria-busy={loading}
          >
            {#if loading}
              <div class="flex flex-col items-center gap-3">
                <div class="w-6 h-6 border-2 border-lapis-light border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Analysing"></div>
                <p class="text-sm text-quartz font-medium">
                  {verifyMode === 'archival' ? 'Running archival analysis — up to 2 minutes…'
                   : verifyMode === 'deep' ? 'Running deep analysis — up to 60 seconds…'
                   : 'Running standard analysis…'}
                </p>
                {#if fileName}
                  <p class="text-xs text-flint dark:text-flint-light">{fileName}</p>
                {/if}
                {#if analysisElapsed > 2}
                  <p class="text-xs text-flint/60 tabular-nums">{analysisElapsed}s elapsed</p>
                {/if}
              </div>
            {:else}
              <div class="flex flex-col items-center gap-2">
                <svg class="w-10 h-10 text-flint dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                    d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>
                </svg>
                <p class="text-quartz font-medium">Drop a file to verify</p>
                <p class="text-xs text-flint dark:text-flint-light">or click to browse</p>
                <p class="text-xs text-flint/70 dark:text-flint-light/70 mt-1">JPEG · PNG · TIFF · WebP · PDF · MP4 · MOV · WAV · MP3</p>
              </div>
            {/if}
          </button>
          {#if loading}
            <div class="mt-3 flex justify-center">
              <button
                class="text-xs text-flint dark:text-flint-light hover:text-cinnabar dark:hover:text-cinnabar-light transition-colors px-3 py-1.5 min-h-[36px]
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
          <label for="v2-url-input" class="block text-xs text-flint dark:text-flint-light mb-2">
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
              class="flex-1 bg-obsidian border border-border-dark rounded-lg px-3 py-2.5 text-sm text-quartz placeholder:text-flint/50
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light disabled:opacity-50"
            />
            <button
              class="px-4 py-2.5 min-h-[44px] bg-lapis text-quartz text-sm font-medium rounded-lg hover:bg-lapis-dark transition-colors disabled:opacity-50
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
              disabled={!urlInput.trim() || loading}
              onclick={runUrlVerification}
            >{loading ? 'Analysing…' : 'Verify'}</button>
          </div>
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
              <summary class="list-none text-xs text-amber/70 cursor-pointer flex items-center gap-1.5 hover:text-amber-light transition-colors min-h-[24px]
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber rounded" aria-label="Why was AI detection suppressed?">
                <svg class="w-3 h-3 motion-safe:group-open:rotate-90 transition-transform duration-150 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M9 5l7 7-7 7"/>
                </svg>
                Why?
              </summary>
              <p class="mt-2 text-xs text-amber/65 leading-relaxed">{result.contentTypeResult.reasoning}</p>
            </details>
          </div>
        </div>
      </div>
    {/if}

    <!-- ── 1. Trust Score Banner ───────────────────────────────────── -->
    <section aria-label="Verification result summary" class="mb-4">
      <div class="bg-graphite border border-border-dark rounded-xl p-6 flex items-center gap-6 flex-wrap sm:flex-nowrap">

        <!-- Image preview thumbnail -->
        {#if previewUrl}
          <button
            class="flex-shrink-0 w-[200px] h-[150px] rounded-lg overflow-hidden border border-border-dark bg-obsidian relative group
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
              class="absolute bottom-2 right-2 bg-black/60 text-quartz text-[10px] px-1.5 py-0.5 rounded opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity duration-150"
              aria-hidden="true"
            >Enlarge</span>
          </button>
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
            <h2 class="font-serif text-xl text-quartz">{trustLabelText()}</h2>
            {#if trustLevel()}
              <span class="px-2 py-0.5 text-[10px] font-bold tracking-widest uppercase rounded-full border {verdictBadgeClass()}" role="status" aria-live="polite">
                {trustLevel() === 'high' ? 'Authentic' : trustLevel() === 'medium' ? 'Review' : 'Suspicious'}
              </span>
            {/if}
          </div>

          <p class="text-sm text-flint dark:text-flint-light mb-3">
            {fileName}{#if imageDimensions()} · {imageDimensions()}{/if}
          </p>

          <!-- Breakdown row -->
          <div class="flex items-center gap-4 flex-wrap border-t border-border-dark/60 pt-3" role="list" aria-label="Verification summary">
            <div role="listitem" class="flex flex-col gap-0.5">
              <span class="text-[10px] text-flint uppercase tracking-wider">Detectors run</span>
              <span class="text-sm text-quartz font-medium">{detectorsRun()} / 12</span>
            </div>
            <div role="separator" aria-hidden="true" class="w-px h-6 bg-border-dark"></div>
            <div role="listitem" class="flex flex-col gap-0.5">
              <span class="text-[10px] text-flint uppercase tracking-wider">Findings</span>
              <span class="text-sm font-medium {totalFindings > 0 ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}">
                {totalFindings === 0 ? 'None' : totalFindings === 1 ? '1 concern' : `${totalFindings} concerns`}
              </span>
            </div>
            {#if cameraLabel()}
              <div role="separator" aria-hidden="true" class="w-px h-6 bg-border-dark"></div>
              <div role="listitem" class="flex flex-col gap-0.5">
                <span class="text-[10px] text-flint uppercase tracking-wider">Camera</span>
                <span class="text-sm text-quartz font-medium">{cameraLabel()}</span>
              </div>
            {/if}
            <div role="separator" aria-hidden="true" class="w-px h-6 bg-border-dark"></div>
            <div role="listitem" class="flex flex-col gap-0.5">
              <span class="text-[10px] text-flint uppercase tracking-wider">Content Credentials</span>
              <span class="text-sm font-medium {result.c2paValid === true ? 'text-malachite dark:text-malachite-light' : result.c2paValid === false ? 'text-cinnabar dark:text-cinnabar-light' : 'text-flint dark:text-flint-light'}">
                {result.c2paValid === true ? 'Signed & valid' : result.c2paValid === false ? 'Invalid' : 'Not attached'}
              </span>
            </div>
          </div>
        </div>

      </div>
    </section>

    <!-- ── 2. Signal Map Strip ────────────────────────────────────── -->
    <section aria-label="Signal overview — all detectors at a glance" class="mb-5">
      <div class="bg-graphite border border-border-dark rounded-xl px-5 py-4">
        <p class="text-[10px] text-flint uppercase tracking-widest mb-3">Signal Map — click any detector to view detail</p>

        <div class="flex items-start gap-0" role="group" aria-label="Detector signals grouped by category">

          <!-- Provenance group -->
          <div class="flex-1 pr-4">
            <p class="text-[10px] text-flint uppercase tracking-wider font-semibold mb-2" id="sm-prov">Provenance</p>
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
                  <span class="text-xs text-flint dark:text-flint-light {dot.state === 'suspicious' ? 'text-amber-light font-medium' : ''}">{dot.label}</span>
                </button>
              {/each}
            </div>
          </div>

          <div class="w-px bg-border-dark self-stretch mx-1" role="separator" aria-hidden="true"></div>

          <!-- Integrity group -->
          <div class="flex-[2] px-4">
            <p class="text-[10px] text-flint uppercase tracking-wider font-semibold mb-2" id="sm-int">Integrity</p>
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
                  <span class="text-xs text-flint dark:text-flint-light {dot.state === 'suspicious' ? 'text-amber-light font-medium' : ''}">{dot.label}</span>
                </button>
              {/each}
            </div>
          </div>

          <div class="w-px bg-border-dark self-stretch mx-1" role="separator" aria-hidden="true"></div>

          <!-- AI Detection group -->
          <div class="flex-1 pl-4">
            <p class="text-[10px] text-flint uppercase tracking-wider font-semibold mb-2" id="sm-ai">AI Detection</p>
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
                  <span class="text-xs {dot.state === 'suppressed' ? 'text-flint/50 line-through' : 'text-flint dark:text-flint-light'} {dot.state === 'suspicious' ? 'text-amber-light font-medium' : ''}">{dot.label}</span>
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
        <h2 class="font-serif text-lg text-quartz">All Checks</h2>
        <span class="text-xs text-flint dark:text-flint-light">Click a question to expand the full analysis</span>
      </div>

      <!-- Card 1: Does the provenance hold? -->
      <div
        id="card-provenance"
        class="bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'provenance' ? 'border-lapis/30' : 'border-border-dark'}
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
            class="w-4 h-4 text-flint flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'provenance' ? 'rotate-90' : ''}"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true"
          ><path d="M9 5l7 7-7 7"/></svg>

          <span class="flex-1 font-serif text-base text-quartz">Does the provenance hold?</span>

          <span class="text-xs text-flint dark:text-flint-light mr-2">
            {provenanceFindings === 0 ? '2 of 2 passed' : `${2 - provenanceFindings} of 2 passed`}
          </span>

          <span class="text-xs font-semibold {cardPassClass(provenancePass())}">
            {provenancePass() === null ? '—' : provenancePass() ? 'Pass' : 'Concern'}
          </span>
        </button>

        {#if openCard === 'provenance'}
          <div id="card-provenance-body" class="border-t border-border-dark/60">
            <ul class="divide-y divide-border-dark/40" aria-label="Provenance checks">

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
                      <span class="text-sm font-medium text-quartz">EXIF Metadata Analysis</span>
                      {#if result.exifAnalysis}
                        <span class="text-xs px-1.5 py-0.5 rounded bg-graphite-light border border-border-dark text-flint dark:text-flint-light">
                          {result.exifAnalysis.fieldsPopulated} / {result.exifAnalysis.fieldsTotal} fields
                        </span>
                      {/if}
                    </div>
                    {#if result.exifAnalysis}
                      {#if result.exifAnalysis.findings.length === 0}
                        <p class="text-xs text-malachite dark:text-malachite-light">No anomalies detected</p>
                      {:else}
                        <ul class="space-y-1.5 mt-1">
                          {#each highestSeverityFindings(result.exifAnalysis.findings).slice(0, 5) as finding}
                            <li class="text-xs leading-relaxed">
                              <span class="font-medium {finding.severity === 'critical' || finding.severity === 'high' ? 'text-amber dark:text-amber-light' : finding.severity === 'medium' ? 'text-amber/70 dark:text-amber-light/70' : 'text-flint dark:text-flint-light'}">
                                [{finding.severity.toUpperCase()}]
                              </span>
                              <span class="text-quartz ml-1">{finding.title}</span>
                              <span class="text-flint dark:text-flint-light ml-1">— {finding.description}</span>
                            </li>
                          {/each}
                          {#if result.exifAnalysis.findings.length > 5}
                            <li class="text-xs text-flint dark:text-flint-light">
                              + {result.exifAnalysis.findings.length - 5} more — <a href="/verify" class="text-lapis-light underline hover:text-quartz">view all in classic view</a>
                            </li>
                          {/if}
                        </ul>
                      {/if}
                    {:else}
                      <p class="text-xs text-flint dark:text-flint-light">EXIF data not available for this file type.</p>
                    {/if}

                    <!-- Expandable raw EXIF metadata fields -->
                    {#if result.imageMetadata}
                      {@const meta = result.imageMetadata}
                      <details class="mt-2">
                        <summary class="text-[11px] text-lapis cursor-pointer hover:text-lapis-light">
                          View EXIF metadata fields
                        </summary>
                        <div class="mt-2 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs">
                          {#if meta.cameraMake || meta.cameraModel}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Camera</p>
                              <p class="text-quartz">{[meta.cameraMake, meta.cameraModel].filter(Boolean).join(' ')}</p>
                            </div>
                          {/if}
                          {#if meta.software}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Software</p>
                              <p class="text-quartz">{meta.software}</p>
                            </div>
                          {/if}
                          {#if meta.datetimeOriginal}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Date taken</p>
                              <p class="text-quartz">{meta.datetimeOriginal}</p>
                            </div>
                          {/if}
                          {#if meta.datetimeModified}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Date modified</p>
                              <p class="text-quartz">{meta.datetimeModified}</p>
                            </div>
                          {/if}
                          {#if meta.iso}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">ISO</p>
                              <p class="text-quartz">{meta.iso}</p>
                            </div>
                          {/if}
                          {#if meta.focalLength}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Focal length</p>
                              <p class="text-quartz">{meta.focalLength}</p>
                            </div>
                          {/if}
                          {#if meta.exposureTime}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Exposure</p>
                              <p class="text-quartz">{meta.exposureTime}</p>
                            </div>
                          {/if}
                          {#if meta.fNumber}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Aperture</p>
                              <p class="text-quartz">{meta.fNumber}</p>
                            </div>
                          {/if}
                          {#if meta.colorSpace}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Colour space</p>
                              <p class="text-quartz">{meta.colorSpace}</p>
                            </div>
                          {/if}
                          {#if meta.exifWidth && meta.exifHeight}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">EXIF dimensions</p>
                              <p class="text-quartz">{meta.exifWidth} x {meta.exifHeight}</p>
                            </div>
                          {/if}
                          {#if meta.gpsLatitude != null && meta.gpsLongitude != null}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">GPS</p>
                              <p class="text-quartz">{meta.gpsLatitude.toFixed(6)}, {meta.gpsLongitude.toFixed(6)}</p>
                            </div>
                          {/if}
                          {#if meta.artist}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Artist</p>
                              <p class="text-quartz">{meta.artist}</p>
                            </div>
                          {/if}
                          {#if meta.copyright}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider">Copyright</p>
                              <p class="text-quartz">{meta.copyright}</p>
                            </div>
                          {/if}
                          {#if meta.description}
                            <div class="col-span-2">
                              <p class="text-[10px] text-flint uppercase tracking-wider">Description</p>
                              <p class="text-quartz">{meta.description}</p>
                            </div>
                          {/if}
                        </div>
                      </details>
                    {/if}
                  </div>
                  {#if result.exifAnalysis}
                    <span class="text-sm font-medium tabular-nums flex-shrink-0 {forensicScoreClass(1 - result.exifAnalysis.trustScore)}">
                      {Math.round(result.exifAnalysis.trustScore * 100)}%
                    </span>
                  {/if}
                </div>
              </li>

              <!-- Content Credentials (C2PA) — 3-tier progressive disclosure
                   per C2PA UX Recommendations v1.4. Internal identifiers
                   (c2paManifest, c2paValid, card-provenance) are unchanged.
              -->
              <li
                id="section-c2pa"
                class="px-5 py-4 {result.c2paValid === false ? 'bg-cinnabar/[0.03]' : ''}"
                aria-label="Content Credentials"
              >

                <!-- ── L1: Seal + one-line summary (always visible) ── -->
                <div class="flex items-start gap-3">
                  <div class="mt-0.5 flex-shrink-0"
                       aria-label="{result.c2paValid === true ? 'Content Credentials valid' : result.c2paValid === false ? 'Content Credentials invalid' : 'No Content Credentials'}">
                    <ContentCredentialsSeal state={c2paSealState()} size="md" />
                  </div>

                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-1 flex-wrap">
                      <span class="text-sm font-medium text-quartz">Content Credentials</span>
                    </div>

                    <!-- L1 one-line summary -->
                    {#if result.c2paValid === true && result.c2paManifest}
                      <p class="text-xs text-malachite dark:text-malachite-light leading-relaxed">
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
                      {#if c2paValidAtSigning}
                        <p class="text-xs text-amber dark:text-amber-light mt-1 leading-relaxed">
                          Certificate expired; signature verified via trusted timestamp.
                        </p>
                      {/if}
                    {:else if result.c2paValid === false}
                      <p class="text-xs text-cinnabar dark:text-cinnabar-light leading-relaxed">
                        Content Credential unavailable or invalid — the record may have been altered.
                      </p>
                    {:else}
                      <p class="text-xs text-flint dark:text-flint-light leading-relaxed">
                        No Content Credentials attached. This is normal for most images.
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

                      <!-- ── L2: Issued by, Edits, Digital source type ── -->
                      {#if c2paShowL2}
                        <div id="c2pa-l2" class="mt-3 space-y-3 border-t border-border-dark/40 pt-3">

                          <!-- Issued by — mandatory per C2PA UX Rec v1.4 §4.2 -->
                          <div>
                            <p class="text-[10px] text-flint uppercase tracking-wider mb-0.5">Issued by</p>
                            <p class="text-xs text-quartz">
                              {c2paSignerName ?? 'Unknown'}
                            </p>
                            {#if result.c2paManifest.signedByIssuer}
                              <p class="text-[11px] text-flint dark:text-flint-light mt-0.5">
                                via {result.c2paManifest.signedByIssuer}
                              </p>
                            {/if}
                          </div>

                          <!-- Signed date -->
                          {#if c2paSignedDate()}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider mb-0.5">Date</p>
                              <p class="text-xs text-quartz">{c2paSignedDate()}</p>
                            </div>
                          {/if}

                          <!-- Edits and activity — per C2PA UX Rec v1.4 §4.3 -->
                          {#if c2paActions().length > 0}
                            <div>
                              <p class="text-[10px] text-flint uppercase tracking-wider mb-1.5">Edits and activity</p>
                              <ul class="space-y-2" aria-label="Recorded edits and activity">
                                {#each c2paActions() as action}
                                  <li class="flex items-start gap-2">
                                    <span class="inline-flex items-center px-2 py-0.5 rounded-full text-[11px] shrink-0
                                                 bg-graphite-light border border-border-dark text-quartz">
                                      {action.label}
                                    </span>
                                    <div class="text-[11px] leading-snug">
                                      {#if action.description}
                                        <p class="text-quartz">{action.description}</p>
                                      {/if}
                                      {#if action.sourceType}
                                        <p class="text-flint dark:text-flint-light">
                                          Source: {action.sourceType}
                                        </p>
                                      {/if}
                                      {#if action.softwareAgent}
                                        <p class="text-flint dark:text-flint-light">
                                          Tool: {action.softwareAgent}
                                        </p>
                                      {/if}
                                      {#if !action.description && !action.sourceType && !action.softwareAgent}
                                        <p class="text-flint/60 italic">No additional detail recorded</p>
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
                              <p class="text-[10px] text-flint uppercase tracking-wider mb-0.5">Digital source type</p>
                              <p class="text-xs text-quartz">{c2paDigitalSourceType()}</p>
                            </div>
                          {/if}

                          <!-- ── Provenance chain timeline (C2PA UX Rec v1.4 §5.4) ── -->
                          {#if result.c2paChain && result.c2paChain.ingredients.length > 0}
                            {@const chain = result.c2paChain}
                            {@const originManifest = chain.ingredients[chain.ingredients.length - 1]}
                            {@const middleIngredients = chain.ingredients.slice(0, -1)}
                            {@const shouldCollapse = chain.manifestCount >= 4}
                            <div class="mt-1 pt-3 border-t border-border-dark/40">
                              <p class="text-[10px] text-flint uppercase tracking-wider mb-2">Provenance chain</p>
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
                                  <p class="text-[10px] font-semibold text-flint uppercase tracking-wider leading-none mb-0.5">Active</p>
                                  <p class="text-xs text-quartz leading-snug">{chainSignerName(chain.active)}</p>
                                  {#if chainSignedDate(chain.active)}
                                    <p class="text-[11px] text-flint dark:text-flint-light">{chainSignedDate(chain.active)}</p>
                                  {/if}
                                  {#if chainActionSummary(chain.active)}
                                    <p class="text-[11px] text-flint dark:text-flint-light italic">{chainActionSummary(chain.active)}</p>
                                  {/if}
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
                                             bg-graphite-light dark:bg-obsidian border border-border-dark text-flint dark:text-flint-light
                                             hover:text-quartz hover:border-lapis/50 transition-colors
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
                                      <p class="text-xs text-quartz leading-snug">{chainSignerName(ingredient)}</p>
                                      {#if chainSignedDate(ingredient)}
                                        <p class="text-[11px] text-flint dark:text-flint-light">{chainSignedDate(ingredient)}</p>
                                      {/if}
                                      {#if chainActionSummary(ingredient)}
                                        <p class="text-[11px] text-flint dark:text-flint-light italic">{chainActionSummary(ingredient)}</p>
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
                                    class="absolute left-0 top-1.5 w-2.5 h-2.5 rounded-full border-2 border-malachite bg-obsidian dark:bg-obsidian"
                                    aria-hidden="true"
                                  ></span>
                                  <p class="text-[10px] font-semibold text-flint uppercase tracking-wider leading-none mb-0.5">Origin</p>
                                  <p class="text-xs text-quartz leading-snug">{chainSignerName(originManifest)}</p>
                                  {#if chainSignedDate(originManifest)}
                                    <p class="text-[11px] text-flint dark:text-flint-light">{chainSignedDate(originManifest)}</p>
                                  {/if}
                                  {#if chainActionSummary(originManifest)}
                                    <p class="text-[11px] text-flint dark:text-flint-light italic">{chainActionSummary(originManifest)}</p>
                                  {/if}
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
                            <div id="c2pa-l3" class="space-y-3 border-t border-border-dark/40 pt-3">

                              <!-- Manifest selector tabs — only shown when chain has multiple manifests -->
                              {#if hasChain}
                                <div role="tablist" aria-label="Manifest in chain" class="flex flex-wrap gap-1.5">
                                  {#each manifests as manifest, idx}
                                    {@const tabLabel = idx === 0 ? 'Active'
                                      : idx === manifests.length - 1 ? 'Origin'
                                      : `Intermediate ${idx}`}
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
                                               : 'bg-graphite-light/40 text-flint hover:bg-graphite-light/70 hover:text-quartz'}"
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
                                          <p class="text-[10px] text-flint uppercase tracking-wider mb-0.5">Title</p>
                                          <p class="text-xs text-quartz">{sm.title}</p>
                                        </div>
                                      {/if}
                                      {#if sm.format}
                                        <div>
                                          <p class="text-[10px] text-flint uppercase tracking-wider mb-0.5">Format</p>
                                          <p class="text-xs text-quartz font-mono">{sm.format}</p>
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
                                    {@const certExpired = checks.some(c => c.code === 'signingCredential.expired' && c.outcome === 'fail')}
                                    {@const certUntrusted = checks.some(c => c.code === 'signingCredential.untrusted' && c.outcome === 'fail')}
                                    {@const hashFail = checks.some(c => c.code.includes('dataHash.mismatch') && c.outcome === 'fail')}
                                    {@const passCount = checks.filter(c => c.outcome === 'pass').length}
                                    {@const failCount = checks.filter(c => c.outcome === 'fail').length}
                                    {@const infoCount = checks.filter(c => c.outcome === 'info').length}
                                    <div>
                                      <p class="text-[10px] text-flint uppercase tracking-wider mb-2">Validation summary</p>
                                      <div class="space-y-1.5">
                                        <div class="flex items-center gap-2 text-xs">
                                          <span class="w-4 text-center {sigValid ? 'text-malachite' : 'text-cinnabar'}">{sigValid ? '\u2713' : '\u2717'}</span>
                                          <span class="text-quartz">Claim signature {sigValid ? 'verified' : 'failed'}</span>
                                        </div>
                                        <div class="flex items-center gap-2 text-xs">
                                          <span class="w-4 text-center {dataValid ? 'text-malachite' : hashFail ? 'text-cinnabar' : 'text-flint'}">{dataValid ? '\u2713' : hashFail ? '\u2717' : '\u2014'}</span>
                                          <span class="text-quartz">{dataValid ? 'Data integrity confirmed — file has not been modified' : hashFail ? 'Data integrity failed — file has been modified since signing' : 'Data hash not checked'}</span>
                                        </div>
                                        {#if tsValid}
                                          <div class="flex items-center gap-2 text-xs">
                                            <span class="w-4 text-center text-malachite">{'\u2713'}</span>
                                            <span class="text-quartz">Timestamp verified{certExpired ? ' — signature was valid at signing time' : ''}</span>
                                          </div>
                                        {/if}
                                        {#if certExpired}
                                          <div class="flex items-center gap-2 text-xs">
                                            <span class="w-4 text-center text-amber">{'\u26A0'}</span>
                                            <span class="text-quartz">Signing certificate has expired{tsValid ? ' (mitigated by trusted timestamp)' : ''}</span>
                                          </div>
                                        {/if}
                                        {#if certUntrusted}
                                          <div class="flex items-center gap-2 text-xs">
                                            <span class="w-4 text-center text-flint">{'\u26A0'}</span>
                                            <span class="text-quartz">Signing certificate not on a public trust list</span>
                                          </div>
                                        {/if}
                                        <p class="text-[11px] text-flint mt-1">{passCount} passed, {failCount} failed, {infoCount} informational</p>
                                      </div>

                                      <!-- L4: Raw validation codes (collapsible) -->
                                      <details class="mt-2">
                                        <summary class="text-[11px] text-lapis cursor-pointer hover:text-lapis-light">
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
                                                <span class="text-[11px] font-mono text-flint dark:text-flint-light">{check.code}</span>
                                                {#if check.explanation}
                                                  <p class="text-[11px] text-quartz/70 mt-0.5">{check.explanation}</p>
                                                {/if}
                                              </div>
                                            </li>
                                          {/each}
                                        </ul>
                                      </details>
                                    </div>
                                  {/if}

                                  <!-- Assertions -->
                                  {#if sm && sm.assertions.length > 0}
                                    <div>
                                      <p class="text-[10px] text-flint uppercase tracking-wider mb-1.5">Assertions</p>
                                      <ul class="space-y-1" aria-label="Manifest assertions">
                                        {#each sm.assertions as assertion}
                                          <li class="text-[11px]">
                                            <span class="font-mono text-flint dark:text-flint-light">{assertion.label}</span>
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
        class="bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'integrity' ? 'border-lapis/30' : 'border-border-dark'}
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
            class="w-4 h-4 text-flint flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'integrity' ? 'rotate-90' : ''}"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true"
          ><path d="M9 5l7 7-7 7"/></svg>

          <span class="flex-1 font-serif text-base text-quartz">Is the content intact?</span>

          <span class="text-xs text-flint dark:text-flint-light mr-2">
            {(() => {
              const total = [result.elaResult, result.noiseResult, result.copyMoveResult, result.jpegGhostResult, result.segmentedElaResult, result.colourTemperatureResult].filter(Boolean).length;
              return `${total - integrityFindings} of ${total} passed`;
            })()}
          </span>

          <span class="text-xs font-semibold {cardPassClass(integrityPass())}">
            {integrityPass() === null ? '—' : integrityPass() ? 'Pass' : 'Concern'}
          </span>
        </button>

        {#if openCard === 'integrity'}
          <div id="card-integrity-body" class="border-t border-border-dark/60">
            <ul class="divide-y divide-border-dark/40" aria-label="Integrity checks">

              {#if result.elaResult}
                <li class="px-5 py-3 {result.elaResult.suspicious ? 'bg-amber/[0.04]' : ''}">
                  <div class="flex items-center gap-3">
                    <svg class="w-3.5 h-3.5 flex-shrink-0 {result.elaResult.suspicious ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      {#if result.elaResult.suspicious}
                        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                      {:else}
                        <polyline points="20 6 9 17 4 12"/>
                      {/if}
                    </svg>
                    <span class="text-sm {result.elaResult.suspicious ? 'text-amber-light font-medium' : 'text-quartz'} flex-1">Error Level Analysis</span>
                    <span class="text-xs tabular-nums {forensicScoreClass(result.elaResult.score)}">{Math.round(result.elaResult.score * 100)}%</span>
                    {#if result.elaResult.elaImageBase64}
                      <img src="data:image/png;base64,{result.elaResult.elaImageBase64}" alt="ELA heatmap" class="w-12 h-8 rounded object-cover border border-border-dark flex-shrink-0" />
                    {/if}
                  </div>
                </li>
              {/if}

              {#if result.noiseResult}
                <li class="px-5 py-3 {result.noiseResult.suspicious ? 'bg-amber/[0.04]' : ''}">
                  <div class="flex items-center gap-3">
                    <svg class="w-3.5 h-3.5 flex-shrink-0 {result.noiseResult.suspicious ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      {#if result.noiseResult.suspicious}
                        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                      {:else}
                        <polyline points="20 6 9 17 4 12"/>
                      {/if}
                    </svg>
                    <span class="text-sm {result.noiseResult.suspicious ? 'text-amber-light font-medium' : 'text-quartz'} flex-1">Noise Pattern Analysis</span>
                    <span class="text-xs tabular-nums {forensicScoreClass(result.noiseResult.score)}">{Math.round(result.noiseResult.score * 100)}%</span>
                  </div>
                  {#if result.noiseResult.suspicious}
                    <p class="text-xs text-flint dark:text-flint-light mt-1 ml-6">{result.noiseResult.anomalousBlocks} of {result.noiseResult.totalBlocks} blocks flagged</p>
                  {/if}
                </li>
              {/if}

              {#if result.copyMoveResult}
                <li class="px-5 py-3 {result.copyMoveResult.suspicious ? 'bg-amber/[0.04]' : ''}">
                  <div class="flex items-center gap-3">
                    <svg class="w-3.5 h-3.5 flex-shrink-0 {result.copyMoveResult.suspicious ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      {#if result.copyMoveResult.suspicious}
                        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                      {:else}
                        <polyline points="20 6 9 17 4 12"/>
                      {/if}
                    </svg>
                    <span class="text-sm {result.copyMoveResult.suspicious ? 'text-amber-light font-medium' : 'text-quartz'} flex-1">Copy-Move Detection</span>
                    <span class="text-xs tabular-nums {forensicScoreClass(result.copyMoveResult.score)}">{Math.round(result.copyMoveResult.score * 100)}%</span>
                  </div>
                  {#if result.copyMoveResult.suspicious && result.copyMoveResult.cloneRegions.length > 0}
                    <p class="text-xs text-flint dark:text-flint-light mt-1 ml-6">{result.copyMoveResult.cloneRegions.length} cloned region{result.copyMoveResult.cloneRegions.length === 1 ? '' : 's'} detected</p>
                  {/if}
                </li>
              {/if}

              {#if result.jpegGhostResult}
                <li class="px-5 py-3 {result.jpegGhostResult.suspicious ? 'bg-amber/[0.04]' : ''}">
                  <div class="flex items-center gap-3">
                    <svg class="w-3.5 h-3.5 flex-shrink-0 {result.jpegGhostResult.suspicious ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      {#if result.jpegGhostResult.suspicious}<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                      {:else}<polyline points="20 6 9 17 4 12"/>{/if}
                    </svg>
                    <span class="text-sm {result.jpegGhostResult.suspicious ? 'text-amber-light font-medium' : 'text-quartz'} flex-1">
                      JPEG Ghost
                      <ExperimentalPill variant="uncalibrated" tooltip="JPEG Ghost is weighted at 0.5× in the trust score. See methodology." />
                    </span>
                    <span class="text-xs tabular-nums {forensicScoreClass(result.jpegGhostResult.score)}">{Math.round(result.jpegGhostResult.score * 100)}%</span>
                  </div>
                </li>
              {/if}

              {#if result.segmentedElaResult}
                <li class="px-5 py-3 {result.segmentedElaResult.suspicious ? 'bg-amber/[0.04]' : ''}">
                  <div class="flex items-center gap-3">
                    <svg class="w-3.5 h-3.5 flex-shrink-0 {result.segmentedElaResult.suspicious ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      {#if result.segmentedElaResult.suspicious}<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                      {:else}<polyline points="20 6 9 17 4 12"/>{/if}
                    </svg>
                    <span class="text-sm {result.segmentedElaResult.suspicious ? 'text-amber-light font-medium' : 'text-quartz'} flex-1">Segmented ELA</span>
                    <span class="text-xs tabular-nums {forensicScoreClass(result.segmentedElaResult.score)}">{Math.round(result.segmentedElaResult.score * 100)}%</span>
                  </div>
                  {#if result.segmentedElaResult.suspicious}
                    <p class="text-xs text-flint dark:text-flint-light mt-1 ml-6">{result.segmentedElaResult.anomalousRegions} of {result.segmentedElaResult.totalRegions} regions flagged</p>
                  {/if}
                </li>
              {/if}

              {#if result.colourTemperatureResult}
                <li class="px-5 py-3 {result.colourTemperatureResult.suspicious ? 'bg-amber/[0.04]' : ''}">
                  <div class="flex items-center gap-3">
                    <svg class="w-3.5 h-3.5 flex-shrink-0 {result.colourTemperatureResult.suspicious ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      {#if result.colourTemperatureResult.suspicious}<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                      {:else}<polyline points="20 6 9 17 4 12"/>{/if}
                    </svg>
                    <span class="text-sm {result.colourTemperatureResult.suspicious ? 'text-amber-light font-medium' : 'text-quartz'} flex-1">Colour Temperature</span>
                    <span class="text-xs tabular-nums {forensicScoreClass(result.colourTemperatureResult.score)}">{Math.round(result.colourTemperatureResult.score * 100)}%</span>
                  </div>
                </li>
              {/if}

              {#if !result.elaResult && !result.noiseResult && !result.copyMoveResult}
                <li class="px-5 py-4">
                  <p class="text-sm text-flint dark:text-flint-light">
                    Integrity checks require the Analysis Engine. {sidecarAvailable ? 'No data returned for this file type.' : 'Start the sidecar to enable forensic analysis.'}
                    <a href="/settings" class="text-lapis-light underline hover:text-quartz ml-1">Check service status</a>
                  </p>
                </li>
              {/if}

            </ul>

            <!-- On-demand tools -->
            <div class="px-5 py-3 border-t border-border-dark/40 bg-white/[0.01] flex items-center gap-3 flex-wrap">
              <span class="text-[10px] text-flint uppercase tracking-wider font-semibold">On-demand tools</span>
              <span class="text-xs text-flint dark:text-flint-light">
                NPR, Shadow Consistency, and Splice Boundary require deep or archival mode.
                <a href="/verify" class="text-lapis-light underline hover:text-quartz">Run in classic view</a>
              </span>
            </div>

            {#if result.inputQuality}
              <LimitationBanner quality={result.inputQuality} />
            {/if}
          </div>
        {/if}
      </div>

      <!-- Card 3: Is this AI-generated? -->
      <div
        id="card-ai"
        class="bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'ai' ? 'border-lapis/30' : 'border-border-dark'}
               focus-within:border-lapis/25"
      >
        <button
          class="w-full flex items-center gap-3 px-5 py-4 text-left min-h-[56px] hover:bg-white/[0.02] transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis-light rounded-xl"
          aria-expanded={openCard === 'ai'}
          aria-controls="card-ai-body"
          onclick={() => openCard = openCard === 'ai' ? null : 'ai'}
        >
          <svg
            class="w-4 h-4 text-flint flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'ai' ? 'rotate-90' : ''}"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true"
          ><path d="M9 5l7 7-7 7"/></svg>

          <span class="flex-1 font-serif text-base text-quartz">Is this AI-generated?</span>

          {#if aiDetectionSuppressed}
            <span class="text-xs text-amber dark:text-amber-light mr-2">Suppressed</span>
          {:else}
            <span class="text-xs text-flint dark:text-flint-light mr-2">
              {(() => {
                const total = [result.deepfakeResult, result.clipResult].filter(Boolean).length;
                return `${total - aiFindings} of ${total} passed`;
              })()}
            </span>
            <span class="text-xs font-semibold {cardPassClass(aiPass())}">
              {aiPass() === null ? '—' : aiPass() ? 'Pass' : 'Concern'}
            </span>
          {/if}
        </button>

        {#if openCard === 'ai'}
          <div id="card-ai-body" class="border-t border-border-dark/60">
            {#if aiDetectionSuppressed}
              <div class="px-5 py-4">
                <p class="text-sm text-amber/80 dark:text-amber-light/80">
                  AI detection signals have been suppressed for this file. The content-type classifier determined that deepfake and CLIP models are not reliable for this content type.
                  Scores were neutralised in the trust calculation.
                </p>
                <p class="text-xs text-flint dark:text-flint-light mt-2">
                  <a href="/verify" class="text-lapis-light underline hover:text-quartz">View raw scores in classic view</a>
                </p>
              </div>
            {:else}
              <ul class="divide-y divide-border-dark/40" aria-label="AI detection checks">

                {#if result.deepfakeResult}
                  <li class="px-5 py-4 {result.deepfakeResult.suspicious ? 'bg-amber/[0.04]' : ''}">
                    <div class="flex items-start gap-3">
                      <svg class="w-4 h-4 mt-0.5 flex-shrink-0 {result.deepfakeResult.suspicious ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        {#if result.deepfakeResult.suspicious}<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                        {:else}<polyline points="20 6 9 17 4 12"/>{/if}
                      </svg>
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2 mb-1">
                          <span class="text-sm font-medium {result.deepfakeResult.suspicious ? 'text-amber-light' : 'text-quartz'}">AI Generation (GBM Deepfake)</span>
                          <span class="text-xs px-1.5 py-0.5 rounded bg-graphite-light border border-border-dark text-flint dark:text-flint-light">{result.deepfakeResult.confidence} confidence</span>
                        </div>
                        <p class="text-xs text-flint dark:text-flint-light">{result.deepfakeResult.summary}</p>
                        {#if result.deepfakeResult.verdictLevel}
                          <p class="text-xs mt-1 font-medium {result.deepfakeResult.verdictLevel === 'synthetic' ? 'text-cinnabar dark:text-cinnabar-light' : result.deepfakeResult.verdictLevel === 'inconclusive' ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}">
                            Verdict: {result.deepfakeResult.verdictLevel.charAt(0).toUpperCase() + result.deepfakeResult.verdictLevel.slice(1)}
                          </p>
                        {/if}
                      </div>
                      <span class="text-sm font-medium tabular-nums flex-shrink-0 {forensicScoreClass(result.deepfakeResult.score)}">{Math.round(result.deepfakeResult.score * 100)}%</span>
                    </div>
                  </li>
                {/if}

                {#if result.clipResult}
                  <li class="px-5 py-4 {result.clipResult.verdictLevel === 'synthetic' ? 'bg-amber/[0.04]' : ''}">
                    <div class="flex items-start gap-3">
                      <svg class="w-4 h-4 mt-0.5 flex-shrink-0 {result.clipResult.verdictLevel === 'synthetic' ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        {#if result.clipResult.verdictLevel === 'synthetic'}<path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                        {:else}<polyline points="20 6 9 17 4 12"/>{/if}
                      </svg>
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2 mb-1">
                          <span class="text-sm font-medium {result.clipResult.verdictLevel === 'synthetic' ? 'text-amber-light' : 'text-quartz'}">CLIP / UnivFD Probe</span>
                          <ExperimentalPill variant="uncalibrated" tooltip="CLIP probe AUC 0.9933. See methodology for limitations." />
                        </div>
                        <p class="text-xs text-flint dark:text-flint-light">{result.clipResult.summary}</p>
                      </div>
                      <span class="text-sm font-medium tabular-nums flex-shrink-0 {forensicScoreClass(result.clipResult.score)}">{Math.round(result.clipResult.score * 100)}%</span>
                    </div>
                  </li>
                {/if}

                {#if result.watermarkExtractResult}
                  <li class="px-5 py-3">
                    <div class="flex items-center gap-3">
                      <svg class="w-3.5 h-3.5 flex-shrink-0 text-malachite dark:text-malachite-light" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <polyline points="20 6 9 17 4 12"/>
                      </svg>
                      <span class="text-sm text-quartz flex-1">Watermark Detection</span>
                      <span class="text-xs text-flint dark:text-flint-light">
                        {result.watermarkExtractResult.hasWatermark ? 'Watermark found' : 'No watermark detected'}
                      </span>
                    </div>
                  </li>
                {/if}

                {#if !result.deepfakeResult && !result.clipResult}
                  <li class="px-5 py-4">
                    <p class="text-sm text-flint dark:text-flint-light">
                      AI detection requires the Analysis Engine.
                      {#if !sidecarAvailable}<a href="/settings" class="text-lapis-light underline hover:text-quartz">Check service status</a>{/if}
                    </p>
                  </li>
                {/if}

              </ul>
            {/if}

            <!-- Video deepfake placeholder -->
            {#if result.videoDeepfakeResult}
              <div class="px-5 py-3 border-t border-border-dark/40 bg-white/[0.01]">
                <p class="text-xs text-flint dark:text-flint-light">
                  Video deepfake analysis available.
                  <a href="/verify" class="text-lapis-light underline hover:text-quartz ml-1">View per-frame detail in classic view</a>
                </p>
              </div>
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
        class="bg-graphite border rounded-xl overflow-hidden transition-colors
               {openCard === 'claims' ? 'border-lapis/30' : 'border-border-dark'}
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
            class="w-4 h-4 text-flint flex-shrink-0 motion-safe:transition-transform motion-safe:duration-200 {openCard === 'claims' ? 'rotate-90' : ''}"
            viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true"
          ><path d="M9 5l7 7-7 7"/></svg>

          <span class="flex-1 font-serif text-base text-quartz">What does it claim?</span>

          <span class="text-xs text-flint dark:text-flint-light mr-2">
            {hasClaimsData ? 'Audio / video only' : 'Not applicable'}
          </span>
        </button>

        {#if openCard === 'claims'}
          <div id="card-claims-body" class="border-t border-border-dark/60 px-5 py-4">
            {#if result.transcriptionResult}
              <div class="mb-4">
                <h3 class="text-xs text-flint uppercase tracking-wider mb-2">Transcription</h3>
                <p class="text-sm text-quartz leading-relaxed">{result.transcriptionResult.text}</p>
              </div>
            {/if}

            {#if result.claimCheckResult}
              <div>
                <h3 class="text-xs text-flint uppercase tracking-wider mb-2">Claim Check</h3>
                <p class="text-sm text-quartz">{result.claimCheckResult.overallVerdict}</p>
                {#if result.claimCheckResult.summary}
                  <p class="text-xs text-flint dark:text-flint-light mt-1">{result.claimCheckResult.summary}</p>
                {/if}
              </div>
            {/if}

            {#if result.ragClaimResult}
              <div>
                <h3 class="text-xs text-flint uppercase tracking-wider mb-2">RAG Claim Verification</h3>
                <p class="text-sm text-quartz leading-relaxed">{result.ragClaimResult.explanation}</p>
              </div>
            {/if}

            {#if !hasClaimsData}
              <p class="text-sm text-flint dark:text-flint-light">
                Transcription and claim checking are only applicable to audio and video files.
                For images, use the EXIF and Content Credentials sections above to assess provenance claims.
              </p>
            {/if}
          </div>
        {/if}
      </div>
    </section>

    <!-- ── Actions footer ──────────────────────────────────────────── -->
    <div class="flex items-center gap-3 flex-wrap bg-graphite border border-border-dark rounded-xl px-5 py-4">
      <button
        class="flex items-center gap-2 px-4 py-2.5 min-h-[44px] bg-lapis text-quartz text-sm font-medium rounded-lg hover:bg-lapis-dark transition-colors disabled:opacity-50
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
        class="flex items-center gap-2 px-4 py-2.5 min-h-[44px] border border-border-dark text-quartz text-sm font-medium rounded-lg hover:bg-white/5 transition-colors disabled:opacity-50
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
        class="flex items-center gap-2 px-4 py-2.5 min-h-[44px] border border-border-dark text-flint dark:text-flint-light text-sm rounded-lg hover:text-quartz hover:bg-white/5 transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light"
        onclick={() => showFalsePositiveModal = true}
        aria-label="Report a false positive result"
      >
        Report False Positive
      </button>

      <a
        href="/verify"
        class="ml-auto text-xs text-flint dark:text-flint-light hover:text-quartz transition-colors underline underline-offset-2 flex items-center gap-1 min-h-[44px] px-2
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis-light rounded"
      >
        View full detail in classic view
        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
      </a>
    </div>

    <!-- Deferred features note -->
    <p class="mt-4 text-xs text-flint/60 dark:text-flint-light/60 text-center">
      Region-of-interest analysis, per-frame video deepfake detail, sun position estimation, and annotation tools are available in the
      <a href="/verify" class="underline hover:text-flint dark:hover:text-flint-light">classic view</a>.
    </p>

  {/if}<!-- end #if checked && result -->

</main>
