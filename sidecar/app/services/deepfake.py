"""
Jura Trace Sidecar — AI-Generated Image Detection.

Detects AI-generated or synthetic images using an ensemble of statistical
features extracted from frequency domain, noise residuals, colour/texture
analysis, JPEG artefacts, and edge structure. Each feature category captures
different aspects of "naturalness" that distinguish real camera photographs
from AI-generated content (GANs, diffusion models, etc.).

The heuristic scorer combines 13 weighted signals into a single score.
A trained classifier (Random Forest / Gradient Boosting) can be plugged in
later once training data is assembled.
"""

import base64
import io
import math

import cv2
import numpy as np
from PIL import Image
from scipy.fft import fft2, fftshift, dctn
from skimage.feature import local_binary_pattern, graycomatrix, graycoprops

from app.models.schemas import DeepfakeResponse, DeepfakeSignal, WatermarkDetection

# Maximum analysis dimension (longest edge)
ANALYSIS_SIZE = 512

# Minimum image dimension for watermark decode
_WATERMARK_MIN_SIZE = 256

# Known SDXL 48-bit watermark pattern (from Stability AI detect.py)
_SDXL_PATTERN = [
    1, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0,
    1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 1, 1,
    1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0,
]


def detect_sd_watermark(image_bytes: bytes) -> list[WatermarkDetection]:
    """Detect invisible DWT watermarks from Stable Diffusion, SDXL, and Flux.

    Uses the ``invisible-watermark`` library's dwtDct method (no PyTorch).
    Returns an empty list if the library is not installed or image is too small.
    """
    try:
        from imwatermark import WatermarkDecoder
    except ImportError:
        return []

    try:
        pil_img = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    except Exception:
        return []

    w, h = pil_img.size
    if w < _WATERMARK_MIN_SIZE or h < _WATERMARK_MIN_SIZE:
        return []

    bgr = cv2.cvtColor(np.array(pil_img), cv2.COLOR_RGB2BGR)
    detections: list[WatermarkDetection] = []

    # --- SD v1 watermark (136-bit string "StableDiffusionV1") ---
    try:
        decoder_v1 = WatermarkDecoder("bytes", 136)
        wm_bytes = decoder_v1.decode(bgr, "dwtDct")
        decoded_str = wm_bytes.decode("utf-8", errors="ignore").rstrip("\x00")

        if decoded_str == "StableDiffusionV1":
            detections.append(WatermarkDetection(
                type="stable_diffusion_v1",
                detected=True,
                confidence=1.0,
                details="Exact Stable Diffusion v1 watermark decoded: 'StableDiffusionV1'",
            ))
    except Exception:
        pass

    # --- SDXL / Flux watermark (48-bit fixed pattern) ---
    try:
        decoder_xl = WatermarkDecoder("bits", 48)
        bits = decoder_xl.decode(bgr, "dwtDct")
        bits_list = [int(b) for b in bits]
        matching = sum(1 for a, b in zip(bits_list, _SDXL_PATTERN) if a == b)

        if matching >= 40:
            detections.append(WatermarkDetection(
                type="sdxl",
                detected=True,
                confidence=round(matching / 48.0, 3),
                details=f"SDXL/Flux watermark detected: {matching}/48 bits match (very likely)",
            ))
        elif matching >= 35:
            detections.append(WatermarkDetection(
                type="sdxl",
                detected=True,
                confidence=round(matching / 48.0, 3),
                details=f"SDXL/Flux watermark partially detected: {matching}/48 bits match (possible)",
            ))
    except Exception:
        pass

    return detections


# ── Codec Classification ─────────────────────────────────────────────────

