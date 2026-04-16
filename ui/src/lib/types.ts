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
export type ContentType = 'image' | 'document' | 'video' | 'audio' | 'unknown';

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
  /**
   * Whether the file uses a modern lossy codec (AVIF or WebP) that destroys
   * JPEG-specific compression artefacts relied upon by ELA, noise analysis,
   * copy-move detection, and JPEG ghost. Both formats also aggressively strip
   * metadata in typical web delivery pipelines.
   */
  isModernLossyCodec: boolean;
  /**
   * Whether the file contains no EXIF data AND no XMP data.
   * A strong indicator of metadata stripping via social media, CDN processing,
   * or format conversion (e.g. AVIF downloaded from the web). When true, all
   * provenance-based checks (camera identification, timestamp verification,
   * AI-provenance declaration) are unavailable.
   */
  metadataCompletelyAbsent: boolean;
  /** Detector names with reduced reliability for this input. */
  degradedDetectors: string[];
}

/**
 * Semantic content-type classification from the ML sidecar.
 *
 * When `aiDetectionSuitable` is `false`, the deepfake and CLIP scores have been
 * neutralised in trust scoring — consumers should surface a notice to the user
 * explaining why AI-detection results are not shown.
 */
export interface ContentTypeResult {
  /** Semantic category returned by the sidecar classifier. */
  category: 'photograph' | 'screenshot' | 'document' | 'artwork' | 'unknown';
  /** Classifier confidence in [0.0, 1.0]. */
  confidence: number;
  /**
   * `false` when AI-detection models (deepfake GBM + CLIP probe) are not
   * reliable for this content type. The Rust pipeline neutralises those
   * signals to 0.5 (neutral) when this flag is false.
   */
  aiDetectionSuitable: boolean;
  /** Raw classification signals returned by the sidecar (passthrough dict). */
  signals: Record<string, unknown>;
  /** Human-readable reasoning string from the classifier. */
  reasoning: string;
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
  clipResult?: ClipDetectionResult;
  segmentedElaResult?: SegmentedElaResult | null;
  shadowConsistencyResult?: ShadowConsistencyResult | null;
  colourTemperatureResult?: ColourTemperatureResult | null;
  spliceBoundaryResult?: SpliceBoundaryResult | null;
  aiGenerator?: string;
  watermarkExtractResult?: WatermarkExtractResult | null;
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
  /**
   * Semantic content-type classification from the sidecar.
   * Present for image content when the sidecar is available.
   * When `aiDetectionSuitable` is `false`, the deepfake and CLIP contributions
   * have been neutralised in trust scoring.
   */
  contentTypeResult?: ContentTypeResult | null;
  /**
   * Stable string identifiers for every detector that actually produced a
   * result for this verification. Added in Sprint 28 (S28-FU1) alongside the
   * schema v6 `detectors_run` DB column. Consumers (PDF renderer, Expert
   * View badges) use this as an authoritative list of what ran, so missing
   * entries can be labelled "not run in this analysis" instead of silently
   * dropped.
   *
   * Vocabulary (must match `src-tauri/src/lib.rs` `detectors_run_list`):
   * `exif_anomaly`, `c2pa`, `ela`, `noise`, `copy_move`, `deepfake`,
   * `jpeg_ghost`, `segmented_ela`, `colour_temperature`, `clip`,
   * `watermark`, `video_deepfake`, `transcription`, and on-demand entries
   * `npr`, `shadow_consistency`, `splice_boundary` when triggered.
   *
   * Optional for backwards compatibility with old DB rows that predate
   * the schema v6 migration — when absent, consumers should fall back to
   * inferring detector presence from individual `*Result` field population.
   */
  detectorsRun?: string[];
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
  /**
   * MakerNote-derived camera authenticity confidence (0.0–1.0). Added in
   * Sprint 29 (Track 1). A value > 0.5 means the file carries a
   * vendor-recognised camera MakerNote — vendor-proprietary binary blobs
   * that AI image generators virtually never synthesise. Used as a
   * positive authenticity signal that suppresses the final deepfake score
   * proportionally to mitigate false positives on computational
   * photography output. 0.0 means no MakerNote, no signal either way.
   *
   * Field name on the Rust side: `camera_authenticity_bonus` on
   * `ExifAnalysis` in `src-tauri/src/exif_anomaly.rs`, serialised via
   * serde camelCase rename (see `camera_authenticity_bonus_serializes_camelcase`
   * test at line 900 of that file).
   */
  cameraAuthenticityBonus?: number;
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
/**
 * Knowledge base retrieval match status.
 *
 * This type intentionally retains the legacy name `ClaimVerdict` for
 * backwards compatibility with older sidecar builds and existing code,
 * but the underlying tool is NOT a fact-checker — see the model card at
 * /help/model-cards#kb-retrieval. The status values were renamed from
 * "supported / disputed / unverified / mixed" in April 2026 to reflect
 * that the tool reports retrieval coverage, not factual verdicts.
 *
 * Legacy values are retained in the union so that older analyses still
 * render correctly when loaded from the SQLite database or imported
 * from v0.9 case exports.
 */
export type ClaimVerdict =
  // New vocabulary (v0.9.0 post-April 2026)
  | 'consistent_with_kb'
  | 'inconsistent_with_kb'
  | 'insufficient_context_in_kb'
  | 'mixed_kb_match'
  | 'unavailable'
  // Legacy vocabulary (pre-April 2026 — retained for backwards compatibility)
  | 'supported'
  | 'disputed'
  | 'unverified'
  | 'mixed';

/** Application statistics for the dashboard */
export interface AppStats {
  totalAssets: number;
  totalFingerprints: number;
  totalVerifications: number;
  c2paSignedCount: number;
}

/** A single edit event from the XMP edit-history stack (xmpMM:History). */
export interface XmpHistoryEvent {
  /** The action performed: "created", "saved", "converted", etc. */
  action: string;
  /** The software agent (e.g. "Adobe Photoshop 25.0 (Macintosh)"). */
  softwareAgent: string;
  /** ISO 8601 timestamp of the action, if present. */
  when?: string | null;
  /** Additional parameters (e.g. "converted from image/jpeg to image/jpeg"). */
  parameters?: string | null;
}

/** Parsed XMP metadata from an image */
export interface XmpMetadata {
  digitalSourceType?: string | null;
  creatorTool?: string | null;
  credit?: string | null;
  creator?: string | null;
  /** Parsed xmpMM:History edit-history stack. */
  history: XmpHistoryEvent[];
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
  /** Parsed XMP metadata including edit-history stack. */
  xmp?: XmpMetadata;
}

/** C2PA manifest information read from a file */
export interface ManifestInfo {
  title?: string;
  format?: string;
  claimGenerator?: string;
  assertions: AssertionInfo[];
  isValid: boolean;
  /** Cert expired but trusted timestamp + valid claim signature prove the signature
   *  was valid when issued (common for short-lived certs like Google Pixel Camera). */
  validAtSigning?: boolean;
  signedAt?: string;
  /** Signer common name from signature_info (e.g. "Pixel Camera"). */
  signedBy?: string;
  /** Signer issuer from signature_info (e.g. "Google LLC"). */
  signedByIssuer?: string;
  /** Individual validation checks from c2pa-rs, grouped by outcome. */
  validationChecks?: ValidationCheck[];
}

/** A single C2PA validation check result. */
export interface ValidationCheck {
  code: string;
  outcome: 'pass' | 'info' | 'fail';
  explanation?: string;
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

// ── Audio deepfake detection ──────────────────────────────────────

/**
 * Result from the two-stage audio deepfake ensemble (Sprint 35).
 * `modelLoaded` is false until trained probe files are deployed; the
 * UI should render "Audio deepfake detection available after model training"
 * rather than a failure indicator in that state.
 */
export interface AudioDeepfakeResult {
  /** Ensemble score 0–1 (0 = authentic, 1 = synthetic). Null when no probe is loaded. */
  score: number | null;
  /** "authentic" | "inconclusive" | "likely_synthetic" | "model_not_loaded" */
  verdict: string;
  /** Stage 1 score from MFCC + GradientBoostingClassifier. */
  stage1Score: number | null;
  /** Stage 2 score from Wav2Vec2-Base + LogisticRegression. */
  stage2Score: number | null;
  /** Which stages actually ran, e.g. ["stage1"] or ["stage1", "stage2"]. */
  stagesAvailable: string[];
  /** False until trained probe files are deployed to models/. */
  modelLoaded: boolean;
  /** Audio duration in seconds if the file could be loaded. */
  durationSeconds: number | null;
  /** Sample rate after resampling (16 000 Hz when librosa is available). */
  sampleRate: number | null;
  /** True when a 160-dim MFCC feature vector was successfully extracted. */
  mfccFeaturesExtracted: boolean;
  /** True when a 768-dim Wav2Vec2 embedding was successfully extracted. */
  wav2vec2EmbeddingExtracted: boolean;
  /** Wall-clock time for the full ensemble call in milliseconds. */
  processingTimeMs: number | null;
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
  /** Ordered list of detector IDs from schema v6 `detectors_run` column.
   *  Absent for rows written before schema v6. */
  detectorsRun?: string[];
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

// ===== Conformant Signing (BYOC) Types =====

/**
 * C2PA signing mode.
 *
 * `bedrock` — per-install local CA chain (offline-first, default).
 *   Produces manifests that show signingCredential.untrusted in external
 *   validators. This is intentional — the Jura Labs offline-first USP.
 *
 * `conformant` — institution-imported certificate from a CA on the C2PA
 *   trust list. Produces manifests that validate in Adobe Inspect and
 *   any conformant C2PA validator. Requires cert import via Settings.
 */
export type SigningMode = 'bedrock' | 'conformant';

/**
 * Metadata about an imported conformant certificate.
 *
 * Mirrors the Rust `ConformantCertificateInfo` struct in `src-tauri/src/c2pa.rs`.
 * All timestamps are ISO 8601 UTC strings.
 */
export interface ConformantCertificateInfo {
  /** Common Name of the end-entity certificate subject. */
  subjectCn: string;
  /** Common Name of the issuing CA (informational — see full chain for detail). */
  issuerCn: string;
  /** ISO 8601 not-before timestamp (certificate valid from). */
  notBefore: string;
  /** ISO 8601 not-after timestamp (certificate expiry). */
  notAfter: string;
  /** SHA-256 fingerprint of the end-entity DER, colon-separated lowercase hex pairs. */
  fingerprintSha256: string;
  /** Signing algorithm (e.g. `"ECDSA-P256-SHA256"`). */
  signingAlgorithm: string;
  /** Key usage flags present on the certificate (e.g. `["DigitalSignature"]`). */
  keyUsage: string[];
  /** Extended key usage friendly names (e.g. `["emailProtection"]`). */
  extendedKeyUsage: string[];
  /** Whether the certificate is currently valid (not expired, not yet-valid). */
  isCurrentlyValid: boolean;
  /** ISO 8601 timestamp when the certificate was imported into Jura Trace. */
  importedAt: string;
}

/** Supported file extensions by content type */
export const SUPPORTED_EXTENSIONS: Record<ContentType, string[]> = {
  image: ['.jpg', '.jpeg', '.png', '.tiff', '.tif', '.webp', '.heic', '.heif', '.bmp', '.gif', '.svg', '.avif', '.ico'],
  document: ['.pdf', '.docx', '.odt', '.epub', '.txt', '.rtf', '.html', '.md', '.csv', '.xlsx'],
  video: ['.mp4', '.mov', '.webm', '.avi', '.mkv', '.m4v'],
  audio: ['.wav', '.mp3', '.flac', '.ogg', '.aac', '.m4a', '.aiff', '.opus'],
  unknown: [],
};
