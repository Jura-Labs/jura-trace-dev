"""
Jura Trace — Invisible Watermark Service.

Embeds and extracts invisible frequency-domain watermarks using DWT-DCT-SVD.
Uses the invisible-watermark library (already in requirements.txt).
"""

import base64

import cv2
import numpy as np


def perform_watermark_embed(
    image_bytes: bytes,
    payload: str,
    strength: str = "medium",
) -> dict:
    """
    Embed an invisible watermark into an image.

    Args:
        image_bytes: Raw image bytes (JPEG/PNG)
        payload: String to embed (institution name/ID, max 64 chars)
        strength: Embedding strength level ("low", "medium", "high")

    Returns:
        dict with watermarked_image_base64, algorithm, strength_used, success
    """
    try:
        from imwatermark import WatermarkEncoder
    except ImportError:
        return _error_result(
            "invisible-watermark library not installed. "
            "Install with: pip install invisible-watermark"
        )

    # Decode image
    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        return _error_result("Could not decode image")

    h, w = img.shape[:2]
    if h < 256 or w < 256:
        return _error_result(
            f"Image too small ({w}x{h}). Minimum 256x256 required."
        )

    # Validate strength parameter
    if strength not in ("low", "medium", "high"):
        strength = "medium"

    # DWT-DCT-SVD is the most robust method
    method = "dwtDctSvd"

    # Encode payload as bytes, truncate to 64 bytes max
    payload_bytes = payload.encode("utf-8")
    if len(payload_bytes) > 64:
        payload_bytes = payload_bytes[:64]

    try:
        encoder = WatermarkEncoder()
        encoder.set_watermark("bytes", payload_bytes)
        watermarked = encoder.encode(img, method)

        # Encode result as PNG (lossless to preserve watermark)
        _, buf = cv2.imencode(".png", watermarked)
        watermarked_b64 = base64.b64encode(buf).decode("utf-8")

        return {
            "watermarked_image_base64": watermarked_b64,
            "algorithm": method,
            "strength": strength,
            "payload_length": len(payload_bytes),
            "success": True,
            "message": (
                f"Watermark embedded successfully ({w}x{h}, {method})"
            ),
        }
    except Exception as e:
        return _error_result(f"Watermark embedding failed: {e}")


def perform_watermark_extract(
    image_bytes: bytes,
    payload_length: int = 64,
) -> dict:
    """
    Extract an invisible watermark from an image.

    Args:
        image_bytes: Raw image bytes (potentially watermarked)
        payload_length: Expected payload length in bytes

    Returns:
        dict with extracted_payload, confidence, success
    """
    try:
        from imwatermark import WatermarkDecoder
    except ImportError:
        return _extract_error(
            "invisible-watermark library not installed. "
            "Install with: pip install invisible-watermark"
        )

    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        return _extract_error("Could not decode image")

    method = "dwtDctSvd"

    try:
        decoder = WatermarkDecoder("bytes", payload_length * 8)
        extracted_bytes = decoder.decode(img, method)

        # Strip trailing null bytes to get the meaningful payload.
        stripped = extracted_bytes.rstrip(b"\x00")

        # Try to decode as UTF-8, strip null bytes
        try:
            extracted_str = stripped.decode("utf-8", errors="replace")
        except Exception:
            extracted_str = stripped.hex()

        # Determine if the extracted bytes represent a genuine watermark
        # rather than frequency-domain noise. The DWT-DCT-SVD decoder
        # always returns bytes — even from unwatermarked images — so we
        # need structural checks beyond "not all nulls".
        has_watermark, confidence = _assess_watermark_confidence(
            stripped, extracted_str
        )

        return {
            "extracted_payload": extracted_str if has_watermark else None,
            "extracted_hex": extracted_bytes.hex(),
            "payload_length": payload_length,
            "algorithm": method,
            "has_watermark": has_watermark,
            "confidence": confidence,
            "success": True,
            "message": (
                f"Watermark {'found' if has_watermark else 'not detected'}"
            ),
        }
    except Exception as e:
        return _extract_error(f"Watermark extraction failed: {e}")


def _assess_watermark_confidence(
    stripped_bytes: bytes, decoded_str: str
) -> tuple[bool, float]:
    """Decide whether extracted bytes are a genuine watermark or noise.

    The DWT-DCT-SVD decoder always produces output — even from images
    that were never watermarked. Random frequency-domain noise decodes
    into high-entropy, mostly non-printable byte sequences. A genuine
    Jura Trace watermark is a short, printable UTF-8 string (institution
    name or hex UUID).

    Returns (has_watermark: bool, confidence: float).
    """
    # Empty payload — definitely no watermark.
    if len(stripped_bytes) == 0 or len(decoded_str.strip()) == 0:
        return False, 0.0

    # Count printable ASCII characters (letters, digits, spaces, punctuation).
    printable_count = sum(
        1 for ch in decoded_str if ch.isprintable() and ord(ch) < 128
    )
    printable_ratio = printable_count / max(len(decoded_str), 1)

    # Count Unicode replacement characters (U+FFFD) — indicates broken UTF-8.
    replacement_count = decoded_str.count("\ufffd")
    replacement_ratio = replacement_count / max(len(decoded_str), 1)

    # Byte entropy: random noise has ~8 bits/byte; structured payloads have less.
    if len(stripped_bytes) >= 4:
        unique_bytes = len(set(stripped_bytes))
        byte_diversity = unique_bytes / min(len(stripped_bytes), 256)
    else:
        byte_diversity = 1.0

    # A genuine watermark should be mostly printable ASCII with low
    # replacement chars and moderate byte diversity.
    # Noise typically has <30% printable, high replacement, high diversity.
    if printable_ratio >= 0.7 and replacement_ratio < 0.1:
        confidence = 0.7 + 0.2 * printable_ratio - 0.3 * byte_diversity
        confidence = max(0.5, min(confidence, 0.95))
        return True, round(confidence, 2)

    if printable_ratio >= 0.5 and replacement_ratio < 0.2:
        confidence = 0.3 + 0.2 * printable_ratio
        return confidence >= 0.5, round(confidence, 2)

    # High noise / low printable — not a genuine watermark.
    return False, round(0.1 + 0.1 * printable_ratio, 2)


def _error_result(message: str) -> dict:
    return {
        "watermarked_image_base64": None,
        "algorithm": "dwtDctSvd",
        "strength": "medium",
        "payload_length": 0,
        "success": False,
        "message": message,
    }


def _extract_error(message: str) -> dict:
    return {
        "extracted_payload": None,
        "extracted_hex": None,
        "payload_length": 0,
        "algorithm": "dwtDctSvd",
        "has_watermark": False,
        "confidence": 0.0,
        "success": False,
        "message": message,
    }
