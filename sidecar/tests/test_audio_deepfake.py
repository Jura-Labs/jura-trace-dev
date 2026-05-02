"""Tests for the audio deepfake detection service (Sprint 35 skeleton)."""

import pytest

# JTV-138 (2026-05-02) — audio deepfake dropped from v1.0; restored alongside
# AASIST retraining under JTV-110/JTV-113 in v1.1.
pytestmark = pytest.mark.skip(reason="JTV-138 v1.0 drop — re-enable under JTV-110/JTV-113")

import io
import os
import struct
import tempfile
import wave
from pathlib import Path

import numpy as np
import pytest


# ── Test helpers ──────────────────────────────────────────────────────────────

def _make_sine_wave_wav(
    duration_seconds: float = 1.5,
    frequency_hz: float = 440.0,
    sample_rate: int = 16_000,
) -> bytes:
    """Generate a mono 16-bit WAV containing a pure sine wave.

    Uses only the stdlib ``wave`` module so the test has no extra deps.
    """
    n_samples = int(duration_seconds * sample_rate)
    t = np.linspace(0.0, duration_seconds, n_samples, endpoint=False)
    pcm = (np.sin(2.0 * np.pi * frequency_hz * t) * 32_000).astype(np.int16)

    buf = io.BytesIO()
    with wave.open(buf, "w") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)  # 16-bit
        wf.setframerate(sample_rate)
        wf.writeframes(pcm.tobytes())
    return buf.getvalue()


def _make_short_wav(duration_seconds: float = 0.2) -> bytes:
    """Generate a WAV shorter than the minimum duration (0.5 s)."""
    return _make_sine_wave_wav(duration_seconds=duration_seconds)


def _write_tmp_wav(content: bytes, suffix: str = ".wav") -> str:
    """Write bytes to a named temp file; caller is responsible for cleanup."""
    fd, path = tempfile.mkstemp(suffix=suffix)
    try:
        os.write(fd, content)
    finally:
        os.close(fd)
    return path


# ── Service-level unit tests ──────────────────────────────────────────────────

class TestExtractMfccFeatures:
    """MFCC feature extraction — scipy/librosa path."""

    def test_sine_wave_returns_160_dim_vector(self):
        from app.services.audio_deepfake import extract_mfcc_features

        wav_bytes = _make_sine_wave_wav(duration_seconds=2.0)
        path = _write_tmp_wav(wav_bytes)
        try:
            features = extract_mfcc_features(path)
        finally:
            os.unlink(path)

        assert features is not None, "Expected a feature vector, got None"
        assert features.shape == (160,), f"Expected (160,), got {features.shape}"
        assert np.isfinite(features).all(), "Feature vector contains NaN or Inf"

    def test_too_short_returns_none(self):
        from app.services.audio_deepfake import extract_mfcc_features

        wav_bytes = _make_short_wav(duration_seconds=0.2)
        path = _write_tmp_wav(wav_bytes)
        try:
            features = extract_mfcc_features(path)
        finally:
            os.unlink(path)

        assert features is None, "Expected None for sub-0.5 s audio"

    def test_corrupt_file_returns_none(self):
        from app.services.audio_deepfake import extract_mfcc_features

        path = _write_tmp_wav(b"not a real wav file at all", suffix=".wav")
        try:
            features = extract_mfcc_features(path)
        finally:
            os.unlink(path)

        assert features is None, "Expected None for corrupt audio"

    def test_nonexistent_file_returns_none(self):
        from app.services.audio_deepfake import extract_mfcc_features

        result = extract_mfcc_features("/tmp/jura_nonexistent_12345.wav")
        assert result is None


