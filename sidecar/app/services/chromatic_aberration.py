"""
Jura Trace Sidecar — Chromatic Aberration Consistency Analyser.

Real camera lenses produce radial chromatic aberration (CA): the R, G, and B
focal lengths differ slightly, so the channels are displaced radially from the
image centre by an amount proportional to the distance from that centre.  This
produces a consistent, lens-model-governed pattern.

AI generators (diffusion models, GANs) have no physical lens; they either
produce no CA at all or produce spatially random channel shifts that lack the
radial structure of a real lens.  Detecting the *absence* of systematic radial
CA is therefore a signal for AI generation, and its *presence* is evidence for
a real photograph.

Algorithm:
1. Convert to RGB, resize to max 512 px on the longest edge.
2. Split into R, G, B channels.
3. For a uniform grid of sample points across the image, extract a 32×32 patch
   from each channel and measure the local sub-pixel shift between the R and G
   channels, and between B and G channels, using normalised cross-correlation.
4. Compute the radial distance of each sample point from the image centre.
5. Fit a linear regression: shift = a * radial_distance + b for both R-G and
   B-G axes independently.
6. Compute the mean R² of the two linear fits.
7. Thresholds (heuristic, tuned against synthetic test images):
     R² > 0.15  → likely real photograph (consistent radial CA)
     R² < 0.05  → likely AI-generated (no systematic pattern)
     0.05–0.15  → ambiguous
8. Return CaResponse with R², per-sample shifts, and a score + suspicious flag.

Note on patch cross-correlation:
  We use `scipy.signal.correlate2d` (or `cv2.matchTemplate`) to find the
  integer-pixel shift between corresponding 32×32 patches.  Sub-pixel accuracy
  is not required — even 1–2 pixel shifts are sufficient to characterise the
  radial CA profile.
"""

import io
import math

import cv2
import numpy as np
from PIL import Image

from app.models.schemas import CaResponse

# Maximum analysis dimension (longest edge)
_ANALYSIS_SIZE = 512

# Patch size for local channel shift estimation
_PATCH_SIZE = 32

# Grid step — sample every N pixels on each axis
_GRID_STEP = 64

# R² thresholds
_R2_CONSISTENT = 0.15   # Above this → likely real photo
_R2_AMBIGUOUS = 0.05    # Below this → likely AI-generated


