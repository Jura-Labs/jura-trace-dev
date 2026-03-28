"""Tests for the weather cross-reference service."""

from unittest.mock import AsyncMock, MagicMock, patch

import httpx
import pytest

from app.services.weather_check import check_weather


class TestCheckWeather:
    @pytest.mark.asyncio
    async def test_invalid_date_raises_valueerror(self):
        """Invalid date format should raise ValueError."""
        with pytest.raises(ValueError, match="Invalid date format"):
            await check_weather(51.5, -0.1, "not-a-date")

    @pytest.mark.asyncio
    async def test_invalid_date_format_raises(self):
        """Partial date should raise ValueError."""
        with pytest.raises(ValueError, match="Invalid date format"):
            await check_weather(51.5, -0.1, "2024-13-01")

    @pytest.mark.asyncio
    async def test_successful_response_structure(self):
        """Successful API response should have all expected keys."""
        mock_json = {
            "daily": {
                "time": ["2024-06-21"],
                "temperature_2m_max": [25.3],
                "temperature_2m_min": [14.1],
                "precipitation_sum": [0.0],
                "rain_sum": [0.0],
                "snowfall_sum": [0.0],
                "windspeed_10m_max": [12.5],
                "weathercode": [1],
            }
        }

        mock_response = MagicMock()
        mock_response.json.return_value = mock_json
        mock_response.raise_for_status = MagicMock()

        mock_client = AsyncMock()
        mock_client.get.return_value = mock_response
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)

        with patch("app.services.weather_check.httpx.AsyncClient", return_value=mock_client):
            result = await check_weather(51.5, -0.1, "2024-06-21")

        assert result["available"] is True
        assert result["date"] == "2024-06-21"
        assert result["latitude"] == 51.5
        assert result["longitude"] == -0.1
        assert result["temperature_max_c"] == 25.3
        assert result["temperature_min_c"] == 14.1
        assert result["precipitation_mm"] == 0.0
        assert result["weather_description"] == "Mainly clear"
        assert "source" in result
        assert "disclaimer" in result

    @pytest.mark.asyncio
    async def test_timeout_returns_unavailable(self):
        """Timeout should return available=False with error message."""
        mock_client = AsyncMock()
        mock_client.get.side_effect = httpx.TimeoutException("timed out")
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)

        with patch("app.services.weather_check.httpx.AsyncClient", return_value=mock_client):
            result = await check_weather(51.5, -0.1, "2024-06-21")

        assert result["available"] is False
        assert "timed out" in result["error"].lower()

    @pytest.mark.asyncio
    async def test_http_error_returns_unavailable(self):
        """HTTP error should return available=False with status code."""
        mock_response = MagicMock()
        mock_response.status_code = 500

        mock_client = AsyncMock()
        mock_client.get.side_effect = httpx.HTTPStatusError(
            "server error",
            request=MagicMock(),
            response=mock_response,
        )
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)

        with patch("app.services.weather_check.httpx.AsyncClient", return_value=mock_client):
            result = await check_weather(51.5, -0.1, "2024-06-21")

        assert result["available"] is False
        assert "500" in result["error"]

    @pytest.mark.asyncio
    async def test_empty_daily_returns_unavailable(self):
        """Empty daily data should return available=False."""
        mock_json = {"daily": {}}

        mock_response = MagicMock()
        mock_response.json.return_value = mock_json
        mock_response.raise_for_status = MagicMock()

        mock_client = AsyncMock()
        mock_client.get.return_value = mock_response
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)

        with patch("app.services.weather_check.httpx.AsyncClient", return_value=mock_client):
            result = await check_weather(51.5, -0.1, "2024-06-21")

        assert result["available"] is False
        assert "No weather data" in result["error"]
