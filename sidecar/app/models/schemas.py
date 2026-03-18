"""
Jura Archive Sidecar — Pydantic schemas for API request/response models.
"""

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


class CapabilitiesResponse(BaseModel):
    """Sidecar capability flags."""

    ela: bool = True
    noise: bool = True
    copy_move: bool = True
    deepfake: bool = True
    jpeg_ghost: bool = True
    npr: bool = True
    chromatic_aberration: bool = True
    clip_detect: bool = False
    rag: bool = False


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    version: str
    service: str
    capabilities: CapabilitiesResponse
    ollama: str | None = None
