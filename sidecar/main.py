# SPDX-License-Identifier: AGPL-3.0-or-later

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
import os
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request, Response
from fastapi.middleware.cors import CORSMiddleware

from app.api import content_type, forensics, health, ollama
from app.config import settings

logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(application: FastAPI):
    """
    Warm up expensive model artefacts at startup so the first real
    verification request does not pay the cold-start loading cost.

    Currently warms up:
    - The GBM deepfake classifier (joblib model file, ~50 ms if present)
    - HEIC/HEIF codec registration via pillow-heif (Linux build hardening;
      iPhone photos are the most common pilot input)
    - CLIP probe availability (cached on application.state so /health does
      not pay a per-request cold-import cost — JTV-142 fix 3, 2026-05-02).
    """
    try:
        import pillow_heif  # noqa: PLC0415

        pillow_heif.register_heif_opener()
        logger.info("HEIC/HEIF codec registered (pillow-heif present)")
    except ImportError:
        logger.critical(
            "pillow-heif unavailable — .heic / .heif files will fail to "
            "decode. Add pillow-heif to the runtime environment "
            "(it is in requirements.txt and requirements-ci.txt)."
        )

    # AVIF codec (added rc.31, 2026-06-09): pillow-heif 1.2.0 does not include
    # AVIF support, and several published AI-generated images circulate as AVIF
    # (Twitter/X stripping, Discord re-encoding, Cloudflare Image Resizing).
    # Without this opener, AVIF files reach detectors as opaque bytes; six
    # detectors return HTTP 400 'cannot identify image file' and four silently
    # return score=0 ('clean'), inflating the composite trust score to a
    # false-pass on known fakes.
    try:
        import pillow_avif  # noqa: F401,PLC0415 — auto-registers AvifImagePlugin

        logger.info("AVIF codec registered (pillow-avif-plugin present)")
    except ImportError:
        logger.warning(
            "pillow-avif-plugin unavailable — .avif files will fail to decode. "
            "Add pillow-avif-plugin to the runtime environment "
            "(it is in requirements.txt and requirements-ci.txt). "
            "Non-fatal: AVIF is rarer than HEIC but increasingly common in "
            "social-media-forwarded images."
        )

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

    # JTV-142 fix 3 (2026-05-02): probe CLIP once at startup and cache the
    # result on application.state so the per-request /health handler does not
    # repeatedly re-import scikit-image / sklearn from a cold _MEIPASS dir.
    # When the Rust readiness poller fires a probe every 200-1600 ms during
    # the startup window, this cache prevents the lazy-load from serialising
    # all probes and exhausting the Rust readiness budget.
    application.state.clip_available = False
    try:
        from app.services.clip_detector import _ensure_model  # noqa: PLC0415

        application.state.clip_available = bool(_ensure_model())
        logger.info(
            "CLIP probe availability cached: %s",
            application.state.clip_available,
        )
    except Exception as exc:  # noqa: BLE001
        logger.warning("CLIP availability probe failed (non-fatal): %s", exc)

    # Mark the application ready *after* all warmup is done. /health/ready
    # reads this flag and short-circuits without touching any service module.
    application.state.ready = True
    logger.info("Sidecar ready — /health/ready will now return 200")

    yield  # application runs here


# Disable interactive API documentation endpoints in all deployments.
# The sidecar is a local process interface, not a public API; exposing
# schema documentation to any local process is unnecessary attack surface.
app = FastAPI(
    title="Jura Trace ML Sidecar",
    version="0.9.0",
    description="Local ML services for content forensics and verification",
    docs_url=None,
    redoc_url=None,
    openapi_url=None,
    lifespan=lifespan,
)


# CORS: allow the Tauri webview (tauri://localhost, https://tauri.localhost,
# http://localhost:1420) to call the sidecar. Without this, the browser sends
# an OPTIONS preflight that returns 405 and the actual POST never fires.
app.add_middleware(
    CORSMiddleware,
    allow_origins=[
        "tauri://localhost",
        "https://tauri.localhost",
        "http://tauri.localhost",
        "http://localhost:1420",
        "http://127.0.0.1:1420",
    ],
    allow_methods=["GET", "POST"],
    allow_headers=["X-Jura-API-Key", "Content-Type"],
)


async def verify_api_key(request: Request, call_next: object) -> Response:
    """
    Enforce shared-secret authentication on all /forensics/* endpoints.

    The /health endpoint is exempt so that availability probes issued by
    the Rust backend before the key is known continue to work.

    Security: an empty `settings.sidecar_key` previously bypassed
    authentication entirely (truthy-string check). That made the sidecar
    openly accessible on localhost whenever the env var was unset. In
    production builds the env var is always set (the Tauri parent auto-
    generates one before spawning the sidecar — see lib.rs setup), but
    a misconfigured dev launch or a wrapper script that drops the var
    would silently disable auth. Tightened on 2026-05-21: an empty key
    now rejects all /forensics requests rather than allowing them. Dev
    workflows that need anonymous access should run with an explicit
    test key (e.g. `JURA_SIDECAR_KEY=dev` uvicorn main:app).
    """
    if request.url.path.startswith("/forensics"):
        if not settings.sidecar_key:
            # pytest sets PYTEST_CURRENT_TEST per-test so a missing key under
            # the test runner is unambiguous and safe to bypass. Outside the
            # test runner an empty key now refuses requests (was the silent
            # auth-bypass vulnerability fixed on 2026-05-21).
            if "PYTEST_CURRENT_TEST" in os.environ:
                return await call_next(request)
            return Response(
                content="Sidecar key not configured. Set JURA_SIDECAR_KEY.",
                status_code=503,
                media_type="text/plain",
            )
        provided = request.headers.get("X-Jura-API-Key", "")
        if provided != settings.sidecar_key:
            return Response(
                content="Unauthorised",
                status_code=401,
                media_type="text/plain",
            )
    return await call_next(request)


# Register the middleware (separated from the def above so the @decorator
# doesn't interleave with the long docstring rationale).
app.middleware("http")(verify_api_key)


app.include_router(health.router, tags=["health"])
app.include_router(forensics.router, prefix="/forensics", tags=["forensics"])
app.include_router(
    content_type.router, prefix="/forensics", tags=["content-type"]
)
# Ollama proxy — unauthenticated (called during setup wizard before API keys exist).
# The auth middleware already exempts paths that don't start with /forensics, so
# no special exemption is needed here.
app.include_router(ollama.router, tags=["ollama"])

if __name__ == "__main__":
    import argparse
    import uvicorn

    # Rust spawn_sidecar (src-tauri/src/lib.rs) launches this binary with
    # `--host 127.0.0.1 --port 8200`. Until 2026-05-02 those arguments were
    # silently ignored because the entrypoint hard-coded the bind address;
    # the Rust side and Python side happened to agree on 8200, so the bug
    # was invisible. Parsing the args makes the contract explicit and lets
    # operators run multiple sidecars on different ports during diagnosis.
    parser = argparse.ArgumentParser(description="Jura Trace ML sidecar")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8200)
    parser.add_argument("--log-level", default="info")
    args = parser.parse_args()

    uvicorn.run(app, host=args.host, port=args.port, log_level=args.log_level)
