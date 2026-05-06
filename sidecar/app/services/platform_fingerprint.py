# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Social media re-upload platform fingerprinting.

Identifies which social media platform (WhatsApp, Telegram, Facebook,
Instagram, Twitter/X, Signal, WeChat) processed an image by analysing
JPEG compression signatures, resolution patterns, and metadata stripping.

Each platform applies characteristic processing: specific quantisation
table patterns, maximum dimension limits, and EXIF handling. This service
compares the observed image properties against known platform profiles
and returns ranked candidates with confidence scores.
"""

import io
import struct
from typing import Any

from PIL import Image


def extract_jpeg_quality(image_bytes: bytes) -> int | None:
    """Estimate JPEG quality from quantisation tables (DQT markers).

    Parses raw JPEG bytes for DQT (0xFFDB) markers, extracts the
    luminance quantisation table, and estimates the original quality
    factor by comparing against the JPEG standard baseline tables
    using the Independent JPEG Group's quality estimation formula.

    Returns:
        Estimated JPEG quality (1-100), or None if no DQT found or
        the file is not a JPEG.
    """
    if len(image_bytes) < 2 or image_bytes[:2] != b"\xff\xd8":
        return None  # Not a JPEG

    # IJG standard luminance quantisation table (quality 50)
    std_lum_table = [
        16, 11, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 69, 56,
        14, 17, 22, 29, 51, 87, 80, 62,
        18, 22, 37, 56, 68, 109, 103, 77,
        24, 35, 55, 64, 81, 104, 113, 92,
        49, 64, 78, 87, 103, 121, 120, 101,
        72, 92, 95, 98, 112, 100, 103, 99,
    ]

    pos = 2
    while pos < len(image_bytes) - 1:
        if image_bytes[pos] != 0xFF:
            pos += 1
            continue

        marker = image_bytes[pos + 1]
        pos += 2

        if marker == 0xDB:  # DQT marker
            if pos + 2 > len(image_bytes):
                break
            length = struct.unpack(">H", image_bytes[pos : pos + 2])[0]
            pos += 2

            if pos >= len(image_bytes):
                break

            precision_and_id = image_bytes[pos]
            table_id = precision_and_id & 0x0F
            precision = (precision_and_id >> 4) & 0x0F

            if table_id == 0:  # Luminance table
                entry_size = 2 if precision == 1 else 1
                table_bytes = image_bytes[pos + 1 : pos + 1 + 64 * entry_size]

                if len(table_bytes) < 64 * entry_size:
                    pos += length - 2
                    continue

                if precision == 1:
                    qt = [
                        struct.unpack(">H", table_bytes[i * 2 : i * 2 + 2])[0]
                        for i in range(64)
                    ]
                else:
                    qt = list(table_bytes[:64])

                # IJG quality estimation: compare against standard table
                # Sum the ratio of each coefficient to the standard
                total = 0.0
                count = 0
                for i in range(64):
                    if std_lum_table[i] > 0:
                        total += qt[i] / std_lum_table[i]
                        count += 1

                if count == 0:
                    return None

                avg_ratio = total / count

                # Convert ratio to quality factor
                # ratio = (200 - 2*Q) / 100 for Q >= 50
                # ratio = 50 / Q for Q < 50
                if avg_ratio < 1.0:
                    quality = int(50.0 / max(avg_ratio, 0.01))
                    quality = min(quality, 100)
                else:
                    quality = int(max(1, 100.0 - 50.0 * (avg_ratio - 1.0)))
                    quality = max(quality, 1)

                return quality

            pos += length - 2
            continue

        # Skip non-DQT markers
        if marker in (0xD8, 0xD9):
            continue
        if 0xD0 <= marker <= 0xD7:
            continue
        if marker == 0xDA:  # Start of scan — stop parsing
            break

        if pos + 2 > len(image_bytes):
            break
        length = struct.unpack(">H", image_bytes[pos : pos + 2])[0]
        pos += length

    return None


def analyse_platform(image_bytes: bytes) -> dict[str, Any]:
    """Identify social media platform processing from image characteristics.

    Examines JPEG quality, maximum dimension, EXIF stripping, and
    resolution patterns to match against known platform signatures.

    Args:
        image_bytes: Raw bytes of the image file.

    Returns:
        Dictionary with keys:
          - detected: whether any platform signature was matched
          - platform: best-match platform name (or None)
          - confidence: confidence score for best match (0.0-1.0)
          - allCandidates: list of all matching platforms with scores
          - maxDimension: longest side in pixels
          - estimatedQuality: JPEG quality estimate (or None)
          - hasExif: whether EXIF data is present
          - summary: human-readable summary string
    """
    try:
        img = Image.open(io.BytesIO(image_bytes))
    except Exception as exc:
        return {
            "detected": False,
            "platform": None,
            "confidence": 0.0,
            "allCandidates": [],
            "maxDimension": None,
            "estimatedQuality": None,
            "hasExif": False,
            "summary": f"Could not open image: {exc}",
        }

    w, h = img.size
    max_dim = max(w, h)
    quality = extract_jpeg_quality(image_bytes)

    # Check for EXIF presence/stripping
    has_exif = False
    try:
        exif = img.getexif()
        has_exif = bool(exif)
    except Exception:
        pass

    candidates: list[tuple[str, float]] = []

    # ── WhatsApp ──────────────────────────────────────────────────────
    # JPEG Q 75-80, max 1600px (older) or 1920px (newer), strips all EXIF
    if max_dim <= 1920 and quality is not None and 73 <= quality <= 82 and not has_exif:
        conf = 0.7
        # Tighter match for classic WhatsApp resolution
        if max_dim <= 1600:
            conf = 0.8
        candidates.append(("WhatsApp", conf))

    # ── Telegram ──────────────────────────────────────────────────────
    # JPEG Q ~87, max 2560px on longest side, strips EXIF
    if max_dim <= 2560 and quality is not None and 85 <= quality <= 89 and not has_exif:
        conf = 0.6
        # Exact 2560px boundary is a strong Telegram indicator
        if 2500 <= max_dim <= 2560:
            conf = 0.75
        candidates.append(("Telegram", conf))

    # ── Facebook ──────────────────────────────────────────────────────
    # JPEG Q 71-85, strips EXIF, max ~2048px
    if max_dim <= 2048 and quality is not None and 70 <= quality <= 85 and not has_exif:
        conf = 0.5
        # Strong Facebook indicator: exactly 2048px longest side
        if max_dim == 2048:
            conf = 0.7
        candidates.append(("Facebook", conf))

    # ── Instagram ─────────────────────────────────────────────────────
    # JPEG Q 71-85, strips EXIF, max 1080px (feed) or 1920px (stories)
    if max_dim <= 1080 and quality is not None and 70 <= quality <= 85 and not has_exif:
        conf = 0.6
        # Square aspect ratio is an Instagram indicator
        if w == h:
            conf = 0.75
        candidates.append(("Instagram", conf))

    # ── Twitter/X ─────────────────────────────────────────────────────
    # JPEG Q 85-87, max 4096px, may preserve some EXIF
    if max_dim <= 4096 and quality is not None and 83 <= quality <= 88:
        conf = 0.5
        # Twitter preserves some EXIF — presence doesn't exclude it
        if not has_exif:
            conf = 0.55
        candidates.append(("Twitter/X", conf))

    # ── Signal ────────────────────────────────────────────────────────
    # Similar to WhatsApp but slightly different Q range
    if max_dim <= 2048 and quality is not None and 78 <= quality <= 85 and not has_exif:
        conf = 0.45
        candidates.append(("Signal", conf))

    # ── WeChat ────────────────────────────────────────────────────────
    # Max 1280px, Q ~75, strips EXIF
    if max_dim <= 1280 and quality is not None and 72 <= quality <= 78 and not has_exif:
        conf = 0.55
        # Exact 1280px boundary is a strong WeChat indicator
        if max_dim == 1280:
            conf = 0.7
        candidates.append(("WeChat", conf))

    # Sort by confidence descending
    candidates.sort(key=lambda x: -x[1])

    if candidates:
        best = candidates[0]
        return {
            "detected": True,
            "platform": best[0],
            "confidence": best[1],
            "allCandidates": [
                {"platform": c[0], "confidence": c[1]} for c in candidates
            ],
            "maxDimension": max_dim,
            "estimatedQuality": quality,
            "hasExif": has_exif,
            "summary": (
                f"Image characteristics match {best[0]} processing "
                f"(confidence {best[1]:.0%}). "
                f"Max dimension: {max_dim}px, estimated quality: {quality}."
            ),
        }
    else:
        return {
            "detected": False,
            "platform": None,
            "confidence": 0.0,
            "allCandidates": [],
            "maxDimension": max_dim,
            "estimatedQuality": quality,
            "hasExif": has_exif,
            "summary": (
                f"No social media platform signature detected. "
                f"Max dimension: {max_dim}px, estimated quality: {quality}."
            ),
        }
