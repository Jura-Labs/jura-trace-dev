"""
Jura Trace Sidecar — Pydantic schemas for API request/response models.
"""

from typing import Optional

from pydantic import BaseModel


class ElaResponse(BaseModel):
    """Error Level Analysis result."""

    ela_image_base64: str
    max_difference: float
    mean_difference: float
    score: float
    suspicious: bool


class NoiseAnalysisResponse(BaseModel):
    """Block-wise noise variance analysis result."""

    heatmap_base64: str
    block_variances: list[float]
    global_variance: float
    anomalous_blocks: int
    total_blocks: int
    score: float
    suspicious: bool


class CloneRegion(BaseModel):
    """A detected clone region bounding box."""

    x: int
    y: int
    width: int
    height: int
    area: int
    point_count: int


class CopyMoveResponse(BaseModel):
    """Copy-move forgery detection result."""

    visualisation_base64: str
    clone_regions: list[dict]
    matched_pairs: int
    score: float
    suspicious: bool


class DeepfakeSignal(BaseModel):
    """A single signal from the deepfake detection ensemble."""

    name: str
    description: str
    weight: float
    triggered: bool


class WatermarkDetection(BaseModel):
    """A detected invisible watermark from an AI image generator."""

    type: str
    detected: bool
    confidence: float
    details: str


class DeepfakeResponse(BaseModel):
    """Deepfake / AI-generated image detection result."""

    score: float
    suspicious: bool
    confidence: str
    verdict_level: str
    signals: list[DeepfakeSignal]
    heatmap_base64: str
    summary: str
    watermarks: list[WatermarkDetection] = []
    classifier_score: float | None = None
    classifier_available: bool = False
    univfd_score: float | None = None
    univfd_available: bool = False


class JpegGhostResponse(BaseModel):
    """JPEG ghost detection result for splice/composite forgery analysis."""

    score: float
    suspicious: bool
    ghost_quality: int
    quality_variance: float
    deviating_blocks: int
    total_blocks: int
    heatmap_base64: str
    summary: str


class NprResponse(BaseModel):
    """Neighbouring Pixel Relationship (NPR) analysis result."""

    score: float
    suspicious: bool
    hv_correlation: float
    diff_variance_ratio: float
    hf_energy_ratio: float
    heatmap_base64: str
    summary: str


class ClipDetectionResponse(BaseModel):
    """CLIP-based AI image detection result."""

    score: float
    suspicious: bool
    verdict_level: str
    confidence: str
    class_probabilities: dict[str, float]
    model_name: str
    model_available: bool
    summary: str
    univfd_score: float | None = None
    univfd_available: bool = False


class ClaimVerdict(BaseModel):
    """A single knowledge-base retrieval match result.

    Despite the class name (retained for backwards-compatible JSON field
    order), this is NOT a verdict about the truth or falsity of a claim.
    It reports whether the analyst-entered text is consistent with the
    retrieved passages from a small preliminary reference corpus. See
    the model card at /help/model-cards#kb-retrieval for scope.
    """

    claim: str
    # One of: "consistent_with_kb", "inconsistent_with_kb",
    # "insufficient_context_in_kb", "unavailable".
    # Legacy builds may still produce "supported" / "disputed" /
    # "unverified" — callers should treat these as aliases for
    # consistent / inconsistent / insufficient respectively.
    verdict: str
    explanation: str
    confidence: float  # 0.0–1.0 — retrieval-match certainty, NOT factual certainty


class ClaimCheckResponse(BaseModel):
    """Knowledge base retrieval match result (preliminary investigative aid).

    This is NOT a fact-checking tool. It reports whether analyst-entered
    text is consistent with a small preliminary reference corpus. Results
    must not be cited as authority for the truth or falsity of any claim.
    See the model card at /help/model-cards#kb-retrieval.
    """

    # One of: "consistent_with_kb", "inconsistent_with_kb",
    # "insufficient_context_in_kb", "mixed_kb_match", "unavailable".
    overall_verdict: str
    claims: list[ClaimVerdict]
    model_used: str
    methodology: str
    summary: str


class ElaRegion(BaseModel):
    """A single grid cell from segmented ELA analysis."""

    x: int
    y: int
    width: int
    height: int
    ela_score: float
    anomalous: bool


