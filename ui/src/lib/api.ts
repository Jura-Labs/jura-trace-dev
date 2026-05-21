// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Tauri IPC wrapper for Jura Trace commands.
 *
 * When running inside Tauri, calls are dispatched via invoke().
 * When running in a browser (development), returns mock data so the
 * UI can be developed without the Rust backend running.
 */

import type { Annotation, AppErrorResponse, AppStats, Asset, AudioMetadataResult, AuditLogEntry, ConformantCertificateInfo, Fingerprint, LicenceTier, ManifestInfo, MetadataSigningWarning, MonitorEvent, MonitorOverview, MonitorUrl, NetworkMode, NprResult, RoiAnalysisResult, ShadowConsistencyResult, SidecarHealth, SidecarStartupSnapshot, SidecarStartupStatus, SigningMode, SimilarAsset, SolarPosition, SpliceBoundaryResult, TimeEstimate, VerificationResult, VerificationSummary, VerifyMode, VideoDeepfakeResult, VideoFramesResult, VideoMetadataResult, WatermarkEmbedResult, WatermarkExtractResult } from './types';

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

// ── File picker filters ─────────────────────────────────────────────
//
// Single source of truth for which file extensions Jura Trace accepts in
// the Protect (import) and Verify (batch) workflows. Extracted for unit
// testability so future regressions that re-add unsupported formats
// trigger a Vitest failure before reaching pilots.
//
// Format support truth-grid (JTV-105 / 2026-04-28 four-agent audit):
//   * image/jpeg, png, tiff, webp, avif, heic — full pipeline
//   * pdf — provenance only
//   * mp4, mov — C2PA experimental + per-frame deepfake
// Excluded for v1.0:
//   * webm, mkv, avi — no C2PA, no watermark, no fingerprint, no trust
//   * docx, odt, epub, txt — returns 0.50 trust with zero analysis
//   * gif — animated GIFs have no C2PA / watermark / fingerprint
//   * audio (wav/mp3/flac/ogg/aac/m4a) — model overfitted (AUC 1.0 on
//     2 speakers + 1 TTS engine). Drop until v1.1 AASIST retraining.
//   * 3D (stl/obj/gltf/glb) — no detector path, fall through to Unknown.

