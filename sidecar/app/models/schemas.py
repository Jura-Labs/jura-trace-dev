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


class CapabilitiesResponse(BaseModel):
    """Sidecar capability flags."""

    ela: bool = True
    deepfake: bool = False
    rag: bool = False


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    version: str
    service: str
    capabilities: CapabilitiesResponse
    ollama: str | None = None