class SegmentedElaResponse(BaseModel):
    """Segmented Error Level Analysis result."""

    heatmap_base64: Optional[str] = None
    regions: list[ElaRegion]
    anomalous_regions: int
    total_regions: int
    inter_region_variance: float
    score: float
    suspicious: bool
    summary: str


class ShadowRegion(BaseModel):
    """A foreground component from shadow consistency analysis."""

    x: int
    y: int
    width: int
    height: int
    area: int
    gradient_angle_mean: float
    deviation_from_global: float
    inconsistent: bool


class ShadowConsistencyResponse(BaseModel):
    """Shadow/lighting direction consistency analysis result."""

    heatmap_base64: Optional[str] = None
    global_light_direction: float
    regions: list[ShadowRegion]
    inconsistent_regions: int
    total_regions: int
    score: float
    suspicious: bool
    summary: str


class ColourTempRegion(BaseModel):
    """A grid cell from colour temperature analysis."""

    x: int
    y: int
    width: int
    height: int
    mean_a: float
    mean_b: float
    deviation_from_global: float
    anomalous: bool


class ColourTemperatureResponse(BaseModel):
    """Colour temperature segmentation analysis result."""

    heatmap_base64: Optional[str] = None
    regions: list[ColourTempRegion]
    anomalous_regions: int
    total_regions: int
    global_mean_a: float
    global_mean_b: float
    score: float
    suspicious: bool
    summary: str


class SpliceBoundary(BaseModel):
    """A detected potential splice boundary."""

    x: int
    y: int
    width: int
    height: int
    jpeg_grid_aligned: bool
    noise_asymmetric: bool
    feathering_detected: bool
    signals_triggered: int
    confidence: float


class SpliceBoundaryResponse(BaseModel):
    """Splice boundary detection result."""

    heatmap_base64: Optional[str] = None
    boundaries: list[SpliceBoundary]
    suspicious_boundaries: int
    total_boundaries_checked: int
    score: float
    suspicious: bool
    summary: str


class WatermarkEmbedResponse(BaseModel):
    """Invisible watermark embedding result."""

    watermarked_image_base64: str | None = None
    algorithm: str
    strength: str
    payload_length: int
    success: bool
    message: str


class WatermarkExtractResponse(BaseModel):
    """Invisible watermark extraction result."""

    extracted_payload: str | None = None
    extracted_hex: str | None = None
    payload_length: int
    algorithm: str
    has_watermark: bool
    confidence: float
    success: bool
    message: str


class VideoMetadataResponse(BaseModel):
    """Video metadata extraction result."""

    duration: float | None = None
    codec: str | None = None
    width: int | None = None
    height: int | None = None
    fps: float | None = None
    has_audio: bool = False
    audio_codec: str | None = None
    bitrate: int | None = None
    file_size: int | None = None
    success: bool
    message: str


class AudioMetadataResponse(BaseModel):
    """Audio metadata extraction result."""

    duration: float | None = None
    codec: str | None = None
    sample_rate: int | None = None
    channels: int | None = None
    bitrate: int | None = None
    file_size: int | None = None
    success: bool
    message: str


class VideoFramesResponse(BaseModel):
    """Video frame extraction result."""

    frames: list[str] = []  # base64 JPEG strings
    count: int = 0
    duration: float | None = None
    success: bool
    message: str


class FrameDeepfakeResult(BaseModel):
    """Per-frame deepfake analysis result within a video."""

    frame_index: int
    timestamp: float = 0.0
    score: float
    suspicious: bool
    verdict_level: str  # "authentic" | "inconclusive" | "synthetic"
    signals: list[DeepfakeSignal]
    classifier_score: float | None = None
    classifier_available: bool = False
    heatmap_base64: str = ""


class VideoDeepfakeResponse(BaseModel):
    """Video-level deepfake analysis result aggregated from per-frame scoring."""

    frame_results: list[FrameDeepfakeResult]
    aggregate_score: float
    aggregate_verdict: str  # "authentic" | "inconclusive" | "synthetic"
    aggregate_confidence: str  # "high" | "medium" | "low"
    frames_analysed: int
    frames_requested: int
    frames_skipped: int = 0
    temporal_available: bool
    temporal_noise_drift: float | None = None
    temporal_spectral_drift: float | None = None
    temporal_lbp_drift: float | None = None
    mode: str
    duration: float | None = None
    success: bool
    message: str