export const PROTECT_FILE_FILTERS: { name: string; extensions: string[] }[] = [
  {
    name: 'All Supported',
    extensions: [
      'jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'heic', 'heif', 'bmp', 'avif',
      'pdf',
      'mp4', 'mov',
    ],
  },
  {
    name: 'Images',
    extensions: ['jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'heic', 'heif', 'bmp', 'avif'],
  },
  { name: 'Documents', extensions: ['pdf'] },
  { name: 'Video', extensions: ['mp4', 'mov'] },
];

export const VERIFY_FILE_FILTERS: { name: string; extensions: string[] }[] = [
  {
    name: 'Supported Files',
    extensions: [
      'jpg', 'jpeg', 'png', 'tiff', 'tif', 'webp', 'avif',
      'heic', 'heif',
      'pdf',
      'mp4', 'mov',
    ],
  },
];

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
    filters: PROTECT_FILE_FILTERS,
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

/** Get filtered assets with optional content type, signed status, fingerprint status, and search.
 *  Results are paginated — `limit` defaults to 200, `offset` to 0. */
export async function getFilteredAssets(
  contentType?: string,
  c2paSigned?: boolean,
  searchQuery?: string,
  fingerprinted?: boolean,
  limit?: number,
  offset?: number,
): Promise<Asset[]> {
  try {
    return await invoke<Asset[]>('get_filtered_assets', {
      contentType: contentType ?? null,
      c2paSigned: c2paSigned ?? null,
      fingerprinted: fingerprinted ?? null,
      searchQuery: searchQuery ?? null,
      limit: limit ?? null,
      offset: offset ?? null,
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

/**
 * JTV-184 Phase 1 — fetch the current sidecar startup snapshot.
 *
 * Surfaces what the Rust shell knows about the sidecar lifecycle so the
 * Settings page can render a three-state badge instead of the old
 * binary online/offline indicator. The snapshot is cheap to fetch — it
 * reads a single AtomicU8 inside AppState; no HTTP probe to the sidecar
 * itself happens at this call site (the background probe in
 * `lib.rs:run()` owns that probing).
 *
 * Call this once on Settings-page mount to get the initial state
 * (events emitted before the listener attaches would otherwise be
 * missed) and then subscribe to {@link onSidecarStatusChanged} for
 * subsequent transitions.
 */
export async function getSidecarStartupStatus(): Promise<SidecarStartupSnapshot> {
  try {
    if (isTauri) {
      return await invoke<SidecarStartupSnapshot>('get_sidecar_startup_status');
    }
  } catch {
    // fall through to mock
  }
  return { status: 'notPresent', elapsedSecs: 0 };
}

/**
 * Subscribe to `sidecar-status-changed` Tauri events emitted by the
 * background readiness probe in the Rust shell. The callback fires on
 * each transition (NotPresent → Connecting → Ready). Returns an
 * unlisten function that must be called on component unmount.
 */
export async function onSidecarStatusChanged(
  callback: (status: SidecarStartupStatus) => void,
): Promise<() => void> {
  if (!isTauri) {
    return () => {};
  }
  // Lazy-import the event API so the browser-mock surface doesn't drag it.
  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<SidecarStartupStatus>(
    'sidecar-status-changed',
    (event) => callback(event.payload),
  );
  return unlisten;
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
    filters: VERIFY_FILE_FILTERS,
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
 *
 * # v1.0 caller contract (project_fp_report_v1_locked.md)
 *
 * The verify-page caller passes only the first three arguments
 * (`reasonCode`, `reasonNote`, `mimeType`). The Tier 2 fields below
 * (`deepfakeScore`, `deepfakeVerdict`, `signalScoresJson`) are
 * intentionally absent in v1.0 — they make the locally-stored report
 * a narrower fingerprint when combined with the timestamp and would
 * also need redaction at every export site. The signatures stay so
 * the Rust API is forward-compatible with the v1.0.1+ growth path.
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

// ── Backup & Restore (JTV-130) ─────────────────────────────────────

/** Result of `backupDatabase`. */
export interface BackupResult {
  snapshotPath: string;
  manifestPath: string;
  sha256: string;
  schemaVersion: number;
  timestamp: string;
}

/** Result of `restoreDatabase` — returned for both validate-only and confirmed runs. */
export interface RestoreResult {
  success: boolean;
  snapshotSchemaVersion: number;
  currentSchemaVersion: number;
  assetCount: number;
  auditChainValid: boolean;
  message: string;
}

/** Result of `importAssetsCsv`. */
export interface CsvImportResult {
  imported: number;
  skippedDuplicates: number;
  failed: number;
  errors: string[];
}

/**
 * Write a `VACUUM INTO` snapshot of the current database to a user-chosen
 * directory.  The snapshot file plus a JSON manifest sidecar are placed in
 * `destDir` with timestamps in their names.  POSIX permissions are set
 * to 0o600 on the snapshot file.
 */
export async function backupDatabase(destDir: string): Promise<BackupResult> {
  return invoke<BackupResult>('backup_database', { destDir });
}

/**
 * Auto-backup the database before the Tauri updater applies a new install.
 *
 * Resolves a stable per-user auto-backup directory under the OS-standard
 * application-data location (e.g. `~/Library/Application Support/Jura Trace/auto-backups/`
 * on macOS), creates it if missing, and writes the snapshot there.  Returns
 * the same `BackupResult` shape as the manual backup path so the snapshot
 * path can be surfaced to the user before the install proceeds.
 *
 * Call this between confirming an update is available and invoking
 * `update.downloadAndInstall()`.  If it throws, the caller should offer the
 * user a clear "abort update" choice — proceeding without a backup risks
 * data loss if the new version's schema migration fails.
 */
export async function autoBackupBeforeUpdate(): Promise<BackupResult> {
  return invoke<BackupResult>('auto_backup_before_update');
}

/**
 * Validate or restore a database snapshot.
 *
 * Two-phase pattern: call with `confirmed=false` first to populate the
 * destructive-action confirmation modal, then re-call with `confirmed=true`
 * if the user proceeds.  The Rust backend re-runs validation on the
 * confirmed call as a last-second tamper guard.
 */
export async function restoreDatabase(
  snapshotPath: string,
  confirmed: boolean,
): Promise<RestoreResult> {
  return invoke<RestoreResult>('restore_database', { snapshotPath, confirmed });
}

/**
 * Import asset metadata rows from a CSV catalogue file.
 *
 * The CSV must have a `file_path` column at minimum.  Optional columns:
 * `sha256_hash`, `file_name`, `content_type`, `c2pa_signed`, `watermarked`.
 * Rows whose SHA-256 already exists in the catalogue are skipped silently.
 * Hard cap of 10 000 rows.
 */
export async function importAssetsCsv(csvPath: string): Promise<CsvImportResult> {
  return invoke<CsvImportResult>('import_assets_csv', { csvPath });
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

// ── Power-saver mode ─────────────────────────────────────────────────

/**
 * Return the current power-saver mode preference.
 * Defaults to `false` when the setting has not been written to config.json.
 */
export async function getPowerSaverMode(): Promise<boolean> {
  try {
    return await invoke<boolean>('get_power_saver_mode');
  } catch {
    return false;
  }
}

/**
 * Persist the power-saver mode preference.
 *
 * When enabled, the analysis engine is stopped after five minutes of
 * inactivity. The first verification afterwards takes 30–90 seconds longer
 * while the engine reloads.
 */
export async function setPowerSaverMode(enabled: boolean): Promise<void> {
  return invoke<void>('set_power_saver_mode', { enabled });
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
 * @param mode      Analysis mode: 'standard' (6 frames) or 'deep' (20).
 *                  Legacy 'archival' is accepted by the Rust backend and
 *                  aliased to 'deep' (retired 2026-04-22); the previously
 *                  advertised 40-frame extraction was never realised — the
 *                  sidecar capped at 20.
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

// ── Network Access Mode ────────────────────────────────────────────

/**
 * Returns the current network access mode for this installation.
 *
 * - `standard` (default): fully local, no outbound network connections.
 *   Certificate revocation checks are skipped.
 * - `enhanced`: enables online verification features including OCSP/CRL
 *   revocation checks and remote Content Credentials retrieval.
 *
 * Defaults to 'standard' when the command is unavailable (browser context).
 */
export async function getNetworkMode(): Promise<NetworkMode> {
  if (isTauri) {
    try {
      return await invoke<NetworkMode>('get_network_mode');
    } catch {
      // Command not yet registered — fall through to default
    }
  }
  return 'standard';
}

/**
 * Persist a network access mode change.
 *
 * @param mode  'standard' for fully local; 'enhanced' for online verification features.
 * @returns     The resolved mode on success.
 */
export async function setNetworkMode(mode: NetworkMode): Promise<NetworkMode> {
  if (isTauri) {
    try {
      return await invoke<NetworkMode>('set_network_mode', { mode });
    } catch {
      // Command not yet registered — return the requested mode as a no-op mock
    }
  }
  return mode;
}

// ──────────────────────────────────────────────────────────────────
// On-demand investigation tools (NPR, Shadow Consistency, Splice
// Boundary).  These detectors do not auto-run in any verify mode —
// see src-tauri/src/lib.rs:1620 — and are surfaced via dedicated
// IPC commands so the v2 verify page's on-demand-tools footer can
// trigger them without re-running the full pipeline.  Each command
// expects a previously-verified file path; the caller is responsible
// for merging the returned result back into the active
// VerificationResult so subsequent renders pick up the new row.
// ──────────────────────────────────────────────────────────────────

/**
 * Run NPR (Neighbouring Pixel Relationships) analysis on demand.
 */
export async function runNprOnDemand(filePath: string): Promise<NprResult> {
  if (isTauri) {
    return await invoke<NprResult>('run_npr_on_demand', { filePath });
  }
  // Browser mock — returns a benign clean-state result so dev preview
  // does not visually claim manipulation when no sidecar is attached.
  return {
    score: 0.05,
    suspicious: false,
    hvCorrelation: 0.91,
    diffVarianceRatio: 0.6,
    hfEnergyRatio: 0.4,
    heatmapUrl: '',
    summary: 'Browser preview — no analysis run.',
  };
}

/**
 * Run shadow consistency analysis on demand.
 */
export async function runShadowConsistencyOnDemand(
  filePath: string,
): Promise<ShadowConsistencyResult> {
  if (isTauri) {
    return await invoke<ShadowConsistencyResult>('run_shadow_consistency_on_demand', { filePath });
  }
  return {
    score: 0.05,
    suspicious: false,
    inconsistentRegions: 0,
    totalRegions: 12,
    globalLightDirection: 90.0,
    heatmapUrl: '',
    summary: 'Browser preview — no analysis run.',
  } as ShadowConsistencyResult;
}

/**
 * Run splice boundary analysis on demand.
 */
export async function runSpliceBoundaryOnDemand(
  filePath: string,
): Promise<SpliceBoundaryResult> {
  if (isTauri) {
    return await invoke<SpliceBoundaryResult>('run_splice_boundary_on_demand', { filePath });
  }
  return {
    score: 0.05,
    suspicious: false,
    suspiciousBoundaries: 0,
    totalBoundariesChecked: 0,
    boundaries: [],
    heatmapUrl: '',
    summary: 'Browser preview — no analysis run.',
  } as SpliceBoundaryResult;
}

/**
 * Historical weather conditions for a single (lat, lon, date, hour) tuple.
 * Returned by `fetchWeatherContext`; mirrors the Rust `WeatherContext` struct.
 */
export interface WeatherContext {
  temperature: number;
  cloudCover: number;
  precipitation: number;
  visibility: number;
  windSpeed: number;
}

/**
 * Fetch historical weather conditions for a GPS coordinate, date, and hour
 * from the Open-Meteo archive. Moved server-side to the Rust IPC layer so
 * the Enhanced-mode network gate is enforced before any outbound HTTP call
 * is made — the previous frontend `networkMode === 'enhanced'` check was
 * bypassable from the browser console (security audit 2026-05-16 NEW-MED-1
 * / JTV-183).
 *
 * Throws if network mode is Standard, if inputs are out of range, or if the
 * upstream API is unreachable.
 */
export async function fetchWeatherContext(
  latitude: number,
  longitude: number,
  date: string,
  hour: number,
): Promise<WeatherContext> {
  if (isTauri) {
    return await invoke<WeatherContext>('fetch_weather_context', {
      latitude,
      longitude,
      date,
      hour,
    });
  }
  throw new Error('Weather context lookup is only available in the desktop application.');
}
