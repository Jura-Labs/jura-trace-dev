# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — 2D Fourier periodic pattern detection.

Applies a 2D FFT to detect periodic patterns in the image frequency spectrum.
Authentic photographs have a smooth, isotropic spectrum. AI-generated images,
screen-recaptured content, and composited sources can exhibit discrete peaks
away from the DC component, revealing periodic artefacts such as:
  - GAN grid artefacts (regular spectral spikes at sub-Nyquist frequencies)
  - Moiré patterns from scanning or screen capture
  - Resampling artefacts from aggressive upscaling
  - Periodic textures introduced by compositing tools
"""

import base64
import io
from typing import Any

import numpy as np
from PIL import Image


def analyse_fourier(image_bytes: bytes) -> dict[str, Any]:
    """Detect periodic patterns in an image via 2D FFT magnitude spectrum.

    Args:
        image_bytes: Raw bytes of the image file.

    Returns:
        Dictionary with keys:
          - spectrumBase64: base64-encoded PNG of the log-magnitude spectrum
          - peakCount: number of peaks above the detection threshold
          - suspicious: True when peakCount exceeds the threshold
          - score: normalised score in [0, 1]
          - summary: human-readable interpretation

    Raises:
        ValueError: if image_bytes is not a valid image.
    """
    try:
        img = Image.open(io.BytesIO(image_bytes)).convert("L")
    except Exception as exc:
        raise ValueError(f"Could not decode image: {exc}") from exc

    # Downsample large images to cap FFT cost.  A 1024×1024 array is
    # sufficient to resolve the periodic artefacts we care about; larger
    # inputs would quadratically increase FFT time without benefit.
    _MAX_DIM = 1024
    ow, oh = img.size
    if ow > _MAX_DIM or oh > _MAX_DIM:
        scale = _MAX_DIM / max(ow, oh)
        img = img.resize((max(1, int(ow * scale)), max(1, int(oh * scale))), Image.LANCZOS)

    arr = np.array(img, dtype=np.float64)
    h, w = arr.shape

    # ── 2D FFT ─────────────────────────────────────────────────────────────
    f_transform = np.fft.fft2(arr)
    f_shift = np.fft.fftshift(f_transform)
    # Log-magnitude spectrum — compresses the dynamic range for visualisation.
    magnitude = np.log1p(np.abs(f_shift))

    # ── Spectrum visualisation ─────────────────────────────────────────────
    mag_min = magnitude.min()
    mag_max = magnitude.max()
    mag_range = mag_max - mag_min + 1e-10
    mag_norm = ((magnitude - mag_min) / mag_range * 255).astype(np.uint8)
    spectrum_img = Image.fromarray(mag_norm)

    buf = io.BytesIO()
    spectrum_img.save(buf, format="PNG")
    spectrum_b64 = base64.b64encode(buf.getvalue()).decode()

    # ── Peak detection ─────────────────────────────────────────────────────
    # Mask out the DC region (central 5%) to avoid counting the DC component
    # and its immediate neighbours as spurious peaks.
    cy, cx = h // 2, w // 2
    mask_radius = max(3, min(h, w) // 20)
    y_grid, x_grid = np.ogrid[:h, :w]
    dc_mask = (y_grid - cy) ** 2 + (x_grid - cx) ** 2 <= mask_radius ** 2
    magnitude_masked = magnitude.copy()
    magnitude_masked[dc_mask] = 0.0

    # Threshold: 3 standard deviations above the mean of the non-DC region.
    non_dc = magnitude_masked[~dc_mask]
    threshold = float(np.mean(non_dc) + 3.0 * np.std(non_dc))
    peak_count = int(np.sum(magnitude_masked > threshold))

    # Threshold for suspicious peak count.
    # Calibrated empirically:
    #   - Authentic photographs: typically 0–10 peaks
    #   - GAN-generated images:  often 20–200+ peaks
    # 20 is a conservative initial threshold (prefer low FP rate over recall).
    peak_threshold = 20
    suspicious = bool(peak_count > peak_threshold)
    score = float(min(peak_count / 100.0, 1.0))

    if suspicious:
        summary = (
            f"{peak_count} frequency peaks detected above threshold. "
            "Periodic patterns found — possible GAN artefacts, moiré, or screen recapture."
        )
    else:
        summary = (
            f"{peak_count} frequency peaks detected. "
            "No significant periodic patterns in the frequency spectrum."
        )

    return {
        "spectrumBase64": spectrum_b64,
        "peakCount": peak_count,
        "suspicious": suspicious,
        "score": score,
        "summary": summary,
    }
