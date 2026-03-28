"""Diffusion model artefact detection.

Identifies artefacts characteristic of latent diffusion models:
unnaturally smooth textures, VAE decoder banding, and resolution
fingerprints matching known generation sizes.
"""
import base64
import io
import logging

import cv2
import numpy as np
from PIL import Image

logger = logging.getLogger(__name__)

# Known AI generation resolutions (width x height)
KNOWN_GEN_SIZES = {
    (512, 512), (768, 768), (1024, 1024),  # SD v1/v2
    (1344, 768), (768, 1344),  # SDXL
    (1024, 576), (576, 1024),  # SDXL
    (1280, 720), (720, 1280),  # Common video gen
    (1792, 1024), (1024, 1792),  # DALL-E 3
}


def detect_diffusion_artefacts(image_bytes: bytes) -> dict:
    """Analyse image for diffusion model generation artefacts.

    Returns dict with:
    - texture_smoothness_score: float 0-1 (1 = unnaturally smooth)
    - texture_smoothness_map_base64: heatmap showing smooth regions (base64 PNG)
    - vae_banding_score: float 0-1 (1 = strong banding detected)
    - resolution_match: bool — dimensions match a known generation size
    - resolution_note: str — description of the match
    - overall_diffusion_score: float 0-1 — combined signal
    """
    if not image_bytes:
        raise ValueError("Empty image data")

    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        raise ValueError("Could not decode image")

    h, w = img.shape[:2]
    grey = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)

    # 1. Texture smoothness analysis using local standard deviation
    # Compute local standard deviation in 16x16 patches
    patch_size = 16
    rows = h // patch_size
    cols = w // patch_size
    local_stds = np.zeros((rows, cols), dtype=np.float32)

    for r in range(rows):
        for c in range(cols):
            patch = grey[
                r * patch_size : (r + 1) * patch_size,
                c * patch_size : (c + 1) * patch_size,
            ].astype(np.float32)
            local_stds[r, c] = np.std(patch)

    # Smoothness: ratio of patches with very low texture
    smooth_threshold = 8.0  # std < 8 is quite smooth
    smooth_ratio = float(np.mean(local_stds < smooth_threshold))

    # Natural images typically have 10-30% smooth patches (sky, walls)
    # AI images often have 40-70% smooth patches
    texture_smoothness_score = min(max((smooth_ratio - 0.15) / 0.45, 0.0), 1.0)

    # Generate smoothness heatmap
    if local_stds.max() > 0:
        std_norm = (local_stds / max(local_stds.max(), 1.0) * 255).astype(np.uint8)
    else:
        std_norm = np.zeros_like(local_stds, dtype=np.uint8)
    # Invert so smooth = bright (hot)
    std_inv = 255 - std_norm
    std_resized = cv2.resize(std_inv, (w, h), interpolation=cv2.INTER_NEAREST)
    std_coloured = cv2.applyColorMap(std_resized, cv2.COLORMAP_HOT)
    std_rgb = cv2.cvtColor(std_coloured, cv2.COLOR_BGR2RGB)

    smoothness_pil = Image.fromarray(std_rgb)
    buf = io.BytesIO()
    smoothness_pil.save(buf, format="PNG")
    smoothness_b64 = base64.b64encode(buf.getvalue()).decode("utf-8")

    # 2. VAE banding detection
    # Look for subtle colour banding in smooth gradient regions
    # Compute gradient magnitude and check for step-like transitions
    grad_x = cv2.Sobel(grey.astype(np.float32), cv2.CV_32F, 1, 0, ksize=3)
    grad_y = cv2.Sobel(grey.astype(np.float32), cv2.CV_32F, 0, 1, ksize=3)
    grad_mag = np.sqrt(grad_x**2 + grad_y**2)

    # In smooth regions, check if gradients are quantised (step-like)
    smooth_mask = local_stds < smooth_threshold
    smooth_mask_full = cv2.resize(
        smooth_mask.astype(np.uint8), (w, h), interpolation=cv2.INTER_NEAREST
    )

    smooth_grads = grad_mag[smooth_mask_full > 0]
    if len(smooth_grads) > 100:
        # Histogram of gradient magnitudes in smooth regions
        hist, _ = np.histogram(smooth_grads, bins=50, range=(0, 20))
        hist_norm = hist / (hist.sum() + 1e-8)
        # Peaky histogram suggests quantised (banded) gradients
        peak_ratio = float(hist_norm.max())
        vae_banding_score = min(max((peak_ratio - 0.1) / 0.3, 0.0), 1.0)
    else:
        vae_banding_score = 0.0

    # 3. Resolution fingerprint
    resolution_match = (w, h) in KNOWN_GEN_SIZES
    if resolution_match:
        resolution_note = f"Dimensions {w}x{h} match a known AI generation size"
    elif w == h and w in (512, 768, 1024, 2048):
        resolution_match = True
        resolution_note = (
            f"Square {w}x{h} matches common AI generation aspect ratio"
        )
    else:
        resolution_note = f"Dimensions {w}x{h} do not match known generation sizes"

    # 4. Combined score
    weights = [0.50, 0.30, 0.20]
    signals = [
        texture_smoothness_score,
        vae_banding_score,
        0.8 if resolution_match else 0.0,
    ]
    overall = sum(w_ * s for w_, s in zip(weights, signals))

    return {
        "texture_smoothness_score": round(texture_smoothness_score, 4),
        "texture_smoothness_map_base64": smoothness_b64,
        "vae_banding_score": round(vae_banding_score, 4),
        "resolution_match": resolution_match,
        "resolution_note": resolution_note,
        "overall_diffusion_score": round(overall, 4),
    }
