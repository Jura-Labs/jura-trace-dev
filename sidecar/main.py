"""
Jura Trace — Python ML Sidecar

Provides image forensics, deepfake detection, and RAG pipeline
capabilities that require Python ML libraries.

Usage:
    uvicorn main:app --host 127.0.0.1 --port 8200 --reload

Authentication:
    Set the JURA_SIDECAR_KEY environment variable to a shared secret.
    All /forensics/* requests must then include the header:
        X-Jura-API-Key: <value>
    The /health endpoint is always unauthenticated (used by availability probes).
    If JURA_SIDECAR_KEY is empty (default), no authentication is enforced.
"""

import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request, Response

from app.api import forensics, health
from app.config import settings

logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(application: FastAPI):
    """
    Warm up expensive model artefacts at startup so the first real
    verification request does not pay the cold-start loading cost.

    Currently warms up:
    - The GBM deepfake classifier (joblib model file, ~50 ms if present)
    """
    try:
        from app.services.deepfake import _load_classifier  # noqa: PLC0415

        clf = _load_classifier()
        if clf is not None:
            logger.info("Deepfake classifier warmed up at startup")
        else:
            logger.info(
                "Deepfake classifier not available (model file absent — skipping warmup)"
            )
    except Exception as exc:  # noqa: BLE001
        logger.warning("Deepfake classifier warmup failed (non-fatal): %s", exc)

    yield  # application runs here


# Disable interactive API documentation endpoints in all deployments.
# The sidecar is a local process interface, not a public API; exposing
# schema documentation to any local process is unnecessary attack surface.
app = FastAPI(
    title="Jura Trace ML Sidecar",
    version="0.2.0",
    description="Local ML services for content forensics and verification",
    docs_url=None,
    redoc_url=None,
    openapi_url=None,
    lifespan=lifespan,
)


@app.middleware("http")
async def verify_api_key(request: Request, call_next: object) -> Response:
    """
    Enforce shared-secret authentication on all /forensics/* endpoints.

    The /health endpoint is exempt so that availability probes issued by
    the Rust backend before the key is known continue to work.

    If settings.sidecar_key is empty the middleware is a no-op, allowing
    unauthenticated development use.
    """
    if settings.sidecar_key and request.url.path.startswith("/forensics"):
        provided = request.headers.get("X-Jura-API-Key", "")
        if provided != settings.sidecar_key:
            return Response(
                content="Unauthorised",
                status_code=401,
                media_type="text/plain",
            )
    return await call_next(request)


app.include_router(health.router, tags=["health"])
app.include_router(forensics.router, prefix="/forensics", tags=["forensics"])

if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="127.0.0.1", port=8200, log_level="info")
