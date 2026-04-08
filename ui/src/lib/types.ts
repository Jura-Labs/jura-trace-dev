/**
 * Jura Trace TypeScript type definitions.
 * These mirror the Rust structs in src-tauri/src/lib.rs.
 * Field names are camelCase (Rust uses serde rename_all).
 */

/**
 * Structured error response from Tauri commands that use AppError.
 *
 * Commands migrated to AppError serialise their errors as:
 *   { "code": "Sidecar", "message": "Analysis service unavailable: ..." }
 *
 * Commands still using `map_err(|e| e.to_string())` return a plain string.
 * Use `parseAppError` in api.ts to handle both forms.
 */
export interface AppErrorResponse {
  code: 'Database' | 'FileSystem' | 'Sidecar' | 'Validation' | 'C2pa' | 'Internal';
  message: string;
}

/**
 * Verify pipeline investigation modes:
 * - standard: EXIF + C2PA + ELA + deepfake ensemble (~15s) — default
 * - deep: Full pipeline including all detectors (~60s)
 * - archival: Deep with scanner-calibrated tolerances
 *
 * Legacy 'quick'/'fast' values remain accepted by the Rust backend for
 * backwards compatibility but are no longer exposed in the UI.
 */
export type VerifyMode = 'standard' | 'deep' | 'archival';

/**
 * Licence tier for this installation.
 *
 * Values mirror the `LicenceTier` Rust enum with `serde(rename_all = "camelCase")`.
 * Internal geological codenames: Community=Flint, Professional=Stratum,
 * Team=Geode, Enterprise=Bedrock.
 *
 * During the pilot phase this can be set manually from Settings.
 * Post-v1.0, tier enforcement will use a signed JWT.
 */
export type LicenceTier = 'community' | 'professional' | 'team' | 'enterprise';

/** Display metadata for a licence tier. */
export interface TierInfo {
  /** Internal tier value (matches the Rust enum variant, camelCase). */
  tier: LicenceTier;
  /** User-facing plain name. */
  name: string;
  /** Internal geological codename (used only in admin/pilot UI). */
  codename: string;
  /** Short description for the Settings panel. */
  description: string;
  /** Tailwind colour class for the badge background. */
  badgeClass: string;
  /** Tailwind text colour class for the badge. */
  badgeTextClass: string;
}

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
  /** Whether at least one perceptual fingerprint exists for this asset. */
  fingerprinted: boolean;
  createdAt: string;
  sha256Hash?: string;
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

/** An invisible watermark detected in an image (e.g. Stable Diffusion, SDXL) */
export interface WatermarkDetection {
  watermarkType: string;
  detected: boolean;
  confidence: number;
  details: string;
}

/** Three-way verdict from deepfake detection */
export type VerdictLevel = 'authentic' | 'inconclusive' | 'synthetic';

/** Deepfake / AI-generated image detection result from the ML sidecar */
export interface DeepfakeResult {
  score: number;
  suspicious: boolean;
  confidence: string;
  verdictLevel?: VerdictLevel;
  signals: DeepfakeSignal[];
  heatmapBase64: string;
  summary: string;
  watermarks?: WatermarkDetection[];
  classifierScore?: number | null;
  classifierAvailable?: boolean;
}

/** NPR (Neighbouring Pixel Relationships) analysis result */
export interface NprResult {
  score: number;
  suspicious: boolean;
  hvCorrelation: number;
  diffVarianceRatio: number;
  hfEnergyRatio: number;
  heatmapBase64: string;
  summary: string;
}

/** JPEG ghost detection result for splice/composite forgery analysis */
export interface JpegGhostResult {
  score: number;
  suspicious: boolean;
  ghostQuality: number;
  qualityVariance: number;
  deviatingBlocks: number;
  totalBlocks: number;
  heatmapBase64: string;
  summary: string;
}

/** Chromatic Aberration consistency analysis result */
export interface CaResult {
  rSquared: number;
  isConsistent: boolean;
  score: number;
  suspicious: boolean;
  sampleCount: number;
  summary: string;
}

// ── Region-based Forensic Detectors ───────────────────────────────

/** A single region analysed by the segmented ELA detector */
export interface ElaRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  elaScore: number;
  anomalous: boolean;
}

