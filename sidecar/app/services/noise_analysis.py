"""
Jura Archive Sidecar — Block-wise Noise Variance Analysis.

Detects inconsistent noise patterns across image blocks that may indicate
localised manipulation. Authentic photographs have relatively uniform
sensor noise; spliced or inpainted regions often show different noise
characteristics.

Algorithm:
1. Convert image to greyscale.
2. Apply Laplacian filter to isolate high-frequency noise.
3. Divide into NxN blocks and compute variance per block.
4. Flag blocks whose variance deviates significantly from the median
   (using Median Absolute Deviation for robust outlier detection).
5. Generate a heatmap of block variances.
"""

import base64
import io
import math

import cv2
import numpy as np
from PIL import Image

from app.models.schemas import NoiseAnalysisResponse


def perform_noise_analysis(
    image_bytes: bytes,
    block_size: int = 32,
) -> NoiseAnalysisResponse:
    """
    Perform block-wise noise variance analysis on an image.

    Args:
        image_bytes: Raw bytes of the input image.
        block_size: Side length of each analysis block in pixels.

    Returns:
        NoiseAnalysisResponse with heatmap, statistics, and anomaly flag.

    Raises:
        ValueError: If image cannot be decoded.
    """
    try:
        pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
        img_array = np.array(pil_image)
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    # Convert to greyscale
    grey = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)

    # Apply Laplacian filter to isolate high-frequency components
    laplacian = cv2.Laplacian(grey, cv2.CV_64F)

    h, w = laplacian.shape
    rows = h // block_size
    cols = w // block_size

    if rows < 2 or cols < 2:
        # Image too small for meaningful block analysis
        return NoiseAnalysisResponse(
            heatmap_base64="",
            block_variances=[],
            global_variance=round(float(np.var(laplacian)), 2),
            anomalous_blocks=0,
            total_blocks=0,
            score=0.0,
            suspicious=False,
        )

    # Compute variance per block
    variances = np.zeros((rows, cols), dtype=np.float64)
    for r in range(rows):
        for c in range(cols):
            block = laplacian[
                r * block_size : (r + 1) * block_size,
                c * block_size : (c + 1) * block_size,
            ]
            variances[r, c] = np.var(block)

    global_variance = float(np.var(laplacian))

    # Robust outlier detection via Median Absolute Deviation (MAD)
    flat_vars = variances.flatten()
    median_var = float(np.median(flat_vars))
    mad = float(np.median(np.abs(flat_vars - median_var)))

    if mad > 0:
        # Modified z-score (0.6745 is the 0.75th quantile of the standard normal).
        # Threshold 4.5 (raised from 3.5) is more robust against AVIF/WebP
        # variable quantisation which creates legitimate noise variation.
        z_scores = 0.6745 * (flat_vars - median_var) / mad
        anomalous = int(np.sum(np.abs(z_scores) > 4.5))
    else:
        # All blocks identical — no anomalies
        anomalous = 0

    total_blocks = rows * cols

    # Generate heatmap
    heatmap = _generate_heatmap(variances, (h, w))
    buffer = io.BytesIO()
    Image.fromarray(heatmap).save(buffer, format="PNG")
    heatmap_base64 = base64.b64encode(buffer.getvalue()).decode("utf-8")

    # Score: sigmoid of anomalous proportion, centred at 30%.
    # Previous linear formula saturated at 5% anomalous blocks, producing
    # false positives for modern lossy codecs (AVIF, WebP) whose variable
    # block-size quantisation creates legitimate noise variation across
    # 10-40% of blocks. The sigmoid (midpoint=30%, k=12) gives a gradual
    # curve: 5%→0.04, 15%→0.14, 25%→0.38, 30%→0.50, 40%→0.73, 50%→0.88.
    anomalous_ratio = anomalous / total_blocks if total_blocks > 0 else 0.0
    score = 1.0 / (1.0 + math.exp(-12.0 * (anomalous_ratio - 0.30)))

    # Suspicious if more than 25% of blocks are anomalous
    suspicious = anomalous_ratio > 0.25

    return NoiseAnalysisResponse(
        heatmap_base64=heatmap_base64,
        block_variances=[round(float(v), 2) for v in flat_vars.tolist()],
        global_variance=round(global_variance, 2),
        anomalous_blocks=anomalous,
        total_blocks=total_blocks,
        score=round(score, 4),
        suspicious=suspicious,
    )


def _generate_heatmap(
    variances: np.ndarray,
    original_size: tuple[int, int],
) -> np.ndarray:
    """Create a colour heatmap from block variance values, resized to match original."""
    rows, cols = variances.shape

    # Normalise to 0-255
    v_min = variances.min()
    v_max = variances.max()
    if v_max > v_min:
        normalised = ((variances - v_min) / (v_max - v_min) * 255).astype(np.uint8)
    else:
        normalised = np.zeros_like(variances, dtype=np.uint8)

    # Upscale to block resolution
    block_h = original_size[0] // rows
    block_w = original_size[1] // cols
    upscaled = np.repeat(np.repeat(normalised, block_h, axis=0), block_w, axis=1)

    # Apply colour map (COLORMAP_JET: blue=low, red=high)
    heatmap_bgr = cv2.applyColorMap(upscaled, cv2.COLORMAP_JET)
    heatmap_rgb = cv2.cvtColor(heatmap_bgr, cv2.COLOR_BGR2RGB)

    return heatmap_rgb