# Per-codec threshold profiles. Calibrated against real mobile phone photos
# (iPhone, Android) with computational photography (Smart HDR, night mode,
# denoising) and lossy compression. Previous thresholds were calibrated
# against synthetic test images and produced false positives on ~80% of
# real camera photos.
#
# Key calibration findings from real iPhone JPEGs:
#   - noise_std: 1.8-3.0 (computational denoising), AI-generated: < 1.0
#   - hf_energy: 0.0003-0.001 (JPEG strips HF), AI-generated: < 0.0001
#   - noise_cv: 1.0-1.5 (indoor scenes), AI-generated: < 0.5
#   - lbp_cv: 0.08-0.15 (skin, walls, fabric), AI-generated: < 0.05
#   - spectral_decay beta: 2.5-3.2 (JPEG steepens), AI-generated: > 3.5 or < 1.0
#   - channel_corr: 0.97-0.99 (indoor), AI-generated: > 0.995
#   - multiscale_gradient: 1.5-2.6 (comp. photo), AI-generated: > 3.0
#   - benford_div: 0.1-0.3 (JPEG distorts), AI-generated: > 0.4
CODEC_THRESHOLDS: dict[str, dict[str, float]] = {
    "raw": {
        "noise_std": 1.5, "hf_energy": 0.0005, "noise_cv": 1.0,
        "lbp_cv": 0.08, "sharp_cv": 0.5, "patch_spec_cv": 0.8,
        "glcm_energy": 0.06,
    },
    "jpeg": {
        "noise_std": 1.0, "hf_energy": 0.0002, "noise_cv": 1.0,
        "lbp_cv": 0.08, "sharp_cv": 0.5, "patch_spec_cv": 0.8,
        "glcm_energy": 0.06,
    },
    "modern_lossy": {
        "noise_std": 0.8, "hf_energy": 0.0001, "noise_cv": 0.5,
        "lbp_cv": 0.05, "sharp_cv": 0.3, "patch_spec_cv": 0.6,
        "glcm_energy": 0.06,
    },
    "heavy_jpeg": {
        "noise_std": 0.8, "hf_energy": 0.0001, "noise_cv": 0.8,
        "lbp_cv": 0.06, "sharp_cv": 0.4, "patch_spec_cv": 0.7,
        "glcm_energy": 0.05,
    },
    "lossless": {
        "noise_std": 1.5, "hf_energy": 0.0005, "noise_cv": 1.0,
        "lbp_cv": 0.08, "sharp_cv": 0.5, "patch_spec_cv": 0.8,
        "glcm_energy": 0.06,
    },
}


def _classify_codec(mime_type: str) -> str:
    """Classify a MIME type into a codec profile for threshold selection."""
    mime_lower = mime_type.lower()
    if mime_lower in ("image/avif", "image/webp", "image/heic", "image/heif"):
        return "modern_lossy"
    if mime_lower in ("image/tiff", "image/x-adobe-dng", "image/x-canon-cr2",
                      "image/x-nikon-nef", "image/bmp"):
        return "raw"
    if mime_lower in ("image/png", "image/gif"):
        return "lossless"
    # Default: JPEG (most common)
    return "jpeg"


