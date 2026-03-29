"""Jura Trace Sidecar — GAN Spectral Fingerprint Visualisation.

GAN-generated images exhibit characteristic spectral fingerprints caused by
transposed convolution (deconvolution) upsampling layers.  These manifest as
periodic peaks in the 2D Fourier magnitude spectrum that are absent in natural
photographs, whose spectra follow a smooth 1/f power-law decay.

This service:
1. Computes a multi-channel 2D FFT and averages the magnitude spectra.
2. Fits a 1/f natural-image model via radial profile regression.
3. Subtracts the 1/f model to isolate anomalous peaks (GAN artefacts).
4. Detects peaks using dilation-based local maxima above a 3-sigma threshold.
5. Checks for ring patterns (StyleGAN2) or checkerboard patterns (ProGAN).
6. Renders an annotated spectrum (INFERNO colourmap, green circles on peaks).
7. Renders a residual spectrum (HOT colourmap).

Returns a dict consumed by the ``/forensics/gan-fingerprint`` endpoint.
"""

import base64
import io
import logging
import math

import cv2
import numpy as np
from PIL import Image
from scipy.ndimage import maximum_filter

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _radial_profile(magnitude: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Compute the azimuthally-averaged radial profile of a 2D spectrum.

    Returns (radii, profile) where *radii* is an array of integer radial
    distances from the centre and *profile* is the mean magnitude at each
    radius.
    """
    h, w = magnitude.shape
    cy, cx = h // 2, w // 2
    Y, X = np.ogrid[:h, :w]
    r = np.sqrt((X - cx) ** 2 + (Y - cy) ** 2).astype(int)
    max_r = min(cx, cy)
    radii = np.arange(1, max_r)
    profile = np.array([magnitude[r == rad].mean() for rad in radii])
    return radii, profile


def _fit_one_over_f(radii: np.ndarray, profile: np.ndarray) -> np.ndarray:
    """Fit a 1/f model (linear in log-log space) and return the fitted curve.

    Model: log(magnitude) = a * log(frequency) + b  =>  magnitude = b * f^a
    """
    log_r = np.log(radii.astype(np.float64) + 1e-8)
    log_p = np.log(profile + 1e-8)

    # Least-squares linear fit in log-log space
    A = np.vstack([log_r, np.ones_like(log_r)]).T
    coeffs, _, _, _ = np.linalg.lstsq(A, log_p, rcond=None)
    fitted_log = coeffs[0] * log_r + coeffs[1]
    return np.exp(fitted_log)


def _build_radial_map(shape: tuple[int, int], values: np.ndarray, max_r: int) -> np.ndarray:
    """Expand a 1-D radial profile back into a 2-D image of given *shape*."""
    h, w = shape
    cy, cx = h // 2, w // 2
    Y, X = np.ogrid[:h, :w]
    r = np.sqrt((X - cx) ** 2 + (Y - cy) ** 2).astype(int)
    out = np.zeros((h, w), dtype=np.float64)
    for i, rad in enumerate(range(1, max_r)):
        out[r == rad] = values[i]
    return out


def _detect_peaks(residual: np.ndarray, sigma_threshold: float = 3.0,
                  neighbourhood: int = 11) -> list[dict]:
    """Find local maxima in *residual* that exceed *sigma_threshold* standard
    deviations above the mean.

    Uses dilation-based local maximum detection.
    """
    mean_val = residual.mean()
    std_val = residual.std()
    threshold = mean_val + sigma_threshold * std_val

    dilated = maximum_filter(residual, size=neighbourhood)
    local_max = (residual == dilated) & (residual > threshold)

    ys, xs = np.where(local_max)
    h, w = residual.shape
    cy, cx = h // 2, w // 2

    peaks = []
    for y, x in zip(ys, xs):
        dx = x - cx
        dy = y - cy
        freq = math.sqrt(dx ** 2 + dy ** 2)
        angle = math.degrees(math.atan2(dy, dx))
        mag = float(residual[y, x])
        peaks.append({"frequency": round(freq, 2),
                       "magnitude": round(mag, 4),
                       "angle": round(angle, 2),
                       "_x": int(x), "_y": int(y)})

    # Sort by magnitude descending, keep top 10
    peaks.sort(key=lambda p: p["magnitude"], reverse=True)
    return peaks[:10]


def _check_ring_pattern(peaks: list[dict], tolerance: float = 5.0) -> bool:
    """Check whether peaks cluster at a common radius (ring pattern).

    StyleGAN2 produces ring-shaped spectral artefacts from its upsampling
    layers.  We test whether at least 4 peaks share a similar radial
    frequency (within *tolerance* pixels).
    """
    if len(peaks) < 4:
        return False
    freqs = sorted([p["frequency"] for p in peaks])
    for i in range(len(freqs)):
        cluster = [f for f in freqs if abs(f - freqs[i]) <= tolerance]
        if len(cluster) >= 4:
            return True
    return False


def _check_checkerboard_pattern(peaks: list[dict], w: int, h: int) -> bool:
    """Check for checkerboard spectral peaks at (w/2, h/2) corners.

    ProGAN and other networks using stride-2 transposed convolution produce
    a characteristic spike near the Nyquist frequency in all four quadrants.
    """
    if len(peaks) < 2:
        return False
    cx, cy = w // 2, h // 2
    corner_hits = 0
    margin = max(w, h) * 0.1
    for p in peaks:
        px, py = p["_x"], p["_y"]
        # Near corners of the spectrum
        near_corner = (
            (abs(px - 0) < margin or abs(px - w) < margin)
            and (abs(py - 0) < margin or abs(py - h) < margin)
        )
        # Near edges (Nyquist) — halfway between centre and edge
        near_nyquist = p["frequency"] > min(cx, cy) * 0.7
        if near_corner or near_nyquist:
            corner_hits += 1
    return corner_hits >= 2


def _encode_png_base64(img_array: np.ndarray) -> str:
    """Convert a uint8 numpy array (RGB or grayscale) to base64 PNG."""
    pil = Image.fromarray(img_array)
    buf = io.BytesIO()
    pil.save(buf, format="PNG")
    return base64.b64encode(buf.getvalue()).decode("utf-8")


# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def visualise_gan_fingerprint(image_bytes: bytes) -> dict:
    """Analyse an image for GAN spectral fingerprints.

    Args:
        image_bytes: Raw bytes of a JPEG/PNG image.

    Returns:
        Dict with keys:
        - spectrum_base64: annotated FFT magnitude spectrum (base64 PNG)
        - residual_spectrum_base64: residual after 1/f subtraction (base64 PNG)
        - spectral_peaks: list of {frequency, magnitude, angle} (top 10)
        - has_periodic_artefacts: bool (True if >= 4 peaks)
        - model_attribution: str or None (StyleGAN2, ProGAN, StyleGAN3)
        - confidence: float 0-1
        - analysis_notes: list of str

    Raises:
        ValueError: If the input cannot be decoded as an image.
    """
    if len(image_bytes) == 0:
        raise ValueError("Could not decode image")

    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        raise ValueError("Could not decode image")

    img_rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
    h, w = img_rgb.shape[:2]
    notes: list[str] = []

    # ------------------------------------------------------------------
    # 1. Multi-channel 2D FFT — average the magnitude spectra
    # ------------------------------------------------------------------
    channels = cv2.split(img_rgb.astype(np.float32))
    magnitudes = []
    for ch in channels:
        f = np.fft.fft2(ch)
        fshift = np.fft.fftshift(f)
        magnitudes.append(np.log1p(np.abs(fshift)))
    avg_magnitude = np.mean(magnitudes, axis=0)

    # ------------------------------------------------------------------
    # 2. Fit 1/f natural image model via radial profile regression
    # ------------------------------------------------------------------
    radii, profile = _radial_profile(avg_magnitude)
    fitted = _fit_one_over_f(radii, profile)

    # ------------------------------------------------------------------
    # 3. Subtract 1/f model to isolate anomalous peaks
    # ------------------------------------------------------------------
    max_r = min(w // 2, h // 2)
    model_map = _build_radial_map(avg_magnitude.shape, fitted, max_r)
    residual = avg_magnitude - model_map
    residual = np.clip(residual, 0, None)

    # ------------------------------------------------------------------
    # 4. Detect peaks using dilation-based local maxima above 3-sigma
    # ------------------------------------------------------------------
    peaks = _detect_peaks(residual, sigma_threshold=3.0, neighbourhood=11)
    has_periodic = len(peaks) >= 4

    if has_periodic:
        notes.append(f"Detected {len(peaks)} anomalous spectral peaks above 3-sigma threshold.")
    else:
        notes.append("No significant periodic artefacts detected in the frequency domain.")

    # ------------------------------------------------------------------
    # 5. Check for ring patterns (StyleGAN2) or checkerboard (ProGAN)
    # ------------------------------------------------------------------
    is_ring = _check_ring_pattern(peaks)
    is_checkerboard = _check_checkerboard_pattern(peaks, w, h)

    model_attribution: str | None = None
    if is_ring and is_checkerboard:
        model_attribution = "StyleGAN3"
        notes.append("Ring and checkerboard patterns detected — consistent with StyleGAN3.")
    elif is_ring:
        model_attribution = "StyleGAN2"
        notes.append("Ring pattern detected in spectral peaks — consistent with StyleGAN2 upsampling.")
    elif is_checkerboard:
        model_attribution = "ProGAN"
        notes.append("Checkerboard pattern detected — consistent with ProGAN transposed convolution.")
    else:
        notes.append("No characteristic GAN model spectral signature identified.")

    # Confidence scoring
    if has_periodic and model_attribution:
        confidence = min(1.0, 0.5 + len(peaks) * 0.05)
    elif has_periodic:
        confidence = min(0.7, 0.3 + len(peaks) * 0.04)
    else:
        confidence = max(0.0, min(0.3, len(peaks) * 0.1))

    # ------------------------------------------------------------------
    # 6. Render annotated spectrum (INFERNO colourmap, green circles on peaks)
    # ------------------------------------------------------------------
    mag_norm = (
        (avg_magnitude - avg_magnitude.min())
        / (avg_magnitude.max() - avg_magnitude.min() + 1e-8)
        * 255
    ).astype(np.uint8)
    spectrum_coloured = cv2.applyColorMap(mag_norm, cv2.COLORMAP_INFERNO)

    # Draw green circles on peak locations
    for p in peaks:
        cv2.circle(spectrum_coloured, (p["_x"], p["_y"]), 8, (0, 255, 0), 2)

    spectrum_rgb = cv2.cvtColor(spectrum_coloured, cv2.COLOR_BGR2RGB)
    spectrum_b64 = _encode_png_base64(spectrum_rgb)

    # ------------------------------------------------------------------
    # 7. Render residual spectrum (HOT colourmap)
    # ------------------------------------------------------------------
    res_norm = (
        (residual - residual.min())
        / (residual.max() - residual.min() + 1e-8)
        * 255
    ).astype(np.uint8)
    residual_coloured = cv2.applyColorMap(res_norm, cv2.COLORMAP_HOT)
    residual_rgb = cv2.cvtColor(residual_coloured, cv2.COLOR_BGR2RGB)
    residual_b64 = _encode_png_base64(residual_rgb)

    # Clean peaks for output (remove internal coordinates)
    output_peaks = [
        {"frequency": p["frequency"], "magnitude": p["magnitude"], "angle": p["angle"]}
        for p in peaks
    ]

    return {
        "spectrum_base64": spectrum_b64,
        "residual_spectrum_base64": residual_b64,
        "spectral_peaks": output_peaks,
        "has_periodic_artefacts": has_periodic,
        "model_attribution": model_attribution,
        "confidence": round(confidence, 4),
        "analysis_notes": notes,
    }