class TestScoreAudioDeepfake:
    """Ensemble scorer — model_not_loaded path (no probe files)."""

    def test_returns_model_not_loaded_when_no_probes(self):
        """Core contract: without probe files the response says model_loaded=False."""
        from app.services.audio_deepfake import score_audio_deepfake

        wav_bytes = _make_sine_wave_wav(duration_seconds=2.0)
        path = _write_tmp_wav(wav_bytes)
        try:
            result = score_audio_deepfake(
                path,
                mfcc_probe_path="/tmp/jura_missing_stage1.joblib",
                wav2vec2_probe_path="/tmp/jura_missing_stage2.joblib",
            )
        finally:
            os.unlink(path)

        assert not result.model_loaded
        assert result.verdict == "model_not_loaded"
        assert result.score is None
        assert result.stage1_score is None
        assert result.stage2_score is None
        assert result.stages_available == []

    def test_mfcc_extracted_even_without_probes(self):
        """MFCC extraction should succeed even when no probe is deployed."""
        from app.services.audio_deepfake import score_audio_deepfake

        wav_bytes = _make_sine_wave_wav(duration_seconds=2.0)
        path = _write_tmp_wav(wav_bytes)
        try:
            result = score_audio_deepfake(
                path,
                mfcc_probe_path="/tmp/jura_missing_stage1.joblib",
                wav2vec2_probe_path="/tmp/jura_missing_stage2.joblib",
            )
        finally:
            os.unlink(path)

        assert result.mfcc_features_extracted, (
            "MFCC extraction should work on a valid WAV even when no probe exists"
        )

    def test_duration_and_sample_rate_populated(self):
        """Metadata fields are populated regardless of probe availability."""
        from app.services.audio_deepfake import score_audio_deepfake

        wav_bytes = _make_sine_wave_wav(duration_seconds=3.0)
        path = _write_tmp_wav(wav_bytes)
        try:
            result = score_audio_deepfake(
                path,
                mfcc_probe_path="/tmp/jura_missing_stage1.joblib",
                wav2vec2_probe_path="/tmp/jura_missing_stage2.joblib",
            )
        finally:
            os.unlink(path)

        assert result.duration_seconds is not None
        # Allow ±0.1 s tolerance (resampling rounding)
        assert abs(result.duration_seconds - 3.0) < 0.1
        assert result.sample_rate == 16_000

    def test_processing_time_populated(self):
        from app.services.audio_deepfake import score_audio_deepfake

        wav_bytes = _make_sine_wave_wav(duration_seconds=1.5)
        path = _write_tmp_wav(wav_bytes)
        try:
            result = score_audio_deepfake(
                path,
                mfcc_probe_path="/tmp/jura_missing_stage1.joblib",
                wav2vec2_probe_path="/tmp/jura_missing_stage2.joblib",
            )
        finally:
            os.unlink(path)

        assert result.processing_time_ms is not None
        assert result.processing_time_ms >= 0.0


# ── API endpoint tests ────────────────────────────────────────────────────────

class TestAudioDeepfakeEndpoint:
    """FastAPI endpoint POST /forensics/audio/deepfake."""

    @pytest.fixture()
    def client(self):
        from fastapi.testclient import TestClient
        from app.api.forensics import router
        from fastapi import FastAPI

        app = FastAPI()
        app.include_router(router, prefix="/forensics")
        return TestClient(app)

    def test_valid_wav_returns_200(self, client):
        wav_bytes = _make_sine_wave_wav(duration_seconds=2.0)
        response = client.post(
            "/forensics/audio/deepfake",
            files={"file": ("test.wav", wav_bytes, "audio/wav")},
        )
        assert response.status_code == 200
        body = response.json()
        assert "verdict" in body
        assert "modelLoaded" in body or "model_loaded" in body

    def test_model_not_loaded_is_not_an_error(self, client):
        """HTTP 200 even when verdict is model_not_loaded."""
        wav_bytes = _make_sine_wave_wav(duration_seconds=2.0)
        response = client.post(
            "/forensics/audio/deepfake",
            files={"file": ("clip.wav", wav_bytes, "audio/wav")},
        )
        assert response.status_code == 200
        body = response.json()
        # If no probes deployed (normal in skeleton phase), model_loaded is false
        # and verdict is model_not_loaded — this should be 200, not 4xx/5xx.
        verdict = body.get("verdict") or body.get("model_not_loaded")
        assert verdict is not None

    def test_non_audio_file_returns_400(self, client):
        """Sending a JPEG to the audio endpoint must return 400."""
        # Minimal valid PNG header
        fake_image = (
            b"\x89PNG\r\n\x1a\n"
            b"\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01"
            b"\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx"
            b"\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00\x00"
            b"\x00\x00IEND\xaeB`\x82"
        )
        response = client.post(
            "/forensics/audio/deepfake",
            files={"file": ("photo.png", fake_image, "image/png")},
        )
        assert response.status_code == 400, (
            f"Expected 400 for non-audio extension, got {response.status_code}"
        )