def perform_ca_analysis(image_bytes: bytes) -> CaResponse:
    """
    Analyse chromatic aberration consistency in an image.

    Args:
        image_bytes: Raw bytes of the input image (any PIL-readable format).

    Returns:
        CaResponse with R², per-sample shift data, score, and suspicious flag.

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
    h, w = img_array.shape[:2]

    # Split channels
    r_ch = img_array[:, :, 0].astype(np.float32)
    g_ch = img_array[:, :, 1].astype(np.float32)
    b_ch = img_array[:, :, 2].astype(np.float32)

    cx, cy = w / 2.0, h / 2.0

    # ── Grid sampling ─────────────────────────────────────────────────────
    half = _PATCH_SIZE // 2
    sample_radii: list[float] = []
    rg_shifts: list[float] = []
    bg_shifts: list[float] = []

    ys = range(half + 1, h - half - 1, _GRID_STEP)
    xs = range(half + 1, w - half - 1, _GRID_STEP)

    for y in ys:
        for x in xs:
            y0, y1 = y - half, y + half
            x0, x1 = x - half, x + half

            patch_r = r_ch[y0:y1, x0:x1]
            patch_g = g_ch[y0:y1, x0:x1]
            patch_b = b_ch[y0:y1, x0:x1]

            # Skip patches with no useful signal (very uniform regions)
            if np.std(patch_g) < 2.0:
                continue

            rg_shift = _estimate_shift_magnitude(patch_r, patch_g)
            bg_shift = _estimate_shift_magnitude(patch_b, patch_g)

            radial_dist = math.sqrt((x - cx) ** 2 + (y - cy) ** 2)

            sample_radii.append(radial_dist)
            rg_shifts.append(rg_shift)
            bg_shifts.append(bg_shift)

    sample_count = len(sample_radii)

    if sample_count < 4:
        # Not enough samples for a meaningful fit — return a neutral result
        return CaResponse(
            r_squared=0.0,
            is_consistent=False,
            score=0.5,
            suspicious=False,
            sample_count=sample_count,
            summary="Insufficient samples for chromatic aberration analysis",
        )

    radii_arr = np.array(sample_radii, dtype=np.float64)
    rg_arr = np.array(rg_shifts, dtype=np.float64)
    bg_arr = np.array(bg_shifts, dtype=np.float64)

    # ── Linear regression ─────────────────────────────────────────────────
    r2_rg = _r_squared(radii_arr, rg_arr)
    r2_bg = _r_squared(radii_arr, bg_arr)
    mean_r2 = float((r2_rg + r2_bg) / 2.0)

    # ── Scoring ───────────────────────────────────────────────────────────
    # High R² → real lens → NOT suspicious (score toward 0.0).
    # Low R²  → no radial pattern → suspicious (score toward 1.0).
    # Sigmoid: centred at the midpoint of the R² ambiguous zone (0.10),
    # with negative steepness so that higher R² → lower score.
    midpoint = (_R2_CONSISTENT + _R2_AMBIGUOUS) / 2.0  # 0.10
    score = float(1.0 / (1.0 + math.exp(20.0 * (mean_r2 - midpoint))))

    is_consistent = mean_r2 >= _R2_CONSISTENT
    suspicious = score > 0.5

    # Build summary
    if is_consistent:
        summary = (
            f"Consistent radial chromatic aberration detected (R\u00b2={mean_r2:.3f}), "
            "consistent with camera-lens origin"
        )
    elif mean_r2 < _R2_AMBIGUOUS:
        summary = (
            f"No systematic chromatic aberration (R\u00b2={mean_r2:.3f}), "
            "consistent with AI generation (no lens model)"
        )
    else:
        summary = (
            f"Ambiguous chromatic aberration pattern (R\u00b2={mean_r2:.3f})"
        )

    return CaResponse(
        r_squared=round(mean_r2, 4),
        is_consistent=is_consistent,
        score=round(score, 4),
        suspicious=suspicious,
        sample_count=sample_count,
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


def _estimate_shift_magnitude(
    patch_a: np.ndarray,
    patch_b: np.ndarray,
) -> float:
    """
    Estimate the magnitude of the pixel shift between two image patches using
    normalised cross-correlation (NCC) via cv2.matchTemplate.

    The reference patch (patch_b) is the green channel; patch_a is R or B.
    We search for the best-matching position within ±4 pixels (enough to
    capture typical lens CA at image periphery) and return the Euclidean shift
    magnitude.

    Returns 0.0 if the shift cannot be estimated reliably.
    """
    search_range = 4  # pixels
    ps = _PATCH_SIZE  # 32

    # Pad patch_b (search area) by search_range on all sides
    padded = cv2.copyMakeBorder(
        patch_b,
        search_range, search_range, search_range, search_range,
        cv2.BORDER_REFLECT,
    )

    # Template is patch_a (the channel to align)
    result = cv2.matchTemplate(
        padded.astype(np.float32),
        patch_a.astype(np.float32),
        cv2.TM_CCOEFF_NORMED,
    )

    _, _, _, max_loc = cv2.minMaxLoc(result)
    # max_loc is (col, row) of the best match top-left corner in `result`.
    # The centre of the best-match region in padded coordinates:
    match_cx = max_loc[0] + ps // 2
    match_cy = max_loc[1] + ps // 2

    # Shift relative to the unpadded reference centre (search_range, search_range)
    ref_cx = search_range + ps // 2
    ref_cy = search_range + ps // 2

    dx = float(match_cx - ref_cx)
    dy = float(match_cy - ref_cy)
    return math.sqrt(dx * dx + dy * dy)


def _r_squared(x: np.ndarray, y: np.ndarray) -> float:
    """
    Compute the R² (coefficient of determination) for a linear fit y ~ x.

    Returns 0.0 if the fit cannot be computed (e.g. constant x or y).
    """
    if len(x) < 2:
        return 0.0
    x_std = float(np.std(x))
    if x_std < 1e-10:
        return 0.0

    # Fit via polyfit (degree 1)
    coeffs = np.polyfit(x, y, 1)
    y_pred = np.polyval(coeffs, x)

    ss_res = float(np.sum((y - y_pred) ** 2))
    ss_tot = float(np.sum((y - np.mean(y)) ** 2))

    if ss_tot < 1e-10:
        # y is constant — R² is undefined; return 0 (no explanatory power)
        return 0.0

    r2 = max(0.0, 1.0 - ss_res / ss_tot)
    return float(r2)
