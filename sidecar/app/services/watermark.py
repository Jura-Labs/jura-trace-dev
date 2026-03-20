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

        # Try to decode as UTF-8, strip null bytes
        try:
            extracted_str = extracted_bytes.rstrip(b"\x00").decode(
                "utf-8", errors="replace"
            )
        except Exception:
            extracted_str = extracted_bytes.hex()

        # Check if we got anything meaningful
        has_content = (
            len(extracted_str.strip()) > 0
            and extracted_str.strip() != "\x00" * len(extracted_str)
        )

        return {
            "extracted_payload": extracted_str if has_content else None,
            "extracted_hex": extracted_bytes.hex(),
            "payload_length": payload_length,
            "algorithm": method,
            "has_watermark": has_content,
            "confidence": 0.8 if has_content else 0.1,
            "success": True,
            "message": (
                f"Watermark {'found' if has_content else 'not detected'}"
            ),
        }
    except Exception as e:
        return _extract_error(f"Watermark extraction failed: {e}")


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
