"""
Jura Trace Sidecar — Video frame extraction via FFmpeg.

Extracts evenly-spaced frames from a video file as base64-encoded JPEG
images, suitable for forensic analysis or thumbnail generation.
FFmpeg is optional — graceful degradation when not installed.
"""

import base64
import json
import os
import subprocess
import tempfile

from app.models.schemas import VideoFramesResponse


def perform_frame_extraction(
    video_bytes: bytes, count: int = 6,
) -> VideoFramesResponse:
    """
    Extract N evenly-spaced frames from a video as JPEG base64 strings.

    Args:
        video_bytes: Raw bytes of the input video file.
        count: Number of frames to extract (1-12).

    Returns:
        VideoFramesResponse with list of base64 JPEG frames.
    """
    with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as f:
        f.write(video_bytes)
        tmp_path = f.name

    try:
        # Get video duration via ffprobe
        duration = _get_duration(tmp_path)
        if duration is None:
            return _error_result("Could not determine video duration")
        if duration <= 0:
            return _error_result("Video has zero or negative duration")

        # Calculate evenly-spaced timestamps
        timestamps = [
            duration * i / (count + 1) for i in range(1, count + 1)
        ]

        frames: list[str] = []
        for ts in timestamps:
            frame_b64 = _extract_frame_at(tmp_path, ts)
            if frame_b64 is not None:
                frames.append(frame_b64)

        return VideoFramesResponse(
            frames=frames,
            count=len(frames),
            duration=duration,
            success=True,
            message=f"Extracted {len(frames)} frames from {duration:.1f}s video",
        )
    except FileNotFoundError:
        return _error_result("FFmpeg is not installed. Install from ffmpeg.org.")
    except subprocess.TimeoutExpired:
        return _error_result("FFmpeg timed out processing this file")
    except Exception as e:
        return _error_result(f"Frame extraction failed: {e}")
    finally:
        os.unlink(tmp_path)


def _get_duration(file_path: str) -> float | None:
    """Get video duration in seconds via ffprobe."""
    result = subprocess.run(
        [
            "ffprobe", "-v", "quiet", "-print_format", "json",
            "-show_format", file_path,
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        return None

    try:
        fmt = json.loads(result.stdout).get("format", {})
        return float(fmt["duration"])
    except (KeyError, ValueError, json.JSONDecodeError):
        return None


def _extract_frame_at(file_path: str, timestamp: float) -> str | None:
    """Extract a single frame at the given timestamp as a base64 JPEG string."""
    result = subprocess.run(
        [
            "ffmpeg", "-ss", str(timestamp), "-i", file_path,
            "-vframes", "1", "-f", "image2", "-c:v", "mjpeg",
            "-q:v", "2", "pipe:1",
        ],
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0 or not result.stdout:
        return None

    return base64.b64encode(result.stdout).decode("utf-8")


def _error_result(msg: str) -> VideoFramesResponse:
    """Return a VideoFramesResponse indicating failure."""
    return VideoFramesResponse(
        frames=[],
        count=0,
        duration=None,
        success=False,
        message=msg,
    )
