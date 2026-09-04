# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — AI Image Description Service.

Generates a natural-language description of an image using a local Ollama
LLaVA multimodal model (llava:7b by default).  No external network calls are
made — inference runs entirely on-device.

If Ollama is unavailable, or if LLaVA is not pulled, the service degrades
gracefully: the endpoint returns HTTP 200 with ``success=False`` and a
human-readable ``message`` explaining the situation.

Algorithm:
1. Receive raw image bytes.
2. Base64-encode the image.
3. POST to the local Ollama ``/api/generate`` endpoint with the multimodal
   payload (image + text prompt).
4. Return the model's description, stripped of whitespace.
"""

from __future__ import annotations

import base64
import logging

import httpx

from app.models.schemas import ImageDescribeResponse

logger = logging.getLogger(__name__)

# ── Constants ──────────────────────────────────────────────────────────────────

_DESCRIBE_PROMPT = (
    "Describe this image in detail. "
    "Focus on the subject matter, composition, lighting, and any notable "
    "features. Be concise but thorough (2-4 sentences)."
)

_EXTRACT_TEXT_PROMPT = (
    "What text do you see in this image? Read every word on any sign, "
    "label, screen, document, watermark, caption, chalkboard, poster, "
    "clothing, or other surface. Transcribe exactly as written."
)

_LLAVA_MODEL = "llava:7b"

# Ollama's /api/generate endpoint with stream=False returns the full response
# in one JSON object.  LLaVA inference on CPU can take 10-30 s per image so
# we allow a generous timeout.
_TIMEOUT_SECONDS = 60.0


# ── Public API ─────────────────────────────────────────────────────────────────


async def describe_image(
    image_bytes: bytes,
    ollama_base_url: str = "http://127.0.0.1:11434",
    model: str = _LLAVA_MODEL,
    timeout: float = _TIMEOUT_SECONDS,
) -> ImageDescribeResponse:
    """
    Generate a natural-language description of ``image_bytes`` using LLaVA.

    Args:
        image_bytes:     Raw bytes of the image to describe.
        ollama_base_url: Base URL of the local Ollama instance (no trailing
                         slash).  Defaults to ``http://127.0.0.1:11434``.
        model:           Ollama model name.  Must be a multimodal (vision)
                         model.  Defaults to ``"llava:7b"``.
        timeout:         HTTP timeout in seconds.

    Returns:
        An :class:`ImageDescribeResponse`.  ``success`` is ``False`` when
        Ollama is unreachable or the model is unavailable; the caller should
        treat ``description=None`` as a graceful no-op.
    """
    # ── Availability check ─────────────────────────────────────────────────────
    if not await _check_ollama_available(ollama_base_url):
        logger.debug(
            "Ollama unavailable at %s — skipping image description", ollama_base_url
        )
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message="Ollama is not running. Start Ollama to enable AI image descriptions.",
        )

    if not await _check_model_available(ollama_base_url, model):
        logger.info(
            "LLaVA model '%s' not found in Ollama — skipping image description", model
        )
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=(
                f"Model '{model}' is not available in Ollama. "
                f"Pull it with: ollama pull {model}"
            ),
        )

    # ── Inference ──────────────────────────────────────────────────────────────
    image_b64 = base64.b64encode(image_bytes).decode("ascii")

    payload = {
        "model": model,
        "prompt": _DESCRIBE_PROMPT,
        "images": [image_b64],
        "stream": False,
        "options": {
            "temperature": 0.2,
            "num_predict": 256,
        },
    }

    try:
        async with httpx.AsyncClient(timeout=timeout) as client:
            resp = await client.post(
                f"{ollama_base_url}/api/generate",
                json=payload,
            )
            resp.raise_for_status()
            data = resp.json()
            description = data.get("response", "").strip()

        if not description:
            return ImageDescribeResponse(
                description=None,
                model_used=model,
                success=False,
                message="Ollama returned an empty description.",
            )

        logger.info(
            "Image description generated (%d chars) via %s", len(description), model
        )
        return ImageDescribeResponse(
            description=description,
            model_used=model,
            success=True,
            message="OK",
        )

    except httpx.TimeoutException:
        logger.warning(
            "Ollama timed out while generating image description (model=%s)", model
        )
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=f"Ollama timed out after {timeout:.0f} s. Try a smaller model or increase timeout.",
        )
    except httpx.HTTPStatusError as exc:
        logger.warning("Ollama HTTP error: %s", exc)
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=f"Ollama returned HTTP {exc.response.status_code}.",
        )
    except Exception as exc:  # noqa: BLE001
        logger.warning("Unexpected error calling Ollama: %s", exc)
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=f"Image description failed: {exc}",
        )


async def extract_text_from_image(
    image_bytes: bytes,
    ollama_base_url: str = "http://127.0.0.1:11434",
    model: str = _LLAVA_MODEL,
    timeout: float = _TIMEOUT_SECONDS,
) -> ImageDescribeResponse:
    """
    Extract and transcribe all visible text from ``image_bytes`` using LLaVA.

    Suitable for screenshots, memes, social media posts, and document images
    where the primary goal is reading the text content rather than describing
    the image visually.

    Args:
        image_bytes:     Raw bytes of the image to read text from.
        ollama_base_url: Base URL of the local Ollama instance (no trailing
                         slash).  Defaults to ``http://127.0.0.1:11434``.
        model:           Ollama model name.  Must be a multimodal (vision)
                         model.  Defaults to ``"llava:7b"``.
        timeout:         HTTP timeout in seconds.

    Returns:
        An :class:`ImageDescribeResponse`.  The ``description`` field contains
        the transcribed text.  ``success`` is ``False`` when Ollama is
        unreachable or the model is unavailable.
    """
    # ── Availability check ─────────────────────────────────────────────────────
    if not await _check_ollama_available(ollama_base_url):
        logger.debug(
            "Ollama unavailable at %s — skipping text extraction", ollama_base_url
        )
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message="Ollama is not running. Start Ollama to enable text extraction.",
        )

    if not await _check_model_available(ollama_base_url, model):
        logger.info(
            "LLaVA model '%s' not found in Ollama — skipping text extraction", model
        )
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=(
                f"Model '{model}' is not available in Ollama. "
                f"Pull it with: ollama pull {model}"
            ),
        )

    # ── Inference ──────────────────────────────────────────────────────────────
    image_b64 = base64.b64encode(image_bytes).decode("ascii")

    payload = {
        "model": model,
        "prompt": _EXTRACT_TEXT_PROMPT,
        "images": [image_b64],
        "stream": False,
        "options": {
            # Lower temperature for more faithful transcription
            "temperature": 0.1,
            # Allow more tokens for longer text passages
            "num_predict": 512,
        },
    }

    try:
        async with httpx.AsyncClient(timeout=timeout) as client:
            resp = await client.post(
                f"{ollama_base_url}/api/generate",
                json=payload,
            )
            resp.raise_for_status()
            data = resp.json()
            extracted = data.get("response", "").strip()

        if not extracted:
            return ImageDescribeResponse(
                description=None,
                model_used=model,
                success=False,
                message="Ollama returned an empty response.",
            )

        logger.info(
            "Text extraction completed (%d chars) via %s", len(extracted), model
        )
        return ImageDescribeResponse(
            description=extracted,
            model_used=model,
            success=True,
            message="OK",
        )

    except httpx.TimeoutException:
        logger.warning("Ollama timed out during text extraction (model=%s)", model)
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=f"Ollama timed out after {timeout:.0f} s. Try a smaller model or increase timeout.",
        )
    except httpx.HTTPStatusError as exc:
        logger.warning("Ollama HTTP error during text extraction: %s", exc)
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=f"Ollama returned HTTP {exc.response.status_code}.",
        )
    except Exception as exc:  # noqa: BLE001
        logger.warning("Unexpected error during text extraction: %s", exc)
        return ImageDescribeResponse(
            description=None,
            model_used=model,
            success=False,
            message=f"Text extraction failed: {exc}",
        )


# ── Availability helpers ───────────────────────────────────────────────────────


async def _check_ollama_available(ollama_base_url: str) -> bool:
    """Return True if the Ollama /api/tags endpoint responds within 2 seconds."""
    try:
        async with httpx.AsyncClient(timeout=2.0) as client:
            resp = await client.get(f"{ollama_base_url}/api/tags")
            return resp.status_code == 200
    except Exception:
        return False


async def _check_model_available(ollama_base_url: str, model: str) -> bool:
    """Return True if *model* (or its base name) appears in Ollama's model list."""
    try:
        async with httpx.AsyncClient(timeout=2.0) as client:
            resp = await client.get(f"{ollama_base_url}/api/tags")
            if resp.status_code != 200:
                return False
            data = resp.json()
            models = data.get("models", [])
            base_model = model.split(":")[0]
            return any(m.get("name", "").split(":")[0] == base_model for m in models)
    except Exception:
        return False
