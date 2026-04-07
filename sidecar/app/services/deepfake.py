"""
Jura Trace Sidecar — AI-Generated Image Detection.

Detects AI-generated or synthetic images using an ensemble of statistical
features extracted from frequency domain, noise residuals, colour/texture
analysis, JPEG artefacts, and edge structure. Each feature category captures
different aspects of "naturalness" that distinguish real camera photographs
from AI-generated content (GANs, diffusion models, etc.).

The heuristic scorer combines 21 weighted signals into a single score.
A trained classifier (Random Forest / Gradient Boosting) can be plugged in
later once training data is assembled.
"""

import base64
import hashlib
import io
import logging
import math
import os
import sys

import cv2
import numpy as np
from PIL import Image
from scipy.fft import fft2, fftshift, dctn
from scipy.ndimage import laplace
from skimage.feature import local_binary_pattern, graycomatrix, graycoprops
from skimage.restoration import denoise_wavelet

from app.models.schemas import DeepfakeResponse, DeepfakeSignal, WatermarkDetection

logger = logging.getLogger(__name__)


def _resolve_models_dir() -> str:
    """Resolve the models directory. Supports both dev and frozen (PyInstaller) contexts.

    In a frozen binary (sys.frozen == True), the caller (Tauri Rust backend) is
    expected to set JURA_MODELS_DIR to the absolute path of the models/ directory
    shipped alongside the binary. In dev mode the variable is typically unset, so
    we fall back to the path relative to this source file.
    """
    env_dir = os.environ.get("JURA_MODELS_DIR")
    if env_dir and os.path.isdir(env_dir):
        return env_dir
    # Dev mode: sidecar/app/services/deepfake.py → ../../.. → sidecar/ → ../models/
    return os.path.normpath(
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "models")
    )


MODELS_DIR: str = _resolve_models_dir()

# Maximum analysis dimension (longest edge)
ANALYSIS_SIZE = 512

# ── Stable Feature Vector ──────────────────────────────────────────────
# Ordered list of every feature key produced by the 8 core extractors
# plus the 5 new-signal extractors. Conditional (lossless-only) features
# are included at the end; they default to NaN when not applicable.

FEATURE_NAMES: list[str] = [
    # _extract_frequency_features (9)
    "spectral_decay_beta",
    "hf_energy_ratio",
    "mf_energy_ratio",
    "hf_to_mf_ratio",
    "az_var_band_0",
    "az_var_band_1",
    "az_var_band_2",
    "az_var_band_3",
    "spectral_entropy",
    # _extract_noise_features (9)
    "noise_mean_abs",
    "noise_std",
    "noise_kurtosis",
    "noise_skewness",
    "noise_autocorr_h1",
    "noise_autocorr_v1",
    "noise_autocorr_d1",
    "noise_var_cv",
    "noise_spectral_flatness",
    # _extract_color_features (22)
    "color_b_mean",
    "color_b_std",
    "color_b_skew",
    "color_b_kurt",
    "color_b_entropy",
    "color_g_mean",
    "color_g_std",
    "color_g_skew",
    "color_g_kurt",
    "color_g_entropy",
    "color_r_mean",
    "color_r_std",
    "color_r_skew",
    "color_r_kurt",
    "color_r_entropy",
    "color_corr_rg",
    "color_corr_rb",
    "color_corr_gb",
    "sat_mean",
    "sat_std",
    "sat_kurtosis",
    "color_gamut_coverage",
    # _extract_texture_features (15)
    "lbp_entropy",
    "lbp_uniformity",
    "lbp_mean",
    "lbp_var",
    "lbp_block_var_mean",
    "lbp_block_var_std",
    "lbp_block_var_cv",
    "glcm_contrast_mean",
    "glcm_contrast_std",
    "glcm_homogeneity_mean",
    "glcm_homogeneity_std",
    "glcm_energy_mean",
    "glcm_energy_std",
    "glcm_correlation_mean",
    "glcm_correlation_std",
    # _extract_jpeg_features (5)
    "dct_benford_div",
    "dct_ac_mean",
    "dct_ac_std",
    "dct_ac_kurtosis",
    "blocking_strength",
    # _extract_edge_features (9)
    "edge_mag_mean",
    "edge_mag_std",
    "edge_mag_kurtosis",
    "edge_dir_entropy",
    "edge_dir_uniformity",
    "laplacian_var",
    "laplacian_mean",
    "sharpness_cv",
    # _extract_patch_spectral_features (1)
    "patch_spectral_cv",
    # _extract_multiscale_gradient_features (1)
    "multiscale_gradient_ratio",
    # _extract_noise_autocorrelation (1)
    "noise_autocorr_tau",
    # _extract_cross_channel_noise (2)
    "cross_channel_noise_corr_mean",
    "cross_channel_noise_corr_max",
    # _extract_vae_grid_artefacts (1)
    "vae_grid_energy_ratio",
    # _extract_ca_absence (1)
    "ca_radial_trend",
    # _extract_saturation_luminance (1)
    "sat_lum_extreme_ratio",
    # _extract_bitplane_regularity (lossless-only, 2)
    "lsb_randomness",
    "lsb_entropy_mean",
    # _extract_demosaicing_traces (lossless-only, 2)
    "demosaic_peak_count",
    "demosaic_peak_strength",
    # ── Sprint 29 Track 3 — camera ISP vs VAE discriminators (4) ────────
    # These four features are appended AFTER all 80 original features so
    # that existing GBM v4 models (trained on features[0:80]) remain valid.
    # At inference time the classifier's ``n_features_in_`` attribute is
    # checked; if it is 80 the vector is trimmed to the first 80 values
    # before calling ``predict_proba``.  New models retrained on all 84
    # features will set ``n_features_in_ = 84`` and receive the full vector.
    # _extract_noise_lf_hf_ratio (1)
    "noise_lf_hf_ratio",
    # _extract_demosaic_inter_channel_coherence (1)
    "demosaic_inter_channel_coherence",
    # _extract_noise_anisotropy (2)
    "noise_anisotropy_mean",
    "noise_anisotropy_std",
]


def extract_feature_vector(features: dict[str, float]) -> list[float]:
    """Return feature values in FEATURE_NAMES order.

    Missing keys are filled with ``float('nan')``.
    """
    return [features.get(name, float("nan")) for name in FEATURE_NAMES]


