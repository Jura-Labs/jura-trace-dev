"""
Jura Trace Sidecar — Audio ENF (Electrical Network Frequency) analysis.

Extracts and analyses mains hum (50/60 Hz) from audio recordings for
temporal and geographic provenance. Power grids operate at either 50 Hz
(Europe, Asia, Africa, Oceania) or 60 Hz (Americas, parts of Asia).
The mains frequency leaves a subtle but measurable imprint in audio
recordings made near electrical infrastructure.

ENF analysis can help establish:
  - Geographic provenance (50 Hz vs 60 Hz grid region)
  - Temporal consistency (ENF fluctuations are unique to each moment)
  - Recording authenticity (absence of ENF in claimed indoor recordings)
"""

import io
from typing import Any

import numpy as np
from scipy import signal as scipy_signal
from scipy.io import wavfile


def analyse_enf(
    audio_bytes: bytes,
    expected_freq: float = 50.0,
) -> dict[str, Any]:
    """Extract ENF (mains hum) from audio for temporal provenance.

    Applies a narrow bandpass filter around the expected mains frequency,
    then tracks instantaneous frequency via STFT to measure ENF presence,
    stability, and signal-to-noise ratio.

    Args:
        audio_bytes: Raw bytes of a WAV audio file.
        expected_freq: Expected mains frequency in Hz (50.0 or 60.0).

    Returns:
        Dictionary with keys:
          - detected: whether ENF was found above the SNR threshold
          - meanFrequency: mean ENF frequency in Hz
          - frequencyStd: standard deviation of ENF frequency
          - snr: signal-to-noise ratio of ENF band
          - gridRegion: human-readable grid region description
          - expectedFrequency: the target frequency used for analysis
          - durationSeconds: audio duration
          - sampleCount: number of STFT time windows analysed
          - frequencyTrace: list of per-window peak frequencies
          - summary: human-readable summary string
    """
    # Validate expected frequency
    if expected_freq not in (50.0, 60.0):
        return {
            "detected": False,
            "meanFrequency": None,
            "frequencyStd": None,
            "snr": None,
            "gridRegion": None,
            "expectedFrequency": expected_freq,
            "durationSeconds": None,
            "sampleCount": 0,
            "frequencyTrace": [],
            "summary": (
                f"Invalid expected frequency {expected_freq} Hz. "
                f"Must be 50.0 (Europe/Asia/Africa) or 60.0 (Americas)."
            ),
        }

    # Load audio
    try:
        sr, audio = wavfile.read(io.BytesIO(audio_bytes))
    except Exception as exc:
        return {
            "detected": False,
            "meanFrequency": None,
            "frequencyStd": None,
            "snr": None,
            "gridRegion": None,
            "expectedFrequency": expected_freq,
            "durationSeconds": None,
            "sampleCount": 0,
            "frequencyTrace": [],
            "summary": f"Could not decode audio file: {exc}",
        }

    # Convert to mono if stereo
    if len(audio.shape) > 1:
        audio = audio.mean(axis=1)

    audio = audio.astype(np.float64)

    duration_seconds = len(audio) / sr

    # Check minimum duration (need at least a few seconds for meaningful ENF)
    if duration_seconds < 2.0:
        return {
            "detected": False,
            "meanFrequency": None,
            "frequencyStd": None,
            "snr": None,
            "gridRegion": None,
            "expectedFrequency": expected_freq,
            "durationSeconds": round(duration_seconds, 1),
            "sampleCount": 0,
            "frequencyTrace": [],
            "summary": (
                f"Audio too short ({duration_seconds:.1f}s). "
                f"ENF analysis requires at least 2 seconds of audio."
            ),
        }

    # Bandpass filter around expected ENF (e.g. 49.5-50.5 Hz)
    nyq = sr / 2.0
    low = (expected_freq - 0.5) / nyq
    high = (expected_freq + 0.5) / nyq

    if high >= 1.0 or low <= 0.0:
        return {
            "detected": False,
            "meanFrequency": None,
            "frequencyStd": None,
            "snr": None,
            "gridRegion": None,
            "expectedFrequency": expected_freq,
            "durationSeconds": round(duration_seconds, 1),
            "sampleCount": 0,
            "frequencyTrace": [],
            "summary": (
                f"Sample rate {sr} Hz too low for ENF extraction "
                f"at {expected_freq} Hz. Minimum required: "
                f"{int((expected_freq + 0.5) * 2 + 1)} Hz."
            ),
        }

    b, a = scipy_signal.butter(4, [low, high], btype="band")
    filtered = scipy_signal.filtfilt(b, a, audio)

    # STFT to track ENF over time — 1-second windows
    window_sec = 1.0
    nperseg = int(sr * window_sec)
    freqs, times, Zxx = scipy_signal.stft(filtered, sr, nperseg=nperseg)

    # Isolate frequency bins within the ENF band
    freq_mask = (freqs >= expected_freq - 0.5) & (freqs <= expected_freq + 0.5)
    if not np.any(freq_mask):
        return {
            "detected": False,
            "meanFrequency": None,
            "frequencyStd": None,
            "snr": None,
            "gridRegion": None,
            "expectedFrequency": expected_freq,
            "durationSeconds": round(duration_seconds, 1),
            "sampleCount": len(times),
            "frequencyTrace": [],
            "summary": "No ENF band found in frequency range.",
        }

    enf_magnitudes = np.abs(Zxx[freq_mask, :])
    peak_indices = np.argmax(enf_magnitudes, axis=0)
    masked_freqs = freqs[freq_mask]
    peak_freqs = masked_freqs[peak_indices]
    peak_strengths = np.max(enf_magnitudes, axis=0)

    # Overall ENF strength relative to noise floor
    noise_floor = np.std(np.abs(Zxx)) + 1e-10
    enf_snr = float(np.mean(peak_strengths) / noise_floor)

    # Detection threshold: ENF must be at least 3x above noise floor
    detected = enf_snr > 3.0

    mean_freq = float(np.mean(peak_freqs))
    freq_std = float(np.std(peak_freqs))

    # Determine likely grid region
    if abs(mean_freq - 50.0) < abs(mean_freq - 60.0):
        grid_region = "50 Hz region (Europe, Asia, Africa, Oceania)"
    else:
        grid_region = "60 Hz region (Americas, parts of Asia)"

    # Build frequency trace (subsample for large files)
    trace = peak_freqs.tolist()
    if len(trace) > 500:
        step = len(trace) // 500
        trace = trace[::step]

    if detected:
        summary = (
            f"ENF detected at {mean_freq:.3f} Hz (SNR {enf_snr:.1f}). "
            f"Consistent with {grid_region}. "
            f"Frequency stability: \u00b1{freq_std * 1000:.2f} mHz "
            f"over {duration_seconds:.1f}s."
        )
    else:
        summary = (
            f"No clear ENF signal detected (SNR {enf_snr:.1f}, below threshold). "
            f"Recording may be from a digital source, outdoors, or heavily processed."
        )

    return {
        "detected": detected,
        "meanFrequency": round(mean_freq, 4),
        "frequencyStd": round(freq_std, 6),
        "snr": round(enf_snr, 2),
        "gridRegion": grid_region,
        "expectedFrequency": expected_freq,
        "durationSeconds": round(duration_seconds, 1),
        "sampleCount": len(times),
        "frequencyTrace": [round(f, 4) for f in trace],
        "summary": summary,
    }