/** Segmented (region-aware) Error Level Analysis result */
export interface SegmentedElaResult {
  heatmapBase64: string | null;
  regions: ElaRegion[];
  anomalousRegions: number;
  totalRegions: number;
  interRegionVariance: number;
  score: number;
  suspicious: boolean;
  summary: string;
}

/** A single region analysed by the shadow consistency detector */
export interface ShadowRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  area: number;
  gradientAngleMean: number;
  deviationFromGlobal: number;
  inconsistent: boolean;
}

/** Shadow direction consistency analysis result */
export interface ShadowConsistencyResult {
  heatmapBase64: string | null;
  globalLightDirection: number;
  regions: ShadowRegion[];
  inconsistentRegions: number;
  totalRegions: number;
  score: number;
  suspicious: boolean;
  summary: string;
}

/** A single region analysed by the colour temperature detector */
export interface ColourTempRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  meanA: number;
  meanB: number;
  deviationFromGlobal: number;
  anomalous: boolean;
}

/** Colour temperature consistency analysis result */
export interface ColourTemperatureResult {
  heatmapBase64: string | null;
  regions: ColourTempRegion[];
  anomalousRegions: number;
  totalRegions: number;
  globalMeanA: number;
  globalMeanB: number;
  score: number;
  suspicious: boolean;
  summary: string;
}

/** A candidate splice boundary detected at a grid junction */
export interface SpliceBoundary {
  x: number;
  y: number;
  width: number;
  height: number;
  jpegGridAligned: boolean;
  noiseAsymmetric: boolean;
  featheringDetected: boolean;
  signalsTriggered: number;
  confidence: number;
}

/** Splice boundary detection result */
export interface SpliceBoundaryResult {
  heatmapBase64: string | null;
  boundaries: SpliceBoundary[];
  suspiciousBoundaries: number;
  totalBoundariesChecked: number;
  score: number;
  suspicious: boolean;
  summary: string;
}

/** ML sidecar capability flags */
export interface SidecarCapabilities {
  ela: boolean;
  noise: boolean;
  copyMove: boolean;
  deepfake: boolean;
  jpegGhost: boolean;
  npr: boolean;
  chromaticAberration: boolean;
  rag: boolean;
  watermark: boolean;
  clipDetect: boolean;
  videoMetadata: boolean;
  audioMetadata: boolean;
  videoFrames: boolean;
  videoDeepfake: boolean;
  transcription: boolean;
}

/** ML sidecar health response */
export interface SidecarHealth {
  status: string;
  version: string;
  service: string;
  capabilities: SidecarCapabilities;
  ollama: string | null;
  ollamaModels?: string[] | null;
}

/** CLIP-based AI classification result from the ML sidecar */
export interface ClipDetectionResult {
  score: number;
  verdictLevel: VerdictLevel;
  confidence: string;
  classProbs: Record<string, number>;
  summary: string;
}

/** RAG claim verification source reference */
export interface ClaimSource {
  title: string;
  excerpt: string;
  relevance: number;
}

/** RAG claim verification result */
export interface RagClaimResult {
  verdict: ClaimVerdict;
  confidence: number;
  explanation: string;
  sources: ClaimSource[];
}

/** Solar position calculation result from the NOAA algorithm. */
export interface SolarPosition {
  azimuth: number;
  elevation: number;
  solarNoonUtc: number;
  dayLengthHours: number;
}

/** Noise pattern visualisation result. */
export interface NoiseVisualisationResult {
  noiseResidualBase64: string;
  varianceHeatmapBase64: string;
  noiseStd: number;
  noiseMean: number;
}

/** CLAHE enhancement result. */
export interface ClaheResult {
  enhancedImageBase64: string;
  clipLimit: number;
}

/** Frequency domain visualisation result. */
export interface FrequencyVisualisationResult {
  fftMagnitudeBase64: string;
  dctHeatmapBase64: string;
  hasJpegGrid: boolean;
  dominantFrequency: number;
}

/** JPEG quantisation grid visualisation result. */
export interface JpegGridResult {
  gridArtefactBase64: string;
  qTable: number[][] | null;
  gridConsistency: number;
}

