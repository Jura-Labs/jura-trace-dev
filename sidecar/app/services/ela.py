# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Error Level Analysis (ELA).

ELA detects JPEG compression artefacts that indicate image manipulation.
When an unmodified JPEG is re-saved at the same quality level, compression
artefacts are uniform. Edited regions show different error levels because
they were compressed at a different stage.

Algorithm:
1. Load original image as RGB.
2. Re-save as JPEG at a known quality level (default 90).
3. Compute pixel-wise absolute difference.
4. Scale/enhance the difference image for visibility.
5. Regions with significantly different error levels indicate manipulation.
"""

import base64
import io

import numpy as np
from PIL import Image, ImageChops, ImageEnhance

from app.models.schemas import ElaResponse


def perform_ela(image_bytes: bytes, quality: int = 90) -> ElaResponse:
    """
    Perform Error Level Analysis on an image.

    Args:
        image_bytes: Raw bytes of the input image.
        quality: JPEG recompression quality (1-100). 90 recommended.

    Returns:
        ElaResponse with heatmap, statistics, and manipulation score.

    Raises:
        ValueError: If image cannot be decoded.
    """
    try:
        original = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    # Re-save at specified JPEG quality
    buffer = io.BytesIO()
    original.save(buffer, format="JPEG", quality=quality)
    buffer.seek(0)
    recompressed = Image.open(buffer)

    # Pixel-wise difference
    ela_image = ImageChops.difference(original, recompressed)

    # Statistics
    extrema = ela_image.getextrema()
    max_diff = max(ex[1] for ex in extrema)

    ela_array = np.array(ela_image, dtype=np.float64)
    mean_diff = float(ela_array.mean())

    # Scale/enhance for visibility
    if max_diff > 0:
        scale = 255.0 / max_diff
    else:
        scale = 1.0

    ela_enhanced = ImageEnhance.Brightness(ela_image).enhance(scale)

    # Encode as base64 PNG
    output_buffer = io.BytesIO()
    ela_enhanced.save(output_buffer, format="PNG")
    ela_base64 = base64.b64encode(output_buffer.getvalue()).decode("utf-8")

    # Normalised score: 0.0 (clean) to 1.0 (highly manipulated)
    score = min(mean_diff / 25.0, 1.0)

    # Heuristic threshold for suspicious flag
    suspicious = mean_diff > 12.0

    return ElaResponse(
        ela_image_base64=ela_base64,
        max_difference=round(max_diff, 2),
        mean_difference=round(mean_diff, 2),
        score=round(score, 4),
        suspicious=suspicious,
    )
