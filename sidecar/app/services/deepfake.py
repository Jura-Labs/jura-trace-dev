"""
Jura Archive Sidecar — AI-Generated Image Detection.

Detects AI-generated or synthetic images using an ensemble of statistical
features extracted from frequency domain, noise residuals, colour/texture
analysis, JPEG artefacts, and edge structure. Each feature category captures
different aspects of "naturalness" that distinguish real camera photographs
from AI-generated content (GANs, diffusion models, etc.).

The heuristic scorer combines 8 weighted signals into a single score.
A trained classifier (Random Forest / Gradient Boosting) can be plugged in
later once training data is assembled.
"""

import base64
import io

import cv2
import numpy as np
from PIL import Image
from scipy.fft import fft2, fftshift, dctn
from skimage.feature import local_binary_pattern, graycomatrix, graycoprops

from app.models.schemas import DeepfakeResponse, DeepfakeSignal

# Maximum analysis dimension (longest edge)
ANALYSIS_SIZE = 512


def perform_deepfake_detection(
    image_bytes: bytes,
    analysis_size: int = ANALYSIS_SIZE,
) -> DeepfakeResponse:
    """
    Detect AI-generated content in an image.

    Args:
        image_bytes: Raw bytes of the input image.
        analysis_size: Resize longest edge to this for consistent analysis.

    Returns:
        DeepfakeResponse with score, signals, and heatmap.

    Raises:
        ValueError: If image cannot be decoded.
    """
    try:
        pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
        img_array = np.array(pil_image)
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    # Resize for consistent analysis
    img_array = _resize(img_array, analysis_size)
    img_bgr = cv2.cvtColor(img_array, cv2.COLOR_RGB2BGR)
    grey = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)

    # Extract all features
    features: dict[str, float] = {}
    features.update(_extract_frequency_features(grey))
    features.update(_extract_noise_features(img_bgr))
    features.update(_extract_color_features(img_bgr))
    features.update(_extract_texture_features(grey))
    features.update(_extract_jpeg_features(grey))
    features.update(_extract_edge_features(grey))

    # Score via heuristic ensemble
    score, signals = _heuristic_score(features)

    # Generate frequency spectrum heatmap
    heatmap_base64 = _generate_spectrum_heatmap(grey)

    # Confidence based on score extremity
    if score > 0.75 or score < 0.2:
        confidence = "high"
    elif score > 0.6 or score < 0.35:
        confidence = "medium"
    else:
        confidence = "low"

    # Summary
    triggered_count = sum(1 for s in signals if s.triggered)
    if score > 0.6:
        summary = f"Strong synthetic indicators ({triggered_count} of {len(signals)} signals triggered)"
    elif score > 0.4:
        summary = f"Mixed indicators ({triggered_count} of {len(signals)} signals triggered)"
    else:
        summary = f"Image appears authentic ({triggered_count} of {len(signals)} signals triggered)"

    return DeepfakeResponse(
        score=round(score, 4),
        suspicious=score > 0.5,
        confidence=confidence,
        signals=signals,
        heatmap_base64=heatmap_base64,
        summary=summary,
    )


# ── Feature Extractors ────────────────────────────────────────────────


