/**
 * Tauri IPC wrapper for Jura Trace commands.
 *
 * When running inside Tauri, calls are dispatched via invoke().
 * When running in a browser (development), returns mock data so the
 * UI can be developed without the Rust backend running.
 */

import type { AppStats, Asset, AudioMetadataResult, AuditLogEntry, Fingerprint, ManifestInfo, MetadataSigningWarning, MonitorOverview, SidecarHealth, SimilarAsset, VerificationResult, VerificationSummary, VerifyMode, VideoFramesResult, VideoMetadataResult, WatermarkEmbedResult, WatermarkExtractResult } from './types';

// Detect if running inside Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Invoke a Tauri command, falling back to mock data in browser.
 */
async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke: tauriInvoke } = await import('@tauri-apps/api/core');
    return tauriInvoke<T>(command, args);
  }
  throw new Error(`Tauri not available: ${command}`);
}

// ── Stats ──────────────────────────────────────────────────────────

export async function getStats(): Promise<AppStats> {
  try {
    return await invoke<AppStats>('get_stats');
  } catch {
    return { totalAssets: 0, totalFingerprints: 0, totalVerifications: 0, c2paSignedCount: 0 };
  }
}

// ── Import ─────────────────────────────────────────────────────────

/**
 * Import files into the PROTECT pipeline.
 * In browser mode, returns mock assets for UI development.
 */
export async function importFiles(paths: string[]): Promise<Asset[]> {
  if (isTauri) {
    return invoke<Asset[]>('import_files', { paths });
  }
  // Browser mock: simulate importing by returning stub assets
  return paths.map((p, i) => ({
    assetId: `mock-${Date.now()}-${i}`,
    filePath: p,
    fileName: p.split('/').pop() || p.split('\\').pop() || p,
    contentType: 'image' as const,
    mimeType: 'image/jpeg',
    fileSize: 1024 * (100 + Math.floor(Math.random() * 900)),
    width: 1920,
    height: 1080,
    c2paSigned: false,
    watermarked: false,
    createdAt: new Date().toISOString(),
  }));
}

/**
 * Open a native file picker dialog and import selected files.
 * Returns the imported assets or an empty array if cancelled.
 */
export async function openFileDialog(): Promise<Asset[]> {
  if (!isTauri) {
    // Browser fallback: use HTML file input
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.multiple = true;
      input.onchange = async () => {
        if (input.files?.length) {
          const paths = Array.from(input.files).map((f) => f.name);
          resolve(await importFiles(paths));
        } else {
          resolve([]);
        }
      };
      input.click();
    });
  }

  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({
    multiple: true,
    title: 'Import Files',
    filters: [
      {
        name: 'All Supported',
        extensions: [
          'jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'heic', 'heif', 'bmp', 'gif', 'svg', 'avif',
          'pdf', 'docx', 'odt', 'epub', 'txt',
          'mp4', 'mov', 'webm', 'avi', 'mkv',
          'wav', 'mp3', 'flac', 'ogg', 'aac',
          'stl', 'obj', 'gltf', 'glb',
        ],
      },
      { name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'heic', 'bmp', 'gif', 'svg', 'avif'] },
      { name: 'Documents', extensions: ['pdf', 'docx', 'odt', 'epub', 'txt'] },
      { name: 'Video', extensions: ['mp4', 'mov', 'webm', 'avi', 'mkv'] },
      { name: 'Audio', extensions: ['wav', 'mp3', 'flac', 'ogg', 'aac'] },
    ],
  });

  if (!selected) return [];
  const paths = Array.isArray(selected) ? selected : [selected];
  return importFiles(paths);
}

// ── Assets ─────────────────────────────────────────────────────────

export async function getAssets(): Promise<Asset[]> {
  try {
    return await invoke<Asset[]>('get_assets');
  } catch {
    return [];
  }
}

