"""Region-of-interest (ROI) forensic analysis.

Re-runs noise, ELA, and frequency analysis on a user-selected
rectangular region, enabling comparison between suspicious and
reference regions within the same image.
"""
import base64
import io
import logging

import cv2
import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)


def analyse_roi(
    image_bytes: bytes,
    x: int,
    y: int,
    width: int,
    height: int,
) -> dict:
    """Run forensic analysis on a rectangular region.

    Args:
        image_bytes: Full image bytes
        x, y: Top-left corner of ROI
        width, height: ROI dimensions

    Returns dict with regional analysis:
    - noise_std: float — noise standard deviation in region
    - noise_mean: float — mean absolute noise in region
    - ela_mean: float — mean ELA value in region
    - frequency_energy: float — high-frequency energy ratio
    - texture_complexity: float — local texture variance
    - noise_residual_base64: regional noise residual (base64 PNG)
    """
    if not image_bytes:
        raise ValueError("Empty image data")

    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        raise ValueError("Could not decode image")

    h_img, w_img = img.shape[:2]

    # Validate and clamp ROI
    x = max(0, min(x, w_img - 1))
    y = max(0, min(y, h_img - 1))
    width = max(1, min(width, w_img - x))
    height = max(1, min(height, h_img - y))

    roi = img[y : y + height, x : x + width]
    grey = cv2.cvtColor(roi, cv2.COLOR_BGR2GRAY).astype(np.float32)

    # 1. Noise analysis
    denoised = cv2.medianBlur(grey.astype(np.uint8), 3).astype(np.float32)
    noise = grey - denoised
    noise_std = float(np.std(noise))
    noise_mean = float(np.mean(np.abs(noise)))

    # Noise residual visualisation
    max_val = max(np.abs(noise).max(), 1.0)
    noise_vis = ((noise / max_val) * 127 + 128).clip(0, 255).astype(np.uint8)
    noise_pil = Image.fromarray(noise_vis)
    buf = io.BytesIO()
    noise_pil.save(buf, format="PNG")
    noise_b64 = base64.b64encode(buf.getvalue()).decode("utf-8")

    # 2. ELA-like analysis (re-compress and compare)
    encode_param = [int(cv2.IMWRITE_JPEG_QUALITY), 75]
    _, encoded = cv2.imencode(".jpg", roi, encode_param)
    recompressed = cv2.imdecode(encoded, cv2.IMREAD_COLOR)
    ela_diff = np.abs(roi.astype(np.float32) - recompressed.astype(np.float32))
    ela_mean = float(np.mean(ela_diff))

    # 3. Frequency energy
    f = np.fft.fft2(grey)
    fshift = np.fft.fftshift(f)
    magnitude = np.abs(fshift)

    ch, cw = grey.shape[0] // 2, grey.shape[1] // 2
    # High frequency: outer 75% of spectrum
    total_energy = np.sum(magnitude)
    # Create distance mask
    yy, xx = np.ogrid[: grey.shape[0], : grey.shape[1]]
    dist = np.sqrt((xx - cw) ** 2 + (yy - ch) ** 2)
    max_dist = np.sqrt(ch**2 + cw**2)
    high_freq_mask = dist > (max_dist * 0.25)
    high_freq_energy = np.sum(magnitude[high_freq_mask])
    frequency_energy = float(high_freq_energy / (total_energy + 1e-8))

    # 4. Texture complexity (local std of std)
    patch_size = 8
    rows = max(1, grey.shape[0] // patch_size)
    cols = max(1, grey.shape[1] // patch_size)
    stds: list[float] = []
    for r in range(rows):
        for c in range(cols):
            patch = grey[
                r * patch_size : (r + 1) * patch_size,
                c * patch_size : (c + 1) * patch_size,
            ]
            if patch.size > 0:
                stds.append(float(np.std(patch)))
    texture_complexity = float(np.std(stds)) if stds else 0.0

    return {
        "noise_std": round(noise_std, 4),
        "noise_mean": round(noise_mean, 4),
        "ela_mean": round(ela_mean, 4),
        "frequency_energy": round(frequency_energy, 4),
        "texture_complexity": round(texture_complexity, 4),
        "noise_residual_base64": noise_b64,
        "roi": {"x": x, "y": y, "width": width, "height": height},
    }
