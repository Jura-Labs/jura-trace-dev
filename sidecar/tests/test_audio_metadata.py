# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the audio metadata extraction service."""

import pytest

# JTV-138 (2026-05-02) — FFprobe audio metadata dropped from v1.0.
pytestmark = pytest.mark.skip(reason="JTV-138 v1.0 drop — re-enable under JTV-139")

import shutil
import subprocess
from unittest.mock import patch

import pytest

from app.services.audio_metadata import perform_audio_metadata

HAS_FFMPEG = shutil.which("ffprobe") is not None


def _make_test_audio() -> bytes:
    """Create a minimal test audio file using ffmpeg (1s sine wave)."""
    result = subprocess.run(
        [
            "ffmpeg", "-y", "-f", "lavfi", "-i",
            "sine=frequency=440:duration=1",
            "-c:a", "libmp3lame", "-b:a", "128k",
            "-f", "mp3", "pipe:1",
        ],
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        pytest.skip("Could not generate test audio with ffmpeg")
    return result.stdout


class TestAudioMetadata:
    def test_ffmpeg_not_found(self):
        """When ffprobe is not on PATH, return graceful failure."""
        with patch(
            "app.services.audio_metadata.subprocess.run",
            side_effect=FileNotFoundError("ffprobe not found"),
        ):
            result = perform_audio_metadata(b"fake audio data")
            assert result.success is False
            assert "not installed" in result.message

    def test_error_on_invalid_data(self):
        """Invalid data should return success=False, not crash."""
        result = perform_audio_metadata(b"this is not an audio file")
        assert result.success is False

    @pytest.mark.skipif(not HAS_FFMPEG, reason="ffprobe not installed")
    def test_valid_metadata_fields(self):
        """A valid audio file should return populated metadata fields."""
        audio_bytes = _make_test_audio()
        result = perform_audio_metadata(audio_bytes)
        assert result.success is True
        assert result.codec is not None
        assert result.duration is not None
        assert result.duration > 0
        assert result.sample_rate is not None
        assert result.sample_rate > 0
        assert result.channels is not None
        assert result.channels >= 1
        assert isinstance(result.message, str)
