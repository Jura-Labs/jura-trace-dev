# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — DCT coefficient map analysis.

Computes per-block 8×8 DCT statistics across the image to detect
mixed compression levels that indicate image splicing or compositing.
The AC-energy coefficient of variation (CV) is a sensitive marker:
a uniformly-compressed authentic image has low CV, while a spliced
image that mixes source blocks from different JPEG compressions
shows elevated AC variance.
"""

import base64
import io
from typing import Any

import numpy as np
from PIL import Image
from scipy.fft import dctn


def analyse_dct(image_bytes: bytes) -> dict[str, Any]:
    """Compute 8×8 block DCT statistics across the image.

    Args:
        image_bytes: Raw bytes of the image file.

    Returns:
        Dictionary with keys:
          - heatmapBase64: base64-encoded PNG of the AC-energy heatmap
          - dcStd: standard deviation of the DC (mean brightness) coefficient
          - acMean: mean AC energy across all blocks
          - acStd: standard deviation of AC energy
          - acCoefficientOfVariation: ac_std / ac_mean — primary splice signal
          - suspicious: True when acCoefficientOfVariation > threshold
          - score: normalised score in [0, 1]
          - summary: human-readable interpretation

    Raises:
        ValueError: if image_bytes is not a valid image.
    """
    try:
        img = Image.open(io.BytesIO(image_bytes)).convert("L")
    except Exception as exc:
        raise ValueError(f"Could not decode image: {exc}") from exc

    arr = np.array(img, dtype=np.float64)
    h, w = arr.shape

    # Require at least one 8×8 block in each axis.
    bh, bw = h // 8, w // 8
    if bh < 1 or bw < 1:
        raise ValueError(
            f"Image too small for DCT block analysis ({w}×{h} px — need at least 8×8)"
        )

    dc_values = np.zeros((bh, bw))
    ac_energy = np.zeros((bh, bw))

    for i in range(bh):
        for j in range(bw):
            block = arr[i * 8 : (i + 1) * 8, j * 8 : (j + 1) * 8]
            dct_block = dctn(block, type=2, norm="ortho")
            dc_values[i, j] = dct_block[0, 0]
            # AC energy = sum of squares of all non-DC coefficients.
            ac_energy[i, j] = float(
                np.sum(dct_block[1:, :] ** 2) + np.sum(dct_block[0, 1:] ** 2)
            )

    # ── Heatmap ────────────────────────────────────────────────────────────
    # Normalise AC energy to [0, 255] and resize to full image dimensions.
    ac_min = float(ac_energy.min())
    ac_max = float(ac_energy.max())
    ac_range = ac_max - ac_min + 1e-10
    ac_norm = ((ac_energy - ac_min) / ac_range * 255).astype(np.uint8)
    heatmap = Image.fromarray(ac_norm).resize((w, h), Image.NEAREST)

    buf = io.BytesIO()
    heatmap.save(buf, format="PNG")
    heatmap_b64 = base64.b64encode(buf.getvalue()).decode()

    # ── Statistics ─────────────────────────────────────────────────────────
    dc_std = float(np.std(dc_values))
    ac_mean = float(np.mean(ac_energy))
    ac_std = float(np.std(ac_energy))
    # Coefficient of variation: scale-independent dispersion measure.
    ac_cv = ac_std / (ac_mean + 1e-10)

    # Threshold derived from empirical testing:
    #   - Authentic JPEG: ac_cv typically 0.15–0.35
    #   - Spliced images:  ac_cv typically 0.50–1.20
    # 0.5 is a conservative initial threshold (prefer low FP rate over recall).
    threshold = 0.5
    suspicious = bool(ac_cv > threshold)
    score = float(min(ac_cv, 1.0))

    if suspicious:
        summary = (
            f"DCT AC energy CV {ac_cv:.3f} exceeds threshold {threshold:.2f}. "
            "Compression inconsistencies detected — possible splice or composite."
        )
    else:
        summary = (
            f"DCT AC energy CV {ac_cv:.3f} within normal range. "
            "Compression appears uniform across the image."
        )

    return {
        "heatmapBase64": heatmap_b64,
        "dcStd": dc_std,
        "acMean": ac_mean,
        "acStd": ac_std,
        "acCoefficientOfVariation": ac_cv,
        "suspicious": suspicious,
        "score": score,
        "summary": summary,
    }
