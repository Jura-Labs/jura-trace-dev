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


class CapabilitiesResponse(BaseModel):
    """Sidecar capability flags."""

    ela: bool = True
    noise: bool = True
    copy_move: bool = True
    deepfake: bool = False
    rag: bool = False


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    version: str
    service: str
    capabilities: CapabilitiesResponse
    ollama: str | None = None
