# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Neighbouring Pixel Relationships (NPR) Detector.

Detects AI-generated images by analysing the statistical relationships between
adjacent pixels. Camera sensors produce noise with characteristic inter-pixel
correlations driven by the physical sensor array, optics, and demosaicing.
AI generators (diffusion models, GANs) produce pixels independently or through
learned convolution kernels, resulting in subtly different NPR statistics.

Based on: Tan et al., "Rethinking the Up-Sampling Operations in CNN-based
Generative Network for Generalizable Deepfake Detection", AAAI 2024.

Algorithm:
1. Convert to greyscale float64, resize to max 512px on the longest edge.
2. Compute four directional pixel-difference maps: horizontal, vertical,
   diagonal-right (top-left to bottom-right), diagonal-left (top-right to
   bottom-left).
3. Compute per-difference-map statistics: mean, std, kurtosis, skewness.
4. Compute the cross-correlation coefficient between horizontal and vertical
   difference maps (H-V correlation).
5. Compute the ratio of high-frequency to low-frequency energy in the
   difference maps using 2D FFT.
6. Score via a weighted heuristic:
     - Lower variance  → AI indicator (more uniform generation)
     - Higher H-V cross-correlation → AI indicator (grid-like generation)
     - Lower HF energy ratio → AI indicator (smoother pixel transitions)
7. Return NprResponse with score, per-feature values, and a heatmap of the
   pixel-difference magnitude.
