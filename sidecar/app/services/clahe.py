# SPDX-License-Identifier: AGPL-3.0-or-later

"""CLAHE (Contrast-Limited Adaptive Histogram Equalisation) for forensic analysis.

Applies per-channel CLAHE to reveal hidden detail in shadows, highlights,
and low-contrast regions. Useful for exposing hidden text, watermarks,
and steganographic content.
"""
import base64
import io
import logging

import cv2
import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


def perform_clahe(image_bytes: bytes, clip_limit: float = 2.0) -> dict:
    """Apply CLAHE to an image.

    Args:
        image_bytes: Raw image file bytes.
        clip_limit: CLAHE clip limit (0.5-10.0). Higher = more contrast enhancement.

    Returns:
        Dict with enhanced_image_base64 (PNG) and clip_limit used.
    """
    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        raise ValueError("Could not decode image")

    # Convert to LAB colour space
    lab = cv2.cvtColor(img, cv2.COLOR_BGR2LAB)
    l_channel, a_channel, b_channel = cv2.split(lab)

    # Apply CLAHE to L channel
    clahe = cv2.createCLAHE(clipLimit=clip_limit, tileGridSize=(8, 8))
    l_enhanced = clahe.apply(l_channel)

    # Merge and convert back to RGB
    lab_enhanced = cv2.merge([l_enhanced, a_channel, b_channel])
    result_bgr = cv2.cvtColor(lab_enhanced, cv2.COLOR_LAB2BGR)
    result_rgb = cv2.cvtColor(result_bgr, cv2.COLOR_BGR2RGB)

    # Encode as PNG
    pil_img = Image.fromarray(result_rgb)
    buf = io.BytesIO()
    pil_img.save(buf, format='PNG')
    enhanced_b64 = base64.b64encode(buf.getvalue()).decode('utf-8')

    return {
        "enhanced_image_base64": enhanced_b64,
        "clip_limit": clip_limit,
    }
