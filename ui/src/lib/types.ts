/**
 * Jura Archive TypeScript type definitions.
 * These mirror the Rust structs in src-tauri/src/lib.rs.
 * Field names are camelCase (Rust uses serde rename_all).
 */

/** Asset record stored in the local database */
export interface Asset {
  assetId: string;
  filePath: string;
  fileName: string;
  contentType: ContentType;
  mimeType: string;
  fileSize: number;
  width?: number;
  height?: number;
  aiDescription?: string;
  aiTags?: string[];
  metadataJson?: string;
  c2paSigned: boolean;
  watermarked: boolean;
  createdAt: string;
}

/** Supported content types */
export type ContentType = 'image' | 'document' | 'video' | 'audio' | '3d' | 'web' | 'unknown';

/** Verification result from the VERIFY pipeline */
export interface VerificationResult {
  sourceType: string;
  contentType: string;
  elaScore?: number;
  deepfakeScore?: number;
  c2paValid?: boolean;
  metadataFlags: string[];
  claimVerdict?: ClaimVerdict;
  overallTrust: number;
}

/** Claim verdict from RAG verification */
export type ClaimVerdict = 'supported' | 'disputed' | 'unverified' | 'mixed';

/** Application statistics for the dashboard */
export interface AppStats {
  totalAssets: number;
  totalFingerprints: number;
  totalVerifications: number;
  c2paSignedCount: number;
}

/** Parsed EXIF metadata from an image */
export interface ImageMetadata {
  cameraMake?: string;
  cameraModel?: string;
  software?: string;
  datetimeOriginal?: string;
  datetimeModified?: string;
  exifWidth?: number;
  exifHeight?: number;
  colorSpace?: string;
  gpsLatitude?: number;
  gpsLongitude?: number;
  iso?: number;
  focalLength?: string;
  exposureTime?: string;
  fNumber?: string;
  copyright?: string;
  artist?: string;
  description?: string;
  orientation?: number;
}

/** Trust level derived from overall trust score */
export type TrustLevel = 'high' | 'medium' | 'low';

/** Get trust level from score (0.0-1.0) */
export function getTrustLevel(score: number): TrustLevel {
  if (score >= 0.7) return 'high';
  if (score >= 0.4) return 'medium';
  return 'low';
}

/** Parse the metadataJson field from an asset */
export function parseMetadata(asset: Asset): ImageMetadata | null {
  if (!asset.metadataJson) return null;
  try {
    return JSON.parse(asset.metadataJson) as ImageMetadata;
  } catch {
    return null;
  }
}

/** Format file size for display */
export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** Content type display labels */
export const CONTENT_TYPE_LABELS: Record<ContentType, string> = {
  image: 'Image',
  document: 'Document',
  video: 'Video',
  audio: 'Audio',
  '3d': '3D Model',
  web: 'Web',
  unknown: 'Unknown',
};

/** Supported file extensions by content type */
export const SUPPORTED_EXTENSIONS: Record<ContentType, string[]> = {
  image: ['.jpg', '.jpeg', '.png', '.tiff', '.tif', '.webp', '.heic', '.heif', '.bmp', '.gif', '.svg', '.avif', '.ico'],
  document: ['.pdf', '.docx', '.odt', '.epub', '.txt', '.rtf', '.html', '.md', '.csv', '.xlsx'],
  video: ['.mp4', '.mov', '.webm', '.avi', '.mkv', '.m4v'],
  audio: ['.wav', '.mp3', '.flac', '.ogg', '.aac', '.m4a', '.aiff', '.opus'],
  '3d': ['.stl', '.obj', '.gltf', '.glb', '.fbx', '.ply', '.usdz', '.3mf', '.dae'],
  web: ['.html'],
  unknown: [],
};
