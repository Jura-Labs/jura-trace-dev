# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the video metadata extraction service."""

import pytest

# JTV-138 (2026-05-02) — FFprobe video metadata dropped from v1.0.
pytestmark = pytest.mark.skip(reason="JTV-138 v1.0 drop — re-enable under JTV-139")

import shutil
import subprocess
from unittest.mock import patch

import pytest

from app.services.video_metadata import perform_video_metadata

HAS_FFMPEG = shutil.which("ffprobe") is not None


def _make_test_video() -> bytes:
    """Create a minimal test video using ffmpeg (1s, 320x240, silent)."""
    result = subprocess.run(
        [
            "ffmpeg", "-y", "-f", "lavfi", "-i",
            "color=c=blue:s=320x240:d=1:r=25",
            "-c:v", "libx264", "-t", "1",
            "-f", "mp4", "-movflags", "+frag_keyframe+empty_moov",
            "pipe:1",
        ],
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        pytest.skip("Could not generate test video with ffmpeg")
    return result.stdout


class TestVideoMetadata:
    def test_ffmpeg_not_found(self):
        """When ffprobe is not on PATH, return graceful failure."""
        with patch(
            "app.services.video_metadata.subprocess.run",
            side_effect=FileNotFoundError("ffprobe not found"),
        ):
            result = perform_video_metadata(b"fake video data")
            assert result.success is False
            assert "not installed" in result.message

    def test_error_on_invalid_data(self):
        """Invalid data should return success=False, not crash."""
        result = perform_video_metadata(b"this is not a video file")
        assert result.success is False

    @pytest.mark.skipif(not HAS_FFMPEG, reason="ffprobe not installed")
    def test_valid_metadata_fields(self):
        """A valid video should return populated metadata fields."""
        video_bytes = _make_test_video()
        result = perform_video_metadata(video_bytes)
        assert result.success is True
        assert result.codec is not None
        assert result.width == 320
        assert result.height == 240
        assert result.duration is not None
        assert result.duration > 0
        assert result.fps is not None
        assert result.fps > 0
        assert isinstance(result.message, str)
