<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getFilteredAssets, deleteAsset, importFiles, openFileDialog, signAsset, checkMetadataBeforeSign, embedWatermark, getVideoMetadata, getAudioMetadata, getVideoFrames, getSigningMode } from '$lib/api';
  import ContextualHelpLink from '$lib/components/ContextualHelpLink.svelte';
  import { createBlobTracker } from '$lib/blob';
  import {
    type Asset,
    type ContentType,
    type ImageMetadata,
    type VideoMetadataResult,
    type AudioMetadataResult,
    type VideoFramesResult,
    type MetadataSigningWarning,
    type SigningMode,
    type WatermarkEmbedResult,
    parseMetadata,
    formatFileSize,
    CONTENT_TYPE_LABELS,
  } from '$lib/types';

  const blobs = createBlobTracker();
  async function setupTauriProtectDragDrop() {
    try {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      const webview = getCurrentWebviewWindow();
      _unlistenProtectDragDrop = await webview.onDragDropEvent((event) => {
        if (event.payload.type === 'over') {
          dragOver = true;
        } else if (event.payload.type === 'leave') {
          dragOver = false;
        } else if (event.payload.type === 'drop') {
          dragOver = false;
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            importingCount = paths.length;
            error = null;
            importFiles(paths).then((imported) => {
              if (imported.length) {
                assets = [...imported, ...assets];
              }
            }).catch((e) => {
              error = e instanceof Error ? e.message : String(e);
            }).finally(() => {
              importingCount = 0;
            });
          }
        }
      });
    } catch {
      // Not in Tauri — browser drag-and-drop handles it
    }
  }

  onMount(() => {
    setupTauriProtectDragDrop();
  });

  onDestroy(() => {
    blobs.revokeAll();
    _unlistenProtectDragDrop?.();
  });

  // ── View layout ──────────────────────────────────────────────────
  let viewLayout = $state<'list' | 'grid'>('list');

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
  let lastSignedAssetId: string | null = $state(null); // tracks which asset was most recently signed

  // Active signing mode (Local/Sovereign vs Conformant). The value is a
  // per-install setting managed in /settings; surfaced read-only here
  // so users see which mode they will sign in BEFORE clicking Sign.
  // JTV-115 — Amara persona blocker: Sovereign-mode credentials display
  // as `signingCredential.untrusted` in third-party validators, which
  // is non-negotiable for ICC submissions.
  let signingMode = $state<SigningMode>('bedrock');
  $effect(() => {
    void (async () => {
      try {
        signingMode = await getSigningMode();
      } catch {
        // getSigningMode throws in browser mock; default to bedrock.
        signingMode = 'bedrock';
      }
    })();
  });


  // ── Watermark state ───────────────────────────────────────────────
  let watermarkAssetId: string | null = $state(null);
  let watermarkPayload = $state('');
  let watermarkStrength = $state<number>(2); // 1 = low, 2 = medium, 3 = high
  let watermarking = $state(false);
  let watermarkResult = $state<WatermarkEmbedResult | null>(null);

  // ── Batch C2PA sign state ─────────────────────────────────────────
  let showBatchSign = $state(false);
  let batchSignCreatorName = $state('');
  let batchSignLicense = $state('All Rights Reserved');
  let batchSignRunning = $state(false);
  let batchSignProgress = $state(0);
  let batchSignTotal = $state(0);
  let batchSignCurrentFile = $state('');
  let batchSignSuccessCount = $state(0);
  let batchSignFailCount = $state(0);
  let batchSignCancelled = $state(false);
  let batchSignDone = $state(false);
  let batchSignEta = $state<string | null>(null);
  let batchSignErrors = $state<{ fileName: string; error: string }[]>([]);
  let showBatchSignErrors = $state(false);
  let _batchSignTimes: number[] = [];

  // ── Batch watermark state ─────────────────────────────────────────
  let showBatchWatermark = $state(false);
  let batchPayload = $state('');
  let batchStrength = $state<number>(2);
  let batchRunning = $state(false);
  let batchProgress = $state(0);
  let batchTotal = $state(0);
  let batchCurrentFile = $state('');
  let batchSuccessCount = $state(0);
  let batchFailCount = $state(0);
  let batchCancelled = $state(false);
  let batchDone = $state(false);
  let batchEta = $state<string | null>(null);  // "~X min remaining"
  let batchErrors = $state<{ fileName: string; error: string }[]>([]);
  let showBatchErrors = $state(false);
  let _batchTimes: number[] = [];  // per-file durations in ms for ETA

  // ── Video / Audio metadata state ───────────────────────────────────
  let videoMetadata = $state<VideoMetadataResult | null>(null);
  let audioMetadata = $state<AudioMetadataResult | null>(null);
  let videoFrames = $state<VideoFramesResult | null>(null);
  let loadingMediaMeta = $state(false);

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
    // 'watermarked' and 'unprotected' are applied client-side; 'signed' goes to backend
    const c2paSigned  = filterStatus === 'signed' ? true : undefined;
    const query = searchDebounced.trim() || undefined;

    getFilteredAssets(contentType, c2paSigned, query).then((result) => {
      assets = result;
    });
  });

  // ── Sorted + client-side filtered derived list ───────────────────
  const displayedAssets = $derived(
    [...assets]
      .filter(a => {
        if (filterStatus === 'watermarked') return a.watermarked;
        if (filterStatus === 'unprotected') return !a.c2paSigned && !a.watermarked;
        return true;
      })
      .sort((a, b) => {
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

  // ── Images eligible for batch watermarking ───────────────────────
  const unwatermarkedImages = $derived(
    assets.filter(a => !a.watermarked && canWatermark(a))
  );

  // ── Assets eligible for batch C2PA signing ────────────────────────
  const unsignedAssets = $derived(
    assets.filter(a => !a.c2paSigned && canSignC2pa(a))
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

  // ── Drag and drop ─────────────────────────────────────────────────
  let _unlistenProtectDragDrop: (() => void) | null = null;

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

    // Browser File objects don't have .path in Tauri v2 — Tauri drag-drop listener handles this
    const paths = Array.from(files).map((f) => (f as any).path || f.name);
    if (paths.every(p => !p.includes('/') && !p.includes('\\')) && typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      // In Tauri but no full paths — the Tauri drag-drop listener should handle
      return;
    }
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
      confirmDeleteId = null;
      videoMetadata = null;
      audioMetadata = null;
      videoFrames = null;
    } else {
      selectedAsset = asset;
      // Close any open panels when switching rows
      signingAssetId = null;
      watermarkAssetId = null;
      watermarkResult = null;
      lastSignedAssetId = null;
      confirmDeleteId = null;
      // Reset and fetch media metadata for video/audio assets
      videoMetadata = null;
      audioMetadata = null;
      videoFrames = null;
      if (asset.contentType === 'video' || asset.contentType === 'audio') {
        fetchMediaMetadata(asset);
      }
    }
  }

  async function fetchMediaMetadata(asset: Asset) {
    loadingMediaMeta = true;
    try {
      if (asset.contentType === 'video') {
        const [meta, frames] = await Promise.all([
          getVideoMetadata(asset.assetId),
          getVideoFrames(asset.assetId, 4),
        ]);
        videoMetadata = meta;
        videoFrames = frames;
      } else if (asset.contentType === 'audio') {
        audioMetadata = await getAudioMetadata(asset.assetId);
      }
    } catch {
      // Non-fatal — media metadata is supplementary
    } finally {
      loadingMediaMeta = false;
    }
  }

  function getMetadata(asset: Asset): ImageMetadata | null {
    return parseMetadata(asset);
  }

  function canSignC2pa(asset: Asset): boolean {
    if (asset.contentType !== 'image') return false;
    return ['image/jpeg', 'image/png', 'image/tiff', 'image/webp', 'image/avif', 'image/heic', 'image/heif'].includes(asset.mimeType);
  }

  /** Mirror of Rust `supports_watermarking` — raster bitmaps only (DWT-DCT-SVD). */
  function canWatermark(asset: Asset): boolean {
    if (asset.contentType !== 'image') return false;
    return ['image/jpeg', 'image/png', 'image/tiff', 'image/webp', 'image/bmp', 'image/avif'].includes(asset.mimeType);
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
      lastSignedAssetId = updated.assetId;
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

  // ── Delete confirmation state ─────────────────────────────────────
  let confirmDeleteId = $state<string | null>(null);

  // ── Delete asset ─────────────────────────────────────────────────
  async function handleDelete(assetId: string) {
    try {
      await deleteAsset(assetId);
      confirmDeleteId = null;
      assets = assets.filter(a => a.assetId !== assetId);
      if (selectedAsset?.assetId === assetId) selectedAsset = null;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Delete failed';
    }
  }

  // ── CSV export ────────────────────────────────────────────────────
  function exportCsv() {
    const dateStr = new Date().toISOString().slice(0, 10); // YYYY-MM-DD
    const headers = [
      'File Name',
      'Content Type',
      'MIME Type',
      'File Size',
      'Width',
      'Height',
      'C2PA Signed',
      'Watermarked',
      'File Path',
      'Created',
    ];

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
      escapeCsv(a.watermarked ? 'Yes' : 'No'),
      escapeCsv(a.filePath),
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

  // ── Batch watermark handler ───────────────────────────────────────
  async function handleBatchWatermark() {
    if (!batchPayload.trim() || unwatermarkedImages.length === 0) return;
    batchRunning = true;
    batchDone = false;
    batchProgress = 0;
    batchTotal = unwatermarkedImages.length;
    batchSuccessCount = 0;
    batchFailCount = 0;
    batchCancelled = false;
    batchCurrentFile = '';
    batchEta = null;
    batchErrors = [];
    showBatchErrors = false;
    _batchTimes = [];

    for (const asset of unwatermarkedImages) {
      if (batchCancelled) break;
      batchCurrentFile = asset.fileName;
      batchProgress++;

      const t0 = performance.now();
      try {
        const result = await embedWatermark(asset.assetId, batchPayload.trim(), batchStrength);
        if (result.success) {
          assets = assets.map(a => a.assetId === asset.assetId ? { ...a, watermarked: true } : a);
          batchSuccessCount++;
        } else {
          batchFailCount++;
          batchErrors = [...batchErrors, { fileName: asset.fileName, error: result.message || 'Watermark failed' }];
        }
      } catch (e) {
        batchFailCount++;
        batchErrors = [...batchErrors, { fileName: asset.fileName, error: e instanceof Error ? e.message : 'Unknown error' }];
      }
      _batchTimes.push(performance.now() - t0);

      // Compute ETA from windowed rolling average (last 8 files)
      const remaining = batchTotal - batchProgress;
      if (remaining > 0 && _batchTimes.length > 0) {
        const window = _batchTimes.slice(-8);
        const avg = window.reduce((a, b) => a + b, 0) / window.length;
        const etaMs = avg * remaining;
        if (etaMs < 60_000) {
          batchEta = `~${Math.max(1, Math.round(etaMs / 1000))}s remaining`;
        } else {
          batchEta = `~${Math.ceil(etaMs / 60_000)} min remaining`;
        }
      } else {
        batchEta = null;
      }
    }

    batchRunning = false;
    batchDone = true;
    batchCurrentFile = '';
    batchEta = null;
  }

  function exportBatchErrors() {
    if (batchErrors.length === 0) return;
    const csv = ['File Name,Error', ...batchErrors.map(e => `"${e.fileName.replace(/"/g, '""')}","${e.error.replace(/"/g, '""')}"`)].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `jura_batch_errors_${new Date().toISOString().slice(0, 10)}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  }

  function openBatchWatermark() {
    showBatchWatermark = true;
    batchDone = false;
    batchProgress = 0;
    batchSuccessCount = 0;
    batchFailCount = 0;
    batchCancelled = false;
    batchCurrentFile = '';
    batchEta = null;
    // Pre-fill from last single-asset watermark payload if available
    batchPayload = watermarkPayload || batchPayload;
  }

  function closeBatchWatermark() {
    if (batchRunning) return; // block dismiss while running
    showBatchWatermark = false;
  }

  // ── Batch C2PA sign handlers ──────────────────────────────────────
  function openBatchSign() {
    showBatchSign = true;
    batchSignDone = false;
    batchSignProgress = 0;
    batchSignSuccessCount = 0;
    batchSignFailCount = 0;
    batchSignCancelled = false;
    batchSignCurrentFile = '';
    batchSignEta = null;
    batchSignErrors = [];
    showBatchSignErrors = false;
    _batchSignTimes = [];
    batchSignCreatorName = creatorName || batchSignCreatorName;
  }

  function closeBatchSign() {
    if (batchSignRunning) return;
    showBatchSign = false;
  }

  async function handleBatchSign() {
    if (!batchSignCreatorName.trim() || unsignedAssets.length === 0) return;
    batchSignRunning = true;
    batchSignDone = false;
    batchSignProgress = 0;
    batchSignTotal = unsignedAssets.length;
    batchSignSuccessCount = 0;
    batchSignFailCount = 0;
    batchSignCancelled = false;
    batchSignCurrentFile = '';
    batchSignEta = null;
    batchSignErrors = [];
    showBatchSignErrors = false;
    _batchSignTimes = [];

    for (const asset of unsignedAssets) {
      if (batchSignCancelled) break;
      batchSignCurrentFile = asset.fileName;
      batchSignProgress++;

      const t0 = performance.now();
      try {
        const updated = await signAsset(asset.assetId, batchSignCreatorName.trim(), batchSignLicense);
        assets = assets.map(a => a.assetId === updated.assetId ? updated : a);
        if (selectedAsset?.assetId === updated.assetId) selectedAsset = updated;
        batchSignSuccessCount++;
      } catch (e) {
        batchSignFailCount++;
        batchSignErrors = [
          ...batchSignErrors,
          { fileName: asset.fileName, error: e instanceof Error ? e.message : 'Signing failed' },
        ];
      }
      _batchSignTimes.push(performance.now() - t0);

      const remaining = batchSignTotal - batchSignProgress;
      if (remaining > 0 && _batchSignTimes.length > 0) {
        const window = _batchSignTimes.slice(-8);
        const avg = window.reduce((a, b) => a + b, 0) / window.length;
        const etaMs = avg * remaining;
        if (etaMs < 60_000) {
          batchSignEta = `~${Math.max(1, Math.round(etaMs / 1000))}s remaining`;
        } else {
          batchSignEta = `~${Math.ceil(etaMs / 60_000)} min remaining`;
        }
      } else {
        batchSignEta = null;
      }
    }

    batchSignRunning = false;
    batchSignDone = true;
    batchSignCurrentFile = '';
    batchSignEta = null;
  }

  function exportBatchSignErrors() {
    if (batchSignErrors.length === 0) return;
    const csv = [
      'File Name,Error',
      ...batchSignErrors.map(e => `"${e.fileName.replace(/"/g, '""')}","${e.error.replace(/"/g, '""')}"`),
    ].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `jura_sign_errors_${new Date().toISOString().slice(0, 10)}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  }

  // ── Format video/audio duration as mm:ss ──────────────────────────
  function formatDurationSecs(seconds: number): string {
    const m = Math.floor(seconds / 60);
    const s = Math.round(seconds % 60);
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  // ── Format bitrate for display ────────────────────────────────────
  function formatBitrate(bps: number): string {
    if (bps >= 1_000_000) return `${(bps / 1_000_000).toFixed(1)} Mbps`;
    if (bps >= 1_000) return `${Math.round(bps / 1_000)} kbps`;
    return `${bps} bps`;
  }

  // ── Content type icon ────────────────────────────────────────────
  function contentTypeIcon(type: string): string {
    switch (type) {
      case 'image':    return 'IMG';
      case 'document': return 'DOC';
      case 'video':    return 'VID';
      case 'audio':    return 'AUD';
      default:         return 'FILE';
    }
  }

  // Sort arrow indicator
  function sortArrow(key: SortKey): string {
    if (sortKey !== key) return '';
    return sortDir === 'asc' ? ' ↑' : ' ↓';
  }

  // ── Tauri environment detection ───────────────────────────────────
  const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // Platform detection for reveal-in-folder label
  const isWindows = typeof navigator !== 'undefined' && navigator.platform.toLowerCase().includes('win');
  const isLinux   = typeof navigator !== 'undefined' && !isWindows && navigator.platform.toLowerCase().includes('linux');
  const revealLabel = isWindows ? 'Show in Explorer' : isLinux ? 'Open folder' : 'Reveal in Finder';

  // ── Path utilities ────────────────────────────────────────────────
  let copyPathFeedback = $state<string | null>(null);
  let copyPathTimer: ReturnType<typeof setTimeout> | null = null;

  async function copyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      copyPathFeedback = path;
      if (copyPathTimer) clearTimeout(copyPathTimer);
      copyPathTimer = setTimeout(() => { copyPathFeedback = null; }, 2000);
    } catch {
      // Clipboard API unavailable — silently fail
    }
  }

  async function openInFinder(filePath: string) {
    if (!inTauri) return;
    try {
      const { Command } = await import('@tauri-apps/plugin-shell');
      if (isWindows) {
        // /select highlights the specific file in Explorer
        await Command.create('explorer', ['/select,', filePath]).execute();
      } else if (isLinux) {
        const dir = filePath.substring(0, filePath.lastIndexOf('/')) || filePath;
        await Command.create('xdg-open', [dir]).execute();
      } else {
        // macOS: -R reveals the file in Finder
        await Command.create('open', ['-R', filePath]).execute();
      }
    } catch (e) {
      console.warn('Reveal in file manager failed:', e);
    }
  }

  // ── Image thumbnail URLs ──────────────────────────────────────────
  // Reactive map of assetId -> safe asset:// URL for CSP-compliant thumbnail display
  let thumbnailUrls = $state<Record<string, string>>({});

  /**
   * Svelte action that resolves a thumbnail URL via Tauri's convertFileSrc
   * when the row is first mounted. Safe no-op in browser mode.
   */
  function loadThumbnailEffect(node: HTMLElement, asset: Asset) {
    function resolve(a: Asset) {
      if (inTauri && a.contentType === 'image' && !thumbnailUrls[a.assetId]) {
        import('@tauri-apps/api/core').then(({ convertFileSrc }) => {
          thumbnailUrls = { ...thumbnailUrls, [a.assetId]: convertFileSrc(a.filePath) };
        }).catch(() => { /* Tauri API unavailable */ });
      }
    }
    resolve(asset);
    return {
      update(newAsset: Asset) { resolve(newAsset); },
    };
  }
</script>

<div class="space-y-6">
  <!-- Beta notice — JTV-116. Rewritten 2026-04-28 to drop the
       "Content Credentials" Adobe trademark, clarify the actual
       conformance status (Validator-submitted / Generator-deferred),
       and name the active signing mode at the point of disclosure.
       Cross-reference: project_c2pa_conformance_gate1.md +
       project_c2pa_validator_feedback.md. -->
  <div
    role="note"
    class="rounded-lg border border-amber/40 bg-amber/10 dark:bg-amber/5 px-4 py-3 text-sm text-amber-dark dark:text-amber-light leading-relaxed"
  >
    <strong class="font-semibold">Beta — pre-conformance.</strong>
    Jura Trace writes structurally valid C2PA provenance manifests.
    Validator-track conformance was submitted on 14 April 2026 and is
    awaiting evaluation; Generator-track conformance for the signing
    path is planned for v1.1 once dual-mode signing has completed
    pilot testing.
    {#if signingMode === 'conformant'}
      You are signing in <strong>Conformant</strong> mode — manifests
      will validate against the C2PA trust list when your imported
      certificate is recognised.
    {:else}
      You are signing in <strong>Local Signing</strong> mode (default).
      Manifests will validate cryptographically but display as
      <code class="font-mono text-[11px]">signingCredential.untrusted</code>
      in third-party validators until you import a trust-list
      certificate from <a href="/settings#signing-mode-heading" class="underline underline-offset-2 hover:no-underline">Settings → Signing Mode</a>.
    {/if}
    Treat signed output as preview only.
  </div>

  <!-- Page header -->
  <div class="flex items-start justify-between gap-4">
    <div>
      <h1 class="text-2xl font-heading text-text-light dark:text-quartz">Protect</h1>
      <p class="text-flint-dark dark:text-flint-light text-sm mt-1">
        Import, catalogue, and safeguard your digital content.
      </p>
    </div>

    <!-- Asset count + CSV export -->
    <div class="flex items-center gap-3 flex-shrink-0 pt-1 flex-wrap justify-end">
      <span class="text-xs text-flint-dark dark:text-flint-light" aria-live="polite" aria-atomic="true">
        {displayedAssets.length} asset{displayedAssets.length !== 1 ? 's' : ''}
        {#if displayedAssets.length !== assets.length}
          <span class="sr-only">(filtered)</span>
        {/if}
      </span>
      {#if displayedAssets.length > 0}
        <button
          class="text-xs px-3 py-2.5 min-h-[44px] inline-flex items-center rounded border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:border-lapis/50 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          onclick={exportCsv}
          title="Download a spreadsheet of all assets in your collection, including protection status, file paths, and metadata."
          aria-label="Export Asset Database — download all visible assets as a spreadsheet"
        >
          Export Asset Database
        </button>
      {/if}
    </div>
  </div>

  <!-- Error banner -->
  {#if error}
    <div
      class="bg-cinnabar/10 border border-cinnabar/30 rounded-lg px-4 py-3 text-sm text-cinnabar-dark dark:text-cinnabar-light"
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
    disabled={importingCount > 0}
    aria-label={importingCount > 0 ? 'Importing files, please wait' : 'Drop files here or click to browse and import files'}
  >
    {#if importingCount > 0}
      <div class="flex flex-col items-center gap-3">
        <div
          class="w-6 h-6 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
          aria-hidden="true"
        ></div>
        <p class="text-sm text-flint-dark dark:text-flint-light" aria-live="polite">
          {importingCount > 1
            ? `Importing ${importingCount} file${importingCount !== 1 ? 's' : ''}...`
            : 'Importing files...'}
        </p>
      </div>
    {:else}
      <div class="flex flex-col items-center gap-2">
        <svg class="w-10 h-10 text-flint-dark dark:text-flint-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
            d="M12 16V4m0 0L8 8m4-4l4 4M4 14v4a2 2 0 002 2h12a2 2 0 002-2v-4" />
        </svg>
        <p class="font-heading text-text-light dark:text-quartz">Drop files or folders here</p>
        <p class="text-xs text-flint-dark dark:text-flint-light mt-1">
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
      <label for="filter-content-type" class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide flex-shrink-0">Type</label>
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
      </select>
    </div>

    <!-- Status filter -->
    <div class="flex items-center gap-2">
      <label for="filter-status" class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide flex-shrink-0">Status</label>
      <select
        id="filter-status"
        bind:value={filterStatus}
        class="px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite text-text-light dark:text-quartz text-sm
               hover:border-lapis/50 transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
      >
        <option value="">All Status</option>
        <option value="signed">Signed</option>
        <option value="watermarked">Watermarked</option>
        <option value="unprotected">Unprotected</option>
      </select>
    </div>

    <!-- Sort dropdown -->
    <div class="flex items-center gap-2">
      <label for="filter-sort" class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide flex-shrink-0">Sort</label>
      <select
        id="filter-sort"
        onchange={(e) => {
          const val = (e.currentTarget as HTMLSelectElement).value;
          if (val === 'name-asc')   { sortKey = 'fileName';  sortDir = 'asc';  }
          else if (val === 'size-desc') { sortKey = 'fileSize';  sortDir = 'desc'; }
          else if (val === 'size-asc')  { sortKey = 'fileSize';  sortDir = 'asc';  }
          else if (val === 'date-asc')  { sortKey = 'createdAt'; sortDir = 'asc';  }
          else                          { sortKey = 'createdAt'; sortDir = 'desc'; }
        }}
        value={sortKey === 'fileName' ? 'name-asc' : sortKey === 'fileSize' ? (sortDir === 'desc' ? 'size-desc' : 'size-asc') : (sortDir === 'asc' ? 'date-asc' : 'date-desc')}
        class="px-3 py-2 rounded border border-border-light dark:border-border-dark bg-white dark:bg-graphite text-text-light dark:text-quartz text-sm
               hover:border-lapis/50 transition-colors
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
        aria-label="Sort assets by"
      >
        <option value="date-desc">Date (newest first)</option>
        <option value="date-asc">Date (oldest first)</option>
        <option value="name-asc">Name (A–Z)</option>
        <option value="size-desc">Size (largest first)</option>
        <option value="size-asc">Size (smallest first)</option>
      </select>
    </div>

    <!-- Search input -->
    <div class="flex items-center gap-2 flex-1 min-w-48">
      <label for="filter-search" class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide flex-shrink-0 sr-only">Search</label>
      <div class="relative flex-1">
        <svg
          class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-flint-dark dark:text-flint-light pointer-events-none"
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
                 placeholder:text-flint-dark dark:text-flint-light
                 hover:border-lapis/50 transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
          aria-label="Search files by name"
        />
      </div>
    </div>

    <!-- Clear filters -->
    {#if filterContentType || filterStatus || searchRaw}
      <button
        class="text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors underline underline-offset-2
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
        onclick={() => { filterContentType = ''; filterStatus = ''; searchRaw = ''; }}
        aria-label="Clear all filters"
      >
        Clear filters
      </button>
    {/if}

    <!-- View toggle: List / Grid -->
    <div
      class="ml-auto flex-shrink-0 flex items-center rounded border border-border-light dark:border-border-dark overflow-hidden text-xs"
      role="group"
      aria-label="Asset view layout"
    >
      <button
        onclick={() => viewLayout = 'list'}
        class="px-2.5 py-1.5 min-h-[36px] flex items-center gap-1.5 transition-colors duration-150
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
               {viewLayout === 'list'
                 ? 'bg-lapis/15 text-lapis dark:text-lapis-light'
                 : 'text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}"
        aria-pressed={viewLayout === 'list'}
        title="List view"
        aria-label="Switch to list view"
      >
        <!-- List icon -->
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M4 6h16M4 10h16M4 14h16M4 18h16" />
        </svg>
        <span class="hidden sm:inline">List</span>
      </button>
      <button
        onclick={() => viewLayout = 'grid'}
        class="px-2.5 py-1.5 min-h-[36px] flex items-center gap-1.5 border-l border-border-light dark:border-border-dark transition-colors duration-150
               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
               {viewLayout === 'grid'
                 ? 'bg-lapis/15 text-lapis dark:text-lapis-light'
                 : 'text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz'}"
        aria-pressed={viewLayout === 'grid'}
        title="Grid view"
        aria-label="Switch to grid view"
      >
        <!-- Grid icon -->
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
            d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z" />
        </svg>
        <span class="hidden sm:inline">Grid</span>
      </button>
    </div>
  </div>

  <!-- Bulk actions — shown only when eligible assets exist.
       Extracted from the filter bar (JTV-122): batch triggers are
       destructive operations and must not live in a role="search" region. -->
  {#if unsignedAssets.length > 0 || unwatermarkedImages.length > 0}
    <section
      aria-labelledby="bulk-actions-heading"
      class="bg-white dark:bg-graphite border border-border-light dark:border-border-dark rounded-xl px-4 py-3 flex flex-wrap items-center gap-3"
    >
      <h2
        id="bulk-actions-heading"
        class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide flex-shrink-0 mr-1"
      >
        Bulk Actions
      </h2>

      {#if unsignedAssets.length > 0}
        <button
          class="text-xs px-3 py-2 min-h-[44px] inline-flex items-center gap-1.5 rounded border border-lapis/50 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian flex-shrink-0"
          onclick={openBatchSign}
          aria-label="Add credentials to all unsigned assets — {unsignedAssets.length} eligible"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
              d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          Add Credentials to All
          <span class="inline-flex items-center justify-center min-w-[18px] h-[18px] rounded-full bg-lapis/20 text-lapis dark:text-lapis-light text-[10px] font-medium px-1" aria-hidden="true">
            {unsignedAssets.length}
          </span>
          <span class="sr-only">({unsignedAssets.length} eligible)</span>
        </button>
      {/if}

      {#if unwatermarkedImages.length > 0}
        <button
          class="text-xs px-3 py-2 min-h-[44px] inline-flex items-center gap-1.5 rounded border border-lapis/50 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian flex-shrink-0"
          onclick={openBatchWatermark}
          aria-label="Watermark all unwatermarked images — {unwatermarkedImages.length} eligible"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
              d="M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.955 11.955 0 010 12c0 6.627 5.373 12 12 12s12-5.373 12-12c0-2.416-.714-4.668-1.952-6.56m-8.048.56A4 4 0 0112 8v4m0 0v4m0-4h4m-4 0H8" />
          </svg>
          Watermark All
          <span class="inline-flex items-center justify-center min-w-[18px] h-[18px] rounded-full bg-lapis/20 text-lapis dark:text-lapis-light text-[10px] font-medium px-1" aria-hidden="true">
            {unwatermarkedImages.length}
          </span>
          <span class="sr-only">({unwatermarkedImages.length} eligible)</span>
        </button>
      {/if}
    </section>
  {/if}

  <!-- Batch C2PA sign panel -->
  {#if showBatchSign}
    <div
      class="bg-white dark:bg-graphite rounded-lg border border-lapis/30 dark:border-lapis/20 shadow-sm overflow-hidden"
      role="region"
      aria-label="Batch content credential signing panel"
      aria-live="polite"
    >
      <!-- Panel header -->
      <div class="px-5 py-4 border-b border-border-light dark:border-graphite-light/50 flex items-center justify-between gap-4">
        <h2 class="text-base text-text-light dark:text-quartz">
          Add Credentials to All
        </h2>
        {#if !batchSignRunning}
          <button
            class="text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite rounded px-2 py-1 min-h-[44px] inline-flex items-center"
            onclick={closeBatchSign}
            aria-label="Close batch signing panel"
          >
            Close
          </button>
        {/if}
      </div>

      <div class="px-5 py-4">

        <!-- Stage: configuration -->
        {#if !batchSignRunning && !batchSignDone}
          <div class="space-y-4">

            <!-- Eligible asset count -->
            <p class="text-sm text-flint-dark dark:text-flint-light">
              <span class="font-medium text-text-light dark:text-quartz">{unsignedAssets.length}</span>
              {unsignedAssets.length === 1 ? 'image' : 'images'} eligible &mdash; not yet signed.
            </p>

            <!-- Creator name input -->
            <div>
              <label
                class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
                for="batch-sign-creator"
              >
                Creator / Rights Holder Name
              </label>
              <input
                id="batch-sign-creator"
                type="text"
                bind:value={batchSignCreatorName}
                placeholder="e.g. Jane Smith / National Archive UK"
                maxlength={128}
                class="w-full mt-1.5 px-3 py-2.5 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian-dark text-text-light dark:text-quartz text-sm
                       placeholder:text-flint-dark dark:text-flint-light
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                aria-describedby="batch-sign-creator-hint"
              />
              <p id="batch-sign-creator-hint" class="mt-1 text-xs text-flint-dark dark:text-flint-light leading-relaxed">
                Written into the manifest as the declared creator. This is a self-attestation — Jura Trace does not verify the name. A third-party validator will display it alongside an "Issuer not trusted" warning until you import a trust-list certificate.
              </p>
            </div>

            <!-- Licence selector -->
            <div>
              <label
                class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
                for="batch-sign-licence"
              >
                Licence
              </label>
              <select
                id="batch-sign-licence"
                bind:value={batchSignLicense}
                class="mt-1.5 w-full rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian-dark text-text-light dark:text-quartz text-sm px-3 py-2.5
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
              >
                <option value="All Rights Reserved">All Rights Reserved</option>
                <option value="CC BY 4.0">CC BY 4.0 — Attribution</option>
                <option value="CC BY-SA 4.0">CC BY-SA 4.0 — Attribution-ShareAlike</option>
                <option value="CC BY-NC 4.0">CC BY-NC 4.0 — Attribution-NonCommercial</option>
                <option value="CC BY-ND 4.0">CC BY-ND 4.0 — Attribution-NoDerivatives</option>
                <option value="CC0 1.0">CC0 1.0 — Public Domain</option>
              </select>
            </div>

            <!-- Active signing-mode badge — JTV-115. Read-only here;
                 mode is set per-install in Settings. Critical for the
                 Amara persona who must sign in Conformant mode for
                 ICC-tribunal-grade evidence submissions. -->
            <div
              class="rounded-md border px-3 py-2 mb-1 text-xs leading-relaxed
                     {signingMode === 'conformant'
                       ? 'bg-malachite/10 border-malachite/30 text-malachite-dark dark:text-malachite-light'
                       : 'bg-lapis/10 border-lapis/30 text-lapis-dark dark:text-lapis-light'}"
              data-testid="batch-signing-mode-badge"
            >
              <span class="font-semibold">Active signing mode:</span>
              {#if signingMode === 'conformant'}
                Conformant — credentials validate against the C2PA trust list.
              {:else}
                Local Signing (default) — credentials will display as
                <code class="font-mono text-[10px]">signingCredential.untrusted</code>
                in external verifiers.
                <a
                  href="/settings#signing-mode-heading"
                  class="underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                >Switch to Conformant Signing →</a>
              {/if}
            </div>

            <!-- Action buttons -->
            <div class="flex gap-3 pt-1">
              <button
                class="px-5 py-2.5 min-h-[44px] inline-flex items-center gap-2 bg-lapis text-white text-sm rounded hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                       disabled:opacity-50 disabled:cursor-not-allowed
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                onclick={handleBatchSign}
                disabled={!batchSignCreatorName.trim() || unsignedAssets.length === 0}
                aria-label="Begin adding credentials to {unsignedAssets.length} {unsignedAssets.length === 1 ? 'image' : 'images'} in {signingMode === 'conformant' ? 'Conformant' : 'Local'} signing mode"
              >
                Begin Signing
              </button>
              <button
                class="px-4 py-2.5 min-h-[44px] inline-flex items-center text-flint-dark dark:text-flint-light text-sm rounded hover:text-text-light dark:hover:text-quartz transition-colors
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                onclick={closeBatchSign}
              >
                Cancel
              </button>
            </div>
          </div>

        <!-- Stage: in progress -->
        {:else if batchSignRunning}
          <div class="space-y-4" aria-live="polite" aria-atomic="false">
            <p class="text-sm text-text-light dark:text-quartz font-medium">
              Signing {batchSignProgress} of {batchSignTotal} {batchSignTotal === 1 ? 'image' : 'images'}...
            </p>

            <!-- Progress bar -->
            <div class="h-2 bg-gray-200 dark:bg-graphite-light rounded-full overflow-hidden" role="progressbar"
                 aria-valuenow={batchSignProgress} aria-valuemin={0} aria-valuemax={batchSignTotal}
                 aria-label="Signing progress">
              <div
                class="h-full bg-lapis rounded-full motion-safe:transition-all motion-safe:duration-300"
                style="width: {batchSignTotal > 0 ? Math.round((batchSignProgress / batchSignTotal) * 100) : 0}%"
              ></div>
            </div>

            {#if batchSignCurrentFile}
              <p class="text-xs text-flint-dark dark:text-flint-light truncate">
                Signing: <span class="text-text-light dark:text-quartz">{batchSignCurrentFile}</span>
              </p>
            {/if}
            {#if batchSignEta}
              <p class="text-xs text-flint-dark dark:text-flint-light">{batchSignEta}</p>
            {/if}

            <button
              class="px-4 py-2 min-h-[44px] text-sm text-cinnabar-dark dark:text-cinnabar-light border border-cinnabar/30 rounded hover:bg-cinnabar/10 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
              onclick={() => { batchSignCancelled = true; }}
            >
              Cancel
            </button>
          </div>

        <!-- Stage: complete -->
        {:else if batchSignDone}
          <div class="space-y-4" role="status" aria-live="polite">
            <div class="flex items-center gap-3">
              <div class="w-8 h-8 rounded-full bg-malachite/15 flex items-center justify-center flex-shrink-0" aria-hidden="true">
                <svg class="w-4 h-4 text-malachite-dark dark:text-malachite-light" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
              </div>
              <div>
                <p class="text-sm font-medium text-text-light dark:text-quartz">
                  {batchSignCancelled ? 'Signing cancelled' : 'Signing complete'}
                </p>
                <p class="text-xs text-flint-dark dark:text-flint-light mt-0.5">
                  {batchSignSuccessCount} signed successfully{batchSignFailCount > 0 ? `, ${batchSignFailCount} failed` : ''}
                </p>
              </div>
            </div>

            {#if batchSignErrors.length > 0}
              <div>
                <button
                  class="text-xs text-amber-dark dark:text-amber-light hover:underline focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-lapis rounded"
                  onclick={() => { showBatchSignErrors = !showBatchSignErrors; }}
                  aria-expanded={showBatchSignErrors}
                >
                  {showBatchSignErrors ? 'Hide' : 'Show'} {batchSignErrors.length} {batchSignErrors.length === 1 ? 'error' : 'errors'}
                </button>
                {#if showBatchSignErrors}
                  <ul class="mt-2 space-y-1" aria-label="Signing errors">
                    {#each batchSignErrors as err}
                      <li class="text-xs text-cinnabar-dark dark:text-cinnabar-light">
                        <span class="font-medium">{err.fileName}</span>: {err.error}
                      </li>
                    {/each}
                  </ul>
                  <button
                    class="mt-2 text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz underline focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-lapis rounded"
                    onclick={exportBatchSignErrors}
                  >
                    Download error log
                  </button>
                {/if}
              </div>
            {/if}

            <button
              class="px-4 py-2.5 min-h-[44px] text-sm text-lapis dark:text-lapis-light border border-lapis/40 rounded hover:bg-lapis/10 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
              onclick={closeBatchSign}
            >
              Close
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Batch watermark panel -->
  {#if showBatchWatermark}
    <div
      class="bg-white dark:bg-graphite rounded-lg border border-lapis/30 dark:border-lapis/20 shadow-sm overflow-hidden"
      role="region"
      aria-label="Batch watermark panel"
      aria-live="polite"
    >
      <!-- Panel header -->
      <div class="px-5 py-4 border-b border-border-light dark:border-graphite-light/50 flex items-center justify-between gap-4">
        <div class="flex items-center gap-1.5">
          <h2 class="text-base text-text-light dark:text-quartz">
            Watermark All Images
          </h2>
          <ContextualHelpLink href="/help/protect#watermarking" label="Learn about invisible watermarking" />
        </div>
        {#if !batchRunning}
          <button
            class="text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite rounded px-2 py-1 min-h-[44px] inline-flex items-center"
            onclick={closeBatchWatermark}
            aria-label="Close batch watermark panel"
          >
            Close
          </button>
        {/if}
      </div>

      <div class="px-5 py-4">

        <!-- Stage: configuration -->
        {#if !batchRunning && !batchDone}
          <div class="space-y-4">

            <!-- Eligible image count -->
            <p class="text-sm text-flint-dark dark:text-flint-light">
              <span class="font-medium text-text-light dark:text-quartz">{unwatermarkedImages.length}</span>
              {unwatermarkedImages.length === 1 ? 'image' : 'images'} eligible &mdash; not yet watermarked.
            </p>

            <!-- Institution name input -->
            <div>
              <label
                class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
                for="batch-watermark-payload"
              >
                Organisation Name or Identifier
              </label>
              <input
                id="batch-watermark-payload"
                type="text"
                bind:value={batchPayload}
                placeholder="e.g. National Archive UK — 2026"
                maxlength={64}
                class="w-full mt-1.5 px-3 py-2.5 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian-dark text-text-light dark:text-quartz text-sm
                       placeholder:text-flint-dark dark:text-flint-light
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                aria-describedby="batch-payload-hint"
              />
              <p id="batch-payload-hint" class="mt-1 text-xs text-flint-dark dark:text-flint-light">
                Encoded invisibly into each file. Maximum 64 characters.
              </p>
            </div>

            <!-- Strength selector -->
            <fieldset>
              <legend class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide mb-2">
                Embedding Strength
              </legend>
              <div class="flex gap-2">
                {#each [
                  { value: 1, label: 'Low' },
                  { value: 2, label: 'Medium' },
                  { value: 3, label: 'High' },
                ] as opt (opt.value)}
                  <button
                    type="button"
                    class="flex-1 min-h-[44px] px-3 py-2 text-sm rounded border transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                           {batchStrength === opt.value
                             ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                             : 'border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light hover:border-lapis/50 hover:text-text-light dark:hover:text-quartz'}"
                    onclick={() => batchStrength = opt.value}
                    aria-pressed={batchStrength === opt.value}
                  >
                    {opt.label}
                  </button>
                {/each}
              </div>

              <!-- Live explainer — updates to match the selected strength.
                   Rewritten 2026-04-28 (JTV-117) to ground claims in
                   PSNR/SSIM and remove overstated robustness language
                   (cropping/screenshot survival was not defensible). -->
              <p
                class="mt-2 text-xs text-flint-dark dark:text-flint-light leading-relaxed"
                aria-live="polite"
              >
                {#if batchStrength === 1}
                  <strong class="text-text-light dark:text-quartz">Low:</strong>
                  PSNR ≈ 48 dB, SSIM &gt; 0.99 — imperceptible on all content.
                  Fragile — does not survive JPEG re-saves below quality 75. Use
                  for archival originals that stay in your own storage.
                {:else if batchStrength === 2}
                  <strong class="text-text-light dark:text-quartz">Medium (recommended):</strong>
                  PSNR ≈ 42 dB, SSIM &gt; 0.98 — imperceptible on ordinary
                  content. Survives JPEG re-saves at quality 75+ and routine
                  platform re-encoding.
                {:else}
                  <strong class="text-text-light dark:text-quartz">High:</strong>
                  PSNR ≈ 36 dB, SSIM &gt; 0.96 — imperceptible on most content,
                  faintly visible on smooth gradients under magnification.
                  Survives JPEG re-saves at quality 50+ and multi-platform
                  forwarding.
                {/if}
              </p>

              <!-- Limits — common to all strengths. JTV-117. -->
              <div class="mt-2 px-3 py-2 rounded border border-amber/30 bg-amber/5 text-[11px] text-amber-dark dark:text-amber-light leading-relaxed">
                <strong class="font-semibold">Limits — common to all strengths.</strong>
                Frequency-domain watermarks do <em>not</em> survive: screenshots,
                geometric cropping greater than ~10% of any edge, AI image-to-image
                regeneration, or adversarial removal. They are an attribution
                signal, not a tamper-proof seal.
              </div>

              <!-- Expandable trade-off explanation -->
              <details class="mt-2 group">
                <summary class="text-xs text-lapis dark:text-lapis-light cursor-pointer inline-flex items-center gap-1 hover:underline underline-offset-2
                                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-1 list-none">
                  <svg class="w-3 h-3 transition-transform group-open:rotate-90"
                       fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                  </svg>
                  How the strengths differ
                </summary>
                <div class="mt-2 p-3 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian/30 text-xs text-flint-dark dark:text-flint-light leading-relaxed space-y-1.5">
                  <p>
                    Strength controls the magnitude of singular-value perturbation
                    in the DCT-of-DWT-LL subband (DWT-DCT-SVD; Cox, Miller &amp;
                    Bloom, <em>Digital Watermarking</em> 2nd ed., 2008). Higher
                    strength shifts more energy into mid-frequency bands, raising
                    survival against re-compression but lowering structural
                    similarity (SSIM).
                  </p>
                  <ul class="list-disc pl-5 space-y-1">
                    <li><strong class="text-text-light dark:text-quartz">Low:</strong> PSNR ≈ 48 dB, SSIM &gt; 0.99 — fragile against JPEG &lt; 75</li>
                    <li><strong class="text-text-light dark:text-quartz">Medium:</strong> PSNR ≈ 42 dB, SSIM &gt; 0.98 — survives JPEG ≥ 75, typical platform re-encoding</li>
                    <li><strong class="text-text-light dark:text-quartz">High:</strong> PSNR ≈ 36 dB, SSIM &gt; 0.96 — survives JPEG ≥ 50, multi-platform forwarding</li>
                  </ul>
                  <p class="text-[11px] italic">
                    Output is always written as a new PNG file alongside the
                    original — JPEG inputs are decoded, watermarked, and saved
                    as PNG.
                  </p>
                </div>
              </details>
            </fieldset>

            <!-- PNG-output advisory before Embed (JTV-118).
                 watermark.rs:152 forces .png output; users dropping
                 JPEG/TIFF/HEIC files would otherwise be surprised
                 only after the operation runs. -->
            <p class="text-[11px] text-flint-dark dark:text-flint-light pt-1" data-testid="batch-watermark-png-notice">
              <span class="font-semibold text-text-light dark:text-quartz">Output:</span>
              every watermarked file is saved as a new PNG alongside the
              original. Originals are not modified.
            </p>

            <!-- Action buttons -->
            <div class="flex gap-3 pt-1">
              <button
                class="px-5 py-2.5 min-h-[44px] inline-flex items-center gap-2 bg-lapis text-white text-sm rounded hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                       disabled:opacity-50 disabled:cursor-not-allowed
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                onclick={handleBatchWatermark}
                disabled={!batchPayload.trim() || unwatermarkedImages.length === 0}
                aria-label="Begin watermarking {unwatermarkedImages.length} {unwatermarkedImages.length === 1 ? 'image' : 'images'} — output saved as PNG alongside original"
              >
                Begin Watermarking
              </button>
              <button
                class="px-4 py-2.5 min-h-[44px] inline-flex items-center text-flint-dark dark:text-flint-light text-sm rounded hover:text-text-light dark:hover:text-quartz transition-colors
                       focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                onclick={closeBatchWatermark}
              >
                Cancel
              </button>
            </div>
          </div>

        <!-- Stage: in progress -->
        {:else if batchRunning}
          <div class="space-y-4" aria-live="polite" aria-atomic="false">
            <!-- Progress status label -->
            <p class="text-sm text-text-light dark:text-quartz font-medium">
              Watermarking {batchProgress} of {batchTotal} {batchTotal === 1 ? 'image' : 'images'}...
            </p>

            <!-- Current file name + ETA -->
            {#if batchCurrentFile}
              <p class="text-xs text-flint-dark dark:text-flint-light truncate" aria-live="polite">
                Current: <span class="text-text-light dark:text-quartz">{batchCurrentFile}</span>
                {#if batchEta}
                  <span class="ml-2 text-flint-dark dark:text-flint-light">{batchEta}</span>
                {/if}
              </p>
            {/if}

            <!-- Progress bar -->
            <div
              class="h-1.5 w-full rounded-full bg-gray-200 dark:bg-graphite-light overflow-hidden"
              role="progressbar"
              aria-valuenow={batchProgress}
              aria-valuemin={0}
              aria-valuemax={batchTotal}
              aria-label="Batch watermarking progress"
            >
              <div
                class="h-full bg-lapis dark:bg-lapis-light rounded-full motion-safe:transition-all motion-safe:duration-200"
                style="width: {batchTotal > 0 ? Math.round((batchProgress / batchTotal) * 100) : 0}%"
              ></div>
            </div>

            <!-- Cancel -->
            <button
              class="px-4 py-2.5 min-h-[44px] inline-flex items-center text-flint-dark dark:text-flint-light text-sm rounded border border-border-light dark:border-border-dark hover:text-text-light dark:hover:text-quartz hover:border-cinnabar/50 transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
              onclick={() => batchCancelled = true}
              aria-label="Cancel batch watermarking after current file completes"
            >
              Stop after Current File
            </button>

            {#if batchCancelled}
              <p class="text-xs text-amber-dark dark:text-amber-light" role="status" aria-live="polite">
                Cancelling after current file...
              </p>
            {/if}
          </div>

        <!-- Stage: complete -->
        {:else if batchDone}
          <div class="space-y-4">
            {#if batchFailCount === 0}
              <div
                class="px-4 py-3 rounded-md bg-malachite/10 border border-malachite/30 text-sm text-malachite-dark dark:text-malachite-light"
                role="status"
                aria-live="polite"
              >
                <span class="font-medium">Complete.</span>
                {batchSuccessCount} {batchSuccessCount === 1 ? 'image' : 'images'} watermarked successfully.
                {#if batchCancelled}
                  <span class="block mt-0.5 text-xs opacity-80">Stopped early by request.</span>
                {/if}
              </div>
            {:else if batchSuccessCount === 0}
              <div
                class="px-4 py-3 rounded-md bg-cinnabar/10 border border-cinnabar/30 text-sm text-cinnabar-dark dark:text-cinnabar-light"
                role="alert"
                aria-live="assertive"
              >
                <span class="font-medium">No images watermarked.</span>
                {batchFailCount} {batchFailCount === 1 ? 'file' : 'files'} failed.
              </div>
            {:else}
              <div
                class="px-4 py-3 rounded-md bg-amber/10 border border-amber/30 text-sm text-amber-dark dark:text-amber-light"
                role="status"
                aria-live="polite"
              >
                <span class="font-medium">Partial success.</span>
                {batchSuccessCount} succeeded, {batchFailCount} failed.
                {#if batchCancelled}
                  <span class="block mt-0.5 text-xs opacity-80">Stopped early by request.</span>
                {/if}
              </div>
            {/if}

            <!-- Per-file error list (dismissible) -->
            {#if batchErrors.length > 0}
              <div class="space-y-2">
                <button
                  class="text-xs text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                  onclick={() => showBatchErrors = !showBatchErrors}
                  aria-expanded={showBatchErrors}
                  aria-controls="batch-error-list"
                >
                  {showBatchErrors ? 'Hide' : 'Show'} {batchErrors.length} failed {batchErrors.length === 1 ? 'file' : 'files'}
                </button>

                {#if showBatchErrors}
                  <ul id="batch-error-list" aria-label="Failed watermark files" class="text-xs space-y-1 max-h-32 overflow-y-auto rounded border border-border-light dark:border-border-dark p-2 bg-white/50 dark:bg-obsidian/50">
                    {#each batchErrors as err}
                      <li class="flex gap-2">
                        <span class="text-text-light dark:text-quartz truncate flex-1">{err.fileName}</span>
                        <span class="text-cinnabar-dark dark:text-cinnabar-light flex-shrink-0">{err.error}</span>
                      </li>
                    {/each}
                  </ul>

                  <button
                    class="text-xs text-lapis dark:text-lapis-light hover:text-lapis-dark dark:hover:text-lapis transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded min-h-[24px]"
                    onclick={exportBatchErrors}
                    aria-label="Export failed watermark files as CSV"
                  >
                    Export errors as CSV
                  </button>
                {/if}
              </div>
            {/if}

            <button
              class="px-5 py-2.5 min-h-[44px] inline-flex items-center bg-lapis text-white text-sm rounded hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
              onclick={closeBatchWatermark}
            >
              Done
            </button>
          </div>
        {/if}

      </div>
    </div>
  {/if}

  <!-- Asset list -->
  {#if displayedAssets.length === 0 && importingCount === 0}
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark p-10 text-center">
      {#if filterContentType || filterStatus || searchRaw}
        <p class="text-flint-dark dark:text-flint-light">No assets match the current filters.</p>
        <button
          class="mt-3 text-sm text-lapis dark:text-lapis-light hover:text-lapis-dark dark:hover:text-lapis transition-colors underline underline-offset-2
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded"
          onclick={() => { filterContentType = ''; filterStatus = ''; searchRaw = ''; }}
        >
          Clear filters
        </button>
      {:else}
        <div class="max-w-md mx-auto">
          <h2
            class="font-heading text-xl font-normal text-text-light dark:text-quartz mb-3"
            style="letter-spacing: -0.01em;"
          >
            Your collection is empty
          </h2>
          <div class="earth-line mb-5" aria-hidden="true"></div>
          <p class="text-sm text-flint-dark dark:text-flint-light dark:text-[#9B9890] leading-relaxed mb-2">
            Import images, documents, or media files to begin protecting
            your content with content credentials and invisible watermarks.
          </p>
          <p class="text-sm text-flint-dark dark:text-flint-light leading-relaxed mb-5">
            Drop files above or click to browse.
          </p>
          <button
            class="inline-flex items-center gap-2 px-6 py-3 min-h-[44px] bg-lapis text-white text-sm rounded hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                   focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
            onclick={handleFilePicker}
            aria-label="Import files — open file browser"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"
                d="M12 16V4m0 0L8 8m4-4l4 4M4 14v4a2 2 0 002 2h12a2 2 0 002-2v-4" />
            </svg>
            Import Files
          </button>
        </div>
      {/if}
    </div>

  {:else if displayedAssets.length > 0}

    <!-- ── Grid view ──────────────────────────────────────────────── -->
    {#if viewLayout === 'grid'}
      <div
        class="grid gap-3"
        style="grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));"
        role="list"
        aria-label="Asset grid"
      >
        {#each displayedAssets as asset (asset.assetId)}
          <div role="listitem" use:loadThumbnailEffect={asset}>
            <button
              class="w-full flex flex-col rounded-lg border overflow-hidden text-left transition-all duration-150 group
                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian
                     {selectedAsset?.assetId === asset.assetId
                       ? 'border-lapis bg-lapis/5 dark:bg-lapis/10'
                       : 'border-border-light dark:border-border-dark bg-white dark:bg-graphite hover:border-lapis/50 hover:shadow-sm'}"
              onclick={() => selectAsset(asset)}
              aria-expanded={selectedAsset?.assetId === asset.assetId}
              aria-label="View details for {asset.fileName}"
            >
              <!-- Thumbnail -->
              <div class="w-full aspect-square bg-gray-100 dark:bg-graphite-light flex items-center justify-center overflow-hidden relative">
                {#if asset.contentType === 'image' && thumbnailUrls[asset.assetId]}
                  <img
                    src={thumbnailUrls[asset.assetId]}
                    alt=""
                    class="w-full h-full object-cover transition-transform duration-200 group-hover:scale-105"
                    loading="lazy"
                    onerror={() => { thumbnailUrls = { ...thumbnailUrls, [asset.assetId]: '' }; }}
                  />
                {:else}
                  <span class="text-xl font-mono text-flint-dark dark:text-flint-light uppercase">
                    {asset.fileName.split('.').pop()?.slice(0, 4) ?? contentTypeIcon(asset.contentType)}
                  </span>
                {/if}
                <!-- Status badge overlay -->
                {#if asset.c2paSigned}
                  <span
                    class="absolute top-1.5 right-1.5 w-5 h-5 rounded-full bg-malachite/90 flex items-center justify-center"
                    title="Signed"
                    aria-label="Content credential signed"
                  >
                    <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M5 13l4 4L19 7" />
                    </svg>
                  </span>
                {/if}
              </div>
              <!-- Filename -->
              <div class="px-2 py-2 w-full min-w-0">
                <p class="text-xs text-text-light dark:text-quartz truncate leading-tight" title={asset.fileName}>
                  {asset.fileName}
                </p>
                <p class="text-[10px] text-flint-dark dark:text-flint-light mt-0.5 truncate">{formatFileSize(asset.fileSize)}</p>
              </div>
            </button>

            <!-- Expanded detail panel for grid selected asset -->
            {#if selectedAsset?.assetId === asset.assetId}
              {@const meta = getMetadata(asset)}
              <div
                class="mt-1 rounded-lg border border-lapis/30 bg-gray-50 dark:bg-obsidian-dark/50 p-3 text-xs space-y-1.5"
                role="region"
                aria-label="Asset details for {asset.fileName}"
              >
                <p class="text-flint-dark dark:text-flint-light truncate" title={asset.filePath}>{asset.filePath}</p>
                {#if asset.width && asset.height}
                  <p class="text-text-light dark:text-quartz">{asset.width} &times; {asset.height} px</p>
                {/if}
                {#if meta?.cameraMake || meta?.cameraModel}
                  <p class="text-flint-dark dark:text-flint-light">{[meta.cameraMake, meta.cameraModel].filter(Boolean).join(' ')}</p>
                {/if}
                <!-- Quick action buttons -->
                <div class="flex flex-wrap gap-1.5 pt-1">
                  {#if !asset.c2paSigned && canSignC2pa(asset)}
                    <button
                      class="text-[10px] px-2 py-1 min-h-[28px] rounded border border-lapis/50 text-lapis dark:text-lapis-light hover:bg-lapis/10 transition-colors
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1"
                      onclick={() => openSigningPanel(asset.assetId, getMetadata(asset)?.artist ?? null)}
                      aria-label="Add content credential to {asset.fileName}"
                    >
                      Sign
                    </button>
                  {/if}
                  {#if !asset.watermarked && canWatermark(asset)}
                    <button
                      class="text-[10px] px-2 py-1 min-h-[28px] rounded border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light hover:border-lapis/50 hover:text-lapis dark:hover:text-lapis dark:text-lapis-light transition-colors
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1"
                      onclick={() => { watermarkAssetId = asset.assetId; }}
                      aria-label="Watermark {asset.fileName}"
                    >
                      Watermark
                    </button>
                  {/if}
                  {#if confirmDeleteId === asset.assetId}
                    <button
                      class="text-[10px] px-2 py-1 min-h-[28px] rounded border border-cinnabar bg-cinnabar text-white hover:bg-cinnabar/80 transition-colors
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-1"
                      onclick={() => handleDelete(asset.assetId)}
                      aria-label="Confirm deletion of {asset.fileName}"
                    >
                      Confirm
                    </button>
                    <button
                      class="text-[10px] px-2 py-1 min-h-[28px] rounded border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light hover:border-lapis/50 transition-colors
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1"
                      onclick={() => { confirmDeleteId = null; }}
                      aria-label="Cancel deletion of {asset.fileName}"
                    >
                      Cancel
                    </button>
                  {:else}
                    <button
                      class="text-[10px] px-2 py-1 min-h-[28px] rounded border border-cinnabar/30 text-cinnabar-dark dark:text-cinnabar-light hover:bg-cinnabar/10 transition-colors
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-1"
                      onclick={() => { confirmDeleteId = asset.assetId; }}
                      aria-label="Delete {asset.fileName}"
                    >
                      Delete
                    </button>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>

    {:else}

    <!-- ── List view (default) ─────────────────────────────────────── -->
    <div class="bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark overflow-hidden">

      <!-- Column headers (sortable) — desktop only -->
      <div
        class="hidden sm:grid grid-cols-[1fr_80px_170px_90px_130px] gap-4 px-4 py-2 border-b border-border-light dark:border-border-dark text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
        role="row"
        aria-label="Asset list column headers"
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
        <span role="columnheader">Type</span>

        <!-- Status (non-sortable label) -->
        <span role="columnheader">Status</span>

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
            <span class="text-xs font-mono px-1.5 py-0.5 rounded bg-gray-100 dark:bg-graphite-light text-text-light dark:text-flint-light flex-shrink-0" aria-hidden="true">{contentTypeIcon(asset.contentType)}</span>
            <p class="text-sm text-text-light dark:text-quartz truncate flex-1">{asset.fileName}</p>
            {#if asset.c2paSigned}
              <span class="text-xs px-1.5 py-0.5 rounded bg-malachite/15 text-malachite-dark dark:text-malachite-light flex-shrink-0">Signed</span>
            {/if}
            {#if asset.watermarked}
              <span class="text-xs px-1.5 py-0.5 rounded bg-lapis/15 text-lapis dark:text-lapis-light flex-shrink-0">Watermarked</span>
            {/if}
          </div>
          <div class="flex items-center gap-3 mt-1.5 text-xs text-flint-dark dark:text-flint-light">
            <span>{asset.mimeType}</span>
            <span>{formatFileSize(asset.fileSize)}</span>
            <span>{new Date(asset.createdAt).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })}</span>
          </div>
        </button>

        <!-- Desktop row -->
        <button
          class="hidden sm:grid w-full grid-cols-[1fr_80px_170px_90px_130px] gap-4 px-4 py-3 border-b border-border-light/50 dark:border-graphite-light/50
                 hover:bg-gray-50 dark:hover:bg-graphite-light/30 transition-colors text-left
                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-lapis
                 {selectedAsset?.assetId === asset.assetId
                   ? 'bg-lapis/10 border-l-2 border-l-lapis'
                   : ''}"
          onclick={() => selectAsset(asset)}
          aria-expanded={selectedAsset?.assetId === asset.assetId}
          aria-label="View details for {asset.fileName}"
        >
          <div class="flex items-center gap-3 min-w-0" use:loadThumbnailEffect={asset}>
            <!-- Thumbnail or type badge -->
            <div
              class="w-10 h-10 rounded overflow-hidden flex-shrink-0 bg-gray-100 dark:bg-graphite-light flex items-center justify-center"
              aria-hidden="true"
            >
              {#if asset.contentType === 'image' && thumbnailUrls[asset.assetId]}
                <img
                  src={thumbnailUrls[asset.assetId]}
                  alt=""
                  class="w-full h-full object-cover"
                  loading="lazy"
                  onerror={() => {
                    thumbnailUrls = { ...thumbnailUrls, [asset.assetId]: '' };
                  }}
                />
              {:else}
                <span class="text-xs font-mono text-text-light dark:text-flint-light uppercase">
                  {asset.fileName.split('.').pop()?.slice(0, 4) ?? contentTypeIcon(asset.contentType)}
                </span>
              {/if}
            </div>
            <div class="min-w-0">
              <p class="text-sm text-text-light dark:text-quartz truncate">{asset.fileName}</p>
              <p class="text-xs text-flint-dark dark:text-flint-light truncate">{asset.mimeType}</p>
            </div>
          </div>

          <span class="text-sm text-flint-dark dark:text-flint-light self-center">
            {CONTENT_TYPE_LABELS[asset.contentType] || asset.contentType}
          </span>

          <!-- Status badges cell -->
          <div class="self-center flex flex-wrap gap-1" aria-label="Protection status">
            {#if asset.c2paSigned}
              <span class="text-[10px] px-1.5 py-0.5 rounded bg-malachite/15 text-malachite-dark dark:text-malachite-light leading-tight">Signed</span>
            {/if}
            {#if asset.watermarked}
              <span class="text-[10px] px-1.5 py-0.5 rounded bg-lapis/15 text-lapis dark:text-lapis-light leading-tight">Watermarked</span>
            {/if}
            {#if !asset.c2paSigned && !asset.watermarked}
              <span class="text-[10px] text-flint-dark dark:text-flint-light italic">Unprotected</span>
            {/if}
          </div>

          <span class="text-sm text-flint-dark dark:text-flint-light self-center">{formatFileSize(asset.fileSize)}</span>

          <span class="text-xs text-flint-dark dark:text-flint-light self-center">
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
              <div class="col-span-2 md:col-span-3">
                <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Path</span>
                <div class="flex items-center gap-2 mt-0.5">
                  <p
                    class="text-text-light dark:text-quartz text-xs truncate max-w-[300px] select-all"
                    title={asset.filePath}
                  >
                    {asset.filePath}
                  </p>
                  <!-- Copy path button -->
                  <button
                    type="button"
                    onclick={() => copyPath(asset.filePath)}
                    class="flex-shrink-0 p-1 rounded text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:bg-gray-100 dark:hover:bg-graphite-light transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    aria-label="Copy file path to clipboard"
                    title="Copy path"
                  >
                    {#if copyPathFeedback === asset.filePath}
                      <!-- Tick — confirmed -->
                      <svg class="w-3.5 h-3.5 text-malachite-dark dark:text-malachite-light" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                      </svg>
                      <span class="sr-only">Path copied</span>
                    {:else}
                      <!-- Clipboard icon -->
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                          d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3" />
                      </svg>
                    {/if}
                  </button>
                  <!-- Open in Finder / Explorer button (Tauri only) -->
                  {#if inTauri}
                    <button
                      type="button"
                      onclick={() => openInFinder(asset.filePath)}
                      class="flex-shrink-0 p-1 rounded text-flint-dark dark:text-flint-light hover:text-text-light dark:hover:text-quartz hover:bg-gray-100 dark:hover:bg-graphite-light transition-colors
                             focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-1 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                      aria-label="{revealLabel} — {asset.fileName}"
                      title={revealLabel}
                    >
                      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                          d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                      </svg>
                    </button>
                  {/if}
                </div>
              </div>

              {#if asset.width && asset.height}
                <div>
                  <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Dimensions</span>
                  <p class="text-text-light dark:text-quartz mt-0.5">{asset.width} &times; {asset.height} px</p>
                </div>
              {/if}

              <!-- EXIF metadata -->
              {#if meta}
                {#if meta.cameraMake || meta.cameraModel}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Camera</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{[meta.cameraMake, meta.cameraModel].filter(Boolean).join(' ')}</p>
                  </div>
                {/if}
                {#if meta.datetimeOriginal}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Date Taken</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.datetimeOriginal}</p>
                  </div>
                {/if}
                {#if meta.software}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Software</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.software}</p>
                  </div>
                {/if}
                {#if meta.iso}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">ISO</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.iso}</p>
                  </div>
                {/if}
                {#if meta.focalLength}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Focal Length</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.focalLength}</p>
                  </div>
                {/if}
                {#if meta.exposureTime}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Exposure</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.exposureTime}</p>
                  </div>
                {/if}
                {#if meta.fNumber}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Aperture</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.fNumber}</p>
                  </div>
                {/if}
                {#if meta.copyright}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Copyright</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.copyright}</p>
                  </div>
                {/if}
                {#if meta.artist}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Artist</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.artist}</p>
                  </div>
                {/if}
                {#if meta.gpsLatitude != null && meta.gpsLongitude != null}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">GPS</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">{meta.gpsLatitude.toFixed(6)}, {meta.gpsLongitude.toFixed(6)}</p>
                  </div>
                {/if}
              {/if}

              <!-- ── Video metadata ──────────────────────────────── -->
              {#if asset.contentType === 'video'}
                {#if loadingMediaMeta && selectedAsset?.assetId === asset.assetId}
                  <div class="col-span-full flex items-center gap-2 mt-1">
                    <span
                      class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                      aria-hidden="true"
                    ></span>
                    <p class="text-xs text-flint-dark dark:text-flint-light">Loading video metadata...</p>
                  </div>
                {:else if videoMetadata && selectedAsset?.assetId === asset.assetId && videoMetadata.success}
                  {#if videoMetadata.duration != null}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Duration</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">{formatDurationSecs(videoMetadata.duration)}</p>
                    </div>
                  {/if}
                  {#if videoMetadata.codec}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Video Codec</span>
                      <p class="text-text-light dark:text-quartz mt-0.5 uppercase">{videoMetadata.codec}</p>
                    </div>
                  {/if}
                  {#if videoMetadata.width && videoMetadata.height}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Resolution</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">{videoMetadata.width} &times; {videoMetadata.height}</p>
                    </div>
                  {/if}
                  {#if videoMetadata.fps != null}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Frame Rate</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">{videoMetadata.fps} fps</p>
                    </div>
                  {/if}
                  {#if videoMetadata.bitrate != null}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Bitrate</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">{formatBitrate(videoMetadata.bitrate)}</p>
                    </div>
                  {/if}
                  <div>
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Audio</span>
                    <p class="text-text-light dark:text-quartz mt-0.5">
                      {videoMetadata.hasAudio
                        ? videoMetadata.audioCodec
                          ? videoMetadata.audioCodec.toUpperCase()
                          : 'Present'
                        : 'None'}
                    </p>
                  </div>
                {/if}

                <!-- Video frame thumbnails -->
                {#if videoFrames && selectedAsset?.assetId === asset.assetId && videoFrames.success && videoFrames.frames.length > 0}
                  <div class="col-span-full mt-2">
                    <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide block mb-2">
                      Frame Samples
                    </span>
                    <div
                      class="grid gap-1.5"
                      style="grid-template-columns: repeat({Math.min(videoFrames.frames.length, 4)}, 1fr);"
                      role="list"
                      aria-label="Representative video frame thumbnails"
                    >
                      {#each videoFrames.frames as frame, i (i)}
                        <div
                          class="rounded overflow-hidden border border-border-light dark:border-border-dark bg-gray-100 dark:bg-obsidian aspect-video"
                          role="listitem"
                        >
                          <img
                            src={blobs.url(frame, 'image/jpeg')}
                            alt="Frame {i + 1} of {videoFrames.frames.length}"
                            class="w-full h-full object-cover"
                          />
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}
              {/if}

              <!-- ── Audio metadata ──────────────────────────────── -->
              {#if asset.contentType === 'audio'}
                {#if loadingMediaMeta && selectedAsset?.assetId === asset.assetId}
                  <div class="col-span-full flex items-center gap-2 mt-1">
                    <span
                      class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                      aria-hidden="true"
                    ></span>
                    <p class="text-xs text-flint-dark dark:text-flint-light">Loading audio metadata...</p>
                  </div>
                {:else if audioMetadata && selectedAsset?.assetId === asset.assetId && audioMetadata.success}
                  {#if audioMetadata.duration != null}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Duration</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">{formatDurationSecs(audioMetadata.duration)}</p>
                    </div>
                  {/if}
                  {#if audioMetadata.codec}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Codec</span>
                      <p class="text-text-light dark:text-quartz mt-0.5 uppercase">{audioMetadata.codec}</p>
                    </div>
                  {/if}
                  {#if audioMetadata.sampleRate != null}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Sample Rate</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">{(audioMetadata.sampleRate / 1000).toFixed(1)} kHz</p>
                    </div>
                  {/if}
                  {#if audioMetadata.channels != null}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Channels</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">
                        {audioMetadata.channels === 1 ? 'Mono' : audioMetadata.channels === 2 ? 'Stereo' : `${audioMetadata.channels} ch`}
                      </p>
                    </div>
                  {/if}
                  {#if audioMetadata.bitrate != null}
                    <div>
                      <span class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide">Bitrate</span>
                      <p class="text-text-light dark:text-quartz mt-0.5">{formatBitrate(audioMetadata.bitrate)}</p>
                    </div>
                  {/if}
                {/if}
              {/if}

              <!-- Status badges -->
              <div class="col-span-full flex gap-2 mt-2 flex-wrap">
                <span
                  class="text-xs px-2 py-0.5 rounded {asset.c2paSigned
                    ? 'bg-malachite/15 text-malachite-dark dark:text-malachite-light'
                    : 'bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light'}"
                >
                  {asset.c2paSigned ? 'Signed' : 'Unsigned'}
                </span>
                <span
                  class="text-xs px-2 py-0.5 rounded {asset.watermarked
                    ? 'bg-malachite/15 text-malachite-dark dark:text-malachite-light'
                    : 'bg-gray-100 dark:bg-graphite-light text-flint-dark dark:text-flint-light'}"
                >
                  {asset.watermarked ? 'Watermarked' : 'No Watermark'}
                </span>
              </div>

              <!-- C2PA signing form -->
              {#if !asset.c2paSigned && canSignC2pa(asset)}
                {#if signingAssetId === asset.assetId}
                  <div class="col-span-full mt-3 p-3 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark">
                    <div class="flex items-center gap-1.5 mb-2">
                      <p class="text-sm text-text-light dark:text-quartz">Add Content Credential</p>
                      <ContextualHelpLink href="/help/protect#c2pa-signing" label="Learn about content credentials" />
                    </div>

                    <p class="text-xs text-malachite-dark dark:text-malachite-light mb-2 leading-relaxed">
                      No data leaves your device. This creates a fully valid content credential embedded in your file.
                    </p>

                    <!-- Active signing-mode badge — JTV-115. -->
                    <div
                      class="rounded-md border px-3 py-2 mb-3 text-xs leading-relaxed
                             {signingMode === 'conformant'
                               ? 'bg-malachite/10 border-malachite/30 text-malachite-dark dark:text-malachite-light'
                               : 'bg-lapis/10 border-lapis/30 text-lapis-dark dark:text-lapis-light'}"
                      data-testid="single-signing-mode-badge"
                    >
                      <span class="font-semibold">Active signing mode:</span>
                      {#if signingMode === 'conformant'}
                        Conformant — credentials validate against the C2PA trust list.
                      {:else}
                        Local Signing (default) — credentials will display as
                        <code class="font-mono text-[10px]">signingCredential.untrusted</code>
                        in external verifiers.
                        <a
                          href="/settings#signing-mode-heading"
                          class="underline underline-offset-2 hover:no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded"
                        >Switch to Conformant Signing →</a>
                      {/if}
                    </div>

                    {#if metadataWarningLoading}
                      <div class="mb-3 flex items-center gap-2 text-xs text-flint-dark dark:text-flint-light">
                        <span
                          class="w-3 h-3 border-2 border-lapis border-t-transparent rounded-full motion-safe:animate-spin"
                          aria-hidden="true"
                        ></span>
                        Checking existing metadata...
                      </div>
                    {/if}

                    {#if metadataWarning?.warningMessage}
                      <div
                        class="mb-3 px-3 py-2 rounded-md bg-amber/10 border border-amber/30 text-xs text-amber-dark dark:text-amber-light"
                        role="alert"
                      >
                        <span class="font-medium">Note:</span>
                        {metadataWarning.warningMessage}
                        {#if metadataWarning.hasExistingC2pa}
                          The new content credential will be added as an additional assertion layer.
                        {/if}
                      </div>
                    {/if}

                    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                      <div>
                        <label
                          class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
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
                          placeholder="e.g. Jane Smith / National Archive UK"
                          aria-describedby="creator-name-hint"
                        />
                        <p id="creator-name-hint" class="mt-1 text-xs text-flint-dark dark:text-flint-light leading-relaxed">
                          Written into the manifest as the declared creator. This is a self-attestation — Jura Trace does not verify the name. A third-party validator will display it alongside an "Issuer not trusted" warning until you import a trust-list certificate.
                        </p>
                      </div>
                      <div>
                        <label
                          class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
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
                        class="px-4 py-2.5 min-h-[44px] inline-flex items-center text-flint-dark dark:text-flint-light text-sm rounded hover:text-text-light dark:hover:text-quartz transition-colors
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
                    class="col-span-full mt-2 px-4 py-2.5 min-h-[44px] inline-flex items-center text-sm border border-lapis/50 text-lapis dark:text-lapis-light rounded
                           hover:bg-lapis/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    onclick={() => openSigningPanel(asset.assetId, meta?.artist ?? null)}
                  >
                    Add Content Credential
                  </button>
                {/if}
              {/if}

              <!-- Metadata preservation statement — shown after a successful C2PA sign in this session -->
              {#if asset.c2paSigned && lastSignedAssetId === asset.assetId}
                <div
                  class="col-span-full rounded-lg border border-malachite/20 bg-malachite/5 px-4 py-3 mt-3"
                  role="status"
                  aria-live="polite"
                >
                  <div class="flex items-start gap-3">
                    <svg
                      class="w-4 h-4 flex-shrink-0 mt-0.5 text-malachite-dark dark:text-malachite-light"
                      aria-hidden="true"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke-width="2"
                      stroke="currentColor"
                    >
                      <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z" />
                    </svg>
                    <div class="text-xs text-malachite-dark dark:text-malachite-light leading-relaxed">
                      <p class="font-semibold mb-1">Content credential added successfully</p>
                      <p>Existing file metadata (EXIF, IPTC, XMP) has been preserved. The content credential was added alongside your existing metadata — no fields were removed or overwritten.</p>
                    </div>
                  </div>
                </div>
              {/if}

              <!-- Watermark embedding -->
              {#if canWatermark(asset)}
                {#if watermarkAssetId === asset.assetId}
                  <!-- Watermark form -->
                  <div
                    class="col-span-full mt-3 p-3 bg-white dark:bg-graphite rounded-lg border border-border-light dark:border-border-dark"
                    role="region"
                    aria-label="Embed watermark"
                  >
                    <div class="flex items-center gap-1.5 mb-3">
                      <p class="font-heading text-sm text-text-light dark:text-quartz">
                        Embed Invisible Watermark
                      </p>
                      <ContextualHelpLink href="/help/protect#watermarking" label="Learn about watermarking" />
                    </div>

                    {#if watermarkResult}
                      <!-- Success / error feedback -->
                      {#if watermarkResult.success}
                        <div
                          class="mb-3 px-3 py-2 rounded-md bg-malachite/10 border border-malachite/30 text-xs text-malachite-dark dark:text-malachite-light"
                          role="status"
                          aria-live="polite"
                        >
                          <span class="font-medium">Watermark embedded.</span>
                          Output saved to:
                          <span class="block mt-0.5 font-mono break-all">{watermarkResult.outputPath}</span>
                        </div>
                      {:else}
                        <div
                          class="mb-3 px-3 py-2 rounded-md bg-cinnabar/10 border border-cinnabar/30 text-xs text-cinnabar-dark dark:text-cinnabar-light"
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
                          class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide"
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
                                 placeholder:text-flint-dark dark:text-flint-light
                                 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                          aria-describedby="watermark-payload-hint-{asset.assetId}"
                        />
                        <p
                          id="watermark-payload-hint-{asset.assetId}"
                          class="mt-1 text-xs text-flint-dark dark:text-flint-light"
                        >
                          This text will be encoded invisibly into the file. Max 64 characters.
                        </p>
                      </div>

                      <!-- Strength selector -->
                      <fieldset>
                        <legend class="text-xs text-flint-dark dark:text-flint-light uppercase tracking-wide mb-2">
                          Embedding Strength
                        </legend>
                        <div
                          class="flex gap-2"
                          role="group"
                          aria-label="Embedding strength"
                        >
                          {#each [
                            { value: 1, label: 'Low' },
                            { value: 2, label: 'Medium' },
                            { value: 3, label: 'High' },
                          ] as opt (opt.value)}
                            <button
                              type="button"
                              class="flex-1 min-h-[44px] px-3 py-2 text-sm rounded border transition-colors
                                     focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite
                                     {watermarkStrength === opt.value
                                       ? 'border-lapis bg-lapis/10 text-lapis dark:text-lapis-light font-medium'
                                       : 'border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light hover:border-lapis/50 hover:text-text-light dark:hover:text-quartz'}"
                              onclick={() => watermarkStrength = opt.value}
                              aria-pressed={watermarkStrength === opt.value}
                            >
                              {opt.label}
                            </button>
                          {/each}
                        </div>

                        <!-- Live explainer (JTV-117 — calibrated PSNR/SSIM). -->
                        <p
                          class="mt-2 text-xs text-flint-dark dark:text-flint-light leading-relaxed"
                          aria-live="polite"
                        >
                          {#if watermarkStrength === 1}
                            <strong class="text-text-light dark:text-quartz">Low:</strong>
                            PSNR ≈ 48 dB, SSIM &gt; 0.99 — imperceptible on all
                            content. Fragile — does not survive JPEG re-saves
                            below quality 75. Use for archival originals that
                            stay in your own storage.
                          {:else if watermarkStrength === 2}
                            <strong class="text-text-light dark:text-quartz">Medium (recommended):</strong>
                            PSNR ≈ 42 dB, SSIM &gt; 0.98 — imperceptible on
                            ordinary content. Survives JPEG re-saves at
                            quality 75+ and routine platform re-encoding.
                          {:else}
                            <strong class="text-text-light dark:text-quartz">High:</strong>
                            PSNR ≈ 36 dB, SSIM &gt; 0.96 — imperceptible on
                            most content, faintly visible on smooth gradients
                            under magnification. Survives JPEG re-saves at
                            quality 50+ and multi-platform forwarding.
                          {/if}
                        </p>

                        <!-- Limits — common to all strengths. -->
                        <div class="mt-2 px-3 py-2 rounded border border-amber/30 bg-amber/5 text-[11px] text-amber-dark dark:text-amber-light leading-relaxed">
                          <strong class="font-semibold">Limits — common to all strengths.</strong>
                          Frequency-domain watermarks do <em>not</em> survive:
                          screenshots, geometric cropping greater than ~10% of
                          any edge, AI image-to-image regeneration, or
                          adversarial removal. They are an attribution signal,
                          not a tamper-proof seal.
                        </div>

                        <!-- Expandable explanation -->
                        <details class="mt-2 group">
                          <summary class="text-xs text-lapis dark:text-lapis-light cursor-pointer inline-flex items-center gap-1 hover:underline underline-offset-2
                                          focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis rounded px-1 list-none">
                            <svg class="w-3 h-3 transition-transform group-open:rotate-90"
                                 fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                            </svg>
                            How the strengths differ
                          </summary>
                          <div class="mt-2 p-3 rounded border border-border-light dark:border-border-dark bg-white dark:bg-obsidian/30 text-xs text-flint-dark dark:text-flint-light leading-relaxed space-y-1.5">
                            <p>
                              Strength controls the magnitude of singular-value
                              perturbation in the DCT-of-DWT-LL subband
                              (DWT-DCT-SVD; Cox, Miller &amp; Bloom,
                              <em>Digital Watermarking</em> 2nd ed., 2008).
                              Higher strength shifts more energy into
                              mid-frequency bands, raising survival against
                              re-compression but lowering structural similarity.
                            </p>
                            <ul class="list-disc pl-5 space-y-1">
                              <li><strong class="text-text-light dark:text-quartz">Low:</strong> PSNR ≈ 48 dB, SSIM &gt; 0.99 — fragile against JPEG &lt; 75</li>
                              <li><strong class="text-text-light dark:text-quartz">Medium:</strong> PSNR ≈ 42 dB, SSIM &gt; 0.98 — survives JPEG ≥ 75</li>
                              <li><strong class="text-text-light dark:text-quartz">High:</strong> PSNR ≈ 36 dB, SSIM &gt; 0.96 — survives JPEG ≥ 50</li>
                            </ul>
                            <p class="text-[11px] italic">
                              Output is always saved as a new PNG file alongside
                              the original — JPEG inputs are decoded, watermarked
                              and re-encoded as PNG.
                            </p>
                          </div>
                        </details>
                      </fieldset>
                    </div>

                    <!-- PNG-output advisory before Embed (JTV-118). -->
                    <p class="text-[11px] text-flint-dark dark:text-flint-light mt-3" data-testid="single-watermark-png-notice">
                      <span class="font-semibold text-text-light dark:text-quartz">Output:</span>
                      this file will be saved as a new PNG alongside the
                      original. The original is not modified.
                    </p>

                    <div class="flex gap-2 mt-3">
                      <button
                        class="px-4 py-2.5 min-h-[44px] inline-flex items-center gap-2 bg-lapis text-white text-sm rounded hover:bg-lapis-dark dark:hover:bg-lapis-light transition-colors
                               disabled:opacity-50 disabled:cursor-not-allowed
                               focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-graphite"
                        onclick={handleWatermark}
                        disabled={watermarking || !watermarkPayload.trim()}
                        aria-busy={watermarking}
                        aria-label="Embed watermark — output saved as PNG alongside original"
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
                        class="px-4 py-2.5 min-h-[44px] inline-flex items-center text-flint-dark dark:text-flint-light text-sm rounded hover:text-text-light dark:hover:text-quartz transition-colors
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
                    class="col-span-full mt-2 px-4 py-2.5 min-h-[44px] inline-flex items-center text-sm border border-lapis/50 text-lapis dark:text-lapis-light rounded
                           hover:bg-lapis/10 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    onclick={() => {
                      watermarkAssetId = asset.assetId;
                      watermarkPayload = '';
                      watermarkStrength = 2;
                      watermarkResult = null;
                      // Close other panels
                      signingAssetId = null;
                    }}
                    aria-label="Embed invisible watermark in {asset.fileName}"
                  >
                    Watermark
                  </button>
                {/if}
              {/if}

              <!-- Delete asset -->
              <div class="col-span-full mt-3 pt-3 border-t border-border-light/50 dark:border-graphite-light/50 flex items-center justify-end gap-2">
                {#if confirmDeleteId === asset.assetId}
                  <span class="text-xs text-flint-dark dark:text-flint-light" id="delete-confirm-label-{asset.assetId}">
                    Permanently delete this asset?
                  </span>
                  <button
                    class="text-sm px-3 py-1.5 min-h-[44px] inline-flex items-center rounded border border-cinnabar/60 bg-cinnabar text-white
                           hover:bg-cinnabar/80 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    onclick={() => handleDelete(asset.assetId)}
                    aria-describedby="delete-confirm-label-{asset.assetId}"
                    aria-label="Confirm deletion of {asset.fileName}"
                  >
                    Confirm Delete
                  </button>
                  <button
                    class="text-sm px-3 py-1.5 min-h-[44px] inline-flex items-center rounded border border-border-light dark:border-border-dark text-flint-dark dark:text-flint-light
                           hover:text-text-light dark:hover:text-quartz hover:border-lapis/50 transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lapis focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian"
                    onclick={() => { confirmDeleteId = null; }}
                    aria-label="Cancel deletion of {asset.fileName}"
                  >
                    Cancel
                  </button>
                {:else}
                  <button
                    class="text-sm text-cinnabar-dark dark:text-cinnabar-light hover:text-cinnabar-dark dark:text-cinnabar-light dark:hover:text-cinnabar-light transition-colors
                           focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cinnabar focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-offset-obsidian rounded px-2 py-1 min-h-[44px] inline-flex items-center"
                    onclick={() => { confirmDeleteId = asset.assetId; }}
                    aria-label="Delete asset {asset.fileName}"
                  >
                    Delete Asset
                  </button>
                {/if}
              </div>

            </div>
          </div>
        {/if}
      {/each}
    </div>
    <!-- End list view -->
    {/if}
    <!-- End viewLayout conditional -->

  {/if}
  <!-- End displayedAssets conditional -->

</div>
