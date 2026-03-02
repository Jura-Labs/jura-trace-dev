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

/** Error Level Analysis result from the ML sidecar */
export interface ElaResult {
  elaImageBase64: string;
  maxDifference: number;
  meanDifference: number;
  score: number;
  suspicious: boolean;
}

/** Block-wise noise variance analysis result from the ML sidecar */
export interface NoiseResult {
  heatmapBase64: string;
  blockVariances: number[];
  globalVariance: number;
  anomalousBlocks: number;
  totalBlocks: number;
  score: number;
  suspicious: boolean;
}

/** A detected clone region bounding box */
export interface CloneRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  area: number;
  pointCount: number;
}

/** Copy-move forgery detection result from the ML sidecar */
export interface CopyMoveResult {
  visualisationBase64: string;
  cloneRegions: CloneRegion[];
  matchedPairs: number;
  score: number;
  suspicious: boolean;
}

/** A single signal from the deepfake detection ensemble */
export interface DeepfakeSignal {
  name: string;
  description: string;
  weight: number;
  triggered: boolean;
}

/** Deepfake / AI-generated image detection result from the ML sidecar */
export interface DeepfakeResult {
  score: number;
  suspicious: boolean;
  confidence: string;
  signals: DeepfakeSignal[];
  heatmapBase64: string;
  summary: string;
}

/** ML sidecar capability flags */
export interface SidecarCapabilities {
  ela: boolean;
  noise: boolean;
  copyMove: boolean;
  deepfake: boolean;
  rag: boolean;
}

/** ML sidecar health response */
export interface SidecarHealth {
  status: string;
  version: string;
  service: string;
  capabilities: SidecarCapabilities;
  ollama: string | null;
}

/** Verification result from the VERIFY pipeline */
export interface VerificationResult {
  sourceType: string;
  contentType: string;
  elaScore?: number;
  noiseScore?: number;
  copyMoveScore?: number;
  deepfakeScore?: number;
  c2paValid?: boolean;
  metadataFlags: string[];
  claimVerdict?: ClaimVerdict;
  overallTrust: number;
  exifAnalysis?: ExifAnalysis;
  c2paManifest?: ManifestInfo;
  elaResult?: ElaResult;
  noiseResult?: NoiseResult;
  copyMoveResult?: CopyMoveResult;
  deepfakeResult?: DeepfakeResult;
}

/** Severity level for an EXIF anomaly finding */
export type Severity = 'info' | 'low' | 'medium' | 'high' | 'critical';

/** A single anomaly finding from EXIF analysis */
export interface AnomalyFinding {
  checkId: string;
  title: string;
  description: string;
  severity: Severity;
  category: string;
}

/** Complete EXIF anomaly analysis result */
export interface ExifAnalysis {
  findings: AnomalyFinding[];
  trustScore: number;
  fieldsPopulated: number;
  fieldsTotal: number;
  hasExif: boolean;
}

/** Severity display configuration */
export const SEVERITY_CONFIG: Record<Severity, { label: string; textClass: string; bgClass: string }> = {
  info: { label: 'Info', textClass: 'text-flint', bgClass: 'bg-graphite' },
  low: { label: 'Low', textClass: 'text-lapis', bgClass: 'bg-lapis/10' },
  medium: { label: 'Medium', textClass: 'text-amber', bgClass: 'bg-amber/10' },
  high: { label: 'High', textClass: 'text-cinnabar', bgClass: 'bg-cinnabar/10' },
  critical: { label: 'Critical', textClass: 'text-cinnabar', bgClass: 'bg-cinnabar/20' },
};

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

/** C2PA manifest information read from a file */
export interface ManifestInfo {
  title?: string;
  format?: string;
  claimGenerator?: string;
  assertions: AssertionInfo[];
  isValid: boolean;
  signedAt?: string;
}

/** A single assertion within a C2PA manifest */
export interface AssertionInfo {
  label: string;
  value: string;
}

/** Perceptual hash types */
export type HashType = 'ahash' | 'dhash' | 'phash';

/** Fingerprint record from the database */
export interface Fingerprint {
  fingerprintId: string;
  assetId: string;
  hashType: HashType;
  hashValue: string;
  createdAt: string;
}

/** Result of a similarity search */
export interface SimilarAsset {
  assetId: string;
  fileName: string;
  hashType: HashType;
  distance: number;
  similarity: number;
}

/** Hash type display labels */
export const HASH_TYPE_LABELS: Record<HashType, string> = {
  ahash: 'Average Hash',
  dhash: 'Difference Hash',
  phash: 'Perceptual Hash',
};

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
