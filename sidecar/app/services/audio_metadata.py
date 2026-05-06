# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Audio metadata extraction via FFmpeg/ffprobe.

Extracts audio stream metadata (codec, sample rate, channels, duration,
bitrate) from audio files using ffprobe subprocess calls.
FFmpeg is optional — graceful degradation when not installed.
"""

import json
import os
import subprocess
import tempfile

from app.models.schemas import AudioMetadataResponse


def perform_audio_metadata(audio_bytes: bytes) -> AudioMetadataResponse:
    """
    Extract audio metadata using ffprobe.

    Args:
        audio_bytes: Raw bytes of the input audio file.

    Returns:
        AudioMetadataResponse with codec, sample_rate, channels, duration, etc.
    """
    tmp_path: str | None = None
    try:
        with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as f:
            f.write(audio_bytes)
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

        audio_stream = next(
            (s for s in probe.get("streams", []) if s.get("codec_type") == "audio"),
            None,
        )
        fmt = probe.get("format", {})

        if not audio_stream:
            return _error_result("No audio stream found in file")

        codec = audio_stream.get("codec_name")
        sample_rate = (
            int(audio_stream["sample_rate"])
            if audio_stream.get("sample_rate")
            else None
        )
        channels = (
            int(audio_stream["channels"])
            if audio_stream.get("channels")
            else None
        )

        return AudioMetadataResponse(
            duration=float(fmt.get("duration", 0)),
            codec=codec,
            sample_rate=sample_rate,
            channels=channels,
            bitrate=int(fmt["bit_rate"]) if fmt.get("bit_rate") else None,
            file_size=int(fmt["size"]) if fmt.get("size") else None,
            success=True,
            message=(
                f"Audio metadata extracted "
                f"({codec or 'unknown'}, {sample_rate or '?'} Hz, "
                f"{channels or '?'} ch)"
            ),
        )
    except FileNotFoundError:
        return _error_result("FFmpeg is not installed. Install from ffmpeg.org.")
    except subprocess.TimeoutExpired:
        return _error_result("FFmpeg timed out processing this file")
    except Exception as e:
        return _error_result(f"Audio analysis failed: {e}")
    finally:
        if tmp_path is not None:
            try:
                os.unlink(tmp_path)
            except FileNotFoundError:
                pass


def _error_result(msg: str) -> AudioMetadataResponse:
    """Return an AudioMetadataResponse indicating failure."""
    return AudioMetadataResponse(
        duration=None,
        codec=None,
        sample_rate=None,
        channels=None,
        bitrate=None,
        file_size=None,
        success=False,
        message=msg,
    )
