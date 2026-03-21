"""
Jura Trace Sidecar — Health check endpoint.
"""

import shutil

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

    # FFmpeg availability gates video/audio services.
    ffmpeg_available = shutil.which("ffprobe") is not None

    # Check if faster-whisper is available for transcription.
    whisper_available = False
    try:
        from app.services.transcription import is_whisper_available
        whisper_available = is_whisper_available()
    except Exception:
        pass

    return HealthResponse(
        status="ok",
        version="0.2.0",
        service="jura-trace-sidecar",
        capabilities=CapabilitiesResponse(
            ela=True, noise=True, copy_move=True, deepfake=True,
            watermark=True, clip_detect=clip_available, rag=rag_available,
            video_metadata=ffmpeg_available,
            audio_metadata=ffmpeg_available,
            video_frames=ffmpeg_available,
            video_deepfake=ffmpeg_available,
            transcription=whisper_available,
        ),
        ollama=ollama_status,
    )
