"""
Jura Trace Sidecar — Ollama proxy endpoint.

Proxies model pull requests to the locally-configured Ollama instance so
that the Tauri webview (which is restricted to connect-src 127.0.0.1:8200
by CSP) does not need a direct connection to Ollama.  This is especially
important for remote Ollama instances (e.g. http://192.168.1.100:11434)
where a direct fetch from the webview would be CSP-blocked.

The endpoint is intentionally unauthenticated: it is called during the
setup wizard before the user has configured any API keys, and it only ever
forwards requests to the Ollama instance that is already trusted by the
sidecar configuration.
"""

import logging

import httpx
from fastapi import APIRouter
from fastapi.responses import JSONResponse
from pydantic import BaseModel

from app.config import settings

logger = logging.getLogger(__name__)

router = APIRouter()

# Ollama model pulls can take several minutes for large models (LLaVA 7B is
# ~4 GB).  Use a generous timeout so the pull is not prematurely abandoned.
_PULL_TIMEOUT_SECONDS = 600.0


class PullRequest(BaseModel):
    """Body accepted by POST /ollama/pull."""

    name: str


@router.post("/ollama/pull")
async def pull_model(body: PullRequest) -> JSONResponse:
    """
    Proxy a model pull request to the Ollama instance configured via
    JURA_OLLAMA_BASE_URL (default: http://localhost:11434).

    Accepts:  {"name": "llava:7b"}
    Returns:  the Ollama JSON response (status "success") or an error body.

    The upstream Ollama /api/pull endpoint blocks until the pull is complete
    when called with stream=false, which is what we want here: the wizard
    can await this call and then refresh the health check to confirm the
    model is listed.
    """
    target = f"{settings.ollama_base_url}/api/pull"
    logger.info("Proxying model pull for %r to %s", body.name, target)

    try:
        async with httpx.AsyncClient(timeout=_PULL_TIMEOUT_SECONDS) as client:
            resp = await client.post(
                target,
                json={"name": body.name, "stream": False},
            )
    except httpx.ConnectError as exc:
        logger.warning("Ollama not reachable at %s: %s", settings.ollama_base_url, exc)
        return JSONResponse(
            status_code=502,
            content={
                "error": "ollama_unreachable",
                "message": (
                    f"Could not connect to Ollama at {settings.ollama_base_url}. "
                    "Please ensure Ollama is running and try again."
                ),
            },
        )
    except httpx.TimeoutException:
        logger.warning("Ollama pull timed out after %s s", _PULL_TIMEOUT_SECONDS)
        return JSONResponse(
            status_code=504,
            content={
                "error": "pull_timeout",
                "message": (
                    f"Model pull for '{body.name}' timed out after "
                    f"{int(_PULL_TIMEOUT_SECONDS // 60)} minutes. "
                    "The download may still be in progress — check Ollama directly."
                ),
            },
        )
    except Exception as exc:  # noqa: BLE001
        logger.error("Unexpected error proxying Ollama pull: %s", exc)
        return JSONResponse(
            status_code=500,
            content={"error": "proxy_error", "message": str(exc)},
        )

    # Forward whatever status code Ollama returned.
    try:
        body_json = resp.json()
    except Exception:  # noqa: BLE001
        body_json = {"raw": resp.text}

    logger.info(
        "Ollama pull for %r completed with status %d", body.name, resp.status_code
    )
    return JSONResponse(status_code=resp.status_code, content=body_json)