def _extract_frequency_features(grey: np.ndarray) -> dict[str, float]:
    """Extract frequency domain features via 2D FFT."""
    grey_f = grey.astype(np.float64)
    f_transform = fft2(grey_f)
    f_shifted = fftshift(f_transform)
    power_spectrum = np.abs(f_shifted) ** 2

    h, w = grey.shape
    cy, cx = h // 2, w // 2

    # Radial distance from centre
    y_coords, x_coords = np.ogrid[:h, :w]
    radius = np.sqrt((x_coords - cx) ** 2 + (y_coords - cy) ** 2).astype(int)
    max_radius = min(cx, cy)

    # Azimuthally averaged power spectral density
    psd = np.zeros(max(max_radius, 1))
    for r in range(max_radius):
        mask = radius == r
        if mask.any():
            psd[r] = power_spectrum[mask].mean()

    # Spectral decay slope (beta)
    valid = psd[1:max_radius] > 0
    freqs = np.arange(1, max_radius)
    if valid.sum() > 10:
        log_f = np.log(freqs[valid])
        log_p = np.log(psd[1:max_radius][valid])
        coeffs = np.polyfit(log_f, log_p, 1)
        beta = abs(coeffs[0])
    else:
        beta = 0.0

    # Energy ratios
    total_energy = power_spectrum.sum() + 1e-10
    hf_mask = radius > (max_radius * 0.75)
    mf_mask = (radius > max_radius * 0.25) & (radius <= max_radius * 0.75)
    hf_energy = power_spectrum[hf_mask].sum() / total_energy
    mf_energy = power_spectrum[mf_mask].sum() / total_energy
    hf_to_mf = hf_energy / (mf_energy + 1e-10)

    # Azimuthal variance in 4 bands
    n_bands = 4
    band_width = max(max_radius // n_bands, 1)
    az_vars = []
    for b in range(n_bands):
        r_min = b * band_width
        r_max = (b + 1) * band_width
        band_mask = (radius >= r_min) & (radius < r_max)
        if band_mask.any():
            vals = power_spectrum[band_mask]
            mean_val = vals.mean()
            az_vars.append(float(np.var(vals) / (mean_val**2 + 1e-10)))
        else:
            az_vars.append(0.0)

    # Spectral entropy
    psd_valid = psd[1:max_radius]
    psd_norm = psd_valid / (psd_valid.sum() + 1e-10)
    psd_norm = psd_norm[psd_norm > 0]
    spectral_entropy = float(-np.sum(psd_norm * np.log2(psd_norm + 1e-10)))

    return {
        "spectral_decay_beta": beta,
        "hf_energy_ratio": float(hf_energy),
        "mf_energy_ratio": float(mf_energy),
        "hf_to_mf_ratio": float(hf_to_mf),
        "az_var_band_0": az_vars[0],
        "az_var_band_1": az_vars[1],
        "az_var_band_2": az_vars[2],
        "az_var_band_3": az_vars[3],
        "spectral_entropy": spectral_entropy,
    }


def _extract_noise_features(img_bgr: np.ndarray) -> dict[str, float]:
    """Extract noise residual features via median filter denoising."""
    grey = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2GRAY).astype(np.float64)

    denoised = cv2.medianBlur(grey.astype(np.uint8), 3).astype(np.float64)
    noise = grey - denoised

    flat = noise.flatten()
    noise_mean = float(np.mean(np.abs(noise)))
    noise_std = float(np.std(noise))
    noise_kurtosis = _kurtosis(flat)
    noise_skewness = _skewness(flat)

    # Spatial autocorrelation (PRNU indicator)
    nr = noise - noise.mean()
    autocorr = np.real(np.fft.ifft2(np.abs(np.fft.fft2(nr)) ** 2))
    ac_norm = autocorr[0, 0] + 1e-10
    ac_h1 = float(autocorr[0, 1] / ac_norm)
    ac_v1 = float(autocorr[1, 0] / ac_norm)
    ac_d1 = float(autocorr[1, 1] / ac_norm)

    # Block-wise noise variance consistency
    block_size = 32
    h, w = grey.shape
    block_vars = []
    for i in range(0, h - block_size, block_size):
        for j in range(0, w - block_size, block_size):
            block = noise[i : i + block_size, j : j + block_size]
            block_vars.append(float(np.var(block)))

    if block_vars:
        bv_mean = np.mean(block_vars)
        noise_var_cv = float(np.std(block_vars) / (bv_mean + 1e-10))
    else:
        noise_var_cv = 0.0

    # Noise spectral flatness
    noise_fft = np.abs(np.fft.fft2(noise))
    log_mean = float(np.mean(np.log(noise_fft + 1e-10)))
    arith_mean = float(np.mean(noise_fft) + 1e-10)
    spectral_flatness = float(np.exp(log_mean) / arith_mean)

    return {
        "noise_mean_abs": noise_mean,
        "noise_std": noise_std,
        "noise_kurtosis": noise_kurtosis,
        "noise_skewness": noise_skewness,
        "noise_autocorr_h1": ac_h1,
        "noise_autocorr_v1": ac_v1,
        "noise_autocorr_d1": ac_d1,
        "noise_var_cv": noise_var_cv,
        "noise_spectral_flatness": spectral_flatness,
    }