class TranscriptionSegment(BaseModel):
    """A single timestamped segment from speech transcription."""

    start: float  # seconds
    end: float
    text: str


class TranscriptionResponse(BaseModel):
    """Audio/video speech transcription result."""

    text: str  # full transcription
    segments: list[TranscriptionSegment] = []
    language: str | None = None
    language_probability: float | None = None
    duration: float | None = None
    model_size: str = "base"
    success: bool
    message: str


class ImageDescribeResponse(BaseModel):
    """AI-generated natural-language description of an image via Ollama LLaVA."""

    description: str | None = None
    model_used: str
    success: bool
    message: str


class CapabilitiesResponse(BaseModel):
    """Sidecar capability flags."""

    ela: bool = True
    noise: bool = True
    copy_move: bool = True
    deepfake: bool = True
    jpeg_ghost: bool = True
    npr: bool = True
    segmented_ela: bool = True
    shadow_consistency: bool = True
    colour_temperature: bool = True
    splice_boundary: bool = True
    watermark: bool = True
    clip_detect: bool = False
    rag: bool = False
    video_metadata: bool = False
    audio_metadata: bool = False
    video_frames: bool = False
    video_deepfake: bool = False
    transcription: bool = False
    audio_deepfake: bool = False


class AudioDeepfakeResponse(BaseModel):
    """Audio deepfake detection result (Sprint 35 two-stage ensemble)."""

    score: float | None = None
    """Ensemble score 0-1 (0 = authentic, 1 = synthetic).  None when no probe is loaded."""

    verdict: str = "model_not_loaded"
    """One of: ``authentic``, ``inconclusive``, ``likely_synthetic``, ``model_not_loaded``."""

    stage1_score: float | None = None
    """Stage 1 score from MFCC + GradientBoostingClassifier.  None when unavailable."""

    stage2_score: float | None = None
    """Stage 2 score from Wav2Vec2-Base + LogisticRegression.  None when unavailable."""

    stages_available: list[str] = []
    """Which stages actually ran, e.g. ``["stage1"]`` or ``["stage1", "stage2"]``."""

    model_loaded: bool = False
    """False until trained probe files are deployed to ``models/``."""

    duration_seconds: float | None = None
    """Audio duration in seconds, if the file could be loaded."""

    sample_rate: int | None = None
    """Sample rate after resampling (always 16 000 Hz when librosa is available)."""

    mfcc_features_extracted: bool = False
    """True when a 160-dim MFCC feature vector was successfully extracted."""

    wav2vec2_embedding_extracted: bool = False
    """True when a 768-dim Wav2Vec2 embedding was successfully extracted."""

    processing_time_ms: float | None = None
    """Wall-clock time for the full ensemble call in milliseconds."""


class DctAnalysisResponse(BaseModel):
    """8×8 block DCT coefficient map analysis result."""

    heatmap_base64: str
    dc_std: float
    ac_mean: float
    ac_std: float
    ac_coefficient_of_variation: float
    suspicious: bool
    score: float
    summary: str


class FourierAnalysisResponse(BaseModel):
    """2D FFT periodic pattern detection result."""

    spectrum_base64: str
    peak_count: int
    suspicious: bool
    score: float
    summary: str


class PlatformCandidate(BaseModel):
    """A candidate social media platform match."""

    platform: str
    confidence: float


class PlatformFingerprintResponse(BaseModel):
    """Social media re-upload platform fingerprinting result."""

    detected: bool
    platform: str | None = None
    confidence: float
    allCandidates: list[PlatformCandidate] = []
    maxDimension: int | None = None
    estimatedQuality: int | None = None
    hasExif: bool
    summary: str


class EnfAnalysisResponse(BaseModel):
    """Audio ENF (Electrical Network Frequency) analysis result."""

    detected: bool
    meanFrequency: float | None = None
    frequencyStd: float | None = None
    snr: float | None = None
    gridRegion: str | None = None
    expectedFrequency: float
    durationSeconds: float | None = None
    sampleCount: int = 0
    frequencyTrace: list[float] = []
    summary: str


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    version: str
    service: str
    capabilities: CapabilitiesResponse
    ollama: str | None = None
    ollama_models: list[str] | None = None
