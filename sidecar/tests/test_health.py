"""Tests for the health endpoint."""

import pytest
from httpx import ASGITransport, AsyncClient

from main import app


@pytest.mark.asyncio
async def test_health_returns_200():
    """Health endpoint should return 200 OK."""
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        response = await client.get("/health")
    assert response.status_code == 200


@pytest.mark.asyncio
async def test_health_response_structure():
    """Health response should have required fields."""
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        response = await client.get("/health")
    data = response.json()
    assert data["status"] == "ok"
    assert data["version"] == "0.2.0"
    assert data["service"] == "jura-sidecar"
    assert "capabilities" in data
    assert data["capabilities"]["ela"] is True
    assert data["capabilities"]["deepfake"] is False
    assert data["capabilities"]["rag"] is False


@pytest.mark.asyncio
async def test_health_ollama_field():
    """Health response should include Ollama status."""
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as client:
        response = await client.get("/health")
    data = response.json()
    # Ollama likely unavailable in test environment
    assert data["ollama"] in ("available", "unavailable")
