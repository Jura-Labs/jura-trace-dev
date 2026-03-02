<script lang="ts">
  import { verifyFile } from '$lib/api';
  import { getTrustLevel, SEVERITY_CONFIG, formatFileSize } from '$lib/types';
  import type { VerificationResult, AnomalyFinding } from '$lib/types';

  // ── State ──────────────────────────────────────────────────────────
  let filePath = $state<string | null>(null);
  let fileName = $state<string | null>(null);
  let result = $state<VerificationResult | null>(null);
  let checked = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let dragOver = $state(false);

  // ── Derived ────────────────────────────────────────────────────────
  const trustLevel = $derived(result ? getTrustLevel(result.overallTrust) : null);

  const trustScorePercent = $derived(
    result ? Math.round(result.overallTrust * 100) : 0
  );

  const trustTextClass = $derived(() => {
    if (!trustLevel) return 'text-flint';
    if (trustLevel === 'high') return 'text-malachite';
    if (trustLevel === 'medium') return 'text-amber';
    return 'text-cinnabar';
  });

  const trustLabelText = $derived(() => {
    if (trustLevel === 'high') return 'High Trust';
    if (trustLevel === 'medium') return 'Moderate Trust';
    if (trustLevel === 'low') return 'Low Trust';
    return '';
  });

  // ── Drag and drop ──────────────────────────────────────────────────
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
    await runVerification(path, file.name);
  }

  // ── File dialog ────────────────────────────────────────────────────
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
        await runVerification(selected, name);
      }
    } catch {
      // Browser fallback — Tauri not available
    }
  }

  // ── Core verification ──────────────────────────────────────────────
  async function runVerification(path: string, name: string) {
    filePath = path;
    fileName = name;
    result = null;
    checked = false;
    error = null;
    loading = true;

    try {
      result = await verifyFile(path);
      checked = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Verification failed';
    } finally {
      loading = false;
    }
  }

  function reset() {
    filePath = null;
    fileName = null;
    result = null;
    checked = false;
    error = null;
    loading = false;
  }

  // ── Helpers ────────────────────────────────────────────────────────
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

  function highestSeverityFindings(findings: AnomalyFinding[]): AnomalyFinding[] {
    const order: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3, info: 4 };
    return [...findings].sort((a, b) => (order[a.severity] ?? 5) - (order[b.severity] ?? 5));
  }
</script>

<div class="space-y-6">

  <!-- Page heading -->
  <div>
    <h1 class="text-2xl font-heading text-quartz">Verify</h1>
    <p class="text-flint text-sm mt-1">
      Check the authenticity and provenance of files. All analysis happens locally on your device.
    </p>
  </div>

  <!-- Error banner -->
  {#if error}
    <div
      class="bg-cinnabar/10 border border-cinnabar/30 rounded-lg px-4 py-3 text-sm text-cinnabar-light"
      role="alert"
      aria-live="assertive"
    >
      <span class="font-medium">Error:</span> {error}
    </div>
  {/if}

  <!-- Drop zone -->
  <button
    class="w-full border-2 border-dashed rounded-lg p-10 text-center transition-all duration-200 cursor-pointer
           focus:outline-none focus:ring-2 focus:ring-lapis focus:ring-offset-2 focus:ring-offset-obsidian
           {dragOver
             ? 'border-lapis bg-lapis/5 scale-[1.01]'
             : 'border-graphite-light hover:border-lapis/50'}
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

  <!-- Input method placeholders -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
    <div class="border border-graphite-light rounded-lg p-6 text-center opacity-40" aria-hidden="true">
      <p class="font-medium text-quartz text-sm mb-1">Check URL</p>
      <p class="text-xs text-flint">Coming in Phase 2</p>
    </div>
    <div class="border border-graphite-light rounded-lg p-6 text-center opacity-40" aria-hidden="true">
      <p class="font-medium text-quartz text-sm mb-1">Check Claim</p>
      <p class="text-xs text-flint">Coming in Phase 2</p>
    </div>
  </div>

  <!-- ── Results ─────────────────────────────────────────────────── -->

  {#if checked && result}

    <!-- Trust Score header -->
    <div class="bg-graphite rounded-lg border border-graphite-light overflow-hidden">
      <div class="px-5 py-4 border-b border-graphite-light flex items-center justify-between gap-4">
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
            <p class="text-sm text-quartz truncate" title={fileName ?? undefined}>{fileName}</p>
            <p class="text-xs text-flint mt-0.5">{result.contentType}</p>
          </div>
        </div>
        <button
          class="flex-shrink-0 text-xs text-flint hover:text-quartz transition-colors duration-150
                 focus:outline-none focus:ring-2 focus:ring-lapis rounded px-2 py-1"
          onclick={reset}
          aria-label="Clear result and verify another file"
        >
          Clear
        </button>
      </div>

      <!-- Metadata flags (if any) -->
      {#if result.metadataFlags.length > 0}
        <div class="px-5 py-3 border-b border-graphite-light flex flex-wrap gap-2" aria-label="Metadata flags">
          {#each result.metadataFlags as flag}
            <span class="text-xs px-2 py-0.5 rounded bg-amber/10 text-amber border border-amber/20">
              {flag}
            </span>
          {/each}
        </div>
      {/if}

      <!-- ── EXIF Analysis ─────────────────────────────────────────── -->
      {#if result.exifAnalysis}
        {@const exif = result.exifAnalysis}
        <section class="px-5 py-4 border-b border-graphite-light" aria-labelledby="exif-heading">
          <div class="flex items-center justify-between mb-3">
            <h2 id="exif-heading" class="text-sm font-medium text-quartz">EXIF Analysis</h2>
            <span class="text-xs text-flint">
              {exif.fieldsPopulated}/{exif.fieldsTotal} fields populated
            </span>
          </div>

          <!-- Completeness bar -->
          <div class="mb-4">
            <div
              class="h-1.5 rounded-full bg-graphite-light overflow-hidden"
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
                  <!-- Severity badge -->
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
                  <!-- Content -->
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
          </div>

          <!-- Manifest details grid -->
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

          <!-- Assertions -->
          {#if manifest.assertions.length > 0}
            <div>
              <h3 class="text-xs text-flint uppercase tracking-wide mb-2">
                Assertions
                <span class="normal-case ml-1 text-flint/70">({manifest.assertions.length})</span>
              </h3>
              <div class="space-y-2" role="list" aria-label="C2PA assertions">
                {#each manifest.assertions as assertion (assertion.label)}
                  <div class="bg-obsidian/50 rounded-md p-3" role="listitem">
                    <p class="text-xs font-mono text-lapis mb-1 break-all">{assertion.label}</p>
                    <pre class="text-xs text-flint whitespace-pre-wrap break-words leading-relaxed">{assertion.value}</pre>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>

      {:else if checked}
        <!-- No C2PA manifest found -->
        <section class="px-5 py-4" aria-labelledby="c2pa-heading">
          <div class="flex items-center gap-3 mb-3">
            <h2 id="c2pa-heading" class="text-sm font-medium text-quartz">C2PA Credentials</h2>
            <span class="text-xs font-medium px-2 py-0.5 rounded bg-graphite-light text-flint border border-graphite-light">
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

  {:else if !checked && !loading}

    <!-- Pre-verification idle state -->
    <div class="bg-graphite rounded-lg border border-graphite-light p-8 text-center">
      <p class="text-flint text-sm">
        Drop a file above to analyse its EXIF metadata and C2PA Content Credentials.
      </p>
    </div>

  {/if}

</div>