/** Get filtered assets with optional content type, signed status, and search. */
export async function getFilteredAssets(
  contentType?: string,
  c2paSigned?: boolean,
  searchQuery?: string,
): Promise<Asset[]> {
  try {
    return await invoke<Asset[]>('get_filtered_assets', {
      contentType: contentType ?? null,
      c2paSigned: c2paSigned ?? null,
      searchQuery: searchQuery ?? null,
    });
  } catch {
    return [];
  }
}

/** Get recent assets for the dashboard. */
export async function getRecentAssets(limit?: number): Promise<Asset[]> {
  try {
    return await invoke<Asset[]>('get_recent_assets', {
      limit: limit ?? null,
    });
  } catch {
    return [];
  }
}

/** Delete an asset by ID. */
export async function deleteAsset(assetId: string): Promise<void> {
  await invoke<void>('delete_asset', { assetId });
}

// ── Verify ─────────────────────────────────────────────────────────

export async function verifyContent(
  source: string,
  sourceType: string,
  mode: VerifyMode = 'deep',
): Promise<VerificationResult> {
  return invoke<VerificationResult>('verify_content', { source, sourceType, mode });
}

/** Run verification pipeline on a file.
 *  mode='fast' runs EXIF + C2PA only (<5 s).
 *  mode='deep' (default) runs the full forensic pipeline (30-60 s).
 */
export async function verifyFile(
  filePath: string,
  mode: VerifyMode = 'deep',
): Promise<VerificationResult> {
  return invoke<VerificationResult>('verify_content', {
    source: filePath,
    sourceType: 'file',
    mode,
  });
}

/** Verify content from a URL. Downloads and analyses the content.
 *  mode='fast' runs EXIF + C2PA only; mode='deep' (default) runs full pipeline.
 */
export async function verifyUrl(url: string, mode: VerifyMode = 'deep'): Promise<VerificationResult> {
  if (isTauri) {
    return invoke<VerificationResult>('verify_url', { url, mode });
  }
  // Browser mock
  return {
    sourceType: 'url',
    contentType: 'image',
    overallTrust: 0.65,
    metadataFlags: ['Mock URL verification'],
    elaScore: 0.2,
    c2paValid: false,
  };
}

/** Check ML sidecar health status. */
export async function checkSidecarHealth(): Promise<SidecarHealth | null> {
  try {
    if (isTauri) {
      return await invoke<SidecarHealth>('check_sidecar_health');
    }
    // Browser mock
    return {
      status: 'mock',
      version: '0.2.0-dev',
      service: 'jura-sidecar',
      capabilities: { ela: true, noise: true, copyMove: true, deepfake: true, jpegGhost: true, npr: true, chromaticAberration: true, rag: false },
      ollama: null,
    };
  } catch {
    return null;
  }
}

// ── Batch Verify ──────────────────────────────────────────────────

/**
 * Open a native file picker that allows multiple selections for batch verification.
 * Returns an array of { filePath, fileName } objects ready for batch queuing.
 */
export async function openBatchFileDialog(): Promise<{ filePath: string; fileName: string }[]> {
  if (!isTauri) {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.multiple = true;
      input.onchange = () => {
        const files = Array.from(input.files ?? []);
        resolve(files.map(f => ({ filePath: f.name, fileName: f.name })));
      };
      input.click();
    });
  }

  const { open } = await import('@tauri-apps/plugin-dialog');
  const selected = await open({
    multiple: true,
    title: 'Select Files to Verify',
    filters: [
      {
        name: 'Supported Files',
        extensions: [
          'jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'avif',
          'heic', 'heif', 'pdf', 'docx', 'mp4', 'mov', 'webm',
        ],
      },
    ],
  });

  if (!selected) return [];
  const paths = Array.isArray(selected) ? selected : [selected];
  return paths.map(p => ({
    filePath: p,
    fileName: p.split('/').pop() ?? p.split('\\').pop() ?? p,
  }));
}

// ── C2PA ──────────────────────────────────────────────────────────