/** Input quality assessment — identifies conditions that degrade detector reliability. */
export interface InputQualityAssessment {
  /** Estimated JPEG quality factor (1–100). null for non-JPEG. */
  jpegQualityEstimate?: number | null;
  /** Resolution category: "high", "medium", "low", "thumbnail", "n/a". */
  resolutionCategory: string;
  /** Image width in pixels. */
  width?: number | null;
  /** Image height in pixels. */
  height?: number | null;
  /** Whether the image appears to be a screenshot. */
  isScreenshotLikely: boolean;
  /** Whether the file is JPEG format. */
  isJpeg: boolean;
  /** Whether EXIF GPS coordinates are present. */
  hasGps: boolean;
  /** Whether EXIF timestamp is present. */
  hasTimestamp: boolean;
  /** Detector names with reduced reliability for this input. */
  degradedDetectors: string[];
}

/** Verification result from the VERIFY pipeline */
export interface VerificationResult {
  /** Investigation mode used: 'standard' | 'deep' | 'archival' */
  mode?: string;
  /** SHA-256 hash of the input file, if computed by the backend. */
  inputSha256?: string;
  sourceType: string;
  contentType: string;
  elaScore?: number;
  noiseScore?: number;
  copyMoveScore?: number;
  deepfakeScore?: number;
  c2paValid?: boolean;
  metadataFlags: string[];
  claimVerdict?: ClaimVerdict;
  ragClaimResult?: RagClaimResult;
  overallTrust: number;
  exifAnalysis?: ExifAnalysis;
  c2paManifest?: ManifestInfo;
  elaResult?: ElaResult;
  noiseResult?: NoiseResult;
  copyMoveResult?: CopyMoveResult;
  deepfakeResult?: DeepfakeResult;
  nprResult?: NprResult;
  jpegGhostResult?: JpegGhostResult;
  caResult?: CaResult;
  clipResult?: ClipDetectionResult;
  segmentedElaResult?: SegmentedElaResult | null;
  shadowConsistencyResult?: ShadowConsistencyResult | null;
  colourTemperatureResult?: ColourTemperatureResult | null;
  spliceBoundaryResult?: SpliceBoundaryResult | null;
  aiGenerator?: string;
  watermarkExtractResult?: WatermarkExtractResult | null;
  videoFramesResult?: VideoFramesResult | null;
  videoDeepfakeResult?: VideoDeepfakeResult | null;
  transcriptionResult?: TranscriptionResult | null;
  claimCheckResult?: ClaimCheckResult | null;
  /** AI-generated natural-language description via Ollama LLaVA. Only present
   * for image content when Ollama is running with llava:7b pulled. */
  aiDescription?: string | null;
  /** EXIF thumbnail vs main image consistency check (images only). */
  thumbnailCheck?: ThumbnailCheck | null;
  /** Methodology metadata for reproducibility (pipeline version, sidecar version, classifier hash). */
  methodology?: MethodologyRecord | null;
  /** Input quality assessment — conditions that degrade detector reliability. */
  inputQuality?: InputQualityAssessment | null;
}

/** Methodology metadata captured at verification time for reproducibility. */
export interface MethodologyRecord {
  /** Jura Trace application version (e.g. "0.9.0"). */
  pipelineVersion: string;
  /** Python ML sidecar version (e.g. "0.2.0"), if available. */
  sidecarVersion?: string | null;
  /** SHA-256 hex digest of the GBM classifier model file, if present. */
  classifierModelHash?: string | null;
  /** Investigation mode used (quick, standard, deep, archival). */
  analysisMode: string;
  /** ISO 8601 timestamp when the analysis was performed. */
  analysedAt: string;
}

