<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { writable } from 'svelte/store';
  import { verifyFile, verifyUrl, checkSidecarHealth, openBatchFileDialog, markFalsePositive, parseAppError, getLicenceTier } from '$lib/api';
  import { getTrustLevel, SEVERITY_CONFIG, formatFileSize, formatDuration } from '$lib/types';
  import { createBlobTracker } from '$lib/blob';
  import type { LicenceTier, VerificationResult, AnomalyFinding, SidecarHealth, VerifyMode, BatchItem, SegmentedElaResult, ShadowConsistencyResult, ColourTemperatureResult, SpliceBoundaryResult, ClipDetectionResult, RagClaimResult, VideoDeepfakeResult, FrameDeepfakeResult, TranscriptionResult, ClaimCheckResult } from '$lib/types';
  import VerdictSummary from '$lib/components/VerdictSummary.svelte';
  import MethodologyPanel from '$lib/components/MethodologyPanel.svelte';
  import InspectionChecklist from '$lib/components/InspectionChecklist.svelte';
  import SignalAgreement from '$lib/components/SignalAgreement.svelte';
  import ContextualHelpLink from '$lib/components/ContextualHelpLink.svelte';
  import { generateTrustReport } from '$lib/pdf';
  import type { ReportContext } from '$lib/pdf';
  import { exportCaseZip } from '$lib/zip';
  import { getVersion } from '$lib/api';

  // ── State ──────────────────────────────────────────────────────────
  let activeTab = $state<'file' | 'batch' | 'url'>('file');
  let filePath = $state<string | null>(null);
  let fileName = $state<string | null>(null);
  let urlInput = $state('');
  let result = $state<VerificationResult | null>(null);
  let checked = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  /** Machine-readable error category derived from AppError.code or legacy string-sniffing. */
  let errorType = $state<'sidecar' | 'format' | 'network' | 'general' | null>(null);

  /**
   * Classify a caught error from a Tauri command into one of four categories
   * and set the `error` / `errorType` reactive state.
   *
   * Handles both structured AppError responses (`{ code, message }`) from
   * commands that have been migrated to AppError, and plain strings from
   * commands still using `map_err(|e| e.to_string())`.
   */
  function setError(e: unknown, context: string) {
    const { code, message } = parseAppError(e);

    // Determine category from structured code first (most reliable)
    if (code !== null) {
      switch (code) {
        case 'Sidecar':
          errorType = 'sidecar';
          error = import.meta.env.DEV
            ? message
            : 'The Analysis Engine is not currently running. Core features (C2PA verification, EXIF analysis) are still available.\n\nFor full forensic analysis including AI detection, noise analysis, and copy-move detection, restart Jura Trace or check Settings \u2192 Service Status.';
          break;
        case 'Validation':
          errorType = 'format';
          error = message; // already user-safe from the backend
          break;
        case 'FileSystem':
          errorType = 'general';
          error = 'The file could not be read. Check it is not open in another application and try again.';
          break;
        case 'Database':
          errorType = 'general';
          error = 'A database error occurred. Your work has been saved. Restart Jura Trace if the problem persists.';
          break;
        case 'C2pa':
          errorType = 'general';
          error = 'The content credential operation could not be completed. The file has not been modified.';
          break;
        case 'Internal':
          errorType = 'general';
          error = 'An unexpected error occurred. Please restart Jura Trace.';
          break;
        default:
          errorType = 'general';
          error = message;
      }
      return;
    }

    // Fallback: string-sniff for plain string errors from unmigrated commands
    const lower = message.toLowerCase();
    if (lower.includes('tauri not available') || lower.includes('invoke')) {
      error = `${context}: application bridge unavailable`;
      errorType = 'general';
    } else if (lower.includes('unsupported') || lower.includes('format') || lower.includes('mime')) {
      error = `Unsupported file format. Jura Trace supports JPEG, PNG, TIFF, WebP, PDF, MP4, MOV, WAV, and MP3.`;
      errorType = 'format';
    } else if (lower.includes('sidecar') || lower.includes('connection refused') || lower.includes('127.0.0.1:8200')) {
      error = import.meta.env.DEV
        ? `Analysis services are not running. Start the sidecar with: uvicorn main:app --host 127.0.0.1 --port 8200`
        : `The Analysis Engine is not currently running. Core features (C2PA verification, EXIF analysis) are still available.\n\nFor full forensic analysis including AI detection, noise analysis, and copy-move detection, restart Jura Trace or check Settings \u2192 Service Status.`;
      errorType = 'sidecar';
    } else if (lower.includes('fetch') || lower.includes('network') || lower.includes('ssrf') || lower.includes('url')) {
      error = `Could not fetch the URL. Check the address is correct and publicly accessible.`;
      errorType = 'network';
    } else {
      error = `${context}: ${message}`;
      errorType = 'general';
    }
  }
  let dragOver = $state(false);
  let sidecarHealth = $state<SidecarHealth | null>(null);
  let verifyMode = $state<VerifyMode>('standard');

  // ── Video analysis progress state ─────────────────────────────────
  /** Human-readable phase description shown beneath the spinner for video files. */
  let analysisPhase = $state<string | null>(null);
  /** Set to true when the user cancels mid-analysis; causes the result to be discarded. */
  let cancelled = $state(false);

  /** Estimated analysis duration label based on the current verify mode. */
  const estimatedTime = $derived(
    verifyMode === 'archival' ? 'Estimated time: ~90 seconds'
    : verifyMode === 'deep' ? 'Estimated time: ~45 seconds'
    : 'Estimated time: ~15 seconds'
  );
  let showTechnicalDetails = $state(false);
  let showInvestigatePanel = $state(false);
  let showSignalAgreement = $state(false);
  let showInspectionChecklist = $state(false);
  let showRegionAnalysis = $state(false);
  let expandedFrameIndex = $state<number | null>(null);

  // ── Summary / Detail view mode ────────────────────────────────────
  let viewMode = $state<'summary' | 'detail'>('summary');

  // Blob URL tracker — converts base64 data to CSP-safe blob: URLs and
  // revokes them on component destroy to prevent memory leaks.
  const blobs = createBlobTracker();
  onDestroy(() => blobs.revokeAll());

  // ── Test hook: allow Playwright to inject a mock result ──────────
  // Writable store bridges external Playwright calls into Svelte 5
  // reactivity. The $-prefixed store reference in $effect creates
  // a proper reactive subscription.
  const _testResultStore = writable<VerificationResult | null>(null);
  if (typeof window !== 'undefined' && import.meta.env.DEV) {
    (window as any).__juraSetVerifyResult = (data: VerificationResult) => {
      _testResultStore.set(data);
    };
    (window as any).__juraSetVerifyError = (msg: string) => {
      setError(msg, 'Verification');
    };
  }
  // Bridge store into $state via $effect. The _testApplied guard
  // prevents the infinite loop that occurs because $effect tracks
  // result reads elsewhere in the template.
  let _testApplied = false;
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

  // ── Export state ─────────────────────────────────────────────────
  let showReportModal = $state(false);
  let analystNote = $state('');
  // Analyst declaration fields — name and organisation persist across sessions
  let analystName = $state('');
  let analystOrg = $state('');
  let analystCaseRef = $state('');
  let analystDate = $state('');
  let exportingReport = $state(false);
  let exportingCase = $state(false);
  let appVersion = $state('0.2.0-dev');
  let licenceTier = $state<LicenceTier>('community');

  // Focus the first input when the report modal opens (WCAG 2.4.3 Focus Order)
  $effect(() => {
    if (showReportModal) {
      // Defer to next microtask so the DOM has been painted
      Promise.resolve().then(() => {
        const el = document.getElementById('decl-analyst-name');
        if (el) (el as HTMLElement).focus();
      });
    }
  });

  // ── False positive state ──────────────────────────────────────────
  let showFalsePositiveModal = $state(false);
  let fpReasonCode = $state('modern_codec');
  let fpReasonNote = $state('');
  let fpSubmitting = $state(false);
  let fpSubmitted = $state(false);

  // ── Platform detection ──────────────────────────────────────────
  const isMac = typeof navigator !== 'undefined' && navigator.platform.startsWith('Mac');
  const modKey = isMac ? 'Cmd' : 'Ctrl';

  // ── Batch state ────────────────────────────────────────────────────
  let batchItems = $state<BatchItem[]>([]);
  let batchRunning = $state(false);
  let batchDragOver = $state(false);
  let expandedBatchId = $state<string | null>(null);

  // ── Derived ────────────────────────────────────────────────────────
  const rawTrustLevel = $derived(result ? getTrustLevel(result.overallTrust) : null);

  /** Whether the deepfake detector returned an inconclusive or synthetic verdict */
  const hasUncertainDeepfake = $derived(
    result?.deepfakeResult?.verdictLevel === 'inconclusive' ||
    result?.deepfakeResult?.verdictLevel === 'synthetic'
  );

  /**
   * Effective trust level — forced to 'medium' or 'low' when the deepfake
   * verdict is inconclusive/synthetic, regardless of the numeric score.
   * This prevents "High Trust" in green appearing alongside an amber
   * "Inconclusive" verdict — a dangerously contradictory display.
   */
  const trustLevel = $derived(() => {
    if (!rawTrustLevel) return null;
    if (hasUncertainDeepfake && rawTrustLevel === 'high') return 'medium';
    return rawTrustLevel;
  });

  const trustScorePercent = $derived(
    result ? Math.round(result.overallTrust * 100) : 0
  );

  const trustTextClass = $derived(() => {
    const level = trustLevel();
    if (!level) return 'text-flint dark:text-flint-light';
    if (level === 'high') return 'text-malachite';
    if (level === 'medium') return 'text-amber';
    return 'text-cinnabar';
  });

  const trustLabelText = $derived(() => {
    const level = trustLevel();
    if (hasUncertainDeepfake) {
      if (result?.deepfakeResult?.verdictLevel === 'synthetic') return 'Low Trust';
      return 'Uncertain';
    }
    if (level === 'high') return 'High Trust';
    if (level === 'medium') return 'Moderate Trust';
    if (level === 'low') return 'Low Trust';
    return '';
  });

  const sidecarAvailable = $derived(sidecarHealth?.status === 'ok');

  // Degraded = sidecar is connected but some optional capabilities are missing
  const sidecarDegraded = $derived(() => {
    if (!sidecarHealth?.capabilities) return false;
    const c = sidecarHealth.capabilities;
    return sidecarAvailable && (!c.videoMetadata || !c.transcription);
  });

  const sidecarDegradedHint = $derived(() => {
    if (!sidecarHealth?.capabilities) return '';
    const missing: string[] = [];
    const c = sidecarHealth.capabilities;
    if (!c.videoMetadata) missing.push('FFmpeg');
    if (!c.transcription) missing.push('Whisper');
    if (!c.clipDetect) missing.push('CLIP');
    if (!c.rag) missing.push('Ollama');
    return missing.length > 0 ? `Missing: ${missing.join(', ')}` : '';
  });

  // ── Lifecycle ─────────────────────────────────────────────────────
  // Persist view mode preference
  $effect(() => {
    localStorage.setItem('jura-verify-view-mode', viewMode);
  });

  onMount(() => {
    // Restore persisted view mode (summary/detail)
    const savedViewMode = localStorage.getItem('jura-verify-view-mode');
    if (savedViewMode === 'detail') viewMode = 'detail';

    // Restore persisted investigation mode
    const savedMode = localStorage.getItem('jura-verify-mode');
    if (savedMode === 'standard' || savedMode === 'deep' || savedMode === 'archival') {
      verifyMode = savedMode;
    }

    // Restore persisted analyst declaration fields
    analystName = localStorage.getItem('jura-analyst-name') ?? '';
    analystOrg = localStorage.getItem('jura-analyst-org') ?? '';
    // Populate analysis date with today — user may edit
    analystDate = new Date().toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    });

    // Async init — fire-and-forget; cleanup is returned synchronously below
    (async () => {
      sidecarHealth = await checkSidecarHealth();
      appVersion = await getVersion();
      licenceTier = await getLicenceTier();
    })();

    // Keyboard shortcuts
    function handleKey(e: KeyboardEvent) {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;
      // Don't fire when typing in inputs
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return;

      switch (e.key) {
        case 'o':
          if (e.shiftKey) {
            e.preventDefault();
            activeTab = 'batch';
            handleBatchBrowse();
          } else {
            e.preventDefault();
            activeTab = 'file';
            handleFileClick();
          }
          break;
        case 'Enter':
          e.preventDefault();
          if (activeTab === 'url' && urlInput.trim() && !loading) runUrlVerification();
          break;
        case 'e':
          if (result) {
            e.preventDefault();
            if (e.shiftKey) {
              handleExportCase();
            } else {
              showReportModal = true;
            }
          }
          break;
        case '1':
          e.preventDefault();
          activeTab = 'file';
          break;
        case '2':
          e.preventDefault();
          activeTab = 'batch';
          break;
        case '3':
          e.preventDefault();
          activeTab = 'url';
          break;
      }
    }

    function handleEsc(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        if (showFalsePositiveModal) {
          showFalsePositiveModal = false;
        } else if (showReportModal) {
          showReportModal = false;
        } else if (loading) {
          cancelAnalysis();
        } else if (result) {
          reset();
        }
      }
    }

    window.addEventListener('keydown', handleKey);
    window.addEventListener('keydown', handleEsc);
    return () => {
      window.removeEventListener('keydown', handleKey);
      window.removeEventListener('keydown', handleEsc);
    };
  });

  // ── Drag and drop ─────────────────────────────────────────────────
  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    dragOver = true;
  }

  function handleDragLeave() {
    dragOver = false;
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;

    const files = e.dataTransfer?.files;
    if (!files?.length) return;

    const file = files[0];
    const path = (file as any).path || file.name;
    await runFileVerification(path, file.name);
  }

  // ── File dialog ──────────────────────────────────────────────────
  async function handleFileClick() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        title: 'Select File to Verify',
        filters: [
          {
            name: 'Supported Files',
            extensions: [
              'jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'avif',
              'heic', 'heif', 'pdf', 'docx', 'mp4', 'mov', 'webm',
            ],
          },
          { name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'avif', 'heic', 'heif'] },
          { name: 'Documents', extensions: ['pdf', 'docx'] },
          { name: 'Video', extensions: ['mp4', 'mov', 'webm'] },
        ],
      });
      if (selected && typeof selected === 'string') {
        const name = selected.split('/').pop() || selected.split('\\').pop() || selected;
        await runFileVerification(selected, name);
      }
    } catch {
      // Browser fallback — Tauri not available
    }
  }

  // ── Core verification ─────────────────────────────────────────────

  /** Returns true if the given filename has a video file extension. */
  function isVideoFileName(name: string): boolean {
    return /\.(mp4|mov|avi|mkv|webm)$/i.test(name);
  }

  function cancelAnalysis() {
    cancelled = true;
    loading = false;
    analysisPhase = null;
    error = 'Analysis cancelled.';
    errorType = 'general';
  }

  async function runFileVerification(path: string, name: string) {
    filePath = path;
    fileName = name;
    result = null;
    checked = false;
    error = null;
    errorType = null;
    cancelled = false;
    loading = true;

    const isVideo = isVideoFileName(name);

    if (isVideo) {
      analysisPhase = 'Extracting video frames...';
      // After a short delay, move to the deepfake detection phase message.
      // The timer is intentionally fire-and-forget; we guard with `loading` so
      // the message does not appear after a fast completion or cancellation.
      const phaseTimer = setTimeout(() => {
        if (loading && !cancelled) {
          analysisPhase = 'Running deepfake detection...';
        }
      }, 3000);

      try {
        result = await verifyFile(path, verifyMode);
        clearTimeout(phaseTimer);
        if (!cancelled) {
          checked = true;
        }
      } catch (e) {
        clearTimeout(phaseTimer);
        if (!cancelled) {
          setError(e, 'File verification failed');
        }
      } finally {
        if (!cancelled) {
          loading = false;
        }
        analysisPhase = null;
      }
    } else {
      try {
        result = await verifyFile(path, verifyMode);
        if (!cancelled) {
          checked = true;
        }
      } catch (e) {
        if (!cancelled) {
          setError(e, 'File verification failed');
        }
      } finally {
        if (!cancelled) {
          loading = false;
        }
      }
    }
  }

  async function runUrlVerification() {
    const url = urlInput.trim();
    if (!url) return;

    fileName = url.split('/').pop()?.split('?')[0] || url;
    filePath = null;
    result = null;
    checked = false;
    error = null;
    errorType = null;
    loading = true;

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
    filePath = null;
    fileName = null;
    urlInput = '';
    result = null;
    checked = false;
    error = null;
    loading = false;
    analysisPhase = null;
    cancelled = false;
    showInvestigatePanel = false;
    showSignalAgreement = false;
    showInspectionChecklist = false;
    showRegionAnalysis = false;
  }

  // ── Export helpers ────────────────────────────────────────────────
  function triggerDownload(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function handleExportReport() {
    if (!result || exportingReport) return;
    exportingReport = true;
    try {
      // Persist analyst name and organisation for future sessions
      if (analystName.trim()) {
        localStorage.setItem('jura-analyst-name', analystName.trim());
      } else {
        localStorage.removeItem('jura-analyst-name');
      }
      if (analystOrg.trim()) {
        localStorage.setItem('jura-analyst-org', analystOrg.trim());
      } else {
        localStorage.removeItem('jura-analyst-org');
      }

      const ctx: ReportContext = {
        analystName: analystName.trim() || undefined,
        organisation: analystOrg.trim() || undefined,
        caseReference: analystCaseRef.trim() || undefined,
        analysisDate: analystDate.trim() || undefined,
      };

      const blob = generateTrustReport(
        result,
        {
          fileName: fileName ?? 'Unknown',
          fileSize: 0,
          analysedAt: new Date().toISOString(),
          analystNote: analystNote.trim() || undefined,
          appVersion,
        },
        ctx,
      );
      const ts = Math.floor(Date.now() / 1000);
      const safeName = (fileName ?? 'file').replace(/[^a-zA-Z0-9._-]/g, '_');
      triggerDownload(blob, `jura-report-${safeName}-${ts}.pdf`);
    } finally {
      exportingReport = false;
      showReportModal = false;
      analystNote = '';
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
      const safeName = (fileName ?? 'file').replace(/[^a-zA-Z0-9._-]/g, '_');
      triggerDownload(blob, `jura-case-${safeName}-${ts}.zip`);
    } finally {
      exportingCase = false;
    }
  }

  // ── False positive submit ─────────────────────────────────────────
  async function handleFalsePositiveSubmit() {
    if (!result || fpSubmitting) return;
    fpSubmitting = true;

    const signalScores = result.deepfakeResult?.signals
      ? JSON.stringify(
          result.deepfakeResult.signals.map(s => ({
            name: s.name,
            weight: s.weight,
            triggered: s.triggered,
          }))
        )
      : undefined;

    try {
      await markFalsePositive(
        fpReasonCode,
        fpReasonNote.trim() || undefined,
        result.contentType,
        result.deepfakeResult?.score,
        result.deepfakeResult?.verdictLevel,
        signalScores,
      );
      fpSubmitted = true;
      // Auto-close after 2 seconds
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

  // ── Batch helpers ──────────────────────────────────────────────────
  const batchCompleted = $derived(batchItems.filter(i => i.status === 'done' || i.status === 'error').length);
  const batchQueued = $derived(batchItems.filter(i => i.status === 'queued').length);

  function addBatchFiles(files: { filePath: string; fileName: string }[]) {
    const newItems: BatchItem[] = files.map(f => ({
      id: crypto.randomUUID(),
      filePath: f.filePath,
      fileName: f.fileName,
      status: 'queued' as const,
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
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (!files.length) return;
    addBatchFiles(files.map(f => ({
      filePath: (f as any).path || f.name,
      fileName: f.name,
    })));
  }

  async function handleBatchBrowse() {
    const files = await openBatchFileDialog();
    if (files.length) addBatchFiles(files);
  }

  async function runBatch() {
    if (batchRunning) return;
    batchRunning = true;

    const CONCURRENCY = 3;

    async function processItem(item: BatchItem) {
      // Mutate the reactive item in place so the table updates immediately
      item.status = 'running';
      item.startedAt = Date.now();
      try {
        const res = await verifyFile(item.filePath, verifyMode);
        item.result = res;
        item.status = 'done';
      } catch (e) {
        item.error = e instanceof Error ? e.message : String(e);
        item.status = 'error';
      }
      item.finishedAt = Date.now();
    }

    const queued = batchItems.filter(i => i.status === 'queued');
    for (let i = 0; i < queued.length; i += CONCURRENCY) {
      const chunk = queued.slice(i, i + CONCURRENCY);
      await Promise.all(chunk.map(processItem));
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

  // ── Helpers ──────────────────────────────────────────────────────
  function formatSignedAt(raw: string): string {
    try {
      return new Date(raw).toLocaleString('en-GB', {
        day: 'numeric',
        month: 'short',
        year: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return raw;
    }
  }

  // ── Reverse image search URLs ─────────────────────────────────────
  //
  // For URL-sourced verifications, we can pass the URL directly to search
  // engines. For file verifications, we direct the user to the upload page
  // of each service so they can upload manually — browser security prevents
  // us programmatically uploading a local file path to a remote service.

  const reverseSearchLinks = $derived(() => {
    const sourceUrl = result?.sourceType === 'url' ? urlInput.trim() : null;

    return [
      {
        label: 'Google Lens',
        href: sourceUrl
          ? `https://lens.google.com/uploadbyurl?url=${encodeURIComponent(sourceUrl)}`
          : 'https://lens.google.com',
        title: sourceUrl
          ? 'Search via Google Lens — this will share the image URL with Google'
          : 'Open Google Lens — upload the file manually',
      },
      {
        label: 'TinEye',
        href: sourceUrl
          ? `https://tineye.com/search?url=${encodeURIComponent(sourceUrl)}`
          : 'https://tineye.com',
        title: sourceUrl
          ? 'Search via TinEye — this will share the image URL with TinEye'
          : 'Open TinEye — upload the file manually',
      },
      {
        label: 'Yandex Images',
        href: sourceUrl
          ? `https://yandex.com/images/search?rpt=imageview&url=${encodeURIComponent(sourceUrl)}`
          : 'https://yandex.com/images',
        title: sourceUrl
          ? 'Search via Yandex — this will share the image URL with Yandex'
          : 'Open Yandex Images — upload the file manually',
      },
    ];
  });

  function highestSeverityFindings(findings: AnomalyFinding[]): AnomalyFinding[] {
    const order: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
    return [...findings].sort((a, b) => (order[a.severity] ?? 5) - (order[b.severity] ?? 5));
  }

  function forensicScoreClass(score: number): string {
    if (score < 0.3) return 'text-malachite';
    if (score < 0.6) return 'text-amber';
    return 'text-cinnabar';
  }

  function forensicScoreBgClass(score: number): string {
    if (score < 0.3) return 'bg-malachite/15 border-malachite/20';
    if (score < 0.6) return 'bg-amber/15 border-amber/20';
    return 'bg-cinnabar/15 border-cinnabar/20';
  }

  /** Short label for a region detector score. */
  function regionScoreLabel(score: number): string {
    if (score < 0.3) return 'Clean';
    if (score < 0.6) return 'Review';
    return 'Suspicious';
  }

  /**
   * Whether the Region Analysis section should be shown.
   * Requires deep or archival mode AND at least one regional result present.
   */
  const hasRegionResults = $derived(
    (verifyMode === 'deep' || verifyMode === 'archival') && result != null && (
      result.segmentedElaResult != null ||
      result.shadowConsistencyResult != null ||
      result.colourTemperatureResult != null ||
      result.spliceBoundaryResult != null
    )
  );

  /** Count of regional detectors that returned suspicious. */
  const regionSuspiciousCount = $derived(
    result == null ? 0 : [
      result.segmentedElaResult?.suspicious,
      result.shadowConsistencyResult?.suspicious,
      result.colourTemperatureResult?.suspicious,
      result.spliceBoundaryResult?.suspicious,
    ].filter(Boolean).length
  );

  /** Total number of regional detectors that ran (returned a result). */
  const regionRunCount = $derived(
    result == null ? 0 : [
      result.segmentedElaResult,
      result.shadowConsistencyResult,
      result.colourTemperatureResult,
      result.spliceBoundaryResult,
    ].filter(v => v != null).length
  );

  // ── Summary view: plain-English top signals ───────────────────────
  /**
   * Derives up to three plain-English signal summaries from a VerificationResult.
   * Signals are ordered by severity (suspicious first) so the most important
   * findings appear regardless of which three are selected.
   */
  function getTopSignals(r: VerificationResult): string[] {
    type SignalEntry = { text: string; weight: number };
    const signals: SignalEntry[] = [];

    // ELA
    if (r.elaScore !== null && r.elaScore !== undefined) {
      if (r.elaScore >= 0.6)
        signals.push({ text: 'Error level analysis detected significant compression inconsistencies', weight: 3 });
      else if (r.elaScore >= 0.3)
        signals.push({ text: 'Error level analysis detected minor compression inconsistencies', weight: 2 });
      else
        signals.push({ text: 'Error level analysis is consistent with authentic content', weight: 0 });
    }

    // Noise
    if (r.noiseScore !== null && r.noiseScore !== undefined) {
      if (r.noiseScore >= 0.6)
        signals.push({ text: 'Noise pattern anomalies detected — may indicate compositing or AI generation', weight: 3 });
      else if (r.noiseScore >= 0.3)
        signals.push({ text: 'Minor noise pattern irregularities detected', weight: 2 });
      else
        signals.push({ text: 'Noise pattern is consistent with authentic photography', weight: 0 });
    }

    // Copy-move
    if (r.copyMoveScore !== null && r.copyMoveScore !== undefined) {
      if (r.copyMoveScore >= 0.6)
        signals.push({ text: 'Copy-move forgery detection found evidence of duplicated regions', weight: 3 });
      else if (r.copyMoveScore >= 0.3)
        signals.push({ text: 'Copy-move detection found possible repeated regions', weight: 2 });
      else
        signals.push({ text: 'No copy-move forgery regions detected', weight: 0 });
    }

    // Deepfake / AI generation
    if (r.deepfakeResult) {
      const vl = r.deepfakeResult.verdictLevel;
      if (vl === 'synthetic')
        signals.push({ text: 'AI generation signals strongly indicate synthetic content', weight: 4 });
      else if (vl === 'inconclusive')
        signals.push({ text: 'AI generation analysis returned an inconclusive result', weight: 2 });
      else
        signals.push({ text: 'AI generation signals are consistent with authentic content', weight: 0 });
    }

    // C2PA
    if (r.c2paValid === true)
      signals.push({ text: 'C2PA Content Credentials are present and valid', weight: 0 });
    else if (r.c2paValid === false)
      signals.push({ text: 'C2PA Content Credentials are present but failed validation', weight: 3 });

    // EXIF anomalies
    if (r.exifAnalysis) {
      const high = r.exifAnalysis.findings.filter(f => f.severity === 'high' || f.severity === 'critical');
      const medium = r.exifAnalysis.findings.filter(f => f.severity === 'medium');
      if (high.length > 0)
        signals.push({ text: `EXIF metadata contains ${high.length} high-severity anomal${high.length === 1 ? 'y' : 'ies'}`, weight: 3 });
      else if (medium.length > 0)
        signals.push({ text: `EXIF metadata contains ${medium.length} moderate anomal${medium.length === 1 ? 'y' : 'ies'}`, weight: 2 });
      else if (r.exifAnalysis.hasExif)
        signals.push({ text: 'No EXIF anomalies detected', weight: 0 });
      else
        signals.push({ text: 'No EXIF metadata present', weight: 1 });
    }

    // Metadata flags
    if (r.metadataFlags.length > 0)
      signals.push({ text: `${r.metadataFlags.length} metadata flag${r.metadataFlags.length === 1 ? '' : 's'} raised`, weight: 2 });

    // Watermark (Jura Trace)
    if (r.watermarkExtractResult?.hasWatermark)
      signals.push({ text: 'Jura Trace watermark detected — provenance credential embedded', weight: 0 });

    // Sort: most suspicious first, then clip to three
    signals.sort((a, b) => b.weight - a.weight);
    return signals.slice(0, 3).map(s => s.text);
  }
</script>

<div class="space-y-6">

  <!-- Page heading + sidecar status -->
  <div class="flex items-start justify-between">
    <div>
      <h1 class="text-2xl font-heading text-text-light dark:text-quartz">Verify</h1>
      <p class="text-flint dark:text-flint-light text-sm mt-1">
        Check the authenticity and provenance of files. All analysis happens locally on your device.
      </p>
    </div>
    <div class="flex items-center gap-3">
      <!-- Mode toggle -->
      <div class="flex items-center gap-1.5">
        <div
          class="flex items-stretch text-xs rounded-lg border border-border-light dark:border-border-dark overflow-hidden"
          role="radiogroup"
          aria-label="Verification mode"
        >
          {#each [
            {
              mode: 'standard' as VerifyMode,
              label: 'Standard',
              description: '~15 seconds — EXIF, C2PA, ELA, AI detection',
            },
            {
              mode: 'deep' as VerifyMode,
              label: 'Deep',
              description: '~60 seconds — all Standard detectors plus regional analysis, NPR, chromatic aberration, JPEG ghost',
            },
            {
              mode: 'archival' as VerifyMode,
              label: 'Archival',
              description: '~90 seconds — full Deep analysis with scanner-calibrated tolerances for digitised collections',
            },
          ] as opt}
            <button
              class="px-3 py-2 min-h-[44px] text-left transition-colors duration-150
                     {verifyMode === opt.mode
                       ? 'bg-lapis/20 text-lapis dark:text-lapis-light'
                       : 'text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis"
              role="radio"
              aria-checked={verifyMode === opt.mode}
              onclick={() => { verifyMode = opt.mode; localStorage.setItem('jura-verify-mode', opt.mode); }}
            >
              <span class="block font-medium">{opt.label}</span>
              <span class="block text-[10px] leading-tight mt-0.5
                           {verifyMode === opt.mode
                             ? 'text-lapis/70 dark:text-lapis-light/70'
                             : 'text-flint/70 dark:text-flint-light/60'}">
                {opt.description}
              </span>
            </button>
          {/each}
        </div>
        <ContextualHelpLink href="/help/verify#investigation-modes" label="Learn about investigation modes" />
      </div>

      <!-- Sidecar status -->
      <div
        class="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-full border
               {sidecarAvailable
                 ? sidecarDegraded()
                   ? 'bg-amber/10 text-amber dark:text-amber-light border-amber/20'
                   : 'bg-malachite/10 text-malachite dark:text-malachite-light border-malachite/20'
                 : 'bg-white dark:bg-graphite text-flint dark:text-flint-light border-border-light dark:border-border-dark'}"
        title={sidecarAvailable
          ? sidecarDegraded()
            ? `Analysis services limited — ${sidecarDegradedHint()}`
            : `Analysis services v${sidecarHealth?.version} — all capabilities available`
          : 'Analysis services offline — forensics not available'}
        aria-label={sidecarAvailable
          ? sidecarDegraded()
            ? `Analysis services limited — ${sidecarDegradedHint()}`
            : `Analysis services version ${sidecarHealth?.version} — all capabilities available`
          : 'Analysis services offline — forensics not available'}
        role="status"
      >
        <span
          class="w-1.5 h-1.5 rounded-full {sidecarAvailable ? sidecarDegraded() ? 'bg-amber' : 'bg-malachite' : 'bg-flint/50'}"
          aria-hidden="true"
        ></span>
        {sidecarAvailable
          ? sidecarDegraded() ? 'Analysis services limited' : 'Analysis services connected'
          : 'Analysis services offline'}
      </div>
    </div>
  </div>

  <!-- Error banner — structured by error type -->
  {#if error}
    <div
      class="rounded-lg px-4 py-3 text-sm border
        {errorType === 'sidecar' ? 'bg-amber/10 border-amber/30 text-amber dark:text-amber-light' :
         errorType === 'format' ? 'bg-lapis/10 border-lapis/30 text-lapis dark:text-lapis-light' :
         'bg-cinnabar/10 border-cinnabar/30 text-cinnabar dark:text-cinnabar-light'}"
      role="alert"
      aria-live="assertive"
      data-testid="error-banner"
      data-error-code={errorType}
    >
      <div class="flex items-start gap-2">
        <span class="font-medium flex-shrink-0">
          {errorType === 'sidecar' ? 'Analysis Engine offline' :
           errorType === 'format' ? 'Unsupported format' :
           errorType === 'network' ? 'Network error' : 'Error'}:
        </span>
        <span>{error}</span>
      </div>
    </div>
  {/if}

  <!-- ── Input Tabs ─────────────────────────────────────────────────── -->
  <div>
    <!-- Tab bar — soft pill style -->
    <div class="flex items-end border-b border-border-light dark:border-[rgba(122,119,112,0.15)] mb-4">
      <div role="tablist" aria-label="Verification input method" class="flex">
        <button
          class="px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                 {activeTab === 'file'
                   ? 'text-lapis dark:text-lapis-light border-lapis'
                   : 'text-flint dark:text-flint-light border-transparent hover:text-text-light dark:hover:text-quartz'}"
          role="tab"
          aria-selected={activeTab === 'file'}
          aria-controls="tab-panel-file"
          id="tab-file"
          onclick={() => { activeTab = 'file'; }}
        >
          File
        </button>
        <button
          class="px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                 {activeTab === 'batch'
                   ? 'text-lapis dark:text-lapis-light border-lapis'
                   : 'text-flint dark:text-flint-light border-transparent hover:text-text-light dark:hover:text-quartz'}"
          role="tab"
          aria-selected={activeTab === 'batch'}
          aria-controls="tab-panel-batch"
          id="tab-batch"
          onclick={() => { activeTab = 'batch'; }}
        >
          Batch
          {#if batchItems.length > 0}
            <span class="ml-1 text-xs text-flint dark:text-flint-light" aria-label="{batchItems.length} files queued">({batchItems.length})</span>
          {/if}
        </button>
        <button
          class="px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                 {activeTab === 'url'
                   ? 'text-lapis dark:text-lapis-light border-lapis'
                   : 'text-flint dark:text-flint-light border-transparent hover:text-text-light dark:hover:text-quartz'}"
          role="tab"
          aria-selected={activeTab === 'url'}
          aria-controls="tab-panel-url"
          id="tab-url"
          onclick={() => { activeTab = 'url'; }}
        >
          URL
        </button>
      </div>
      <div class="flex-1"></div>
      <p class="px-4 pb-2.5 text-xs text-flint/50" aria-label="Claim checking is planned for a future phase">
        Claim checking — Phase 2
      </p>
    </div>

    <!-- File tab -->
    {#if activeTab === 'file'}
    <div id="tab-panel-file" role="tabpanel" aria-labelledby="tab-file">
      <button
        class="w-full border-2 border-dashed rounded-lg p-10 text-center transition-all duration-200 cursor-pointer
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
               {dragOver
                 ? 'border-lapis bg-lapis/5 scale-[1.01]'
                 : 'border-border-light dark:border-border-dark hover:border-lapis/50'}
               {loading ? 'opacity-60 pointer-events-none' : ''}"
        ondragover={handleDragOver}
        ondragleave={handleDragLeave}
        ondrop={handleDrop}
        onclick={handleFileClick}
        aria-label="Drop a file here or click to select a file for verification"
        aria-busy={loading}
      >
        {#if loading}
          <div class="flex flex-col items-center gap-3">
            <div
              class="w-6 h-6 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
              role="status"
              aria-label={analysisPhase ?? 'Analysing file'}
            ></div>
            {#if analysisPhase}
              <!-- Video analysis: show the current phase message -->
              <p class="text-sm text-flint dark:text-flint-light">{analysisPhase}</p>
            {:else}
              <p class="text-sm text-flint dark:text-flint-light">Analysing file — this may take a moment...</p>
            {/if}
            {#if fileName}
              <p class="text-xs text-flint/70">{fileName}</p>
            {/if}
          </div>
        {:else}
          <div class="flex flex-col items-center gap-2">
            <svg class="w-10 h-10 text-flint dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
            <p class="text-text-light dark:text-quartz font-medium">Drop a file to verify</p>
            <p class="text-xs text-flint dark:text-flint-light">
              or click to browse
            </p>
            <p class="text-xs text-flint dark:text-flint-light mt-1">
              Supported: JPEG, PNG, TIFF, WebP, PDF, MP4, MOV, WAV, MP3
            </p>
          </div>
        {/if}
      </button>

      <!-- Video analysis progress footer — rendered below the drop zone so the
           cancel button is outside the outer <button> element (nested buttons
           are invalid HTML and would be unreachable). Only shown during video
           file loading. -->
      {#if loading && analysisPhase !== null}
        <div
          class="mt-3 flex flex-col items-center gap-2"
          role="status"
          aria-live="polite"
          aria-atomic="false"
          aria-label="Video analysis progress"
        >
          <p class="text-xs text-flint dark:text-flint-light">{estimatedTime}</p>
          <button
            type="button"
            onclick={cancelAnalysis}
            class="text-xs px-3 py-1.5 min-h-[32px] rounded border border-border-light dark:border-border-dark
                   text-flint dark:text-flint-light
                   hover:text-cinnabar hover:border-cinnabar/50 dark:hover:text-cinnabar dark:hover:border-cinnabar/50
                   transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
            aria-label="Cancel video analysis"
          >
            Cancel analysis
          </button>
          <p class="text-xs text-flint/50 dark:text-flint-light/50">
            Press Escape to cancel
          </p>
        </div>
      {/if}
    </div>
    {/if}

    <!-- Batch tab -->
    {#if activeTab === 'batch'}
    <div id="tab-panel-batch" role="tabpanel" aria-labelledby="tab-batch">
      <!-- Drop zone -->
      <button
        class="w-full border-2 border-dashed rounded-lg p-8 text-center transition-all duration-200 cursor-pointer
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
               {batchDragOver
                 ? 'border-lapis bg-lapis/5 scale-[1.01]'
                 : 'border-border-light dark:border-border-dark hover:border-lapis/50'}
               {batchRunning ? 'opacity-60 pointer-events-none' : ''}"
        ondragover={(e) => { e.preventDefault(); batchDragOver = true; }}
        ondragleave={() => { batchDragOver = false; }}
        ondrop={handleBatchDrop}
        onclick={handleBatchBrowse}
        aria-label="Drop files here or click to select files for batch verification"
      >
        <div class="flex flex-col items-center gap-2">
          <svg class="w-8 h-8 text-flint dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
              d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
          </svg>
          <p class="text-text-light dark:text-quartz font-medium">Drop multiple files to verify</p>
          <p class="text-xs text-flint dark:text-flint-light">or click to browse — files will be queued for sequential verification</p>
        </div>
      </button>

      <!-- Batch controls -->
      {#if batchItems.length > 0}
        <div class="flex items-center justify-between mt-4">
          <div class="flex items-center gap-3">
            <button
              class="px-4 py-2.5 min-h-[44px] bg-lapis hover:bg-lapis-dark dark:hover:bg-lapis-light text-white text-sm font-medium rounded-lg
                     transition-colors disabled:opacity-50 disabled:cursor-not-allowed
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
              onclick={runBatch}
              disabled={batchRunning || batchQueued === 0}
            >
              {#if batchRunning}
                Running...
              {:else}
                Run Batch
              {/if}
            </button>
            <span class="text-xs text-flint dark:text-flint-light">
              {batchCompleted} of {batchItems.length} complete
            </span>
          </div>
          <button
            class="text-xs text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-2 py-1"
            onclick={clearBatch}
            disabled={batchRunning}
          >
            Clear all
          </button>
        </div>

        <!-- Results table -->
        <div class="mt-4 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark overflow-x-auto">
          <!-- Header -->
          <div class="grid grid-cols-[1fr_90px_80px_80px_60px] gap-3 px-4 py-2 border-b border-border-light dark:border-border-dark text-xs text-flint dark:text-flint-light uppercase tracking-wide min-w-[480px]">
            <span>File</span>
            <span>Status</span>
            <span>Trust</span>
            <span>Duration</span>
            <span></span>
          </div>

          <!-- Rows -->
          {#each batchItems as item (item.id)}
            <div class="border-b border-border-light/50 dark:border-border-dark/50 min-w-[480px]">
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="w-full grid grid-cols-[1fr_90px_80px_80px_60px] gap-3 px-4 py-2.5 text-left cursor-pointer
                       hover:bg-gray-50 dark:hover:bg-graphite-light/30 transition-colors
                       {expandedBatchId === item.id ? 'bg-lapis/10' : ''}"
                onclick={() => {
                  if (item.status === 'done') {
                    expandedBatchId = expandedBatchId === item.id ? null : item.id;
                  }
                }}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); if (item.status === 'done') expandedBatchId = expandedBatchId === item.id ? null : item.id; } }}
                role="button"
                tabindex="0"
                aria-expanded={expandedBatchId === item.id}
              >
                <!-- File name -->
                <span class="text-sm text-text-light dark:text-quartz truncate" title={item.filePath}>
                  {item.fileName}
                </span>

                <!-- Status -->
                <span class="text-xs self-center">
                  {#if item.status === 'queued'}
                    <span class="text-flint dark:text-flint-light">Queued</span>
                  {:else if item.status === 'running'}
                    <span class="flex items-center gap-1.5">
                      <span
                        class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                        role="status"
                        aria-label="Verifying"
                      ></span>
                      <span class="text-lapis">Running</span>
                    </span>
                  {:else if item.status === 'done'}
                    {@const level = getTrustLevel(item.result?.overallTrust ?? 0)}
                    <span class="font-medium px-1.5 py-0.5 rounded
                      {level === 'high' ? 'text-malachite bg-malachite/10' :
                       level === 'medium' ? 'text-amber bg-amber/10' :
                       'text-cinnabar bg-cinnabar/10'}">
                      Done
                    </span>
                  {:else}
                    <span class="text-cinnabar">Error</span>
                  {/if}
                </span>

                <!-- Trust -->
                <span class="text-xs tabular-nums self-center">
                  {#if item.status === 'done' && item.result}
                    {@const level = getTrustLevel(item.result.overallTrust)}
                    <span class="{level === 'high' ? 'text-malachite' : level === 'medium' ? 'text-amber' : 'text-cinnabar'}">
                      {Math.round(item.result.overallTrust * 100)}%
                    </span>
                  {:else}
                    <span class="text-flint/50">—</span>
                  {/if}
                </span>

                <!-- Duration -->
                <span class="text-xs text-flint dark:text-flint-light tabular-nums self-center">
                  {#if item.startedAt && item.finishedAt}
                    {formatDuration(item.startedAt, item.finishedAt)}
                  {:else}
                    —
                  {/if}
                </span>

                <!-- Remove -->
                <span class="self-center text-right">
                  {#if !batchRunning || item.status !== 'running'}
                    <button
                      class="text-xs text-flint dark:text-flint-light hover:text-cinnabar transition-colors p-1 min-w-[24px] min-h-[24px]
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                      onclick={(e) => { e.stopPropagation(); removeBatchItem(item.id); }}
                      aria-label="Remove {item.fileName}"
                    >
                      &times;
                    </button>
                  {/if}
                </span>
              </div>

              <!-- Error message -->
              {#if item.status === 'error' && item.error}
                <div class="px-4 py-2 bg-cinnabar/5 text-xs text-cinnabar">
                  {item.error}
                </div>
              {/if}

              <!-- Expanded detail -->
              {#if expandedBatchId === item.id && item.result}
                <div class="px-4 py-4 bg-gray-50 dark:bg-obsidian/50 border-t border-border-light dark:border-border-dark">
                  <VerdictSummary result={item.result} fileName={item.fileName} />
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
    {/if}

    <!-- URL tab -->
    {#if activeTab === 'url'}
    <div id="tab-panel-url" role="tabpanel" aria-labelledby="tab-url">
      <div class="flex gap-3">
        <label for="url-verify-input" class="sr-only">URL to verify</label>
        <input
          id="url-verify-input"
          type="url"
          bind:value={urlInput}
          placeholder="https://example.com/image.jpg"
          disabled={loading}
          class="flex-1 bg-white dark:bg-obsidian border border-border-light dark:border-border-dark rounded-lg px-4 py-3 text-sm text-text-light dark:text-quartz
                 placeholder:text-flint/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-lapis
                 disabled:opacity-50"
          onkeydown={(e) => { if (e.key === 'Enter') runUrlVerification(); }}
        />
        <button
          onclick={runUrlVerification}
          disabled={loading || !urlInput.trim()}
          class="px-6 py-3 min-h-[44px] bg-lapis hover:bg-lapis-dark dark:hover:bg-lapis-light text-white text-sm font-medium rounded-lg
                 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {#if loading}
            Verifying...
          {:else}
            Verify
          {/if}
        </button>
      </div>
      <p class="text-xs text-flint dark:text-flint-light mt-2">
        Enter a URL to an image or document. The content will be downloaded and analysed locally.
      </p>
    </div>
    {/if}
  </div>

  <!-- ── Results ─────────────────────────────────────────────────── -->

  {#if checked && result}

    <!-- Document analysis notice -->
    {#if result.contentType === 'document'}
      <div
        class="rounded-lg border border-lapis/30 bg-lapis/10 px-4 py-3 mb-4 flex gap-3"
        role="note"
        aria-label="Limited analysis notice"
      >
        <svg
          class="w-4 h-4 flex-shrink-0 mt-0.5 text-lapis dark:text-lapis-light"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M12 2a10 10 0 110 20 10 10 0 010-20z" />
        </svg>
        <div class="text-xs text-lapis dark:text-lapis-light leading-relaxed">
          <p class="font-semibold mb-1">Document analysis — limited signals available</p>
          <p>Image forensic detectors (ELA, noise analysis, deepfake detection) do not apply to PDF documents.
             Trust is based on C2PA Content Credentials{result.c2paValid === true ? ' (valid credential found)' : result.c2paValid === false ? ' (invalid credential detected)' : ' (no credentials present)'}
             and file metadata only.</p>
        </div>
      </div>
    {/if}

    <!-- Trust Score header -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark overflow-hidden">
      <div class="px-5 py-4 border-b border-border-light dark:border-border-dark flex items-center justify-between gap-4">
        <div class="flex items-center gap-4 min-w-0">
          <div>
            <div class="flex items-center gap-1.5 mb-0.5">
              <p class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Trust Score</p>
              <ContextualHelpLink href="/help/verify#trust-score" label="Learn about trust scores" />
            </div>
            <div class="flex items-baseline gap-2">
              <span
                class="text-3xl font-heading tabular-nums {trustTextClass()}"
                aria-label="Trust score: {trustScorePercent} per cent"
              >
                {trustScorePercent}%
              </span>
              <span class="text-sm {trustTextClass()}">{trustLabelText()}</span>
            </div>
          </div>
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <p class="text-sm text-text-light dark:text-quartz truncate" title={fileName ?? undefined}>{fileName}</p>
              {#if result.mode}
                <span
                  class="flex-shrink-0 text-xs px-2 py-0.5 rounded border font-medium
                         {result.mode === 'archival'
                           ? 'bg-lapis/15 text-lapis dark:text-lapis-light border-lapis/30'
                           : result.mode === 'deep'
                             ? 'bg-lapis/10 text-lapis dark:text-lapis-light border-lapis/20'
                             : 'bg-graphite text-flint dark:text-flint-light border-border-dark dark:border-border-dark'}"
                  title="Investigation mode used for this analysis"
                  aria-label="Investigation mode: {result.mode}"
                >
                  {result.mode.charAt(0).toUpperCase() + result.mode.slice(1)}
                </span>
              {/if}
            </div>
            <p class="text-xs text-flint dark:text-flint-light mt-0.5">
              {result.contentType}
              {#if result.sourceType === 'url'}
                <span class="ml-1 text-lapis">(via URL)</span>
              {/if}
            </p>
          </div>
        </div>
        <!-- View mode toggle + Clear -->
        <div class="flex-shrink-0 flex items-center gap-3">
          <div
            class="flex items-center rounded-full border border-border-light dark:border-border-dark overflow-hidden text-xs"
            role="group"
            aria-label="Result view mode"
          >
            <button
              onclick={() => viewMode = 'summary'}
              class="px-3 py-1.5 min-h-[36px] transition-colors duration-150
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                     {viewMode === 'summary'
                       ? 'bg-lapis text-white dark:bg-lapis text-white'
                       : 'text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}"
              aria-pressed={viewMode === 'summary'}
            >
              Summary
            </button>
            <button
              onclick={() => viewMode = 'detail'}
              class="px-3 py-1.5 min-h-[36px] transition-colors duration-150 border-l border-border-light dark:border-border-dark
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                     {viewMode === 'detail'
                       ? 'bg-lapis text-white dark:bg-lapis text-white'
                       : 'text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}"
              aria-pressed={viewMode === 'detail'}
            >
              Full Analysis
            </button>
          </div>
          <button
            class="flex-shrink-0 text-xs text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-2 py-1 min-h-[44px] min-w-[44px] flex items-center"
            onclick={reset}
            aria-label="Clear result and verify another file"
          >
            Clear
          </button>
        </div>
      </div>

      <!-- Metadata flags (if any) -->
      {#if result.metadataFlags.length > 0}
        <div class="px-5 py-3 border-b border-border-light dark:border-border-dark flex flex-wrap gap-2" aria-label="Metadata flags">
          {#each result.metadataFlags as flag}
            <span class="text-xs px-2 py-0.5 rounded bg-amber/10 text-amber border border-amber/20">
              {flag}
            </span>
          {/each}
        </div>
      {/if}

      <!-- ── Summary view ──────────────────────────────────────────── -->
      {#if viewMode === 'summary'}

        <!-- Verdict Summary -->
        <div class="px-5 py-4 border-b border-border-light dark:border-border-dark">
          <VerdictSummary {result} fileName={fileName ?? 'Unknown file'} />

          <!-- Contextual caveat — one-line plain-English note keyed to the broad verdict category -->
          <p class="text-xs text-flint dark:text-flint-light leading-relaxed mt-2">
            {#if result.deepfakeResult?.verdictLevel === 'synthetic' || (result.deepfakeResult?.suspicious && result.deepfakeResult?.verdictLevel !== 'authentic')}
              Multiple detectors flagged signs of AI generation or manipulation. Review the signal breakdown below for details.
            {:else if result.deepfakeResult?.verdictLevel === 'inconclusive' || trustLevel() === 'medium'}
              Automated analysis could not make a confident determination. Apply professional judgement alongside these findings.
            {:else}
              No signs of manipulation or AI generation were detected by automated analysis. This does not guarantee the content is unmodified.
            {/if}
          </p>
        </div>

        <!-- Top signals in plain English -->
        {@const topSignals = getTopSignals(result)}
        {#if topSignals.length > 0}
          <div
            class="px-5 py-4 border-b border-border-light dark:border-border-dark"
            aria-label="Key findings"
          >
            <p class="text-xs font-medium text-flint dark:text-flint-light uppercase tracking-wide mb-3">
              Key Findings
            </p>
            <ul class="space-y-2" role="list">
              {#each topSignals as signal}
                <li class="flex items-start gap-2.5 text-sm text-text-light dark:text-quartz leading-relaxed">
                  <span
                    class="flex-shrink-0 mt-1.5 w-1.5 h-1.5 rounded-full
                           {signal.toLowerCase().includes('detected') || signal.toLowerCase().includes('anomal') || signal.toLowerCase().includes('failed') || signal.toLowerCase().includes('inconsistenc') || signal.toLowerCase().includes('synthetic') || signal.toLowerCase().includes('inconclusive')
                             ? 'bg-amber'
                             : 'bg-malachite'}"
                    aria-hidden="true"
                  ></span>
                  {signal}
                </li>
              {/each}
            </ul>
          </div>
        {/if}

        <!-- View full analysis CTA -->
        <div class="px-5 py-3 flex items-center justify-between gap-4">
          <p class="text-xs text-flint dark:text-flint-light">
            All analysis runs locally on your device.
          </p>
          <button
            onclick={() => viewMode = 'detail'}
            class="flex-shrink-0 text-xs px-3 py-2 min-h-[36px] rounded border border-lapis/50 text-lapis dark:text-lapis-light
                   hover:bg-lapis/10 transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          >
            View full analysis
          </button>
        </div>

      {:else}

      <!-- ── RAG Claim Verdict ─────────────────────────────────────── -->
      {#if result.claimVerdict || result.ragClaimResult}
        {@const rag = result.ragClaimResult}
        {@const verdict = result.claimVerdict ?? rag?.verdict}
        <div
          class="px-5 py-3 border-b border-border-light dark:border-border-dark
                 {verdict === 'supported'
                   ? 'bg-malachite/5'
                   : verdict === 'disputed'
                     ? 'bg-cinnabar/5'
                     : 'bg-amber/5'}"
          aria-label="Claim verification verdict"
        >
          <div class="flex items-center gap-3 mb-1.5">
            <span class="text-xs font-medium uppercase tracking-wide text-flint dark:text-flint-light">Claim Verification</span>
            <span
              class="text-xs font-medium px-2 py-0.5 rounded border
                     {verdict === 'supported'
                       ? 'bg-malachite/15 text-malachite border-malachite/30'
                       : verdict === 'disputed'
                         ? 'bg-cinnabar/15 text-cinnabar border-cinnabar/30'
                         : verdict === 'mixed'
                           ? 'bg-amber/15 text-amber border-amber/30'
                           : 'bg-graphite text-flint dark:text-flint-light border-border-dark'}"
            >
              {verdict === 'supported' ? 'Supported'
                : verdict === 'disputed' ? 'Disputed'
                : verdict === 'mixed' ? 'Mixed'
                : 'Unverified'}
            </span>
            {#if rag?.confidence != null}
              <span class="text-xs text-flint dark:text-flint-light tabular-nums">
                {Math.round(rag.confidence * 100)}% confidence
              </span>
            {/if}
          </div>
          {#if rag?.explanation}
            <p class="text-xs text-flint dark:text-flint-light leading-relaxed">{rag.explanation}</p>
          {/if}
          {#if rag?.sources && rag.sources.length > 0}
            <details class="group mt-2">
              <summary class="text-xs text-lapis dark:text-lapis-light cursor-pointer hover:opacity-80 transition-opacity list-none flex items-center gap-1.5">
                <svg
                  class="w-3 h-3 transition-transform duration-200 motion-safe:group-open:rotate-90"
                  fill="none" stroke="currentColor" viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
                {rag.sources.length} source{rag.sources.length === 1 ? '' : 's'} consulted
              </summary>
              <div class="mt-2 space-y-1.5" role="list" aria-label="RAG verification sources">
                {#each rag.sources as source, i (i)}
                  <div class="rounded-md px-3 py-2 bg-gray-50 dark:bg-obsidian/50 border border-border-light dark:border-border-dark text-xs" role="listitem">
                    <p class="font-medium text-text-light dark:text-quartz">{source.title}</p>
                    <p class="text-flint dark:text-flint-light mt-0.5 leading-relaxed">{source.excerpt}</p>
                    <p class="text-flint/50 tabular-nums mt-0.5">Relevance: {Math.round(source.relevance * 100)}%</p>
                  </div>
                {/each}
              </div>
            </details>
          {/if}
        </div>
      {/if}

      <!-- ── Verdict Summary ──────────────────────────────────────── -->
      <div class="px-5 py-4 border-b border-border-light dark:border-border-dark">
        <VerdictSummary {result} fileName={fileName ?? 'Unknown file'} />

        <!-- Contextual caveat -->
        <p class="text-xs text-flint dark:text-flint-light leading-relaxed mt-2">
          {#if result.deepfakeResult?.verdictLevel === 'synthetic' || (result.deepfakeResult?.suspicious && result.deepfakeResult?.verdictLevel !== 'authentic')}
            Multiple detectors flagged signs of AI generation or manipulation. Review the signal breakdown below for details.
          {:else if result.deepfakeResult?.verdictLevel === 'inconclusive' || trustLevel() === 'medium'}
            Automated analysis could not make a confident determination. Apply professional judgement alongside these findings.
          {:else}
            No signs of manipulation or AI generation were detected by automated analysis. This does not guarantee the content is unmodified.
          {/if}
        </p>
      </div>

      <!-- ── Signal Agreement ─────────────────────────────────────── -->
      <div class="px-5 py-3 border-b border-border-dark">
        <button
          class="flex items-center gap-2 text-sm text-flint dark:text-flint-light hover:text-quartz transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          onclick={() => { showSignalAgreement = !showSignalAgreement; }}
          aria-expanded={showSignalAgreement}
          aria-controls="signal-agreement-panel"
        >
          <svg
            class="w-3.5 h-3.5 transition-transform duration-200 {showSignalAgreement ? 'rotate-90' : ''}"
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
          Signal Agreement
          <span class="text-xs text-flint/60">detector cross-check</span>
        </button>
        {#if showSignalAgreement}
          <div id="signal-agreement-panel" class="mt-3">
            <SignalAgreement {result} />
          </div>
        {/if}
      </div>

      <!-- ── Visual Inspection Checklist ──────────────────────────── -->
      <div class="px-5 py-3 border-b border-border-dark">
        <button
          class="flex items-center gap-2 text-sm text-flint dark:text-flint-light hover:text-quartz transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          onclick={() => { showInspectionChecklist = !showInspectionChecklist; }}
          aria-expanded={showInspectionChecklist}
          aria-controls="inspection-checklist-panel"
        >
          <svg
            class="w-3.5 h-3.5 transition-transform duration-200 {showInspectionChecklist ? 'rotate-90' : ''}"
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
          Visual Inspection Checklist
          <span class="text-xs text-flint/60">manual assessment</span>
        </button>
        {#if showInspectionChecklist}
          <div id="inspection-checklist-panel" class="mt-3">
            <InspectionChecklist />
          </div>
        {/if}
      </div>

      <!-- ── Investigate Further ───────────────────────────────────── -->
      <div class="px-5 py-3 border-b border-border-dark">
        <button
          class="flex items-center gap-2 text-sm text-flint dark:text-flint-light hover:text-quartz transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          onclick={() => { showInvestigatePanel = !showInvestigatePanel; }}
          aria-expanded={showInvestigatePanel}
          aria-controls="investigate-further-panel"
        >
          <svg
            class="w-3.5 h-3.5 transition-transform duration-200 {showInvestigatePanel ? 'rotate-90' : ''}"
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
          Investigate Further
          <span class="text-xs text-flint/60">reverse image search</span>
          {#if licenceTier === 'community'}
            <span class="text-xs text-flint/50 italic">Professional plan includes API-integrated search</span>
          {/if}
        </button>
        {#if showInvestigatePanel}
          <div id="investigate-further-panel" class="mt-3">
            <!-- Source protection privacy warning -->
            <div
              class="rounded-lg border border-amber/30 bg-amber/10 px-4 py-3 mb-3 flex gap-3"
              role="note"
              aria-label="Source protection privacy caution"
            >
              <svg
                class="w-4 h-4 flex-shrink-0 mt-0.5 text-amber dark:text-amber-light"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 9v4m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                />
              </svg>
              <p class="text-xs text-amber dark:text-amber-light leading-relaxed">
                <span class="font-semibold">Caution:</span> using reverse image search services will share
                the image URL (and your IP address) with third-party commercial services. If you are
                verifying sensitive or unpublished material, consider whether this is appropriate for your
                source protection obligations.
              </p>
            </div>

            <div
              class="rounded-lg border border-border-dark bg-obsidian/50 px-4 py-3"
              aria-label="Reverse image search options"
            >
              <p class="text-xs text-flint dark:text-flint-light mb-3 leading-relaxed">
                Search for this image across the web to find other appearances, earlier versions, or
                context that may help verify its origin.
                {#if result.sourceType !== 'url'}
                  The file path cannot be sent directly — open the search engine's upload page and
                  drag the file in manually.
                {:else}
                  <span class="block mt-1 text-amber/80">
                    Privacy note: clicking a link will share the image URL with the selected search engine.
                  </span>
                {/if}
              </p>
              <div class="flex flex-wrap gap-2" role="group" aria-label="Search engine links">
                {#each reverseSearchLinks() as link}
                  <a
                    href={link.href}
                    target="_blank"
                    rel="noopener noreferrer"
                    title={link.title}
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md min-h-[44px]
                           border border-lapis/40 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis/70
                           transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                           focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                  >
                    {link.label}
                    <!-- External link indicator -->
                    <svg class="w-3 h-3 opacity-60" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                    </svg>
                    <span class="sr-only">(opens in new tab)</span>
                  </a>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- ── Export buttons ──────────────────────────────────────── -->
      <div class="px-5 py-3 border-b border-border-light dark:border-border-dark flex flex-wrap items-center gap-3">
        <button
          class="px-4 py-2.5 min-h-[44px] text-sm bg-lapis hover:bg-lapis-dark dark:hover:bg-lapis-light text-white rounded transition-colors
                 disabled:opacity-50 disabled:cursor-not-allowed
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          onclick={() => { showReportModal = true; }}
          disabled={exportingReport}
        >
          {exportingReport ? 'Generating...' : 'Export Report'}
        </button>
        <button
          class="px-4 py-2.5 min-h-[44px] text-sm border border-lapis/50 text-lapis rounded
                 hover:bg-lapis/10 transition-colors
                 disabled:opacity-50 disabled:cursor-not-allowed
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          onclick={handleExportCase}
          disabled={exportingCase}
        >
          {exportingCase ? 'Packaging...' : 'Export Case'}
        </button>
        <span class="text-xs text-flint/50">
          {modKey}+E report &middot; {modKey}+Shift+E case
        </span>

        {#if licenceTier === 'community'}
          <span class="text-xs text-flint dark:text-flint-light">
            Professional plan includes branded reports with your organisation name and case reference.
          </span>
        {/if}

        <!-- False positive report — secondary action, pushed to far right -->
        <div class="flex-1 flex justify-end">
          <button
            class="inline-flex items-center gap-1.5 px-3 py-2 min-h-[44px] text-xs text-flint dark:text-flint-light border border-border-light dark:border-border-dark rounded
                   hover:border-amber/50 hover:text-amber dark:hover:text-amber-light transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
            onclick={() => { showFalsePositiveModal = true; }}
            aria-label="Report this result as a false positive"
          >
            <!-- Flag icon -->
            <svg class="w-3.5 h-3.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75"
                d="M3 3v18M3 5l9-2 9 2v10l-9-2-9 2V5z" />
            </svg>
            Report False Positive
          </button>
        </div>
      </div>

      <!-- ── Technical Details Toggle ──────────────────────────────── -->
      <div class="px-5 py-3 border-b border-border-light dark:border-border-dark">
        <button
          class="flex items-center gap-2 text-sm text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          onclick={() => { showTechnicalDetails = !showTechnicalDetails; }}
          aria-expanded={showTechnicalDetails}
          aria-controls="technical-details"
        >
          <svg
            class="w-3.5 h-3.5 transition-transform duration-200 {showTechnicalDetails ? 'rotate-90' : ''}"
            fill="none" stroke="currentColor" viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
          Technical Details
          <span class="text-xs text-flint/60">
            ({verifyMode === 'standard' ? 'EXIF + C2PA + ELA + deepfake' : verifyMode === 'deep' ? 'full pipeline' : 'archival pipeline'})
          </span>
        </button>
      </div>

      {#if showTechnicalDetails}
      <div id="technical-details">

      <!-- ── ELA Analysis ──────────────────────────────────────────── -->
      {#if result.elaResult}
        {@const ela = result.elaResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="ela-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="ela-heading" class="text-sm font-medium text-text-light dark:text-quartz">Error Level Analysis</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(ela.score)} {forensicScoreClass(ela.score)}"
              >
                {ela.suspicious ? 'Suspicious' : 'Normal'}
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(ela.score)}">
              Score: {(ela.score * 100).toFixed(1)}%
            </span>
          </div>

          <!-- ELA heatmap -->
          <div class="mb-3 rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
            <img
              src={blobs.url(ela.elaImageBase64, 'image/png')}
              alt="Error Level Analysis heatmap showing compression artefact differences"
              class="w-full max-h-64 object-contain"
            />
          </div>

          <!-- Stats -->
          <div class="grid grid-cols-2 gap-4 text-xs">
            <div>
              <span class="text-flint dark:text-flint-light">Max Difference</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{ela.maxDifference.toFixed(1)}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Mean Difference</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{ela.meanDifference.toFixed(1)}</p>
            </div>
          </div>

          {#if ela.suspicious}
            <div class="mt-3 text-xs text-amber bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
              Elevated compression artefact variation detected. This may indicate pixel-level editing
              or compositing. Consider alongside other verification signals.
            </div>
          {/if}
        </section>
      {:else if checked && !sidecarAvailable}
        <section class="px-5 py-3 border-b border-border-dark" aria-labelledby="ela-heading">
          <div class="flex items-center gap-3">
            <h2 id="ela-heading" class="text-sm font-medium text-text-light dark:text-quartz">Error Level Analysis</h2>
            <span class="text-xs text-flint dark:text-flint-light bg-gray-100 dark:bg-graphite-light px-2 py-0.5 rounded border border-border-light dark:border-border-dark">
              Unavailable
            </span>
          </div>
          <p class="text-xs text-flint dark:text-flint-light mt-1.5">
            Analysis Engine is offline. Start the Analysis Engine to enable forensic analysis.
          </p>
        </section>
      {/if}

      <!-- ── Noise Analysis ────────────────────────────────────────── -->
      {#if result.noiseResult}
        {@const noise = result.noiseResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="noise-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="noise-heading" class="text-sm font-medium text-text-light dark:text-quartz">Noise Analysis</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(noise.score)} {forensicScoreClass(noise.score)}"
              >
                {noise.suspicious ? 'Suspicious' : 'Normal'}
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(noise.score)}">
              Score: {(noise.score * 100).toFixed(1)}%
            </span>
          </div>

          <!-- Noise heatmap -->
          {#if noise.heatmapBase64}
            <div class="mb-3 rounded-md overflow-hidden border border-border-dark bg-obsidian">
              <img
                src={blobs.url(noise.heatmapBase64, 'image/png')}
                alt="Noise variance heatmap — blue is low variance, red is high variance"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Stats -->
          <div class="grid grid-cols-3 gap-4 text-xs">
            <div>
              <span class="text-flint dark:text-flint-light">Global Variance</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{noise.globalVariance.toFixed(1)}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Anomalous Blocks</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{noise.anomalousBlocks} / {noise.totalBlocks}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Block Count</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{noise.totalBlocks}</p>
            </div>
          </div>

          {#if noise.suspicious}
            <div class="mt-3 text-xs text-amber bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
              Inconsistent noise patterns detected across image blocks. This may indicate region-level
              editing, splicing, or inpainting. Consider alongside other verification signals.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── Copy-Move Detection ───────────────────────────────────── -->
      {#if result.copyMoveResult}
        {@const cm = result.copyMoveResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="copymove-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="copymove-heading" class="text-sm font-medium text-text-light dark:text-quartz">Copy-Move Detection</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(cm.score)} {forensicScoreClass(cm.score)}"
              >
                {cm.suspicious ? 'Suspicious' : 'Clean'}
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(cm.score)}">
              Score: {(cm.score * 100).toFixed(1)}%
            </span>
          </div>

          <!-- Visualisation -->
          {#if cm.visualisationBase64}
            <div class="mb-3 rounded-md overflow-hidden border border-border-dark bg-obsidian">
              <img
                src={blobs.url(cm.visualisationBase64, 'image/png')}
                alt="Copy-move detection visualisation showing matched feature pairs and clone region bounding boxes"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Stats -->
          <div class="grid grid-cols-2 gap-4 text-xs">
            <div>
              <span class="text-flint dark:text-flint-light">Matched Pairs</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{cm.matchedPairs}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Clone Regions</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{cm.cloneRegions.length}</p>
            </div>
          </div>

          {#if cm.suspicious}
            <div class="mt-3 text-xs text-cinnabar bg-cinnabar/10 border border-cinnabar/20 rounded-md px-3 py-2">
              Duplicated regions detected within the image. This is a strong indicator of copy-move
              forgery — content appears to have been cloned from one area to another.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── Region Analysis ───────────────────────────────────────── -->
      {#if hasRegionResults && result}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="region-analysis-heading">

          <!-- Section header with expand/collapse toggle -->
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="region-analysis-heading" class="text-sm font-medium text-text-light dark:text-quartz">Region Analysis</h2>
              <!-- Summary badge: n of m suspicious -->
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border
                       {regionSuspiciousCount === 0
                         ? 'bg-malachite/15 border-malachite/20 text-malachite'
                         : regionSuspiciousCount >= 2
                           ? 'bg-cinnabar/15 border-cinnabar/20 text-cinnabar'
                           : 'bg-amber/15 border-amber/20 text-amber'}"
                aria-label="{regionSuspiciousCount} of {regionRunCount} region detectors suspicious"
              >
                {regionSuspiciousCount} of {regionRunCount} suspicious
              </span>
            </div>
            <button
              class="flex items-center gap-1 text-xs text-flint dark:text-flint-light hover:text-quartz transition-colors duration-150
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-1"
              onclick={() => { showRegionAnalysis = !showRegionAnalysis; }}
              aria-expanded={showRegionAnalysis}
              aria-controls="region-analysis-detail"
            >
              <svg
                class="w-3.5 h-3.5 transition-transform duration-200 {showRegionAnalysis ? 'rotate-90' : ''}"
                fill="none" stroke="currentColor" viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
              </svg>
              {showRegionAnalysis ? 'Collapse' : 'Expand'}
            </button>
          </div>

          <!-- Collapsed summary: one line per detector -->
          {#if !showRegionAnalysis}
            <div class="space-y-1" aria-label="Region detector summary">
              {#if result.segmentedElaResult}
                {@const seg = result.segmentedElaResult}
                <div class="flex items-center justify-between text-xs">
                  <span class="text-flint dark:text-flint-light">Segmented ELA</span>
                  <span class="{forensicScoreClass(seg.score)} tabular-nums">
                    {seg.anomalousRegions}/{seg.totalRegions} anomalous regions
                  </span>
                </div>
              {/if}
              {#if result.shadowConsistencyResult}
                {@const sh = result.shadowConsistencyResult}
                <div class="flex items-center justify-between text-xs">
                  <span class="text-flint dark:text-flint-light">Shadow Consistency</span>
                  <span class="{forensicScoreClass(sh.score)} tabular-nums">
                    {sh.inconsistentRegions} inconsistent
                  </span>
                </div>
              {/if}
              {#if result.colourTemperatureResult}
                {@const ct = result.colourTemperatureResult}
                <div class="flex items-center justify-between text-xs">
                  <span class="text-flint dark:text-flint-light">Colour Temperature</span>
                  <span class="{forensicScoreClass(ct.score)} tabular-nums">
                    {ct.anomalousRegions} deviating regions
                  </span>
                </div>
              {/if}
              {#if result.spliceBoundaryResult}
                {@const sb = result.spliceBoundaryResult}
                <div class="flex items-center justify-between text-xs">
                  <span class="text-flint dark:text-flint-light">Splice Boundary</span>
                  <span class="{forensicScoreClass(sb.score)} tabular-nums">
                    {sb.suspiciousBoundaries} of {sb.totalBoundariesChecked} boundaries
                  </span>
                </div>
              {/if}
            </div>
          {/if}

          <!-- Expanded detail panels -->
          {#if showRegionAnalysis}
            <div id="region-analysis-detail" class="space-y-5 mt-1">

              <!-- Segmented ELA sub-section -->
              {#if result.segmentedElaResult}
                {@const seg = result.segmentedElaResult}
                <details class="group">
                  <summary
                    class="flex items-center justify-between cursor-pointer list-none py-2 border-t border-border-light dark:border-border-dark
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                  >
                    <div class="flex items-center gap-3">
                      <span class="text-xs font-medium text-text-light dark:text-quartz">Segmented ELA</span>
                      <span
                        class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(seg.score)} {forensicScoreClass(seg.score)}"
                        aria-label="Segmented ELA: {regionScoreLabel(seg.score)}"
                      >
                        {regionScoreLabel(seg.score)}
                      </span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-xs tabular-nums {forensicScoreClass(seg.score)}">{(seg.score * 100).toFixed(1)}%</span>
                      <svg
                        class="w-3.5 h-3.5 text-flint dark:text-flint-light transition-transform duration-200 group-open:rotate-90"
                        fill="none" stroke="currentColor" viewBox="0 0 24 24"
                        aria-hidden="true"
                      >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                      </svg>
                    </div>
                  </summary>

                  <div class="mt-3 space-y-3">
                    <p class="text-xs text-flint dark:text-flint-light leading-relaxed">{seg.summary}</p>

                    {#if seg.heatmapBase64}
                      <div class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
                        <img
                          src={blobs.url(seg.heatmapBase64, 'image/png')}
                          alt="Segmented ELA heatmap showing per-region compression anomaly scores"
                          class="w-full max-h-64 object-contain"
                        />
                      </div>
                    {/if}

                    <div class="grid grid-cols-3 gap-3 text-xs">
                      <div>
                        <span class="text-flint dark:text-flint-light">Anomalous Regions</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{seg.anomalousRegions} / {seg.totalRegions}</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Inter-region Variance</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{seg.interRegionVariance.toFixed(3)}</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Total Regions</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{seg.totalRegions}</p>
                      </div>
                    </div>

                    {#if seg.suspicious}
                      <div class="text-xs text-amber bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
                        Elevated compression variance detected across image regions. Inconsistent ELA patterns
                        between blocks may indicate that regions were edited or inserted separately.
                      </div>
                    {/if}
                  </div>
                </details>
              {/if}

              <!-- Shadow Consistency sub-section -->
              {#if result.shadowConsistencyResult}
                {@const sh = result.shadowConsistencyResult}
                <details class="group">
                  <summary
                    class="flex items-center justify-between cursor-pointer list-none py-2 border-t border-border-light dark:border-border-dark
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                  >
                    <div class="flex items-center gap-3">
                      <span class="text-xs font-medium text-text-light dark:text-quartz">Shadow Consistency</span>
                      <span
                        class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(sh.score)} {forensicScoreClass(sh.score)}"
                        aria-label="Shadow Consistency: {regionScoreLabel(sh.score)}"
                      >
                        {regionScoreLabel(sh.score)}
                      </span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-xs tabular-nums {forensicScoreClass(sh.score)}">{(sh.score * 100).toFixed(1)}%</span>
                      <svg
                        class="w-3.5 h-3.5 text-flint dark:text-flint-light transition-transform duration-200 group-open:rotate-90"
                        fill="none" stroke="currentColor" viewBox="0 0 24 24"
                        aria-hidden="true"
                      >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                      </svg>
                    </div>
                  </summary>

                  <div class="mt-3 space-y-3">
                    <p class="text-xs text-flint dark:text-flint-light leading-relaxed">{sh.summary}</p>

                    {#if sh.heatmapBase64}
                      <div class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
                        <img
                          src={blobs.url(sh.heatmapBase64, 'image/png')}
                          alt="Shadow consistency heatmap showing regions with inconsistent light direction"
                          class="w-full max-h-64 object-contain"
                        />
                      </div>
                    {/if}

                    <div class="grid grid-cols-3 gap-3 text-xs">
                      <div>
                        <span class="text-flint dark:text-flint-light">Light Direction</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{sh.globalLightDirection.toFixed(1)}&deg;</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Inconsistent Regions</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{sh.inconsistentRegions} / {sh.totalRegions}</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Total Regions</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{sh.totalRegions}</p>
                      </div>
                    </div>

                    {#if sh.suspicious}
                      <div class="text-xs text-amber bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
                        Shadow directions in one or more regions deviate significantly from the global light
                        direction. This may indicate that elements were composited from differently-lit sources.
                      </div>
                    {/if}
                  </div>
                </details>
              {/if}

              <!-- Colour Temperature sub-section -->
              {#if result.colourTemperatureResult}
                {@const ct = result.colourTemperatureResult}
                <details class="group">
                  <summary
                    class="flex items-center justify-between cursor-pointer list-none py-2 border-t border-border-light dark:border-border-dark
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                  >
                    <div class="flex items-center gap-3">
                      <span class="text-xs font-medium text-text-light dark:text-quartz">Colour Temperature</span>
                      <span
                        class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(ct.score)} {forensicScoreClass(ct.score)}"
                        aria-label="Colour Temperature: {regionScoreLabel(ct.score)}"
                      >
                        {regionScoreLabel(ct.score)}
                      </span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-xs tabular-nums {forensicScoreClass(ct.score)}">{(ct.score * 100).toFixed(1)}%</span>
                      <svg
                        class="w-3.5 h-3.5 text-flint dark:text-flint-light transition-transform duration-200 group-open:rotate-90"
                        fill="none" stroke="currentColor" viewBox="0 0 24 24"
                        aria-hidden="true"
                      >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                      </svg>
                    </div>
                  </summary>

                  <div class="mt-3 space-y-3">
                    <p class="text-xs text-flint dark:text-flint-light leading-relaxed">{ct.summary}</p>

                    {#if ct.heatmapBase64}
                      <div class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
                        <img
                          src={blobs.url(ct.heatmapBase64, 'image/png')}
                          alt="Colour temperature heatmap showing regions deviating from the global colour balance"
                          class="w-full max-h-64 object-contain"
                        />
                      </div>
                    {/if}

                    <div class="grid grid-cols-2 gap-3 text-xs">
                      <div>
                        <span class="text-flint dark:text-flint-light">Global A (green-red)</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{ct.globalMeanA.toFixed(2)}</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Global B (blue-yellow)</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{ct.globalMeanB.toFixed(2)}</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Anomalous Regions</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{ct.anomalousRegions} / {ct.totalRegions}</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Total Regions</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{ct.totalRegions}</p>
                      </div>
                    </div>

                    {#if ct.suspicious}
                      <div class="text-xs text-amber bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
                        Regions with significantly different colour temperatures were found. Inconsistent
                        white balance across an image may indicate elements were captured under different
                        lighting conditions and composited together.
                      </div>
                    {/if}
                  </div>
                </details>
              {/if}

              <!-- Splice Boundary sub-section -->
              {#if result.spliceBoundaryResult}
                {@const sb = result.spliceBoundaryResult}
                <details class="group">
                  <summary
                    class="flex items-center justify-between cursor-pointer list-none py-2 border-t border-border-light dark:border-border-dark
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                  >
                    <div class="flex items-center gap-3">
                      <span class="text-xs font-medium text-text-light dark:text-quartz">Splice Boundary</span>
                      <span
                        class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(sb.score)} {forensicScoreClass(sb.score)}"
                        aria-label="Splice Boundary: {regionScoreLabel(sb.score)}"
                      >
                        {regionScoreLabel(sb.score)}
                      </span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-xs tabular-nums {forensicScoreClass(sb.score)}">{(sb.score * 100).toFixed(1)}%</span>
                      <svg
                        class="w-3.5 h-3.5 text-flint dark:text-flint-light transition-transform duration-200 group-open:rotate-90"
                        fill="none" stroke="currentColor" viewBox="0 0 24 24"
                        aria-hidden="true"
                      >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                      </svg>
                    </div>
                  </summary>

                  <div class="mt-3 space-y-3">
                    <p class="text-xs text-flint dark:text-flint-light leading-relaxed">{sb.summary}</p>

                    {#if sb.heatmapBase64}
                      <div class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
                        <img
                          src={blobs.url(sb.heatmapBase64, 'image/png')}
                          alt="Splice boundary heatmap showing candidate cut edges between composited regions"
                          class="w-full max-h-64 object-contain"
                        />
                      </div>
                    {/if}

                    <div class="grid grid-cols-2 gap-3 text-xs">
                      <div>
                        <span class="text-flint dark:text-flint-light">Suspicious Boundaries</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{sb.suspiciousBoundaries}</p>
                      </div>
                      <div>
                        <span class="text-flint dark:text-flint-light">Total Checked</span>
                        <p class="text-text-light dark:text-quartz tabular-nums">{sb.totalBoundariesChecked}</p>
                      </div>
                    </div>

                    {#if sb.boundaries.length > 0}
                      <details class="group/inner">
                        <summary class="text-xs text-lapis cursor-pointer hover:text-lapis-light transition-colors">
                          {sb.suspiciousBoundaries} candidate {sb.suspiciousBoundaries === 1 ? 'boundary' : 'boundaries'} — view details
                        </summary>
                        <div class="mt-2 space-y-1.5" role="list" aria-label="Splice boundary candidates">
                          {#each sb.boundaries as boundary, i (i)}
                            <div
                              class="rounded-md px-3 py-2 text-xs border
                                     {boundary.confidence > 0.6
                                       ? 'bg-cinnabar/10 border-cinnabar/20'
                                       : boundary.confidence > 0.3
                                         ? 'bg-amber/10 border-amber/20'
                                         : 'bg-graphite-light/50 border-border-dark'}"
                              role="listitem"
                            >
                              <div class="flex items-center justify-between gap-2 mb-1">
                                <span class="font-mono text-quartz">
                                  ({boundary.x}, {boundary.y}) &mdash; {boundary.width}&times;{boundary.height}px
                                </span>
                                <span class="tabular-nums text-flint dark:text-flint-light">{(boundary.confidence * 100).toFixed(0)}% confidence</span>
                              </div>
                              <div class="flex flex-wrap gap-x-3 gap-y-0.5 text-flint dark:text-flint-light">
                                {#if boundary.jpegGridAligned}
                                  <span>JPEG grid aligned</span>
                                {/if}
                                {#if boundary.noiseAsymmetric}
                                  <span>Asymmetric noise</span>
                                {/if}
                                {#if boundary.featheringDetected}
                                  <span>Feathering detected</span>
                                {/if}
                                <span>{boundary.signalsTriggered} signal{boundary.signalsTriggered === 1 ? '' : 's'} triggered</span>
                              </div>
                            </div>
                          {/each}
                        </div>
                      </details>
                    {/if}

                    {#if sb.suspicious}
                      <div class="text-xs text-cinnabar bg-cinnabar/10 border border-cinnabar/20 rounded-md px-3 py-2">
                        One or more cut edges with multiple corroborating signals were found. This pattern
                        is consistent with content being inserted or replaced at a region boundary.
                      </div>
                    {/if}
                  </div>
                </details>
              {/if}

            </div>
          {/if}

        </section>
      {/if}

      <!-- ── NPR Analysis ──────────────────────────────────────────── -->
      {#if result.nprResult}
        {@const npr = result.nprResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="npr-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="npr-heading" class="text-sm font-medium text-text-light dark:text-quartz">Neighbouring Pixel Relationships</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(npr.score)} {forensicScoreClass(npr.score)}"
              >
                {npr.suspicious ? 'Suspicious' : 'Clean'}
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(npr.score)}">
              Score: {(npr.score * 100).toFixed(1)}%
            </span>
          </div>

          <!-- NPR heatmap -->
          {#if npr.heatmapBase64}
            <div class="mb-3 rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
              <img
                src={blobs.url(npr.heatmapBase64, 'image/png')}
                alt="Neighbouring pixel relationship heatmap showing local correlation anomalies"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Stats -->
          <div class="grid grid-cols-3 gap-4 text-xs mb-3">
            <div>
              <span class="text-flint dark:text-flint-light">H-V Correlation</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{npr.hvCorrelation.toFixed(4)}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Diff Variance Ratio</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{npr.diffVarianceRatio.toFixed(4)}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">HF Energy Ratio</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{npr.hfEnergyRatio.toFixed(4)}</p>
            </div>
          </div>

          <p class="text-xs text-flint dark:text-flint-light leading-relaxed">{npr.summary}</p>

          {#if npr.suspicious}
            <div class="mt-3 text-xs text-amber bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
              Anomalous pixel neighbourhood correlation detected. This pattern can result from local
              resampling, inpainting, or region insertion that disrupts the natural statistical
              relationship between adjacent pixels.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── JPEG Ghost Detection ───────────────────────────────────── -->
      {#if result.jpegGhostResult}
        {@const jg = result.jpegGhostResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="jpegGhost-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="jpegGhost-heading" class="text-sm font-medium text-text-light dark:text-quartz">JPEG Ghost Detection</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(jg.score)} {forensicScoreClass(jg.score)}"
              >
                {jg.suspicious ? 'Suspicious' : 'Clean'}
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(jg.score)}">
              Score: {(jg.score * 100).toFixed(1)}%
            </span>
          </div>

          <!-- JPEG Ghost heatmap -->
          {#if jg.heatmapBase64}
            <div class="mb-3 rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
              <img
                src={blobs.url(jg.heatmapBase64, 'image/png')}
                alt="JPEG ghost heatmap showing blocks with mismatched compression quality history"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Stats -->
          <div class="grid grid-cols-2 gap-4 text-xs mb-3">
            <div>
              <span class="text-flint dark:text-flint-light">Dominant Ghost Quality</span>
              <p class="text-text-light dark:text-quartz tabular-nums">Q{jg.ghostQuality}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Quality Variance</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{jg.qualityVariance.toFixed(3)}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Deviating Blocks</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{jg.deviatingBlocks} / {jg.totalBlocks}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Total Blocks</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{jg.totalBlocks}</p>
            </div>
          </div>

          <p class="text-xs text-flint dark:text-flint-light leading-relaxed">{jg.summary}</p>

          {#if jg.suspicious}
            <div class="mt-3 text-xs text-amber bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
              Blocks with different JPEG compression histories detected. This is a marker of splice
              forgery — regions from a differently-compressed source image leave a ghost artefact
              pattern when re-compressed at the target quality level.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── Chromatic Aberration Analysis ─────────────────────────── -->
      {#if result.caResult}
        {@const ca = result.caResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="ca-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="ca-heading" class="text-sm font-medium text-text-light dark:text-quartz">Chromatic Aberration</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border
                       {ca.isConsistent
                         ? 'bg-malachite/15 border-malachite/20 text-malachite'
                         : 'bg-amber/15 border-amber/20 text-amber'}"
              >
                {ca.isConsistent ? 'Consistent' : 'Inconsistent'}
              </span>
              <!-- Informational tag — always shown -->
              <span
                class="text-xs px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light border border-border-light dark:border-border-dark"
                title="Chromatic aberration analysis is informational only — results may be unreliable for mobile phone photos processed with computational lens correction"
              >
                Informational
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(ca.score)}">
              Score: {(ca.score * 100).toFixed(1)}%
            </span>
          </div>

          <!-- Stats -->
          <div class="grid grid-cols-3 gap-4 text-xs mb-3">
            <div>
              <span class="text-flint dark:text-flint-light">R² Value</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{ca.rSquared.toFixed(4)}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Sample Count</span>
              <p class="text-text-light dark:text-quartz tabular-nums">{ca.sampleCount}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Consistent</span>
              <p class="text-text-light dark:text-quartz">{ca.isConsistent ? 'Yes' : 'No'}</p>
            </div>
          </div>

          <p class="text-xs text-flint dark:text-flint-light leading-relaxed mb-2">{ca.summary}</p>

          <p class="text-xs text-flint/60 italic leading-relaxed">
            Note: this detector is informational only. Results are unreliable for mobile phone
            photos processed with computational lens correction (iPhone, Pixel, Samsung), HDR
            composites, or images that have been resized or cropped.
          </p>
        </section>
      {/if}

      <!-- ── CLIP Detection ─────────────────────────────────────────── -->
      {#if result.clipResult}
        {@const clip = result.clipResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="clip-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="clip-heading" class="text-sm font-medium text-text-light dark:text-quartz">CLIP Classification</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(clip.score)} {forensicScoreClass(clip.score)}"
              >
                {clip.verdictLevel === 'authentic' ? 'Authentic' : clip.verdictLevel === 'synthetic' ? 'Synthetic' : 'Inconclusive'}
              </span>
              <!-- Experimental badge -->
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border bg-amber/10 text-amber border-amber/30"
                title="CLIP-based AI classification is experimental. Do not use as sole evidence."
              >
                Experimental
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(clip.score)}">
              Score: {(clip.score * 100).toFixed(1)}%
            </span>
          </div>

          <div class="grid grid-cols-2 gap-4 text-xs mb-3">
            <div>
              <span class="text-flint dark:text-flint-light">Verdict</span>
              <p class="text-text-light dark:text-quartz capitalize">{clip.verdictLevel}</p>
            </div>
            <div>
              <span class="text-flint dark:text-flint-light">Confidence</span>
              <p class="text-text-light dark:text-quartz capitalize">{clip.confidence}</p>
            </div>
          </div>

          <!-- Class probabilities -->
          {#if Object.keys(clip.classProbs).length > 0}
            <div class="mb-3">
              <p class="text-xs text-flint dark:text-flint-light mb-2">Class probabilities</p>
              <div class="space-y-1.5" role="list" aria-label="CLIP class probabilities">
                {#each Object.entries(clip.classProbs).sort((a, b) => b[1] - a[1]) as [label, prob] (label)}
                  <div class="flex items-center gap-3 text-xs" role="listitem">
                    <span class="w-32 text-flint dark:text-flint-light capitalize truncate" title={label}>{label}</span>
                    <div class="flex-1 h-1.5 rounded-full bg-gray-200 dark:bg-graphite-light overflow-hidden" role="presentation">
                      <div
                        class="h-full rounded-full bg-lapis/60 transition-all duration-300 ease-out"
                        style="width: {Math.round(prob * 100)}%"
                      ></div>
                    </div>
                    <span class="w-10 tabular-nums text-right text-flint dark:text-flint-light">{Math.round(prob * 100)}%</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <p class="text-xs text-flint dark:text-flint-light leading-relaxed mb-2">{clip.summary}</p>

          <div class="text-xs text-amber/80 bg-amber/5 border border-amber/20 rounded-md px-3 py-2">
            This result is experimental. CLIP-based classification has not been independently
            validated for forensic use. Treat it as a supporting signal only, not as evidence
            of manipulation or AI generation.
          </div>
        </section>
      {/if}

      <!-- ── AI Generation Detection ─────────────────────────────── -->
      {#if result.deepfakeResult}
        {@const df = result.deepfakeResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="deepfake-heading">
          <!-- AI Watermark Detections -->
          {#if df.watermarks?.some(w => w.detected)}
            <div class="mb-3 rounded-md border border-cinnabar/30 bg-cinnabar/10 px-4 py-3">
              <p class="text-xs font-medium text-cinnabar-light mb-1.5">
                AI Generator Watermark Detected
              </p>
              {#each df.watermarks.filter(w => w.detected) as wm (wm.watermarkType)}
                <div class="flex items-center justify-between text-xs mb-1 last:mb-0">
                  <span class="text-text-light dark:text-quartz font-mono">
                    {wm.watermarkType.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
                  </span>
                  <span class="text-flint dark:text-flint-light tabular-nums">
                    {(wm.confidence * 100).toFixed(0)}% confidence
                  </span>
                </div>
                <p class="text-xs text-flint dark:text-flint-light mb-1">{wm.details}</p>
              {/each}
            </div>
          {/if}

          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="deepfake-heading" class="text-sm font-medium text-text-light dark:text-quartz">AI Generation Detection</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(df.score)} {forensicScoreClass(df.score)}"
              >
                {df.suspicious ? 'Suspicious' : 'Normal'}
              </span>
              <span
                class="text-xs px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light border border-border-light dark:border-border-dark"
                title="Confidence level of the detection"
              >
                {df.confidence} confidence
              </span>
            </div>
            <span class="text-xs tabular-nums {forensicScoreClass(df.score)}">
              Score: {(df.score * 100).toFixed(1)}%
            </span>
          </div>

          <!-- Frequency spectrum heatmap -->
          {#if df.heatmapBase64}
            <div class="mb-3 rounded-md overflow-hidden border border-border-dark bg-obsidian">
              <img
                src={blobs.url(df.heatmapBase64, 'image/png')}
                alt="Frequency spectrum heatmap for AI generation detection"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Summary -->
          <p class="text-xs text-flint dark:text-flint-light mb-3">{df.summary}</p>

          <!-- Signal list -->
          {#if df.signals.length > 0}
            <details class="group">
              <summary class="text-xs text-lapis cursor-pointer hover:text-lapis-light transition-colors">
                {df.signals.filter(s => s.triggered).length} of {df.signals.length} signals triggered — view details
              </summary>
              <div class="mt-2 space-y-1.5" role="list" aria-label="Detection signals">
                {#each df.signals as signal (signal.name)}
                  <div
                    class="flex items-start gap-2 rounded-md px-3 py-2 text-xs
                           {signal.triggered ? 'bg-amber/10 border border-amber/20' : 'bg-graphite-light/50 border border-border-dark'}"
                    role="listitem"
                  >
                    <span
                      class="flex-shrink-0 w-1.5 h-1.5 mt-1 rounded-full {signal.triggered ? 'bg-amber' : 'bg-flint/30'}"
                      aria-hidden="true"
                    ></span>
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center justify-between gap-2">
                        <span class="font-mono {signal.triggered ? 'text-amber' : 'text-flint dark:text-flint-light'}">{signal.name}</span>
                        <span class="text-flint/60 tabular-nums">weight: {signal.weight.toFixed(1)}</span>
                      </div>
                      <p class="text-flint dark:text-flint-light mt-0.5">{signal.description}</p>
                    </div>
                  </div>
                {/each}
              </div>
            </details>
          {/if}

          {#if df.suspicious}
            <div class="mt-3 text-xs text-cinnabar bg-cinnabar/10 border border-cinnabar/20 rounded-md px-3 py-2">
              Multiple statistical signals suggest this image may be AI-generated or synthetically produced.
              Consider alongside other verification signals and the specific context of use.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── EXIF Analysis ─────────────────────────────────────────── -->
      {#if result.exifAnalysis}
        {@const exif = result.exifAnalysis}
        <section class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="exif-heading">
          <div class="flex items-center justify-between mb-3">
            <h2 id="exif-heading" class="text-sm font-medium text-text-light dark:text-quartz">EXIF Analysis</h2>
            <span class="text-xs text-flint dark:text-flint-light">
              {exif.fieldsPopulated}/{exif.fieldsTotal} fields populated
            </span>
          </div>

          <!-- Completeness bar -->
          <div class="mb-4">
            <div
              class="h-1.5 rounded-full bg-gray-200 dark:bg-graphite-light overflow-hidden"
              role="progressbar"
              aria-valuenow={exif.fieldsPopulated}
              aria-valuemin={0}
              aria-valuemax={exif.fieldsTotal}
              aria-label="EXIF completeness: {exif.fieldsPopulated} of {exif.fieldsTotal} fields populated"
            >
              <div
                class="h-full rounded-full transition-all duration-300 ease-out
                       {exif.fieldsPopulated / exif.fieldsTotal >= 0.7
                         ? 'bg-malachite'
                         : exif.fieldsPopulated / exif.fieldsTotal >= 0.4
                           ? 'bg-amber'
                           : 'bg-cinnabar'}"
                style="width: {Math.round((exif.fieldsPopulated / exif.fieldsTotal) * 100)}%"
              ></div>
            </div>
            <p class="text-xs text-flint dark:text-flint-light mt-1">
              {exif.fieldsPopulated} of {exif.fieldsTotal} EXIF fields populated
              {#if !exif.hasExif}
                <span class="text-amber ml-1">— no EXIF data present</span>
              {/if}
            </p>
          </div>

          <!-- Findings list -->
          {#if exif.findings.length > 0}
            <div class="space-y-2" role="list" aria-label="EXIF anomaly findings">
              {#each highestSeverityFindings(exif.findings) as finding (finding.checkId)}
                {@const config = SEVERITY_CONFIG[finding.severity]}
                <div
                  class="flex items-start gap-3 rounded-md px-3 py-2.5 {config.bgClass}"
                  role="listitem"
                >
                  <span
                    class="flex-shrink-0 text-xs font-medium px-2 py-0.5 rounded-full {config.bgClass} {config.textClass} border
                           {finding.severity === 'critical' || finding.severity === 'high'
                             ? 'border-cinnabar/30'
                             : finding.severity === 'medium'
                               ? 'border-amber/30'
                               : finding.severity === 'low'
                                 ? 'border-lapis/30'
                                 : 'border-border-dark'}"
                    aria-label="Severity: {config.label}"
                  >
                    {config.label}
                  </span>
                  <div class="min-w-0">
                    <p class="text-sm text-text-light dark:text-quartz leading-snug">{finding.title}</p>
                    <p class="text-xs text-flint dark:text-flint-light mt-0.5 leading-relaxed">{finding.description}</p>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-xs text-flint dark:text-flint-light">No anomalies detected in EXIF metadata.</p>
          {/if}
        </section>
      {/if}

      <!-- ── C2PA Credentials ──────────────────────────────────────── -->
      {#if result.c2paManifest}
        {@const manifest = result.c2paManifest}
        <section class="px-5 py-4" aria-labelledby="c2pa-heading">
          <div class="flex items-center gap-3 mb-4">
            <h2 id="c2pa-heading" class="text-sm font-medium text-text-light dark:text-quartz">C2PA Credentials</h2>
            <span
              class="text-xs font-medium px-2 py-0.5 rounded
                     {manifest.isValid
                       ? 'bg-malachite/15 text-malachite-light border border-malachite/20'
                       : 'bg-cinnabar/15 text-cinnabar-light border border-cinnabar/20'}"
              aria-label="C2PA signature is {manifest.isValid ? 'valid' : 'invalid'}"
            >
              {manifest.isValid ? 'Valid' : 'Invalid'}
            </span>
            {#if result.aiGenerator}
              <span
                class="text-xs font-medium px-2 py-0.5 rounded
                       bg-cinnabar/15 text-cinnabar-light border border-cinnabar/20"
                aria-label="AI-generated content detected"
              >
                AI-Generated
              </span>
            {/if}
          </div>

          {#if result.aiGenerator}
            <div
              class="rounded-lg border border-amber/30 bg-amber/10 px-4 py-3 mb-4 flex gap-3"
              role="note"
              aria-label="AI generation provenance confirmation"
            >
              <svg class="w-4 h-4 flex-shrink-0 mt-0.5 text-amber dark:text-amber-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v4m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
              </svg>
              <div class="text-xs text-amber dark:text-amber-light leading-relaxed">
                <p class="font-semibold mb-1">Verified AI-generated content</p>
                <p>This content carries a valid, signed C2PA provenance record which confirms it was created using AI generation.
                   Source: <span class="font-medium">{result.aiGenerator}</span>.
                   The provenance chain is cryptographically intact — the content itself declares its synthetic origin.</p>
              </div>
            </div>
          {/if}

          <div class="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-3 text-sm mb-4">
            {#if manifest.claimGenerator}
              <div>
                <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Claim Generator</span>
                <p class="text-text-light dark:text-quartz mt-0.5 break-words">{manifest.claimGenerator}</p>
              </div>
            {/if}
            {#if manifest.format}
              <div>
                <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Format</span>
                <p class="text-text-light dark:text-quartz mt-0.5">{manifest.format}</p>
              </div>
            {/if}
            {#if manifest.title}
              <div>
                <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Title</span>
                <p class="text-text-light dark:text-quartz mt-0.5 break-words">{manifest.title}</p>
              </div>
            {/if}
            {#if manifest.signedAt}
              <div>
                <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Signed At</span>
                <p class="text-text-light dark:text-quartz mt-0.5">{formatSignedAt(manifest.signedAt)}</p>
              </div>
            {/if}
          </div>

          {#if manifest.assertions.length > 0}
            <div>
              <h3 class="text-xs text-flint dark:text-flint-light uppercase tracking-wide mb-2">
                Assertions
                <span class="normal-case ml-1 text-flint/70">({manifest.assertions.length})</span>
              </h3>
              <div class="space-y-2" role="list" aria-label="C2PA assertions">
                {#each manifest.assertions as assertion (assertion.label)}
                  <div class="bg-gray-100 dark:bg-obsidian/50 rounded-md p-3" role="listitem">
                    <p class="text-xs font-mono text-lapis dark:text-lapis mb-1 break-all">{assertion.label}</p>
                    <pre class="text-xs text-flint dark:text-flint-light whitespace-pre-wrap break-words leading-relaxed">{assertion.value}</pre>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>

      {:else if checked}
        <section class="px-5 py-4" aria-labelledby="c2pa-heading">
          <div class="flex items-center gap-3 mb-3">
            <h2 id="c2pa-heading" class="text-sm font-medium text-text-light dark:text-quartz">C2PA Credentials</h2>
            <span class="text-xs font-medium px-2 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light border border-border-light dark:border-border-dark">
              Not Found
            </span>
          </div>
          <p class="text-sm text-flint dark:text-flint-light">
            No C2PA Content Credentials found in
            <span class="text-text-light dark:text-quartz">{fileName}</span>.
            This file has not been signed with C2PA provenance data.
          </p>
        </section>
      {/if}

      <!-- ── Jura Trace Watermark Detection ────────────────────────── -->
      {#if result.watermarkExtractResult}
        {@const wm = result.watermarkExtractResult}
        <section
          class="px-5 py-4 border-t border-border-light dark:border-border-dark"
          aria-labelledby="watermark-detect-heading"
        >
          <div class="flex items-center gap-3 mb-3">
            <h2
              id="watermark-detect-heading"
              class="text-sm font-medium text-text-light dark:text-quartz"
            >
              Jura Trace Watermark
            </h2>
            <span
              class="text-xs font-medium px-2 py-0.5 rounded border
                     {wm.hasWatermark
                       ? 'bg-malachite/15 text-malachite dark:text-malachite-light border-malachite/30'
                       : 'bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light border-border-light dark:border-border-dark'}"
            >
              {wm.hasWatermark ? 'Detected' : 'Not Found'}
            </span>
          </div>

          {#if wm.hasWatermark}
            <!-- Found: show extracted payload + confidence -->
            <div class="space-y-3">
              <div class="flex items-center gap-2">
                <span
                  class="flex-shrink-0 w-2 h-2 rounded-full bg-malachite"
                  aria-hidden="true"
                ></span>
                <p class="text-sm text-malachite dark:text-malachite-light font-medium">
                  Watermark detected — this file carries Jura Trace provenance data.
                </p>
              </div>

              {#if wm.extractedPayload}
                <div class="rounded-md bg-gray-50 dark:bg-obsidian/50 border border-border-light dark:border-border-dark px-4 py-3">
                  <p class="text-xs text-flint dark:text-flint-light uppercase tracking-wide mb-1">Extracted Institution</p>
                  <p class="text-sm text-text-light dark:text-quartz font-mono break-all">{wm.extractedPayload}</p>
                </div>
              {/if}

              <div class="grid grid-cols-2 gap-4 text-xs">
                <div>
                  <span class="text-flint dark:text-flint-light">Confidence</span>
                  <p class="text-text-light dark:text-quartz tabular-nums mt-0.5">
                    {Math.round(wm.confidence * 100)}%
                  </p>
                </div>
                {#if wm.extractedHex}
                  <div>
                    <span class="text-flint dark:text-flint-light">Hex Payload</span>
                    <p class="text-text-light dark:text-quartz font-mono text-xs mt-0.5 break-all">{wm.extractedHex}</p>
                  </div>
                {/if}
              </div>

              <p class="text-xs text-flint dark:text-flint-light leading-relaxed">
                This watermark was embedded using Jura Trace. The extracted institution name can be
                used to verify the asset's provenance against the originating collection record.
              </p>
            </div>

          {:else}
            <!-- Not found -->
            <p class="text-sm text-flint dark:text-flint-light leading-relaxed">
              No Jura Trace invisible watermark was detected in this file. The file may originate
              from outside the Jura Archive workflow, or the watermark may have been removed or
              degraded by subsequent processing.
            </p>
          {/if}

          {#if !wm.success && wm.message}
            <p class="mt-2 text-xs text-amber dark:text-amber-light">
              Note: {wm.message}
            </p>
          {/if}
        </section>
      {/if}

      <!-- ── Video Analysis ──────────────────────────────────────────── -->
      {#if result.videoDeepfakeResult?.success}
        {@const vd = result.videoDeepfakeResult}
        <section
          class="px-5 py-4 border-t border-border-light dark:border-border-dark"
          aria-labelledby="video-analysis-heading"
        >
          <div class="flex items-center gap-3 mb-3">
            <h2
              id="video-analysis-heading"
              class="text-sm font-medium text-text-light dark:text-quartz"
            >
              Video Analysis
            </h2>
            <span
              class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium
                {vd.aggregateVerdict === 'authentic' ? 'bg-malachite/10 text-malachite' :
                 vd.aggregateVerdict === 'synthetic' ? 'bg-cinnabar/10 text-cinnabar' :
                 'bg-amber/10 text-amber'}"
            >
              {vd.aggregateVerdict === 'authentic' ? 'Authentic' :
               vd.aggregateVerdict === 'synthetic' ? 'Synthetic' : 'Inconclusive'}
            </span>
            <span class="text-xs text-flint dark:text-flint-light">
              Score: {(vd.aggregateScore * 100).toFixed(0)}%
              &middot; {vd.framesAnalysed} frame{vd.framesAnalysed !== 1 ? 's' : ''} analysed
              {#if vd.duration != null}
                &middot; {Math.floor(vd.duration / 60)}:{String(Math.round(vd.duration % 60)).padStart(2, '0')} duration
              {/if}
            </span>
          </div>

          <!-- Temporal consistency -->
          {#if vd.temporalAvailable}
            <p class="text-xs text-flint dark:text-flint-light mb-3">
              {#if (vd.temporalNoiseDrift ?? 0) > 0.4 || (vd.temporalSpectralDrift ?? 0) > 0.4 || (vd.temporalLbpDrift ?? 0) > 0.4}
                Frame-to-frame drift detected in forensic features.
              {:else}
                Temporal signals: stable across frames.
              {/if}
            </p>
          {/if}

          <!-- Frame timeline with score badges -->
          {#if vd.frameResults.length > 0}
            {@const frames = result.videoFramesResult?.frames ?? []}
            <div
              class="grid gap-2 mb-3"
              style="grid-template-columns: repeat({Math.min(vd.frameResults.length, 6)}, 1fr);"
              role="list"
              aria-label="Video frame deepfake analysis timeline"
            >
              {#each vd.frameResults as fr, i (i)}
                {@const verdictColour = fr.verdictLevel === 'authentic' ? 'malachite' :
                  fr.verdictLevel === 'synthetic' ? 'cinnabar' : 'amber'}
                {@const isExpanded = expandedFrameIndex === i}
                <div
                  class="relative rounded-md overflow-hidden border transition-colors
                    {isExpanded ? 'border-lapis ring-1 ring-lapis/30' : 'border-border-light dark:border-border-dark'}
                    bg-gray-100 dark:bg-obsidian cursor-pointer"
                  role="listitem"
                >
                  <!-- Clickable thumbnail + badge -->
                  <button
                    type="button"
                    class="w-full text-left"
                    aria-expanded={isExpanded}
                    aria-controls="frame-detail-{i}"
                    onclick={() => { expandedFrameIndex = isExpanded ? null : i; }}
                  >
                    {#if frames[i]}
                      <div class="aspect-video">
                        <img
                          src={blobs.url(frames[i], 'image/jpeg')}
                          alt="Frame {i + 1}: {fr.verdictLevel} (score {(fr.score * 100).toFixed(0)}%)"
                          class="w-full h-full object-cover"
                        />
                      </div>
                    {:else}
                      <div class="aspect-video flex items-center justify-center">
                        <span class="text-xs text-flint dark:text-flint-light">F{i + 1}</span>
                      </div>
                    {/if}

                    <!-- Score badge -->
                    <span
                      class="absolute bottom-1 right-1 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium"
                      style="color: {fr.verdictLevel === 'authentic' ? 'rgb(76, 175, 80)' :
                        fr.verdictLevel === 'synthetic' ? 'rgb(211, 47, 47)' : 'rgb(255, 160, 0)'};
                        background: {fr.verdictLevel === 'authentic' ? 'rgba(76,175,80,0.15)' :
                        fr.verdictLevel === 'synthetic' ? 'rgba(211,47,47,0.15)' : 'rgba(255,160,0,0.15)'};"
                    >
                      {(fr.score * 100).toFixed(0)}%
                    </span>

                    <!-- Score bar -->
                    <div class="h-1.5 w-full bg-graphite/20">
                      <div
                        class="h-full transition-all"
                        style="width: {Math.max(2, fr.score * 100)}%;
                          background-color: {fr.verdictLevel === 'authentic' ? 'rgb(76, 175, 80)' :
                            fr.verdictLevel === 'synthetic' ? 'rgb(211, 47, 47)' : 'rgb(255, 160, 0)'};"
                      ></div>
                    </div>
                  </button>

                  <!-- Expanded frame detail accordion -->
                  {#if isExpanded}
                    <div
                      id="frame-detail-{i}"
                      class="p-3 border-t border-border-light dark:border-border-dark bg-white dark:bg-graphite space-y-2"
                    >
                      <div class="flex items-center justify-between">
                        <span class="text-xs font-medium text-text-light dark:text-quartz">
                          Frame {fr.frameIndex + 1} at {fr.timestamp.toFixed(1)}s
                        </span>
                        <span class="text-xs tabular-nums {fr.verdictLevel === 'authentic' ? 'text-malachite' : fr.verdictLevel === 'synthetic' ? 'text-cinnabar' : 'text-amber'}">
                          {fr.verdictLevel.charAt(0).toUpperCase() + fr.verdictLevel.slice(1)} ({(fr.score * 100).toFixed(1)}%)
                        </span>
                      </div>

                      {#if fr.classifierAvailable && fr.classifierScore != null}
                        <div class="text-[10px] text-flint dark:text-flint-light">
                          GBM classifier: {(fr.classifierScore * 100).toFixed(1)}%
                        </div>
                      {/if}

                      <!-- Per-frame heatmap -->
                      {#if fr.heatmapBase64}
                        <div class="rounded overflow-hidden border border-border-light dark:border-border-dark">
                          <img
                            src={blobs.url(fr.heatmapBase64, 'image/png')}
                            alt="Frequency spectrum heatmap for frame {fr.frameIndex + 1}"
                            class="w-full max-h-48 object-contain"
                          />
                        </div>
                      {/if}

                      <!-- Signals list -->
                      {#if fr.signals.length > 0}
                        <div class="space-y-1">
                          <span class="text-[10px] font-medium text-flint dark:text-flint-light uppercase tracking-wider">Signals</span>
                          {#each fr.signals as signal}
                            <div class="flex items-center gap-2 text-[10px]">
                              <span
                                class="w-1.5 h-1.5 rounded-full flex-shrink-0"
                                style="background: {signal.triggered ? 'rgb(211, 47, 47)' : 'rgb(76, 175, 80)'};"
                              ></span>
                              <span class="text-flint dark:text-flint-light flex-1">{signal.name}</span>
                              <span class="tabular-nums text-text-light dark:text-quartz">{(signal.weight * 100).toFixed(0)}%</span>
                            </div>
                          {/each}
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>

            <!-- Colour key -->
            <div class="flex items-center gap-4 text-[10px] text-flint dark:text-flint-light mb-2">
              <span class="flex items-center gap-1">
                <span class="inline-block w-2 h-2 rounded-full" style="background: rgb(76, 175, 80);"></span>
                Authentic (&lt;35%)
              </span>
              <span class="flex items-center gap-1">
                <span class="inline-block w-2 h-2 rounded-full" style="background: rgb(255, 160, 0);"></span>
                Inconclusive (35-60%)
              </span>
              <span class="flex items-center gap-1">
                <span class="inline-block w-2 h-2 rounded-full" style="background: rgb(211, 47, 47);"></span>
                Synthetic (&gt;60%)
              </span>
            </div>
          {/if}

          <!-- Summary -->
          <p class="text-xs text-flint dark:text-flint-light leading-relaxed">
            {vd.message}
          </p>
        </section>

      {:else if result.videoDeepfakeResult && !result.videoDeepfakeResult.success}
        <section class="px-5 py-4 border-t border-border-light dark:border-border-dark">
          <h2 class="text-sm font-medium text-text-light dark:text-quartz mb-2">
            Video Analysis
          </h2>
          <p class="text-xs text-cinnabar">
            {result.videoDeepfakeResult.message}
          </p>
        </section>
      {/if}

      <!-- ── Video Frame Thumbnails (when no deepfake analysis) ────── -->
      {#if !result.videoDeepfakeResult && result.videoFramesResult?.success && result.videoFramesResult.frames.length > 0}
        {@const vf = result.videoFramesResult}
        <section
          class="px-5 py-4 border-t border-border-light dark:border-border-dark"
          aria-labelledby="video-frames-heading"
        >
          <div class="flex items-center gap-3 mb-3">
            <h2
              id="video-frames-heading"
              class="text-sm font-medium text-text-light dark:text-quartz"
            >
              Video Frame Samples
            </h2>
            <span class="text-xs text-flint dark:text-flint-light">
              {vf.count} frame{vf.count !== 1 ? 's' : ''}
              {#if vf.duration != null}
                &middot; {Math.floor(vf.duration / 60)}:{String(Math.round(vf.duration % 60)).padStart(2, '0')} duration
              {/if}
            </span>
          </div>

          <div
            class="grid gap-2"
            style="grid-template-columns: repeat({Math.min(vf.frames.length, 4)}, 1fr);"
            role="list"
            aria-label="Representative video frame thumbnails"
          >
            {#each vf.frames as frame, i (i)}
              <div
                class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian aspect-video"
                role="listitem"
              >
                <img
                  src={blobs.url(frame, 'image/jpeg')}
                  alt="Frame {i + 1} of {vf.frames.length} from video"
                  class="w-full h-full object-cover"
                />
              </div>
            {/each}
          </div>

          <p class="mt-2 text-xs text-flint dark:text-flint-light leading-relaxed">
            Representative frames sampled evenly across the video duration. Inspect for visual
            discontinuities, splice artefacts, or temporal inconsistencies.
          </p>
        </section>
      {/if}

      <!-- ── Transcription ──────────────────────────────────────────── -->
      {#if result.transcriptionResult?.success}
        {@const tr = result.transcriptionResult}
        <section
          class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5"
          aria-labelledby="transcription-heading"
        >
          <div class="flex items-center justify-between mb-4">
            <h3
              id="transcription-heading"
              class="font-serif text-base font-semibold text-obsidian dark:text-white"
            >
              Transcription
            </h3>
            {#if tr.language}
              <span class="text-xs text-flint dark:text-flint-light">
                Language: <span class="font-medium text-obsidian dark:text-white">{tr.language.toUpperCase()}</span>
                {#if tr.languageProbability != null}
                  <span class="ml-1 text-flint dark:text-flint-light">({(tr.languageProbability * 100).toFixed(1)}%)</span>
                {/if}
                {#if tr.duration != null}
                  <span class="mx-1">·</span>
                  <span>{tr.duration.toFixed(1)}s</span>
                {/if}
                <span class="mx-1">·</span>
                <span>Model: {tr.modelSize}</span>
              </span>
            {/if}
          </div>

          <!-- Full transcript text -->
          {#if tr.text}
            <div
              class="mb-4 max-h-48 overflow-y-auto rounded-md border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian p-3"
            >
              <p class="text-sm text-obsidian dark:text-white leading-relaxed whitespace-pre-wrap">{tr.text}</p>
            </div>
          {/if}

          <!-- Timestamped segments -->
          {#if tr.segments && tr.segments.length > 0}
            <details class="group">
              <summary class="cursor-pointer text-xs font-medium text-lapis hover:underline">
                Show {tr.segments.length} timestamped segment{tr.segments.length !== 1 ? 's' : ''}
              </summary>
              <div class="mt-2 max-h-64 overflow-y-auto space-y-1">
                {#each tr.segments as seg, i}
                  <div class="flex gap-3 py-1 px-2 rounded text-xs {i % 2 === 0 ? 'bg-gray-50 dark:bg-obsidian/50' : ''}">
                    <span class="flex-shrink-0 font-mono text-flint dark:text-flint-light w-24">
                      {seg.start.toFixed(1)}s – {seg.end.toFixed(1)}s
                    </span>
                    <span class="text-obsidian dark:text-white">{seg.text}</span>
                  </div>
                {/each}
              </div>
            </details>
          {/if}
        </section>
      {:else if (result.contentType === 'audio' || result.contentType === 'video') && !result.transcriptionResult}
        <section
          class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5"
          aria-labelledby="transcription-unavailable-heading"
        >
          <h3
            id="transcription-unavailable-heading"
            class="font-serif text-base font-semibold text-obsidian dark:text-white mb-2"
          >
            Transcription
          </h3>
          <p class="text-xs text-flint dark:text-flint-light">
            Speech transcription model not available. Install <code class="bg-gray-100 dark:bg-obsidian px-1 rounded">faster-whisper</code> in the Analysis Engine to enable audio transcription.
          </p>
        </section>
      {/if}

      <!-- ── Claim Check (from transcription) ──────────────────────── -->
      {#if result.claimCheckResult}
        {@const cc = result.claimCheckResult}
        <section
          class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5"
          aria-labelledby="claim-check-heading"
        >
          <div class="flex items-center justify-between mb-4">
            <h3
              id="claim-check-heading"
              class="font-serif text-base font-semibold text-obsidian dark:text-white"
            >
              Claim Verification
            </h3>
            <span class="text-xs px-2 py-0.5 rounded font-medium
              {cc.overallVerdict === 'supported' ? 'bg-malachite/10 text-malachite' :
               cc.overallVerdict === 'disputed' ? 'bg-cinnabar/10 text-cinnabar' :
               cc.overallVerdict === 'mixed' ? 'bg-amber/10 text-amber' :
               'bg-graphite/20 text-flint dark:text-flint-light'}">
              {cc.overallVerdict.charAt(0).toUpperCase() + cc.overallVerdict.slice(1)}
            </span>
          </div>

          <p class="text-xs text-flint dark:text-flint-light mb-3">{cc.summary}</p>

          {#if cc.claims.length > 0}
            <div class="space-y-2">
              {#each cc.claims as claim}
                <div class="rounded-md border border-border-light dark:border-border-dark p-3">
                  <div class="flex items-start justify-between gap-2 mb-1">
                    <p class="text-xs font-medium text-obsidian dark:text-white">{claim.claim}</p>
                    <span class="flex-shrink-0 text-xs px-1.5 py-0.5 rounded
                      {claim.verdict === 'supported' ? 'bg-malachite/10 text-malachite' :
                       claim.verdict === 'disputed' ? 'bg-cinnabar/10 text-cinnabar' :
                       'bg-graphite/20 text-flint dark:text-flint-light'}">
                      {claim.verdict}
                    </span>
                  </div>
                  <p class="text-xs text-flint dark:text-flint-light">{claim.explanation}</p>
                  {#if claim.confidence > 0}
                    <div class="mt-1 flex items-center gap-1">
                      <div class="h-1 w-16 rounded-full bg-gray-200 dark:bg-obsidian">
                        <div
                          class="h-1 rounded-full {claim.verdict === 'supported' ? 'bg-malachite' : claim.verdict === 'disputed' ? 'bg-cinnabar' : 'bg-amber'}"
                          style="width: {claim.confidence * 100}%"
                        ></div>
                      </div>
                      <span class="text-[10px] text-flint dark:text-flint-light">{(claim.confidence * 100).toFixed(0)}%</span>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}

          <p class="mt-3 text-[10px] text-flint dark:text-flint-light">
            Model: {cc.modelUsed} · {cc.methodology}
          </p>
        </section>
      {/if}

      </div>
      {/if}
      <!-- End Technical Details -->

      {/if}
      <!-- End Detail view -->

    </div>

    <!-- ── Methodology Panel ────────────────────────────────────────── -->
    <MethodologyPanel {result} {sidecarHealth} />

  {:else if !checked && !loading}

    <!-- Pre-verification idle state -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-8 text-center">
      <p class="text-flint dark:text-flint-light text-sm">
        {#if activeTab === 'file'}
          Drop a file above to analyse its metadata, compression artefacts, and C2PA Content Credentials.
        {:else if activeTab === 'batch'}
          Drop multiple files above to queue them for batch verification.
        {:else}
          Enter a URL above to download and verify content from the web.
        {/if}
      </p>
    </div>

  {/if}

</div>

<!-- ── False Positive Modal ──────────────────────────────────────── -->
{#if showFalsePositiveModal}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="fp-modal-title"
    tabindex="-1"
    onkeydown={(e) => { if (e.key === 'Escape') { showFalsePositiveModal = false; } }}
    onclick={(e) => { if (e.target === e.currentTarget) showFalsePositiveModal = false; }}
  >
    <div class="bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-lg shadow-xl w-full max-w-md mx-4 p-6">

      {#if fpSubmitted}
        <!-- Success state -->
        <div class="flex flex-col items-center gap-3 py-4 text-center">
          <div class="w-10 h-10 rounded-full bg-malachite/15 border border-malachite/30 flex items-center justify-center" aria-hidden="true">
            <svg class="w-5 h-5 text-malachite" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <p class="text-sm font-medium text-text-light dark:text-quartz">Report submitted</p>
          <p class="text-xs text-flint dark:text-flint-light">Thank you. This helps improve detection accuracy.</p>
        </div>

      {:else}
        <!-- Form -->
        <h2 id="fp-modal-title" class="text-lg font-medium text-text-light dark:text-quartz mb-1">
          Report False Positive
        </h2>
        <p class="text-sm text-flint dark:text-flint-light mb-5">
          If this result appears to be a false positive, let us know why. Reports help calibrate the detection system.
        </p>

        <!-- Reason code -->
        <fieldset class="mb-4">
          <legend class="block text-xs font-medium text-flint dark:text-flint-light mb-2">
            Reason <span class="text-cinnabar" aria-hidden="true">*</span>
            <span class="sr-only">(required)</span>
          </legend>
          <div class="space-y-2">
            {#each [
              { code: 'modern_codec', label: 'Modern codec (AVIF/WebP)', description: 'Modern compression introduces patterns that resemble manipulation artefacts' },
              { code: 'social_media', label: 'Social media re-upload', description: 'Re-encoding from social platforms degrades metadata and introduces artefacts' },
              { code: 'scanner', label: 'Scanner output', description: 'Scanned documents or film produce noise profiles that trigger false detections' },
              { code: 'computational_photography', label: 'Computational photography (HDR/Night Mode)', description: 'Multi-frame compositing and tone-mapping from mobile cameras' },
              { code: 'other', label: 'Other', description: 'Another reason not listed above' },
            ] as opt}
              <label
                class="flex items-start gap-3 rounded-md px-3 py-2.5 cursor-pointer transition-colors duration-150
                       border {fpReasonCode === opt.code
                         ? 'border-lapis/50 bg-lapis/8 dark:bg-lapis/10'
                         : 'border-border-light dark:border-border-dark hover:border-lapis/30 hover:bg-gray-50 dark:hover:bg-graphite-light/20'}"
              >
                <input
                  type="radio"
                  name="fp-reason"
                  value={opt.code}
                  bind:group={fpReasonCode}
                  class="mt-0.5 flex-shrink-0 accent-lapis focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1"
                />
                <div class="min-w-0">
                  <span class="text-sm text-text-light dark:text-quartz leading-snug block">{opt.label}</span>
                  <span class="text-xs text-flint dark:text-flint-light leading-relaxed">{opt.description}</span>
                </div>
              </label>
            {/each}
          </div>
        </fieldset>

        <!-- Optional note -->
        <div class="mb-5">
          <label for="fp-note" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
            Additional notes <span class="text-flint/50">(optional)</span>
          </label>
          <textarea
            id="fp-note"
            class="w-full h-20 px-3 py-2 text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                   text-text-light dark:text-quartz placeholder:text-flint/40 resize-none
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
            placeholder="e.g. AVIF file exported from Lightroom, high ISO scan from Epson V600..."
            maxlength={500}
            bind:value={fpReasonNote}
          ></textarea>
          <p class="text-xs text-flint/50 mt-1 text-right">{fpReasonNote.length} / 500</p>
        </div>

        <!-- Actions -->
        <div class="flex gap-3 justify-end">
          <button
            class="px-4 py-2.5 min-h-[44px] text-sm text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
            onclick={() => { showFalsePositiveModal = false; fpReasonNote = ''; fpReasonCode = 'modern_codec'; }}
            disabled={fpSubmitting}
          >
            Cancel
          </button>
          <button
            class="px-4 py-2.5 min-h-[44px] text-sm bg-lapis hover:bg-lapis-dark dark:hover:bg-lapis-light text-white rounded transition-colors
                   disabled:opacity-50 disabled:cursor-not-allowed
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
            onclick={handleFalsePositiveSubmit}
            disabled={fpSubmitting}
          >
            {fpSubmitting ? 'Submitting...' : 'Submit Report'}
          </button>
        </div>
      {/if}

    </div>
  </div>
{/if}

<!-- ── Analyst Declaration Modal ──────────────────────────────────── -->
{#if showReportModal}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="report-modal-title"
    tabindex="-1"
    onkeydown={(e) => { if (e.key === 'Escape') showReportModal = false; }}
    onclick={(e) => { if (e.target === e.currentTarget) showReportModal = false; }}
  >
    <div class="bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-lg shadow-xl w-full max-w-lg mx-4 p-6">

      <!-- Modal heading -->
      <div class="mb-5">
        <h2 id="report-modal-title" class="text-lg font-medium text-text-light dark:text-quartz">Export Forensic Report</h2>
        <p class="text-sm text-flint dark:text-flint-light mt-1">
          Add your details to the report declaration. All fields are optional — leave blank to export without attribution.
        </p>
      </div>

      <!-- Field grid -->
      <div class="space-y-4">

        <!-- Row 1: Analyst name + Date -->
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="decl-analyst-name" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
              Analyst Name
              <span class="text-flint/50 font-normal ml-1">(optional)</span>
            </label>
            <input
              id="decl-analyst-name"
              type="text"
              class="w-full px-3 py-2.5 min-h-[44px] text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                     text-text-light dark:text-quartz placeholder:text-flint/40
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
              placeholder="e.g. Niamh Farrell"
              autocomplete="name"
              bind:value={analystName}
            />
          </div>
          <div>
            <label for="decl-analysis-date" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
              Date of Analysis
            </label>
            <input
              id="decl-analysis-date"
              type="text"
              class="w-full px-3 py-2.5 min-h-[44px] text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                     text-text-light dark:text-quartz placeholder:text-flint/40
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
              bind:value={analystDate}
            />
          </div>
        </div>

        <!-- Row 2: Organisation -->
        <div>
          <label for="decl-organisation" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
            Organisation
            <span class="text-flint/50 font-normal ml-1">(optional)</span>
          </label>
          <input
            id="decl-organisation"
            type="text"
            class="w-full px-3 py-2.5 min-h-[44px] text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                   text-text-light dark:text-quartz placeholder:text-flint/40
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
            placeholder="e.g. Clarke &amp; Associates Solicitors"
            autocomplete="organization"
            bind:value={analystOrg}
          />
          <p class="text-xs text-flint/50 mt-1">Name and organisation are remembered for your next export.</p>
        </div>

        <!-- Row 3: Case reference -->
        <div>
          <label for="decl-case-ref" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
            Case Reference
            <span class="text-flint/50 font-normal ml-1">(optional, not saved)</span>
          </label>
          <input
            id="decl-case-ref"
            type="text"
            class="w-full px-3 py-2.5 min-h-[44px] text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                   text-text-light dark:text-quartz placeholder:text-flint/40
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
            placeholder="e.g. CF-2026-0047"
            bind:value={analystCaseRef}
          />
        </div>

        <!-- Row 4: Analyst note (existing) -->
        <div>
          <label for="analyst-note" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
            Analyst Note
            <span class="text-flint/50 font-normal ml-1">(optional, max 500 chars)</span>
          </label>
          <textarea
            id="analyst-note"
            class="w-full h-20 px-3 py-2 text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                   text-text-light dark:text-quartz placeholder:text-flint/40 resize-none
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
            placeholder="e.g. Initial assessment suggests authentic capture with minor metadata gaps..."
            maxlength={500}
            bind:value={analystNote}
          ></textarea>
          <p class="text-xs text-flint/50 mt-1 text-right" aria-live="polite" aria-atomic="true">
            <span class="sr-only">Characters used: </span>{analystNote.length} / 500
          </p>
        </div>

      </div>

      <!-- Tier hint for Community plan -->
      {#if licenceTier === 'community'}
        <p class="mt-4 text-xs text-lapis dark:text-lapis-light bg-lapis/8 dark:bg-lapis/10 border border-lapis/20 rounded px-3 py-2.5">
          Professional plan includes branded reports — upgrade for your organisation's logo and sector-specific templates.
        </p>
      {/if}

      <!-- Actions -->
      <div class="flex gap-3 justify-end mt-5">
        <button
          type="button"
          class="px-4 py-2.5 min-h-[44px] text-sm text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          onclick={() => { showReportModal = false; }}
          disabled={exportingReport}
        >
          Cancel
        </button>
        <button
          type="button"
          class="px-4 py-2.5 min-h-[44px] text-sm bg-lapis hover:bg-lapis-dark dark:hover:bg-lapis-light text-white rounded transition-colors
                 disabled:opacity-50 disabled:cursor-not-allowed
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
          onclick={handleExportReport}
          disabled={exportingReport}
        >
          {exportingReport ? 'Generating...' : 'Export Report'}
        </button>
      </div>

    </div>
  </div>
{/if}
