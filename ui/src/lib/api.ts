/**
 * Tauri IPC wrapper for Jura Trace commands.
 *
 * When running inside Tauri, calls are dispatched via invoke().
 * When running in a browser (development), returns mock data so the
 * UI can be developed without the Rust backend running.
 */

import type { Annotation, AppErrorResponse, AppStats, Asset, AudioMetadataResult, AuditLogEntry, ConformantCertificateInfo, Fingerprint, LicenceTier, ManifestInfo, MetadataSigningWarning, MonitorEvent, MonitorOverview, MonitorUrl, RoiAnalysisResult, SidecarHealth, SigningMode, SimilarAsset, SolarPosition, TimeEstimate, VerificationResult, VerificationSummary, VerifyMode, VideoDeepfakeResult, VideoFramesResult, VideoMetadataResult, WatermarkEmbedResult, WatermarkExtractResult } from './types';

// Detect if running inside Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Parse an error thrown by a Tauri `invoke()` call into a structured object.
 *
 * Tauri commands that use `AppError` serialise their errors as a JSON object:
 *   `{ "code": "Sidecar", "message": "Analysis service unavailable: ..." }`
 *
 * Commands still using `map_err(|e| e.to_string())` return a plain string.
 * This helper normalises both into `{ code, message }` so callers can branch
 * on `code` without string-sniffing.
 */
export function parseAppError(err: unknown): { code: AppErrorResponse['code'] | null; message: string } {
  // Structured AppError from a migrated command — { code, message }
  if (
    err !== null &&
    typeof err === 'object' &&
    'code' in err &&
    'message' in err &&
    typeof (err as Record<string, unknown>).code === 'string' &&
    typeof (err as Record<string, unknown>).message === 'string'
  ) {
    return {
      code: (err as AppErrorResponse).code,
      message: (err as AppErrorResponse).message,
    };
  }

  // Plain string from a command using map_err(|e| e.to_string())
  if (typeof err === 'string') {
    return { code: null, message: err };
  }

  // Error object (e.g. thrown from the browser mock invoke())
  if (err instanceof Error) {
    return { code: null, message: err.message };
  }

  // Fallback for unknown shapes
  return { code: null, message: String(err) };
}

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
    fingerprinted: false,
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