/** EXIF thumbnail vs main image consistency check. */
export interface ThumbnailCheck {
  hasThumbnail: boolean;
  hammingDistance?: number;
  mismatch: boolean;
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
  /** GPS latitude in decimal degrees (added by Rust backend; optional pending backend update). */
  gpsLatitude?: number;
  /** GPS longitude in decimal degrees (added by Rust backend; optional pending backend update). */
  gpsLongitude?: number;
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

// ── Batch Verification ────────────────────────────────────────────

/** Status of a single item in a batch verification queue */
export type BatchItemStatus = 'queued' | 'running' | 'done' | 'error';

/** A single row in the batch verification results table */
export interface BatchItem {
  id: string;
  filePath: string;
  fileName: string;
  status: BatchItemStatus;
  result: VerificationResult | null;
  error: string | null;
  startedAt: number | null;
  finishedAt: number | null;
}

/** Duration string for a completed batch item */
export function formatDuration(startedAt: number, finishedAt: number): string {
  const ms = finishedAt - startedAt;
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}

// ── Metadata Signing Warning ──────────────────────────────────────

/** Warning about existing metadata before C2PA signing */
export interface MetadataSigningWarning {
  hasExistingArtist: boolean;
  existingArtist: string | null;
  hasExistingCopyright: boolean;
  existingCopyright: string | null;
  hasExistingDescription: boolean;
  existingDescription: string | null;
  hasExistingC2pa: boolean;
  warningMessage: string | null;
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

// ── Transcription types ───────────────────────────────────────────

/** A single timestamped segment from speech transcription */
export interface TranscriptionSegment {
  start: number;
  end: number;
  text: string;
}

/** Audio/video speech transcription result from the ML sidecar */
export interface TranscriptionResult {
  text: string;
  segments: TranscriptionSegment[];
  language?: string;
  languageProbability?: number;
  duration?: number;
  modelSize: string;
  success: boolean;
  message: string;
}

/** A single claim verdict from the RAG claim checker */
export interface ClaimCheckVerdict {
  claim: string;
  verdict: string;
  explanation: string;
  confidence: number;
}

/** RAG claim verification result from the sidecar */
export interface ClaimCheckResult {
  overallVerdict: string;
  claims: ClaimCheckVerdict[];
  modelUsed: string;
  methodology: string;
  summary: string;
}

// ── Video / Audio types ───────────────────────────────────────────

/** Technical metadata extracted from a video file */
export interface VideoMetadataResult {
  duration?: number;
  codec?: string;
  width?: number;
  height?: number;
  fps?: number;
  hasAudio: boolean;
  audioCodec?: string;
  bitrate?: number;
  fileSize?: number;
  success: boolean;
  message: string;
}

/** Technical metadata extracted from an audio file */
export interface AudioMetadataResult {
  duration?: number;
  codec?: string;
  sampleRate?: number;
  channels?: number;
  bitrate?: number;
  fileSize?: number;
  success: boolean;
  message: string;
}

/** A set of representative frame thumbnails from a video */
export interface VideoFramesResult {
  frames: string[];  // base64-encoded JPEG thumbnails
  count: number;
  duration?: number;
  success: boolean;
  message: string;
}

// ── Video Deepfake Analysis ───────────────────────────────────────

/** Per-frame deepfake analysis result within a video */
export interface FrameDeepfakeResult {
  frameIndex: number;
  timestamp: number;
  score: number;
  suspicious: boolean;
  verdictLevel: VerdictLevel;
  signals: DeepfakeSignal[];
  classifierScore?: number | null;
  classifierAvailable?: boolean;
  heatmapBase64?: string;
}

/** Video-level deepfake analysis result aggregated from per-frame scoring */
export interface VideoDeepfakeResult {
  frameResults: FrameDeepfakeResult[];
  aggregateScore: number;
  aggregateVerdict: VerdictLevel;
  aggregateConfidence: string;
  framesAnalysed: number;
  framesRequested: number;
  temporalAvailable: boolean;
  temporalNoiseDrift?: number | null;
  temporalSpectralDrift?: number | null;
  temporalLbpDrift?: number | null;
  mode: string;
  duration?: number | null;
  success: boolean;
  message: string;
}

// ── Watermarking ──────────────────────────────────────────────────

/** Result from embedding an invisible watermark into an asset */
export interface WatermarkEmbedResult {
  outputPath: string;
  payloadHex: string;
  success: boolean;
  message: string;
}

/** Result from extracting an invisible watermark from an asset */
export interface WatermarkExtractResult {
  extractedPayload?: string | null;
  extractedHex?: string | null;
  hasWatermark: boolean;
  confidence: number;
  success: boolean;
  message: string;
}

// ── Monitor types ────────────────────────────────────────────────

/** A single entry from the audit log */
export interface AuditLogEntry {
  logId: number;
  action: string;
  targetType: string;
  targetId: string;
  details?: string;
  createdAt: string;
}

/** Summary row from the verifications table */
export interface VerificationSummary {
  verificationId: string;
  sourceType: string;
  contentType: string;
  elaScore?: number;
  deepfakeScore?: number;
  c2paValid?: boolean;
  overallTrust: number;
  createdAt: string;
}

/** Aggregate trust distribution across all verifications */
export interface TrustDistribution {
  total: number;
  highCount: number;
  mediumCount: number;
  lowCount: number;
  averageTrust: number;
  latestAt?: string;
}

/** Aggregate protection statistics across all assets */
export interface ProtectionSummary {
  totalAssets: number;
  c2paSigned: number;
  watermarked: number;
  fingerprinted: number;
  byContentType: Record<string, number>;
  earliestAt?: string;
  latestAt?: string;
}

/** Activity counts for a single calendar day */
export interface ActivityDay {
  date: string;
  imports: number;
  verifications: number;
  signings: number;
  deletions: number;
}

/** Top-level monitor overview combining all data sources */
export interface MonitorOverview {
  protection: ProtectionSummary;
  trust: TrustDistribution;
  recentActivity: AuditLogEntry[];
  activityDays: ActivityDay[];
}

// ── Monitor URL Watchlist types ───────────────────────────────────

/** A URL registered for periodic monitoring */
export interface MonitorUrl {
  urlId: string;
  assetId: string | null;
  url: string;
  label: string | null;
  checkFrequency: string;
  lastCheckedAt: string | null;
  /** Outcome of the most recent check: "ok" | "changed" | "missing" | "error" */
  lastStatus: string | null;
  lastContentHash: string | null;
  lastC2paValid: boolean | null;
  lastWatermarkMatch: boolean | null;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

/** One recorded check result for a monitored URL */
export interface MonitorEvent {
  eventId: string;
  urlId: string;
  /** Categorised outcome: "check_ok" | "content_changed" | "c2pa_stripped" | etc. */
  eventType: string;
  checkedAt: string;
  contentHash: string | null;
  c2paValid: boolean | null;
  watermarkUuid: string | null;
  watermarkConfidence: number | null;
  httpStatus: number | null;
  responseTimeMs: number | null;
  /** Case state: "new" | "investigating" | "resolved" | "escalated" | "dismissed" */
  caseStatus: string;
  caseNotes: string | null;
  caseUpdatedAt: string | null;
}

// ── Sprint 24 investigation types ────────────────────────────────

/** Result from a region-of-interest forensic analysis. */
export interface RoiAnalysisResult {
  noiseStd: number;
  noiseMean: number;
  elaMean: number;
  frequencyEnergy: number;
  textureComplexity: number;
  noiseResidualBase64: string;
  roi: { x: number; y: number; width: number; height: number };
}

/** A candidate time estimate from shadow azimuth inversion. */
export interface TimeEstimate {
  hourUtc: number;
  timeFormatted: string;
  sunElevation: number;
  azimuthError: number;
}

/** Diffusion model artefact analysis result. */
export interface DiffusionArtefactsResult {
  textureSmoothnessScore: number;
  textureSmoothnessMapBase64: string;
  vaeBandingScore: number;
  resolutionMatch: boolean;
  resolutionNote: string;
  overallDiffusionScore: number;
}

// ── Annotation types ─────────────────────────────────────────────

/**
 * A persisted annotation drawn on a verification result image.
 * The geometry is serialised as JSON in `dataJson` so that the
 * Rust backend can store it without knowing the annotation shape.
 */
export interface Annotation {
  annotationId: string;
  verificationId?: string;
  assetId?: string;
  annotationType: 'arrow' | 'circle' | 'rectangle' | 'text' | 'freehand';
  dataJson: string;
  createdAt: string;
}

/**
 * The geometry payload serialised into `Annotation.dataJson`.
 * All coordinate values are in natural image pixels so that
 * annotations remain accurate regardless of display size.
 */
export interface AnnotationData {
  x: number;
  y: number;
  x2?: number;
  y2?: number;
  width?: number;
  height?: number;
  radius?: number;
  text?: string;
  colour: string;
  strokeWidth: number;
}

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
