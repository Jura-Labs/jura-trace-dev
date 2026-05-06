# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Content-type classification endpoint.

Exposes the heuristic content-type classifier at `/forensics/content-type`.
The verify pipeline calls this endpoint before dispatching AI-detection
models so it can suppress them on screenshots, documents, and unknown
content where the ML detectors are known to produce unreliable verdicts
(see `docs/screenshot-detection-findings.md`, 2026-03-28).
"""

from fastapi import APIRouter, File, HTTPException, UploadFile

from app.config import settings
from app.models.content_type import ContentTypeResult
from app.services.content_type import classify_content

router = APIRouter()


@router.post("/content-type", response_model=ContentTypeResult)
async def classify_content_type(
    file: UploadFile = File(...),
) -> ContentTypeResult:
    """
    Classify an uploaded image as photograph / screenshot / document /
    artwork / unknown using fast heuristics (<500 ms, no ML load).

    Intended to be called by the Rust verify pipeline before the AI-detection
    stage so that non-photographic content can be gated away from UnivFD and
    the GBM deepfake classifier.
    """
    image_bytes = await file.read()
    if len(image_bytes) == 0:
        raise HTTPException(status_code=400, detail="Empty file uploaded")
    if len(image_bytes) > settings.max_image_size:
        raise HTTPException(
            status_code=400,
            detail=(
                f"File too large ({len(image_bytes)} bytes). "
                f"Max: {settings.max_image_size}"
            ),
        )
    return classify_content(image_bytes)
