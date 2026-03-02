/**
 * Tauri IPC wrapper for Jura Archive commands.
 *
 * When running inside Tauri, calls are dispatched via invoke().
 * When running in a browser (development), returns mock data so the
 * UI can be developed without the Rust backend running.
 */

import type { AppStats, Asset, Fingerprint, ManifestInfo, SidecarHealth, SimilarAsset, VerificationResult } from './types';

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

export async function verifyContent(source: string, sourceType: string): Promise<VerificationResult> {
  return invoke<VerificationResult>('verify_content', { source, sourceType });
}

/** Run full verification pipeline on a file. */
export async function verifyFile(filePath: string): Promise<VerificationResult> {
  return invoke<VerificationResult>('verify_content', {
    source: filePath,
    sourceType: 'file',
  });
}

/** Verify content from a URL. Downloads and analyses the content. */
export async function verifyUrl(url: string): Promise<VerificationResult> {
  if (isTauri) {
    return invoke<VerificationResult>('verify_url', { url });
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
      capabilities: { ela: true, deepfake: false, rag: false },
      ollama: null,
    };
  } catch {
    return null;
  }
}

// ── C2PA ──────────────────────────────────────────────────────────

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
