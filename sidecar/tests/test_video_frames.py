# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for the video frame extraction service."""

import shutil
import subprocess
from unittest.mock import patch

import pytest

from app.services.video_frames import perform_frame_extraction

# JTV-138 (2026-05-02) — video frame extraction dropped from v1.0.
pytestmark = pytest.mark.skip(reason="JTV-138 v1.0 drop — re-enable under JTV-139")

HAS_FFMPEG = shutil.which("ffprobe") is not None


def _make_test_video() -> bytes:
    """Create a minimal test video using ffmpeg (2s, 320x240, silent)."""
    result = subprocess.run(
        [
            "ffmpeg", "-y", "-f", "lavfi", "-i",
            "color=c=red:s=320x240:d=2:r=25",
            "-c:v", "libx264", "-t", "2",
            "-f", "mp4", "-movflags", "+frag_keyframe+empty_moov",
            "pipe:1",
        ],
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        pytest.skip("Could not generate test video with ffmpeg")
    return result.stdout


class TestVideoFrames:
    def test_ffmpeg_not_found(self):
        """When ffmpeg is not on PATH, return graceful failure."""
        with patch(
            "app.services.video_frames.subprocess.run",
            side_effect=FileNotFoundError("ffmpeg not found"),
        ):
            result = perform_frame_extraction(b"fake video data")
            assert result.success is False
            assert "not installed" in result.message

    def test_error_on_invalid_data(self):
        """Invalid data should return success=False, not crash."""
        result = perform_frame_extraction(b"this is not a video file")
        assert result.success is False

    @pytest.mark.skipif(not HAS_FFMPEG, reason="ffprobe not installed")
    def test_frame_extraction_returns_result(self):
        """A valid video should yield extracted frames as base64 strings."""
        video_bytes = _make_test_video()
        result = perform_frame_extraction(video_bytes, count=3)
        assert result.success is True
        assert result.count > 0
        assert len(result.frames) == result.count
        assert result.duration is not None
        assert result.duration > 0
        # Frames should be non-empty base64 strings
        for frame in result.frames:
            assert len(frame) > 100
            assert isinstance(frame, str)