/** Check for existing metadata before C2PA signing. */
export async function checkMetadataBeforeSign(assetId: string): Promise<MetadataSigningWarning> {
  return invoke<MetadataSigningWarning>('check_metadata_before_sign', { assetId });
}


/** Sign an asset with C2PA Content Credentials. */
export async function signAsset(
  assetId: string,
  creatorName: string,
  license?: string
): Promise<Asset> {
  return invoke<Asset>('sign_asset', {
    assetId,
    creatorName,
    license: license || null,
  });
}

/** Read a C2PA manifest from a file. Returns null if no manifest found. */
export async function readManifest(filePath: string): Promise<ManifestInfo | null> {
  return invoke<ManifestInfo | null>('read_manifest', { filePath });
}

// ── Fingerprints ──────────────────────────────────────────────────

/** Get perceptual fingerprints for an asset. */
export async function getFingerprints(assetId: string): Promise<Fingerprint[]> {
  try {
    return await invoke<Fingerprint[]>('get_fingerprints', { assetId });
  } catch {
    return [];
  }
}

/** Find assets with similar perceptual hashes. */
export async function findSimilar(
  assetId: string,
  threshold?: number
): Promise<SimilarAsset[]> {
  try {
    return await invoke<SimilarAsset[]>('find_similar', {
      assetId,
      threshold: threshold ?? null,
    });
  } catch {
    return [];
  }
}

// ── Version ────────────────────────────────────────────────────────

export async function getVersion(): Promise<string> {
  try {
    return await invoke<string>('get_version');
  } catch {
    return '0.1.0-dev';
  }
}

// ── False Positive Reporting ────────────────────────────────────────

/**
 * Submit a false positive report for a verification result.
 * Returns the generated report ID.
 */
export async function markFalsePositive(
  reasonCode: string,
  reasonNote?: string,
  mimeType?: string,
  deepfakeScore?: number,
  deepfakeVerdict?: string,
  signalScoresJson?: string,
): Promise<string> {
  try {
    return await invoke<string>('mark_false_positive', {
      reasonCode,
      reasonNote: reasonNote ?? null,
      mimeType: mimeType ?? null,
      deepfakeScore: deepfakeScore ?? null,
      deepfakeVerdict: deepfakeVerdict ?? null,
      signalScoresJson: signalScoresJson ?? null,
    });
  } catch {
    // Browser mock: return a stub report ID
    return `mock-fp-${Date.now()}`;
  }
}

/**
 * Retrieve the total number of false positive reports submitted.
 */
export async function getFalsePositiveStats(): Promise<number> {
  try {
    return await invoke<number>('get_false_positive_stats');
  } catch {
    return 0;
  }
}

// ── Monitor ────────────────────────────────────────────────────────

/**
 * Retrieve the Monitor overview: protection summary, trust distribution,
 * recent audit activity, and daily activity counts.
 */
export async function getMonitorOverview(): Promise<MonitorOverview> {
  try {
    return await invoke<MonitorOverview>('get_monitor_overview');
  } catch {
    return {
      protection: {
        totalAssets: 0,
        c2paSigned: 0,
        watermarked: 0,
        fingerprinted: 0,
        byContentType: {},
      },
      trust: {
        total: 0,
        highCount: 0,
        mediumCount: 0,
        lowCount: 0,
        averageTrust: 0,
      },
      recentActivity: [],
      activityDays: [],
    };
  }
}

/**
 * Retrieve audit log entries.
 * @param limit  Maximum number of entries to return (default 50).
 * @param actionFilter  If provided, only return entries with this action value.
 */
export async function getAuditLog(
  limit?: number,
  actionFilter?: string,
): Promise<AuditLogEntry[]> {
  try {
    return await invoke<AuditLogEntry[]>('get_audit_log', {
      limit: limit ?? null,
      actionFilter: actionFilter ?? null,
    });
  } catch {
    return [];
  }
}

/**
 * Retrieve paginated verification history summaries.
 */
