"""Weather cross-reference for temporal verification.

Queries the Open-Meteo historical weather API (free, no key required)
to compare visible weather conditions against recorded weather data.
This is an opt-in network feature — clearly disclosed to the user.
"""
import logging
from datetime import date

import httpx

logger = logging.getLogger(__name__)

OPEN_METEO_URL = "https://archive-api.open-meteo.com/v1/archive"


async def check_weather(
    latitude: float,
    longitude: float,
    check_date: str,  # ISO format: "2024-06-21"
) -> dict:
    """Query historical weather for a given location and date.

    Args:
        latitude: Decimal degrees (-90 to 90)
        longitude: Decimal degrees (-180 to 180)
        check_date: ISO date string (YYYY-MM-DD)

    Returns:
        Dict with weather data or error message.
    """
    try:
        parsed_date = date.fromisoformat(check_date)  # noqa: F841
    except ValueError:
        raise ValueError(f"Invalid date format: {check_date}. Use YYYY-MM-DD.")

    params = {
        "latitude": latitude,
        "longitude": longitude,
        "start_date": check_date,
        "end_date": check_date,
        "daily": (
            "temperature_2m_max,temperature_2m_min,precipitation_sum,"
            "rain_sum,snowfall_sum,windspeed_10m_max,weathercode"
        ),
        "timezone": "UTC",
    }

    try:
        async with httpx.AsyncClient(timeout=15.0) as client:
            resp = await client.get(OPEN_METEO_URL, params=params)
            resp.raise_for_status()
            data = resp.json()
    except httpx.TimeoutException:
        return {"available": False, "error": "Weather API request timed out"}
    except httpx.HTTPStatusError as e:
        return {
            "available": False,
            "error": f"Weather API returned {e.response.status_code}",
        }
    except Exception as e:
        return {"available": False, "error": str(e)}

    daily = data.get("daily", {})
    if not daily or not daily.get("time"):
        return {
            "available": False,
            "error": "No weather data available for this date/location",
        }

    # WMO weather codes to human-readable descriptions
    wmo_codes = {
        0: "Clear sky",
        1: "Mainly clear",
        2: "Partly cloudy",
        3: "Overcast",
        45: "Fog",
        48: "Depositing rime fog",
        51: "Light drizzle",
        53: "Moderate drizzle",
        55: "Dense drizzle",
        61: "Slight rain",
        63: "Moderate rain",
        65: "Heavy rain",
        71: "Slight snowfall",
        73: "Moderate snowfall",
        75: "Heavy snowfall",
        80: "Slight rain showers",
        81: "Moderate rain showers",
        82: "Violent rain showers",
        85: "Slight snow showers",
        86: "Heavy snow showers",
        95: "Thunderstorm",
        96: "Thunderstorm with slight hail",
        99: "Thunderstorm with heavy hail",
    }

    weather_code = daily.get("weathercode", [None])[0]
    weather_desc = (
        wmo_codes.get(weather_code, f"Unknown (code {weather_code})")
        if weather_code is not None
        else "Unknown"
    )

    return {
        "available": True,
        "date": check_date,
        "latitude": latitude,
        "longitude": longitude,
        "temperature_max_c": daily.get("temperature_2m_max", [None])[0],
        "temperature_min_c": daily.get("temperature_2m_min", [None])[0],
        "precipitation_mm": daily.get("precipitation_sum", [None])[0],
        "rain_mm": daily.get("rain_sum", [None])[0],
        "snowfall_cm": daily.get("snowfall_sum", [None])[0],
        "max_wind_kmh": daily.get("windspeed_10m_max", [None])[0],
        "weather_code": weather_code,
        "weather_description": weather_desc,
        "source": "Open-Meteo Historical Weather API",
        "disclaimer": (
            "Weather data is from the nearest weather station and may not "
            "reflect microclimatic conditions at the exact location."
        ),
    }
