"""Frequency domain visualisation for forensic analysis.

Displays 2D FFT magnitude spectrum, annotated with known artefact
frequencies (JPEG 8x8 grid, GAN upsampling checkerboard patterns).
"""
import base64
import io
import logging

import cv2
import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


def perform_frequency_visualisation(image_bytes: bytes) -> dict:
    """Compute frequency domain visualisations.

    Returns:
        Dict with:
        - fft_magnitude_base64: 2D FFT log-magnitude spectrum (base64 PNG)
        - dct_heatmap_base64: 8x8 block-averaged DCT coefficient magnitude (base64 PNG)
        - has_jpeg_grid: bool — whether periodic peaks at 1/8 frequency detected
        - dominant_frequency: float — strongest non-DC frequency component
    """
    if len(image_bytes) == 0:
        raise ValueError("Could not decode image")

    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise ValueError("Could not decode image")

    h, w = img.shape

    # 2D FFT
    f = np.fft.fft2(img.astype(np.float32))
    fshift = np.fft.fftshift(f)
    magnitude = np.log1p(np.abs(fshift))

    # Normalise to 0-255
    mag_norm = (
        (magnitude - magnitude.min())
        / (magnitude.max() - magnitude.min() + 1e-8)
        * 255
    ).astype(np.uint8)

    # Apply colourmap for better visualisation
    mag_coloured = cv2.applyColorMap(mag_norm, cv2.COLORMAP_INFERNO)
    mag_rgb = cv2.cvtColor(mag_coloured, cv2.COLOR_BGR2RGB)

    fft_pil = Image.fromarray(mag_rgb)
    buf = io.BytesIO()
    fft_pil.save(buf, format="PNG")
    fft_b64 = base64.b64encode(buf.getvalue()).decode("utf-8")

    # Check for JPEG 8x8 grid (peaks at multiples of w/8 and h/8)
    cx, cy = w // 2, h // 2
    has_jpeg_grid = False
    if w >= 64 and h >= 64:
        # Check for peaks at 1/8 frequency intervals
        freq_x = w // 8
        freq_y = h // 8
        # Sample magnitude at grid frequency positions
        grid_vals = []
        bg_vals = []
        for mx in range(1, 4):
            for my in range(1, 4):
                gx = cx + mx * freq_x
                gy = cy + my * freq_y
                if 0 <= gx < w and 0 <= gy < h:
                    grid_vals.append(magnitude[gy, gx])
                # Background sample offset by half
                bx = cx + mx * freq_x + freq_x // 2
                by = cy + my * freq_y + freq_y // 2
                if 0 <= bx < w and 0 <= by < h:
                    bg_vals.append(magnitude[by, bx])

        if grid_vals and bg_vals:
            grid_mean = np.mean(grid_vals)
            bg_mean = np.mean(bg_vals)
            has_jpeg_grid = bool(grid_mean > bg_mean * 1.3)

    # Dominant non-DC frequency
    mag_copy = magnitude.copy()
    # Zero out DC component (centre 5x5)
    dc_r = 3
    y_lo = max(cy - dc_r, 0)
    y_hi = min(cy + dc_r + 1, h)
    x_lo = max(cx - dc_r, 0)
    x_hi = min(cx + dc_r + 1, w)
    mag_copy[y_lo:y_hi, x_lo:x_hi] = 0
    dominant_frequency = float(np.max(mag_copy))

    # DCT block heatmap (8x8 block-averaged)
    rows8 = h // 8
    cols8 = w // 8
    dct_map = np.zeros((max(rows8, 1), max(cols8, 1)), dtype=np.float32)
    for r in range(rows8):
        for c in range(cols8):
            block = img[r * 8 : (r + 1) * 8, c * 8 : (c + 1) * 8].astype(
                np.float32
            )
            dct_block = cv2.dct(block)
            # Sum of AC coefficients (exclude DC at [0,0])
            dct_map[r, c] = np.sum(np.abs(dct_block)) - abs(dct_block[0, 0])

    if dct_map.max() > 0:
        dct_norm = (dct_map / dct_map.max() * 255).astype(np.uint8)
    else:
        dct_norm = np.zeros_like(dct_map, dtype=np.uint8)

    dct_resized = cv2.resize(dct_norm, (w, h), interpolation=cv2.INTER_NEAREST)
    dct_coloured = cv2.applyColorMap(dct_resized, cv2.COLORMAP_VIRIDIS)
    dct_rgb = cv2.cvtColor(dct_coloured, cv2.COLOR_BGR2RGB)

    dct_pil = Image.fromarray(dct_rgb)
    buf2 = io.BytesIO()
    dct_pil.save(buf2, format="PNG")
    dct_b64 = base64.b64encode(buf2.getvalue()).decode("utf-8")

    return {
        "fft_magnitude_base64": fft_b64,
        "dct_heatmap_base64": dct_b64,
        "has_jpeg_grid": has_jpeg_grid,
        "dominant_frequency": round(dominant_frequency, 4),
    }