def extract_features_for_training(
    image_bytes: bytes,
    mime_type: str = "image/jpeg",
    has_camera_exif: bool = False,
) -> tuple[dict[str, float], str]:
    """Extract the raw feature dict and codec class for training.

    Returns:
        (features_dict, codec_class) — the features dict contains all keys
        produced by the extractors.  codec_class is one of "raw", "jpeg",
        "modern_lossy", "lossless".
    """
    pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    img_array = np.array(pil_image)
    img_array = _resize(img_array, ANALYSIS_SIZE)
    img_bgr = cv2.cvtColor(img_array, cv2.COLOR_RGB2BGR)
    grey = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)

    features: dict[str, float] = {}
    features.update(_extract_frequency_features(grey))
    features.update(_extract_noise_features(img_bgr))
    features.update(_extract_color_features(img_bgr))
    features.update(_extract_texture_features(grey))
    features.update(_extract_jpeg_features(grey))
    features.update(_extract_edge_features(grey))
    features.update(_extract_patch_spectral_features(grey))
    features.update(_extract_multiscale_gradient_features(grey))
    features.update(_extract_noise_autocorrelation(grey))
    features.update(_extract_cross_channel_noise(img_bgr))
    features.update(_extract_vae_grid_artefacts(grey))
    features.update(_extract_ca_absence(img_bgr))
    features.update(_extract_saturation_luminance(img_bgr))

    codec_class = _classify_codec(mime_type)
    if codec_class == "lossless":
        features.update(_extract_bitplane_regularity(grey))
        features.update(_extract_demosaicing_traces(img_bgr))

    # Sprint 29 Track 3 — camera ISP vs VAE discriminators (appended last)
    features.update(_extract_noise_lf_hf_ratio(grey))
    features.update(_extract_demosaic_inter_channel_coherence(img_bgr))
    features.update(_extract_noise_anisotropy(grey))

    return features, codec_class


# ── Trained Classifier (lazy-loaded) ───────────────────────────────────
_classifier = None
_classifier_loaded = False


_DEEPFAKE_CLASSIFIER_SHA256 = (
    "2931f197cba6f376e85b1cbcfd584e6802f36e4fbf68ff00c83d61d4d655db18"
)


def _check_file_sha256(path: str, expected: str) -> bool:
    """Return True if the SHA-256 of *path* matches *expected* (hex string)."""
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest() == expected


def _load_classifier():
    """Attempt to load the trained GBM classifier. Returns None on failure.

    Performs a SHA-256 integrity check before loading. If the hash does not
    match the expected value the model is not loaded and a WARNING is logged.
    This prevents a tampered or corrupted model file from being used silently.
    """
    global _classifier, _classifier_loaded
    if _classifier_loaded:
        return _classifier
    _classifier_loaded = True
    try:
        import joblib
        model_path = os.path.join(MODELS_DIR, "deepfake_classifier.joblib")
        if os.path.exists(model_path):
            if not _check_file_sha256(model_path, _DEEPFAKE_CLASSIFIER_SHA256):
                logger.warning(
                    "deepfake_classifier.joblib failed SHA-256 integrity check — "
                    "refusing to load. Re-train or restore from a trusted source."
                )
                return None
            _classifier = joblib.load(model_path)
    except Exception:
        _classifier = None
    return _classifier

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


# ── Screenshot Pre-Classifier ─────────────────────────────────────────────

# Common display resolutions (width, height). Both orientations are checked.
_SCREEN_RESOLUTIONS: set[tuple[int, int]] = {
    # Desktop 16:9
    (1920, 1080), (2560, 1440), (3840, 2160), (1366, 768), (1600, 900),
    (1536, 864), (1280, 720), (1280, 800),
    # macOS Retina / non-Retina
    (1440, 900), (2880, 1800), (1680, 1050), (3360, 2100),
    (2560, 1600), (3024, 1964), (2880, 1920),
    # Ultrawide
    (3440, 1440), (2560, 1080),
    # Phone (portrait)
    (750, 1334), (1170, 2532), (1284, 2778), (1080, 2400),
    (1080, 1920), (1440, 2560), (1440, 3200), (1080, 2340),
    (828, 1792), (1125, 2436), (1242, 2688),
    # Tablets
    (2048, 2732), (2360, 1640), (2388, 1668), (2732, 2048),
}


