"""
Jura Archive Sidecar — Health check endpoint.
"""

import httpx
from fastapi import APIRouter

from app.config import settings
from app.models.schemas import CapabilitiesResponse, HealthResponse

router = APIRouter()


@router.get("/health", response_model=HealthResponse)
async def health() -> HealthResponse:
    """Health check with capability declaration and Ollama connectivity."""
    ollama_status: str | None = None

    try:
        async with httpx.AsyncClient(timeout=2.0) as client:
            resp = await client.get(f"{settings.ollama_base_url}/api/tags")
            ollama_status = "available" if resp.status_code == 200 else "unavailable"
    except Exception:
        ollama_status = "unavailable"

    # Check if CLIP model is available
    clip_available = False
    try:
        from app.services.clip_detector import _ensure_model
        clip_available = _ensure_model()
    except Exception:
        pass

    # RAG claim checking is available whenever Ollama is reachable.
    rag_available = ollama_status == "available"

    return HealthResponse(
        status="ok",
        version="0.2.0",
        service="jura-sidecar",
        capabilities=CapabilitiesResponse(
            ela=True, noise=True, copy_move=True, deepfake=True,
            clip_detect=clip_available, rag=rag_available,
        ),
        ollama=ollama_status,
    )