"""

import base64
import io
import math

import cv2
import numpy as np
from PIL import Image
from scipy.stats import kurtosis as scipy_kurtosis
from scipy.stats import skew as scipy_skew

from app.models.schemas import NprResponse

# Maximum analysis dimension (longest edge in pixels)
_ANALYSIS_SIZE = 512

# Score thresholds for the three heuristic features:
#   - diff_variance_ratio: ratio of image variance to difference-map variance.
#     Real photos: close to 1.0.  AI: lower (< 0.4 indicates uniform diffs).
#   - hv_correlation: Pearson r between H and V difference maps.
#     Real photos: weakly correlated (r < 0.3).  AI: more correlated (> 0.5).
#   - hf_energy_ratio: fraction of difference-map energy at high spatial freqs.
#     Real photos: higher HF ratio (> 0.08).  AI: lower (< 0.04, smooth).

_VAR_RATIO_LOW = 0.4  # Below this → AI indicator (low variance in diffs)
_HV_CORR_HIGH = 0.50  # Above this → AI indicator (grid-like H-V coupling)
_HF_RATIO_LOW = 0.04  # Below this → AI indicator (smooth freq content)


def perform_npr_analysis(image_bytes: bytes) -> NprResponse:
    """
    Perform NPR (Neighbouring Pixel Relationship) analysis on an image.

    Args:
        image_bytes: Raw bytes of the input image (any PIL-readable format).

    Returns:
        NprResponse with score, feature values, and a difference-magnitude
        heatmap encoded as base64 PNG.

    Raises:
        ValueError: If the image cannot be decoded.
    """
    try:
        pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
        img_array = np.array(pil_image)
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    # Resize for consistent analysis
    img_array = _resize(img_array, _ANALYSIS_SIZE)

    # Convert to greyscale float64
    grey = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY).astype(np.float64)

    # ── Step 1: Compute directional difference maps ───────────────────────
    # Each map has the same spatial dimensions as the clipped image region.
    diff_h = grey[:, 1:] - grey[:, :-1]  # horizontal: right − left
    diff_v = grey[1:, :] - grey[:-1, :]  # vertical:   down  − up
    diff_dr = grey[1:, 1:] - grey[:-1, :-1]  # diagonal right (TL→BR)
    diff_dl = grey[1:, :-1] - grey[:-1, 1:]  # diagonal left  (TR→BL)

    diff_maps = [diff_h, diff_v, diff_dr, diff_dl]
    diff_names = ["horizontal", "vertical", "diagonal_right", "diagonal_left"]

    # ── Step 2: Per-map statistics ────────────────────────────────────────
    stats: dict[str, float] = {}
    for name, dm in zip(diff_names, diff_maps):
        flat = dm.flatten()
        stats[f"{name}_mean"] = float(np.mean(flat))
        stats[f"{name}_std"] = float(np.std(flat))
        # scipy kurtosis returns excess kurtosis by default (Fisher=True)
        stats[f"{name}_kurtosis"] = float(scipy_kurtosis(flat, bias=True))
        stats[f"{name}_skewness"] = float(scipy_skew(flat, bias=True))

    # ── Step 3: H-V cross-correlation ────────────────────────────────────
    # Align both maps to the same shape (they may differ by 1 row/col).
    min_rows = min(diff_h.shape[0], diff_v.shape[0])
    min_cols = min(diff_h.shape[1], diff_v.shape[1])
    h_aligned = diff_h[:min_rows, :min_cols].flatten()
    v_aligned = diff_v[:min_rows, :min_cols].flatten()
    hv_correlation = _safe_corrcoef(h_aligned, v_aligned)

    # ── Step 4: Difference-map variance ratio ─────────────────────────────
    # Compare variance of the original pixel values to variance of the
    # horizontal difference map.  A near-constant image trivially has zero
    # variance in both, so guard against division by zero.
    img_var = float(np.var(grey))
    h_var = float(np.var(diff_h))
    diff_variance_ratio = h_var / (img_var + 1e-10)

    # ── Step 5: HF energy ratio in the difference map ────────────────────
    hf_energy_ratio = _compute_hf_energy_ratio(diff_h)

    # ── Step 6: Heuristic score ───────────────────────────────────────────
    # Three weighted sub-signals, each contributing to a [0, 1] activation.
    # Weights reflect empirical discriminability (HV corr is most reliable).
    signal_weight_low_var = 0.30
    signal_weight_hv_corr = 0.45
    signal_weight_low_hf = 0.25

    # Each signal fires [0, 1] — use a soft activation rather than binary
    # thresholding so borderline images are not harshly classified.
    activation_low_var = _soft_threshold_below(diff_variance_ratio, _VAR_RATIO_LOW)
    activation_hv_corr = _soft_threshold_above(hv_correlation, _HV_CORR_HIGH)
    activation_low_hf = _soft_threshold_below(hf_energy_ratio, _HF_RATIO_LOW)

    raw = (
        signal_weight_low_var * activation_low_var
        + signal_weight_hv_corr * activation_hv_corr
        + signal_weight_low_hf * activation_low_hf
    )
    # Map raw [0, 1] through a sigmoid centred at 0.5 with k=6 to get a
    # smooth, bounded score.  Mid-range activations map to ~0.5; clear
    # AI patterns push toward 1.0; clear authentic images toward 0.0.
    score = float(1.0 / (1.0 + math.exp(-6.0 * (raw - 0.5))))

    # ── Step 7: Heatmap (pixel-difference magnitude) ──────────────────────
    heatmap_base64 = _generate_heatmap(diff_h, diff_v, img_array.shape[:2])

    # Build summary
    triggered = []
    if activation_low_var > 0.5:
        triggered.append("low difference variance")
    if activation_hv_corr > 0.5:
        triggered.append("high H-V correlation")
    if activation_low_hf > 0.5:
        triggered.append("low high-frequency energy")

    if score > 0.6:
        summary = (
            f"Strong NPR synthetic indicators detected ({len(triggered)} of 3 signals"
            f"{': ' + ', '.join(triggered) if triggered else ''})"
        )
    elif score > 0.4:
        summary = f"Mixed NPR indicators ({len(triggered)} of 3 signals triggered)"
    else:
        summary = "NPR analysis consistent with camera-sensor origin"

    return NprResponse(
        score=round(score, 4),
        suspicious=score > 0.5,
        hv_correlation=round(hv_correlation, 4),
        diff_variance_ratio=round(diff_variance_ratio, 4),
        hf_energy_ratio=round(hf_energy_ratio, 6),
        heatmap_base64=heatmap_base64,
        summary=summary,
    )


# ── Private helpers ───────────────────────────────────────────────────────


def _resize(img: np.ndarray, max_edge: int) -> np.ndarray:
    """Resize image so its longest edge equals max_edge (no upscaling)."""
    h, w = img.shape[:2]
    if max(h, w) <= max_edge:
        return img
    scale = max_edge / max(h, w)
    new_w = max(1, int(w * scale))
    new_h = max(1, int(h * scale))
    return cv2.resize(img, (new_w, new_h), interpolation=cv2.INTER_AREA)


def _safe_corrcoef(a: np.ndarray, b: np.ndarray) -> float:
    """Pearson correlation coefficient; returns 0.0 for constant arrays."""
    if np.std(a) < 1e-10 or np.std(b) < 1e-10:
        return 0.0
    return float(np.corrcoef(a, b)[0, 1])


def _compute_hf_energy_ratio(diff_map: np.ndarray) -> float:
    """
    Compute the ratio of high-frequency to total energy in a difference map.

    High spatial frequencies (outer 25% of the FFT frequency domain) indicate
    fine pixel-level variation.  Real camera images have more HF content than
    AI-generated ones, whose generation process tends to be bandwidth-limited.
    """
    f_transform = np.fft.fft2(diff_map)
    f_shifted = np.fft.fftshift(f_transform)
    power = np.abs(f_shifted) ** 2

    h, w = diff_map.shape
    cy, cx = h // 2, w // 2
    y_coords, x_coords = np.ogrid[:h, :w]
    radius = np.sqrt((x_coords - cx) ** 2 + (y_coords - cy) ** 2)
    max_radius = min(cx, cy)

    total_energy = power.sum() + 1e-10
    hf_mask = radius > (max_radius * 0.75)
    hf_energy = float(power[hf_mask].sum() / total_energy)
    return hf_energy


def _soft_threshold_above(
    value: float, threshold: float, steepness: float = 8.0
) -> float:
    """Sigmoid activation that rises toward 1.0 as value exceeds threshold."""
    return float(1.0 / (1.0 + math.exp(-steepness * (value - threshold))))


def _soft_threshold_below(
    value: float, threshold: float, steepness: float = 8.0
) -> float:
    """Sigmoid activation that rises toward 1.0 as value falls below threshold."""
    return float(1.0 / (1.0 + math.exp(steepness * (value - threshold))))


def _generate_heatmap(
    diff_h: np.ndarray,
    diff_v: np.ndarray,
    original_shape: tuple[int, int],
) -> str:
    """
    Generate a colour heatmap of pixel-difference magnitude as base64 PNG.

    The heatmap shows the combined horizontal + vertical difference magnitude,
    upscaled to the original image dimensions.  Bright areas indicate regions
    with high pixel-level variation; dark areas indicate smoothness.
    """
    # Combine H and V magnitudes into a single magnitude map.
    # Align shapes (diff_h loses last col, diff_v loses last row).
    min_rows = min(diff_h.shape[0], diff_v.shape[0])
    min_cols = min(diff_h.shape[1], diff_v.shape[1])
    magnitude = np.sqrt(
        diff_h[:min_rows, :min_cols] ** 2 + diff_v[:min_rows, :min_cols] ** 2
    )

    # Normalise to [0, 255]
    mag_min = magnitude.min()
    mag_max = magnitude.max()
    if mag_max > mag_min:
        normalised = ((magnitude - mag_min) / (mag_max - mag_min) * 255).astype(
            np.uint8
        )
    else:
        normalised = np.zeros_like(magnitude, dtype=np.uint8)

    # Resize to original image dimensions
    orig_h, orig_w = original_shape
    resized = cv2.resize(normalised, (orig_w, orig_h), interpolation=cv2.INTER_LINEAR)

    # Apply COLORMAP_MAGMA: dark=low diff, bright=high diff
    heatmap_bgr = cv2.applyColorMap(resized, cv2.COLORMAP_MAGMA)
    heatmap_rgb = cv2.cvtColor(heatmap_bgr, cv2.COLOR_BGR2RGB)

    buf = io.BytesIO()
    Image.fromarray(heatmap_rgb).save(buf, format="PNG")
    return base64.b64encode(buf.getvalue()).decode("utf-8")
