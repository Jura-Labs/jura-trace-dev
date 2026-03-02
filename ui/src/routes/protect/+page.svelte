<script lang="ts">
  import { onMount } from 'svelte';
  import { getAssets, importFiles, openFileDialog } from '$lib/api';
  import {
    type Asset,
    type ImageMetadata,
    parseMetadata,
    formatFileSize,
    CONTENT_TYPE_LABELS,
  } from '$lib/types';

  let assets: Asset[] = $state([]);
  let importing = $state(false);
  let dragOver = $state(false);
  let selectedAsset: Asset | null = $state(null);
  let error: string | null = $state(null);

  onMount(async () => {
    assets = await getAssets();
  });

  async function handleFilePicker() {
    importing = true;
    error = null;
    try {
      const imported = await openFileDialog();
      if (imported.length) {
        assets = [...imported, ...assets];
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Import failed';
    } finally {
      importing = false;
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

    // In Tauri, dropped files have full paths
    const paths = Array.from(files).map((f) => (f as any).path || f.name);

    importing = true;
    error = null;
    try {
      const imported = await importFiles(paths);
      if (imported.length) {
        assets = [...imported, ...assets];
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Import failed';
    } finally {
      importing = false;
    }
  }

  function selectAsset(asset: Asset) {
    selectedAsset = selectedAsset?.assetId === asset.assetId ? null : asset;
  }

  function getMetadata(asset: Asset): ImageMetadata | null {
    return parseMetadata(asset);
  }

  function contentTypeIcon(type: string): string {
    switch (type) {
      case 'image': return 'IMG';
      case 'document': return 'DOC';
      case 'video': return 'VID';
      case 'audio': return 'AUD';
      case '3d': return '3D';
      case 'web': return 'WEB';
      default: return 'FILE';
    }
  }
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-heading text-quartz">Protect</h1>
      <p class="text-flint text-sm mt-1">
        Import, catalogue, and safeguard your digital content.
      </p>
    </div>
    <div class="flex items-center gap-3">
      <span class="text-xs text-flint">{assets.length} asset{assets.length !== 1 ? 's' : ''}</span>
    </div>
  </div>

  {#if error}
    <div class="bg-cinnabar/10 border border-cinnabar/30 rounded-lg px-4 py-3 text-sm text-cinnabar-light">
      {error}
    </div>
  {/if}

  <!-- Drop zone -->
  <button
    class="w-full border-2 border-dashed rounded-lg p-12 text-center transition-all duration-200 cursor-pointer
           {dragOver
             ? 'border-lapis bg-lapis/5 scale-[1.01]'
             : 'border-graphite-light hover:border-lapis/50'}
           {importing ? 'opacity-60 pointer-events-none' : ''}"
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
    onclick={handleFilePicker}
    aria-label="Drop files here or click to import"
  >
    {#if importing}
      <div class="flex flex-col items-center gap-3">
        <div class="w-6 h-6 border-2 border-lapis border-t-transparent rounded-full animate-spin"></div>
        <p class="text-sm text-flint">Importing files...</p>
      </div>
    {:else}
      <div class="flex flex-col items-center gap-2">
        <svg class="w-10 h-10 text-flint" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
            d="M12 16V4m0 0L8 8m4-4l4 4M4 14v4a2 2 0 002 2h12a2 2 0 002-2v-4" />
        </svg>
        <p class="text-quartz">Drop files or folders here</p>
        <p class="text-xs text-flint">
          or click to browse — JPEG, PNG, TIFF, WebP, PDF, MP4, WAV, and more
        </p>
      </div>
    {/if}
  </button>

  <!-- Asset list -->
  {#if assets.length === 0 && !importing}
    <div class="bg-graphite rounded-lg border border-graphite-light p-10 text-center">
      <p class="text-flint">No assets imported yet. Drop files above to get started.</p>
    </div>
  {:else if assets.length > 0}
    <div class="bg-graphite rounded-lg border border-graphite-light overflow-hidden">
      <!-- Header -->
      <div class="grid grid-cols-[1fr_100px_100px_120px] gap-4 px-4 py-2 border-b border-graphite-light text-xs text-flint uppercase tracking-wide">
        <span>File</span>
        <span>Type</span>
        <span>Size</span>
        <span>Imported</span>
      </div>

      <!-- Rows -->
      {#each assets as asset (asset.assetId)}
        <button
          class="w-full grid grid-cols-[1fr_100px_100px_120px] gap-4 px-4 py-3 border-b border-graphite-light/50 hover:bg-graphite-light/30 transition-colors text-left
                 {selectedAsset?.assetId === asset.assetId ? 'bg-lapis/10 border-l-2 border-l-lapis' : ''}"
          onclick={() => selectAsset(asset)}
        >
          <div class="flex items-center gap-3 min-w-0">
            <span class="text-xs font-mono px-1.5 py-0.5 rounded bg-graphite-light text-flint flex-shrink-0">{contentTypeIcon(asset.contentType)}</span>
            <div class="min-w-0">
              <p class="text-sm text-quartz truncate">{asset.fileName}</p>
              <p class="text-xs text-flint truncate">{asset.mimeType}</p>
            </div>
          </div>
          <span class="text-sm text-flint self-center">
            {CONTENT_TYPE_LABELS[asset.contentType] || asset.contentType}
          </span>
          <span class="text-sm text-flint self-center">{formatFileSize(asset.fileSize)}</span>
          <span class="text-xs text-flint self-center">
            {new Date(asset.createdAt).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })}
          </span>
        </button>

        <!-- Expanded metadata panel -->
        {#if selectedAsset?.assetId === asset.assetId}
          {@const meta = getMetadata(asset)}
          <div class="px-4 py-4 bg-obsidian/50 border-b border-graphite-light">
            <div class="grid grid-cols-2 md:grid-cols-3 gap-x-8 gap-y-3 text-sm">
              <!-- File info -->
              <div>
                <span class="text-xs text-flint uppercase tracking-wide">Path</span>
                <p class="text-quartz text-xs mt-0.5 truncate" title={asset.filePath}>{asset.filePath}</p>
              </div>
              {#if asset.width && asset.height}
                <div>
                  <span class="text-xs text-flint uppercase tracking-wide">Dimensions</span>
                  <p class="text-quartz mt-0.5">{asset.width} × {asset.height} px</p>
                </div>
              {/if}

              <!-- EXIF metadata (images) -->
              {#if meta}
                {#if meta.cameraMake || meta.cameraModel}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Camera</span>
                    <p class="text-quartz mt-0.5">{[meta.cameraMake, meta.cameraModel].filter(Boolean).join(' ')}</p>
                  </div>
                {/if}
                {#if meta.datetimeOriginal}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Date Taken</span>
                    <p class="text-quartz mt-0.5">{meta.datetimeOriginal}</p>
                  </div>
                {/if}
                {#if meta.software}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Software</span>
                    <p class="text-quartz mt-0.5">{meta.software}</p>
                  </div>
                {/if}
                {#if meta.iso}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">ISO</span>
                    <p class="text-quartz mt-0.5">{meta.iso}</p>
                  </div>
                {/if}
                {#if meta.focalLength}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Focal Length</span>
                    <p class="text-quartz mt-0.5">{meta.focalLength}</p>
                  </div>
                {/if}
                {#if meta.exposureTime}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Exposure</span>
                    <p class="text-quartz mt-0.5">{meta.exposureTime}</p>
                  </div>
                {/if}
                {#if meta.fNumber}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Aperture</span>
                    <p class="text-quartz mt-0.5">{meta.fNumber}</p>
                  </div>
                {/if}
                {#if meta.copyright}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Copyright</span>
                    <p class="text-quartz mt-0.5">{meta.copyright}</p>
                  </div>
                {/if}
                {#if meta.artist}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">Artist</span>
                    <p class="text-quartz mt-0.5">{meta.artist}</p>
                  </div>
                {/if}
                {#if meta.gpsLatitude != null && meta.gpsLongitude != null}
                  <div>
                    <span class="text-xs text-flint uppercase tracking-wide">GPS</span>
                    <p class="text-quartz mt-0.5">{meta.gpsLatitude.toFixed(6)}, {meta.gpsLongitude.toFixed(6)}</p>
                  </div>
                {/if}
              {/if}

              <!-- Status badges -->
              <div class="col-span-full flex gap-2 mt-2">
                <span class="text-xs px-2 py-0.5 rounded {asset.c2paSigned ? 'bg-malachite/15 text-malachite-light' : 'bg-graphite-light text-flint'}">
                  {asset.c2paSigned ? 'C2PA Signed' : 'Not Signed'}
                </span>
                <span class="text-xs px-2 py-0.5 rounded {asset.watermarked ? 'bg-malachite/15 text-malachite-light' : 'bg-graphite-light text-flint'}">
                  {asset.watermarked ? 'Watermarked' : 'No Watermark'}
                </span>
              </div>
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>
