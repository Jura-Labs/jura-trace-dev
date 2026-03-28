"""
Tests for the AI image description service.

Ollama is not required — all HTTP calls are mocked via unittest.mock.patch.
The test suite covers:
  - Graceful degradation when Ollama is unavailable
  - Graceful degradation when the LLaVA model is not pulled
  - Graceful degradation on Ollama timeout
  - Graceful degradation on unexpected HTTP errors
  - Positive path (mocked Ollama returning a description)
  - Empty description handling
  - The POST /forensics/describe HTTP endpoint
"""

from __future__ import annotations

import io
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from httpx import ASGITransport, AsyncClient
from PIL import Image

from app.models.schemas import ImageDescribeResponse
from app.services.describe_image import (
    _check_model_available,
    _check_ollama_available,
    describe_image,
)
from main import app


# ── Helpers ────────────────────────────────────────────────────────────────────


def _make_jpeg_bytes(width: int = 64, height: int = 64) -> bytes:
    """Create a minimal JPEG image as raw bytes."""
    img = Image.new("RGB", (width, height), color=(100, 149, 237))
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    return buf.getvalue()


# ── Graceful degradation: Ollama unavailable ──────────────────────────────────


class TestOllamaUnavailable:
    """describe_image must degrade gracefully when Ollama is not running."""

    @pytest.mark.asyncio
    async def test_returns_success_false_when_ollama_down(self):
        with patch(
            "app.services.describe_image._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            result = await describe_image(_make_jpeg_bytes())

        assert isinstance(result, ImageDescribeResponse)
        assert result.success is False
        assert result.description is None
        assert "Ollama" in result.message

    @pytest.mark.asyncio
    async def test_model_used_field_populated_when_ollama_down(self):
        with patch(
            "app.services.describe_image._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            result = await describe_image(_make_jpeg_bytes(), model="llava:7b")

        assert result.model_used == "llava:7b"


# ── Graceful degradation: model not pulled ────────────────────────────────────


class TestModelNotAvailable:
    """describe_image must degrade gracefully when the LLaVA model is not pulled."""

    @pytest.mark.asyncio
    async def test_returns_success_false_when_model_missing(self):
        with (
            patch(
                "app.services.describe_image._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.describe_image._check_model_available",
                new_callable=AsyncMock,
                return_value=False,
            ),
        ):
            result = await describe_image(_make_jpeg_bytes())

        assert result.success is False
        assert result.description is None
        assert "ollama pull" in result.message

    @pytest.mark.asyncio
    async def test_model_name_in_message_when_missing(self):
        with (
            patch(
                "app.services.describe_image._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.describe_image._check_model_available",
                new_callable=AsyncMock,
                return_value=False,
            ),
        ):
            result = await describe_image(_make_jpeg_bytes(), model="llava:13b")

        assert "llava:13b" in result.message


# ── Graceful degradation: timeout ─────────────────────────────────────────────


class TestTimeout:
    """describe_image must degrade gracefully on Ollama timeout."""

    @pytest.mark.asyncio
    async def test_returns_success_false_on_timeout(self):
        import httpx

        with (
            patch(
                "app.services.describe_image._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.describe_image._check_model_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "httpx.AsyncClient.post",
                side_effect=httpx.TimeoutException("timed out"),
            ),
        ):
            result = await describe_image(_make_jpeg_bytes())

        assert result.success is False
        assert result.description is None
        assert "timed out" in result.message.lower() or "timeout" in result.message.lower()


# ── Positive path ─────────────────────────────────────────────────────────────


class TestPositivePath:
    """describe_image must return the model's description on success."""

    def _make_mock_response(self, json_data: dict) -> MagicMock:
        """Build a mock httpx Response where .json() is synchronous."""
        mock_resp = MagicMock()
        mock_resp.raise_for_status = MagicMock()  # no-op
        mock_resp.json = MagicMock(return_value=json_data)
        return mock_resp

    @pytest.mark.asyncio
    async def test_returns_description_on_success(self):
        expected = "A cornflower-blue rectangle on a white background."

        mock_resp = self._make_mock_response({"response": expected})
        mock_post = AsyncMock(return_value=mock_resp)

        with (
            patch(
                "app.services.describe_image._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.describe_image._check_model_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch("httpx.AsyncClient.post", mock_post),
        ):
            result = await describe_image(_make_jpeg_bytes())

        assert result.success is True
        assert result.description == expected
        assert result.message == "OK"

    @pytest.mark.asyncio
    async def test_description_stripped_of_whitespace(self):
        mock_resp = self._make_mock_response(
            {"response": "  A simple test image.  \n"}
        )
        mock_post = AsyncMock(return_value=mock_resp)

        with (
            patch(
                "app.services.describe_image._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.describe_image._check_model_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch("httpx.AsyncClient.post", mock_post),
        ):
            result = await describe_image(_make_jpeg_bytes())

        assert result.description == "A simple test image."

    @pytest.mark.asyncio
    async def test_empty_response_treated_as_failure(self):
        mock_resp = self._make_mock_response({"response": "  "})
        mock_post = AsyncMock(return_value=mock_resp)

        with (
            patch(
                "app.services.describe_image._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.describe_image._check_model_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch("httpx.AsyncClient.post", mock_post),
        ):
            result = await describe_image(_make_jpeg_bytes())

        assert result.success is False
        assert result.description is None


# ── HTTP endpoint ─────────────────────────────────────────────────────────────


class TestDescribeEndpoint:
    """POST /forensics/describe must respond correctly under mocked Ollama."""

    @pytest.mark.asyncio
    async def test_endpoint_returns_200_when_ollama_down(self):
        """Endpoint must return 200 (not 503) when Ollama is unavailable."""
        with patch(
            "app.services.describe_image._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                image_bytes = _make_jpeg_bytes()
                response = await client.post(
                    "/forensics/describe",
                    files={"file": ("test.jpg", image_bytes, "image/jpeg")},
                )

        assert response.status_code == 200
        data = response.json()
        assert data["success"] is False
        assert data["description"] is None

    @pytest.mark.asyncio
    async def test_endpoint_returns_description_on_success(self):
        expected = "A solid blue rectangle photographed from above."

        # Patch the service function itself so the test client's own HTTP
        # transport is not affected.
        mock_service = AsyncMock(
            return_value=ImageDescribeResponse(
                description=expected,
                model_used="llava:7b",
                success=True,
                message="OK",
            )
        )

        with patch("app.api.forensics._describe_image", mock_service):
            async with AsyncClient(
                transport=ASGITransport(app=app), base_url="http://test"
            ) as client:
                image_bytes = _make_jpeg_bytes()
                response = await client.post(
                    "/forensics/describe",
                    files={"file": ("test.jpg", image_bytes, "image/jpeg")},
                )

        assert response.status_code == 200
        data = response.json()
        assert data["success"] is True
        assert data["description"] == expected

    @pytest.mark.asyncio
    async def test_endpoint_rejects_empty_file(self):
        """Empty file upload should be rejected with HTTP 400."""
        async with AsyncClient(
            transport=ASGITransport(app=app), base_url="http://test"
        ) as client:
            response = await client.post(
                "/forensics/describe",
                files={"file": ("empty.jpg", b"", "image/jpeg")},
            )
        assert response.status_code == 400
