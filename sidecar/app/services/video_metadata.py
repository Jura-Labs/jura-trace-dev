"""
Jura Trace Sidecar — Video metadata extraction via FFmpeg/ffprobe.

Extracts video stream metadata (codec, resolution, frame rate, duration)
and audio stream metadata from video files using ffprobe subprocess calls.
FFmpeg is optional — graceful degradation when not installed.
"""

import json
import os
import subprocess
import tempfile

from app.models.schemas import VideoMetadataResponse


def perform_video_metadata(video_bytes: bytes) -> VideoMetadataResponse:
    """
    Extract video metadata using ffprobe.

    Args:
        video_bytes: Raw bytes of the input video file.

    Returns:
        VideoMetadataResponse with codec, resolution, fps, duration, etc.
    """
    tmp_path: str | None = None
    try:
        with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as f:
            f.write(video_bytes)
            tmp_path = f.name

        result = subprocess.run(
            [
                "ffprobe", "-v", "quiet", "-print_format", "json",
                "-show_format", "-show_streams", tmp_path,
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            return _error_result("FFmpeg not available or file not recognised")

        probe = json.loads(result.stdout)

        video_stream = next(
            (s for s in probe.get("streams", []) if s.get("codec_type") == "video"),
            None,
        )
        audio_stream = next(
            (s for s in probe.get("streams", []) if s.get("codec_type") == "audio"),
            None,
        )
        fmt = probe.get("format", {})

        codec = video_stream.get("codec_name") if video_stream else None
        width = int(video_stream.get("width", 0)) if video_stream else None
        height = int(video_stream.get("height", 0)) if video_stream else None

        if video_stream:
            message = (
                f"Video metadata extracted "
                f"({codec or 'unknown'} {width or '?'}x{height or '?'})"
            )
        else:
            message = "No video stream found"

        return VideoMetadataResponse(
            duration=float(fmt.get("duration", 0)),
            codec=codec,
            width=width,
            height=height,
            fps=_parse_fps(video_stream.get("r_frame_rate", "0/1")) if video_stream else None,
            has_audio=audio_stream is not None,
            audio_codec=audio_stream.get("codec_name") if audio_stream else None,
            bitrate=int(fmt["bit_rate"]) if fmt.get("bit_rate") else None,
            file_size=int(fmt["size"]) if fmt.get("size") else None,
            success=True,
            message=message,
        )
    except FileNotFoundError:
        return _error_result("FFmpeg is not installed. Install from ffmpeg.org.")
    except subprocess.TimeoutExpired:
        return _error_result("FFmpeg timed out processing this file")
    except Exception as e:
        return _error_result(f"Video analysis failed: {e}")
    finally:
        if tmp_path is not None:
            try:
                os.unlink(tmp_path)
            except FileNotFoundError:
                pass


def _parse_fps(fps_str: str) -> float | None:
    """Parse a rational frame rate string like '30/1' into a float."""
    try:
        num, den = fps_str.split("/")
        return round(int(num) / int(den), 2)
    except Exception:
        return None


def _error_result(msg: str) -> VideoMetadataResponse:
    """Return a VideoMetadataResponse indicating failure."""
    return VideoMetadataResponse(
        duration=None,
        codec=None,
        width=None,
        height=None,
        fps=None,
        has_audio=False,
        audio_codec=None,
        bitrate=None,
        file_size=None,
        success=False,
        message=msg,
    )
