<script lang="ts">
  import { onMount } from 'svelte';
  import { verifyFile, verifyUrl, checkSidecarHealth, openBatchFileDialog, markFalsePositive } from '$lib/api';
  import { getTrustLevel, SEVERITY_CONFIG, formatFileSize, formatDuration } from '$lib/types';
  import type { VerificationResult, AnomalyFinding, SidecarHealth, VerifyMode, BatchItem } from '$lib/types';
  import VerdictSummary from '$lib/components/VerdictSummary.svelte';
  import MethodologyPanel from '$lib/components/MethodologyPanel.svelte';
  import InspectionChecklist from '$lib/components/InspectionChecklist.svelte';
  import SignalAgreement from '$lib/components/SignalAgreement.svelte';
  import { generateTrustReport } from '$lib/pdf';
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
  let dragOver = $state(false);
  let sidecarHealth = $state<SidecarHealth | null>(null);
  let verifyMode = $state<VerifyMode>('standard');
  let showTechnicalDetails = $state(false);
  let showInvestigatePanel = $state(false);
  let showSignalAgreement = $state(false);
  let showInspectionChecklist = $state(false);

  // ── Export state ─────────────────────────────────────────────────
  let showReportModal = $state(false);
  let analystNote = $state('');
  let exportingReport = $state(false);
  let exportingCase = $state(false);
  let appVersion = $state('0.2.0-dev');

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
    if (!level) return 'text-flint';
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

  // ── Lifecycle ─────────────────────────────────────────────────────
  onMount(() => {
    // Async init — fire-and-forget; cleanup is returned synchronously below
    (async () => {
      sidecarHealth = await checkSidecarHealth();
      appVersion = await getVersion();
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
          loading = false;
          error = 'Cancelled';
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
  async function runFileVerification(path: string, name: string) {
    filePath = path;
    fileName = name;
    result = null;
    checked = false;
    error = null;
    loading = true;

    try {
      result = await verifyFile(path, verifyMode);
      checked = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Verification failed';
    } finally {
      loading = false;
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
    loading = true;

    try {
      result = await verifyUrl(url, verifyMode);
      checked = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'URL verification failed';
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
    showInvestigatePanel = false;
    showSignalAgreement = false;
    showInspectionChecklist = false;
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
      const blob = generateTrustReport(result, {
        fileName: fileName ?? 'Unknown',
        fileSize: 0,
        analysedAt: new Date().toISOString(),
        analystNote: analystNote.trim() || undefined,
        appVersion,
      });
      const ts = Math.floor(Date.now() / 1000);
      const safeName = (fileName ?? 'file').replace(/[^a-zA-Z0-9._-]/g, '_');
      triggerDownload(blob, `jura-report-${safeName}-${ts}.pdf`);
    } finally {
      exportingReport = false;
      showReportModal = false;
      analystNote = '';
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
          ? 'Search for this image via Google Lens'
          : 'Open Google Lens — upload the file manually',
      },
      {
        label: 'TinEye',
        href: sourceUrl
          ? `https://tineye.com/search?url=${encodeURIComponent(sourceUrl)}`
          : 'https://tineye.com',
        title: sourceUrl
          ? 'Search for this image via TinEye reverse image search'
          : 'Open TinEye — upload the file manually',
      },
      {
        label: 'Yandex Images',
        href: sourceUrl
          ? `https://yandex.com/images/search?rpt=imageview&url=${encodeURIComponent(sourceUrl)}`
          : 'https://yandex.com/images',
        title: sourceUrl
          ? 'Search for this image via Yandex Images'
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
</script>

<div class="space-y-6">

  <!-- Page heading + sidecar status -->
  <div class="flex items-start justify-between">
    <div>
      <h1 class="text-2xl font-heading text-text-light dark:text-quartz">Verify</h1>
      <p class="text-flint text-sm mt-1">
        Check the authenticity and provenance of files. All analysis happens locally on your device.
      </p>
    </div>
    <div class="flex items-center gap-3">
      <!-- Mode toggle -->
      <div
        class="flex items-center text-xs rounded-full border border-border-light dark:border-graphite-light overflow-hidden"
        role="radiogroup"
        aria-label="Verification mode"
      >
        {#each [
          { mode: 'standard' as VerifyMode, label: 'Standard', title: 'Standard: EXIF + C2PA + ELA + AI detection (under 15 seconds)' },
          { mode: 'deep' as VerifyMode, label: 'Deep', title: 'Deep: full forensic pipeline with all detectors (30-60 seconds)' },
          { mode: 'archival' as VerifyMode, label: 'Archival', title: 'Archival: deep analysis with scanner-calibrated tolerances' },
        ] as opt}
          <button
            class="px-3 py-2.5 min-h-[44px] transition-colors duration-150
                   {verifyMode === opt.mode
                     ? 'bg-lapis/20 text-lapis dark:text-lapis-light'
                     : 'text-flint hover:text-text-light dark:hover:text-quartz'}
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis"
            role="radio"
            aria-checked={verifyMode === opt.mode}
            onclick={() => { verifyMode = opt.mode; }}
            title={opt.title}
          >
            {opt.label}
          </button>
        {/each}
      </div>

      <!-- Sidecar status -->
      <div
        class="flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-full border
               {sidecarAvailable
                 ? 'bg-malachite/10 text-malachite dark:text-malachite-light border-malachite/20'
                 : 'bg-white dark:bg-graphite text-flint border-border-light dark:border-graphite-light'}"
        title={sidecarAvailable
          ? `ML Sidecar v${sidecarHealth?.version} — forensics available`
          : 'ML Sidecar offline — forensics not available'}
      >
        <span
          class="w-1.5 h-1.5 rounded-full {sidecarAvailable ? 'bg-malachite' : 'bg-flint/50'}"
          aria-hidden="true"
        ></span>
        {sidecarAvailable ? 'ML Connected' : 'ML Offline'}
      </div>
    </div>
  </div>

  <!-- Error banner -->
  {#if error}
    <div
      class="bg-cinnabar/10 border border-cinnabar/30 rounded-lg px-4 py-3 text-sm text-cinnabar dark:text-cinnabar-light"
      role="alert"
      aria-live="assertive"
    >
      <span class="font-medium">Error:</span> {error}
    </div>
  {/if}

  <!-- ── Input Tabs ─────────────────────────────────────────────────── -->
  <div>
    <!-- Tab bar -->
    <div class="flex border-b border-border-light dark:border-graphite-light mb-4" role="tablist">
      <button
        class="px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
               {activeTab === 'file'
                 ? 'text-lapis dark:text-lapis-light border-lapis'
                 : 'text-flint border-transparent hover:text-text-light dark:hover:text-quartz'}"
        role="tab"
        aria-selected={activeTab === 'file'}
        onclick={() => { activeTab = 'file'; }}
      >
        File
      </button>
      <button
        class="px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
               {activeTab === 'batch'
                 ? 'text-lapis dark:text-lapis-light border-lapis'
                 : 'text-flint border-transparent hover:text-text-light dark:hover:text-quartz'}"
        role="tab"
        aria-selected={activeTab === 'batch'}
        onclick={() => { activeTab = 'batch'; }}
      >
        Batch
        {#if batchItems.length > 0}
          <span class="ml-1 text-xs text-flint">({batchItems.length})</span>
        {/if}
      </button>
      <button
        class="px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
               {activeTab === 'url'
                 ? 'text-lapis dark:text-lapis-light border-lapis'
                 : 'text-flint border-transparent hover:text-text-light dark:hover:text-quartz'}"
        role="tab"
        aria-selected={activeTab === 'url'}
        onclick={() => { activeTab = 'url'; }}
      >
        URL
      </button>
      <div class="flex-1"></div>
      <div class="px-4 py-2.5 text-xs text-flint/50 border-b-2 border-transparent">
        Claim checking — Phase 2
      </div>
    </div>

    <!-- File tab -->
    {#if activeTab === 'file'}
      <button
        class="w-full border-2 border-dashed rounded-lg p-10 text-center transition-all duration-200 cursor-pointer
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
               {dragOver
                 ? 'border-lapis bg-lapis/5 scale-[1.01]'
                 : 'border-border-light dark:border-graphite-light hover:border-lapis/50'}
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
              aria-label="Analysing file"
            ></div>
            <p class="text-sm text-flint">Analysing file — this may take a moment...</p>
            {#if fileName}
              <p class="text-xs text-flint/70">{fileName}</p>
            {/if}
          </div>
        {:else}
          <div class="flex flex-col items-center gap-2">
            <svg class="w-10 h-10 text-flint" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
            </svg>
            <p class="text-quartz font-medium">Drop a file to verify</p>
            <p class="text-xs text-flint">
              or click to browse — JPEG, PNG, TIFF, WebP, PDF, MP4, and more
            </p>
          </div>
        {/if}
      </button>
    {/if}

    <!-- Batch tab -->
    {#if activeTab === 'batch'}
      <!-- Drop zone -->
      <button
        class="w-full border-2 border-dashed rounded-lg p-8 text-center transition-all duration-200 cursor-pointer
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
               {batchDragOver
                 ? 'border-lapis bg-lapis/5 scale-[1.01]'
                 : 'border-border-light dark:border-graphite-light hover:border-lapis/50'}
               {batchRunning ? 'opacity-60 pointer-events-none' : ''}"
        ondragover={(e) => { e.preventDefault(); batchDragOver = true; }}
        ondragleave={() => { batchDragOver = false; }}
        ondrop={handleBatchDrop}
        onclick={handleBatchBrowse}
        aria-label="Drop files here or click to select files for batch verification"
      >
        <div class="flex flex-col items-center gap-2">
          <svg class="w-8 h-8 text-flint" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
              d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
          </svg>
          <p class="text-quartz font-medium">Drop multiple files to verify</p>
          <p class="text-xs text-flint">or click to browse — files will be queued for sequential verification</p>
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
            <span class="text-xs text-flint">
              {batchCompleted} of {batchItems.length} complete
            </span>
          </div>
          <button
            class="text-xs text-flint hover:text-text-light dark:hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-2 py-1"
            onclick={clearBatch}
            disabled={batchRunning}
          >
            Clear all
          </button>
        </div>

        <!-- Results table -->
        <div class="mt-4 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-graphite-light overflow-x-auto">
          <!-- Header -->
          <div class="grid grid-cols-[1fr_90px_80px_80px_60px] gap-3 px-4 py-2 border-b border-border-light dark:border-graphite-light text-xs text-flint uppercase tracking-wide min-w-[480px]">
            <span>File</span>
            <span>Status</span>
            <span>Trust</span>
            <span>Duration</span>
            <span></span>
          </div>

          <!-- Rows -->
          {#each batchItems as item (item.id)}
            <div class="border-b border-border-light/50 dark:border-graphite-light/50 min-w-[480px]">
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
                    <span class="text-flint">Queued</span>
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
                <span class="text-xs text-flint tabular-nums self-center">
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
                      class="text-xs text-flint hover:text-cinnabar transition-colors p-1 min-w-[24px] min-h-[24px]
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
                <div class="px-4 py-4 bg-gray-50 dark:bg-obsidian/50 border-t border-border-light dark:border-graphite-light">
                  <VerdictSummary result={item.result} fileName={item.fileName} />
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/if}

    <!-- URL tab -->
    {#if activeTab === 'url'}
      <div class="flex gap-3">
        <input
          type="url"
          bind:value={urlInput}
          placeholder="https://example.com/image.jpg"
          disabled={loading}
          class="flex-1 bg-white dark:bg-obsidian border border-border-light dark:border-graphite-light rounded-lg px-4 py-3 text-sm text-text-light dark:text-quartz
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
      <p class="text-xs text-flint mt-2">
        Enter a URL to an image or document. The content will be downloaded and analysed locally.
      </p>
    {/if}
  </div>

  <!-- ── Results ─────────────────────────────────────────────────── -->

  {#if checked && result}

    <!-- Trust Score header -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-graphite-light overflow-hidden">
      <div class="px-5 py-4 border-b border-border-light dark:border-graphite-light flex items-center justify-between gap-4">
        <div class="flex items-center gap-4 min-w-0">
          <div>
            <p class="text-xs text-flint uppercase tracking-wide mb-0.5">Trust Score</p>
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
            <p class="text-sm text-text-light dark:text-quartz truncate" title={fileName ?? undefined}>{fileName}</p>
            <p class="text-xs text-flint mt-0.5">
              {result.contentType}
              {#if result.sourceType === 'url'}
                <span class="ml-1 text-lapis">(via URL)</span>
              {/if}
            </p>
          </div>
        </div>
        <button
          class="flex-shrink-0 text-xs text-flint hover:text-text-light dark:hover:text-quartz transition-colors duration-150
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-2 py-1 min-h-[44px] min-w-[44px] flex items-center"
          onclick={reset}
          aria-label="Clear result and verify another file"
        >
          Clear
        </button>
      </div>

      <!-- Metadata flags (if any) -->
      {#if result.metadataFlags.length > 0}
        <div class="px-5 py-3 border-b border-border-light dark:border-graphite-light flex flex-wrap gap-2" aria-label="Metadata flags">
          {#each result.metadataFlags as flag}
            <span class="text-xs px-2 py-0.5 rounded bg-amber/10 text-amber border border-amber/20">
              {flag}
            </span>
          {/each}
        </div>
      {/if}

      <!-- ── Verdict Summary ──────────────────────────────────────── -->
      <div class="px-5 py-4 border-b border-border-light dark:border-graphite-light">
        <VerdictSummary {result} fileName={fileName ?? 'Unknown file'} />
      </div>

      <!-- ── Signal Agreement ─────────────────────────────────────── -->
      <div class="px-5 py-3 border-b border-graphite-light">
        <button
          class="flex items-center gap-2 text-sm text-flint hover:text-quartz transition-colors duration-150
                 focus:outline-none focus:ring-2 focus:ring-lapis rounded"
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
      <div class="px-5 py-3 border-b border-graphite-light">
        <button
          class="flex items-center gap-2 text-sm text-flint hover:text-quartz transition-colors duration-150
                 focus:outline-none focus:ring-2 focus:ring-lapis rounded"
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
      <div class="px-5 py-3 border-b border-graphite-light">
        <button
          class="flex items-center gap-2 text-sm text-flint hover:text-quartz transition-colors duration-150
                 focus:outline-none focus:ring-2 focus:ring-lapis rounded"
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
        </button>
        {#if showInvestigatePanel}
          <div id="investigate-further-panel" class="mt-3">
            <div
              class="rounded-lg border border-graphite bg-obsidian/50 px-4 py-3"
              aria-label="Reverse image search options"
            >
              <p class="text-xs text-flint mb-3 leading-relaxed">
                Search for this image across the web to find other appearances, earlier versions, or
                context that may help verify its origin.
                {#if result.sourceType !== 'url'}
                  The file path cannot be sent directly — open the search engine's upload page and
                  drag the file in manually.
                {/if}
              </p>
              <div class="flex flex-wrap gap-2" role="group" aria-label="Search engine links">
                {#each reverseSearchLinks() as link}
                  <a
                    href={link.href}
                    target="_blank"
                    rel="noopener noreferrer"
                    title={link.title}
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md
                           border border-lapis/40 text-lapis hover:bg-lapis/10 hover:border-lapis/70
                           transition-colors duration-150
                           focus:outline-none focus:ring-2 focus:ring-lapis focus:ring-offset-2
                           focus:ring-offset-obsidian"
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
      <div class="px-5 py-3 border-b border-border-light dark:border-graphite-light flex flex-wrap items-center gap-3">
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

        <!-- False positive report — secondary action, pushed to far right -->
        <div class="flex-1 flex justify-end">
          <button
            class="inline-flex items-center gap-1.5 px-3 py-2 min-h-[44px] text-xs text-flint border border-border-light dark:border-graphite-light rounded
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
      <div class="px-5 py-3 border-b border-border-light dark:border-graphite-light">
        <button
          class="flex items-center gap-2 text-sm text-flint hover:text-text-light dark:hover:text-quartz transition-colors duration-150
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
        <section class="px-5 py-4 border-b border-border-light dark:border-graphite-light" aria-labelledby="ela-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="ela-heading" class="text-sm font-medium text-quartz">Error Level Analysis</h2>
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
          <div class="mb-3 rounded-md overflow-hidden border border-border-light dark:border-graphite-light bg-gray-100 dark:bg-obsidian">
            <img
              src="data:image/png;base64,{ela.elaImageBase64}"
              alt="Error Level Analysis heatmap showing compression artefact differences"
              class="w-full max-h-64 object-contain"
            />
          </div>

          <!-- Stats -->
          <div class="grid grid-cols-2 gap-4 text-xs">
            <div>
              <span class="text-flint">Max Difference</span>
              <p class="text-quartz tabular-nums">{ela.maxDifference.toFixed(1)}</p>
            </div>
            <div>
              <span class="text-flint">Mean Difference</span>
              <p class="text-quartz tabular-nums">{ela.meanDifference.toFixed(1)}</p>
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
        <section class="px-5 py-3 border-b border-graphite-light" aria-labelledby="ela-heading">
          <div class="flex items-center gap-3">
            <h2 id="ela-heading" class="text-sm font-medium text-quartz">Error Level Analysis</h2>
            <span class="text-xs text-flint bg-gray-100 dark:bg-graphite-light px-2 py-0.5 rounded border border-border-light dark:border-graphite-light">
              Unavailable
            </span>
          </div>
          <p class="text-xs text-flint mt-1.5">
            ML Sidecar is offline. Start the sidecar to enable forensic analysis.
          </p>
        </section>
      {/if}

      <!-- ── Noise Analysis ────────────────────────────────────────── -->
      {#if result.noiseResult}
        {@const noise = result.noiseResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-graphite-light" aria-labelledby="noise-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="noise-heading" class="text-sm font-medium text-quartz">Noise Analysis</h2>
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
            <div class="mb-3 rounded-md overflow-hidden border border-graphite-light bg-obsidian">
              <img
                src="data:image/png;base64,{noise.heatmapBase64}"
                alt="Noise variance heatmap — blue is low variance, red is high variance"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Stats -->
          <div class="grid grid-cols-3 gap-4 text-xs">
            <div>
              <span class="text-flint">Global Variance</span>
              <p class="text-quartz tabular-nums">{noise.globalVariance.toFixed(1)}</p>
            </div>
            <div>
              <span class="text-flint">Anomalous Blocks</span>
              <p class="text-quartz tabular-nums">{noise.anomalousBlocks} / {noise.totalBlocks}</p>
            </div>
            <div>
              <span class="text-flint">Block Count</span>
              <p class="text-quartz tabular-nums">{noise.totalBlocks}</p>
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
        <section class="px-5 py-4 border-b border-border-light dark:border-graphite-light" aria-labelledby="copymove-heading">
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="copymove-heading" class="text-sm font-medium text-quartz">Copy-Move Detection</h2>
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
            <div class="mb-3 rounded-md overflow-hidden border border-graphite-light bg-obsidian">
              <img
                src="data:image/png;base64,{cm.visualisationBase64}"
                alt="Copy-move detection visualisation showing matched feature pairs and clone region bounding boxes"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Stats -->
          <div class="grid grid-cols-2 gap-4 text-xs">
            <div>
              <span class="text-flint">Matched Pairs</span>
              <p class="text-quartz tabular-nums">{cm.matchedPairs}</p>
            </div>
            <div>
              <span class="text-flint">Clone Regions</span>
              <p class="text-quartz tabular-nums">{cm.cloneRegions.length}</p>
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

      <!-- ── AI Generation Detection ─────────────────────────────── -->
      {#if result.deepfakeResult}
        {@const df = result.deepfakeResult}
        <section class="px-5 py-4 border-b border-border-light dark:border-graphite-light" aria-labelledby="deepfake-heading">
          <!-- AI Watermark Detections -->
          {#if df.watermarks?.some(w => w.detected)}
            <div class="mb-3 rounded-md border border-cinnabar/30 bg-cinnabar/10 px-4 py-3">
              <p class="text-xs font-medium text-cinnabar-light mb-1.5">
                AI Generator Watermark Detected
              </p>
              {#each df.watermarks.filter(w => w.detected) as wm (wm.watermarkType)}
                <div class="flex items-center justify-between text-xs mb-1 last:mb-0">
                  <span class="text-quartz font-mono">
                    {wm.watermarkType.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
                  </span>
                  <span class="text-flint tabular-nums">
                    {(wm.confidence * 100).toFixed(0)}% confidence
                  </span>
                </div>
                <p class="text-xs text-flint mb-1">{wm.details}</p>
              {/each}
            </div>
          {/if}

          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-3">
              <h2 id="deepfake-heading" class="text-sm font-medium text-quartz">AI Generation Detection</h2>
              <span
                class="text-xs font-medium px-2 py-0.5 rounded border {forensicScoreBgClass(df.score)} {forensicScoreClass(df.score)}"
              >
                {df.suspicious ? 'Suspicious' : 'Normal'}
              </span>
              <span
                class="text-xs px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-flint border border-border-light dark:border-graphite-light"
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
            <div class="mb-3 rounded-md overflow-hidden border border-graphite-light bg-obsidian">
              <img
                src="data:image/png;base64,{df.heatmapBase64}"
                alt="Frequency spectrum heatmap for AI generation detection"
                class="w-full max-h-64 object-contain"
              />
            </div>
          {/if}

          <!-- Summary -->
          <p class="text-xs text-flint mb-3">{df.summary}</p>

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
                           {signal.triggered ? 'bg-amber/10 border border-amber/20' : 'bg-graphite-light/50 border border-graphite-light'}"
                    role="listitem"
                  >
                    <span
                      class="flex-shrink-0 w-1.5 h-1.5 mt-1 rounded-full {signal.triggered ? 'bg-amber' : 'bg-flint/30'}"
                      aria-hidden="true"
                    ></span>
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center justify-between gap-2">
                        <span class="font-mono {signal.triggered ? 'text-amber' : 'text-flint'}">{signal.name}</span>
                        <span class="text-flint/60 tabular-nums">weight: {signal.weight.toFixed(1)}</span>
                      </div>
                      <p class="text-flint mt-0.5">{signal.description}</p>
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
        <section class="px-5 py-4 border-b border-border-light dark:border-graphite-light" aria-labelledby="exif-heading">
          <div class="flex items-center justify-between mb-3">
            <h2 id="exif-heading" class="text-sm font-medium text-quartz">EXIF Analysis</h2>
            <span class="text-xs text-flint">
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
            <p class="text-xs text-flint mt-1">
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
                                 : 'border-graphite-light'}"
                    aria-label="Severity: {config.label}"
                  >
                    {config.label}
                  </span>
                  <div class="min-w-0">
                    <p class="text-sm text-quartz leading-snug">{finding.title}</p>
                    <p class="text-xs text-flint mt-0.5 leading-relaxed">{finding.description}</p>
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-xs text-flint">No anomalies detected in EXIF metadata.</p>
          {/if}
        </section>
      {/if}

      <!-- ── C2PA Credentials ──────────────────────────────────────── -->
      {#if result.c2paManifest}
        {@const manifest = result.c2paManifest}
        <section class="px-5 py-4" aria-labelledby="c2pa-heading">
          <div class="flex items-center gap-3 mb-4">
            <h2 id="c2pa-heading" class="text-sm font-medium text-quartz">C2PA Credentials</h2>
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
                title="C2PA credentials indicate this was created by an AI image generator"
              >
                AI: {result.aiGenerator}
              </span>
            {/if}
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-3 text-sm mb-4">
            {#if manifest.claimGenerator}
              <div>
                <span class="text-xs text-flint uppercase tracking-wide">Claim Generator</span>
                <p class="text-quartz mt-0.5 break-words">{manifest.claimGenerator}</p>
              </div>
            {/if}
            {#if manifest.format}
              <div>
                <span class="text-xs text-flint uppercase tracking-wide">Format</span>
                <p class="text-quartz mt-0.5">{manifest.format}</p>
              </div>
            {/if}
            {#if manifest.title}
              <div>
                <span class="text-xs text-flint uppercase tracking-wide">Title</span>
                <p class="text-quartz mt-0.5 break-words">{manifest.title}</p>
              </div>
            {/if}
            {#if manifest.signedAt}
              <div>
                <span class="text-xs text-flint uppercase tracking-wide">Signed At</span>
                <p class="text-quartz mt-0.5">{formatSignedAt(manifest.signedAt)}</p>
              </div>
            {/if}
          </div>

          {#if manifest.assertions.length > 0}
            <div>
              <h3 class="text-xs text-flint uppercase tracking-wide mb-2">
                Assertions
                <span class="normal-case ml-1 text-flint/70">({manifest.assertions.length})</span>
              </h3>
              <div class="space-y-2" role="list" aria-label="C2PA assertions">
                {#each manifest.assertions as assertion (assertion.label)}
                  <div class="bg-gray-100 dark:bg-obsidian/50 rounded-md p-3" role="listitem">
                    <p class="text-xs font-mono text-lapis dark:text-lapis mb-1 break-all">{assertion.label}</p>
                    <pre class="text-xs text-flint whitespace-pre-wrap break-words leading-relaxed">{assertion.value}</pre>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>

      {:else if checked}
        <section class="px-5 py-4" aria-labelledby="c2pa-heading">
          <div class="flex items-center gap-3 mb-3">
            <h2 id="c2pa-heading" class="text-sm font-medium text-quartz">C2PA Credentials</h2>
            <span class="text-xs font-medium px-2 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-flint border border-border-light dark:border-graphite-light">
              Not Found
            </span>
          </div>
          <p class="text-sm text-flint">
            No C2PA Content Credentials found in
            <span class="text-quartz">{fileName}</span>.
            This file has not been signed with C2PA provenance data.
          </p>
        </section>
      {/if}

      </div>
      {/if}
      <!-- End Technical Details -->

    </div>

    <!-- ── Methodology Panel ────────────────────────────────────────── -->
    <MethodologyPanel {result} {sidecarHealth} />

  {:else if !checked && !loading}

    <!-- Pre-verification idle state -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-graphite-light p-8 text-center">
      <p class="text-flint text-sm">
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
    <div class="bg-white dark:bg-graphite border border-border-light dark:border-graphite-light rounded-lg shadow-xl w-full max-w-md mx-4 p-6">

      {#if fpSubmitted}
        <!-- Success state -->
        <div class="flex flex-col items-center gap-3 py-4 text-center">
          <div class="w-10 h-10 rounded-full bg-malachite/15 border border-malachite/30 flex items-center justify-center" aria-hidden="true">
            <svg class="w-5 h-5 text-malachite" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <p class="text-sm font-medium text-text-light dark:text-quartz">Report submitted</p>
          <p class="text-xs text-flint">Thank you. This helps improve detection accuracy.</p>
        </div>

      {:else}
        <!-- Form -->
        <h2 id="fp-modal-title" class="text-lg font-medium text-text-light dark:text-quartz mb-1">
          Report False Positive
        </h2>
        <p class="text-sm text-flint mb-5">
          If this result appears to be a false positive, let us know why. Reports help calibrate the detection system.
        </p>

        <!-- Reason code -->
        <fieldset class="mb-4">
          <legend class="block text-xs font-medium text-flint mb-2">
            Reason <span class="text-cinnabar" aria-hidden="true">*</span>
            <span class="sr-only">(required)</span>
          </legend>
          <div class="space-y-2" role="radiogroup" aria-label="False positive reason">
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
                         : 'border-border-light dark:border-graphite-light hover:border-lapis/30 hover:bg-gray-50 dark:hover:bg-graphite-light/20'}"
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
                  <span class="text-xs text-flint leading-relaxed">{opt.description}</span>
                </div>
              </label>
            {/each}
          </div>
        </fieldset>

        <!-- Optional note -->
        <div class="mb-5">
          <label for="fp-note" class="block text-xs font-medium text-flint mb-1">
            Additional notes <span class="text-flint/50">(optional)</span>
          </label>
          <textarea
            id="fp-note"
            class="w-full h-20 px-3 py-2 text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-graphite-light rounded
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
            class="px-4 py-2.5 min-h-[44px] text-sm text-flint hover:text-text-light dark:hover:text-quartz transition-colors
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

<!-- ── Analyst Note Modal ─────────────────────────────────────────── -->
{#if showReportModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="report-modal-title"
    tabindex="-1"
    onkeydown={(e) => { if (e.key === 'Escape') showReportModal = false; }}
    onclick={(e) => { if (e.target === e.currentTarget) showReportModal = false; }}
  >
    <div class="bg-white dark:bg-graphite border border-border-light dark:border-graphite-light rounded-lg shadow-xl w-full max-w-md mx-4 p-6">
      <h2 id="report-modal-title" class="text-lg font-medium text-text-light dark:text-quartz mb-1">Export Trust Report</h2>
      <p class="text-sm text-flint mb-4">
        Add an optional analyst note to include in the PDF report.
      </p>

      <label for="analyst-note" class="block text-xs font-medium text-flint mb-1">
        Analyst Note <span class="text-flint/50">(optional, max 500 chars)</span>
      </label>
      <textarea
        id="analyst-note"
        class="w-full h-24 px-3 py-2 text-sm bg-gray-50 dark:bg-obsidian border border-border-light dark:border-graphite-light rounded
               text-text-light dark:text-quartz placeholder:text-flint/40 resize-none
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:border-transparent"
        placeholder="e.g. Initial assessment suggests authentic capture with minor metadata gaps..."
        maxlength={500}
        bind:value={analystNote}
      ></textarea>
      <p class="text-xs text-flint/50 mt-1 mb-4 text-right">
        {analystNote.length} / 500
      </p>

      <div class="flex gap-3 justify-end">
        <button
          class="px-4 py-2.5 min-h-[44px] text-sm text-flint hover:text-text-light dark:hover:text-quartz transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
          onclick={() => { analystNote = ''; handleExportReport(); }}
          disabled={exportingReport}
        >
          Export without note
        </button>
        <button
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
