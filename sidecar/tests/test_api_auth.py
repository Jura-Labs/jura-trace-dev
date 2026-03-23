"""
Tests for the sidecar API key authentication middleware (MEDIUM-1).

Verifies that:
- /health is exempt from authentication at all times.
- /forensics/* returns 401 when a key is configured and no header is provided.
- /forensics/* returns 401 when a key is configured and the wrong header is provided.
- /forensics/* proceeds normally when the correct key is provided.
- When no key is configured the middleware is a no-op.
"""

import pytest
from httpx import ASGITransport, AsyncClient

import main as main_module
from app.config import settings


@pytest.mark.asyncio
async def test_health_is_always_accessible():
    """/health must return 200 regardless of the API key configuration."""
    original = settings.sidecar_key
    try:
        settings.sidecar_key = "supersecret"
        transport = ASGITransport(app=main_module.app)
        async with AsyncClient(transport=transport, base_url="http://test") as client:
            response = await client.get("/health")
        assert response.status_code == 200
    finally:
        settings.sidecar_key = original


@pytest.mark.asyncio
async def test_forensics_blocked_without_key_when_auth_enabled():
    """/forensics/* should return 401 when no X-Jura-API-Key header is provided."""
    original = settings.sidecar_key
    try:
        settings.sidecar_key = "supersecret"
        transport = ASGITransport(app=main_module.app)
        async with AsyncClient(transport=transport, base_url="http://test") as client:
            response = await client.post("/forensics/ela")
        assert response.status_code == 401
    finally:
        settings.sidecar_key = original


@pytest.mark.asyncio
async def test_forensics_blocked_with_wrong_key():
    """/forensics/* should return 401 when an incorrect key is provided."""
    original = settings.sidecar_key
    try:
        settings.sidecar_key = "supersecret"
        transport = ASGITransport(app=main_module.app)
        async with AsyncClient(transport=transport, base_url="http://test") as client:
            response = await client.post(
                "/forensics/ela",
                headers={"X-Jura-API-Key": "wrongkey"},
            )
        assert response.status_code == 401
    finally:
        settings.sidecar_key = original


@pytest.mark.asyncio
async def test_forensics_allowed_with_correct_key():
    """
    /forensics/* should pass the auth check and reach the endpoint handler
    when the correct key is provided.  The endpoint itself may return a
    validation error (422 — no file uploaded) but must not return 401.
    """
    original = settings.sidecar_key
    try:
        settings.sidecar_key = "supersecret"
        transport = ASGITransport(app=main_module.app)
        async with AsyncClient(transport=transport, base_url="http://test") as client:
            response = await client.post(
                "/forensics/ela",
                headers={"X-Jura-API-Key": "supersecret"},
            )
        # 422 means the request reached the handler (no file supplied is fine here).
        # Anything other than 401 confirms auth passed.
        assert response.status_code != 401
    finally:
        settings.sidecar_key = original


@pytest.mark.asyncio
async def test_no_auth_when_key_not_configured():
    """/forensics/* should be accessible without a header when key is empty."""
    original = settings.sidecar_key
    try:
        settings.sidecar_key = ""
        transport = ASGITransport(app=main_module.app)
        async with AsyncClient(transport=transport, base_url="http://test") as client:
            response = await client.post("/forensics/ela")
        # 422 = reached the handler, auth middleware was a no-op.
        assert response.status_code != 401
    finally:
        settings.sidecar_key = original