def _extract_color_features(img_bgr: np.ndarray) -> dict[str, float]:
    """Extract colour histogram and correlation features."""
    features: dict[str, float] = {}

    for i, ch in enumerate(("b", "g", "r")):
        channel = img_bgr[:, :, i].astype(np.float64).flatten()
        features[f"color_{ch}_mean"] = float(np.mean(channel))
        features[f"color_{ch}_std"] = float(np.std(channel))
        features[f"color_{ch}_skew"] = _skewness(channel)
        features[f"color_{ch}_kurt"] = _kurtosis(channel)

        # Histogram entropy
        hist = cv2.calcHist([img_bgr], [i], None, [256], [0, 256]).flatten()
        hist_norm = hist / (hist.sum() + 1e-10)
        hist_pos = hist_norm[hist_norm > 0]
        features[f"color_{ch}_entropy"] = float(-np.sum(hist_pos * np.log2(hist_pos + 1e-10)))

    # Inter-channel correlations (handle constant channels gracefully)
    b, g, r = [img_bgr[:, :, i].flatten().astype(np.float64) for i in range(3)]
    features["color_corr_rg"] = _safe_corrcoef(r, g)
    features["color_corr_rb"] = _safe_corrcoef(r, b)
    features["color_corr_gb"] = _safe_corrcoef(g, b)

    # Saturation statistics (HSV)
    hsv = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2HSV)
    sat = hsv[:, :, 1].astype(np.float64).flatten()
    features["sat_mean"] = float(np.mean(sat))
    features["sat_std"] = float(np.std(sat))
    features["sat_kurtosis"] = _kurtosis(sat)

    # Colour gamut coverage
    quantised = (img_bgr // 16).reshape(-1, 3)
    unique_colours = len(np.unique(quantised, axis=0))
    features["color_gamut_coverage"] = unique_colours / (16**3)

    return features


def _extract_texture_features(grey: np.ndarray) -> dict[str, float]:
    """Extract texture features via LBP and GLCM."""
    features: dict[str, float] = {}

    # Local Binary Pattern (uniform)
    radius = 1
    n_points = 8 * radius
    lbp = local_binary_pattern(grey, n_points, radius, method="uniform")
    n_bins = n_points + 2
    lbp_hist, _ = np.histogram(lbp, bins=n_bins, range=(0, n_bins), density=True)

    lbp_pos = lbp_hist[lbp_hist > 0]
    features["lbp_entropy"] = float(-np.sum(lbp_pos * np.log2(lbp_pos + 1e-10)))
    features["lbp_uniformity"] = float(np.sum(lbp_hist**2))
    features["lbp_mean"] = float(np.mean(lbp))
    features["lbp_var"] = float(np.var(lbp))

    # Block-wise LBP variance
    block_size = 64
    h, w = grey.shape
    block_lbp_vars = []
    for i in range(0, h - block_size, block_size):
        for j in range(0, w - block_size, block_size):
            block = grey[i : i + block_size, j : j + block_size]
            block_lbp = local_binary_pattern(block, n_points, radius, method="uniform")
            block_lbp_vars.append(float(np.var(block_lbp)))

    if block_lbp_vars:
        bv_mean = np.mean(block_lbp_vars)
        features["lbp_block_var_mean"] = float(bv_mean)
        features["lbp_block_var_std"] = float(np.std(block_lbp_vars))
        features["lbp_block_var_cv"] = float(np.std(block_lbp_vars) / (bv_mean + 1e-10))
    else:
        features["lbp_block_var_mean"] = 0.0
        features["lbp_block_var_std"] = 0.0
        features["lbp_block_var_cv"] = 0.0

    # GLCM (quantise to 64 levels)
    grey_q = (grey // 4).astype(np.uint8)
    distances = [1, 3]
    angles = [0, np.pi / 4, np.pi / 2, 3 * np.pi / 4]
    glcm = graycomatrix(grey_q, distances=distances, angles=angles, levels=64, symmetric=True, normed=True)

    for prop in ("contrast", "homogeneity", "energy", "correlation"):
        vals = graycoprops(glcm, prop)
        features[f"glcm_{prop}_mean"] = float(vals.mean())
        features[f"glcm_{prop}_std"] = float(vals.std())

    return features


def _extract_jpeg_features(grey: np.ndarray) -> dict[str, float]:
    """Extract JPEG compression artefact features via DCT analysis."""
    h, w = grey.shape
    h8 = (h // 8) * 8
    w8 = (w // 8) * 8

    if h8 < 8 or w8 < 8:
        return {"dct_benford_div": 0.0, "dct_ac_mean": 0.0, "dct_ac_std": 0.0, "dct_ac_kurtosis": 0.0, "blocking_strength": 1.0}

    img = grey[:h8, :w8].astype(np.float64)

    # DCT coefficients from 8x8 blocks
    all_ac = []
    for i in range(0, h8, 8):
        for j in range(0, w8, 8):
            block = img[i : i + 8, j : j + 8]
            dct_block = dctn(block, type=2, norm="ortho")
            all_ac.append(dct_block.flatten()[1:])  # Skip DC

    ac_matrix = np.concatenate(all_ac)

    # Benford's law on first digits
    nonzero = np.abs(ac_matrix)
    nonzero = nonzero[nonzero >= 1.0]
    if len(nonzero) > 100:
        first_digits = (nonzero / (10 ** np.floor(np.log10(nonzero + 1e-10)))).astype(int)
        first_digits = first_digits[(first_digits >= 1) & (first_digits <= 9)]
        if len(first_digits) > 0:
            digit_hist = np.histogram(first_digits, bins=range(1, 11), density=True)[0]
            benford_expected = np.log10(1 + 1 / np.arange(1, 10))
            benford_div = float(np.sum((digit_hist - benford_expected) ** 2 / (benford_expected + 1e-10)))
        else:
            benford_div = 0.0
    else:
        benford_div = 0.0

    # AC statistics
    dct_ac_mean = float(np.mean(np.abs(ac_matrix)))
    dct_ac_std = float(np.std(ac_matrix))
    dct_ac_kurtosis = _kurtosis(ac_matrix)

    # Blocking artefact strength
    boundary_diffs = []
    interior_diffs = []
    for i in range(h8 - 1):
        for j in range(w8 - 1):
            h_diff = abs(float(img[i, j]) - float(img[i, j + 1]))
            if (j + 1) % 8 == 0:
                boundary_diffs.append(h_diff)
            else:
                interior_diffs.append(h_diff)

    if boundary_diffs and interior_diffs:
        blocking = float(np.mean(boundary_diffs) / (np.mean(interior_diffs) + 1e-10))
    else:
        blocking = 1.0

    return {
        "dct_benford_div": benford_div,
        "dct_ac_mean": dct_ac_mean,
        "dct_ac_std": dct_ac_std,
        "dct_ac_kurtosis": dct_ac_kurtosis,
        "blocking_strength": blocking,
    }


def _extract_edge_features(grey: np.ndarray) -> dict[str, float]:
    """Extract edge and structural features via Sobel gradients."""
    grey_f = grey.astype(np.float64)

    gx = cv2.Sobel(grey_f, cv2.CV_64F, 1, 0, ksize=3)
    gy = cv2.Sobel(grey_f, cv2.CV_64F, 0, 1, ksize=3)
    magnitude = np.sqrt(gx**2 + gy**2)
    direction = np.arctan2(gy, gx)

    mag_flat = magnitude.flatten()
    features: dict[str, float] = {
        "edge_mag_mean": float(np.mean(magnitude)),
        "edge_mag_std": float(np.std(magnitude)),
        "edge_mag_kurtosis": _kurtosis(mag_flat),
    }

    # Edge direction histogram
    dir_hist, _ = np.histogram(direction.flatten(), bins=36, range=(-np.pi, np.pi), density=True)
    dir_pos = dir_hist[dir_hist > 0]
    features["edge_dir_entropy"] = float(-np.sum(dir_pos * np.log2(dir_pos + 1e-10)))
    features["edge_dir_uniformity"] = float(np.sum(dir_hist**2))

    # Laplacian
    laplacian = cv2.Laplacian(grey_f, cv2.CV_64F)
    features["laplacian_var"] = float(np.var(laplacian))
    features["laplacian_mean"] = float(np.mean(np.abs(laplacian)))

    # Block-wise sharpness consistency
    block_size = 64
    h, w = grey.shape
    block_lap_vars = []
    for i in range(0, h - block_size, block_size):
        for j in range(0, w - block_size, block_size):
            block = grey_f[i : i + block_size, j : j + block_size]
            block_lap = cv2.Laplacian(block, cv2.CV_64F)
            block_lap_vars.append(float(np.var(block_lap)))

    if block_lap_vars:
        bv_mean = np.mean(block_lap_vars)
        features["sharpness_cv"] = float(np.std(block_lap_vars) / (bv_mean + 1e-10))
    else:
        features["sharpness_cv"] = 0.0

    return features


# ── Heuristic Scorer ──────────────────────────────────────────────────


def _heuristic_score(
    features: dict[str, float],
) -> tuple[float, list[DeepfakeSignal]]:
    """
    Rule-based scoring using known statistical indicators of AI generation.

    Returns (score, list_of_signals).
    """
    signals: list[DeepfakeSignal] = []

    def _add(
        name: str,
        weight: float,
        triggered: bool,
        desc_triggered: str,
        desc_normal: str,
    ) -> float:
        signals.append(
            DeepfakeSignal(
                name=name,
                weight=weight,
                triggered=triggered,
                description=desc_triggered if triggered else desc_normal,
            )
        )
        return (0.8 if triggered else 0.25) * weight

    components: list[float] = []

    # 1. Noise residual level
    noise_std = features.get("noise_std", 5.0)
    components.append(
        _add(
            "noise_residual",
            2.0,
            noise_std < 2.0,
            f"Very low noise residual (std={noise_std:.2f}), common in AI-generated images",
            f"Noise residual within normal range (std={noise_std:.2f})",
        )
    )

    # 2. Noise spatial autocorrelation (PRNU)
    ac_h = features.get("noise_autocorr_h1", 0.1)
    ac_v = features.get("noise_autocorr_v1", 0.1)
    mean_ac = (ac_h + ac_v) / 2
    components.append(
        _add(
            "noise_prnu",
            1.5,
            mean_ac < 0.05,
            f"Noise lacks spatial correlation (autocorr={mean_ac:.4f}), no camera sensor pattern detected",
            f"Noise shows spatial correlation consistent with camera sensor (autocorr={mean_ac:.4f})",
        )
    )

    # 3. Noise variance consistency
    noise_cv = features.get("noise_var_cv", 0.5)
    components.append(
        _add(
            "noise_consistency",
            1.5,
            noise_cv < 0.3,
            f"Unnaturally consistent noise across image blocks (CV={noise_cv:.3f})",
            f"Normal noise variance across blocks (CV={noise_cv:.3f})",
        )
    )

    # 4. High-frequency energy
    hf_ratio = features.get("hf_energy_ratio", 0.05)
    components.append(
        _add(
            "frequency_energy",
            1.5,
            hf_ratio < 0.01,
            f"Deficient high-frequency energy (ratio={hf_ratio:.5f}), suggests AI smoothing",
            f"Normal high-frequency energy distribution (ratio={hf_ratio:.5f})",
        )
    )

    # 5. Spectral decay slope
    beta = features.get("spectral_decay_beta", 2.0)
    out_of_range = beta > 2.5 or beta < 1.5
    components.append(
        _add(
            "spectral_decay",
            1.0,
            out_of_range,
            f"Unusual spectral decay (beta={beta:.2f}), expected 1.5-2.5 for natural images",
            f"Normal spectral decay (beta={beta:.2f})",
        )
    )

    # 6. Texture consistency (LBP)
    lbp_cv = features.get("lbp_block_var_cv", 0.5)
    components.append(
        _add(
            "texture_consistency",
            1.0,
            lbp_cv < 0.2,
            f"Unnaturally uniform texture patterns (LBP CV={lbp_cv:.3f})",
            f"Normal texture variation (LBP CV={lbp_cv:.3f})",
        )
    )

    # 7. Colour gamut
    gamut = features.get("color_gamut_coverage", 0.1)
    components.append(
        _add(
            "color_gamut",
            0.5,
            gamut < 0.05,
            f"Narrow colour gamut ({gamut:.3f}), may indicate limited AI colour range",
            f"Normal colour gamut usage ({gamut:.3f})",
        )
    )

    # 8. Sharpness consistency
    sharp_cv = features.get("sharpness_cv", 0.5)
    components.append(
        _add(
            "sharpness_consistency",
            1.0,
            sharp_cv < 0.3,
            f"Unnaturally consistent sharpness (CV={sharp_cv:.3f})",
            f"Normal sharpness variation (CV={sharp_cv:.3f})",
        )
    )

    total_weight = sum(s.weight for s in signals)
    score = sum(components) / total_weight

    return score, signals


# ── Helpers ───────────────────────────────────────────────────────────


def _generate_spectrum_heatmap(grey: np.ndarray) -> str:
    """Generate a frequency magnitude spectrum heatmap as base64 PNG."""
    grey_f = grey.astype(np.float64)
    f_transform = fft2(grey_f)
    f_shifted = fftshift(f_transform)
    magnitude = np.log1p(np.abs(f_shifted))

    # Normalise to 0-255
    mag_min = magnitude.min()
    mag_max = magnitude.max()
    if mag_max > mag_min:
        normalised = ((magnitude - mag_min) / (mag_max - mag_min) * 255).astype(np.uint8)
    else:
        normalised = np.zeros_like(magnitude, dtype=np.uint8)

    # Apply colour map
    heatmap_bgr = cv2.applyColorMap(normalised, cv2.COLORMAP_INFERNO)
    heatmap_rgb = cv2.cvtColor(heatmap_bgr, cv2.COLOR_BGR2RGB)

    buffer = io.BytesIO()
    Image.fromarray(heatmap_rgb).save(buffer, format="PNG")
    return base64.b64encode(buffer.getvalue()).decode("utf-8")


def _resize(img: np.ndarray, max_edge: int) -> np.ndarray:
    """Resize so longest edge is max_edge."""
    h, w = img.shape[:2]
    if max(h, w) <= max_edge:
        return img
    scale = max_edge / max(h, w)
    new_w = int(w * scale)
    new_h = int(h * scale)
    return cv2.resize(img, (new_w, new_h), interpolation=cv2.INTER_AREA)


def _safe_corrcoef(a: np.ndarray, b: np.ndarray) -> float:
    """Compute Pearson correlation, returning 0.0 for constant arrays."""
    if np.std(a) < 1e-10 or np.std(b) < 1e-10:
        return 0.0
    return float(np.corrcoef(a, b)[0, 1])


def _kurtosis(x: np.ndarray) -> float:
    """Compute excess kurtosis."""
    mean = np.mean(x)
    std = np.std(x)
    if std < 1e-10:
        return 0.0
    return float(np.mean(((x - mean) / std) ** 4) - 3.0)


def _skewness(x: np.ndarray) -> float:
    """Compute skewness."""
    mean = np.mean(x)
    std = np.std(x)
    if std < 1e-10:
        return 0.0
    return float(np.mean(((x - mean) / std) ** 3))
