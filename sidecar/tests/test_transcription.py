# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the audio/video transcription service."""

import shutil
import subprocess
from unittest.mock import patch

import pytest

from app.services.transcription import (
    _WHISPER_AVAILABLE,
    _is_video,
    perform_transcription,
)

# JTV-138 (2026-05-02) — transcription dropped from v1.0.
pytestmark = pytest.mark.skip(reason="JTV-138 v1.0 drop — re-enable under JTV-139")

HAS_FFMPEG = shutil.which("ffmpeg") is not None


def _make_test_audio() -> bytes:
    """Create a minimal test WAV file using ffmpeg (1s sine wave)."""
    result = subprocess.run(
        [
            "ffmpeg", "-y", "-f", "lavfi", "-i",
            "sine=frequency=440:duration=1",
            "-c:a", "pcm_s16le", "-ar", "16000", "-ac", "1",
            "-f", "wav", "pipe:1",
        ],
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        pytest.skip("Could not generate test audio with ffmpeg")
    return result.stdout


class TestTranscription:
    @pytest.mark.skipif(
        not _WHISPER_AVAILABLE, reason="faster-whisper not installed"
    )
    @pytest.mark.skipif(not HAS_FFMPEG, reason="ffmpeg not installed")
    def test_returns_valid_response(self):
        """A valid audio file should produce a successful transcription result."""
        audio_bytes = _make_test_audio()
        result = perform_transcription(audio_bytes, model_size="tiny")
        assert result["success"] is True
        assert isinstance(result["text"], str)
        assert isinstance(result["segments"], list)
        assert result["language"] is not None
        assert result["duration"] is not None
        assert result["duration"] > 0
        assert result["model_size"] == "tiny"

    def test_handles_missing_whisper(self):
        """When faster-whisper is not importable, return graceful failure."""
        with patch(
            "app.services.transcription._WHISPER_AVAILABLE", False,
        ):
            result = perform_transcription(b"fake audio data")
            assert result["success"] is False
            assert "not installed" in result["message"]

    def test_handles_invalid_audio(self):
        """Invalid audio data should return success=False, not crash."""
        if not _WHISPER_AVAILABLE:
            pytest.skip("faster-whisper not installed")
        result = perform_transcription(b"this is not audio data at all")
        assert result["success"] is False

    def test_handles_empty_file(self):
        """An empty file should return success=False."""
        result = perform_transcription(b"")
        assert result["success"] is False
        assert "Empty" in result["message"]

    def test_is_video_mp4(self):
        """MP4 magic bytes should be detected as video."""
        # Minimal ftyp box header
        mp4_header = b"\x00\x00\x00\x1cftypisom"
        assert _is_video(mp4_header) is True

    def test_is_video_wav(self):
        """WAV data should not be detected as video."""
        wav_header = b"RIFF\x00\x00\x00\x00WAVEfmt "
        assert _is_video(wav_header) is False

    def test_is_video_short_data(self):
        """Very short data should not crash the video check."""
        assert _is_video(b"") is False
        assert _is_video(b"\x00") is False

    @pytest.mark.skipif(not HAS_FFMPEG, reason="ffmpeg not installed")
    def test_video_audio_extraction_with_invalid_data(self):
        """Attempting to extract audio from non-video should return None."""
        from app.services.transcription import _extract_audio_from_video

        result = _extract_audio_from_video(b"not a video file")
        assert result is None
