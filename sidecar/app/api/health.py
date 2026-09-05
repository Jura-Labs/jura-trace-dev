# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Health check endpoints.

Two endpoints are provided:

* ``GET /health/ready`` — constant-time bool read of an application.state
  flag set during the FastAPI lifespan. The Rust startup readiness poller
  hits this endpoint with exponential back-off; it must remain cheap so a
  cold-start CLIP / scikit-image lazy import never serialises probes and
  exhausts the readiness budget. JTV-142 fix 3, 2026-05-02.

* ``GET /health`` — full capability JSON consumed by the frontend. Reads
  the CLIP availability flag cached by the lifespan (no per-request lazy
  import). Probes Ollama with a 2 s timeout. v1.0 (JTV-138, 2 May 2026)
  reports the audio/video capability fields as ``False`` because the Rust
  pipeline gates those code paths off; the fields stay in the schema for
  v1.0.x re-add (JTV-139).
"""

import httpx
from fastapi import APIRouter, Request, Response

from app.config import settings
from app.models.schemas import CapabilitiesResponse, HealthResponse

router = APIRouter()


@router.get("/health/ready")
async def health_ready(request: Request) -> Response:
    """
    Lightweight readiness probe for the Rust startup poller.

    Returns 200 once :pyfunc:`lifespan` has finished warmup and set
    ``application.state.ready = True``. Returns 503 otherwise. Importantly,
    this handler does **not** import service modules or touch the Ollama
    base URL — it must be O(1) so the Rust poller's exponential back-off
    sees real success/failure transitions instead of CLIP-import latency.
    """
    if getattr(request.app.state, "ready", False):
        return Response(status_code=200, content="ready", media_type="text/plain")
    return Response(status_code=503, content="warming-up", media_type="text/plain")


@router.get("/health", response_model=HealthResponse)
async def health(request: Request) -> HealthResponse:
    """Health check with capability declaration and Ollama connectivity."""
    ollama_status: str | None = None
    ollama_models: list[str] | None = None

    try:
        async with httpx.AsyncClient(timeout=2.0) as client:
            resp = await client.get(f"{settings.ollama_base_url}/api/tags")
            if resp.status_code == 200:
                ollama_status = "available"
                try:
                    data = resp.json()
                    ollama_models = [
                        m["name"] for m in data.get("models", []) if "name" in m
                    ]
                except Exception:
                    ollama_models = []
            else:
                ollama_status = "unavailable"
    except Exception:
        ollama_status = "unavailable"

    # JTV-142 fix 3 (2026-05-02): read the CLIP probe result cached during
    # FastAPI lifespan warmup instead of re-running _ensure_model() on every
    # health probe. Avoids a per-request scikit-image / sklearn cold import
    # from _MEIPASS that previously stalled the Rust startup poller.
    clip_available = bool(getattr(request.app.state, "clip_available", False))

    # Knowledge Base Retrieval (RAG claim checker) deferred from v1.0
    # on 2026-05-17 pending corpus expansion and formal accuracy evaluation.
    # The Python service code remains for re-enablement in a future release;
    # the capability flag is forced to false so the desktop UI hides the
    # claims card and the REST API does not advertise the feature.
    rag_available = False

    # JTV-138 (2026-05-02): the Rust pipeline gates the video and audio
    # sidecar groups off, so even where ffprobe is on PATH those endpoints
    # are unreachable. video_metadata and audio_metadata below are therefore
    # reported False unconditionally.
    #
    # There is NO ffmpeg capability on the wire. An `ffmpeg_available` probe
    # was computed here and discarded, under a comment claiming the
    # capability "is still reported truthfully" so the Setup Wizard could
    # preview readiness. It never was: CapabilitiesResponse has no such
    # field. The dead probe is removed rather than left to imply otherwise.
    # Adding the field is a JTV-139 decision, since it changes the API
    # contract and the frontend, and is not a lint fix.

    return HealthResponse(
        status="ok",
        # Read from the FastAPI app rather than a literal. This drifted once:
        # main.py was bumped to 1.0.0 for the June release and this stayed at
        # "0.9.0", so every shipped sidecar reported the wrong version to the
        # desktop app, the REST API and the Setup Wizard.
        version=request.app.version,
        service="jura-trace-sidecar",
        capabilities=CapabilitiesResponse(
            ela=True,
            noise=True,
            copy_move=True,
            deepfake=True,
            # watermark deferred to v1.1 (V1_SHOW_WATERMARK=false in frontend).
            # invisible-watermark removed from CI bundle 2026-05-21 to keep
            # the JTV-184 onedir size envelope safe. Surface stays false so
            # callers (CLI, Tauri command, REST API) reflect the bundle state.
            watermark=False,
            clip_detect=clip_available,
            rag=rag_available,
            # JTV-138 v1.0 drop — these stay False until JTV-139 re-add.
            video_metadata=False,
            audio_metadata=False,
            video_frames=False,
            video_deepfake=False,
            transcription=False,
        ),
        ollama=ollama_status,
        ollama_models=ollama_models,
    )
