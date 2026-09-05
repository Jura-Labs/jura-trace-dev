# SPDX-License-Identifier: AGPL-3.0-or-later

"""Noise pattern visualisation for forensic analysis.

Exposes the noise residual that the deepfake detector already computes
internally, plus a block-wise variance heatmap. These visualisations
help trained analysts identify spliced regions with inconsistent noise
characteristics.
"""

import base64
import io
import logging

import cv2
import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


def perform_noise_visualisation(image_bytes: bytes) -> dict:
    """Extract and visualise noise patterns from an image.

    Returns a dict with:
    - noise_residual_base64: greyscale noise residual image (base64 PNG)
    - variance_heatmap_base64: block-wise noise variance heatmap (base64 PNG)
    - noise_std: float — global noise standard deviation
    - noise_mean: float — global noise mean
    """
    # Decode image
    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        raise ValueError("Could not decode image")

    grey = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY).astype(np.float32)

    # Extract noise residual via median filter denoising
    denoised = cv2.medianBlur(grey.astype(np.uint8), 3).astype(np.float32)
    noise = grey - denoised

    # Statistics
    noise_std = float(np.std(noise))
    noise_mean = float(np.mean(np.abs(noise)))

    # Normalise noise to 0-255 for visualisation
    # Map [-max, +max] to [0, 255] with 128 as zero
    max_val = max(np.abs(noise).max(), 1.0)
    noise_vis = ((noise / max_val) * 127 + 128).clip(0, 255).astype(np.uint8)

    # Encode noise residual as PNG
    noise_pil = Image.fromarray(noise_vis)
    buf = io.BytesIO()
    noise_pil.save(buf, format="PNG")
    noise_residual_b64 = base64.b64encode(buf.getvalue()).decode("utf-8")

    # Block-wise variance heatmap (32x32 blocks)
    block_size = 32
    h, w = grey.shape
    rows = h // block_size
    cols = w // block_size
    variance_map = np.zeros((rows, cols), dtype=np.float32)

    for r in range(rows):
        for c in range(cols):
            block = noise[
                r * block_size : (r + 1) * block_size,
                c * block_size : (c + 1) * block_size,
            ]
            variance_map[r, c] = np.var(block)

    # Normalise and colourmap
    if variance_map.max() > 0:
        variance_norm = (variance_map / variance_map.max() * 255).astype(np.uint8)
    else:
        variance_norm = np.zeros_like(variance_map, dtype=np.uint8)

    # Resize to original dimensions
    variance_resized = cv2.resize(
        variance_norm, (w, h), interpolation=cv2.INTER_NEAREST
    )
    variance_coloured = cv2.applyColorMap(variance_resized, cv2.COLORMAP_JET)
    variance_rgb = cv2.cvtColor(variance_coloured, cv2.COLOR_BGR2RGB)

    variance_pil = Image.fromarray(variance_rgb)
    buf2 = io.BytesIO()
    variance_pil.save(buf2, format="PNG")
    variance_heatmap_b64 = base64.b64encode(buf2.getvalue()).decode("utf-8")

    return {
        "noise_residual_base64": noise_residual_b64,
        "variance_heatmap_base64": variance_heatmap_b64,
        "noise_std": round(noise_std, 4),
        "noise_mean": round(noise_mean, 4),
    }
