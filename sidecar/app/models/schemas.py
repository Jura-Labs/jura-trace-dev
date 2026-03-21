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


class CaResponse(BaseModel):
    """Chromatic Aberration consistency analysis result."""

    r_squared: float
    is_consistent: bool
    score: float
    suspicious: bool
    sample_count: int
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
    """A single claim verification result."""

    claim: str
    verdict: str  # "supported", "disputed", "unverified", "unavailable"
    explanation: str
    confidence: float  # 0.0–1.0


class ClaimCheckResponse(BaseModel):
    """RAG claim verification result."""

    overall_verdict: str  # "supported", "disputed", "unverified", "mixed", "unavailable"
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


class CapabilitiesResponse(BaseModel):
    """Sidecar capability flags."""

    ela: bool = True
    noise: bool = True
    copy_move: bool = True
    deepfake: bool = True
    jpeg_ghost: bool = True
    npr: bool = True
    chromatic_aberration: bool = True
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


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    version: str
    service: str
    capabilities: CapabilitiesResponse
    ollama: str | None = None
