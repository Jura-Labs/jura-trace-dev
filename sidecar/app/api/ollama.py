"""
Jura Trace Sidecar — Ollama proxy endpoint.

Proxies model pull requests to the locally-configured Ollama instance so
that the Tauri webview (which is restricted to connect-src 127.0.0.1:8200
by CSP) does not need a direct connection to Ollama.

Supports both streaming (progress updates) and non-streaming modes.
"""

import json
import logging

import httpx
from fastapi import APIRouter
from fastapi.responses import JSONResponse, StreamingResponse
from pydantic import BaseModel

from app.config import settings

logger = logging.getLogger(__name__)

router = APIRouter()

_PULL_TIMEOUT_SECONDS = 600.0


class PullRequest(BaseModel):
    """Body accepted by POST /ollama/pull."""

    name: str
    stream: bool = True


@router.post("/ollama/pull")
async def pull_model(body: PullRequest):
    """
    Proxy a model pull request to the Ollama instance.

    With stream=true (default), returns Server-Sent Events with progress:
      data: {"status":"pulling","completed":1234567,"total":4700000000}

    With stream=false, blocks until complete and returns the final status.
    """
    target = f"{settings.ollama_base_url}/api/pull"
    logger.info("Proxying model pull for %r to %s (stream=%s)", body.name, target, body.stream)

    if body.stream:
        return StreamingResponse(
            _stream_pull(body.name, target),
            media_type="text/event-stream",
            headers={
                "Cache-Control": "no-cache",
                "X-Accel-Buffering": "no",
            },
        )

    # Non-streaming fallback
    try:
        async with httpx.AsyncClient(timeout=_PULL_TIMEOUT_SECONDS) as client:
            resp = await client.post(target, json={"name": body.name, "stream": False})
    except httpx.ConnectError:
        return JSONResponse(status_code=502, content={
            "error": "ollama_unreachable",
            "message": f"Could not connect to Ollama at {settings.ollama_base_url}.",
        })
    except httpx.TimeoutException:
        return JSONResponse(status_code=504, content={
            "error": "pull_timeout",
            "message": f"Model pull timed out after {int(_PULL_TIMEOUT_SECONDS // 60)} minutes.",
        })

    try:
        body_json = resp.json()
    except Exception:
        body_json = {"raw": resp.text}
    return JSONResponse(status_code=resp.status_code, content=body_json)


async def _stream_pull(model_name: str, target: str):
    """Stream Ollama pull progress as SSE events."""
    try:
        async with httpx.AsyncClient(timeout=_PULL_TIMEOUT_SECONDS) as client:
            async with client.stream(
                "POST", target, json={"name": model_name, "stream": True}
            ) as resp:
                async for line in resp.aiter_lines():
                    if not line.strip():
                        continue
                    try:
                        data = json.loads(line)
                        yield f"data: {json.dumps(data)}\n\n"
                    except json.JSONDecodeError:
                        continue

        yield f"data: {json.dumps({'status': 'success'})}\n\n"
    except httpx.ConnectError:
        yield f"data: {json.dumps({'error': 'ollama_unreachable', 'message': f'Could not connect to Ollama at {settings.ollama_base_url}'})}\n\n"
    except httpx.TimeoutException:
        yield f"data: {json.dumps({'error': 'pull_timeout', 'message': 'Download timed out'})}\n\n"
    except Exception as exc:
        yield f"data: {json.dumps({'error': 'proxy_error', 'message': str(exc)})}\n\n"
