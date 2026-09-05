# SPDX-License-Identifier: AGPL-3.0-or-later

"""JPEG quantisation grid visualisation.

Visualises the 8x8 block grid alignment and boundary artefacts.
Regions spliced from a differently-compressed JPEG show misaligned
grids or different boundary artefact levels.
"""

import base64
import io
import logging

import cv2
import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


def perform_jpeg_grid_visualisation(image_bytes: bytes) -> dict:
    """Visualise JPEG 8x8 block boundary artefacts.

    Returns:
        Dict with:
        - grid_artefact_base64: heatmap of block boundary strength (base64 PNG)
        - q_table: list of lists — JPEG quantisation table if extractable
        - grid_consistency: float 0-1 — how uniform the grid artefacts are
          (1.0 = uniform)
    """
    if len(image_bytes) == 0:
        raise ValueError("Could not decode image")

    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise ValueError("Could not decode image")

    h, w = img.shape
    img_f = img.astype(np.float32)

    # Compute block boundary artefact strength
    # Horizontal boundaries: difference at every 8th column
    h_diff = np.zeros((h, w), dtype=np.float32)
    for x in range(8, w - 1):
        if x % 8 == 0:
            h_diff[:, x] = np.abs(img_f[:, x] - img_f[:, x - 1])

    # Vertical boundaries: difference at every 8th row
    v_diff = np.zeros((h, w), dtype=np.float32)
    for y in range(8, h - 1):
        if y % 8 == 0:
            v_diff[y, :] = np.abs(img_f[y, :] - img_f[y - 1, :])

    # Combine
    grid_strength = h_diff + v_diff

    # Smooth slightly for visualisation
    grid_smooth = cv2.GaussianBlur(grid_strength, (3, 3), 0)

    # Normalise
    if grid_smooth.max() > 0:
        grid_norm = (grid_smooth / grid_smooth.max() * 255).astype(np.uint8)
    else:
        grid_norm = np.zeros_like(grid_smooth, dtype=np.uint8)

    grid_coloured = cv2.applyColorMap(grid_norm, cv2.COLORMAP_HOT)
    grid_rgb = cv2.cvtColor(grid_coloured, cv2.COLOR_BGR2RGB)

    grid_pil = Image.fromarray(grid_rgb)
    buf = io.BytesIO()
    grid_pil.save(buf, format="PNG")
    grid_b64 = base64.b64encode(buf.getvalue()).decode("utf-8")

    # Extract Q-table from JPEG
    q_table = None
    try:
        pil_img = Image.open(io.BytesIO(image_bytes))
        if hasattr(pil_img, "quantization") and pil_img.quantization:
            # Get first Q-table (luminance)
            qt = pil_img.quantization.get(0)
            if qt:
                q_table = [list(qt[i : i + 8]) for i in range(0, 64, 8)]
    except Exception:
        pass

    # Grid consistency: measure how uniform the artefact level is across blocks
    rows8 = h // 8
    cols8 = w // 8
    if rows8 > 2 and cols8 > 2:
        block_strengths = []
        for r in range(rows8 - 1):
            for c in range(cols8 - 1):
                region = grid_strength[r * 8 : (r + 1) * 8, c * 8 : (c + 1) * 8]
                block_strengths.append(float(np.mean(region)))
        if block_strengths:
            mean_s = np.mean(block_strengths)
            std_s = np.std(block_strengths)
            grid_consistency = float(1.0 - min(std_s / (mean_s + 1e-8), 1.0))
        else:
            grid_consistency = 0.0
    else:
        grid_consistency = 0.0

    return {
        "grid_artefact_base64": grid_b64,
        "q_table": q_table,
        "grid_consistency": round(grid_consistency, 4),
    }
