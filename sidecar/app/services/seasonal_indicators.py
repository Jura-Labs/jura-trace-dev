"""Seasonal indicator analysis.

Analyses vegetation greenness, snow coverage, and overall colour
temperature to estimate the likely season of capture.
"""
import base64
import io
import logging

import cv2
import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


def analyse_seasonal_indicators(image_bytes: bytes) -> dict:
    """Estimate season from visual indicators.

    Returns dict with:
    - greenness_index: float 0-1 (vegetation greenness)
    - snow_coverage: float 0-1 (proportion of bright, low-sat pixels)
    - warmth_index: float 0-1 (overall colour warmth, warm=summer)
    - estimated_season: str — "spring", "summer", "autumn", "winter",
      or "indeterminate"
    - confidence: float 0-1
    - indicators: list of str — human-readable indicator descriptions
    """
    if not image_bytes:
        raise ValueError("Empty image data")

    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        raise ValueError("Could not decode image")

    hsv = cv2.cvtColor(img, cv2.COLOR_BGR2HSV)
    lab = cv2.cvtColor(img, cv2.COLOR_BGR2LAB)

    h_chan, s_chan, v_chan = cv2.split(hsv)
    _l_chan, _a_chan, b_chan = cv2.split(lab)

    total_pixels = float(img.shape[0] * img.shape[1])
    indicators: list[str] = []

    # 1. Greenness index — proportion of green-hue, saturated pixels
    green_mask = (h_chan >= 35) & (h_chan <= 85) & (s_chan > 40) & (v_chan > 40)
    greenness = float(np.sum(green_mask)) / total_pixels

    if greenness > 0.25:
        indicators.append(f"High vegetation coverage ({greenness:.0%})")
    elif greenness > 0.10:
        indicators.append(f"Moderate vegetation ({greenness:.0%})")
    elif greenness > 0.02:
        indicators.append(f"Sparse vegetation ({greenness:.0%})")
    else:
        indicators.append("Minimal or no visible vegetation")

    # 2. Snow coverage — bright, low-saturation pixels
    snow_mask = (v_chan > 200) & (s_chan < 30)
    snow_coverage = float(np.sum(snow_mask)) / total_pixels

    if snow_coverage > 0.20:
        indicators.append(f"Significant snow coverage ({snow_coverage:.0%})")
    elif snow_coverage > 0.05:
        indicators.append(f"Some snow visible ({snow_coverage:.0%})")

    # 3. Warmth index — LAB b channel (positive = yellow/warm, negative = blue/cool)
    b_mean = float(np.mean(b_chan.astype(np.float32)))
    # LAB b: 128 is neutral, >128 warm, <128 cool
    warmth = (b_mean - 128.0) / 40.0  # normalise roughly to -1..1
    warmth_index = max(0.0, min(1.0, (warmth + 1.0) / 2.0))  # map to 0..1

    if warmth_index > 0.65:
        indicators.append("Warm colour temperature (golden/yellow tones)")
    elif warmth_index < 0.35:
        indicators.append("Cool colour temperature (blue/grey tones)")

    # 4. Brown/autumn colours
    brown_mask = (h_chan >= 10) & (h_chan <= 30) & (s_chan > 50) & (v_chan > 50)
    brown_ratio = float(np.sum(brown_mask)) / total_pixels
    if brown_ratio > 0.10:
        indicators.append(f"Autumn-toned foliage ({brown_ratio:.0%})")

    # Estimate season
    scores = {"spring": 0.0, "summer": 0.0, "autumn": 0.0, "winter": 0.0}

    scores["summer"] += greenness * 2.0 + warmth_index * 0.5
    scores["spring"] += greenness * 1.0 + warmth_index * 0.3
    scores["autumn"] += brown_ratio * 3.0 + (1.0 - greenness) * 0.3
    scores["winter"] += (
        snow_coverage * 3.0
        + (1.0 - warmth_index) * 0.5
        + (1.0 - greenness) * 0.3
    )

    best_season = max(scores, key=scores.get)  # type: ignore[arg-type]
    best_score = scores[best_season]
    total_score = sum(scores.values())
    confidence = best_score / (total_score + 1e-8)

    if confidence < 0.35:
        estimated_season = "indeterminate"
        confidence = 0.0
    else:
        estimated_season = best_season

    return {
        "greenness_index": round(greenness, 4),
        "snow_coverage": round(snow_coverage, 4),
        "warmth_index": round(warmth_index, 4),
        "estimated_season": estimated_season,
        "confidence": round(float(confidence), 4),
        "indicators": indicators,
    }