def is_likely_screenshot(image_bytes: bytes) -> tuple[bool, float, dict[str, float]]:
    """Detect whether an image is likely a screenshot (UI render, not a photograph).

    Screenshots share several features with AI-generated images — no EXIF, PNG
    format, very low noise, high LBP uniformity — which cause the heuristic
    ensemble to produce false positives. This lightweight pre-classifier
    (~5-20 ms) gates obvious screenshots away from the full detection pipeline.

    Returns:
        (is_screenshot, confidence, signal_dict) where signal_dict contains
        the individual signal values for diagnostic/testing purposes.
    """
    try:
        pil_image = Image.open(io.BytesIO(image_bytes))
    except Exception:
        return False, 0.0, {}

    signals: dict[str, float] = {}

    # ── 1. Format: screenshots are almost always PNG ──────────────────
    is_png = pil_image.format == "PNG"
    signals["png_format"] = 1.0 if is_png else 0.0

    # ── 2. EXIF: screenshots have no camera metadata ─────────────────
    exif = pil_image.getexif()
    has_exif = len(exif) > 0
    signals["no_exif"] = 0.0 if has_exif else 1.0

    # Convert to greyscale array for analysis
    img_rgb = pil_image.convert("RGB")
    arr = np.array(img_rgb)
    if len(arr.shape) == 3:
        grey = np.mean(arr, axis=2)
    else:
        grey = arr.astype(np.float64)

    # ── 3. Noise variance via Laplacian (robust, edge-excluded) ─────
    # Rendered pixels have near-zero noise; camera sensors always add noise.
    # Global Laplacian variance is inflated by sharp UI edges, so we use
    # the *median* absolute Laplacian — robust to the sparse edge pixels
    # that dominate variance in screenshots.
    # Screenshots typically: median_abs_lap < 0.5. Photos: > 2.0.
    # AI-generated PNGs: 0.5-3.0 (model-dependent).
    lap = laplace(grey)
    abs_lap = np.abs(lap)
    median_abs_lap = float(np.median(abs_lap))
    if median_abs_lap < 0.5:
        signals["low_noise"] = 1.0
    elif median_abs_lap < 2.0:
        signals["low_noise"] = 0.5
    else:
        signals["low_noise"] = 0.0

    # ── 4. Solid-colour region ratio ─────────────────────────────────
    # UI elements produce large uniform blocks (backgrounds, panels, bars).
    # Photographs rarely have blocks with std < 3.0.
    h, w = grey.shape
    block_h, block_w = max(h // 16, 1), max(w // 16, 1)
    n_rows, n_cols = h // block_h, w // block_w
    if n_rows > 0 and n_cols > 0:
        uniform_blocks = 0
        total_blocks = 0
        for by in range(n_rows):
            for bx in range(n_cols):
                block = grey[by * block_h:(by + 1) * block_h,
                             bx * block_w:(bx + 1) * block_w]
                if np.std(block) < 3.0:
                    uniform_blocks += 1
                total_blocks += 1
        solid_ratio = uniform_blocks / total_blocks if total_blocks > 0 else 0.0
    else:
        solid_ratio = 0.0
    # Screenshots typically have 15-60% solid blocks; photos < 5%
    if solid_ratio > 0.25:
        signals["solid_regions"] = 1.0
    elif solid_ratio > 0.10:
        signals["solid_regions"] = 0.5
    else:
        signals["solid_regions"] = 0.0

    # ── 5. Screen resolution match ───────────────────────────────────
    iw, ih = pil_image.size
    is_screen_res = (iw, ih) in _SCREEN_RESOLUTIONS or (ih, iw) in _SCREEN_RESOLUTIONS
    signals["screen_resolution"] = 1.0 if is_screen_res else 0.0

    # ── 6. Sharp 1-pixel edges (UI borders, text) ────────────────────
    # Rendered UI has perfectly crisp single-step transitions that never
    # occur in camera photos (which always have gradual gradients due to
    # lens blur and sensor interpolation). We detect both horizontal and
    # vertical sharp transitions — isolated large gradient pixels where
    # adjacent gradient values are near zero.
    if w > 4 and h > 4:
        # Horizontal gradients
        h_grad = np.abs(np.diff(grey, axis=1))
        left_calm = np.pad(h_grad[:, :-1], ((0, 0), (1, 0)), constant_values=0) < 5
        right_calm = np.pad(h_grad[:, 1:], ((0, 0), (0, 1)), constant_values=0) < 5
        h_sharp = (h_grad > 20) & left_calm & right_calm

        # Vertical gradients
        v_grad = np.abs(np.diff(grey, axis=0))
        top_calm = np.pad(v_grad[:-1, :], ((1, 0), (0, 0)), constant_values=0) < 5
        bot_calm = np.pad(v_grad[1:, :], ((0, 1), (0, 0)), constant_values=0) < 5
        v_sharp = (v_grad > 20) & top_calm & bot_calm

        total_pixels = max(h_grad.size + v_grad.size, 1)
        sharp_count = float(np.sum(h_sharp) + np.sum(v_sharp))
        sharp_edge_ratio = sharp_count / total_pixels
        # Screenshots: 0.1-2% sharp single-step edges; photos: < 0.02%
        if sharp_edge_ratio > 0.001:
            signals["sharp_edges"] = 1.0
        elif sharp_edge_ratio > 0.0005:
            signals["sharp_edges"] = 0.5
        else:
            signals["sharp_edges"] = 0.0
    else:
        signals["sharp_edges"] = 0.0

    # ── 7. Colour channel uniqueness ─────────────────────────────────
    # Screenshots use limited colour palettes compared to photos.
    # Subsample for speed.
    sample = arr[::4, ::4].reshape(-1, 3) if arr.shape[0] > 8 and arr.shape[1] > 8 else arr.reshape(-1, 3)
    unique_colours = len(np.unique(sample, axis=0))
    total_pixels = len(sample)
    colour_ratio = unique_colours / total_pixels if total_pixels > 0 else 1.0
    # Screenshots: typically < 10% unique colours; photos: 60-95%
    if colour_ratio < 0.05:
        signals["limited_palette"] = 1.0
    elif colour_ratio < 0.15:
        signals["limited_palette"] = 0.5
    else:
        signals["limited_palette"] = 0.0

    # ── Weighted composite score ─────────────────────────────────────
    weights = {
        "png_format": 0.10,
        "no_exif": 0.15,
        "low_noise": 0.20,
        "solid_regions": 0.20,
        "screen_resolution": 0.05,
        "sharp_edges": 0.15,
        "limited_palette": 0.15,
    }

    score = sum(weights[name] * signals[name] for name in weights)
    is_screenshot_flag = score > 0.60

    return is_screenshot_flag, round(score, 4), signals


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
        "noise_std": 0.5, "hf_energy": 0.00003, "noise_cv": 0.3,
        "lbp_cv": 0.04, "sharp_cv": 0.2, "patch_spec_cv": 0.4,
        "glcm_energy": 0.04,
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
        "noise_std": 2.5, "hf_energy": 0.001, "noise_cv": 0.7,
        "lbp_cv": 0.06, "sharp_cv": 0.4, "patch_spec_cv": 0.6,
        "glcm_energy": 0.05,
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


def perform_deepfake_detection_with_features(
    image_bytes: bytes,
    analysis_size: int = ANALYSIS_SIZE,
    mime_type: str = "image/jpeg",
    has_camera_exif: bool = False,
    univfd_score: float | None = None,
    camera_authenticity_bonus: float = 0.0,
) -> tuple[DeepfakeResponse, dict[str, float]]:
    """
    Detect AI-generated content and return both the response and feature dict.

    Returns:
        Tuple of (DeepfakeResponse, feature_dict). The feature dict contains
        all extracted features including at minimum ``noise_std``,
        ``spectral_decay_beta``, and ``glcm_contrast_mean`` keys.
    """
    return _perform_deepfake_detection_impl(
        image_bytes, analysis_size, mime_type, has_camera_exif, univfd_score,
        camera_authenticity_bonus,
    )


def perform_deepfake_detection(
    image_bytes: bytes,
    analysis_size: int = ANALYSIS_SIZE,
    mime_type: str = "image/jpeg",
    has_camera_exif: bool = False,
    univfd_score: float | None = None,
    camera_authenticity_bonus: float = 0.0,
) -> DeepfakeResponse:
    """
    Detect AI-generated content in an image.

    Args:
        image_bytes: Raw bytes of the input image.
        analysis_size: Resize longest edge to this for consistent analysis.
        mime_type: MIME type of the source image for codec-aware thresholds.
        has_camera_exif: True if the image has camera EXIF data (make/model/exposure).
        univfd_score: Optional UnivFD probe score (0.0–1.0) to blend in.
        camera_authenticity_bonus: MakerNote-derived camera-origin confidence
            (0.0–1.0). Suppresses the final score proportionally to mitigate
            false positives on computational photography output.

    Returns:
        DeepfakeResponse with score, signals, and heatmap.

    Raises:
        ValueError: If image cannot be decoded.
    """
    response, _features = _perform_deepfake_detection_impl(
        image_bytes, analysis_size, mime_type, has_camera_exif, univfd_score,
        camera_authenticity_bonus,
    )
    return response


def _perform_deepfake_detection_impl(
    image_bytes: bytes,
    analysis_size: int = ANALYSIS_SIZE,
    mime_type: str = "image/jpeg",
    has_camera_exif: bool = False,
    univfd_score: float | None = None,
    camera_authenticity_bonus: float = 0.0,
) -> tuple[DeepfakeResponse, dict[str, float]]:
    """Internal implementation shared by both public entry points."""
    try:
        pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
        img_array = np.array(pil_image)
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    # ── Minimum size guard ───────────────────────────────────────────
    # Images smaller than 128x128 produce unreliable forensic signals
    # because upscaling to analysis_size creates interpolation artefacts
    # that mimic AI-generated texture smoothness. Return an honest
    # "insufficient data" response rather than a misleading score.
    orig_h, orig_w = img_array.shape[:2]
    if orig_h < 128 or orig_w < 128:
        return DeepfakeResponse(
            score=0.0,
            suspicious=False,
            confidence="low",
            verdict_level="authentic",
            signals=[],
            heatmap_base64="",
            summary=f"Image too small for reliable analysis ({orig_w}x{orig_h}). "
                    f"Minimum 128x128 required for forensic detection.",
            watermarks=[],
        ), {}

    # ── Screenshot pre-classifier ─────────────────────────────────────
    # Screenshots (UI renders) share features with AI images — no EXIF,
    # PNG format, uniform noise, high LBP uniformity — causing false
    # positives. Gate high-confidence screenshots away from the ensemble.
    screenshot_flag, screenshot_conf, screenshot_signals = is_likely_screenshot(image_bytes)
    if screenshot_flag and screenshot_conf > 0.70:
        # Before bypassing, check for AI-specific signals that override
        # the screenshot classification. AI-generated PNGs can look like
        # screenshots (no EXIF, uniform noise) but have subtle texture
        # variation that real screenshots lack.
        override_screenshot = False

        # Check 1: Texture complexity via local standard deviation
        # Real screenshots have very low texture complexity (< 5.0) because
        # pixels are rendered. AI images have subtle noise/texture (> 8.0)
        # even when they appear smooth.
        if len(img_array.shape) == 3:
            _grey = np.mean(img_array, axis=2)
        else:
            _grey = img_array.astype(np.float64)

        # Compute local std in 8x8 patches, take the mean of non-zero patches
        _h, _w = _grey.shape
        _bh, _bw = max(_h // 16, 1), max(_w // 16, 1)
        _local_stds = []
        for _by in range(min(16, _h // _bh)):
            for _bx in range(min(16, _w // _bw)):
                _block = _grey[_by * _bh:(_by + 1) * _bh, _bx * _bw:(_bx + 1) * _bw]
                _local_stds.append(float(np.std(_block)))
        _nonzero_stds = [s for s in _local_stds if s > 1.0]
        _texture_complexity = float(np.mean(_nonzero_stds)) if _nonzero_stds else 0.0

        # Check 2: Colour gradient smoothness
        # AI images have smooth colour gradients across the entire image.
        # Screenshots have sharp colour boundaries (UI elements).
        _h_grad = np.abs(np.diff(_grey, axis=1))
        _v_grad = np.abs(np.diff(_grey, axis=0))
        _mean_grad = float(np.mean(_h_grad) + np.mean(_v_grad)) / 2.0
        _grad_std = float(np.std(_h_grad) + np.std(_v_grad)) / 2.0
        # AI images: moderate mean gradient, low std (smooth transitions)
        # Screenshots: low mean gradient but HIGH std (sharp UI edges)
        _smooth_gradients = _mean_grad > 3.0 and _grad_std < _mean_grad * 3.0

        # Check 3: Colour diversity in non-uniform regions
        # AI images have rich colour variation even in "uniform" areas.
        # Screenshots have exact repeated colours (flat UI fills).
        _sample = img_array[::4, ::4].reshape(-1, 3) if img_array.shape[0] > 8 and img_array.shape[1] > 8 else img_array.reshape(-1, 3)
        _unique_ratio = len(np.unique(_sample, axis=0)) / max(len(_sample), 1)

        # Override conditions: AI-like texture in a "screenshot"
        # Real screenshots have texture_complexity < 8 (rendered pixels).
        # AI images that look like screenshots have complexity 15-60+.
        if _texture_complexity > 15.0:
            # High texture alone is sufficient — real screenshots never
            # have this much local variation in non-edge regions.
            override_screenshot = True
            logger.info(
                "Screenshot override: texture_complexity=%.1f (>15). "
                "Real screenshots never have this much texture variation.",
                _texture_complexity,
            )
        elif _texture_complexity > 10.0 and _smooth_gradients:
            override_screenshot = True
            logger.info(
                "Screenshot override: texture_complexity=%.1f (>10), smooth_gradients=True. "
                "Image has AI-like texture despite screenshot-like features.",
                _texture_complexity,
            )
        elif _texture_complexity > 8.0 and _unique_ratio > 0.15:
            override_screenshot = True
            logger.info(
                "Screenshot override: texture_complexity=%.1f (>8), colour_diversity=%.2f (>0.15). "
                "Image has AI-like colour richness despite screenshot-like features.",
                _texture_complexity, _unique_ratio,
            )

        if not override_screenshot:
            logger.info(
                "Screenshot pre-classifier triggered (confidence=%.2f, "
                "texture=%.1f, signals=%s). Bypassing deepfake ensemble.",
                screenshot_conf, _texture_complexity, screenshot_signals,
            )
            # Generate a minimal heatmap from the decoded image for UI consistency
            img_resized = _resize(img_array, analysis_size)
            grey_for_heatmap = cv2.cvtColor(img_resized, cv2.COLOR_RGB2GRAY)
            heatmap_base64 = _generate_spectrum_heatmap(grey_for_heatmap)

            response = DeepfakeResponse(
                score=0.15,
                suspicious=False,
                confidence="low",
                verdict_level="authentic",
                signals=[
                    DeepfakeSignal(
                        name="screenshot_detected",
                        description=(
                            f"Image identified as a screenshot (confidence: {screenshot_conf:.0%}). "
                            f"Screenshot characteristics (uniform noise, no camera metadata, "
                            f"solid-colour regions) are expected and do not indicate AI generation."
                        ),
                        weight=0.0,
                        triggered=False,
                    ),
                ],
                heatmap_base64=heatmap_base64,
                summary=(
                    f"Image identified as a screenshot (confidence: {screenshot_conf:.0%}). "
                    f"Screenshot characteristics are expected and do not indicate AI generation."
                ),
                watermarks=[],
                classifier_score=None,
                classifier_available=False,
                univfd_score=None,
                univfd_available=False,
            )
            return response, {"screenshot_confidence": screenshot_conf}
        else:
            logger.info(
                "Screenshot pre-classifier overridden — AI-like texture detected. "
                "Running full deepfake ensemble."
            )

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

    # New signals (15-21)
    features.update(_extract_noise_autocorrelation(grey))
    features.update(_extract_cross_channel_noise(img_bgr))
    features.update(_extract_vae_grid_artefacts(grey))
    features.update(_extract_ca_absence(img_bgr))
    features.update(_extract_saturation_luminance(img_bgr))

    # Codec-aware classification (needed for conditional extractors)
    codec_class = _classify_codec(mime_type)

    # Lossless-only extractors
    if codec_class == "lossless":
        features.update(_extract_bitplane_regularity(grey))
        features.update(_extract_demosaicing_traces(img_bgr))

    # Sprint 29 Track 3 — camera ISP vs VAE discriminators (appended last)
    features.update(_extract_noise_lf_hf_ratio(grey))
    features.update(_extract_demosaic_inter_channel_coherence(img_bgr))
    features.update(_extract_noise_anisotropy(grey))

    # Score via heuristic ensemble with codec-aware thresholds
    heuristic_score, signals = _heuristic_score(
        features, codec_class, has_camera_exif=has_camera_exif,
    )

    # ── Classifier blending ────────────────────────────────────────────
    classifier_score = None
    classifier_available = False
    clf = _load_classifier()
    if clf is not None:
        try:
            vec = extract_feature_vector(features)
            import numpy as _np
            vec_clean = [0.0 if _np.isnan(v) else v for v in vec]
            # Backwards-compatibility: GBM v4 was trained on 80 features.
            # If the model's expected input width is smaller than the current
            # vector (84 with Sprint 29 Track 3 additions), trim to the model's
            # width so old checkpoints are not broken.  New v5+ models trained
            # on all 84 features will have n_features_in_ == 84 and receive the
            # full vector.
            expected = getattr(clf, "n_features_in_", len(vec_clean))
            if expected < len(vec_clean):
                vec_clean = vec_clean[:expected]
            proba = clf.predict_proba([vec_clean])[0]
            # proba[1] = probability of class 1 (ai_generated)
            classifier_score = float(proba[1])
            classifier_available = True
        except Exception:
            classifier_score = None
            classifier_available = False

    # ── UnivFD probe (CLIP-based linear classifier) ───────────────────
    # If univfd_score was not passed in, try to get it from the clip_detector.
    univfd_available = univfd_score is not None
    if not univfd_available:
        try:
            from app.services.clip_detector import perform_clip_detection
            clip_result = perform_clip_detection(image_bytes)
            if clip_result.univfd_available and clip_result.univfd_score is not None:
                univfd_score = clip_result.univfd_score
                univfd_available = True
        except Exception:
            pass

    # ── Three-source score blending ───────────────────────────────────
    # When all three sources are available (heuristic, GBM classifier,
    # UnivFD probe), weight the probe highest — it operates on CLIP
    # embeddings which generalise better than hand-crafted features.
    if univfd_available and classifier_available and classifier_score is not None:
        score = 0.20 * heuristic_score + 0.30 * classifier_score + 0.50 * univfd_score
    elif univfd_available:
        # Probe + heuristic only (no GBM classifier)
        score = 0.30 * heuristic_score + 0.70 * univfd_score
    elif classifier_available and classifier_score is not None:
        # GBM classifier + heuristic only (original blending logic)
        disagreement = abs(heuristic_score - classifier_score)
        if disagreement > 0.4:
            score = 0.70 * heuristic_score + 0.30 * classifier_score
        else:
            score = 0.35 * heuristic_score + 0.65 * classifier_score
        # Camera EXIF is a strong prior — cap classifier uplift
        if has_camera_exif and classifier_score > heuristic_score:
            score = min(score, heuristic_score + 0.10)
    else:
        score = heuristic_score

    # ── EXIF-based false positive reduction ───────────────────────────
    # Images with genuine camera EXIF (make, model, exposure) are very
    # unlikely to be AI-generated. CDN-processed PNGs that lack camera
    # EXIF account for most of the false positive rate. When camera EXIF
    # is present AND the heuristic score is below 0.6 (i.e. the
    # statistical signals are not overwhelming), cap the final blended
    # score at 0.55 — within the inconclusive band, not below the
    # authentic threshold. This prevents real camera photos from being
    # flagged unless the evidence is truly compelling (heuristic > 0.6).
    #
    # KNOWN LIMITATION: EXIF metadata can be injected (stripped from a
    # real photo and appended to an AI-generated image). The cap is set
    # at 0.55 (inconclusive) rather than lower to retain a visible signal
    # for analyst review rather than silently clearing it as authentic.
    if has_camera_exif and heuristic_score < 0.6:
        score = min(score, 0.55)

    # ── MakerNote authenticity bonus (Sprint 29 Track 1) ──────────────
    # When a vendor-recognised MakerNote is present, suppress the final
    # score proportionally to the confidence. The maximum reduction is
    # 0.25 at full confidence (bonus=1.0). This mitigates false positives
    # on computational photography output (Pixel HDR+, iPhone Deep Fusion,
    # drone ISPs, mid-range Android handsets) which the GBM classifier
    # confuses with AI-generated content because of their smooth noise
    # floors and bit-plane manipulation.
    #
    # Floor the verdict at "inconclusive" rather than "authentic" to
    # preserve the signal for analyst review — MakerNotes can be forged
    # in principle (though the binary blob complexity makes this rare).
    #
    # The reduction is bypassed if the heuristic score is overwhelming
    # (≥ 0.75) — strong forensic evidence overrides the bonus.
    if camera_authenticity_bonus > 0.0 and heuristic_score < 0.75:
        # Maximum 0.25 reduction at bonus=1.0
        max_reduction = 0.25
        reduction = camera_authenticity_bonus * max_reduction
        score = max(score - reduction, 0.0)
        # Floor at "inconclusive" — never push to authentic territory
        # via MakerNote bonus alone. The score still reflects the original
        # forensic signals, just adjusted for the camera-origin prior.
        if score < 0.20 and camera_authenticity_bonus < 1.0:
            score = max(score, 0.20)

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

    # Three-way verdict: replaces binary suspicious/clean with honest uncertainty.
    # Option C thresholds (0.25/0.55) validated on 150-image confusion matrix:
    #   AI: 100% synthetic, 0% inconclusive, 0% escape
    #   Auth: 89.3% authentic, 10.7% inconclusive, 0% FP
    # Narrower inconclusive band (0.30 width vs 0.45) reduces ambiguous results
    # while maintaining zero escapes and zero false positives.
    if score > 0.55 or any(w.detected for w in watermarks):
        verdict_level = "synthetic"
    elif score < 0.25:
        verdict_level = "authentic"
    else:
        verdict_level = "inconclusive"

    response = DeepfakeResponse(
        score=round(score, 4),
        suspicious=score > 0.5,
        confidence=confidence,
        verdict_level=verdict_level,
        signals=signals,
        heatmap_base64=heatmap_base64,
        summary=summary,
        watermarks=watermarks,
        classifier_score=round(classifier_score, 4) if classifier_score is not None else None,
        classifier_available=classifier_available,
        univfd_score=round(univfd_score, 4) if univfd_score is not None else None,
        univfd_available=univfd_available,
    )
    return response, features


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


# ── New Signal Extractors (Signals 15-21) ─────────────────────────────


def _extract_noise_autocorrelation(grey: np.ndarray) -> dict[str, float]:
    """Signal 15: Noise autocorrelation decay tau.

    Camera noise decorrelates in 1-2 pixels (tau ~0.5-2.0).
    AI noise decorrelates slowly (tau ~3-10).
    """
    try:
        grey_f = grey.astype(np.float64)
        denoised = denoise_wavelet(grey_f, rescale_sigma=True)
        noise = grey_f - denoised
        h, w = noise.shape
        max_lag = min(20, w // 4)
        if max_lag < 4:
            return {"noise_autocorr_tau": 1.0}
        noise_centered = noise - noise.mean()
        var = np.var(noise_centered) + 1e-10
        autocorr = np.zeros(max_lag)
        autocorr[0] = 1.0
        for lag in range(1, max_lag):
            autocorr[lag] = np.mean(noise_centered[:, :w - lag] * noise_centered[:, lag:]) / var
        valid = autocorr[1:] > 0.01
        lags = np.arange(1, max_lag)
        if valid.sum() >= 3:
            slope, _ = np.polyfit(lags[valid], np.log(autocorr[1:][valid]), 1)
            tau = float(np.clip(-1.0 / (slope + 1e-10), 0.1, 50.0))
        else:
            tau = 1.0
        return {"noise_autocorr_tau": tau}
    except Exception:
        return {"noise_autocorr_tau": 1.0}


def _extract_cross_channel_noise(img_bgr: np.ndarray) -> dict[str, float]:
    """Signal 16: Cross-channel noise correlation.

    Camera noise is independent per channel; AI noise is correlated.
    """
    try:
        if img_bgr.ndim < 3 or img_bgr.shape[2] < 3:
            return {"cross_channel_noise_corr_mean": 0.3, "cross_channel_noise_corr_max": 0.3}
        noises = []
        for i in range(3):
            ch = img_bgr[:, :, i].astype(np.float64)
            denoised = denoise_wavelet(ch, rescale_sigma=True)
            noises.append((ch - denoised).flatten())
        correlations = []
        for i in range(3):
            for j in range(i + 1, 3):
                si, sj = np.std(noises[i]), np.std(noises[j])
                if si > 1e-10 and sj > 1e-10:
                    correlations.append(abs(np.corrcoef(noises[i], noises[j])[0, 1]))
                else:
                    correlations.append(0.0)
        return {
            "cross_channel_noise_corr_mean": float(np.mean(correlations)),
            "cross_channel_noise_corr_max": float(np.max(correlations)),
        }
    except Exception:
        return {"cross_channel_noise_corr_mean": 0.3, "cross_channel_noise_corr_max": 0.3}


def _extract_bitplane_regularity(grey: np.ndarray) -> dict[str, float]:
    """Signal 17: Bit-plane regularity (lossless only).

    Camera LSBs are random; AI LSBs are structured.
    """
    try:
        lsb = (grey & 1).astype(np.float64)
        h_changes = np.sum(lsb[:, :-1] != lsb[:, 1:])
        v_changes = np.sum(lsb[:-1, :] != lsb[1:, :])
        total_pairs = lsb.shape[0] * (lsb.shape[1] - 1) + (lsb.shape[0] - 1) * lsb.shape[1]
        lsb_complexity = (h_changes + v_changes) / (total_pairs + 1e-10)
        lsb_randomness = float(lsb_complexity / 0.5)
        # Block entropy
        block_size = 32
        entropies = []
        for i in range(0, grey.shape[0] - block_size, block_size):
            for j in range(0, grey.shape[1] - block_size, block_size):
                block = lsb[i:i + block_size, j:j + block_size].flatten()
                p1 = np.mean(block)
                p0 = 1.0 - p1
                ent = -(p0 * np.log2(p0 + 1e-10) + p1 * np.log2(p1 + 1e-10)) if p0 > 0 and p1 > 0 else 0.0
                entropies.append(ent)
        return {
            "lsb_randomness": lsb_randomness,
            "lsb_entropy_mean": float(np.mean(entropies)) if entropies else 1.0,
        }
    except Exception:
        return {"lsb_randomness": 1.0, "lsb_entropy_mean": 1.0}


def _extract_vae_grid_artefacts(grey: np.ndarray) -> dict[str, float]:
    """Signal 18: VAE grid artefacts.

    Detects periodic structure from VAE decoder's 8x upsampling.
    """
    try:
        grey_f = grey.astype(np.float64)
        blurred = cv2.GaussianBlur(grey_f, (0, 0), sigmaX=2.0)
        residual = grey_f - blurred
        f = fftshift(fft2(residual))
        power = np.abs(f) ** 2
        h, w = power.shape
        cy, cx = h // 2, w // 2
        grid_energy = 0.0
        for mult in [1, 2, 3, 4]:
            fy = min(cy + (h * mult) // 8, h - 1)
            fx = min(cx + (w * mult) // 8, w - 1)
            for dy in range(-2, 3):
                for dx in range(-2, 3):
                    yi, xi = np.clip(fy + dy, 0, h - 1), np.clip(fx + dx, 0, w - 1)
                    grid_energy += power[yi, xi]
                    yi_m, xi_m = np.clip(2 * cy - fy + dy, 0, h - 1), np.clip(2 * cx - fx + dx, 0, w - 1)
                    grid_energy += power[yi_m, xi_m]
        total_energy = power.sum() + 1e-10
        return {"vae_grid_energy_ratio": float(grid_energy / total_energy)}
    except Exception:
        return {"vae_grid_energy_ratio": 0.0}


def _extract_ca_absence(img_bgr: np.ndarray) -> dict[str, float]:
    """Signal 19: Chromatic aberration absence.

    Real lenses have radial CA; AI has none.
    """
    try:
        if img_bgr.ndim < 3 or img_bgr.shape[2] < 3:
            return {"ca_radial_trend": 0.0}
        h, w = img_bgr.shape[:2]
        cy, cx = h // 2, w // 2
        edges = []
        for i in range(3):
            ch = img_bgr[:, :, i].astype(np.float64)
            gx = cv2.Sobel(ch, cv2.CV_64F, 1, 0, ksize=3)
            gy = cv2.Sobel(ch, cv2.CV_64F, 0, 1, ksize=3)
            edges.append(np.sqrt(gx ** 2 + gy ** 2))
        y_coords, x_coords = np.mgrid[:h, :w]
        radius = np.sqrt((x_coords - cx) ** 2 + (y_coords - cy) ** 2)
        max_r = np.sqrt(cx ** 2 + cy ** 2)
        ca_by_radius = []
        for band in range(4):
            r_min, r_max = band * max_r / 4, (band + 1) * max_r / 4
            mask = (radius >= r_min) & (radius < r_max)
            if mask.sum() < 100:
                continue
            mean_edge = (np.mean(edges[2][mask]) + np.mean(edges[0][mask])) / 2 + 1e-10
            ca_by_radius.append(float(np.mean(np.abs(edges[2][mask] - edges[0][mask])) / mean_edge))
        if len(ca_by_radius) >= 3:
            slope, _ = np.polyfit(np.arange(len(ca_by_radius), dtype=np.float64), ca_by_radius, 1)
            return {"ca_radial_trend": float(slope)}
        return {"ca_radial_trend": 0.0}
    except Exception:
        return {"ca_radial_trend": 0.0}


def _extract_demosaicing_traces(img_bgr: np.ndarray) -> dict[str, float]:
    """Signal 20: Demosaicing traces (lossless only).

    Camera images have Bayer pattern traces; AI doesn't.
    """
    try:
        if img_bgr.ndim < 3 or img_bgr.shape[2] < 3:
            return {"demosaic_peak_count": 8.0, "demosaic_peak_strength": 100.0}
        g = img_bgr[:, :, 1].astype(np.float64)
        r = img_bgr[:, :, 2].astype(np.float64)
        b = img_bgr[:, :, 0].astype(np.float64)
        peaks_detected = 0
        total_strength = 0.0
        for diff in [g - r, g - b]:
            f = fftshift(fft2(diff))
            power = np.abs(f) ** 2
            h, w = power.shape
            cy, cx = h // 2, w // 2
            median_power = np.median(power)
            for py, px in [(cy, 0), (cy, w - 1), (0, cx), (h - 1, cx),
                           (0, 0), (0, w - 1), (h - 1, 0), (h - 1, w - 1)]:
                y_lo, y_hi = max(py - 2, 0), min(py + 3, h)
                x_lo, x_hi = max(px - 2, 0), min(px + 3, w)
                local_max = power[y_lo:y_hi, x_lo:x_hi].max()
                if local_max > median_power * 10:
                    peaks_detected += 1
                    total_strength += local_max / (median_power + 1e-10)
        return {
            "demosaic_peak_count": float(peaks_detected),
            "demosaic_peak_strength": float(total_strength / max(peaks_detected, 1)),
        }
    except Exception:
        return {"demosaic_peak_count": 8.0, "demosaic_peak_strength": 100.0}


# ── Sprint 29 Track 3: camera ISP vs VAE discriminators ──────────────────
#
# These three functions extract 4 features that help separate computational
# photography output (Pixel HDR+, iPhone Deep Fusion, DJI drone ISPs) from
# VAE-decoded / diffusion-model images, which both share an unusually smooth
# noise floor that confuses the GBM classifier.
#
# They are intentionally appended AFTER all 80 original features so that GBM
# v4 (trained on 80 features) can be loaded and used without modification —
# see the backwards-compatibility trim in the classifier blending section.


def _extract_noise_lf_hf_ratio(grey: np.ndarray) -> dict[str, float]:
    """Sprint 29 Track 3, Feature 1: noise LF/HF band-power ratio.

    Separates the bilateral-filter noise residual into low-frequency (LF) and
    high-frequency (HF) radial FFT bands and returns their power ratio.

    Interpretation:
        - Real camera photos: high ratio (computational denoising suppresses HF
          sensor read noise but preserves LF noise structure from lens/scene).
        - AI generators: low ratio (VAE decoder suppresses BOTH bands).
        - Flat/smooth synthetics: also low ratio.

    Guard: returns NaN for images smaller than 64px in either dimension.
    """
    try:
        h, w = grey.shape[:2]
        if h < 64 or w < 64:
            return {"noise_lf_hf_ratio": float("nan")}

        luma = grey.astype(np.float32)
        # Bilateral filter on uint8 input; result back to float
        denoised = cv2.bilateralFilter(
            luma.astype(np.uint8), d=9, sigmaColor=75, sigmaSpace=75
        ).astype(np.float32)
        residual = luma - denoised

        # 2D FFT of the noise residual; shift zero-frequency to centre
        f_shift = np.fft.fftshift(np.fft.fft2(residual.astype(np.float64)))
        power = np.abs(f_shift) ** 2

        cy, cx = h // 2, w // 2
        max_r = min(cy, cx)

        # Build a radial distance map from the centre
        yy, xx = np.ogrid[:h, :w]
        r_map = np.sqrt((yy - cy) ** 2 + (xx - cx) ** 2) / (max_r + 1e-10)

        lf_mask = r_map <= 0.15
        hf_mask = (r_map >= 0.30) & (r_map <= 0.50)

        lf_power = float(np.sum(power[lf_mask]))
        hf_power = float(np.sum(power[hf_mask]))

        ratio = lf_power / (hf_power + 1e-10)
        return {"noise_lf_hf_ratio": ratio}
    except Exception:
        return {"noise_lf_hf_ratio": float("nan")}


def _extract_demosaic_inter_channel_coherence(img_bgr: np.ndarray) -> dict[str, float]:
    """Sprint 29 Track 3, Feature 2: inter-channel demosaicing coherence.

    Measures the Pearson correlation of per-channel FFT magnitude maps in a
    small neighbourhood around the Bayer demosaicing frequency (h/2, w/2).

    Interpretation:
        - Real cameras (Bayer sensor): all three channels carry a coherent
          demosaicing artefact → high pairwise correlation (~0.7–0.95).
        - AI generators (no sensor history): the per-channel FFT peaks are
          uncorrelated noise → low correlation (~0.0–0.3).

    Guard: returns NaN for single-channel or <64px images.
    """
    try:
        if img_bgr.ndim < 3 or img_bgr.shape[2] < 3:
            return {"demosaic_inter_channel_coherence": float("nan")}
        h, w = img_bgr.shape[:2]
        if h < 64 or w < 64:
            return {"demosaic_inter_channel_coherence": float("nan")}

        half_win = 8  # sample a 17×17 region around the Bayer frequency
        cy, cx = h // 2, w // 2

        channel_patches: list[np.ndarray] = []
        for ch_idx in range(3):
            ch = img_bgr[:, :, ch_idx].astype(np.float64)
            f_shift = np.fft.fftshift(np.abs(np.fft.fft2(ch)))
            y0 = max(cy - half_win, 0)
            y1 = min(cy + half_win + 1, h)
            x0 = max(cx - half_win, 0)
            x1 = min(cx + half_win + 1, w)
            patch = f_shift[y0:y1, x0:x1].flatten()
            channel_patches.append(patch)

        # Ensure equal lengths (shouldn't differ but guard anyway)
        min_len = min(len(p) for p in channel_patches)
        if min_len < 4:
            return {"demosaic_inter_channel_coherence": float("nan")}
        r_patch, g_patch, b_patch = [p[:min_len] for p in channel_patches]

        corr_rg = _safe_corrcoef(r_patch, g_patch)
        corr_gb = _safe_corrcoef(g_patch, b_patch)
        corr_rb = _safe_corrcoef(r_patch, b_patch)

        coherence = float(np.mean([corr_rg, corr_gb, corr_rb]))
        return {"demosaic_inter_channel_coherence": coherence}
    except Exception:
        return {"demosaic_inter_channel_coherence": float("nan")}


def _extract_noise_anisotropy(grey: np.ndarray) -> dict[str, float]:
    """Sprint 29 Track 3, Features 3–4: directional noise anisotropy.

    Divides the bilateral-filter noise residual into a 4×4 grid and computes
    the log horizontal-to-vertical variance ratio per cell.  Returns the mean
    and std of those 16 values.

    Interpretation:
        - Real sensor noise has a weak row/column readout correlation that
          creates slight directional asymmetry (non-zero mean, modest std).
        - AI noise is isotropic (mean ≈ 0 in log space, low std).
        - Computational photography preserves the row/column pattern.

    Guard: returns NaN when the image is too small for a 4×4 grid.
    """
    try:
        h, w = grey.shape[:2]
        if h < 64 or w < 64:
            return {"noise_anisotropy_mean": float("nan"), "noise_anisotropy_std": float("nan")}

        luma = grey.astype(np.float32)
        denoised = cv2.bilateralFilter(
            luma.astype(np.uint8), d=9, sigmaColor=75, sigmaSpace=75
        ).astype(np.float32)
        residual = (luma - denoised).astype(np.float64)

        cell_h = h // 4
        cell_w = w // 4
        aniso_values: list[float] = []

        for row in range(4):
            for col in range(4):
                cell = residual[
                    row * cell_h : (row + 1) * cell_h,
                    col * cell_w : (col + 1) * cell_w,
                ]
                if cell.size < 4:
                    continue
                # Finite-difference variance in each axis
                h_var = float(np.var(np.diff(cell, axis=1)))
                v_var = float(np.var(np.diff(cell, axis=0)))
                # Log ratio to symmetrise around zero
                log_ratio = math.log(h_var / (v_var + 1e-10) + 1e-10)
                aniso_values.append(log_ratio)

        if len(aniso_values) < 2:
            return {"noise_anisotropy_mean": float("nan"), "noise_anisotropy_std": float("nan")}

        return {
            "noise_anisotropy_mean": float(np.mean(aniso_values)),
            "noise_anisotropy_std": float(np.std(aniso_values)),
        }
    except Exception:
        return {"noise_anisotropy_mean": float("nan"), "noise_anisotropy_std": float("nan")}


def _extract_saturation_luminance(img_bgr: np.ndarray) -> dict[str, float]:
    """Signal 21: Saturation-luminance anomaly.

    AI images often have saturated highlights/shadows.
    """
    try:
        if img_bgr.ndim < 3 or img_bgr.shape[2] < 3:
            return {"sat_lum_extreme_ratio": 0.2}
        hsv = cv2.cvtColor(img_bgr, cv2.COLOR_BGR2HSV)
        v = hsv[:, :, 2].astype(np.float64)
        s = hsv[:, :, 1].astype(np.float64)
        bin_edges = np.linspace(0, 255, 11)
        bin_means = []
        for i in range(10):
            mask = (v >= bin_edges[i]) & (v < bin_edges[i + 1])
            bin_means.append(float(np.mean(s[mask])) if mask.sum() > 50 else 0.0)
        shadow_sat = np.mean(bin_means[:2]) if any(bin_means[:2]) else 0.0
        highlight_sat = np.mean(bin_means[-2:]) if any(bin_means[-2:]) else 0.0
        mid_sat = np.mean(bin_means[3:7]) if any(bin_means[3:7]) else 1.0
        return {"sat_lum_extreme_ratio": float((shadow_sat + highlight_sat) / 2 / (mid_sat + 1e-10))}
    except Exception:
        return {"sat_lum_extreme_ratio": 0.2}


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
    # to total weight is mapped through a sigmoid (midpoint=0.15, k=12).
    # This gives:
    #   0% triggered  → score ~0.14 (authentic)
    #  10% triggered  → score ~0.35
    #  15% triggered  → score ~0.50 (suspicious threshold)
    #  25% triggered  → score ~0.77
    # 100% triggered  → score ~1.00
    #
    # Midpoint lowered from 0.18 to 0.15 to compensate for weight dilution
    # (total weight increased from 17.5 to ~26 with 7 new signals). Steepness
    # kept at 12 for sharp authentic/suspicious separation.
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
        0.75,
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

    # 8. Colour gamut — older cameras and Google Photos recompression
    #    produce gamut as low as 0.014. Only flag below 0.01.
    gamut = features.get("color_gamut_coverage", 0.1)
    _add(
        "color_gamut",
        0.5,
        gamut < 0.01,
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
        0.25,
        benford_div > 0.4,
        f"DCT coefficients deviate from Benford's law (div={benford_div:.3f}), uncommon in natural images",
        f"DCT statistics follow expected distribution (div={benford_div:.3f})",
    )

    # --- Signal 15: Noise autocorrelation decay ---
    # Real camera JPEGs show tau=25-40 due to JPEG compression + computational
    # denoising. Only flag very extreme values (>50) or very fast decay in
    # lossless images where compression doesn't inflate tau.
    tau = features.get("noise_autocorr_tau", 1.0)
    tau_thresh = 5.0 if codec_class in ("lossless", "raw") else 50.0
    _add("noise_autocorr_tau", 2.5, tau > tau_thresh,
         f"Slow noise autocorrelation decay (tau={tau:.1f}), characteristic of AI generation",
         f"Fast noise autocorrelation decay (tau={tau:.1f}), consistent with camera sensor")

    # --- Signal 16: Cross-channel noise correlation ---
    # Real camera JPEGs show 0.84-0.94 correlation due to JPEG compression and
    # computational photography (Smart HDR, denoising). Only flag >0.96 for
    # lossy codecs; >0.70 for lossless/raw where noise should be independent.
    cc_corr = features.get("cross_channel_noise_corr_mean", 0.3)
    # Real camera JPEGs (including Google Photos recompression and older cameras)
    # show 0.84-0.99 cross-channel correlation. Only flag > 0.997 for lossy.
    cc_thresh = 0.70 if codec_class in ("lossless", "raw") else 0.997
    _add("cross_channel_noise_corr", 2.5, cc_corr > cc_thresh,
         f"Highly correlated noise across channels ({cc_corr:.3f}), shared generative process",
         f"Independent noise across channels ({cc_corr:.3f}), consistent with camera sensor")

    # --- Signal 17: Bit-plane regularity (lossless only) ---
    if codec_class == "lossless":
        lsb_rand = features.get("lsb_randomness", 1.0)
        _add("bitplane_regularity", 2.0, lsb_rand < 0.92,
             f"Structured LSB pattern (randomness={lsb_rand:.3f}), computed pixel values",
             f"Random LSB pattern (randomness={lsb_rand:.3f}), consistent with sensor noise")

    # --- Signal 18: VAE grid artefacts ---
    vae_w = 1.5 if codec_class == "lossless" else 0.5
    vae_grid = features.get("vae_grid_energy_ratio", 0.0)
    _add("vae_grid_artefacts", vae_w, vae_grid > 0.002,
         f"Periodic VAE decoder grid detected (ratio={vae_grid:.5f})",
         f"No periodic grid structure (ratio={vae_grid:.5f})")

    # --- Signal 19: Chromatic aberration absence ---
    # Use absolute value: real lenses produce both positive and negative CA trends
    # depending on lens design. Only flag when |trend| is essentially zero.
    ca_trend = features.get("ca_radial_trend", 0.01)
    _add("ca_absence", 1.5, abs(ca_trend) < 0.003,
         f"No radial chromatic aberration (trend={ca_trend:.5f}), inconsistent with optics",
         f"Radial chromatic aberration present (trend={ca_trend:.5f}), consistent with lens")

    # --- Signal 20: Demosaicing traces (lossless only) ---
    if codec_class == "lossless":
        dem_peaks = features.get("demosaic_peak_count", 8.0)
        dem_strength = features.get("demosaic_peak_strength", 100.0)
        _add("demosaicing_traces", 2.0, dem_peaks < 3 and dem_strength < 25,
             f"No Bayer demosaicing traces ({dem_peaks:.0f} peaks), not from camera sensor",
             f"Bayer demosaicing traces present ({dem_peaks:.0f} peaks), consistent with sensor")

    # --- Signal 21: Saturation-luminance anomaly ---
    # Real camera photos show ratios of 0.5-1.5 due to lens flare, white balance,
    # and indoor lighting. Only flag extreme values (>2.0) that indicate AI
    # generation artefacts where saturation doesn't fall off in shadows/highlights.
    # Real camera photos (including older cameras, Google Photos recompression)
    # show ratios of 0.5-1.6. Only flag > 2.0 for genuine AI artefacts.
    sat_ratio = features.get("sat_lum_extreme_ratio", 0.2)
    _add("sat_lum_anomaly", 1.5, sat_ratio > 2.0,
         f"Anomalous saturation in extremes (ratio={sat_ratio:.3f}), violates colour physics",
         f"Normal saturation-luminance curve (ratio={sat_ratio:.3f})")

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
    midpoint = 0.22 if has_camera_exif else 0.15
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
