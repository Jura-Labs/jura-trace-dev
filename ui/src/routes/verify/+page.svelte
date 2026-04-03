<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { writable } from 'svelte/store';
  import { verifyFile, verifyUrl, checkSidecarHealth, openBatchFileDialog, markFalsePositive, parseAppError, getLicenceTier, extractTextFromImage, calculateSunPosition, estimateShadowTime, checkHistoricalWeather, analyseSeasonalIndicators, analyseDiffusionArtefacts, analyseRoi, saveAnnotation, getAnnotations, deleteAnnotationApi } from '$lib/api';
  import { getTrustLevel, SEVERITY_CONFIG, formatFileSize, formatDuration } from '$lib/types';
  import { createBlobTracker } from '$lib/blob';
  import type { Annotation, AnnotationData, LicenceTier, VerificationResult, AnomalyFinding, SidecarHealth, VerifyMode, BatchItem, SegmentedElaResult, ShadowConsistencyResult, ColourTemperatureResult, SpliceBoundaryResult, ClipDetectionResult, RagClaimResult, VideoDeepfakeResult, FrameDeepfakeResult, TranscriptionResult, ClaimCheckResult, SolarPosition, TimeEstimate, WeatherCheckResult, SeasonalIndicatorsResult, DiffusionArtefactsResult, RoiAnalysisResult } from '$lib/types';
  import VerdictSummary from '$lib/components/VerdictSummary.svelte';
  import SimpleVerdict from '$lib/components/SimpleVerdict.svelte';
  import MethodologyPanel from '$lib/components/MethodologyPanel.svelte';
  import InspectionChecklist from '$lib/components/InspectionChecklist.svelte';
  import SignalAgreement from '$lib/components/SignalAgreement.svelte';
  import ContextualHelpLink from '$lib/components/ContextualHelpLink.svelte';
  import { generateTrustReport } from '$lib/pdf';
  import type { ReportContext, ReportFormat } from '$lib/pdf';
  import { exportCaseZip } from '$lib/zip';
  import { getVersion } from '$lib/api';
  import { saveVerifySession, restoreVerifySession, clearVerifySession } from '$lib/stores/verifySession';

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

  /** Analysis mode label shown during progress. */
  const estimatedTime = $derived(
    verifyMode === 'archival' ? 'Running archival analysis…'
    : verifyMode === 'deep' ? 'Running deep analysis…'
    : 'Running standard analysis…'
  );
  let showTechnicalDetails = $state(false);
  let showInvestigatePanel = $state(false);
  let showSignalAgreement = $state(false);
  let showInspectionChecklist = $state(false);
  let showRegionAnalysis = $state(false);
  let expandedFrameIndex = $state<number | null>(null);

  // ── Simple / Expert view mode ─────────────────────────────────────
  // 'simple' = plain-English verdict card (default for all users)
  // 'expert' = full forensic breakdown (accessible via "See Detailed Analysis")
  let viewMode = $state<'simple' | 'expert'>('simple');

  // ── Side-by-side comparison mode state ───────────────────────────
  /** Whether comparison mode is active (second image loaded alongside the primary). */
  let comparisonMode = $state(false);
  /** Blob URL of the comparison image, or null when comparison mode is inactive. */
  let comparisonImageUrl = $state<string | null>(null);
  /** File name of the comparison image, for the overlay label. */
  let comparisonFileName = $state<string | null>(null);

  /** Open a file picker and load a comparison image. */
  async function handleLoadComparison() {
    if (inTauri) {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
          multiple: false,
          title: 'Select Comparison Image',
          filters: [
            { name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'avif', 'bmp'] },
          ],
        });
        if (selected && typeof selected === 'string') {
          const { convertFileSrc } = await import('@tauri-apps/api/core');
          comparisonImageUrl = convertFileSrc(selected);
          comparisonFileName = selected.split('/').pop() ?? selected.split('\\').pop() ?? selected;
          comparisonMode = true;
        }
      } catch {
        // Tauri unavailable — fall through to browser fallback
      }
      return;
    }
    // Browser fallback
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) return;
      if (comparisonImageUrl) URL.revokeObjectURL(comparisonImageUrl);
      comparisonImageUrl = URL.createObjectURL(file);
      comparisonFileName = file.name;
      comparisonMode = true;
    };
    input.click();
  }

  /** Exit comparison mode and revoke the comparison blob URL. */
  function closeComparison() {
    comparisonMode = false;
    if (comparisonImageUrl) {
      // Only revoke blob: URLs — Tauri asset:// URLs are managed by Tauri
      if (comparisonImageUrl.startsWith('blob:')) URL.revokeObjectURL(comparisonImageUrl);
      comparisonImageUrl = null;
    }
    comparisonFileName = null;
  }

  // ── Raw scores preference state ───────────────────────────────────
  /** When true, show raw numerical detector scores alongside traffic-light badges. */
  let showRawScores = $state(false);

  // ── ELA overlay state ────────────────────────────────────────────
  let showElaOverlay = $state(false);
  let elaOpacity = $state(60);
  let elaBlendMode = $state<'normal' | 'multiply' | 'difference'>('normal');

  // ── Image zoom state ─────────────────────────────────────────────
  let showZoomModal = $state(false);
  let zoomLevel = $state(1);

  function handleImageClick() {
    showZoomModal = true;
    zoomLevel = 1;
  }

  function handleZoomWheel(e: WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -0.25 : 0.25;
    zoomLevel = Math.min(Math.max(zoomLevel + delta, 0.5), 5);
  }

  // ── Image inspection filter state ────────────────────────────────
  type InspectFilter = 'none' | 'grayscale' | 'invert' | 'contrast' | 'saturate' | 'edges';
  let activeFilter = $state<InspectFilter>('none');
  let brightness = $state(100);
  let contrast = $state(100);

  // ── Colour channel separation state ──────────────────────────────
  let activeChannel = $state<'none' | 'r' | 'g' | 'b' | 'rg' | 'rb' | 'gb'>('none');
  /** Blob URL of the channel-separated image, or null when no channel is active. */
  let channelImageUrl = $state<string | null>(null);

  /**
   * Applies colour channel separation to the preview image using an off-screen
   * canvas. Returns a data URL of the resulting greyscale channel image.
   * The caller is responsible for revoking any previous blob URL.
   */
  function applyChannelSeparation(imgElement: HTMLImageElement, channel: string): string {
    const canvas = document.createElement('canvas');
    canvas.width = imgElement.naturalWidth;
    canvas.height = imgElement.naturalHeight;
    const ctx = canvas.getContext('2d')!;
    ctx.drawImage(imgElement, 0, 0);
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const data = imageData.data;

    for (let i = 0; i < data.length; i += 4) {
      const r = data[i], g = data[i + 1], b = data[i + 2];
      let val: number;
      switch (channel) {
        case 'r':  val = r; break;
        case 'g':  val = g; break;
        case 'b':  val = b; break;
        case 'rg': val = Math.abs(r - g); break;
        case 'rb': val = Math.abs(r - b); break;
        case 'gb': val = Math.abs(g - b); break;
        default:   val = 0;
      }
      // Display as greyscale
      data[i] = data[i + 1] = data[i + 2] = val;
    }
    ctx.putImageData(imageData, 0, 0);
    return canvas.toDataURL('image/png');
  }

  /**
   * Toggles the active channel: if the same channel is clicked again, resets
   * to 'none'. Otherwise loads the preview image element and computes the
   * channel image, storing it as a blob URL via the existing blob tracker.
   */
  function toggleChannel(channel: typeof activeChannel) {
    if (activeChannel === channel) {
      // Reset
      activeChannel = 'none';
      if (channelImageUrl) {
        URL.revokeObjectURL(channelImageUrl);
        channelImageUrl = null;
      }
      return;
    }
    activeChannel = channel;

    // Re-compute from the currently displayed preview <img>
    const imgEl = document.querySelector<HTMLImageElement>('img[data-preview="true"]');
    if (!imgEl || !imgEl.complete || imgEl.naturalWidth === 0) {
      // Image not ready — silently ignore
      return;
    }

    if (channelImageUrl) {
      URL.revokeObjectURL(channelImageUrl);
    }

    const dataUrl = applyChannelSeparation(imgEl, channel);
    // Convert data URL to blob URL for CSP compliance
    fetch(dataUrl)
      .then(res => res.blob())
      .then(blob => {
        channelImageUrl = URL.createObjectURL(blob);
      })
      .catch(() => {
        // Fallback: store the data URL directly (CSP may block; handled gracefully)
        channelImageUrl = dataUrl;
      });
  }

  // Reset channel state whenever a new result loads (or is cleared).
  // Reading `result` here registers it as a reactive dependency so
  // this effect re-runs every time result changes.
  $effect(() => {
    void result; // dependency registration
    if (channelImageUrl) {
      URL.revokeObjectURL(channelImageUrl);
      channelImageUrl = null;
    }
    activeChannel = 'none';
  });

  function getFilterStyle(filter: InspectFilter): string {
    const base =
      filter === 'grayscale' ? 'grayscale(100%)' :
      filter === 'invert'    ? 'invert(100%)' :
      filter === 'contrast'  ? 'contrast(300%) brightness(1.2)' :
      filter === 'saturate'  ? 'saturate(500%)' :
      filter === 'edges'     ? 'grayscale(100%) contrast(500%) brightness(0.8)' :
      '';
    const adjustments = `brightness(${brightness}%) contrast(${contrast}%)`;
    return base ? `${base} ${adjustments}` : adjustments;
  }

  function resetInspection() {
    activeFilter = 'none';
    brightness = 100;
    contrast = 100;
    activeChannel = 'none';
    if (channelImageUrl) {
      URL.revokeObjectURL(channelImageUrl);
      channelImageUrl = null;
    }
  }

  // ── Scroll-to-top visibility ──────────────────────────────────────
  let showScrollTop = $state(false);

  // ── Active section for sticky nav highlight ───────────────────────
  let activeSection = $state<string | null>(null);

  // Blob URL tracker — converts base64 data to CSP-safe blob: URLs and
  // revokes them on component destroy to prevent memory leaks.
  const blobs = createBlobTracker();
  onDestroy(() => {
    blobs.revokeAll();
    _unlistenDragDrop?.();
  });

  // ── Tauri environment detection ───────────────────────────────────
  const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // ── Image preview for verified files ─────────────────────────────
  // Uses Tauri's convertFileSrc to create a safe asset:// URL from the
  // local file path. Falls back to null in browser mode.
  let previewUrl = $state<string | null>(null);

  $effect(() => {
    const path = filePath;
    const res = result;
    previewUrl = null;
    if (!path || !res || res.contentType !== 'image' || !inTauri) return;
    import('@tauri-apps/api/core').then(({ convertFileSrc }) => {
      previewUrl = convertFileSrc(path);
    }).catch(() => { /* Tauri API unavailable */ });
  });

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
  let reportFormat = $state<ReportFormat>('standard');
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

  // ── Text extraction state ─────────────────────────────────────────
  let extractingText = $state(false);
  let extractedText = $state<string | null>(null);
  let extractTextError = $state<string | null>(null);

  async function handleExtractText() {
    if (!filePath || extractingText) return;
    extractingText = true;
    extractedText = null;
    extractTextError = null;
    try {
      const text = await extractTextFromImage(filePath);
      if (text !== null) {
        extractedText = text;
      } else {
        extractTextError = 'Text extraction is unavailable. Ensure Ollama is running and llava:7b is pulled.';
      }
    } catch {
      extractTextError = 'Text extraction failed. Check that Ollama is running.';
    } finally {
      extractingText = false;
    }
  }

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
    if (level === 'high') return 'text-malachite dark:text-malachite-light';
    if (level === 'medium') return 'text-amber dark:text-amber-light';
    return 'text-cinnabar dark:text-cinnabar-light';
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
  // Persist view mode preference (simple/expert)
  $effect(() => {
    localStorage.setItem('jura-verify-view-mode', viewMode);
  });

  // Persist analyst note across sessions
  $effect(() => {
    localStorage.setItem('jura-analyst-note', analystNote);
  });

  // Save verify session so results survive navigation to Help/Settings
  $effect(() => {
    if (result && fileName) {
      saveVerifySession({
        result,
        fileName,
        filePath,
        previewDataUrl: previewUrl,
        mode: verifyMode,
        verifiedAt: new Date().toISOString(),
        batchItems: batchItems.length > 0 ? batchItems : undefined,
      });
    }
  });

  onMount(() => {
    // Restore persisted view mode (simple/expert).
    // 'detail' is the legacy value from the old summary/detail system — treat it as 'expert'.
    const savedViewMode = localStorage.getItem('jura-verify-view-mode');
    if (savedViewMode === 'expert' || savedViewMode === 'detail') viewMode = 'expert';

    // Restore persisted raw scores preference
    showRawScores = localStorage.getItem('jura-raw-scores-default') === 'true';

    // Restore persisted investigation mode
    const savedMode = localStorage.getItem('jura-verify-mode');
    if (savedMode === 'standard' || savedMode === 'deep' || savedMode === 'archival') {
      verifyMode = savedMode;
    }

    // Restore persisted analyst declaration fields
    analystName = localStorage.getItem('jura-analyst-name') ?? '';
    analystOrg = localStorage.getItem('jura-analyst-org') ?? '';
    analystNote = localStorage.getItem('jura-analyst-note') ?? '';
    // Populate analysis date with today — user may edit
    analystDate = new Date().toLocaleDateString('en-GB', {
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    });

    // Restore previous verify session if the user navigated away and back
    if (!result) {
      const saved = restoreVerifySession();
      if (saved) {
        result = saved.result;
        fileName = saved.fileName;
        filePath = saved.filePath;
        verifyMode = (saved.mode as VerifyMode) || 'standard';
        checked = true;
        previewUrl = saved.previewDataUrl;
        if (saved.batchItems && saved.batchItems.length > 0) {
          batchItems = saved.batchItems as BatchItem[];
          activeTab = 'batch';
        }
      }
    }

    // Async init — fire-and-forget; cleanup is returned synchronously below
    (async () => {
      sidecarHealth = await checkSidecarHealth();
      appVersion = await getVersion();
      licenceTier = await getLicenceTier();
      // Set up Tauri drag-drop listener for file path access
      await setupTauriDragDrop();
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
        if (showZoomModal) {
          showZoomModal = false;
        } else if (showFalsePositiveModal) {
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

    // ── Scroll-to-top visibility ──────────────────────────────────
    function handleScroll() {
      showScrollTop = window.scrollY > 300;
    }
    window.addEventListener('scroll', handleScroll, { passive: true });

    return () => {
      window.removeEventListener('keydown', handleKey);
      window.removeEventListener('keydown', handleEsc);
      window.removeEventListener('scroll', handleScroll);
    };
  });

  // ── Section scroll helpers ────────────────────────────────────────
  function scrollToSection(id: string) {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
      activeSection = id;
    }
  }

  function scrollToTop() {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  // ── Signal strip derivation ───────────────────────────────────────
  /** Compact pass/fail indicators for each forensic detector — drives the signal strip. */
  const signalIndicators = $derived(() => {
    if (!result) return [];
    const indicators: { id: string; name: string; shortName: string; flagged: boolean; available: boolean }[] = [];
    if (result.elaResult) {
      indicators.push({ id: 'section-ela', name: 'Error Level Analysis', shortName: 'ELA', flagged: result.elaResult.suspicious, available: true });
    }
    if (result.noiseResult) {
      indicators.push({ id: 'section-noise', name: 'Noise Analysis', shortName: 'Noise', flagged: result.noiseResult.suspicious, available: true });
    }
    if (result.copyMoveResult) {
      indicators.push({ id: 'section-copymove', name: 'Copy-Move Detection', shortName: 'Copy-Move', flagged: result.copyMoveResult.suspicious, available: true });
    }
    if (result.deepfakeResult) {
      indicators.push({ id: 'section-deepfake', name: 'AI Generation Detection', shortName: 'AI', flagged: result.deepfakeResult.suspicious, available: true });
    }
    if (result.c2paValid !== null && result.c2paValid !== undefined) {
      indicators.push({ id: 'section-c2pa', name: 'C2PA Credentials', shortName: 'C2PA', flagged: result.c2paValid === false, available: true });
    }
    if (result.exifAnalysis) {
      const highFindings = result.exifAnalysis.findings.filter(f => f.severity === 'high' || f.severity === 'critical');
      indicators.push({ id: 'section-exif', name: 'EXIF Metadata', shortName: 'EXIF', flagged: highFindings.length > 0, available: true });
    }
    if (result.nprResult) {
      indicators.push({ id: 'section-npr', name: 'Neighbouring Pixel Relationship', shortName: 'NPR', flagged: result.nprResult.suspicious, available: true });
    }
    if (result.jpegGhostResult) {
      indicators.push({ id: 'section-jpegGhost', name: 'JPEG Ghost', shortName: 'JPEG Ghost', flagged: result.jpegGhostResult.suspicious, available: true });
    }
    if (result.caResult) {
      indicators.push({ id: 'section-ca', name: 'Chromatic Aberration', shortName: 'CA', flagged: !result.caResult.isConsistent, available: true });
    }
    return indicators;
  });

  // ── Drag and drop ─────────────────────────────────────────────────
  // Tauri v2 does not expose file paths via the browser File API.
  // We use Tauri's onDragDropEvent for the desktop app (gives full paths)
  // and fall back to browser drag-and-drop for dev/browser mode.
  let _unlistenDragDrop: (() => void) | null = null;

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

    // In Tauri, the onDragDropEvent handler fires instead.
    // This browser fallback only works in dev/browser mode.
    const files = e.dataTransfer?.files;
    if (!files?.length) return;

    const file = files[0];
    // Browser File objects don't have .path — use name as fallback (browser dev only)
    const path = (file as any).path || file.name;
    if (path === file.name && typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      // In Tauri but no path — the Tauri drag-drop listener should handle this
      return;
    }
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
          if (paths && paths.length > 0) {
            const filePath = paths[0];
            const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || filePath;
            runFileVerification(filePath, fileName);
          }
        }
      });
    } catch (e) {
      // Not in Tauri — browser drag-and-drop will handle it
    }
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
    clearVerifySession();
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

    clearVerifySession();
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
    showElaOverlay = false;
    elaOpacity = 60;
    elaBlendMode = 'normal';
    showZoomModal = false;
    zoomLevel = 1;
    activeFilter = 'none';
    brightness = 100;
    contrast = 100;
    activeChannel = 'none';
    if (channelImageUrl) {
      URL.revokeObjectURL(channelImageUrl);
      channelImageUrl = null;
    }
    activeSection = null;
    closeComparison();
    clearRoi();
    showGeoPanel = false;
    seasonalResult = null;
    seasonalError = null;
    diffusionResult = null;
    diffusionError = null;
    // Clear annotation state
    annotations = [];
    annotationMode = false;
    isDrawingAnnotation = false;
    drawStart = null;
    currentDrawEnd = null;
    hoveredAnnotationId = null;
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
        reportFormat,
      );
      const ts = Math.floor(Date.now() / 1000);
      const safeName = (fileName ?? 'file').replace(/[^a-zA-Z0-9._-]/g, '_');
      triggerDownload(blob, `jura-report-${safeName}-${ts}.pdf`);
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

  // ── Batch report download ─────────────────────────────────────────
  function downloadBatchReport() {
    const completed = batchItems.filter(i => i.status === 'done' && i.result);
    if (completed.length === 0) return;
    const dateStr = new Date().toISOString().slice(0, 10);
    const csvHeaders = 'Filename,Verdict,Trust Score,Mode,Date\n';
    const csvRows = completed.map(item => {
      const r = item.result!;
      const verdictFromDeepfake = r.deepfakeResult?.verdictLevel;
      const verdict = verdictFromDeepfake ?? (r.overallTrust >= 0.7 ? 'authentic' : r.overallTrust >= 0.4 ? 'inconclusive' : 'synthetic');
      const trust = Math.round(r.overallTrust * 100);
      const mode = r.mode ?? verifyMode;
      const date = item.finishedAt ? new Date(item.finishedAt).toISOString().slice(0, 10) : dateStr;
      const name = item.fileName.replace(/"/g, '""');
      return `"${name}","${verdict}",${trust},"${mode}","${date}"`;
    }).join('\n');
    const blob = new Blob([csvHeaders + csvRows], { type: 'text/csv;charset=utf-8;' });
    const dlUrl = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = dlUrl;
    anchor.download = `jura-batch-results-${dateStr}.csv`;
    anchor.click();
    URL.revokeObjectURL(dlUrl);
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
      {
        label: 'Bing Visual Search',
        href: sourceUrl
          ? `https://www.bing.com/images/search?view=detailv2&iss=sbi&q=imgurl:${encodeURIComponent(sourceUrl)}`
          : 'https://www.bing.com/visualsearch',
        title: sourceUrl
          ? 'Search via Bing Visual Search — this will share the image URL with Microsoft'
          : 'Open Bing Visual Search — upload the file manually',
      },
    ];
  });

  /**
   * Opens a URL in the system default browser.
   * In Tauri, uses the shell plugin to avoid opening inside the webview.
   * In browser mode, falls back to window.open.
   */
  async function openExternal(url: string) {
    if (inTauri) {
      try {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open(url);
        return;
      } catch {
        // Shell plugin unavailable — fall through to window.open
      }
    }
    window.open(url, '_blank', 'noopener,noreferrer');
  }

  /**
   * Converts a decimal degree coordinate to DMS (Degrees, Minutes, Seconds) notation.
   * @param decimal - The decimal degree value.
   * @param isLat - True for latitude (N/S), false for longitude (E/W).
   */
  function toDMS(decimal: number, isLat: boolean): string {
    const abs = Math.abs(decimal);
    const d = Math.floor(abs);
    const m = Math.floor((abs - d) * 60);
    const s = ((abs - d - m / 60) * 3600).toFixed(1);
    const dir = isLat ? (decimal >= 0 ? 'N' : 'S') : (decimal >= 0 ? 'E' : 'W');
    return `${d}\u00B0${m}\u2032${s}\u2033\u00A0${dir}`;
  }

  function highestSeverityFindings(findings: AnomalyFinding[]): AnomalyFinding[] {
    const order: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
    return [...findings].sort((a, b) => (order[a.severity] ?? 5) - (order[b.severity] ?? 5));
  }

  function forensicScoreClass(score: number): string {
    if (score < 0.3) return 'text-malachite dark:text-malachite-light';
    if (score < 0.6) return 'text-amber dark:text-amber-light';
    return 'text-cinnabar dark:text-cinnabar-light';
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

  // ── Raw detector scores for Technical View ───────────────────────
  /**
   * Maps forensic detector IDs to their raw score (0.0–1.0) and a threshold,
   * used when showRawScores is true to render numerical values in the signal
   * strip rather than traffic-light colours only.
   */
  const detectorScores = $derived((): Record<string, { score: number; threshold: number } | undefined> => {
    if (!result) return {};
    return {
      'section-ela':       result.elaResult       ? { score: result.elaResult.score,       threshold: 0.40 } : undefined,
      'section-noise':     result.noiseResult      ? { score: result.noiseResult.score,      threshold: 0.40 } : undefined,
      'section-copymove':  result.copyMoveResult   ? { score: result.copyMoveResult.score,   threshold: 0.40 } : undefined,
      'section-deepfake':  result.deepfakeResult   ? { score: result.deepfakeResult.score,   threshold: 0.50 } : undefined,
      'section-npr':       result.nprResult        ? { score: result.nprResult.score,        threshold: 0.40 } : undefined,
      'section-jpegGhost': result.jpegGhostResult  ? { score: result.jpegGhostResult.score,  threshold: 0.40 } : undefined,
      'section-ca':        result.caResult         ? { score: result.caResult.score,         threshold: 0.50 } : undefined,
    };
  });

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

  // ── ROI Selection ────────────────────────────────────────────────
  let roiMode = $state(false);
  let roiRect = $state<{ x: number; y: number; width: number; height: number } | null>(null);
  let roiResult = $state<RoiAnalysisResult | null>(null);
  let roiLoading = $state(false);
  let roiError = $state<string | null>(null);

  // Drag state (not reactive — used only within pointer event handlers)
  let _roiDragStart: { x: number; y: number } | null = null;
  let _roiDragging = false;

  /** Container element for the image preview — used to compute ROI coordinates. */
  let roiContainerEl = $state<HTMLDivElement | null>(null);

  function handleRoiPointerDown(e: PointerEvent) {
    if (!roiMode || !roiContainerEl) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const rect = roiContainerEl.getBoundingClientRect();
    _roiDragStart = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    _roiDragging = true;
    roiRect = null;
    roiResult = null;
    roiError = null;
  }

  function handleRoiPointerMove(e: PointerEvent) {
    if (!_roiDragging || !_roiDragStart || !roiContainerEl) return;
    e.preventDefault();
    const containerRect = roiContainerEl.getBoundingClientRect();
    const currentX = e.clientX - containerRect.left;
    const currentY = e.clientY - containerRect.top;
    const x = Math.min(_roiDragStart.x, currentX);
    const y = Math.min(_roiDragStart.y, currentY);
    const w = Math.abs(currentX - _roiDragStart.x);
    const h = Math.abs(currentY - _roiDragStart.y);
    roiRect = { x, y, width: w, height: h };
  }

  function handleRoiPointerUp(_e: PointerEvent) {
    if (!_roiDragging) return;
    _roiDragging = false;
    _roiDragStart = null;
    // Keep roiRect for the "Analyse Region" button
  }

  /**
   * Converts CSS-pixel coordinates on the displayed <img> element to
   * natural image pixel coordinates, then calls the ROI analysis API.
   */
  async function handleAnalyseRoi() {
    if (!roiRect || !filePath || roiLoading) return;
    const imgEl = roiContainerEl?.querySelector<HTMLImageElement>('img[data-preview="true"]');
    if (!imgEl || !imgEl.complete || imgEl.naturalWidth === 0) return;

    roiLoading = true;
    roiError = null;
    roiResult = null;

    try {
      // Scale CSS pixels to natural image pixels
      const displayRect = imgEl.getBoundingClientRect();
      const containerRect = roiContainerEl!.getBoundingClientRect();
      // The image may be letter-boxed inside the container — compute offset
      const imgOffsetX = displayRect.left - containerRect.left;
      const imgOffsetY = displayRect.top - containerRect.top;
      const scaleX = imgEl.naturalWidth / displayRect.width;
      const scaleY = imgEl.naturalHeight / displayRect.height;

      const naturalX = Math.max(0, Math.round((roiRect.x - imgOffsetX) * scaleX));
      const naturalY = Math.max(0, Math.round((roiRect.y - imgOffsetY) * scaleY));
      const naturalW = Math.min(imgEl.naturalWidth - naturalX, Math.round(roiRect.width * scaleX));
      const naturalH = Math.min(imgEl.naturalHeight - naturalY, Math.round(roiRect.height * scaleY));

      if (naturalW < 4 || naturalH < 4) {
        roiError = 'Selection is too small. Draw a larger region and try again.';
        roiLoading = false;
        return;
      }

      roiResult = await analyseRoi(filePath, naturalX, naturalY, naturalW, naturalH);
    } catch (e) {
      roiError = e instanceof Error ? e.message : 'Region analysis failed. Check that the Analysis Engine is running.';
    } finally {
      roiLoading = false;
    }
  }

  function clearRoi() {
    roiMode = false;
    roiRect = null;
    roiResult = null;
    roiError = null;
    _roiDragStart = null;
    _roiDragging = false;
  }

  // Reset ROI state when result changes
  $effect(() => {
    void result;
    clearRoi();
  });

  // ── Geolocation & Temporal panel ─────────────────────────────────
  let showGeoPanel = $state(false);

  // Sun position sub-panel
  let sunDateInput = $state('');
  let sunHourInput = $state(12);
  let sunPosition = $state<SolarPosition | null>(null);
  let sunLoading = $state(false);
  let sunError = $state<string | null>(null);

  // Shadow time sub-panel
  let shadowAzimuth = $state(180);
  let shadowTimeResults = $state<TimeEstimate[]>([]);
  let shadowLoading = $state(false);
  let shadowError = $state<string | null>(null);

  // Weather sub-panel
  let weatherResult = $state<WeatherCheckResult | null>(null);
  let weatherLoading = $state(false);
  let weatherError = $state<string | null>(null);
  let weatherConsentGiven = $state(false);

  /** GPS coordinates from EXIF, if available in the current result. */
  const gpsCoords = $derived(
    result?.exifAnalysis?.gpsLatitude != null && result?.exifAnalysis?.gpsLongitude != null
      ? { lat: result.exifAnalysis.gpsLatitude, lon: result.exifAnalysis.gpsLongitude }
      : null
  );

  /** Tracks whether the GPS copy feedback tick is showing. */
  let gpsCopied = $state(false);
  let gpsCopyTimer: ReturnType<typeof setTimeout> | null = null;

  /** Copy GPS decimal coordinates to the clipboard and show brief feedback. */
  function copyGpsCoords(lat: number, lon: number) {
    navigator.clipboard.writeText(`${lat.toFixed(6)}, ${lon.toFixed(6)}`).catch(() => {
      // Clipboard unavailable — silently ignore
    });
    if (gpsCopyTimer) clearTimeout(gpsCopyTimer);
    gpsCopied = true;
    gpsCopyTimer = setTimeout(() => { gpsCopied = false; }, 2000);
  }

  /** Parse the sunDateInput string (YYYY-MM-DD) into year/month/day parts. */
  function parseSunDate(): { year: number; month: number; day: number } | null {
    const parts = sunDateInput.split('-').map(Number);
    if (parts.length !== 3 || parts.some(isNaN)) return null;
    const [year, month, day] = parts;
    if (year < 1900 || year > 2100 || month < 1 || month > 12 || day < 1 || day > 31) return null;
    return { year, month, day };
  }

  async function handleCalculateSunPosition() {
    if (!gpsCoords || sunLoading) return;
    const dateParts = parseSunDate();
    if (!dateParts) {
      sunError = 'Please enter a valid date in YYYY-MM-DD format.';
      return;
    }
    sunLoading = true;
    sunError = null;
    sunPosition = null;
    try {
      sunPosition = await calculateSunPosition(
        gpsCoords.lat,
        gpsCoords.lon,
        dateParts.year,
        dateParts.month,
        dateParts.day,
        sunHourInput,
      );
    } catch (e) {
      sunError = e instanceof Error ? e.message : 'Sun position calculation failed.';
    } finally {
      sunLoading = false;
    }
  }

  async function handleEstimateShadowTime() {
    if (!gpsCoords || shadowLoading) return;
    const dateParts = parseSunDate();
    if (!dateParts) {
      shadowError = 'Please enter a valid date (YYYY-MM-DD) before estimating shadow time.';
      return;
    }
    shadowLoading = true;
    shadowError = null;
    shadowTimeResults = [];
    try {
      shadowTimeResults = await estimateShadowTime(
        gpsCoords.lat,
        gpsCoords.lon,
        dateParts.year,
        dateParts.month,
        dateParts.day,
        shadowAzimuth,
      );
    } catch (e) {
      shadowError = e instanceof Error ? e.message : 'Shadow time estimation failed.';
    } finally {
      shadowLoading = false;
    }
  }

  async function handleCheckWeather() {
    if (!gpsCoords || weatherLoading) return;
    const dateParts = parseSunDate();
    if (!dateParts) {
      weatherError = 'Please enter a valid date (YYYY-MM-DD) before checking weather.';
      return;
    }
    weatherLoading = true;
    weatherError = null;
    weatherResult = null;
    try {
      weatherResult = await checkHistoricalWeather(
        gpsCoords.lat,
        gpsCoords.lon,
        dateParts.year,
        dateParts.month,
        dateParts.day,
      );
    } catch (e) {
      weatherError = e instanceof Error ? e.message : 'Weather lookup failed.';
    } finally {
      weatherLoading = false;
    }
  }

  // Reset geo panel state when result changes
  $effect(() => {
    void result;
    showGeoPanel = false;
    sunPosition = null;
    sunError = null;
    shadowTimeResults = [];
    shadowError = null;
    weatherResult = null;
    weatherError = null;
    weatherConsentGiven = false;
    sunDateInput = '';
    sunHourInput = 12;
    shadowAzimuth = 180;
  });

  // Pre-populate date from EXIF DateTimeOriginal when result loads
  $effect(() => {
    const exif = result?.exifAnalysis;
    if (!exif) return;
    // Try to extract a date from datetimeOriginal (format: "YYYY:MM:DD HH:MM:SS")
    const raw = (result?.exifAnalysis as any)?.datetimeOriginal as string | undefined;
    if (raw && /^\d{4}:\d{2}:\d{2}/.test(raw)) {
      sunDateInput = raw.slice(0, 10).replace(/:/g, '-');
      const hour = parseInt(raw.slice(11, 13), 10);
      if (!isNaN(hour)) sunHourInput = hour;
    }
  });

  // ── Seasonal Analysis ─────────────────────────────────────────────
  let seasonalResult = $state<SeasonalIndicatorsResult | null>(null);
  let seasonalLoading = $state(false);
  let seasonalError = $state<string | null>(null);

  async function handleSeasonalAnalysis() {
    if (!filePath || seasonalLoading) return;
    seasonalLoading = true;
    seasonalError = null;
    seasonalResult = null;
    try {
      seasonalResult = await analyseSeasonalIndicators(filePath);
    } catch (e) {
      seasonalError = e instanceof Error ? e.message : 'Seasonal analysis failed. Check that the Analysis Engine is running.';
    } finally {
      seasonalLoading = false;
    }
  }

  // ── Diffusion Artefacts ───────────────────────────────────────────
  let diffusionResult = $state<DiffusionArtefactsResult | null>(null);
  let diffusionLoading = $state(false);
  let diffusionError = $state<string | null>(null);

  async function handleDiffusionCheck() {
    if (!filePath || diffusionLoading) return;
    diffusionLoading = true;
    diffusionError = null;
    diffusionResult = null;
    try {
      diffusionResult = await analyseDiffusionArtefacts(filePath);
    } catch (e) {
      diffusionError = e instanceof Error ? e.message : 'Diffusion artefact analysis failed. Check that the Analysis Engine is running.';
    } finally {
      diffusionLoading = false;
    }
  }

  // Reset on-demand panels when result changes
  $effect(() => {
    void result;
    seasonalResult = null;
    seasonalError = null;
    diffusionResult = null;
    diffusionError = null;
  });

  /** Format a sun azimuth as a compass direction label. */
  function azimuthToCompass(deg: number): string {
    const dirs = ['N', 'NNE', 'NE', 'ENE', 'E', 'ESE', 'SE', 'SSE', 'S', 'SSW', 'SW', 'WSW', 'W', 'WNW', 'NW', 'NNW'];
    const index = Math.round(((deg % 360) + 360) % 360 / 22.5) % 16;
    return dirs[index];
  }

  /** Format a UTC hour float as HH:MM UTC. */
  function formatUtcHour(h: number): string {
    const hh = Math.floor(h);
    const mm = Math.round((h - hh) * 60);
    return `${String(hh).padStart(2, '0')}:${String(mm).padStart(2, '0')} UTC`;
  }

  // ── Annotation canvas state ────────────────────────────────────────

  /** Whether the annotation toolbar is shown over the image preview. */
  let annotationMode = $state(false);
  /** Which drawing tool is active within the annotation toolbar. */
  let annotationTool = $state<'arrow' | 'circle' | 'rectangle' | 'text'>('arrow');
  /** Stroke colour for new annotations (hex). */
  let annotationColour = $state('#5A85B5');
  /** Persisted annotations loaded from the backend (or built up in-session in browser mode). */
  let annotations = $state<Annotation[]>([]);
  /** Whether a pointer-drag draw gesture is in progress. */
  let isDrawingAnnotation = $state(false);
  /** CSS-pixel start point of the current draw gesture (relative to the image container). */
  let drawStart = $state<{ x: number; y: number } | null>(null);
  /** CSS-pixel current end point, used for the live preview ghost shape during dragging. */
  let currentDrawEnd = $state<{ x: number; y: number } | null>(null);
  /** Annotation ID of whichever annotation the pointer is hovering over (for delete button). */
  let hoveredAnnotationId = $state<string | null>(null);
  /** Pending text annotation placement — CSS coordinates where the user clicked. */
  let pendingTextPos = $state<{ cssX: number; cssY: number } | null>(null);
  /** The text being typed for a pending text annotation. */
  let pendingTextValue = $state('');

  /**
   * The annotation container is the same DOM element as roiContainerEl — both
   * refer to the `<div class="relative group ...">` that wraps the preview image.
   * We derive this alias so annotation functions can reference it clearly.
   */
  const annotationContainerEl = $derived(roiContainerEl);

  /**
   * Colour swatches available in the annotation sub-toolbar.
   * Uses brand palette values so annotations harmonise with the Sanctuary theme.
   */
  const annotationColours = [
    { hex: '#5A85B5', label: 'Lapis (blue)' },
    { hex: '#C45B4B', label: 'Cinnabar (red)' },
    { hex: '#5B8A5F', label: 'Malachite (green)' },
    { hex: '#D4A843', label: 'Amber (gold)' },
    { hex: '#EDEAE4', label: 'Quartz (cream)' },
  ] as const;

  /** Load annotations for the current result whenever a new result appears. */
  $effect(() => {
    if (result) {
      getAnnotations(undefined, undefined)
        .then(anns => { annotations = anns; })
        .catch(() => { /* silently tolerate IPC errors in browser mode */ });
    }
  });

  /** Clear all annotation state whenever a new file is loaded. */
  $effect(() => {
    void filePath; // dependency registration — fires on every new file load
    annotations = [];
    annotationMode = false;
    isDrawingAnnotation = false;
    drawStart = null;
    currentDrawEnd = null;
    hoveredAnnotationId = null;
  });

  /**
   * Convert a CSS-pixel point relative to the image container into the
   * equivalent natural-image pixel coordinate.
   *
   * Returns null when the image element is not yet ready.
   */
  function cssToNaturalPx(
    container: HTMLElement,
    cssX: number,
    cssY: number,
  ): { x: number; y: number } | null {
    const imgEl = container.querySelector<HTMLImageElement>('img[data-preview="true"]');
    if (!imgEl || !imgEl.complete || imgEl.naturalWidth === 0) return null;

    const displayRect = imgEl.getBoundingClientRect();
    const containerRect = container.getBoundingClientRect();
    const imgOffsetX = displayRect.left - containerRect.left;
    const imgOffsetY = displayRect.top - containerRect.top;
    const scaleX = imgEl.naturalWidth / displayRect.width;
    const scaleY = imgEl.naturalHeight / displayRect.height;

    return {
      x: Math.round((cssX - imgOffsetX) * scaleX),
      y: Math.round((cssY - imgOffsetY) * scaleY),
    };
  }

  function handleAnnotationPointerDown(e: PointerEvent) {
    if (!annotationMode || !annotationContainerEl) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const rect = annotationContainerEl.getBoundingClientRect();
    drawStart = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    currentDrawEnd = { ...drawStart };
    isDrawingAnnotation = true;
  }

  function handleAnnotationPointerMove(e: PointerEvent) {
    if (!isDrawingAnnotation || !annotationContainerEl) return;
    e.preventDefault();
    const rect = annotationContainerEl.getBoundingClientRect();
    currentDrawEnd = { x: e.clientX - rect.left, y: e.clientY - rect.top };
  }

  async function handleAnnotationPointerUp(e: PointerEvent) {
    if (!isDrawingAnnotation || !drawStart || !annotationContainerEl) return;
    isDrawingAnnotation = false;

    const rect = annotationContainerEl.getBoundingClientRect();
    const endCss = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    currentDrawEnd = null;

    // For text: show an inline input at the click position instead of
    // window.prompt() which is blocked in Tauri webviews.
    if (annotationTool === 'text') {
      pendingTextPos = { cssX: drawStart.x, cssY: drawStart.y };
      pendingTextValue = '';
      drawStart = null;
      isDrawingAnnotation = false;
      // Focus the input after it renders
      requestAnimationFrame(() => {
        const inp = annotationContainerEl?.querySelector<HTMLInputElement>('[data-annotation-text-input]');
        inp?.focus();
      });
      return;
    }

    // Guard: require a minimum drag distance of 8 CSS pixels to avoid
    // accidental micro-annotations on accidental clicks.
    const dx = endCss.x - drawStart.x;
    const dy = endCss.y - drawStart.y;
    if (Math.abs(dx) < 8 && Math.abs(dy) < 8) { drawStart = null; return; }

    const origin = cssToNaturalPx(annotationContainerEl, drawStart.x, drawStart.y);
    const end = cssToNaturalPx(annotationContainerEl, endCss.x, endCss.y);
    if (!origin || !end) { drawStart = null; return; }

    let data: AnnotationData;

    if (annotationTool === 'arrow') {
      data = {
        x: origin.x,
        y: origin.y,
        x2: end.x,
        y2: end.y,
        colour: annotationColour,
        strokeWidth: 2,
      };
    } else if (annotationTool === 'circle') {
      const cx = (origin.x + end.x) / 2;
      const cy = (origin.y + end.y) / 2;
      const radius = Math.sqrt(
        Math.pow(end.x - origin.x, 2) + Math.pow(end.y - origin.y, 2),
      ) / 2;
      data = {
        x: cx,
        y: cy,
        radius,
        colour: annotationColour,
        strokeWidth: 2,
      };
    } else {
      // rectangle
      data = {
        x: Math.min(origin.x, end.x),
        y: Math.min(origin.y, end.y),
        width: Math.abs(end.x - origin.x),
        height: Math.abs(end.y - origin.y),
        colour: annotationColour,
        strokeWidth: 2,
      };
    }

    const ann = await saveAnnotation(annotationTool, JSON.stringify(data));
    annotations = [...annotations, ann];
    drawStart = null;
  }

  async function handleDeleteAnnotation(annotationId: string) {
    annotations = annotations.filter(a => a.annotationId !== annotationId);
    await deleteAnnotationApi(annotationId).catch(() => { /* tolerate IPC errors */ });
    if (hoveredAnnotationId === annotationId) hoveredAnnotationId = null;
  }

  async function clearAllAnnotations() {
    const ids = annotations.map(a => a.annotationId);
    annotations = [];
    hoveredAnnotationId = null;
    await Promise.all(ids.map(id => deleteAnnotationApi(id).catch(() => {})));
  }

  /** Commit the pending inline text annotation. */
  async function commitPendingText() {
    if (!pendingTextPos || !pendingTextValue.trim() || !annotationContainerEl) {
      pendingTextPos = null;
      pendingTextValue = '';
      return;
    }
    const origin = cssToNaturalPx(annotationContainerEl, pendingTextPos.cssX, pendingTextPos.cssY);
    if (!origin) { pendingTextPos = null; return; }

    const data: AnnotationData = {
      x: origin.x,
      y: origin.y,
      text: pendingTextValue.trim(),
      colour: annotationColour,
      strokeWidth: 2,
    };
    const ann = await saveAnnotation('text', JSON.stringify(data));
    annotations = [...annotations, ann];
    pendingTextPos = null;
    pendingTextValue = '';
  }

  /** Cancel the pending text annotation. */
  function cancelPendingText() {
    pendingTextPos = null;
    pendingTextValue = '';
  }

  /**
   * Render a single annotation shape as SVG markup.
   * Returns the coordinates scaled from natural-image pixels back to the
   * CSS-pixel display space so the SVG aligns with the visible image.
   *
   * The function returns a plain object describing the shape; the template
   * renders the actual SVG elements.
   */
  function annotationToSvgProps(
    ann: Annotation,
    imgEl: HTMLImageElement,
    containerEl: HTMLElement,
  ): {
    type: 'arrow' | 'circle' | 'rectangle' | 'text';
    data: AnnotationData;
    // scaled CSS coords
    x: number; y: number; x2: number; y2: number;
    width: number; height: number; radius: number;
  } | null {
    let data: AnnotationData;
    try {
      data = JSON.parse(ann.dataJson) as AnnotationData;
    } catch {
      return null;
    }

    const displayRect = imgEl.getBoundingClientRect();
    const containerRect = containerEl.getBoundingClientRect();
    const imgOffsetX = displayRect.left - containerRect.left;
    const imgOffsetY = displayRect.top - containerRect.top;
    const scaleX = displayRect.width / imgEl.naturalWidth;
    const scaleY = displayRect.height / imgEl.naturalHeight;

    // Scale natural-px coords back to CSS display-px coords
    const sx = (v: number) => v * scaleX + imgOffsetX;
    const sy = (v: number) => v * scaleY + imgOffsetY;

    return {
      type: ann.annotationType as 'arrow' | 'circle' | 'rectangle' | 'text',
      data,
      x: sx(data.x),
      y: sy(data.y),
      x2: data.x2 != null ? sx(data.x2) : sx(data.x),
      y2: data.y2 != null ? sy(data.y2) : sy(data.y),
      width: data.width != null ? data.width * scaleX : 0,
      height: data.height != null ? data.height * scaleY : 0,
      radius: data.radius != null ? data.radius * Math.min(scaleX, scaleY) : 0,
    };
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
              description: 'Core forensic checks — recommended for most content',
            },
            {
              mode: 'deep' as VerifyMode,
              label: 'Deep',
              description: 'Extended analysis with regional and frequency-domain detectors',
            },
            {
              mode: 'archival' as VerifyMode,
              label: 'Archival',
              description: 'Full analysis with scanner-calibrated tolerances for digitised collections',
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
              <span class="block text-xs leading-tight mt-0.5
                           {verifyMode === opt.mode
                             ? 'text-lapis/70 dark:text-lapis-light/70'
                             : 'text-flint/70 dark:text-flint-light/70'}">
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
      <!-- Claim checking is available when Ollama is running and audio/video transcription produces text -->
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
              <p class="text-xs text-flint/70 dark:text-flint-light/70">{fileName}</p>
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
                   hover:text-cinnabar hover:border-cinnabar/50 dark:hover:text-cinnabar-light dark:hover:border-cinnabar-light/50
                   transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
            aria-label="Cancel video analysis"
          >
            Cancel analysis
          </button>
          <p class="text-xs text-flint/50 dark:text-flint-light/60">
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
          {#if batchCompleted > 0}
            <button
              class="text-xs px-3 py-2 min-h-[44px] inline-flex items-center gap-1.5 rounded border border-malachite/50 text-malachite dark:text-malachite-light hover:bg-malachite/10 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-malachite focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
              onclick={downloadBatchReport}
              aria-label="Download batch verification results as a CSV spreadsheet"
            >
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                  d="M12 16v-8m0 8l-3-3m3 3l3-3M4 20h16" />
              </svg>
              Download Batch Report
            </button>
          {/if}
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
                      <span class="text-lapis dark:text-lapis-light">Running</span>
                    </span>
                  {:else if item.status === 'done'}
                    {@const level = getTrustLevel(item.result?.overallTrust ?? 0)}
                    <span class="font-medium px-1.5 py-0.5 rounded
                      {level === 'high' ? 'text-malachite dark:text-malachite-light bg-malachite/10' :
                       level === 'medium' ? 'text-amber dark:text-amber-light bg-amber/10' :
                       'text-cinnabar dark:text-cinnabar-light bg-cinnabar/10'}">
                      Done
                    </span>
                  {:else}
                    <span class="text-cinnabar dark:text-cinnabar-light">Error</span>
                  {/if}
                </span>

                <!-- Trust -->
                <span class="text-xs tabular-nums self-center">
                  {#if item.status === 'done' && item.result}
                    {@const level = getTrustLevel(item.result.overallTrust)}
                    <span class="{level === 'high' ? 'text-malachite dark:text-malachite-light' : level === 'medium' ? 'text-amber dark:text-amber-light' : 'text-cinnabar dark:text-cinnabar-light'}">
                      {Math.round(item.result.overallTrust * 100)}%
                    </span>
                  {:else}
                    <span class="text-flint/50 dark:text-flint-light/60">—</span>
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
                      class="text-xs text-flint dark:text-flint-light hover:text-cinnabar dark:hover:text-cinnabar-light transition-colors p-1 min-w-[24px] min-h-[24px]
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
                <div class="px-4 py-2 bg-cinnabar/5 text-xs text-cinnabar dark:text-cinnabar-light">
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

    <!-- Large image preview — shown above analysis results for visual reference -->
    {#if previewUrl && result.contentType === 'image'}
      {@const elaHeatmapUrl = result.elaResult?.elaImageBase64 ? blobs.url(result.elaResult.elaImageBase64, 'image/png') : null}
      <div class="mb-4 max-w-2xl mx-auto">
        <!-- Image + signal strip side-by-side -->
        <div class="flex gap-3 items-start">
          <!-- Main image container -->
          <div class="flex-1 rounded-lg overflow-hidden border border-border-light dark:border-border-dark bg-obsidian/30">
            <!-- Image with optional ELA overlay — click to open zoom modal, drag to select ROI, or draw annotation -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="relative group {roiMode ? 'cursor-crosshair' : annotationMode ? 'cursor-crosshair' : 'cursor-zoom-in'}"
              style="{roiMode || annotationMode ? 'touch-action: none; user-select: none;' : ''}"
              role="presentation"
              bind:this={roiContainerEl}
              onpointerdown={roiMode ? handleRoiPointerDown : annotationMode ? handleAnnotationPointerDown : undefined}
              onpointermove={roiMode ? handleRoiPointerMove : annotationMode ? handleAnnotationPointerMove : undefined}
              onpointerup={roiMode ? handleRoiPointerUp : annotationMode ? handleAnnotationPointerUp : undefined}
            >
              {#if roiMode || annotationMode}
                <!-- In ROI / annotation mode the outer click-to-zoom button is replaced
                     by a plain image so pointer events on the div can drive the gesture.
                     The image itself is pointer-events-none so all gestures reach the
                     container div's onpointerdown/move/up handlers. -->
                <img
                  src={channelImageUrl ?? previewUrl}
                  alt={roiMode ? 'Analysed file — drag to select a region' : 'Analysed file — drag to draw an annotation'}
                  class="w-full max-h-[400px] object-contain block select-none pointer-events-none"
                  loading="lazy"
                  data-preview="true"
                  draggable="false"
                  style="{channelImageUrl ? '' : `filter: ${getFilterStyle(activeFilter)};`}"
                />
              {:else}
                <button
                  type="button"
                  class="w-full block focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-inset"
                  onclick={handleImageClick}
                  aria-label="Open zoom viewer for {fileName ?? 'analysed file'}"
                >
                  <img
                    src={channelImageUrl ?? previewUrl}
                    alt="Analysed file"
                    class="w-full max-h-[400px] object-contain block"
                    loading="lazy"
                    data-preview="true"
                    style="{channelImageUrl ? '' : `filter: ${getFilterStyle(activeFilter)};`}"
                  />
                </button>
              {/if}

              <!-- Zoom hint badge — hidden in ROI mode -->
              {#if !roiMode}
                <div
                  class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 motion-safe:transition-opacity duration-150 pointer-events-none
                         bg-obsidian/70 rounded px-1.5 py-1 flex items-center gap-1"
                  aria-hidden="true"
                >
                  <svg class="w-3.5 h-3.5 text-quartz" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M21 21l-4.35-4.35M17 11A6 6 0 105 11a6 6 0 0012 0zm-2 0h-4m2-2v4" />
                  </svg>
                  <span class="text-xs text-quartz">Zoom</span>
                </div>
              {/if}

              <!-- ROI mode hint badge -->
              {#if roiMode}
                <div
                  class="absolute top-2 left-2 bg-lapis/80 rounded px-2 py-1 pointer-events-none"
                  aria-hidden="true"
                >
                  <span class="text-xs text-white font-medium">Drag to select region</span>
                </div>
              {/if}

              {#if showElaOverlay && elaHeatmapUrl}
                <img
                  src={elaHeatmapUrl}
                  alt=""
                  aria-hidden="true"
                  class="absolute inset-0 w-full h-full object-contain pointer-events-none"
                  style="opacity: {elaOpacity / 100}; mix-blend-mode: {elaBlendMode};"
                />
              {/if}

              <!-- ROI selection rectangle SVG overlay -->
              {#if roiMode && roiRect && roiRect.width > 2 && roiRect.height > 2}
                <svg
                  class="absolute inset-0 w-full h-full pointer-events-none"
                  aria-hidden="true"
                  style="position: absolute; top: 0; left: 0; width: 100%; height: 100%;"
                >
                  <!-- Darkened overlay outside the selection -->
                  <defs>
                    <mask id="roi-mask">
                      <rect x="0" y="0" width="100%" height="100%" fill="white" />
                      <rect
                        x={roiRect.x}
                        y={roiRect.y}
                        width={roiRect.width}
                        height={roiRect.height}
                        fill="black"
                      />
                    </mask>
                  </defs>
                  <rect x="0" y="0" width="100%" height="100%" fill="rgba(0,0,0,0.35)" mask="url(#roi-mask)" />
                  <!-- Dashed lapis border -->
                  <rect
                    x={roiRect.x}
                    y={roiRect.y}
                    width={roiRect.width}
                    height={roiRect.height}
                    fill="none"
                    stroke="#5A85B5"
                    stroke-width="2"
                    stroke-dasharray="6 3"
                  />
                  <!-- Corner handles -->
                  {#each [
                    [roiRect.x, roiRect.y],
                    [roiRect.x + roiRect.width, roiRect.y],
                    [roiRect.x, roiRect.y + roiRect.height],
                    [roiRect.x + roiRect.width, roiRect.y + roiRect.height],
                  ] as [cx, cy]}
                    <circle cx={cx} cy={cy} r="4" fill="#5A85B5" />
                  {/each}
                </svg>
              {/if}

              <!-- Annotation SVG overlay — persisted annotations + live ghost preview -->
              {#if annotationMode || annotations.length > 0}
                {@const _annContainer = annotationContainerEl}
                {@const imgEl = _annContainer?.querySelector<HTMLImageElement>('img[data-preview="true"]') ?? null}
                {#if imgEl && imgEl.complete && imgEl.naturalWidth > 0 && _annContainer}
                  <svg
                    class="absolute inset-0 w-full h-full pointer-events-none"
                    style="position: absolute; top: 0; left: 0; width: 100%; height: 100%;"
                    aria-label="Image annotations"
                    role="img"
                  >
                    <defs>
                      <!-- Arrowhead marker for arrow annotations -->
                      <marker
                        id="ann-arrowhead"
                        markerWidth="8"
                        markerHeight="8"
                        refX="6"
                        refY="3"
                        orient="auto"
                      >
                        <path d="M0,0 L0,6 L8,3 z" fill={annotationColour} />
                      </marker>
                      <!-- Per-annotation arrowhead markers with the annotation's own colour -->
                      {#each annotations as ann (ann.annotationId)}
                        {#if ann.annotationType === 'arrow'}
                          {@const parsed = (() => { try { return JSON.parse(ann.dataJson) as AnnotationData; } catch { return null; } })()}
                          {#if parsed}
                            <marker
                              id="ann-arrowhead-{ann.annotationId}"
                              markerWidth="8"
                              markerHeight="8"
                              refX="6"
                              refY="3"
                              orient="auto"
                            >
                              <path d="M0,0 L0,6 L8,3 z" fill={parsed.colour} />
                            </marker>
                          {/if}
                        {/if}
                      {/each}
                    </defs>

                    <!-- Render persisted annotations -->
                    {#each annotations as ann (ann.annotationId)}
                      {@const props = annotationToSvgProps(ann, imgEl, _annContainer)}
                      {#if props}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <g
                          class="cursor-pointer"
                          style="pointer-events: {isDrawingAnnotation ? 'none' : 'auto'};"
                          role="graphics-symbol"
                          aria-label="{ann.annotationType} annotation"
                          onmouseenter={() => { hoveredAnnotationId = ann.annotationId; }}
                          onmouseleave={() => { hoveredAnnotationId = null; }}
                          onfocus={() => { hoveredAnnotationId = ann.annotationId; }}
                          onblur={() => { hoveredAnnotationId = null; }}
                        >
                          {#if props.type === 'arrow'}
                            <line
                              x1={props.x}
                              y1={props.y}
                              x2={props.x2}
                              y2={props.y2}
                              stroke={props.data.colour}
                              stroke-width={props.data.strokeWidth}
                              marker-end="url(#ann-arrowhead-{ann.annotationId})"
                              stroke-linecap="round"
                            />
                            <!-- Wider invisible hit target for hover -->
                            <line
                              x1={props.x}
                              y1={props.y}
                              x2={props.x2}
                              y2={props.y2}
                              stroke="transparent"
                              stroke-width="12"
                            />
                          {:else if props.type === 'circle'}
                            <circle
                              cx={props.x}
                              cy={props.y}
                              r={props.radius}
                              fill="none"
                              stroke={props.data.colour}
                              stroke-width={props.data.strokeWidth}
                            />
                          {:else if props.type === 'rectangle'}
                            <rect
                              x={props.x}
                              y={props.y}
                              width={props.width}
                              height={props.height}
                              fill="none"
                              stroke={props.data.colour}
                              stroke-width={props.data.strokeWidth}
                            />
                          {:else if props.type === 'text'}
                            <text
                              x={props.x}
                              y={props.y}
                              fill={props.data.colour}
                              font-size="14"
                              font-family="system-ui, sans-serif"
                              font-weight="600"
                              paint-order="stroke"
                              stroke="rgba(0,0,0,0.6)"
                              stroke-width="3"
                              stroke-linejoin="round"
                            >{props.data.text}</text>
                          {/if}

                          <!-- Delete button — visible on hover -->
                          {#if hoveredAnnotationId === ann.annotationId}
                            <!-- svelte-ignore a11y_interactive_supports_focus -->
                            <g
                              role="button"
                              aria-label="Delete annotation"
                              class="cursor-pointer"
                              onclick={(e) => { e.stopPropagation(); handleDeleteAnnotation(ann.annotationId); }}
                              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleDeleteAnnotation(ann.annotationId); } }}
                              tabindex="0"
                            >
                              <circle
                                cx={props.type === 'arrow' ? (props.x + props.x2) / 2 : props.type === 'circle' ? props.x + props.radius : props.x + props.width}
                                cy={props.type === 'arrow' ? (props.y + props.y2) / 2 : props.y}
                                r="10"
                                fill="#C45B4B"
                                stroke="white"
                                stroke-width="1.5"
                              />
                              <text
                                x={props.type === 'arrow' ? (props.x + props.x2) / 2 : props.type === 'circle' ? props.x + props.radius : props.x + props.width}
                                y={props.type === 'arrow' ? (props.y + props.y2) / 2 : props.y}
                                text-anchor="middle"
                                dominant-baseline="central"
                                fill="white"
                                font-size="11"
                                font-weight="700"
                                font-family="system-ui, sans-serif"
                                pointer-events="none"
                              >&#x2715;</text>
                            </g>
                          {/if}
                        </g>
                      {/if}
                    {/each}

                    <!-- Ghost preview of the shape currently being drawn -->
                    {#if isDrawingAnnotation && drawStart && currentDrawEnd}
                      {#if annotationTool === 'arrow'}
                        <line
                          x1={drawStart.x}
                          y1={drawStart.y}
                          x2={currentDrawEnd.x}
                          y2={currentDrawEnd.y}
                          stroke={annotationColour}
                          stroke-width="2"
                          stroke-opacity="0.7"
                          marker-end="url(#ann-arrowhead)"
                          stroke-linecap="round"
                          stroke-dasharray="4 2"
                        />
                      {:else if annotationTool === 'circle'}
                        <ellipse
                          cx={(drawStart.x + currentDrawEnd.x) / 2}
                          cy={(drawStart.y + currentDrawEnd.y) / 2}
                          rx={Math.abs(currentDrawEnd.x - drawStart.x) / 2}
                          ry={Math.abs(currentDrawEnd.y - drawStart.y) / 2}
                          fill="none"
                          stroke={annotationColour}
                          stroke-width="2"
                          stroke-opacity="0.7"
                          stroke-dasharray="4 2"
                        />
                      {:else if annotationTool === 'rectangle'}
                        <rect
                          x={Math.min(drawStart.x, currentDrawEnd.x)}
                          y={Math.min(drawStart.y, currentDrawEnd.y)}
                          width={Math.abs(currentDrawEnd.x - drawStart.x)}
                          height={Math.abs(currentDrawEnd.y - drawStart.y)}
                          fill="none"
                          stroke={annotationColour}
                          stroke-width="2"
                          stroke-opacity="0.7"
                          stroke-dasharray="4 2"
                        />
                      {/if}
                    {/if}
                  </svg>
                {/if}
              {/if}

              <!-- Annotation mode hint badge -->
              {#if annotationMode}
                <div
                  class="absolute top-2 left-2 bg-lapis/80 rounded px-2 py-1 pointer-events-none"
                  aria-hidden="true"
                >
                  <span class="text-xs text-white font-medium">
                    {annotationTool === 'text' ? 'Click to place text' : 'Drag to draw'}
                  </span>
                </div>
              {/if}

              <!-- Inline text annotation input — positioned at click point -->
              {#if pendingTextPos}
                <div
                  class="absolute z-20 flex items-center gap-1"
                  style="left: {pendingTextPos.cssX}px; top: {pendingTextPos.cssY}px; transform: translateY(-50%);"
                >
                  <input
                    type="text"
                    data-annotation-text-input
                    bind:value={pendingTextValue}
                    placeholder="Type annotation..."
                    class="text-xs px-2 py-1 rounded border border-lapis bg-obsidian text-quartz
                           focus:outline-none focus:ring-2 focus:ring-lapis w-48"
                    onkeydown={(e) => {
                      if (e.key === 'Enter') { e.preventDefault(); commitPendingText(); }
                      if (e.key === 'Escape') { e.preventDefault(); cancelPendingText(); }
                    }}
                  />
                  <button
                    type="button"
                    class="text-xs px-2 py-1 rounded bg-lapis text-white hover:bg-lapis-light"
                    onclick={commitPendingText}
                  >Add</button>
                  <button
                    type="button"
                    class="text-xs px-1 py-1 text-flint hover:text-cinnabar"
                    onclick={cancelPendingText}
                    aria-label="Cancel text annotation"
                  >&times;</button>
                </div>
              {/if}
            </div>

            <!-- Image caption, inspection tools + overlay controls -->
            <div class="px-3 py-2.5 border-t border-border-light dark:border-border-dark bg-white/50 dark:bg-graphite/50 space-y-2.5">
              <div class="flex items-center justify-between gap-2">
                <p class="text-xs text-flint dark:text-flint-light truncate flex-1 min-w-0" title={fileName ?? undefined}>
                  {fileName}
                </p>
                <!-- Compare button — loads a second image for side-by-side inspection -->
                {#if !comparisonMode}
                  <button
                    type="button"
                    onclick={handleLoadComparison}
                    class="flex-shrink-0 text-xs px-2.5 py-1 min-h-[28px] rounded border border-border-light dark:border-border-dark
                           text-flint dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50
                           hover:text-lapis dark:hover:text-lapis-light transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                           focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    aria-label="Load a second image for side-by-side comparison"
                    title="Compare with another image side by side"
                  >
                    Compare
                  </button>
                {:else}
                  <button
                    type="button"
                    onclick={closeComparison}
                    class="flex-shrink-0 text-xs px-2.5 py-1 min-h-[28px] rounded border border-cinnabar/40
                           text-cinnabar dark:text-cinnabar-light hover:bg-cinnabar/10 transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-1
                           focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    aria-label="Close comparison view"
                  >
                    Close comparison
                  </button>
                {/if}
              </div>

              <!-- Inspection filter toolbar -->
              <div>
                <div class="flex items-center gap-1.5 flex-wrap" role="group" aria-label="Visual inspection filters">
                  <span class="text-xs text-flint dark:text-flint-light mr-0.5 flex-shrink-0">Inspect:</span>

                  {#each ([
                    { key: 'grayscale', label: 'Greyscale', title: 'Remove colour to reveal tonal patterns and cloning artefacts' },
                    { key: 'invert',    label: 'Invert',    title: 'Flip colours — can reveal hidden watermarks and subtle gradients' },
                    { key: 'contrast',  label: 'High Contrast', title: 'Amplify regional differences to reveal compression artefacts' },
                    { key: 'saturate',  label: 'Saturate',  title: 'Exaggerate colour differences between potentially spliced regions' },
                    { key: 'edges',     label: 'Edge Detect', title: 'Reveal edge boundaries — useful for spotting composite seams' },
                  ] as const) as item}
                    <button
                      type="button"
                      title={item.title}
                      onclick={() => { activeFilter = activeFilter === item.key ? 'none' : item.key as InspectFilter; }}
                      class="text-xs px-2 py-1 min-h-[28px] rounded border transition-colors duration-150
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                             focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                             {activeFilter === item.key
                               ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                               : 'border-border-light dark:border-border-dark text-gray-600 dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50'}"
                      aria-pressed={activeFilter === item.key}
                    >
                      {item.label}
                    </button>
                  {/each}

                  {#if activeFilter !== 'none' || brightness !== 100 || contrast !== 100 || activeChannel !== 'none'}
                    <button
                      type="button"
                      onclick={resetInspection}
                      class="text-xs px-2 py-1 min-h-[28px] text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                      aria-label="Reset all image inspection filters"
                    >
                      Reset
                    </button>
                  {/if}
                </div>

                <!-- Brightness / Contrast sliders -->
                <div class="flex items-center gap-4 mt-2 flex-wrap">
                  <div class="flex items-center gap-2">
                    <label class="text-xs text-flint dark:text-flint-light flex-shrink-0" for="inspect-brightness">Brightness</label>
                    <input
                      id="inspect-brightness"
                      type="range"
                      min="50"
                      max="200"
                      bind:value={brightness}
                      class="w-20 accent-lapis cursor-pointer"
                      aria-label="Image brightness: {brightness}%"
                    />
                    <span class="text-xs tabular-nums text-flint dark:text-flint-light w-9 flex-shrink-0">{brightness}%</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <label class="text-xs text-flint dark:text-flint-light flex-shrink-0" for="inspect-contrast">Contrast</label>
                    <input
                      id="inspect-contrast"
                      type="range"
                      min="50"
                      max="300"
                      bind:value={contrast}
                      class="w-20 accent-lapis cursor-pointer"
                      aria-label="Image contrast: {contrast}%"
                    />
                    <span class="text-xs tabular-nums text-flint dark:text-flint-light w-9 flex-shrink-0">{contrast}%</span>
                  </div>
                </div>

                <!-- Colour channel separation toolbar -->
                <div class="mt-2 pt-2 border-t border-border-light/60 dark:border-border-dark/60">
                  <div class="flex items-center gap-1.5 flex-wrap" role="group" aria-label="Colour channel separation">
                    <span class="text-xs text-flint dark:text-flint-light mr-0.5 flex-shrink-0">Channels:</span>
                    {#each ([
                      { key: 'r',  label: 'R',   title: 'Red channel only — highlights red-tinted regions and colour inconsistencies' },
                      { key: 'g',  label: 'G',   title: 'Green channel only — often most detail-rich; useful for detecting green screen artefacts' },
                      { key: 'b',  label: 'B',   title: 'Blue channel only — reveals blue cast anomalies and compression artefacts in shadows' },
                      { key: 'rg', label: 'R-G', title: 'Red minus Green difference — amplifies warm/cool colour seams between spliced regions' },
                      { key: 'rb', label: 'R-B', title: 'Red minus Blue difference — highlights magenta/cyan boundaries indicating compositing' },
                      { key: 'gb', label: 'G-B', title: 'Green minus Blue difference — exposes yellow/blue transitions typical in AI-generated skies' },
                    ] as const) as ch}
                      <button
                        type="button"
                        title={ch.title}
                        onclick={() => toggleChannel(ch.key as typeof activeChannel)}
                        class="text-xs px-2 py-1 min-h-[28px] rounded border transition-colors duration-150
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                               focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                               {activeChannel === ch.key
                                 ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                                 : 'border-border-light dark:border-border-dark text-gray-600 dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50'}"
                        aria-pressed={activeChannel === ch.key}
                      >
                        {ch.label}
                      </button>
                    {/each}
                    {#if activeChannel !== 'none'}
                      <span class="text-xs text-lapis dark:text-lapis-light ml-1 flex-shrink-0" aria-live="polite" aria-atomic="true">
                        {activeChannel.length <= 2 ? activeChannel.toUpperCase() + ' channel' : activeChannel.toUpperCase() + ' difference'} active
                      </span>
                    {/if}
                  </div>
                </div>

                <!-- Link to Visual Inspection Checklist + ROI mode toggle -->
                <div class="mt-1.5 flex flex-wrap items-center gap-3">
                  <a
                    href="#inspection-checklist"
                    onclick={(e) => { e.preventDefault(); showInspectionChecklist = true; requestAnimationFrame(() => document.getElementById('inspection-checklist')?.scrollIntoView({ behavior: 'smooth', block: 'start' })); }}
                    class="text-xs text-lapis dark:text-lapis-light hover:underline
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                  >
                    Visual Inspection Checklist (8 items)
                  </a>

                  <!-- ROI mode toggle -->
                  <button
                    type="button"
                    onclick={() => { roiMode = !roiMode; if (!roiMode) { roiRect = null; roiResult = null; roiError = null; } }}
                    class="text-xs px-2 py-1 min-h-[28px] rounded border transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                           focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                           {roiMode
                             ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                             : 'border-border-light dark:border-border-dark text-gray-600 dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50'}"
                    aria-pressed={roiMode}
                    title={roiMode ? 'Exit region selection mode' : 'Select a region of interest to analyse locally'}
                  >
                    {roiMode ? 'Exit Selection' : 'Select Region'}
                  </button>

                  {#if roiMode && roiRect && roiRect.width > 4 && roiRect.height > 4 && !roiLoading}
                    <button
                      type="button"
                      onclick={handleAnalyseRoi}
                      class="text-xs px-2.5 py-1 min-h-[28px] rounded bg-lapis text-white hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors duration-150
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                             focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    >
                      Analyse Region
                    </button>
                  {/if}

                  {#if roiLoading}
                    <span class="flex items-center gap-1.5 text-xs text-flint dark:text-flint-light">
                      <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Analysing region"></span>
                      Analysing...
                    </span>
                  {/if}

                  <!-- Annotate toggle — mutually exclusive with ROI mode -->
                  <button
                    type="button"
                    onclick={() => {
                      if (roiMode) { roiMode = false; roiRect = null; roiResult = null; roiError = null; }
                      annotationMode = !annotationMode;
                    }}
                    class="text-xs px-2 py-1 min-h-[28px] rounded border transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                           focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                           {annotationMode
                             ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                             : 'border-border-light dark:border-border-dark text-gray-600 dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50'}"
                    aria-pressed={annotationMode}
                    title={annotationMode ? 'Exit annotation mode' : 'Draw annotations on the image to mark areas of interest'}
                  >
                    {annotationMode ? 'Exit Annotate' : 'Annotate'}
                  </button>
                </div>

                <!-- Annotation sub-toolbar — shown when annotation mode is active -->
                {#if annotationMode}
                  <div
                    class="mt-2 pt-2 border-t border-border-light/60 dark:border-border-dark/60"
                    role="group"
                    aria-label="Annotation tools"
                  >
                    <div class="flex items-center gap-2 flex-wrap">
                      <!-- Tool selector -->
                      <div class="flex items-center gap-1" role="group" aria-label="Drawing tool">
                        {#each ([
                          { key: 'arrow',     label: 'Arrow',     title: 'Draw an arrow pointing to an area of interest' },
                          { key: 'circle',    label: 'Circle',    title: 'Draw a circle to highlight a region' },
                          { key: 'rectangle', label: 'Rectangle', title: 'Draw a rectangle to frame a region' },
                          { key: 'text',      label: 'Text',      title: 'Place a text label — click to set position' },
                        ] as const) as tool}
                          <button
                            type="button"
                            title={tool.title}
                            onclick={() => { annotationTool = tool.key; }}
                            class="text-xs px-2 py-1 min-h-[28px] rounded border transition-colors duration-150
                                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                                   {annotationTool === tool.key
                                     ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                                     : 'border-border-light dark:border-border-dark text-gray-600 dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50'}"
                            aria-pressed={annotationTool === tool.key}
                          >
                            {tool.label}
                          </button>
                        {/each}
                      </div>

                      <!-- Colour swatches -->
                      <div
                        class="flex items-center gap-1 ml-1"
                        role="group"
                        aria-label="Annotation colour"
                      >
                        <span class="text-xs text-flint dark:text-flint-light flex-shrink-0 mr-0.5">Colour:</span>
                        {#each annotationColours as swatch}
                          <button
                            type="button"
                            title={swatch.label}
                            aria-label="Set annotation colour to {swatch.label}"
                            aria-pressed={annotationColour === swatch.hex}
                            onclick={() => { annotationColour = swatch.hex; }}
                            class="w-5 h-5 min-h-[20px] rounded-full border-2 transition-all duration-150
                                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                                   {annotationColour === swatch.hex
                                     ? 'border-text-light dark:border-quartz scale-110'
                                     : 'border-transparent hover:border-flint/40 dark:hover:border-flint-light/40'}"
                            style="background-color: {swatch.hex};"
                          ></button>
                        {/each}
                      </div>

                      <!-- Clear all annotations -->
                      {#if annotations.length > 0}
                        <button
                          type="button"
                          onclick={clearAllAnnotations}
                          class="text-xs ml-auto px-2 py-1 min-h-[28px] text-flint dark:text-flint-light
                                 hover:text-cinnabar dark:hover:text-cinnabar-light transition-colors duration-150
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                          aria-label="Clear all annotations from this image"
                        >
                          Clear all
                        </button>
                      {/if}
                    </div>

                    <p class="mt-1.5 text-xs text-flint/70 dark:text-flint-light/60" aria-live="polite">
                      {annotations.length === 0
                        ? annotationTool === 'text'
                          ? 'Click on the image to place a text label.'
                          : 'Drag on the image to draw a shape.'
                        : `${annotations.length} annotation${annotations.length === 1 ? '' : 's'} on this image. Hover to delete.`}
                    </p>
                  </div>
                {/if}

                <!-- ROI results panel -->
                {#if roiError}
                  <div class="mt-2 rounded-md px-3 py-2 bg-cinnabar/10 border border-cinnabar/30 text-xs text-cinnabar dark:text-cinnabar-light" role="alert">
                    {roiError}
                  </div>
                {/if}

                {#if roiResult}
                  <div
                    class="mt-2 rounded-md border border-lapis/30 bg-lapis/5 dark:bg-lapis/8 px-3 py-2.5 space-y-1.5"
                    role="region"
                    aria-label="Region of interest analysis results"
                  >
                    <p class="text-xs font-medium text-text-light dark:text-quartz mb-2">Region Analysis</p>
                    <dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
                      <div class="flex justify-between gap-2">
                        <dt class="text-flint dark:text-flint-light">Noise Std</dt>
                        <dd class="tabular-nums font-medium {roiResult.noiseStd > 8 ? 'text-cinnabar dark:text-cinnabar-light' : roiResult.noiseStd > 4 ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}">{roiResult.noiseStd.toFixed(2)}</dd>
                      </div>
                      <div class="flex justify-between gap-2">
                        <dt class="text-flint dark:text-flint-light">Noise Mean</dt>
                        <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{roiResult.noiseMean.toFixed(2)}</dd>
                      </div>
                      <div class="flex justify-between gap-2">
                        <dt class="text-flint dark:text-flint-light">ELA Mean</dt>
                        <dd class="tabular-nums font-medium {roiResult.elaMean > 0.4 ? 'text-cinnabar dark:text-cinnabar-light' : roiResult.elaMean > 0.2 ? 'text-amber dark:text-amber-light' : 'text-malachite dark:text-malachite-light'}">{(roiResult.elaMean * 100).toFixed(1)}%</dd>
                      </div>
                      <div class="flex justify-between gap-2">
                        <dt class="text-flint dark:text-flint-light">Freq. Energy</dt>
                        <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{(roiResult.frequencyEnergy * 100).toFixed(1)}%</dd>
                      </div>
                      <div class="flex justify-between gap-2 col-span-2">
                        <dt class="text-flint dark:text-flint-light">Texture Complexity</dt>
                        <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{(roiResult.textureComplexity * 100).toFixed(1)}%</dd>
                      </div>
                    </dl>
                    <p class="text-xs text-flint/70 dark:text-flint-light/60 pt-1">
                      Region: {roiResult.roi.width}&times;{roiResult.roi.height} px at ({roiResult.roi.x}, {roiResult.roi.y})
                    </p>
                  </div>
                {/if}
              </div>

              {#if elaHeatmapUrl}
                <div class="flex flex-wrap items-center gap-3 pt-1 border-t border-border-light/50 dark:border-border-dark/50">
                  <label class="flex items-center gap-1.5 cursor-pointer select-none min-h-[24px]">
                    <input
                      type="checkbox"
                      bind:checked={showElaOverlay}
                      class="w-3.5 h-3.5 rounded accent-lapis cursor-pointer"
                      aria-describedby="ela-overlay-hint"
                    />
                    <span class="text-xs text-flint dark:text-flint-light">Show ELA overlay</span>
                  </label>
                  {#if showElaOverlay}
                    <div class="flex items-center gap-3 flex-wrap" id="ela-overlay-hint">
                      <div class="flex items-center gap-2">
                        <label class="sr-only" for="ela-opacity-slider">ELA overlay opacity</label>
                        <input
                          id="ela-opacity-slider"
                          type="range"
                          min="10"
                          max="100"
                          step="5"
                          bind:value={elaOpacity}
                          class="w-24 h-1.5 rounded-full accent-lapis cursor-pointer"
                          aria-label="ELA overlay opacity: {elaOpacity}%"
                        />
                        <span class="text-xs tabular-nums text-flint dark:text-flint-light w-8 flex-shrink-0">{elaOpacity}%</span>
                      </div>
                      <div class="flex items-center gap-1.5">
                        <label class="text-xs text-flint dark:text-flint-light" for="ela-blend-mode">Blend:</label>
                        <select
                          id="ela-blend-mode"
                          bind:value={elaBlendMode}
                          class="text-xs border border-border-light dark:border-border-dark rounded px-1.5 py-0.5
                                 bg-white dark:bg-graphite text-gray-700 dark:text-flint-light
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
                          aria-label="ELA overlay blend mode"
                        >
                          <option value="normal">Normal</option>
                          <option value="multiply">Darken</option>
                          <option value="difference">Difference</option>
                        </select>
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          </div>

          <!-- Signal strip — labelled pass/fail indicators, shown in both Summary and Full Analysis views -->
          {#if signalIndicators().length > 0}
            <div
              class="flex flex-col gap-0.5 py-2 flex-shrink-0"
              role="group"
              aria-label="Forensic signal summary — click to jump to section"
            >
              {#each signalIndicators() as signal}
                {@const scoreData = detectorScores()[signal.id]}
                <button
                  onclick={() => {
                    if (viewMode === 'simple') viewMode = 'expert';
                    showTechnicalDetails = true;
                    // Allow DOM update before scrolling
                    requestAnimationFrame(() => scrollToSection(signal.id));
                  }}
                  class="flex items-center gap-1.5 text-xs min-h-[24px] px-1.5 py-0.5 rounded transition-colors duration-150
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                         hover:bg-gray-100 dark:hover:bg-graphite-light/40"
                  title="{signal.name}: {signal.flagged ? 'Flagged' : 'Clean'}{scoreData ? ` — score ${(scoreData.score * 100).toFixed(1)}%` : ''}"
                  aria-label="{signal.name}: {signal.flagged ? 'Flagged' : 'Clean'}{scoreData ? `, score ${(scoreData.score * 100).toFixed(1)} per cent` : ''} — click to view"
                >
                  <span
                    class="w-2.5 h-2.5 rounded-full flex-shrink-0
                           {signal.flagged
                             ? 'bg-cinnabar dark:bg-cinnabar-light'
                             : 'bg-malachite dark:bg-malachite-light'}"
                    aria-hidden="true"
                  ></span>
                  <span class="text-gray-600 dark:text-flint-light whitespace-nowrap">{signal.shortName}</span>
                  {#if showRawScores && scoreData}
                    <span
                      class="tabular-nums text-flint/70 dark:text-flint-light/70 ml-0.5"
                      aria-hidden="true"
                    >
                      {(scoreData.score * 100).toFixed(1)}%
                    </span>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <!-- ── Side-by-side comparison panel ──────────────────────── -->
      {#if comparisonMode && comparisonImageUrl}
        <div
          class="mt-3 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark overflow-hidden"
          role="region"
          aria-label="Side-by-side image comparison"
        >
          <!-- Comparison header -->
          <div class="flex items-center justify-between px-4 py-2.5 border-b border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40">
            <p class="text-xs font-medium text-flint dark:text-flint-light uppercase tracking-wide">
              Side-by-side Comparison
            </p>
            <button
              type="button"
              onclick={closeComparison}
              class="text-xs text-flint dark:text-flint-light hover:text-cinnabar dark:hover:text-cinnabar-light transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-1.5 py-1 min-h-[28px]"
              aria-label="Close comparison view"
            >
              Close
            </button>
          </div>

          <!-- Two-up image grid -->
          <div class="grid grid-cols-2 gap-0 divide-x divide-border-light dark:divide-border-dark">
            <!-- Left: original under examination -->
            <div class="relative bg-gray-100 dark:bg-obsidian/60">
              <img
                src={channelImageUrl ?? previewUrl}
                alt="Original file under examination"
                class="w-full max-h-[380px] object-contain block"
                loading="lazy"
                style="{channelImageUrl ? '' : `filter: ${getFilterStyle(activeFilter)};`}"
              />
              <!-- Overlay label -->
              <div
                class="absolute top-2 left-2 bg-obsidian/75 rounded px-2 py-0.5 pointer-events-none"
                aria-hidden="true"
              >
                <span class="text-xs text-flint-light font-medium">Original</span>
              </div>
            </div>

            <!-- Right: comparison image (plain, no filters) -->
            <div class="relative bg-gray-100 dark:bg-obsidian/60">
              <img
                src={comparisonImageUrl}
                alt="Comparison file"
                class="w-full max-h-[380px] object-contain block"
                loading="lazy"
              />
              <!-- Overlay label -->
              <div
                class="absolute top-2 left-2 bg-obsidian/75 rounded px-2 py-0.5 pointer-events-none"
                aria-hidden="true"
              >
                <span class="text-xs text-flint-light font-medium">Comparison</span>
              </div>
              <!-- Replace comparison button -->
              <button
                type="button"
                onclick={handleLoadComparison}
                class="absolute bottom-2 right-2 text-xs px-2 py-1 min-h-[28px] bg-obsidian/70 rounded
                       text-flint-light hover:bg-lapis/80 hover:text-white transition-colors duration-150
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1
                       focus-visible:ring-offset-obsidian"
                aria-label="Replace comparison image"
              >
                Replace
              </button>
            </div>
          </div>

          <!-- Comparison caption -->
          {#if comparisonFileName}
            <div class="px-4 py-2 border-t border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/30">
              <p class="text-xs text-flint dark:text-flint-light truncate">
                <span class="font-medium">Comparison:</span> {comparisonFileName}
              </p>
            </div>
          {/if}
        </div>
      {/if}
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
                <span class="ml-1 text-lapis dark:text-lapis-light">(via URL)</span>
              {/if}
            </p>
          </div>
        </div>
        <!-- View mode toggle + Raw Scores toggle + Clear -->
        <div class="flex-shrink-0 flex items-center gap-3 flex-wrap justify-end">
          <!-- Raw scores preference toggle -->
          <button
            type="button"
            class="text-xs px-2.5 py-1.5 min-h-[36px] rounded border transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                   focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                   {showRawScores
                     ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light'
                     : 'border-border-light dark:border-border-dark text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}"
            onclick={() => {
              showRawScores = !showRawScores;
              localStorage.setItem('jura-raw-scores-default', String(showRawScores));
            }}
            aria-pressed={showRawScores}
            title={showRawScores ? 'Showing raw numerical scores — click to switch to summary view' : 'Showing traffic-light summary — click to show raw scores'}
          >
            {showRawScores ? 'Technical View' : 'Summary View'}
          </button>
          <div
            class="flex items-center rounded-full border border-border-light dark:border-border-dark overflow-hidden text-xs"
            role="group"
            aria-label="Result view mode"
          >
            <button
              onclick={() => viewMode = 'simple'}
              class="px-3 py-1.5 min-h-[36px] transition-colors duration-150
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                     {viewMode === 'simple'
                       ? 'bg-lapis text-white dark:bg-lapis text-white'
                       : 'text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}"
              aria-pressed={viewMode === 'simple'}
            >
              Simple
            </button>
            <button
              onclick={() => viewMode = 'expert'}
              class="px-3 py-1.5 min-h-[36px] transition-colors duration-150 border-l border-border-light dark:border-border-dark
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                     {viewMode === 'expert'
                       ? 'bg-lapis text-white dark:bg-lapis text-white'
                       : 'text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}"
              aria-pressed={viewMode === 'expert'}
            >
              Expert
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
            <span class="text-xs px-2 py-0.5 rounded bg-amber/10 text-amber dark:text-amber-light border border-amber/20">
              {flag}
            </span>
          {/each}
        </div>
      {/if}

      <!-- ── Sticky section navigation (expert view only) ─────────── -->
      {#if viewMode === 'expert'}
        <nav
          class="sticky top-14 z-20 bg-white/95 dark:bg-graphite/95 backdrop-blur-sm border-b border-border-light dark:border-border-dark py-1.5 px-4 flex gap-1 overflow-x-auto"
          aria-label="Jump to analysis section"
        >
          {#each [
            { id: 'section-verdict', label: 'Verdict', always: true },
            { id: 'section-signals', label: 'Signals', always: true },
            { id: 'section-ela', label: 'ELA', show: !!result.elaResult },
            { id: 'section-noise', label: 'Noise', show: !!result.noiseResult },
            { id: 'section-copymove', label: 'Copy-Move', show: !!result.copyMoveResult },
            { id: 'section-deepfake', label: 'AI Detection', show: !!result.deepfakeResult },
            { id: 'section-c2pa', label: 'C2PA', show: result.c2paValid !== null && result.c2paValid !== undefined },
            { id: 'section-exif', label: 'EXIF', show: !!result.exifAnalysis },
            { id: 'section-npr', label: 'NPR', show: !!result.nprResult },
            { id: 'section-jpegGhost', label: 'JPEG Ghost', show: !!result.jpegGhostResult },
            { id: 'section-ca', label: 'Chromatic', show: !!result.caResult },
            { id: 'section-region', label: 'Regional', show: hasRegionResults },
          ].filter(s => s.always || s.show) as navItem}
            <button
              onclick={() => {
                viewMode = 'expert';
                showTechnicalDetails = true;
                requestAnimationFrame(() => scrollToSection(navItem.id));
              }}
              class="flex-shrink-0 px-2.5 py-1 min-h-[28px] text-xs rounded transition-colors duration-150
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                     {activeSection === navItem.id
                       ? 'bg-lapis/15 text-lapis dark:text-lapis-light font-medium'
                       : 'text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:bg-gray-100 dark:hover:bg-graphite-light/40'}"
              aria-label="Jump to {navItem.label} section"
            >
              {navItem.label}
            </button>
          {/each}
        </nav>
      {/if}

      <!-- ── Simple view ───────────────────────────────────────────── -->
      {#if viewMode === 'simple'}

        <div class="p-5">
          <SimpleVerdict
            {result}
            fileName={fileName ?? 'Unknown file'}
            {sidecarHealth}
            onViewExpert={() => { viewMode = 'expert'; }}
          />

          <!-- Action buttons — export and report -->
          <div class="mt-4 flex flex-wrap items-center gap-3">
            <button
              type="button"
              onclick={() => { showReportModal = true; }}
              class="min-h-[44px] px-4 py-2.5 text-sm font-medium rounded-lg
                     bg-lapis hover:bg-lapis-dark text-white transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                     focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
            >
              {exportingReport ? 'Generating...' : 'Export Report'}
            </button>
            <button
              type="button"
              onclick={handleExportCase}
              disabled={exportingCase}
              class="min-h-[44px] px-4 py-2.5 text-sm font-medium rounded-lg border
                     border-lapis/50 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                     focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                     disabled:opacity-50"
            >
              {exportingCase ? 'Packaging...' : 'Export Case'}
            </button>
            <button
              type="button"
              onclick={() => { showFalsePositiveModal = true; }}
              class="min-h-[44px] px-4 py-2.5 text-sm font-medium rounded-lg border
                     border-border-light dark:border-border-dark text-flint dark:text-flint-light
                     hover:text-text-light dark:hover:text-quartz hover:border-lapis/30 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                     focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
            >
              Report False Positive
            </button>
          </div>

          <!-- AI description — shown beneath the card when available -->
          {#if result.aiDescription}
            <div class="mt-4 rounded-lg border border-lapis/20 bg-lapis/5 px-4 py-3">
              <p class="text-xs font-medium text-lapis dark:text-lapis-light mb-1">AI Description (via Ollama)</p>
              <p class="text-sm text-text-light dark:text-quartz italic leading-relaxed break-words">
                "{result.aiDescription}"
              </p>
            </div>
          {/if}
        </div>

      {:else}

      <!-- ── Back to Simple View (Expert View header) ─────────────── -->
      <div class="px-5 py-3 border-b border-border-light dark:border-border-dark flex items-center justify-between">
        <p class="text-xs text-flint dark:text-flint-light">
          Expert view — full forensic breakdown
        </p>
        <button
          type="button"
          onclick={() => { viewMode = 'simple'; }}
          class="text-xs px-3 py-1.5 min-h-[36px] rounded border border-border-light dark:border-border-dark
                 text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz
                 hover:border-lapis/50 transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis
                 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          aria-label="Return to simple view"
        >
          Simple View
        </button>
      </div>

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
                       ? 'bg-malachite/15 text-malachite dark:text-malachite-light border-malachite/30'
                       : verdict === 'disputed'
                         ? 'bg-cinnabar/15 text-cinnabar dark:text-cinnabar-light border-cinnabar/30'
                         : verdict === 'mixed'
                           ? 'bg-amber/15 text-amber dark:text-amber-light border-amber/30'
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
                    <p class="text-flint/50 dark:text-flint-light/60 tabular-nums mt-0.5">Relevance: {Math.round(source.relevance * 100)}%</p>
                  </div>
                {/each}
              </div>
            </details>
          {/if}
        </div>
      {/if}

      <!-- ── Verdict Summary ──────────────────────────────────────── -->
      <div id="section-verdict" class="px-5 py-4 border-b border-border-light dark:border-border-dark">
        <div class="flex items-center gap-1.5 mb-2">
          <p class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Verdict</p>
          <ContextualHelpLink href="/help/verify#trust-score" label="Learn how the verdict and trust score are calculated" />
        </div>
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

      <!-- ── AI Origin Detection ─────────────────────────────────── -->
      {#if result.aiGenerator !== undefined || result.deepfakeResult !== undefined || result.watermarkExtractResult !== undefined}
        <section
          class="px-5 py-4 border-b border-border-light dark:border-border-dark"
          aria-labelledby="ai-origin-heading"
        >
          <div class="flex items-center gap-1.5 mb-3">
            <h2 id="ai-origin-heading" class="text-sm font-medium text-text-light dark:text-quartz">
              AI Origin Detection
            </h2>
            <ContextualHelpLink href="/help/methodology#detector-reference" label="Learn about AI origin detection methods" />
          </div>
          <div class="space-y-2">

            <!-- C2PA AI Declaration signal -->
            <div class="flex items-center justify-between rounded-lg border border-border-light dark:border-border-dark bg-obsidian/30 px-3 py-2.5">
              <span class="text-xs text-flint dark:text-flint-light">C2PA Declaration</span>
              {#if result.aiGenerator}
                <span
                  class="text-xs font-medium px-2 py-0.5 rounded border bg-amber/15 text-amber dark:text-amber-light border-amber/30"
                  title="Generator: {result.aiGenerator}"
                >
                  AI generation declared
                </span>
              {:else if result.c2paManifest}
                <span class="text-xs font-medium px-2 py-0.5 rounded border bg-malachite/15 text-malachite dark:text-malachite-light border-malachite/30">
                  No AI declaration
                </span>
              {:else}
                <span class="text-xs font-medium px-2 py-0.5 rounded border bg-gray-100 dark:bg-graphite-light text-gray-600 dark:text-flint-light border-gray-200 dark:border-border-dark">
                  No C2PA data
                </span>
              {/if}
            </div>

            <!-- Deepfake Ensemble signal -->
            {#if result.deepfakeResult}
              {@const df = result.deepfakeResult}
              <div class="flex items-center justify-between rounded-lg border border-border-light dark:border-border-dark bg-obsidian/30 px-3 py-2.5">
                <div>
                  <span class="text-xs text-flint dark:text-flint-light">Deepfake Ensemble</span>
                  {#if df.score !== null && df.score !== undefined}
                    <span class="text-xs tabular-nums ml-2 {forensicScoreClass(df.score)}">
                      {Math.round(df.score * 100)}%
                    </span>
                  {/if}
                </div>
                <span
                  class="text-xs font-medium px-2 py-0.5 rounded border
                         {df.verdictLevel === 'synthetic'
                           ? 'bg-cinnabar/15 text-cinnabar dark:text-cinnabar-light border-cinnabar/30'
                           : df.verdictLevel === 'inconclusive'
                             ? 'bg-amber/15 text-amber dark:text-amber-light border-amber/30'
                             : 'bg-malachite/15 text-malachite dark:text-malachite-light border-malachite/30'}"
                >
                  {df.verdictLevel === 'synthetic' ? 'Synthetic' : df.verdictLevel === 'inconclusive' ? 'Inconclusive' : 'Authentic'}
                </span>
              </div>
            {/if}

            <!-- Watermark signal -->
            {#if result.watermarkExtractResult}
              {@const wm = result.watermarkExtractResult}
              <div class="flex items-center justify-between rounded-lg border border-border-light dark:border-border-dark bg-obsidian/30 px-3 py-2.5">
                <span class="text-xs text-flint dark:text-flint-light">Jura Trace Watermark</span>
                {#if wm.hasWatermark}
                  <span
                    class="text-xs font-medium px-2 py-0.5 rounded border bg-malachite/15 text-malachite dark:text-malachite-light border-malachite/30"
                    title={wm.extractedPayload ? `Institution: ${wm.extractedPayload}` : undefined}
                  >
                    {wm.extractedPayload ? wm.extractedPayload : 'Watermark detected'}
                  </span>
                {:else}
                  <span class="text-xs font-medium px-2 py-0.5 rounded border bg-gray-100 dark:bg-graphite-light text-gray-600 dark:text-flint-light border-gray-200 dark:border-border-dark">
                    No watermark detected
                  </span>
                {/if}
              </div>
            {/if}

          </div>
        </section>
      {/if}

      <!-- ── Signal Agreement ─────────────────────────────────────── -->
      <div id="section-signals" class="px-5 py-3 border-b border-border-dark">
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
          <span class="text-xs text-flint/60 dark:text-flint-light/70">detector cross-check</span>
          <ContextualHelpLink href="/help/methodology" label="Learn how signal agreement is calculated across detectors" />
        </button>
        {#if showSignalAgreement}
          <div id="signal-agreement-panel" class="mt-3">
            <SignalAgreement {result} />
          </div>
        {/if}
      </div>

      <!-- ── Visual Inspection Checklist ──────────────────────────── -->
      <div id="inspection-checklist" class="px-5 py-3 border-b border-border-dark">
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
          <span class="text-xs text-flint/60 dark:text-flint-light/70">manual assessment</span>
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
          <span class="text-xs text-flint/60 dark:text-flint-light/70">reverse image search</span>
          {#if licenceTier === 'community'}
            <span class="text-xs text-flint/50 dark:text-flint-light/60 italic">Professional plan includes API-integrated search</span>
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
                  <button
                    type="button"
                    title={link.title}
                    onclick={() => openExternal(link.href)}
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md min-h-[44px]
                           border border-lapis/40 text-lapis dark:text-lapis-light hover:bg-lapis/10 hover:border-lapis/70
                           transition-colors duration-150
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                           focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    aria-label="{link.label} (opens in system browser)"
                  >
                    {link.label}
                    <!-- External link indicator -->
                    <svg class="w-3 h-3 opacity-60" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                    </svg>
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- ── Geolocation & Temporal ────────────────────────────────── -->
      {#if result.contentType === 'image' && result.exifAnalysis}
        {@const exifForGeo = result.exifAnalysis}
        <div class="px-5 py-3 border-b border-border-dark">
          <button
            class="flex items-center gap-2 text-sm text-flint dark:text-flint-light hover:text-quartz transition-colors duration-150
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
            onclick={() => { showGeoPanel = !showGeoPanel; }}
            aria-expanded={showGeoPanel}
            aria-controls="geo-temporal-panel"
          >
            <svg
              class="w-3.5 h-3.5 transition-transform duration-200 {showGeoPanel ? 'rotate-90' : ''}"
              fill="none" stroke="currentColor" viewBox="0 0 24 24"
              aria-hidden="true"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
            Geolocation &amp; Temporal
            {#if gpsCoords}
              <span class="text-xs text-flint/60 dark:text-flint-light/70">sun angle, shadow time, weather</span>
            {:else}
              <span class="text-xs text-flint/50 dark:text-flint-light/60 italic">no GPS data in EXIF</span>
            {/if}
          </button>

          {#if showGeoPanel}
            <div id="geo-temporal-panel" class="mt-3 space-y-4">

              {#if !gpsCoords}
                <!-- No GPS coords available -->
                <p class="text-xs text-flint dark:text-flint-light px-1">
                  No GPS coordinates were found in the EXIF metadata. Sun position, shadow time estimation,
                  and weather cross-referencing require location data embedded in the image.
                </p>
              {:else}
                <!-- GPS coordinates summary -->
                <div class="flex items-center gap-2 px-3 py-2 rounded-md bg-obsidian/30 border border-border-light dark:border-border-dark text-xs">
                  <svg class="w-3.5 h-3.5 flex-shrink-0 text-flint dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                  <span class="tabular-nums text-flint dark:text-flint-light flex-1">
                    {toDMS(gpsCoords.lat, true)}, {toDMS(gpsCoords.lon, false)}
                  </span>
                  <button
                    type="button"
                    class="text-xs text-lapis dark:text-lapis-light hover:underline flex-shrink-0
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                    onclick={() => openExternal(`https://www.openstreetmap.org/?mlat=${gpsCoords.lat}&mlon=${gpsCoords.lon}#map=15/${gpsCoords.lat}/${gpsCoords.lon}`)}
                    aria-label="View GPS location on OpenStreetMap (opens in system browser)"
                  >
                    View on map
                  </button>
                </div>

                <!-- Date/time picker shared by sun position, shadow time, and weather -->
                <fieldset class="rounded-lg border border-border-light dark:border-border-dark bg-white dark:bg-graphite px-4 pt-3 pb-4">
                  <legend class="text-xs font-medium text-text-light dark:text-quartz px-1">Date &amp; Time</legend>
                  <div class="flex flex-wrap items-end gap-4 mt-2">
                    <div>
                      <label for="geo-date-input" class="block text-xs text-flint dark:text-flint-light mb-1">
                        Date <span class="text-flint/50 dark:text-flint-light/60">(YYYY-MM-DD)</span>
                      </label>
                      <input
                        id="geo-date-input"
                        type="date"
                        bind:value={sunDateInput}
                        class="text-xs border border-border-light dark:border-border-dark rounded px-2 py-1.5 min-h-[36px]
                               bg-white dark:bg-obsidian text-text-light dark:text-quartz
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
                      />
                    </div>
                    <div>
                      <label for="geo-hour-input" class="block text-xs text-flint dark:text-flint-light mb-1">
                        Hour UTC <span class="text-flint/50 dark:text-flint-light/60">(0–23)</span>
                      </label>
                      <input
                        id="geo-hour-input"
                        type="number"
                        min="0"
                        max="23"
                        bind:value={sunHourInput}
                        class="w-20 text-xs border border-border-light dark:border-border-dark rounded px-2 py-1.5 min-h-[36px]
                               bg-white dark:bg-obsidian text-text-light dark:text-quartz
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
                        aria-label="Hour in UTC (0 to 23)"
                      />
                    </div>
                    <button
                      type="button"
                      onclick={handleCalculateSunPosition}
                      disabled={sunLoading || !sunDateInput}
                      class="text-xs px-3 py-1.5 min-h-[36px] rounded bg-lapis text-white hover:bg-lapis-dark dark:hover:bg-lapis-light
                             transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                             focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    >
                      {#if sunLoading}
                        <span class="flex items-center gap-1.5">
                          <span class="w-3 h-3 border-2 border-white border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Calculating"></span>
                          Calculating...
                        </span>
                      {:else}
                        Calculate Sun Position
                      {/if}
                    </button>
                  </div>

                  {#if sunError}
                    <p class="mt-2 text-xs text-cinnabar dark:text-cinnabar-light" role="alert">{sunError}</p>
                  {/if}

                  {#if sunPosition}
                    <dl
                      class="mt-3 grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs border-t border-border-light dark:border-border-dark pt-3"
                      aria-label="Solar position results"
                    >
                      <div class="flex justify-between">
                        <dt class="text-flint dark:text-flint-light">Azimuth</dt>
                        <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{sunPosition.azimuth.toFixed(1)}&deg; ({azimuthToCompass(sunPosition.azimuth)})</dd>
                      </div>
                      <div class="flex justify-between">
                        <dt class="text-flint dark:text-flint-light">Elevation</dt>
                        <dd class="tabular-nums font-medium {sunPosition.elevation < 0 ? 'text-flint dark:text-flint-light' : 'text-text-light dark:text-quartz'}">{sunPosition.elevation.toFixed(1)}&deg;</dd>
                      </div>
                      <div class="flex justify-between">
                        <dt class="text-flint dark:text-flint-light">Solar Noon UTC</dt>
                        <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{formatUtcHour(sunPosition.solarNoonUtc)}</dd>
                      </div>
                      <div class="flex justify-between">
                        <dt class="text-flint dark:text-flint-light">Day Length</dt>
                        <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{sunPosition.dayLengthHours.toFixed(2)} hrs</dd>
                      </div>
                    </dl>
                    {#if sunPosition.elevation < 0}
                      <p class="mt-2 text-xs text-amber dark:text-amber-light">The sun is below the horizon at this time and location. No shadows would be cast.</p>
                    {/if}
                  {/if}
                </fieldset>

                <!-- Shadow Time sub-panel -->
                <fieldset class="rounded-lg border border-border-light dark:border-border-dark bg-white dark:bg-graphite px-4 pt-3 pb-4">
                  <legend class="text-xs font-medium text-text-light dark:text-quartz px-1">Shadow Time Estimate</legend>
                  <p class="text-xs text-flint dark:text-flint-light mt-2 mb-3">
                    Measure the direction of a shadow in the image (clockwise from north) and estimate
                    when it was cast. Use the date and GPS coordinates above.
                  </p>
                  <div class="flex flex-wrap items-end gap-4">
                    <div>
                      <label for="shadow-azimuth-input" class="block text-xs text-flint dark:text-flint-light mb-1">
                        Shadow Azimuth <span class="text-flint/50 dark:text-flint-light/60">(0–360&deg;, clockwise from north)</span>
                      </label>
                      <div class="flex items-center gap-2">
                        <input
                          id="shadow-azimuth-input"
                          type="number"
                          min="0"
                          max="360"
                          bind:value={shadowAzimuth}
                          class="w-24 text-xs border border-border-light dark:border-border-dark rounded px-2 py-1.5 min-h-[36px]
                                 bg-white dark:bg-obsidian text-text-light dark:text-quartz
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
                          aria-label="Shadow azimuth in degrees, 0 to 360, clockwise from north"
                        />
                        <span class="text-xs text-flint dark:text-flint-light">({azimuthToCompass(shadowAzimuth)})</span>
                      </div>
                    </div>
                    <button
                      type="button"
                      onclick={handleEstimateShadowTime}
                      disabled={shadowLoading || !sunDateInput}
                      class="text-xs px-3 py-1.5 min-h-[36px] rounded bg-lapis text-white hover:bg-lapis-dark dark:hover:bg-lapis-light
                             transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                             focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    >
                      {#if shadowLoading}
                        <span class="flex items-center gap-1.5">
                          <span class="w-3 h-3 border-2 border-white border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Estimating"></span>
                          Estimating...
                        </span>
                      {:else}
                        Estimate Time
                      {/if}
                    </button>
                  </div>

                  {#if shadowError}
                    <p class="mt-2 text-xs text-cinnabar dark:text-cinnabar-light" role="alert">{shadowError}</p>
                  {/if}

                  {#if shadowTimeResults.length > 0}
                    <div
                      class="mt-3 border-t border-border-light dark:border-border-dark pt-3"
                      aria-label="Shadow time estimates"
                    >
                      <p class="text-xs font-medium text-text-light dark:text-quartz mb-2">
                        Candidate times ({shadowTimeResults.length} found)
                      </p>
                      <ul class="space-y-2">
                        {#each shadowTimeResults as est, i}
                          <li class="flex items-center gap-4 text-xs bg-obsidian/20 dark:bg-obsidian/40 rounded px-3 py-2">
                            <span class="text-flint dark:text-flint-light flex-shrink-0">Option {i + 1}</span>
                            <span class="font-medium text-text-light dark:text-quartz tabular-nums flex-1">{est.timeFormatted}</span>
                            <span class="text-flint dark:text-flint-light tabular-nums">Elev. {est.sunElevation.toFixed(1)}&deg;</span>
                            <span class="text-flint/70 dark:text-flint-light/60 tabular-nums">&plusmn;{est.azimuthError.toFixed(1)}&deg; error</span>
                          </li>
                        {/each}
                      </ul>
                    </div>
                  {:else if !shadowLoading && sunDateInput && shadowTimeResults.length === 0 && shadowError === null}
                    <!-- hint: awaiting user action -->
                  {/if}
                </fieldset>

                <!-- Weather cross-reference -->
                <div class="rounded-lg border border-amber/30 bg-amber/5 dark:bg-amber/8 px-4 pt-3 pb-4">
                  <p class="text-xs font-medium text-amber dark:text-amber-light mb-1">Historical Weather Cross-Reference</p>
                  <p class="text-xs text-amber/80 dark:text-amber-light/70 mb-3 leading-relaxed">
                    This will query the Open-Meteo archive API. Your GPS coordinates and date will be
                    sent to an external service (open-meteo.com). Consider whether this is appropriate
                    for sensitive investigations.
                  </p>

                  {#if !weatherConsentGiven}
                    <button
                      type="button"
                      onclick={() => { weatherConsentGiven = true; handleCheckWeather(); }}
                      disabled={!sunDateInput}
                      class="text-xs px-3 py-1.5 min-h-[36px] rounded border border-amber/50 text-amber dark:text-amber-light
                             hover:bg-amber/10 transition-colors duration-150 disabled:opacity-50 disabled:cursor-not-allowed
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-amber focus-visible:ring-offset-2
                             focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    >
                      Check Historical Weather
                    </button>
                  {/if}

                  {#if weatherLoading}
                    <span class="flex items-center gap-1.5 text-xs text-amber dark:text-amber-light">
                      <span class="w-3 h-3 border-2 border-amber border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Loading weather data"></span>
                      Loading weather data...
                    </span>
                  {/if}

                  {#if weatherError}
                    <p class="mt-2 text-xs text-cinnabar dark:text-cinnabar-light" role="alert">{weatherError}</p>
                  {/if}

                  {#if weatherResult}
                    {#if weatherResult.available && !weatherResult.error}
                      <dl class="grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs mt-2" aria-label="Historical weather data">
                        {#if weatherResult.weatherDescription}
                          <div class="flex justify-between col-span-2">
                            <dt class="text-flint dark:text-flint-light">Conditions</dt>
                            <dd class="font-medium text-text-light dark:text-quartz">{weatherResult.weatherDescription}</dd>
                          </div>
                        {/if}
                        {#if weatherResult.temperatureMaxC != null}
                          <div class="flex justify-between">
                            <dt class="text-flint dark:text-flint-light">Temp max</dt>
                            <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{weatherResult.temperatureMaxC.toFixed(1)}&deg;C</dd>
                          </div>
                        {/if}
                        {#if weatherResult.temperatureMinC != null}
                          <div class="flex justify-between">
                            <dt class="text-flint dark:text-flint-light">Temp min</dt>
                            <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{weatherResult.temperatureMinC.toFixed(1)}&deg;C</dd>
                          </div>
                        {/if}
                        {#if weatherResult.precipitationMm != null}
                          <div class="flex justify-between">
                            <dt class="text-flint dark:text-flint-light">Precipitation</dt>
                            <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{weatherResult.precipitationMm.toFixed(1)} mm</dd>
                          </div>
                        {/if}
                        {#if weatherResult.snowfallCm != null && weatherResult.snowfallCm > 0}
                          <div class="flex justify-between">
                            <dt class="text-flint dark:text-flint-light">Snowfall</dt>
                            <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{weatherResult.snowfallCm.toFixed(1)} cm</dd>
                          </div>
                        {/if}
                        {#if weatherResult.maxWindKmh != null}
                          <div class="flex justify-between">
                            <dt class="text-flint dark:text-flint-light">Max wind</dt>
                            <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{weatherResult.maxWindKmh.toFixed(1)} km/h</dd>
                          </div>
                        {/if}
                      </dl>
                      {#if weatherResult.source || weatherResult.disclaimer}
                        <p class="text-xs text-flint/60 dark:text-flint-light/50 mt-2 leading-relaxed">
                          {#if weatherResult.source}{weatherResult.source}.{/if}
                          {#if weatherResult.disclaimer}{weatherResult.disclaimer}{/if}
                        </p>
                      {/if}
                    {:else}
                      <p class="text-xs text-cinnabar dark:text-cinnabar-light mt-2" role="alert">
                        {weatherResult.error ?? 'Weather data unavailable for this date and location.'}
                      </p>
                    {/if}
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- ── On-demand Investigation: Seasonal & Diffusion ──────────── -->
      {#if result.contentType === 'image' && filePath}
        <div class="px-5 py-3 border-b border-border-dark">
          <p class="text-xs font-medium text-text-light dark:text-quartz mb-2">On-demand Analysis</p>
          <div class="flex flex-wrap gap-2">

            <!-- Seasonal Analysis button -->
            <button
              type="button"
              onclick={handleSeasonalAnalysis}
              disabled={seasonalLoading}
              class="text-xs px-3 py-1.5 min-h-[36px] rounded border border-border-light dark:border-border-dark
                     text-flint dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50
                     hover:text-lapis dark:hover:text-lapis-light transition-colors duration-150
                     disabled:opacity-50 disabled:cursor-not-allowed
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                     focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
              title="Estimate season from vegetation, snow, and colour temperature signals in the image"
            >
              {#if seasonalLoading}
                <span class="flex items-center gap-1.5">
                  <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Analysing"></span>
                  Analysing...
                </span>
              {:else}
                Seasonal Analysis
              {/if}
            </button>

            <!-- Diffusion Artefacts button -->
            <button
              type="button"
              onclick={handleDiffusionCheck}
              disabled={diffusionLoading}
              class="text-xs px-3 py-1.5 min-h-[36px] rounded border border-border-light dark:border-border-dark
                     text-flint dark:text-flint-light hover:border-lapis/50 dark:hover:border-lapis-light/50
                     hover:text-lapis dark:hover:text-lapis-light transition-colors duration-150
                     disabled:opacity-50 disabled:cursor-not-allowed
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                     focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
              title="Check for diffusion model artefacts — texture smoothness, VAE banding, resolution inconsistencies"
            >
              {#if diffusionLoading}
                <span class="flex items-center gap-1.5">
                  <span class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin" role="status" aria-label="Checking"></span>
                  Checking...
                </span>
              {:else}
                Check Diffusion Artefacts
              {/if}
            </button>
          </div>

          <!-- Seasonal result -->
          {#if seasonalError}
            <div class="mt-2 rounded-md px-3 py-2 bg-cinnabar/10 border border-cinnabar/30 text-xs text-cinnabar dark:text-cinnabar-light" role="alert">
              {seasonalError}
            </div>
          {/if}

          {#if seasonalResult}
            <div
              class="mt-3 rounded-lg border border-border-light dark:border-border-dark bg-white dark:bg-graphite px-4 pt-3 pb-4"
              role="region"
              aria-label="Seasonal analysis results"
            >
              <div class="flex items-center gap-3 mb-3">
                <p class="text-xs font-medium text-text-light dark:text-quartz">Seasonal Analysis</p>
                <span class="text-xs px-2 py-0.5 rounded font-medium bg-lapis/10 text-lapis dark:text-lapis-light border border-lapis/20">
                  {seasonalResult.estimatedSeason}
                </span>
                <span class="text-xs text-flint dark:text-flint-light">
                  {Math.round(seasonalResult.confidence * 100)}% confidence
                </span>
              </div>
              <dl class="grid grid-cols-3 gap-x-4 gap-y-1 text-xs mb-3">
                <div class="flex flex-col gap-0.5">
                  <dt class="text-flint dark:text-flint-light">Greenness</dt>
                  <dd class="tabular-nums font-medium {seasonalResult.greennessIndex > 0.5 ? 'text-malachite dark:text-malachite-light' : 'text-text-light dark:text-quartz'}">{(seasonalResult.greennessIndex * 100).toFixed(1)}%</dd>
                </div>
                <div class="flex flex-col gap-0.5">
                  <dt class="text-flint dark:text-flint-light">Snow Coverage</dt>
                  <dd class="tabular-nums font-medium {seasonalResult.snowCoverage > 0.3 ? 'text-lapis dark:text-lapis-light' : 'text-text-light dark:text-quartz'}">{(seasonalResult.snowCoverage * 100).toFixed(1)}%</dd>
                </div>
                <div class="flex flex-col gap-0.5">
                  <dt class="text-flint dark:text-flint-light">Warmth Index</dt>
                  <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{(seasonalResult.warmthIndex * 100).toFixed(1)}%</dd>
                </div>
              </dl>
              {#if seasonalResult.indicators.length > 0}
                <ul class="flex flex-wrap gap-1.5" aria-label="Supporting indicators">
                  {#each seasonalResult.indicators as indicator}
                    <li class="text-xs px-2 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light">{indicator}</li>
                  {/each}
                </ul>
              {/if}
            </div>
          {/if}

          <!-- Diffusion artefacts result -->
          {#if diffusionError}
            <div class="mt-2 rounded-md px-3 py-2 bg-cinnabar/10 border border-cinnabar/30 text-xs text-cinnabar dark:text-cinnabar-light" role="alert">
              {diffusionError}
            </div>
          {/if}

          {#if diffusionResult}
            {@const diffScore = diffusionResult.overallDiffusionScore}
            <div
              class="mt-3 rounded-lg border border-border-light dark:border-border-dark bg-white dark:bg-graphite px-4 pt-3 pb-4"
              role="region"
              aria-label="Diffusion artefact analysis results"
            >
              <div class="flex items-center gap-3 mb-3">
                <p class="text-xs font-medium text-text-light dark:text-quartz">Diffusion Artefacts</p>
                <span class="text-xs px-2 py-0.5 rounded font-medium
                             {diffScore >= 0.5
                               ? 'bg-cinnabar/10 text-cinnabar dark:text-cinnabar-light border border-cinnabar/20'
                               : diffScore >= 0.3
                                 ? 'bg-amber/10 text-amber dark:text-amber-light border border-amber/20'
                                 : 'bg-malachite/10 text-malachite dark:text-malachite-light border border-malachite/20'}">
                  {diffScore >= 0.5 ? 'Likely Diffusion' : diffScore >= 0.3 ? 'Possible Diffusion' : 'Low Signal'}
                </span>
                <span class="text-xs text-flint dark:text-flint-light tabular-nums">{(diffScore * 100).toFixed(1)}%</span>
              </div>
              <dl class="grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs mb-3">
                <div class="flex justify-between">
                  <dt class="text-flint dark:text-flint-light">Texture Smoothness</dt>
                  <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{(diffusionResult.textureSmoothnessScore * 100).toFixed(1)}%</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-flint dark:text-flint-light">VAE Banding</dt>
                  <dd class="tabular-nums font-medium text-text-light dark:text-quartz">{(diffusionResult.vaeBandingScore * 100).toFixed(1)}%</dd>
                </div>
                <div class="flex justify-between col-span-2">
                  <dt class="text-flint dark:text-flint-light">Resolution</dt>
                  <dd class="font-medium {diffusionResult.resolutionMatch ? 'text-malachite dark:text-malachite-light' : 'text-amber dark:text-amber-light'}">
                    {diffusionResult.resolutionNote}
                  </dd>
                </div>
              </dl>
              <p class="text-xs text-flint/60 dark:text-flint-light/50 italic">
                Diffusion artefact detection is an experimental signal. Treat results as one indicator among many.
              </p>
            </div>
          {/if}
        </div>
      {/if}

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
          class="px-4 py-2.5 min-h-[44px] text-sm border border-lapis/50 text-lapis dark:text-lapis-light rounded
                 hover:bg-lapis/10 transition-colors
                 disabled:opacity-50 disabled:cursor-not-allowed
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          onclick={handleExportCase}
          disabled={exportingCase}
        >
          {exportingCase ? 'Packaging...' : 'Export Case'}
        </button>
        <span class="text-xs text-flint/50 dark:text-flint-light/60">
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
          Detailed Forensic Results
          <a
            href="/help/methodology"
            class="text-xs text-lapis dark:text-lapis-light hover:underline ml-1"
            onclick={(e) => e.stopPropagation()}
          >
            What do these mean?
          </a>
        </button>
      </div>

      {#if showTechnicalDetails}
      <div id="technical-details">

      <!-- ── ELA Analysis ──────────────────────────────────────────── -->
      {#if result.elaResult}
        {@const ela = result.elaResult}
        <section id="section-ela" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="ela-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="ela-heading" class="text-sm font-medium text-text-light dark:text-quartz">Error Level Analysis</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(ela.score)} {forensicScoreClass(ela.score)}"
              >
                {ela.suspicious ? 'Suspicious' : 'Normal'}
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs tabular-nums {forensicScoreClass(ela.score)}">
              <span>{(ela.score * 100).toFixed(1)}%</span>
              {#if showRawScores}
                <span class="text-flint dark:text-flint-light font-normal" aria-label="threshold 40 per cent">
                  / threshold 40%
                </span>
              {/if}
            </div>
          </div>

          <!-- ELA heatmap — side-by-side with original when preview is available -->
          {#if previewUrl}
            <div class="mb-3 grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <p class="text-xs text-flint dark:text-flint-light mb-1">Original</p>
                <div class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
                  <img
                    src={previewUrl}
                    alt="Original analysed file"
                    class="w-full max-h-64 object-contain"
                  />
                </div>
              </div>
              <div>
                <p class="text-xs text-flint dark:text-flint-light mb-1">Error Level Analysis</p>
                <div class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
                  <img
                    src={blobs.url(ela.elaImageBase64, 'image/png')}
                    alt="Error Level Analysis heatmap — bright areas indicate higher compression artefact differences"
                    class="w-full max-h-64 object-contain"
                  />
                </div>
              </div>
            </div>
          {:else}
            <div class="mb-3 rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
              <img
                src={blobs.url(ela.elaImageBase64, 'image/png')}
                alt="Error Level Analysis heatmap — bright areas indicate higher compression artefact differences"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

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
            <div class="mt-3 text-xs text-amber dark:text-amber-light bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
              Elevated compression artefact variation detected. This may indicate pixel-level editing
              or compositing. Consider alongside other verification signals.
            </div>
          {/if}
        </section>
      {:else if checked && !sidecarAvailable}
        <section class="px-5 py-3 border-b border-border-dark" aria-labelledby="ela-heading">
          <div class="flex items-center gap-3">
            <h2 id="ela-heading" class="text-sm font-medium text-text-light dark:text-quartz">Error Level Analysis</h2>
            <span class="text-xs text-gray-700 dark:text-flint-light bg-gray-100 dark:bg-graphite-light px-2 py-0.5 rounded border border-gray-300 dark:border-border-dark">
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
        <section id="section-noise" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="noise-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="noise-heading" class="text-sm font-medium text-text-light dark:text-quartz">Noise Analysis</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(noise.score)} {forensicScoreClass(noise.score)}"
              >
                {noise.suspicious ? 'Suspicious' : 'Normal'}
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs tabular-nums {forensicScoreClass(noise.score)}">
              <span>{(noise.score * 100).toFixed(1)}%</span>
              {#if showRawScores}
                <span class="text-flint dark:text-flint-light font-normal" aria-label="threshold 40 per cent">
                  / threshold 40%
                </span>
              {/if}
            </div>
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
            <div class="mt-3 text-xs text-amber dark:text-amber-light bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
              Inconsistent noise patterns detected across image blocks. This may indicate region-level
              editing, splicing, or inpainting. Consider alongside other verification signals.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── Copy-Move Detection ───────────────────────────────────── -->
      {#if result.copyMoveResult}
        {@const cm = result.copyMoveResult}
        <section id="section-copymove" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="copymove-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="copymove-heading" class="text-sm font-medium text-text-light dark:text-quartz">Copy-Move Detection</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(cm.score)} {forensicScoreClass(cm.score)}"
              >
                {cm.suspicious ? 'Suspicious' : 'Clean'}
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs tabular-nums {forensicScoreClass(cm.score)}">
              <span>{(cm.score * 100).toFixed(1)}%</span>
              {#if showRawScores}
                <span class="text-flint dark:text-flint-light font-normal" aria-label="threshold 40 per cent">
                  / threshold 40%
                </span>
              {/if}
            </div>
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
            <div class="mt-3 text-xs text-cinnabar dark:text-cinnabar-light bg-cinnabar/10 border border-cinnabar/20 rounded-md px-3 py-2">
              Duplicated regions detected within the image. This is a strong indicator of copy-move
              forgery — content appears to have been cloned from one area to another.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── Region Analysis ───────────────────────────────────────── -->
      {#if hasRegionResults && result}
        <section id="section-region" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="region-analysis-heading">

          <!-- Section header with expand/collapse toggle -->
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="region-analysis-heading" class="text-sm font-medium text-text-light dark:text-quartz">Region Analysis</h2>
              <!-- Summary badge: n of m suspicious -->
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border
                       {regionSuspiciousCount === 0
                         ? 'bg-malachite/15 border-malachite/20 text-malachite dark:text-malachite-light'
                         : regionSuspiciousCount >= 2
                           ? 'bg-cinnabar/15 border-cinnabar/20 text-cinnabar dark:text-cinnabar-light'
                           : 'bg-amber/15 border-amber/20 text-amber dark:text-amber-light'}"
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
                      <div class="text-xs text-amber dark:text-amber-light bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
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
                      <div class="text-xs text-amber dark:text-amber-light bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
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
                      <div class="text-xs text-amber dark:text-amber-light bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
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
                        <summary class="text-xs text-lapis dark:text-lapis-light cursor-pointer hover:text-lapis-dark dark:hover:text-lapis-light transition-colors">
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
                                         : 'bg-gray-50 border-gray-200 dark:bg-graphite-light/50 dark:border-border-dark'}"
                              role="listitem"
                            >
                              <div class="flex items-center justify-between gap-2 mb-1">
                                <span class="font-mono text-gray-800 dark:text-quartz">
                                  ({boundary.x}, {boundary.y}) &mdash; {boundary.width}&times;{boundary.height}px
                                </span>
                                <span class="tabular-nums text-gray-500 dark:text-flint-light">{(boundary.confidence * 100).toFixed(0)}% confidence</span>
                              </div>
                              <div class="flex flex-wrap gap-x-3 gap-y-0.5 text-gray-600 dark:text-flint-light">
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
                      <div class="text-xs text-cinnabar dark:text-cinnabar-light bg-cinnabar/10 border border-cinnabar/20 rounded-md px-3 py-2">
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
        <section id="section-npr" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="npr-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="npr-heading" class="text-sm font-medium text-text-light dark:text-quartz">Neighbouring Pixel Relationships</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(npr.score)} {forensicScoreClass(npr.score)}"
              >
                {npr.suspicious ? 'Suspicious' : 'Clean'}
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs tabular-nums {forensicScoreClass(npr.score)}">
              <span>{(npr.score * 100).toFixed(1)}%</span>
              {#if showRawScores}
                <span class="text-flint dark:text-flint-light font-normal" aria-label="threshold 40 per cent">
                  / threshold 40%
                </span>
              {/if}
            </div>
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
            <div class="mt-3 text-xs text-amber dark:text-amber-light bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
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
        <section id="section-jpegGhost" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="jpegGhost-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="jpegGhost-heading" class="text-sm font-medium text-text-light dark:text-quartz">JPEG Ghost Detection</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(jg.score)} {forensicScoreClass(jg.score)}"
              >
                {jg.suspicious ? 'Suspicious' : 'Clean'}
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs tabular-nums {forensicScoreClass(jg.score)}">
              <span>{(jg.score * 100).toFixed(1)}%</span>
              {#if showRawScores}
                <span class="text-flint dark:text-flint-light font-normal" aria-label="threshold 40 per cent">
                  / threshold 40%
                </span>
              {/if}
            </div>
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
            <div class="mt-3 text-xs text-amber dark:text-amber-light bg-amber/10 border border-amber/20 rounded-md px-3 py-2">
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
        <section id="section-ca" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="ca-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="ca-heading" class="text-sm font-medium text-text-light dark:text-quartz">Chromatic Aberration</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border
                       {ca.isConsistent
                         ? 'bg-malachite/15 border-malachite/20 text-malachite dark:text-malachite-light'
                         : 'bg-amber/15 border-amber/20 text-amber dark:text-amber-light'}"
              >
                {ca.isConsistent ? 'Consistent' : 'Inconsistent'}
              </span>
              <!-- Informational tag — always shown -->
              <span
                class="text-xs px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-gray-700 dark:text-flint-light border border-gray-300 dark:border-border-dark"
                title="Chromatic aberration analysis is informational only — results may be unreliable for mobile phone photos processed with computational lens correction"
              >
                Informational
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs tabular-nums {forensicScoreClass(ca.score)}">
              <span>{(ca.score * 100).toFixed(1)}%</span>
              {#if showRawScores}
                <span class="text-flint dark:text-flint-light font-normal" aria-label="threshold 50 per cent">
                  / threshold 50%
                </span>
              {/if}
            </div>
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

          <p class="text-xs text-flint/60 dark:text-flint-light/70 italic leading-relaxed">
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
                class="text-xs font-medium px-2 py-0.5 rounded border bg-amber/10 text-amber dark:text-amber-light border-amber/30"
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
        <section id="section-deepfake" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="deepfake-heading">
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
                class="text-xs px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-gray-700 dark:text-flint-light border border-gray-300 dark:border-border-dark"
                title="Confidence level of the detection"
              >
                {df.confidence} confidence
              </span>
            </div>
            <div class="flex items-center gap-2 text-xs tabular-nums {forensicScoreClass(df.score)}">
              <span>{(df.score * 100).toFixed(1)}%</span>
              {#if showRawScores}
                <span class="text-flint dark:text-flint-light font-normal" aria-label="threshold 50 per cent">
                  / threshold 50%
                </span>
              {/if}
            </div>
          </div>

          <!-- Frequency spectrum heatmap — side-by-side with original when preview is available -->
          {#if df.heatmapBase64}
            {#if previewUrl}
              <div class="mb-3 grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div>
                  <p class="text-xs text-flint dark:text-flint-light mb-1">Original</p>
                  <div class="rounded-md overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian">
                    <img
                      src={previewUrl}
                      alt="Original analysed file"
                      class="w-full max-h-64 object-contain"
                    />
                  </div>
                </div>
                <div>
                  <p class="text-xs text-flint dark:text-flint-light mb-1">Frequency Spectrum</p>
                  <div class="rounded-md overflow-hidden border border-border-dark bg-obsidian">
                    <img
                      src={blobs.url(df.heatmapBase64, 'image/png')}
                      alt="Frequency spectrum heatmap — anomalous patterns may indicate AI generation"
                      class="w-full max-h-64 object-contain"
                    />
                  </div>
                </div>
              </div>
            {:else}
              <div class="mb-3 rounded-md overflow-hidden border border-border-dark bg-obsidian">
                <img
                  src={blobs.url(df.heatmapBase64, 'image/png')}
                  alt="Frequency spectrum heatmap — anomalous patterns may indicate AI generation"
                  class="w-full max-h-64 object-contain"
                />
              </div>
            {/if}
          {/if}

          <!-- Summary -->
          <p class="text-xs text-flint dark:text-flint-light mb-3">{df.summary}</p>

          <!-- Signal list -->
          {#if df.signals.length > 0}
            <details class="group">
              <summary class="text-xs text-lapis dark:text-lapis-light cursor-pointer hover:text-lapis-dark dark:hover:text-lapis-light transition-colors">
                {df.signals.filter(s => s.triggered).length} of {df.signals.length} signals triggered — view details
              </summary>
              <div class="mt-2 space-y-1.5" role="list" aria-label="Detection signals">
                {#each df.signals as signal (signal.name)}
                  <div
                    class="flex items-start gap-2 rounded-md px-3 py-2 text-xs
                           {signal.triggered
                             ? 'bg-amber/10 border border-amber/20'
                             : 'bg-gray-50 border border-gray-200 dark:bg-graphite-light/50 dark:border-border-dark'}"
                    role="listitem"
                  >
                    <span
                      class="flex-shrink-0 w-1.5 h-1.5 mt-1 rounded-full
                             {signal.triggered ? 'bg-amber dark:bg-amber-light' : 'bg-gray-400 dark:bg-flint/50'}"
                      aria-hidden="true"
                    ></span>
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center justify-between gap-2">
                        <span class="font-mono {signal.triggered ? 'text-amber-dark dark:text-amber-light' : 'text-gray-800 dark:text-flint-light'}">{signal.name}</span>
                        <div class="flex items-center gap-2 flex-shrink-0">
                          {#if !signal.triggered}
                            <span class="px-1.5 py-0.5 rounded border border-gray-300 dark:border-border-dark bg-white dark:bg-graphite text-gray-700 dark:text-flint-light">
                              Clear
                            </span>
                          {/if}
                          <span class="text-gray-500 dark:text-flint-light/70 tabular-nums">weight: {signal.weight.toFixed(1)}</span>
                        </div>
                      </div>
                      <p class="text-gray-600 dark:text-flint-light mt-0.5">{signal.description}</p>
                    </div>
                  </div>
                {/each}
              </div>
            </details>
          {/if}

          {#if df.suspicious}
            <div class="mt-3 text-xs text-cinnabar dark:text-cinnabar-light bg-cinnabar/10 border border-cinnabar/20 rounded-md px-3 py-2">
              Multiple statistical signals suggest this image may be AI-generated or synthetically produced.
              Consider alongside other verification signals and the specific context of use.
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── EXIF Analysis ─────────────────────────────────────────── -->
      {#if result.exifAnalysis}
        {@const exif = result.exifAnalysis}
        <section id="section-exif" class="px-5 py-4 border-b border-border-light dark:border-border-dark" aria-labelledby="exif-heading">
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
                <span class="text-amber dark:text-amber-light ml-1">— no EXIF data present</span>
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

          <!-- GPS coordinates panel — enhanced -->
          {#if exif.gpsLatitude != null && exif.gpsLongitude != null}
            {@const lat = exif.gpsLatitude}
            {@const lon = exif.gpsLongitude}
            <div class="mt-3 rounded-lg border border-border-light dark:border-border-dark bg-white dark:bg-graphite overflow-hidden">
              <!-- Header row -->
              <div class="flex items-center gap-2 px-3 py-2 border-b border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/40">
                <svg class="w-3.5 h-3.5 flex-shrink-0 text-lapis dark:text-lapis-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                <span class="text-xs font-semibold text-text-light dark:text-quartz">GPS Location</span>
              </div>

              <!-- Coordinate grid -->
              <div class="px-3 py-2.5 grid grid-cols-2 gap-x-6 gap-y-2">
                <div>
                  <p class="text-[10px] uppercase tracking-wide text-flint/60 dark:text-flint-light/60 mb-0.5">DMS</p>
                  <p class="text-xs tabular-nums text-flint dark:text-flint-light leading-snug">
                    {toDMS(lat, true)}<br />{toDMS(lon, false)}
                  </p>
                </div>
                <div>
                  <p class="text-[10px] uppercase tracking-wide text-flint/60 dark:text-flint-light/60 mb-0.5">Decimal degrees</p>
                  <p class="text-xs tabular-nums text-flint dark:text-flint-light leading-snug">
                    {lat.toFixed(6)}<br />{lon.toFixed(6)}
                  </p>
                </div>
              </div>

              <!-- Action buttons -->
              <div class="flex flex-wrap gap-2 px-3 pb-3">
                <!-- Copy coordinates -->
                <button
                  type="button"
                  onclick={() => copyGpsCoords(lat, lon)}
                  class="inline-flex items-center gap-1.5 text-xs px-2.5 py-1.5 min-h-[32px] rounded border
                         border-border-light dark:border-border-dark
                         text-flint dark:text-flint-light
                         hover:border-lapis/40 hover:text-lapis dark:hover:text-lapis-light
                         transition-colors duration-150
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                         focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                  aria-label={gpsCopied ? 'Coordinates copied' : 'Copy decimal coordinates to clipboard'}
                >
                  {#if gpsCopied}
                    <!-- Tick icon -->
                    <svg class="w-3 h-3 text-malachite dark:text-malachite-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
                    </svg>
                    <span class="text-malachite dark:text-malachite-light">Copied</span>
                  {:else}
                    <!-- Clipboard icon -->
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2" />
                    </svg>
                    Copy coordinates
                  {/if}
                </button>

                <!-- View on OpenStreetMap -->
                <button
                  type="button"
                  onclick={() => openExternal(`https://www.openstreetmap.org/?mlat=${lat}&mlon=${lon}#map=15/${lat}/${lon}`)}
                  class="inline-flex items-center gap-1.5 text-xs px-2.5 py-1.5 min-h-[32px] rounded border
                         border-lapis/30 bg-lapis/10 text-lapis dark:text-lapis-light
                         hover:bg-lapis/20 hover:border-lapis/50
                         transition-colors duration-150
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                         focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                  aria-label="View GPS location on OpenStreetMap (opens in system browser)"
                >
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                  OpenStreetMap
                  <span class="sr-only">(opens in system browser)</span>
                </button>

                <!-- View on Google Earth -->
                <button
                  type="button"
                  onclick={() => openExternal(`https://earth.google.com/web/@${lat},${lon},0a,1000d,35y,0h,0t,0r`)}
                  class="inline-flex items-center gap-1.5 text-xs px-2.5 py-1.5 min-h-[32px] rounded border
                         border-border-light dark:border-border-dark
                         text-flint dark:text-flint-light
                         hover:border-lapis/40 hover:text-lapis dark:hover:text-lapis-light
                         transition-colors duration-150
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2
                         focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                  aria-label="View GPS location on Google Earth (opens in system browser)"
                >
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <circle cx="12" cy="12" r="10" stroke-width="2" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10A15.3 15.3 0 018 12a15.3 15.3 0 014-10z" />
                  </svg>
                  Google Earth
                  <span class="sr-only">(opens in system browser)</span>
                </button>
              </div>
            </div>
          {/if}
        </section>
      {/if}

      <!-- ── C2PA Credentials ──────────────────────────────────────── -->
      {#if result.c2paManifest}
        {@const manifest = result.c2paManifest}
        <section id="section-c2pa" class="px-5 py-4" aria-labelledby="c2pa-heading">
          <div class="flex items-center gap-3 mb-4">
            <h2 id="c2pa-heading" class="text-sm font-medium text-text-light dark:text-quartz">C2PA Credentials</h2>
            <ContextualHelpLink href="/help/verify#provenance" label="Learn about C2PA Content Credentials and provenance" />
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
                <span class="normal-case ml-1 text-flint/70 dark:text-flint-light/70">({manifest.assertions.length})</span>
              </h3>
              <div class="space-y-2" role="list" aria-label="C2PA assertions">
                {#each manifest.assertions as assertion (assertion.label)}
                  <div class="bg-gray-100 dark:bg-obsidian/50 rounded-md p-3" role="listitem">
                    <p class="text-xs font-mono text-lapis dark:text-lapis-light mb-1 break-all">{assertion.label}</p>
                    <pre class="text-xs text-flint dark:text-flint-light whitespace-pre-wrap break-words leading-relaxed">{assertion.value}</pre>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- ── Provenance Chain Timeline ─────────────────────────── -->
          <div class="mt-4">
            <h3 class="text-xs text-flint dark:text-flint-light uppercase tracking-wide mb-3">Provenance Chain</h3>

            <!--
              The C2PA manifest exposed by Jura Trace contains a single claim record.
              Multi-claim ingredient history requires a coalitioned C2PA SDK that traverses
              nested ingredient manifests — this is planned for a future release.
              For now, render the single claim as a one-step timeline and note the absence of history.
            -->
            <ol class="relative" aria-label="Provenance timeline">
              <!-- Single claim node -->
              <li class="relative pl-6 pb-2">
                <!-- Vertical connector line — hidden for single-item list -->
                <span
                  class="absolute left-[7px] top-[18px] bottom-0 w-px bg-border-light dark:bg-border-dark"
                  aria-hidden="true"
                ></span>
                <!-- Dot -->
                <span
                  class="absolute left-0 top-1 w-3.5 h-3.5 rounded-full border-2 flex items-center justify-center
                         {manifest.isValid
                           ? 'border-malachite bg-malachite/15'
                           : 'border-cinnabar bg-cinnabar/15'}"
                  aria-hidden="true"
                ></span>

                <div class="bg-gray-50 dark:bg-obsidian/30 rounded-md border border-border-light dark:border-border-dark px-3 py-2">
                  <p class="text-xs font-semibold text-text-light dark:text-quartz mb-0.5">
                    Signed
                    {#if manifest.isValid}
                      <span class="text-malachite dark:text-malachite-light font-medium">(valid)</span>
                    {:else}
                      <span class="text-cinnabar dark:text-cinnabar-light font-medium">(invalid)</span>
                    {/if}
                  </p>
                  {#if manifest.claimGenerator}
                    <p class="text-xs text-flint dark:text-flint-light">
                      <span class="text-flint/60 dark:text-flint-light/60">Generator:</span>
                      {manifest.claimGenerator}
                    </p>
                  {/if}
                  {#if manifest.signedAt}
                    <p class="text-xs text-flint dark:text-flint-light">
                      <span class="text-flint/60 dark:text-flint-light/60">Date:</span>
                      {formatSignedAt(manifest.signedAt)}
                    </p>
                  {/if}
                </div>
              </li>
            </ol>

            <!-- Ingredient history note -->
            <p class="mt-2 text-xs text-flint/70 dark:text-flint-light/60 italic leading-relaxed">
              Single claim — no prior provenance history embedded. Full ingredient chain traversal
              requires multi-manifest C2PA records created by compatible tools (e.g. Adobe Firefly,
              Leica cameras, or Content Credentials enabled at capture).
            </p>
          </div>
        </section>

      {:else if checked}
        <section class="px-5 py-4" aria-labelledby="c2pa-heading">
          <div class="flex items-center gap-3 mb-3">
            <h2 id="c2pa-heading" class="text-sm font-medium text-text-light dark:text-quartz">C2PA Credentials</h2>
            <span class="text-xs font-medium px-2 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-gray-700 dark:text-flint-light border border-gray-300 dark:border-border-dark">
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
                       : 'bg-gray-100 dark:bg-graphite-light text-gray-700 dark:text-flint-light border-gray-300 dark:border-border-dark'}"
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
                {vd.aggregateVerdict === 'authentic' ? 'bg-malachite/10 text-malachite dark:text-malachite-light' :
                 vd.aggregateVerdict === 'synthetic' ? 'bg-cinnabar/10 text-cinnabar dark:text-cinnabar-light' :
                 'bg-amber/10 text-amber dark:text-amber-light'}"
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
                      class="absolute bottom-1 right-1 inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium
                             {fr.verdictLevel === 'authentic'
                               ? 'text-malachite-light bg-malachite/20'
                               : fr.verdictLevel === 'synthetic'
                                 ? 'text-cinnabar-light bg-cinnabar/20'
                                 : 'text-amber-light bg-amber/20'}"
                    >
                      {(fr.score * 100).toFixed(0)}%
                    </span>

                    <!-- Score bar -->
                    <div class="h-1.5 w-full bg-graphite/20">
                      <div
                        class="h-full transition-all
                               {fr.verdictLevel === 'authentic' ? 'bg-malachite dark:bg-malachite-light' :
                                fr.verdictLevel === 'synthetic' ? 'bg-cinnabar dark:bg-cinnabar-light' :
                                'bg-amber dark:bg-amber-light'}"
                        style="width: {Math.max(2, fr.score * 100)}%;"
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
                        <span class="text-xs tabular-nums {fr.verdictLevel === 'authentic' ? 'text-malachite dark:text-malachite-light' : fr.verdictLevel === 'synthetic' ? 'text-cinnabar dark:text-cinnabar-light' : 'text-amber dark:text-amber-light'}">
                          {fr.verdictLevel.charAt(0).toUpperCase() + fr.verdictLevel.slice(1)} ({(fr.score * 100).toFixed(1)}%)
                        </span>
                      </div>

                      {#if fr.classifierAvailable && fr.classifierScore != null}
                        <div class="text-xs text-flint dark:text-flint-light">
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
                          <span class="text-xs font-medium text-gray-600 dark:text-flint-light uppercase tracking-wider">Signals</span>
                          {#each fr.signals as signal}
                            <div class="flex items-center gap-2 text-xs">
                              <span
                                class="w-1.5 h-1.5 rounded-full flex-shrink-0
                                       {signal.triggered ? 'bg-cinnabar dark:bg-cinnabar-light' : 'bg-malachite dark:bg-malachite-light'}"
                                aria-hidden="true"
                              ></span>
                              <span class="text-gray-700 dark:text-flint-light flex-1">{signal.name}</span>
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
            <div class="flex flex-wrap items-center gap-4 text-xs text-flint dark:text-flint-light mb-2" role="img" aria-label="Score badge colour key: Authentic below 35%, Inconclusive 35 to 60%, Synthetic above 60%">
              <span class="flex items-center gap-1" aria-hidden="true">
                <span class="inline-block w-2 h-2 rounded-full bg-malachite dark:bg-malachite-light"></span>
                Authentic (&lt;35%)
              </span>
              <span class="flex items-center gap-1" aria-hidden="true">
                <span class="inline-block w-2 h-2 rounded-full bg-amber dark:bg-amber-light"></span>
                Inconclusive (35-60%)
              </span>
              <span class="flex items-center gap-1" aria-hidden="true">
                <span class="inline-block w-2 h-2 rounded-full bg-cinnabar dark:bg-cinnabar-light"></span>
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
          <p class="text-xs text-cinnabar dark:text-cinnabar-light">
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
              <summary class="cursor-pointer text-xs font-medium text-lapis dark:text-lapis-light hover:underline">
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
              {cc.overallVerdict === 'supported' ? 'bg-malachite/10 text-malachite dark:text-malachite-light' :
               cc.overallVerdict === 'disputed' ? 'bg-cinnabar/10 text-cinnabar dark:text-cinnabar-light' :
               cc.overallVerdict === 'mixed' ? 'bg-amber/10 text-amber dark:text-amber-light' :
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
                      {claim.verdict === 'supported' ? 'bg-malachite/10 text-malachite dark:text-malachite-light' :
                       claim.verdict === 'disputed' ? 'bg-cinnabar/10 text-cinnabar dark:text-cinnabar-light' :
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
                      <span class="text-xs text-flint dark:text-flint-light">{(claim.confidence * 100).toFixed(0)}%</span>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}

          <p class="mt-3 text-xs text-flint dark:text-flint-light">
            Model: {cc.modelUsed} · {cc.methodology}
          </p>
        </section>
      {/if}

      <!-- ── AI Description (Ollama LLaVA) ─────────────────────────── -->
      {#if result.aiDescription}
        <section
          class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5"
          aria-labelledby="ai-description-heading"
        >
          <h3
            id="ai-description-heading"
            class="font-serif text-base font-semibold text-obsidian dark:text-white mb-3"
          >
            AI Image Description
          </h3>
          <p class="text-sm text-obsidian dark:text-quartz leading-relaxed italic break-words whitespace-pre-wrap">
            "{result.aiDescription}"
          </p>
          <p class="mt-2 text-xs text-flint dark:text-flint-light">
            Generated by LLaVA 7B via Ollama. This is an AI-generated description and is not a verified fact.
          </p>
        </section>
      {/if}

      <!-- ── Read Text (Ollama LLaVA) ──────────────────────────────── -->
      {#if result.contentType === 'image' && result.sourceType !== 'url' && filePath && sidecarHealth?.ollama !== null}
        <section aria-labelledby="read-text-heading" class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-5">
          <div class="flex items-center justify-between flex-wrap gap-3 mb-3">
            <h3 id="read-text-heading" class="font-serif text-base font-semibold text-obsidian dark:text-white">Read Text</h3>
            <button type="button" onclick={handleExtractText} disabled={extractingText} aria-busy={extractingText} class="inline-flex items-center gap-2 text-xs px-3 py-2 rounded border border-lapis/40 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors duration-150 min-h-[44px] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite disabled:opacity-50 disabled:cursor-not-allowed">
              {#if extractingText}
                <svg class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24" aria-hidden="true"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" /><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8z" /></svg>
                Reading text&hellip;
              {:else}
                Read Text (Ollama)
              {/if}
            </button>
          </div>
          <p class="text-xs text-flint dark:text-flint-light leading-relaxed mb-3">Transcribe all visible text in this image using LLaVA. Useful for screenshots, memes, social media posts, and document images. Text can then be fed into the claim checker.</p>
          {#if extractTextError}
            <div role="alert" aria-live="assertive" class="rounded-md border border-cinnabar/30 bg-cinnabar/10 px-4 py-3 text-xs text-cinnabar dark:text-cinnabar-light leading-relaxed">{extractTextError}</div>
          {/if}
          {#if extractedText}
            <div role="status" aria-live="polite" class="rounded-lg border border-border-light dark:border-border-dark bg-gray-50 dark:bg-obsidian/50 px-4 py-3">
              <p class="text-xs font-medium text-text-light dark:text-quartz mb-2">Extracted Text</p>
              <pre class="text-sm text-obsidian dark:text-quartz whitespace-pre-wrap font-mono leading-relaxed">{extractedText}</pre>
              <p class="mt-3 text-xs text-flint dark:text-flint-light">Extracted by LLaVA 7B via Ollama. Review carefully — AI models can misread text, especially in low-resolution, stylised, or heavily compressed images.</p>
            </div>
          {/if}
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

<!-- ── Scroll-to-top button — fixed position, outside main layout flow ─── -->
{#if showScrollTop}
  <button
    onclick={scrollToTop}
    class="fixed bottom-6 right-6 z-30 w-11 h-11 rounded-full shadow-lg flex items-center justify-center
           bg-white dark:bg-graphite border border-border-light dark:border-border-dark
           text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:border-lapis/50
           transition-all duration-150
           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
    aria-label="Scroll to top of page"
    title="Scroll to top"
  >
    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" />
    </svg>
  </button>
{/if}

<!-- ── Image Zoom Modal ───────────────────────────────────────────── -->
{#if showZoomModal && previewUrl}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="fixed inset-0 z-50 bg-obsidian/95 flex items-center justify-center"
    role="dialog"
    aria-modal="true"
    aria-label="Image zoom viewer"
    onclick={() => { showZoomModal = false; }}
    onkeydown={(e) => {
      if (e.key === 'Escape') { showZoomModal = false; }
      if (e.key === '+' || e.key === '=') { e.preventDefault(); zoomLevel = Math.min(zoomLevel + 0.5, 5); }
      if (e.key === '-') { e.preventDefault(); zoomLevel = Math.max(zoomLevel - 0.5, 0.5); }
    }}
    tabindex="-1"
  >
    <!-- Close button -->
    <button
      type="button"
      class="absolute top-4 right-4 z-10 p-2 min-h-[44px] min-w-[44px] rounded-full
             text-quartz hover:text-white bg-graphite/60 hover:bg-graphite
             transition-colors duration-150
             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-obsidian"
      onclick={(e) => { e.stopPropagation(); showZoomModal = false; }}
      aria-label="Close zoom viewer"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>

    <!-- Keyboard shortcut hint -->
    <p class="absolute top-4 left-4 text-xs text-quartz/60 select-none pointer-events-none" aria-hidden="true">
      + / − to zoom &nbsp;·&nbsp; Esc to close
    </p>

    <!-- Scrollable image area — wheel zoom applied here -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="overflow-auto max-h-[90vh] max-w-[90vw] flex items-center justify-center"
      onclick={(e) => { e.stopPropagation(); }}
      onwheel={handleZoomWheel}
    >
      <img
        src={previewUrl}
        alt="Zoomed view of {fileName ?? 'analysed file'}"
        class="max-w-none motion-safe:transition-transform duration-200"
        style="transform: scale({zoomLevel}); transform-origin: center; filter: {getFilterStyle(activeFilter)};"
      />
    </div>

    <!-- Zoom controls -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="absolute bottom-5 left-1/2 -translate-x-1/2 flex items-center gap-2
             bg-graphite/90 rounded-lg px-4 py-2 border border-graphite-light/30"
      onclick={(e) => { e.stopPropagation(); }}
      role="group"
      aria-label="Zoom controls"
    >
      <button
        type="button"
        class="w-8 h-8 flex items-center justify-center text-quartz hover:text-white rounded transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis text-lg font-light"
        onclick={() => { zoomLevel = Math.max(zoomLevel - 0.5, 0.5); }}
        aria-label="Zoom out"
        disabled={zoomLevel <= 0.5}
      >
        −
      </button>
      <span class="text-sm text-quartz tabular-nums w-14 text-center select-none">
        {Math.round(zoomLevel * 100)}%
      </span>
      <button
        type="button"
        class="w-8 h-8 flex items-center justify-center text-quartz hover:text-white rounded transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis text-lg font-light"
        onclick={() => { zoomLevel = Math.min(zoomLevel + 0.5, 5); }}
        aria-label="Zoom in"
        disabled={zoomLevel >= 5}
      >
        +
      </button>
      <span class="text-quartz/40 select-none px-1" aria-hidden="true">|</span>
      <button
        type="button"
        class="text-xs text-flint-light hover:text-quartz transition-colors px-2 py-1 rounded
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis"
        onclick={() => { zoomLevel = 1; }}
        aria-label="Reset zoom to 100%"
        disabled={zoomLevel === 1}
      >
        Reset
      </button>
    </div>
  </div>
{/if}

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
            <svg class="w-5 h-5 text-malachite dark:text-malachite-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
            Reason <span class="text-cinnabar dark:text-cinnabar-light" aria-hidden="true">*</span>
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
            Additional notes <span class="text-flint/50 dark:text-flint-light/60">(optional)</span>
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
          <p class="text-xs text-flint/50 dark:text-flint-light/60 mt-1 text-right">{fpReasonNote.length} / 500</p>
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
              <span class="text-flint/50 dark:text-flint-light/60 font-normal ml-1">(optional)</span>
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
            <span class="text-flint/50 dark:text-flint-light/60 font-normal ml-1">(optional)</span>
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
          <p class="text-xs text-flint/50 dark:text-flint-light/60 mt-1">Name and organisation are remembered for your next export.</p>
        </div>

        <!-- Row 3: Case reference -->
        <div>
          <label for="decl-case-ref" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
            Case Reference
            <span class="text-flint/50 dark:text-flint-light/60 font-normal ml-1">(optional, not saved)</span>
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

        <!-- Row 4: Report format -->
        <div>
          <label for="decl-report-format" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
            Report Format
          </label>
          <select
            id="decl-report-format"
            class="w-full px-3 py-2.5 min-h-[44px] text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                   text-text-light dark:text-quartz
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
            bind:value={reportFormat}
          >
            <option value="standard">Standard Trust Report</option>
            <option value="berkeley">Berkeley Protocol (Legal Evidence)</option>
          </select>
          {#if reportFormat === 'berkeley'}
            <p class="text-xs text-amber dark:text-amber mt-1">
              Berkeley Protocol format adds formal evidence documentation sections suitable for legal proceedings and international investigations.
            </p>
          {/if}
        </div>

        <!-- Row 5: Analyst note (existing) -->
        <div>
          <label for="analyst-note" class="block text-xs font-medium text-flint dark:text-flint-light mb-1">
            Analyst Note
            <span class="text-flint/50 dark:text-flint-light/60 font-normal ml-1">(optional, max 2000 chars — saved automatically)</span>
          </label>
          <textarea
            id="analyst-note"
            class="w-full h-20 px-3 py-2 text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-border-dark rounded
                   text-text-light dark:text-quartz placeholder:text-flint/40 resize-none
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
            placeholder="e.g. Initial assessment suggests authentic capture with minor metadata gaps..."
            maxlength={2000}
            bind:value={analystNote}
          ></textarea>
          <p class="text-xs text-flint/50 dark:text-flint-light/60 mt-1 text-right" aria-live="polite" aria-atomic="true">
            <span class="sr-only">Characters used: </span>{analystNote.length} / 2000
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
