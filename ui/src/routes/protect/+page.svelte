<script lang="ts">
  import { getFilteredAssets, deleteAsset, importFiles, openFileDialog, signAsset, getFingerprints, findSimilar, checkMetadataBeforeSign, embedWatermark } from '$lib/api';
  import {
    type Asset,
    type ContentType,
    type ImageMetadata,
    type Fingerprint,
    type SimilarAsset,
    type MetadataSigningWarning,
    type WatermarkEmbedResult,
    parseMetadata,
    formatFileSize,
    CONTENT_TYPE_LABELS,
    HASH_TYPE_LABELS,
  } from '$lib/types';

  // ── Filter state ───────────────────────────────────────────────────
  let filterContentType = $state<string>('');   // '' = All Types
  let filterStatus     = $state<string>('');   // '' | 'signed' | 'unsigned'
  let searchRaw        = $state('');            // bound to input, debounced below
  let searchDebounced  = $state('');            // used in effect

  // ── Asset list ────────────────────────────────────────────────────
  let assets: Asset[] = $state([]);

  // ── Sort state ────────────────────────────────────────────────────
  type SortKey = 'fileName' | 'fileSize' | 'createdAt';
  let sortKey = $state<SortKey>('createdAt');
  let sortDir = $state<'asc' | 'desc'>('desc');

  // ── Import state ─────────────────────────────────────────────────
  let importingCount = $state(0);   // 0 = not importing; >0 = N files in flight
  let dragOver       = $state(false);
  let error: string | null = $state(null);

  // ── C2PA signing state ────────────────────────────────────────────
  let signingAssetId: string | null = $state(null);
  let creatorName   = $state('');
  let selectedLicense = $state('All Rights Reserved');
  let signing = $state(false);
  let metadataWarning = $state<MetadataSigningWarning | null>(null);
  let metadataWarningLoading = $state(false);

  // ── Fingerprint state ─────────────────────────────────────────────
  let showFingerprintsFor: string | null = $state(null);
  let fingerprints: Fingerprint[] = $state([]);
  let similarAssets: SimilarAsset[] = $state([]);
  let loadingFingerprints = $state(false);

  // ── Watermark state ───────────────────────────────────────────────
  let watermarkAssetId: string | null = $state(null);
  let watermarkPayload = $state('');
  let watermarkStrength = $state<number>(2); // 1 = low, 2 = medium, 3 = high
  let watermarking = $state(false);
  let watermarkResult = $state<WatermarkEmbedResult | null>(null);

  // ── Selected asset ────────────────────────────────────────────────
  let selectedAsset: Asset | null = $state(null);

  // ── Debounce search input (300 ms) ────────────────────────────────
  $effect(() => {
    const raw = searchRaw;
    const timer = setTimeout(() => {
      searchDebounced = raw;
    }, 300);
    return () => clearTimeout(timer);
  });

  // ── Re-fetch when filters change ─────────────────────────────────
  $effect(() => {
    const contentType = filterContentType || undefined;
    const c2paSigned  = filterStatus === 'signed'
      ? true
      : filterStatus === 'unsigned'
        ? false
        : undefined;
    const query = searchDebounced.trim() || undefined;

    getFilteredAssets(contentType, c2paSigned, query).then((result) => {
      assets = result;
    });
  });

  // ── Sorted derived list ──────────────────────────────────────────
  const displayedAssets = $derived(
    [...assets].sort((a, b) => {
      let cmp = 0;
      if (sortKey === 'fileName') {
        cmp = a.fileName.localeCompare(b.fileName);
      } else if (sortKey === 'fileSize') {
        cmp = a.fileSize - b.fileSize;
      } else {
        // createdAt — ISO strings sort lexicographically
        cmp = a.createdAt.localeCompare(b.createdAt);
      }
      return sortDir === 'asc' ? cmp : -cmp;
    })
  );

  // ── Sort handler ─────────────────────────────────────────────────
  function handleSort(key: SortKey) {
    if (sortKey === key) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc';
    } else {
      sortKey = key;
      sortDir = 'asc';
    }
  }

  // ── Import handlers ──────────────────────────────────────────────
  async function handleFilePicker() {
    importingCount = 1; // unknown count until dialog resolves
    error = null;
    try {
      const imported = await openFileDialog();
      if (imported.length) {
        assets = [...imported, ...assets];
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Import failed';
    } finally {
      importingCount = 0;
    }
  }

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

    const paths = Array.from(files).map((f) => (f as any).path || f.name);
    importingCount = paths.length;
    error = null;
    try {
      const imported = await importFiles(paths);
      if (imported.length) {
        assets = [...imported, ...assets];
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Import failed';
    } finally {
      importingCount = 0;
    }
  }

  // ── Asset selection ──────────────────────────────────────────────
  function selectAsset(asset: Asset) {
    if (selectedAsset?.assetId === asset.assetId) {
      selectedAsset = null;
    } else {
      selectedAsset = asset;
      // Close any open panels when switching rows
      showFingerprintsFor = null;
      signingAssetId = null;
      watermarkAssetId = null;
      watermarkResult = null;
    }
  }

  function getMetadata(asset: Asset): ImageMetadata | null {
    return parseMetadata(asset);
  }

  function canSignC2pa(asset: Asset): boolean {
    if (asset.contentType !== 'image') return false;
    return ['image/jpeg', 'image/png', 'image/tiff', 'image/webp', 'image/avif', 'image/heic', 'image/heif'].includes(asset.mimeType);
  }

  // ── C2PA signing ─────────────────────────────────────────────────
  async function openSigningPanel(assetId: string, existingArtist: string | null) {
    signingAssetId = assetId;
    creatorName = existingArtist ?? '';
    metadataWarning = null;
    metadataWarningLoading = true;
    try {
      metadataWarning = await checkMetadataBeforeSign(assetId);
    } catch {
      // Non-fatal: proceed without warning
    } finally {
      metadataWarningLoading = false;
    }
  }

  async function handleSign() {
    if (!signingAssetId || !creatorName.trim()) return;
    signing = true;
    error = null;
    try {
      const updated = await signAsset(signingAssetId, creatorName.trim(), selectedLicense);
      assets = assets.map(a => a.assetId === updated.assetId ? updated : a);
      selectedAsset = updated;
      signingAssetId = null;
    } catch (e) {
      error = e instanceof Error ? e.message : typeof e === 'string' ? e : 'Signing failed';
    } finally {
      signing = false;
    }
  }

  // ── Watermark embedding ──────────────────────────────────────────
  async function handleWatermark() {
    if (!watermarkAssetId || !watermarkPayload.trim()) return;
    watermarking = true;
    watermarkResult = null;
    error = null;
    try {
      const result = await embedWatermark(
        watermarkAssetId,
        watermarkPayload.trim(),
        watermarkStrength,
      );
      watermarkResult = result;
      if (result.success) {
        // Mark asset as watermarked in the local list
        assets = assets.map(a =>
          a.assetId === watermarkAssetId ? { ...a, watermarked: true } : a,
        );
        if (selectedAsset?.assetId === watermarkAssetId) {
          selectedAsset = { ...selectedAsset, watermarked: true };
        }
      }
    } catch (e) {
      error = e instanceof Error ? e.message : typeof e === 'string' ? e : 'Watermark embedding failed';
    } finally {
      watermarking = false;
    }
  }

  // ── Delete asset ─────────────────────────────────────────────────
  async function handleDelete(assetId: string) {
    const confirmed = confirm('Delete this asset? This action cannot be undone.');
    if (!confirmed) return;
    try {
      await deleteAsset(assetId);
      assets = assets.filter(a => a.assetId !== assetId);
      if (selectedAsset?.assetId === assetId) selectedAsset = null;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Delete failed';
    }
  }

  // ── CSV export ────────────────────────────────────────────────────
  function exportCsv() {
    const dateStr = new Date().toISOString().slice(0, 10); // YYYY-MM-DD
    const headers = ['File Name', 'Content Type', 'MIME Type', 'File Size', 'Width', 'Height', 'C2PA Signed', 'Created'];

    function escapeCsv(value: string | number | boolean | undefined | null): string {
      if (value == null) return '';
      const str = String(value);
      if (str.includes(',') || str.includes('"') || str.includes('\n')) {
        return `"${str.replace(/"/g, '""')}"`;
      }
      return str;
    }

    const rows = displayedAssets.map(a => [
      escapeCsv(a.fileName),
      escapeCsv(CONTENT_TYPE_LABELS[a.contentType] || a.contentType),
      escapeCsv(a.mimeType),
      escapeCsv(a.fileSize),
      escapeCsv(a.width ?? ''),
      escapeCsv(a.height ?? ''),
      escapeCsv(a.c2paSigned ? 'Yes' : 'No'),
      escapeCsv(new Date(a.createdAt).toISOString()),
    ].join(','));

    const csv = [headers.join(','), ...rows].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `jura_assets_${dateStr}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  }

  // ── Content type icon ────────────────────────────────────────────
  function contentTypeIcon(type: string): string {
    switch (type) {
      case 'image':    return 'IMG';
      case 'document': return 'DOC';
      case 'video':    return 'VID';
      case 'audio':    return 'AUD';
      case '3d':       return '3D';
      case 'web':      return 'WEB';
      default:         return 'FILE';
    }
  }

  // Sort arrow indicator
  function sortArrow(key: SortKey): string {
    if (sortKey !== key) return '';
    return sortDir === 'asc' ? ' ↑' : ' ↓';
  }
</script>

<div class="space-y-6">
  <!-- Page header -->
  <div class="flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-heading text-text-light dark:text-quartz" style="font-family: Georgia, 'Times New Roman', serif;">Protect</h1>
      <p class="text-flint dark:text-flint-light text-sm mt-1">
        Import, catalogue, and safeguard your digital content.
      </p>
    </div>

    <!-- Asset count + CSV export -->
    <div class="flex items-center gap-3 flex-shrink-0 pt-1">
      <span class="text-xs text-flint dark:text-flint-light" aria-live="polite" aria-atomic="true">
        {displayedAssets.length} asset{displayedAssets.length !== 1 ? 's' : ''}
        {#if displayedAssets.length !== assets.length}
          <span class="sr-only">(filtered)</span>
        {/if}
      </span>
      {#if displayedAssets.length > 0}
        <button
          class="text-xs px-3 py-2.5 min-h-[44px] inline-flex items-center rounded border border-border-light dark:border-border-dark text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:border-lapis/50 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          onclick={exportCsv}
          aria-label="Export visible assets as CSV"
        >
          Export CSV
        </button>
      {/if}
    </div>
  </div>

  <!-- Error banner -->
  {#if error}
    <div
      class="bg-cinnabar/10 border border-cinnabar/30 rounded-lg px-4 py-3 text-sm text-cinnabar dark:text-cinnabar-light"
      role="alert"
      aria-live="assertive"
    >
      {error}
    </div>
  {/if}

  <!-- Drop zone / import button -->
  <button
    class="w-full border-2 border-dashed rounded-lg p-14 text-center transition-all duration-200 cursor-pointer
           {dragOver
             ? 'border-lapis bg-lapis/5 scale-[1.01]'
             : 'border-border-light dark:border-[rgba(122,119,112,0.2)] hover:border-lapis/40 dark:hover:border-lapis/30'}
           {importingCount > 0 ? 'opacity-60 pointer-events-none' : ''}
           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
    onclick={handleFilePicker}
    aria-disabled={importingCount > 0}
  >
    {#if importingCount > 0}
      <div class="flex flex-col items-center gap-3">
        <div
          class="w-6 h-6 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
          aria-hidden="true"
        ></div>
        <p class="text-sm text-flint dark:text-flint-light" aria-live="polite">
          {importingCount > 1
            ? `Importing ${importingCount} file${importingCount !== 1 ? 's' : ''}...`
            : 'Importing files...'}
        </p>
      </div>
    {:else}
      <div class="flex flex-col items-center gap-2">
        <svg class="w-10 h-10 text-flint dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
            d="M12 16V4m0 0L8 8m4-4l4 4M4 14v4a2 2 0 002 2h12a2 2 0 002-2v-4" />
        </svg>
        <p class="text-text-light dark:text-quartz" style="font-family: Georgia, 'Times New Roman', serif;">Drop files or folders here</p>
        <p class="text-xs text-flint dark:text-flint-light mt-1">
          or click to browse &mdash; JPEG, PNG, TIFF, WebP, PDF, MP4, WAV, and more
        </p>
      </div>
    {/if}
  </button>

  <!-- Filter bar -->
  <div
    class="flex flex-wrap items-center gap-3"
    role="search"
    aria-label="Filter assets"
  >
    <!-- Content type filter -->
    <div class="flex items-center gap-2">
      <label for="filter-content-type" class="text-xs text-flint dark:text-flint-light uppercase tracking-wide flex-shrink-0">Type</label>
      <select
        id="filter-content-type"
        bind:value={filterContentType}
        class="px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite text-text-light dark:text-quartz text-sm
               hover:border-lapis/50 transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
      >
        <option value="">All Types</option>
        <option value="image">{CONTENT_TYPE_LABELS['image']}</option>
        <option value="document">{CONTENT_TYPE_LABELS['document']}</option>
        <option value="video">{CONTENT_TYPE_LABELS['video']}</option>
        <option value="audio">{CONTENT_TYPE_LABELS['audio']}</option>
        <option value="3d">{CONTENT_TYPE_LABELS['3d']}</option>
        <option value="web">{CONTENT_TYPE_LABELS['web']}</option>
      </select>
    </div>

    <!-- Status filter -->
    <div class="flex items-center gap-2">
      <label for="filter-status" class="text-xs text-flint dark:text-flint-light uppercase tracking-wide flex-shrink-0">Status</label>
      <select
        id="filter-status"
        bind:value={filterStatus}
        class="px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite text-text-light dark:text-quartz text-sm
               hover:border-lapis/50 transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
      >
        <option value="">All Status</option>
        <option value="signed">C2PA Signed</option>
        <option value="unsigned">Not Signed</option>
      </select>
    </div>

    <!-- Search input -->
    <div class="flex items-center gap-2 flex-1 min-w-48">
      <label for="filter-search" class="text-xs text-flint dark:text-flint-light uppercase tracking-wide flex-shrink-0 sr-only">Search</label>
      <div class="relative flex-1">
        <svg
          class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-flint dark:text-flint-light pointer-events-none"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
        </svg>
        <input
          id="filter-search"
          type="search"
          bind:value={searchRaw}
          placeholder="Search files..."
          class="w-full pl-8 pr-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite text-text-light dark:text-quartz text-sm
                 placeholder:text-flint/60
                 hover:border-lapis/50 transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          aria-label="Search files by name"
        />
      </div>
    </div>

    <!-- Clear filters -->
    {#if filterContentType || filterStatus || searchRaw}
      <button
        class="text-xs text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors underline underline-offset-2
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
        onclick={() => { filterContentType = ''; filterStatus = ''; searchRaw = ''; }}
        aria-label="Clear all filters"
      >
        Clear filters
      </button>
    {/if}
  </div>

  <!-- Asset list -->
  {#if displayedAssets.length === 0 && importingCount === 0}
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-10 text-center">
      {#if filterContentType || filterStatus || searchRaw}
        <p class="text-flint dark:text-flint-light">No assets match the current filters.</p>
        <button
          class="mt-3 text-sm text-lapis hover:text-lapis-dark dark:hover:text-lapis-light transition-colors underline underline-offset-2
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
          onclick={() => { filterContentType = ''; filterStatus = ''; searchRaw = ''; }}
        >
          Clear filters
        </button>
      {:else}
        <p class="text-flint dark:text-flint-light">No assets imported yet. Drop files above to get started.</p>
      {/if}
    </div>

  {:else if displayedAssets.length > 0}
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark overflow-hidden">

      <!-- Column headers (sortable) — desktop only -->
      <div
        class="hidden sm:grid grid-cols-[1fr_100px_110px_130px] gap-4 px-4 py-2 border-b border-border-light dark:border-border-dark text-xs text-flint dark:text-flint-light uppercase tracking-wide"
        role="row"
        aria-label="Sort column headers"
      >
        <!-- File Name -->
        <button
          class="flex items-center gap-1 text-left hover:text-text-light dark:hover:text-quartz transition-colors select-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite rounded-sm
                 {sortKey === 'fileName' ? 'text-text-light dark:text-quartz' : ''}"
          onclick={() => handleSort('fileName')}
          aria-label={sortKey === 'fileName'
            ? `Sort by file name, currently ${sortDir === 'asc' ? 'ascending' : 'descending'}`
            : 'Sort by file name'}
        >
          <span>File</span>
          {#if sortKey === 'fileName'}
            <span aria-hidden="true" class="text-lapis dark:text-lapis-light font-normal normal-case tracking-normal">
              {sortDir === 'asc' ? '↑' : '↓'}
            </span>
          {/if}
        </button>

        <!-- Type (non-sortable label) -->
        <span>Type</span>

        <!-- File Size -->
        <button
          class="flex items-center gap-1 text-left hover:text-text-light dark:hover:text-quartz transition-colors select-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite rounded-sm
                 {sortKey === 'fileSize' ? 'text-text-light dark:text-quartz' : ''}"
          onclick={() => handleSort('fileSize')}
          aria-label={sortKey === 'fileSize'
            ? `Sort by file size, currently ${sortDir === 'asc' ? 'ascending' : 'descending'}`
            : 'Sort by file size'}
        >
          <span>Size</span>
          {#if sortKey === 'fileSize'}
            <span aria-hidden="true" class="text-lapis dark:text-lapis-light font-normal normal-case tracking-normal">
              {sortDir === 'asc' ? '↑' : '↓'}
            </span>
          {/if}
        </button>

        <!-- Date -->
        <button
          class="flex items-center gap-1 text-left hover:text-text-light dark:hover:text-quartz transition-colors select-none
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite rounded-sm
                 {sortKey === 'createdAt' ? 'text-text-light dark:text-quartz' : ''}"
          onclick={() => handleSort('createdAt')}
          aria-label={sortKey === 'createdAt'
            ? `Sort by import date, currently ${sortDir === 'asc' ? 'ascending' : 'descending'}`
            : 'Sort by import date'}
        >
          <span>Imported</span>
          {#if sortKey === 'createdAt'}
            <span aria-hidden="true" class="text-lapis dark:text-lapis-light font-normal normal-case tracking-normal">
              {sortDir === 'asc' ? '↑' : '↓'}
            </span>
          {/if}
        </button>
      </div>

      <!-- Rows -->
      {#each displayedAssets as asset (asset.assetId)}
        <!-- Mobile card row -->
        <button
          class="sm:hidden w-full flex flex-col px-4 py-3 border-b border-border-light/50 dark:border-graphite-light/50 hover:bg-gray-50 dark:hover:bg-graphite-light/30 transition-colors text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                 {selectedAsset?.assetId === asset.assetId ? 'bg-lapis/10 border-l-2 border-l-lapis' : ''}"
          onclick={() => selectAsset(asset)}
          aria-expanded={selectedAsset?.assetId === asset.assetId}
          aria-label="View details for {asset.fileName}"
        >
          <div class="flex items-center gap-2">
            <span class="text-xs font-mono px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-text-light dark:text-flint flex-shrink-0" aria-hidden="true">{contentTypeIcon(asset.contentType)}</span>
            <p class="text-sm text-text-light dark:text-quartz truncate flex-1">{asset.fileName}</p>
            {#if asset.c2paSigned}
              <span class="text-xs px-1.5 py-0.5 rounded bg-malachite/15 text-malachite dark:text-malachite-light flex-shrink-0">Signed</span>
            {/if}
          </div>
          <div class="flex items-center gap-3 mt-1.5 text-xs text-flint dark:text-flint-light">
            <span>{asset.mimeType}</span>
            <span>{formatFileSize(asset.fileSize)}</span>
            <span>{new Date(asset.createdAt).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })}</span>
          </div>
        </button>

        <!-- Desktop row -->
        <button
          class="hidden sm:grid w-full grid-cols-[1fr_100px_110px_130px] gap-4 px-4 py-3 border-b border-border-light/50 dark:border-graphite-light/50
                 hover:bg-gray-50 dark:hover:bg-graphite-light/30 transition-colors text-left
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                 {selectedAsset?.assetId === asset.assetId
                   ? 'bg-lapis/10 border-l-2 border-l-lapis'
                   : ''}"
          onclick={() => selectAsset(asset)}
          aria-expanded={selectedAsset?.assetId === asset.assetId}
          aria-label="View details for {asset.fileName}"
        >
          <div class="flex items-center gap-3 min-w-0">
            <span
              class="text-xs font-mono px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-text-light dark:text-flint flex-shrink-0"
              aria-hidden="true"
            >
              {contentTypeIcon(asset.contentType)}
            </span>
            <div class="min-w-0">
              <p class="text-sm text-text-light dark:text-quartz truncate">{asset.fileName}</p>
              <p class="text-xs text-flint dark:text-flint-light truncate">{asset.mimeType}</p>
            </div>
          </div>

          <span class="text-sm text-flint dark:text-flint-light self-center">
            {CONTENT_TYPE_LABELS[asset.contentType] || asset.contentType}
          </span>

          <span class="text-sm text-flint dark:text-flint-light self-center">{formatFileSize(asset.fileSize)}</span>

          <span class="text-xs text-flint dark:text-flint-light self-center">
            {new Date(asset.createdAt).toLocaleDateString('en-GB', {
              day: 'numeric',
              month: 'short',
              hour: '2-digit',
              minute: '2-digit',
            })}
          </span>
        </button>

        <!-- Expanded detail panel -->
        {#if selectedAsset?.assetId === asset.assetId}
          {@const meta = getMetadata(asset)}
          <div
            class="px-4 py-4 bg-gray-50 dark:bg-obsidian-dark/50 border-b border-border-light dark:border-border-dark"
            role="region"
            aria-label="Asset details for {asset.fileName}"
          >
            <div class="grid grid-cols-2 md:grid-cols-3 gap-x-8 gap-y-3 text-sm">

              <!-- File path -->
              <div>
                <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Path</span>
                <p class="text-text-light dark:text-quartz text-xs mt-0.5 truncate" title={asset.filePath}>{asset.filePath}</p>
              </div>

              {#if asset.width && asset.height}
                <div>
                  <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Dimensions</span>
                  <p class="text-text-light dark:text-quartz mt-0.5">{asset.width} &times; {asset.height} px</p>
                </div>
              {/if}

              <!-- EXIF metadata -->
              {#if meta}
                {#if meta.cameraMake || meta.cameraModel}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Camera</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{[meta.cameraMake, meta.cameraModel].filter(Boolean).join(' ')}</p>
                  </div>
                {/if}
                {#if meta.datetimeOriginal}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Date Taken</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.datetimeOriginal}</p>
                  </div>
                {/if}
                {#if meta.software}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Software</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.software}</p>
                  </div>
                {/if}
                {#if meta.iso}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">ISO</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.iso}</p>
                  </div>
                {/if}
                {#if meta.focalLength}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Focal Length</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.focalLength}</p>
                  </div>
                {/if}
                {#if meta.exposureTime}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Exposure</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.exposureTime}</p>
                  </div>
                {/if}
                {#if meta.fNumber}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Aperture</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.fNumber}</p>
                  </div>
                {/if}
                {#if meta.copyright}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Copyright</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.copyright}</p>
                  </div>
                {/if}
                {#if meta.artist}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">Artist</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.artist}</p>
                  </div>
                {/if}
                {#if meta.gpsLatitude != null && meta.gpsLongitude != null}
                  <div>
                    <span class="text-xs text-flint dark:text-flint-light uppercase tracking-wide">GPS</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.gpsLatitude.toFixed(6)}, {meta.gpsLongitude.toFixed(6)}</p>
                  </div>
                {/if}
              {/if}

              <!-- Status badges -->
              <div class="col-span-full flex gap-2 mt-2 flex-wrap">
                <span
                  class="text-xs px-2 py-0.5 rounded {asset.c2paSigned
                    ? 'bg-malachite/15 text-malachite dark:text-malachite-light'
                    : 'bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light'}"
                >
                  {asset.c2paSigned ? 'C2PA Signed' : 'Not Signed'}
                </span>
                <span
                  class="text-xs px-2 py-0.5 rounded {asset.watermarked
                    ? 'bg-malachite/15 text-malachite dark:text-malachite-light'
                    : 'bg-gray-100 dark:bg-graphite-light text-flint dark:text-flint-light'}"
                >
                  {asset.watermarked ? 'Watermarked' : 'No Watermark'}
                </span>
                {#if asset.contentType === 'image'}
                  <span class="text-xs px-2 py-0.5 rounded bg-lapis/15 text-lapis dark:text-lapis-light">
                    Fingerprinted
                  </span>
                {/if}
              </div>

              <!-- C2PA signing form -->
              {#if !asset.c2paSigned && canSignC2pa(asset)}
                {#if signingAssetId === asset.assetId}
                  <div class="col-span-full mt-3 p-3 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark">
                    <p class="text-sm text-text-light dark:text-quartz mb-3">Sign with C2PA Content Credentials</p>

                    {#if metadataWarningLoading}
                      <div class="mb-3 flex items-center gap-2 text-xs text-flint dark:text-flint-light">
                        <span
                          class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                          aria-hidden="true"
                        ></span>
                        Checking existing metadata...
                      </div>
                    {/if}

                    {#if metadataWarning?.warningMessage}
                      <div
                        class="mb-3 px-3 py-2 rounded-md bg-amber/10 border border-amber/30 text-xs text-amber dark:text-amber"
                        role="alert"
                      >
                        <span class="font-medium">Note:</span>
                        {metadataWarning.warningMessage}
                        {#if metadataWarning.hasExistingC2pa}
                          The new C2PA signing will be added as an additional assertion layer.
                        {/if}
                      </div>
                    {/if}

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                      <div>
                        <label
                          class="text-xs text-flint dark:text-flint-light uppercase tracking-wide"
                          for="creator-name"
                        >
                          Creator Name
                        </label>
                        <input
                          id="creator-name"
                          type="text"
                          bind:value={creatorName}
                          class="w-full mt-1 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian-dark text-text-light dark:text-quartz text-sm
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                          placeholder="Your name or organisation"
                        />
                      </div>
                      <div>
                        <label
                          class="text-xs text-flint dark:text-flint-light uppercase tracking-wide"
                          for="license-select"
                        >
                          Licence
                        </label>
                        <select
                          id="license-select"
                          bind:value={selectedLicense}
                          class="w-full mt-1 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian-dark text-text-light dark:text-quartz text-sm
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                        >
                          <option value="All Rights Reserved">All Rights Reserved</option>
                          <option value="CC-BY-4.0">CC BY 4.0</option>
                          <option value="CC-BY-NC-4.0">CC BY-NC 4.0</option>
                          <option value="CC-BY-SA-4.0">CC BY-SA 4.0</option>
                          <option value="CC0-1.0">CC0 (Public Domain)</option>
                        </select>
                      </div>
                    </div>
                    <div class="flex gap-2 mt-3">
                      <button
                        class="px-4 py-2.5 min-h-[44px] inline-flex items-center bg-lapis text-white text-sm rounded hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                               disabled:opacity-50 disabled:cursor-not-allowed
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                        onclick={handleSign}
                        disabled={signing || !creatorName.trim()}
                      >
                        {signing ? 'Signing...' : 'Sign'}
                      </button>
                      <button
                        class="px-4 py-2.5 min-h-[44px] inline-flex items-center text-flint dark:text-flint-light text-sm rounded hover:text-text-light dark:hover:text-quartz transition-colors
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                        onclick={() => signingAssetId = null}
                        disabled={signing}
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                {:else}
                  <button
                    class="col-span-full mt-2 px-4 py-2.5 min-h-[44px] inline-flex items-center text-sm border border-lapis/50 text-lapis rounded
                           hover:bg-lapis/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    onclick={() => openSigningPanel(asset.assetId, meta?.artist ?? null)}
                  >
                    Sign with C2PA
                  </button>
                {/if}
              {/if}

              <!-- Watermark embedding -->
              {#if canSignC2pa(asset)}
                {#if watermarkAssetId === asset.assetId}
                  <!-- Watermark form -->
                  <div
                    class="col-span-full mt-3 p-3 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark"
                    role="region"
                    aria-label="Embed watermark"
                  >
                    <p
                      class="text-sm text-text-light dark:text-quartz mb-3"
                      style="font-family: Georgia, 'Times New Roman', serif;"
                    >
                      Embed Invisible Watermark
                    </p>

                    {#if watermarkResult}
                      <!-- Success / error feedback -->
                      {#if watermarkResult.success}
                        <div
                          class="mb-3 px-3 py-2 rounded-md bg-malachite/10 border border-malachite/30 text-xs text-malachite dark:text-malachite-light"
                          role="status"
                          aria-live="polite"
                        >
                          <span class="font-medium">Watermark embedded.</span>
                          Output saved to:
                          <span class="block mt-0.5 font-mono break-all">{watermarkResult.outputPath}</span>
                        </div>
                      {:else}
                        <div
                          class="mb-3 px-3 py-2 rounded-md bg-cinnabar/10 border border-cinnabar/30 text-xs text-cinnabar dark:text-cinnabar-light"
                          role="alert"
                          aria-live="assertive"
                        >
                          <span class="font-medium">Embedding failed:</span> {watermarkResult.message}
                        </div>
                      {/if}
                    {/if}

                    <div class="grid grid-cols-1 gap-3">
                      <!-- Institution / payload input -->
                      <div>
                        <label
                          class="text-xs text-flint dark:text-flint-light uppercase tracking-wide"
                          for="watermark-payload-{asset.assetId}"
                        >
                          Institution Name or Identifier
                        </label>
                        <input
                          id="watermark-payload-{asset.assetId}"
                          type="text"
                          bind:value={watermarkPayload}
                          placeholder="e.g. National Archive UK — 2026"
                          class="w-full mt-1 px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian-dark text-text-light dark:text-quartz text-sm
                                 placeholder:text-flint/60
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                          aria-describedby="watermark-payload-hint-{asset.assetId}"
                        />
                        <p
                          id="watermark-payload-hint-{asset.assetId}"
                          class="mt-1 text-xs text-flint dark:text-flint-light"
                        >
                          This text will be encoded invisibly into the file. Max 64 characters.
                        </p>
                      </div>

                      <!-- Strength selector -->
                      <fieldset>
                        <legend class="text-xs text-flint dark:text-flint-light uppercase tracking-wide mb-2">
                          Embedding Strength
                        </legend>
                        <div
                          class="flex gap-2"
                          role="group"
                          aria-label="Embedding strength"
                        >
                          {#each [
                            { value: 1, label: 'Low', hint: 'Minimal quality impact, lower robustness' },
                            { value: 2, label: 'Medium', hint: 'Balanced quality and robustness' },
                            { value: 3, label: 'High', hint: 'Maximum robustness, slight quality reduction' },
                          ] as opt (opt.value)}
                            <button
                              type="button"
                              class="flex-1 min-h-[44px] px-3 py-2 text-sm rounded border transition-colors
                                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                                     {watermarkStrength === opt.value
                                       ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                                       : 'border-border-light dark:border-border-dark text-flint dark:text-flint-light hover:border-lapis/50 hover:text-text-light dark:hover:text-quartz'}"
                              onclick={() => watermarkStrength = opt.value}
                              aria-pressed={watermarkStrength === opt.value}
                              title={opt.hint}
                            >
                              {opt.label}
                            </button>
                          {/each}
                        </div>
                      </fieldset>
                    </div>

                    <div class="flex gap-2 mt-3">
                      <button
                        class="px-4 py-2.5 min-h-[44px] inline-flex items-center gap-2 bg-lapis text-white text-sm rounded hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                               disabled:opacity-50 disabled:cursor-not-allowed
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                        onclick={handleWatermark}
                        disabled={watermarking || !watermarkPayload.trim()}
                        aria-busy={watermarking}
                      >
                        {#if watermarking}
                          <span
                            class="w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full motion-safe:animate-spin"
                            aria-hidden="true"
                          ></span>
                          Embedding...
                        {:else}
                          Embed Watermark
                        {/if}
                      </button>
                      <button
                        class="px-4 py-2.5 min-h-[44px] inline-flex items-center text-flint dark:text-flint-light text-sm rounded hover:text-text-light dark:hover:text-quartz transition-colors
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                        onclick={() => { watermarkAssetId = null; watermarkResult = null; }}
                        disabled={watermarking}
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                {:else if !asset.watermarked}
                  <!-- Watermark trigger button (only when not yet watermarked) -->
                  <button
                    class="col-span-full mt-2 px-4 py-2.5 min-h-[44px] inline-flex items-center text-sm border border-lapis/50 text-lapis rounded
                           hover:bg-lapis/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    onclick={() => {
                      watermarkAssetId = asset.assetId;
                      watermarkPayload = '';
                      watermarkStrength = 2;
                      watermarkResult = null;
                      // Close other panels
                      signingAssetId = null;
                      showFingerprintsFor = null;
                    }}
                    aria-label="Embed invisible watermark in {asset.fileName}"
                  >
                    Watermark
                  </button>
                {/if}
              {/if}

              <!-- Fingerprint viewer -->
              {#if asset.contentType === 'image'}
                {#if showFingerprintsFor === asset.assetId}
                  <div class="col-span-full mt-3 p-3 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark">
                    <div class="flex items-center justify-between mb-2">
                      <p class="text-sm text-text-light dark:text-quartz">Perceptual Fingerprints</p>
                      <button
                        class="text-xs text-flint dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite rounded"
                        onclick={() => showFingerprintsFor = null}
                        aria-label="Close fingerprint panel"
                      >
                        Close
                      </button>
                    </div>
                    {#if loadingFingerprints}
                      <div class="flex items-center gap-2">
                        <div
                          class="w-3.5 h-3.5 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                          aria-hidden="true"
                        ></div>
                        <p class="text-xs text-flint dark:text-flint-light">Loading...</p>
                      </div>
                    {:else if fingerprints.length === 0}
                      <p class="text-xs text-flint dark:text-flint-light">No fingerprints found.</p>
                    {:else}
                      <div class="grid grid-cols-1 gap-2">
                        {#each fingerprints as fp}
                          <div class="flex items-center justify-between text-xs">
                            <span class="text-flint dark:text-flint-light uppercase tracking-wide w-32">
                              {HASH_TYPE_LABELS[fp.hashType] || fp.hashType}
                            </span>
                            <code class="text-text-light dark:text-quartz font-mono bg-gray-100 dark:bg-obsidian-dark/50 px-2 py-0.5 rounded">
                              {fp.hashValue}
                            </code>
                          </div>
                        {/each}
                      </div>

                      {#if similarAssets.length > 0}
                        <div class="mt-3 pt-3 border-t border-border-light dark:border-graphite-light">
                          <p class="text-xs text-amber dark:text-amber-light mb-2">
                            {similarAssets.length} similar asset{similarAssets.length !== 1 ? 's' : ''} found
                          </p>
                          {#each similarAssets as match}
                            <div class="flex items-center justify-between text-xs py-1">
                              <span class="text-text-light dark:text-quartz">{match.fileName}</span>
                              <span class="text-flint dark:text-flint-light">
                                {Math.round(match.similarity * 100)}% similar ({HASH_TYPE_LABELS[match.hashType] || match.hashType})
                              </span>
                            </div>
                          {/each}
                        </div>
                      {/if}
                    {/if}
                  </div>
                {:else}
                  <button
                    class="col-span-full mt-2 px-4 py-2.5 min-h-[44px] inline-flex items-center text-sm border border-lapis/50 text-lapis rounded
                           hover:bg-lapis/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    onclick={async () => {
                      showFingerprintsFor = asset.assetId;
                      loadingFingerprints = true;
                      fingerprints = await getFingerprints(asset.assetId);
                      similarAssets = await findSimilar(asset.assetId);
                      loadingFingerprints = false;
                    }}
                  >
                    View Fingerprints
                  </button>
                {/if}
              {/if}

              <!-- Delete asset -->
              <div class="col-span-full mt-3 pt-3 border-t border-border-light/50 dark:border-graphite-light/50 flex justify-end">
                <button
                  class="text-sm text-cinnabar hover:text-cinnabar-dark dark:hover:text-cinnabar-light transition-colors
                         focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded px-2 py-1"
                  onclick={() => handleDelete(asset.assetId)}
                  aria-label="Delete asset {asset.fileName}"
                >
                  Delete Asset
                </button>
              </div>

            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>