def perform_deepfake_detection(
    image_bytes: bytes,
    analysis_size: int = ANALYSIS_SIZE,
    mime_type: str = "image/jpeg",
    has_camera_exif: bool = False,
) -> DeepfakeResponse:
    """
    Detect AI-generated content in an image.

    Args:
        image_bytes: Raw bytes of the input image.
        analysis_size: Resize longest edge to this for consistent analysis.
        mime_type: MIME type of the source image for codec-aware thresholds.
        has_camera_exif: True if the image has camera EXIF data (make/model/exposure).

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
    features.update(_extract_patch_spectral_features(grey))
    features.update(_extract_multiscale_gradient_features(grey))

    # Score via heuristic ensemble with codec-aware thresholds
    codec_class = _classify_codec(mime_type)
    score, signals = _heuristic_score(
        features, codec_class, has_camera_exif=has_camera_exif,
    )

    # Generate frequency spectrum heatmap
    heatmap_base64 = _generate_spectrum_heatmap(grey)

    # Confidence based on score extremity
    if score > 0.70 or score < 0.20:
        confidence = "high"
    elif score > 0.55 or score < 0.30:
        confidence = "medium"
    else:
        confidence = "low"

    # Summary
    triggered_count = sum(1 for s in signals if s.triggered)
    if score > 0.55:
        summary = f"Strong synthetic indicators ({triggered_count} of {len(signals)} signals triggered)"
    elif score > 0.35:
        summary = f"Mixed indicators ({triggered_count} of {len(signals)} signals triggered)"
    else:
        summary = f"Image appears authentic ({triggered_count} of {len(signals)} signals triggered)"

    # Detect invisible AI watermarks (SD v1, SDXL, Flux)
    watermarks = detect_sd_watermark(image_bytes)
    if any(w.detected for w in watermarks):
        confidence = "high"
        wm_types = [w.type.replace("_", " ").title() for w in watermarks if w.detected]
        summary = f"AI watermark detected ({', '.join(wm_types)}). {summary}"

    # Three-way verdict: replaces binary suspicious/clean with honest uncertainty
    if score > 0.65 or any(w.detected for w in watermarks):
        verdict_level = "synthetic"
    elif score < 0.30:
        verdict_level = "authentic"
    else:
        verdict_level = "inconclusive"

    return DeepfakeResponse(
        score=round(score, 4),
        suspicious=score > 0.5,
        confidence=confidence,
        verdict_level=verdict_level,
        signals=signals,
        heatmap_base64=heatmap_base64,
        summary=summary,
        watermarks=watermarks,
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


def _extract_patch_spectral_features(grey: np.ndarray) -> dict[str, float]:
    """Extract per-patch spectral features to detect uniform AI frequency content.

    Real photos have diverse spectral content across spatial patches (textured
    regions vs smooth backgrounds). AI-generated images tend toward more uniform
    spectral distributions across patches.
    """
    h, w = grey.shape
    patch_size = 64
    grey_f = grey.astype(np.float64)

    hf_energies: list[float] = []
    for i in range(0, h - patch_size + 1, patch_size):
        for j in range(0, w - patch_size + 1, patch_size):
            patch = grey_f[i : i + patch_size, j : j + patch_size]
            f_transform = fft2(patch)
            f_shifted = fftshift(f_transform)
            power = np.abs(f_shifted) ** 2

            cy, cx = patch_size // 2, patch_size // 2
            y_coords, x_coords = np.ogrid[:patch_size, :patch_size]
            radius = np.sqrt((x_coords - cx) ** 2 + (y_coords - cy) ** 2)
            max_radius = min(cx, cy)

            hf_mask = radius > (max_radius * 0.75)
            total_energy = power.sum() + 1e-10
            hf_energy = power[hf_mask].sum() / total_energy
            hf_energies.append(float(hf_energy))

    if len(hf_energies) >= 2:
        mean_hf = float(np.mean(hf_energies))
        patch_spectral_cv = float(np.std(hf_energies) / (mean_hf + 1e-10))
    else:
        patch_spectral_cv = 1.0  # Default: assume natural diversity

    return {
        "patch_spectral_cv": patch_spectral_cv,
    }


def _extract_multiscale_gradient_features(grey: np.ndarray) -> dict[str, float]:
    """Extract gradient energy distribution across image scales.

    Real photos lose gradient energy naturally at coarser scales (fine textures
    and edges disappear). AI-generated images tend to maintain more uniform
    gradient energy across scales due to the generation process operating at
    multiple resolutions simultaneously.
    """
    grey_f = grey.astype(np.float64)

    # 3-level Gaussian pyramid: original, 0.5x, 0.25x
    levels = [grey_f]
    current = grey_f
    for _ in range(2):
        if current.shape[0] < 16 or current.shape[1] < 16:
            break
        current = cv2.pyrDown(current)
        levels.append(current)

    if len(levels) < 3:
        return {"multiscale_gradient_ratio": 0.4}  # Default: favour authentic

    # Mean Sobel gradient magnitude at each level
    grad_means: list[float] = []
    for level in levels:
        gx = cv2.Sobel(level, cv2.CV_64F, 1, 0, ksize=3)
        gy = cv2.Sobel(level, cv2.CV_64F, 0, 1, ksize=3)
        mag = np.sqrt(gx**2 + gy**2)
        grad_means.append(float(np.mean(mag)))

    # Ratio: coarsest / finest. Real photos ~0.3-0.6, AI ~0.6-0.9
    level0 = grad_means[0] + 1e-10
    level2 = grad_means[2]
    ratio = level2 / level0

    return {
        "multiscale_gradient_ratio": float(ratio),
    }


# ── Heuristic Scorer ──────────────────────────────────────────────────


def _compute_scene_complexity(features: dict[str, float]) -> float:
    """Compute a 0-1 scene complexity metric from existing features.

    Low complexity (< 0.3) indicates uniform scenes (fog, snow, overcast)
    where texture/sharpness signals should be downweighted to avoid
    false positives.
    """
    gamut = features.get("color_gamut_coverage", 0.1)
    sharp_cv = features.get("sharpness_cv", 0.5)
    lbp_cv = features.get("lbp_block_var_cv", 0.5)
    edge_mean = features.get("edge_mag_mean", 25.0)
    return (
        gamut * 0.3
        + min(sharp_cv, 1.0) * 0.3
        + min(lbp_cv, 1.0) * 0.2
        + min(edge_mean / 50.0, 1.0) * 0.2
    )


def _heuristic_score(
    features: dict[str, float],
    codec_class: str = "jpeg",
    has_camera_exif: bool = False,
) -> tuple[float, list[DeepfakeSignal]]:
    """
    Rule-based scoring using known statistical indicators of AI generation.

    Args:
        features: Extracted image features.
        codec_class: Codec profile ("raw", "jpeg", "modern_lossy", "heavy_jpeg", "lossless").
        has_camera_exif: True if image has camera EXIF (shifts sigmoid midpoint).

    Returns (score, list_of_signals).
    """
    signals: list[DeepfakeSignal] = []
    thresholds = CODEC_THRESHOLDS.get(codec_class, CODEC_THRESHOLDS["jpeg"])

    # Scoring uses sigmoid activation: the ratio of triggered signal weight
    # to total weight is mapped through a sigmoid (midpoint=0.18, k=12).
    # This gives:
    #   0% triggered  → score ~0.10 (authentic)
    #  10% triggered  → score ~0.27
    #  18% triggered  → score ~0.50 (suspicious threshold)
    #  30% triggered  → score ~0.81
    # 100% triggered  → score ~1.00
    #
    # Midpoint lowered from 0.25 to 0.18 to compensate for weight dilution
    # (total weight increased from 12.5 to 17.5 with new signals). Steepness
    # increased from 10 to 12 for sharper authentic/suspicious separation.
    #
    # Signals are designed to be robust against lossy codec artifacts (AVIF, WebP, JPEG)
    # and scene-dependent features (fog, smoke, soft backgrounds). Thresholds are
    # calibrated per codec class to avoid false positives on modern codecs.

    def _add(
        name: str,
        weight: float,
        triggered: bool,
        desc_triggered: str,
        desc_normal: str,
    ) -> None:
        signals.append(
            DeepfakeSignal(
                name=name,
                weight=weight,
                triggered=triggered,
                description=desc_triggered if triggered else desc_normal,
            )
        )

    # --- Noise domain (3 signals) ---

    # 1. Noise residual level — very low noise indicates GAN-style generators
    noise_std = features.get("noise_std", 5.0)
    noise_std_thresh = thresholds["noise_std"]
    _add(
        "noise_residual",
        1.0,
        noise_std < noise_std_thresh,
        f"Very low noise residual (std={noise_std:.2f}), common in AI-generated images",
        f"Noise residual within normal range (std={noise_std:.2f})",
    )

    # 2. Smoothed noise with heavy tails — compound signal. AI images have
    #    very low noise (std < 1.5) with non-Gaussian distribution (kurtosis > 30).
    #    Real camera photos with computational denoising have std 1.5-3.0 with
    #    high kurtosis (50-130) from JPEG blocking — so the noise_std threshold
    #    must be tight (< 1.5, not < 5.0) to avoid flagging every denoised photo.
    noise_kurt = features.get("noise_kurtosis", 3.0)
    smooth_kurtosis = noise_std < 1.5 and noise_kurt > 30.0
    _add(
        "noise_smoothed_kurtosis",
        1.5,
        smooth_kurtosis,
        f"Smooth noise with heavy tails (std={noise_std:.1f}, kurtosis={noise_kurt:.0f}), AI generation pattern",
        f"Noise profile consistent with camera/codec (std={noise_std:.1f}, kurtosis={noise_kurt:.0f})",
    )

    # 3. PRNU asymmetry — real camera sensors have roughly symmetric horizontal/vertical
    #    noise autocorrelation. Diffusion models produce strongly asymmetric patterns.
    ac_h = features.get("noise_autocorr_h1", 0.1)
    ac_v = features.get("noise_autocorr_v1", 0.1)
    prnu_asym = abs(ac_h - ac_v)
    _add(
        "prnu_asymmetry",
        1.5,
        prnu_asym > 0.3,
        f"Asymmetric noise correlation (H={ac_h:.3f}, V={ac_v:.3f}), inconsistent with camera PRNU",
        f"Symmetric noise correlation (H={ac_h:.3f}, V={ac_v:.3f}), consistent with camera sensor",
    )

    # 4. Noise variance consistency — AI images have more uniform block-wise
    #    noise variance than real photos with diverse scene content.
    noise_cv = features.get("noise_var_cv", 0.5)
    noise_cv_thresh = thresholds["noise_cv"]
    _add(
        "noise_consistency",
        1.5,
        noise_cv < noise_cv_thresh,
        f"Unnaturally uniform noise distribution (CV={noise_cv:.3f}), common in AI-generated images",
        f"Normal noise variance distribution (CV={noise_cv:.3f})",
    )

    # --- Frequency domain (2 signals) ---

    # 5. Extreme spectral smoothing — threshold is codec-aware to avoid
    #    false positives from AVIF/WebP lossy compression which strips HF.
    hf_ratio = features.get("hf_energy_ratio", 0.05)
    hf_thresh = thresholds["hf_energy"]
    _add(
        "frequency_energy",
        1.5,
        hf_ratio < hf_thresh,
        f"Severely deficient high-frequency energy (ratio={hf_ratio:.5f}), indicates AI smoothing",
        f"Adequate high-frequency content (ratio={hf_ratio:.5f})",
    )

    # 5. Spectral decay slope — natural images follow 1/f^beta. Uncompressed
    #    naturals have beta in 1.5-2.5, but JPEG compression + computational
    #    photography routinely push beta to 2.5-3.2. Only flag at > 3.5 or < 1.0.
    beta = features.get("spectral_decay_beta", 2.0)
    out_of_range = beta > 3.5 or beta < 1.0
    _add(
        "spectral_decay",
        1.5,
        out_of_range,
        f"Unusual spectral decay (beta={beta:.2f}), expected 1.5-2.5 for natural images",
        f"Normal spectral decay (beta={beta:.2f})",
    )

    # --- Texture domain (1 signal, high weight — strongest discriminator) ---

    # 6. LBP texture consistency — AI images have unnaturally uniform local texture
    #    across image blocks. Real photos vary more due to scene complexity.
    lbp_cv = features.get("lbp_block_var_cv", 0.5)
    lbp_cv_thresh = thresholds["lbp_cv"]
    _add(
        "texture_consistency",
        2.0,
        lbp_cv < lbp_cv_thresh,
        f"Unnaturally uniform texture patterns (LBP CV={lbp_cv:.3f})",
        f"Normal texture variation (LBP CV={lbp_cv:.3f})",
    )

    # --- Patch spectral domain (1 signal) ---

    # 8. Patch spectral variance — real photos have diverse high-frequency
    #    content across spatial patches (textured vs smooth regions). AI images
    #    show more uniform spectral patterns across patches.
    patch_spec_cv = features.get("patch_spectral_cv", 1.0)
    patch_spec_thresh = thresholds["patch_spec_cv"]
    _add(
        "patch_spectral_variance",
        2.0,
        patch_spec_cv < patch_spec_thresh,
        f"Uniform spectral content across patches (CV={patch_spec_cv:.3f}), typical of AI generation",
        f"Diverse spectral content across patches (CV={patch_spec_cv:.3f})",
    )

    # --- Colour domain (2 signals) ---

    # 9. Inter-channel correlation — AI images tend to have unnaturally high
    #    correlation between R, G, B channels. Indoor photos with limited
    #    lighting naturally reach 0.97-0.99, so threshold must be > 0.995.
    corr_rg = features.get("color_corr_rg", 0.8)
    corr_rb = features.get("color_corr_rb", 0.8)
    corr_gb = features.get("color_corr_gb", 0.8)
    mean_corr = (corr_rg + corr_rb + corr_gb) / 3
    _add(
        "channel_correlation",
        1.0,
        mean_corr > 0.995,
        f"Unusually high colour channel correlation ({mean_corr:.3f}), common in AI-generated images",
        f"Normal colour channel variation ({mean_corr:.3f})",
    )

    # 8. Colour gamut
    gamut = features.get("color_gamut_coverage", 0.1)
    _add(
        "color_gamut",
        0.5,
        gamut < 0.05,
        f"Narrow colour gamut ({gamut:.3f}), may indicate limited AI colour range",
        f"Normal colour gamut usage ({gamut:.3f})",
    )

    # --- Texture structure domain (1 signal) ---

    # 14. GLCM energy (angular second moment) — measures texture regularity.
    #     Real photos have structured, locally correlated textures with
    #     GLCM energy > 0.06. AI generators produce more dispersed
    #     co-occurrence patterns with lower energy.
    glcm_energy = features.get("glcm_energy_mean", 0.1)
    glcm_energy_thresh = thresholds.get("glcm_energy", 0.06)
    _add(
        "glcm_texture_structure",
        1.5,
        glcm_energy < glcm_energy_thresh,
        f"Dispersed texture structure (GLCM energy={glcm_energy:.4f}), common in AI-generated images",
        f"Natural texture structure (GLCM energy={glcm_energy:.4f})",
    )

    # --- Structure domain (1 signal) ---

    # 9. Sharpness consistency — real photos have large sharpness variation
    #    (sharp foreground, soft background). AI images have more uniform sharpness.
    sharp_cv = features.get("sharpness_cv", 0.5)
    sharp_cv_thresh = thresholds["sharp_cv"]
    _add(
        "sharpness_consistency",
        1.5,
        sharp_cv < sharp_cv_thresh,
        f"Unnaturally consistent sharpness (CV={sharp_cv:.3f})",
        f"Normal sharpness variation (CV={sharp_cv:.3f})",
    )

    # --- Multi-scale gradient domain (1 signal) ---

    # 12. Multi-scale gradient uniformity — real photos lose gradient energy
    #     at coarser scales as fine detail disappears. AI images maintain more
    #     uniform gradient energy across scales. Real camera photos with
    #     computational photography (Smart HDR, denoising) show ratios of
    #     1.5-2.6, so threshold must be > 3.0 to avoid flagging real photos.
    grad_ratio = features.get("multiscale_gradient_ratio", 0.4)
    _add(
        "multiscale_gradient",
        1.0,
        grad_ratio > 3.0,
        f"Uniform gradient energy across scales (ratio={grad_ratio:.3f}), suggests AI generation",
        f"Natural gradient energy falloff across scales (ratio={grad_ratio:.3f})",
    )

    # --- DCT domain (1 signal) ---

    # 13. Benford's law divergence — first digits of DCT coefficients in natural
    #     images follow Benford's distribution. JPEG compression naturally
    #     distorts this (real photos show 0.1-0.3), so only flag above 0.4.
    benford_div = features.get("dct_benford_div", 0.0)
    _add(
        "benford_divergence",
        0.5,
        benford_div > 0.4,
        f"DCT coefficients deviate from Benford's law (div={benford_div:.3f}), uncommon in natural images",
        f"DCT statistics follow expected distribution (div={benford_div:.3f})",
    )

    # ── Scene complexity adaptation ────────────────────────────────────
    # Low-complexity scenes (fog, snow, overcast) naturally have uniform
    # texture and sharpness. Halve those signal weights to avoid FPs.
    scene_complexity = _compute_scene_complexity(features)
    if scene_complexity < 0.3:
        scene_affected = {"texture_consistency", "patch_spectral_variance", "sharpness_consistency"}
        for s in signals:
            if s.name in scene_affected:
                s.weight *= 0.5

    # ── Anti-correlation penalty ─────────────────────────────────────
    # When texture/sharpness signals trigger but noise/frequency signals
    # do NOT, this pattern indicates a scene characteristic, not AI.
    texture_names = {"texture_consistency", "patch_spectral_variance", "sharpness_consistency"}
    noise_freq_names = {"noise_residual", "noise_smoothed_kurtosis", "prnu_asymmetry",
                        "noise_consistency", "frequency_energy", "spectral_decay"}
    texture_triggered = any(s.triggered for s in signals if s.name in texture_names)
    noise_freq_triggered = any(s.triggered for s in signals if s.name in noise_freq_names)

    # Sigmoid activation: ratio of triggered weight → score via sigmoid
    total_weight = sum(s.weight for s in signals)
    triggered_weight = sum(s.weight for s in signals if s.triggered)
    activation_ratio = triggered_weight / total_weight if total_weight > 0 else 0.0

    # Apply anti-correlation penalty: texture-only triggers → reduce ratio by 40%
    if texture_triggered and not noise_freq_triggered:
        activation_ratio *= 0.6

    # EXIF-informed sigmoid midpoint: camera EXIF is a strong prior toward
    # authenticity — require more evidence (higher midpoint) to flag.
    midpoint = 0.25 if has_camera_exif else 0.18
    score = 1.0 / (1.0 + math.exp(-12.0 * (activation_ratio - midpoint)))

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