export async function getVerificationHistory(
  limit?: number,
  offset?: number,
): Promise<VerificationSummary[]> {
  try {
    return await invoke<VerificationSummary[]>('get_verification_history', {
      limit: limit ?? null,
      offset: offset ?? null,
    });
  } catch {
    return [];
  }
}

// ── Watermarking ────────────────────────────────────────────────────

/**
 * Embed an invisible watermark into an image asset.
 * @param assetId  The asset to watermark.
 * @param payload  Human-readable payload string (e.g. institution name + date).
 * @param strength Embedding strength: 1 = low, 2 = medium (default), 3 = high.
 */
export async function embedWatermark(
  assetId: string,
  payload: string,
  strength: number = 2,
): Promise<WatermarkEmbedResult> {
  if (isTauri) {
    // payload_hex: backend expects a hex-encoded byte string
    const payloadHex = Array.from(new TextEncoder().encode(payload))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
    return invoke<WatermarkEmbedResult>('embed_watermark_asset', {
      assetId,
      payloadHex,
      strength,
    });
  }
  // Browser mock
  return {
    outputPath: `/mock/output/${assetId}_watermarked.jpg`,
    payloadHex: Array.from(new TextEncoder().encode(payload))
      .map(b => b.toString(16).padStart(2, '0'))
      .join(''),
    success: true,
    message: 'Watermark embedded (mock)',
  };
}

/**
 * Extract and check for an invisible watermark in an image asset.
 * @param assetId  The asset to inspect.
 */
export async function extractWatermark(assetId: string): Promise<WatermarkExtractResult> {
  if (isTauri) {
    return invoke<WatermarkExtractResult>('extract_watermark_from_path', { assetId });
  }
  // Browser mock
  return {
    extractedPayload: null,
    extractedHex: null,
    hasWatermark: false,
    confidence: 0,
    success: true,
    message: 'No watermark detected (mock)',
  };
}

// ── Video / Audio Metadata ──────────────────────────────────────────

/**
 * Retrieve technical metadata from a video asset.
 * @param assetId  The asset to inspect.
 */
export async function getVideoMetadata(assetId: string): Promise<VideoMetadataResult> {
  if (isTauri) {
    try {
      return await invoke<VideoMetadataResult>('get_video_metadata', { assetId });
    } catch {
      // Command not yet registered — return empty result
    }
  }
  // Browser mock
  return {
    duration: 142.5,
    codec: 'h264',
    width: 1920,
    height: 1080,
    fps: 25,
    hasAudio: true,
    audioCodec: 'aac',
    bitrate: 4500000,
    fileSize: undefined,
    success: true,
    message: 'Video metadata (mock)',
  };
}

/**
 * Retrieve technical metadata from an audio asset.
 * @param assetId  The asset to inspect.
 */
export async function getAudioMetadata(assetId: string): Promise<AudioMetadataResult> {
  if (isTauri) {
    try {
      return await invoke<AudioMetadataResult>('get_audio_metadata', { assetId });
    } catch {
      // Command not yet registered — return empty result
    }
  }
  // Browser mock
  return {
    duration: 210.3,
    codec: 'flac',
    sampleRate: 44100,
    channels: 2,
    bitrate: 1411200,
    fileSize: undefined,
    success: true,
    message: 'Audio metadata (mock)',
  };
}

/**
 * Extract representative frame thumbnails from a video asset.
 * Returns a small set of base64-encoded JPEG thumbnails evenly spaced
 * across the video duration.
 * @param assetId  The video asset to sample.
 * @param count    Number of frames to extract (default 4).
 */
export async function getVideoFrames(
  assetId: string,
  count: number = 4,
): Promise<VideoFramesResult> {
  if (isTauri) {
    try {
      return await invoke<VideoFramesResult>('get_video_frames', { assetId, count });
    } catch {
      // Command not yet registered — return empty result
    }
  }
  // Browser mock — return an empty result so the UI degrades gracefully
  return {
    frames: [],
    count: 0,
    duration: undefined,
    success: false,
    message: 'Video frame extraction not available (mock)',
  };
}
