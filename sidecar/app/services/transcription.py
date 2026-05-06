# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace — Audio/Video Transcription Service.

Extracts speech from audio/video files using Whisper (faster-whisper for CPU).
Graceful degradation when faster-whisper is not installed.

Dependencies (optional):
    pip install faster-whisper

For video files, audio is extracted via FFmpeg first.
"""

import importlib.util
import logging
import os
import subprocess
import tempfile
from typing import Any

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Lazy-loaded model cache
# ---------------------------------------------------------------------------
_model_cache: dict[str, Any] = {}

# Check availability without importing CTranslate2 shared libraries.
# The actual `from faster_whisper import WhisperModel` is deferred to
# _get_model() so it only runs when transcription is first requested.
_WHISPER_AVAILABLE: bool = importlib.util.find_spec("faster_whisper") is not None


def is_whisper_available() -> bool:
    """Return True if faster-whisper is importable."""
    return _WHISPER_AVAILABLE


# ---------------------------------------------------------------------------
# Magic-byte helpers
# ---------------------------------------------------------------------------

_VIDEO_SIGNATURES: list[tuple[bytes, bytes | None, int]] = [
    # (prefix, secondary, secondary_offset)
    (b"\x00\x00\x00", b"ftyp", 4),       # MP4 / MOV / M4A (ISO BMFF)
    (b"\x1a\x45\xdf\xa3", None, 0),       # WebM / MKV (EBML)
    (b"\x00\x00\x01\xba", None, 0),       # MPEG-PS
    (b"\x00\x00\x01\xb3", None, 0),       # MPEG-1/2 video
]


def _is_video(data: bytes) -> bool:
    """Heuristic check whether *data* looks like a video container."""
    if len(data) < 12:
        return False
    for prefix, secondary, offset in _VIDEO_SIGNATURES:
        if data[:len(prefix)] == prefix:
            if secondary is None:
                return True
            if data[offset:offset + len(secondary)] == secondary:
                return True
    return False


# ---------------------------------------------------------------------------
# Audio extraction from video
# ---------------------------------------------------------------------------

def _extract_audio_from_video(video_bytes: bytes) -> bytes | None:
    """Extract audio track from video bytes as WAV via FFmpeg.

    Returns WAV bytes, or None if extraction fails or FFmpeg is absent.
    """
    tmp_video = None
    tmp_audio = None
    try:
        with tempfile.NamedTemporaryFile(suffix=".mp4", delete=False) as f:
            f.write(video_bytes)
            tmp_video = f.name

        tmp_audio = tmp_video + ".wav"

        result = subprocess.run(
            [
                "ffmpeg", "-y", "-i", tmp_video,
                "-vn",                       # drop video
                "-acodec", "pcm_s16le",      # 16-bit PCM
                "-ar", "16000",              # 16 kHz (Whisper native)
                "-ac", "1",                  # mono
                tmp_audio,
            ],
            capture_output=True,
            timeout=120,
        )

        if result.returncode != 0:
            logger.warning("FFmpeg audio extraction failed: %s", result.stderr[:500])
            return None

        with open(tmp_audio, "rb") as af:
            return af.read()

    except FileNotFoundError:
        logger.warning("FFmpeg is not installed — cannot extract audio from video")
        return None
    except subprocess.TimeoutExpired:
        logger.warning("FFmpeg timed out extracting audio from video")
        return None
    except Exception as exc:
        logger.warning("Audio extraction error: %s", exc)
        return None
    finally:
        if tmp_video and os.path.exists(tmp_video):
            os.unlink(tmp_video)
        if tmp_audio and os.path.exists(tmp_audio):
            os.unlink(tmp_audio)


# ---------------------------------------------------------------------------
# Whisper model loader
# ---------------------------------------------------------------------------

def _get_model(model_size: str) -> Any:
    """Load (or retrieve cached) faster-whisper model.

    Models are downloaded on first use (~75 MB for tiny, ~150 MB for base).
    """
    if not _WHISPER_AVAILABLE:
        raise RuntimeError("faster-whisper is not installed")

    allowed = ("tiny", "base", "small")
    if model_size not in allowed:
        model_size = "base"

    if model_size not in _model_cache:
        from faster_whisper import WhisperModel

        logger.info("Loading Whisper model '%s' (CPU, int8)...", model_size)
        _model_cache[model_size] = WhisperModel(
            model_size, device="cpu", compute_type="int8",
        )
        logger.info("Whisper model '%s' loaded.", model_size)

    return _model_cache[model_size]


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def perform_transcription(
    media_bytes: bytes,
    language: str | None = None,
    model_size: str = "base",
) -> dict:
    """
    Transcribe speech from audio or video bytes.

    For video files, extracts the audio track via FFmpeg first.
    Uses faster-whisper for efficient CPU inference.

    Args:
        media_bytes: Raw bytes of an audio or video file.
        language: ISO 639-1 code (e.g. "en"). None = auto-detect.
        model_size: Whisper model size — "tiny", "base", or "small".

    Returns:
        dict matching TranscriptionResponse schema.
    """
    if not media_bytes:
        return _error_result("Empty file provided")

    if not _WHISPER_AVAILABLE:
        return _error_result(
            "Transcription unavailable: faster-whisper is not installed. "
            "Install with: pip install faster-whisper"
        )

    # If input looks like video, extract audio first.
    audio_bytes = media_bytes
    if _is_video(media_bytes):
        extracted = _extract_audio_from_video(media_bytes)
        if extracted is None:
            return _error_result(
                "Could not extract audio from video. "
                "Ensure FFmpeg is installed."
            )
        audio_bytes = extracted

    # Write audio to temp file for faster-whisper.
    tmp_path = None
    try:
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            f.write(audio_bytes)
            tmp_path = f.name

        model = _get_model(model_size)

        kwargs: dict[str, Any] = {}
        if language:
            kwargs["language"] = language

        segments_iter, info = model.transcribe(tmp_path, **kwargs)

        segments = []
        full_text_parts = []

        for seg in segments_iter:
            segments.append({
                "start": round(seg.start, 3),
                "end": round(seg.end, 3),
                "text": seg.text.strip(),
            })
            full_text_parts.append(seg.text.strip())

        full_text = " ".join(full_text_parts)

        return {
            "text": full_text,
            "segments": segments,
            "language": info.language,
            "language_probability": round(info.language_probability, 4),
            "duration": round(info.duration, 3),
            "model_size": model_size,
            "success": True,
            "message": (
                f"Transcribed {info.duration:.1f}s of audio "
                f"({info.language}, {len(segments)} segments)"
            ),
        }

    except RuntimeError as exc:
        return _error_result(str(exc))
    except Exception as exc:
        logger.exception("Transcription failed")
        return _error_result(f"Transcription failed: {exc}")
    finally:
        if tmp_path and os.path.exists(tmp_path):
            os.unlink(tmp_path)


def _error_result(msg: str) -> dict:
    """Return a failed transcription result."""
    return {
        "text": "",
        "segments": [],
        "language": None,
        "language_probability": None,
        "duration": None,
        "model_size": "base",
        "success": False,
        "message": msg,
    }