/** Get filtered assets with optional content type, signed status, fingerprint status, and search. */
export async function getFilteredAssets(
  contentType?: string,
  c2paSigned?: boolean,
  searchQuery?: string,
  fingerprinted?: boolean,
): Promise<Asset[]> {
  try {
    return await invoke<Asset[]>('get_filtered_assets', {
      contentType: contentType ?? null,
      c2paSigned: c2paSigned ?? null,
      fingerprinted: fingerprinted ?? null,
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
      capabilities: { ela: true, noise: true, copyMove: true, deepfake: true, jpegGhost: true, npr: true, rag: false, watermark: true, clipDetect: false, videoMetadata: false, audioMetadata: false, videoFrames: false, videoDeepfake: false, transcription: false },
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


/** Sign an asset with C2PA provenance. */
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

// ── Monitor URL Watchlist ────────────────────────────────────────────

/**
 * Register a URL for periodic monitoring.
 * @param url  The absolute URL to monitor.
 * @param label  Optional human-readable label.
 * @param frequency  Check cadence: "hourly" | "daily" (default) | "weekly".
 */
export async function addMonitorUrl(
  url: string,
  label?: string,
  frequency?: string,
): Promise<MonitorUrl> {
  return invoke<MonitorUrl>('add_monitor_url', {
    url,
    label: label ?? null,
    assetId: null,
    frequency: frequency ?? null,
  });
}

/**
 * Remove a monitored URL and all its associated events.
 */
export async function removeMonitorUrl(urlId: string): Promise<void> {
  return invoke<void>('remove_monitor_url', { urlId });
}

/**
 * List all monitored URLs.
 * @param enabledOnly  If true, only return enabled (active) URLs.
 */
export async function listMonitorUrls(enabledOnly?: boolean): Promise<MonitorUrl[]> {
  try {
    return await invoke<MonitorUrl[]>('list_monitor_urls', {
      enabledOnly: enabledOnly ?? null,
    });
  } catch {
    return [];
  }
}

/**
 * Return the most recent check events for a given monitored URL.
 * @param urlId  The URL record ID.
 * @param limit  Maximum number of events to return (default 50).
 */
export async function getMonitorEvents(urlId: string, limit?: number): Promise<MonitorEvent[]> {
  try {
    return await invoke<MonitorEvent[]>('get_monitor_events', {
      urlId,
      limit: limit ?? null,
    });
  } catch {
    return [];
  }
}

/**
 * Update the case management status and optional notes on a monitor event.
 * @param eventId  The event record ID.
 * @param status   One of: "new" | "investigating" | "resolved" | "escalated" | "dismissed".
 * @param notes    Optional free-text annotation.
 */
export async function updateMonitorCaseStatus(
  eventId: string,
  status: string,
  notes?: string,
): Promise<void> {
  return invoke<void>('update_monitor_case_status', {
    eventId,
    status,
    notes: notes ?? null,
  });
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

// ── Database Path Configuration ────────────────────────────────────

/**
 * Return the current database file path.
 * Returns an empty string if the command is unavailable (browser context).
 */
export async function getDbPath(): Promise<string> {
  try {
    return await invoke<string>('get_db_path');
  } catch {
    return '';
  }
}

/**
 * Move the database to a new location.
 *
 * The Rust backend copies the existing database to the new path atomically
 * (copy to temp, verify with SQLite, rename), then persists the new path in
 * config.json. If any step fails, the original path is unchanged.
 *
 * @param newPath  Absolute path to the new database file location.
 * @returns        The resolved new path on success, or throws with an error message.
 */
export async function setDbPath(newPath: string): Promise<string> {
  return invoke<string>('set_db_path', { newPath });
}

// ── Setup Wizard Flag ───────────────────────────────────────────────

/**
 * Return whether the first-run setup wizard should be suppressed.
 *
 * When an IT administrator sets `"skip_setup_wizard": true` in the
 * application's `config.json`, this returns `true` and the wizard is not
 * shown, regardless of localStorage state.
 *
 * Defaults to `false` when the field is absent (backward-compatible with
 * existing installations that have no such key in config.json).
 */
export async function getSkipWizard(): Promise<boolean> {
  try {
    return await invoke<boolean>('get_skip_wizard');
  } catch {
    // In browser mode or if the command is unavailable, never suppress the wizard.
    return false;
  }
}

// ── Licence Tier ────────────────────────────────────────────────────

/**
 * Return the current licence tier from the Rust backend.
 *
 * The tier is loaded from config.json at startup and defaults to 'community'.
 * This is a pilot-phase helper — it does not enforce feature gates.
 */
export async function getLicenceTier(): Promise<LicenceTier> {
  try {
    return await invoke<LicenceTier>('get_licence_tier');
  } catch {
    return 'community';
  }
}

/**
 * Persist a licence tier change to config.json.
 *
 * For pilot/admin use only. The change takes effect immediately in the
 * running session and survives application restarts.
 *
 * @param tier  The tier to activate.
 */
export async function setLicenceTier(tier: LicenceTier): Promise<void> {
  return invoke<void>('set_licence_tier', { tier });
}

/**
 * Get the user's AI image description preference.
 *
 * Returns:
 *   - `true`  — explicitly enabled
 *   - `false` — explicitly disabled
 *   - `null`  — not yet decided (treated as disabled by the verify pipeline
 *               until the user makes a choice via Settings)
 */
export async function getAiDescriptionEnabled(): Promise<boolean | null> {
  try {
    return await invoke<boolean | null>('get_ai_description_enabled');
  } catch {
    return null;
  }
}

/**
 * Persist the user's AI image description preference.
 *
 * AI descriptions via Ollama LLaVA add 5–30 seconds to each image verify,
 * so this is gated behind an explicit user opt-in.
 *
 * @param enabled  `true` to enable, `false` to disable, `null` to clear the
 *                 preference back to "not set".
 */
export async function setAiDescriptionEnabled(enabled: boolean | null): Promise<void> {
  return invoke<void>('set_ai_description_enabled', { enabled });
}

// ── API Key Management ─────────────────────────────────────────────

export interface ApiKeyInfo {
  keyId: string;
  name: string;
  rateLimit: number;
  revoked: boolean;
  createdAt: string;
}

export interface CreateKeyResult {
  keyId: string;
  key: string;
  name: string;
  rateLimit: number;
}

/** Create a new API key for the local REST API (port 8300). */
export async function createApiKey(name: string, rateLimit?: number): Promise<CreateKeyResult> {
  return invoke<CreateKeyResult>('create_api_key', { name, rateLimit });
}

/** List all API keys (active and revoked). */
export async function listApiKeys(): Promise<ApiKeyInfo[]> {
  try {
    return await invoke<ApiKeyInfo[]>('list_api_keys');
  } catch {
    return [];
  }
}

/** Revoke an API key by ID. */
export async function revokeApiKey(keyId: string): Promise<void> {
  return invoke<void>('revoke_api_key', { keyId });
}

// ── Text Extraction ─────────────────────────────────────────────────

/**
 * Extract and transcribe all visible text from an image using LLaVA via Ollama.
 *
 * Suitable for screenshots, memes, social media posts, and scanned documents.
 * Only available when Ollama is running and llava:7b is pulled.
 *
 * Returns the transcribed text string, or null if Ollama is unavailable or
 * no text could be extracted.
 *
 * @param filePath  Absolute path to the image file to read text from.
 */
export async function extractTextFromImage(filePath: string): Promise<string | null> {
  try {
    return await invoke<string>('extract_text_from_image', { filePath });
  } catch {
    return null;
  }
}

// ── Video Deepfake Analysis ─────────────────────────────────────────

/**
 * Analyse a video file for AI-generated or manipulated frames.
 * @param filePath  Path to the video file.
 * @param mode      Analysis mode: 'standard' (6 frames), 'deep' (20), 'archival' (40).
 */
export async function analyseVideoDeepfake(
  filePath: string,
  mode: string = 'standard',
): Promise<VideoDeepfakeResult> {
  if (isTauri) {
    try {
      return await invoke<VideoDeepfakeResult>('analyse_video_deepfake', { filePath, mode });
    } catch {
      // Command not yet registered or sidecar unavailable
    }
  }
  // Browser mock
  return {
    frameResults: [
      { frameIndex: 0, timestamp: 1.5, score: 0.22, suspicious: false, verdictLevel: 'authentic', signals: [] },
      { frameIndex: 1, timestamp: 3.0, score: 0.38, suspicious: false, verdictLevel: 'inconclusive', signals: [] },
      { frameIndex: 2, timestamp: 4.5, score: 0.65, suspicious: true, verdictLevel: 'synthetic', signals: [] },
      { frameIndex: 3, timestamp: 6.0, score: 0.31, suspicious: false, verdictLevel: 'authentic', signals: [] },
      { frameIndex: 4, timestamp: 7.5, score: 0.28, suspicious: false, verdictLevel: 'authentic', signals: [] },
      { frameIndex: 5, timestamp: 9.0, score: 0.19, suspicious: false, verdictLevel: 'authentic', signals: [] },
    ],
    aggregateScore: 0.37,
    aggregateVerdict: 'inconclusive',
    aggregateConfidence: 'medium',
    framesAnalysed: 6,
    framesRequested: 6,
    temporalAvailable: true,
    temporalNoiseDrift: 0.12,
    temporalSpectralDrift: 0.08,
    temporalLbpDrift: 0.15,
    mode: 'standard',
    duration: 10.5,
    success: true,
    message: 'Analysed 6 frames in standard mode. 1 frame flagged as suspicious.',
  };
}

// ── Solar Position ──────────────────────────────────────────────────

/**
 * Calculate the solar position (azimuth and elevation) at a given GPS
 * coordinate and UTC datetime using the NOAA solar position algorithm.
 *
 * Useful for cross-referencing image shadow direction with the expected
 * sun position at the reported capture location and time.
 *
 * @param lat      GPS latitude in decimal degrees.
 * @param lon      GPS longitude in decimal degrees.
 * @param year     Year (e.g. 2024).
 * @param month    Month (1–12).
 * @param day      Day of month (1–31).
 * @param hourUtc  Hour of day in UTC (0–23).
 */
export async function calculateSunPosition(
  lat: number,
  lon: number,
  year: number,
  month: number,
  day: number,
  hourUtc: number,
): Promise<SolarPosition> {
  return invoke<SolarPosition>('calculate_sun_position', {
    latitude: lat,
    longitude: lon,
    year,
    month,
    day,
    hourUtc,
  });
}

// ── Sprint 24 investigation APIs ────────────────────────────────────

/**
 * Estimate the time of day from a measured shadow azimuth angle.
 *
 * Given a GPS coordinate, date, and the azimuth of a shadow measured from
 * an image, returns up to two candidate UTC times at which the sun would
 * have cast a shadow in that direction.
 *
 * @param lat            GPS latitude in decimal degrees.
 * @param lon            GPS longitude in decimal degrees.
 * @param year           Year (e.g. 2024).
 * @param month          Month (1–12).
 * @param day            Day of month (1–31).
 * @param shadowAzimuth  Measured shadow direction in degrees (0–360, clockwise from north).
 */
export async function estimateShadowTime(
  lat: number,
  lon: number,
  year: number,
  month: number,
  day: number,
  shadowAzimuth: number,
): Promise<TimeEstimate[]> {
  return invoke<TimeEstimate[]>('estimate_shadow_time', {
    latitude: lat,
    longitude: lon,
    year,
    month,
    day,
    shadowAzimuth,
  });
}

/**
 * Analyse a user-selected region of interest (ROI) within a local image file.
 *
 * Posts the image and bounding-box coordinates to the sidecar's
 * `/forensics/roi-analysis` endpoint. Returns noise statistics, ELA mean,
 * frequency energy, and texture complexity for the selected region.
 *
 * All coordinate values are in natural image pixels (not CSS pixels).
 *
 * @param filePath  Absolute path to the image file.
 * @param x         Left edge of the ROI in natural image pixels.
 * @param y         Top edge of the ROI in natural image pixels.
 * @param width     Width of the ROI in natural image pixels.
 * @param height    Height of the ROI in natural image pixels.
 */
export async function analyseRoi(
  filePath: string,
  x: number,
  y: number,
  width: number,
  height: number,
): Promise<RoiAnalysisResult> {
  if (isTauri) {
    try {
      return await invoke<RoiAnalysisResult>('analyse_roi', { filePath, x, y, width, height });
    } catch {
      // Command not yet registered — fall through to browser mock
    }
  }
  // Browser mock
  return {
    noiseStd: 4.2,
    noiseMean: 1.1,
    elaMean: 0.14,
    frequencyEnergy: 0.38,
    textureComplexity: 0.55,
    noiseResidualBase64: '',
    roi: { x, y, width, height },
  };
}

// ── Annotations ─────────────────────────────────────────────────────

/**
 * Persist an annotation associated with a verification result or asset.
 *
 * The geometry is stored as a JSON string (`dataJson`) so the Rust backend
 * can record it without parsing the shape. All coordinates should be in
 * natural image pixels so annotations scale correctly on different displays.
 *
 * @param annotationType  Shape type: 'arrow' | 'circle' | 'rectangle' | 'text'.
 * @param dataJson        Serialised `AnnotationData` geometry.
 * @param assetId         Optional asset the annotation belongs to.
 * @param verificationId  Optional verification the annotation belongs to.
 */
export async function saveAnnotation(
  annotationType: string,
  dataJson: string,
  assetId?: string,
  verificationId?: string,
): Promise<Annotation> {
  if (isTauri) {
    return invoke<Annotation>('save_annotation', {
      annotationType,
      dataJson,
      assetId: assetId ?? null,
      verificationId: verificationId ?? null,
    });
  }
  // Browser mock — return a stub with a generated ID so the UI can render immediately
  return {
    annotationId: crypto.randomUUID(),
    annotationType: annotationType as Annotation['annotationType'],
    dataJson,
    assetId,
    verificationId,
    createdAt: new Date().toISOString(),
  };
}

/**
 * Retrieve all annotations for a given asset or verification result.
 *
 * Returns an empty array if neither `assetId` nor `verificationId` is
 * provided, or when running in browser mode.
 *
 * @param assetId         Filter by asset ID.
 * @param verificationId  Filter by verification ID.
 */
export async function getAnnotations(
  assetId?: string,
  verificationId?: string,
): Promise<Annotation[]> {
  try {
    if (isTauri) {
      return await invoke<Annotation[]>('get_annotations', {
        assetId: assetId ?? null,
        verificationId: verificationId ?? null,
      });
    }
    // Browser mock — no persisted annotations available
    return [];
  } catch {
    return [];
  }
}

/**
 * Delete a single annotation by its ID.
 *
 * @param annotationId  The ID of the annotation to remove.
 */
export async function deleteAnnotationApi(annotationId: string): Promise<void> {
  if (isTauri) {
    return invoke<void>('delete_annotation', { annotationId });
  }
  // Browser mock — no-op
}

// ── BYOC Signing Mode ──────────────────────────────────────────────────────

/**
 * Returns the active C2PA signing mode for this installation.
 * Defaults to 'bedrock' on first run (before signing_config.json is written).
 */
export async function getSigningMode(): Promise<SigningMode> {
  if (isTauri) {
    return invoke<SigningMode>('get_signing_mode');
  }
  return 'bedrock';
}

/**
 * Switches the active C2PA signing mode.
 * Throws if caller tries to set 'conformant' while no cert is imported.
 */
export async function setSigningMode(mode: SigningMode): Promise<void> {
  if (isTauri) {
    return invoke<void>('set_signing_mode', { mode });
  }
  // Browser mock — no-op
}

/**
 * Returns metadata for the currently imported conformant certificate,
 * or null if no certificate has been imported.
 */
export async function getConformantCertInfo(): Promise<ConformantCertificateInfo | null> {
  if (isTauri) {
    return invoke<ConformantCertificateInfo | null>('get_conformant_cert_info');
  }
  return null;
}

/**
 * Imports a user-provided PEM certificate chain and private key.
 * Validates against the C2PA conformance profile (AKI, SKI, Key Usage, EKU),
 * verifies the key matches the cert, and persists both to the app data directory.
 *
 * @param certPath  Absolute path to the PEM certificate chain file.
 * @param keyPath   Absolute path to the PEM private key file.
 * @returns Metadata for the imported certificate.
 * @throws AppError with a descriptive message if validation fails.
 */
export async function importConformantCertificate(
  certPath: string,
  keyPath: string,
): Promise<ConformantCertificateInfo> {
  if (isTauri) {
    return invoke<ConformantCertificateInfo>('import_conformant_certificate', { certPath, keyPath });
  }
  throw new Error('Certificate import is only available in the desktop application.');
}

/**
 * Deletes the stored conformant certificate files and reverts the active
 * signing mode to Bedrock. Idempotent — safe to call when no cert is imported.
 */
export async function clearConformantCert(): Promise<void> {
  if (isTauri) {
    return invoke<void>('clear_conformant_cert');
  }
  // Browser mock — no-op
}
