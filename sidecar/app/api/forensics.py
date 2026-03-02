"""
Jura Archive Sidecar — Forensics endpoints.
"""

from fastapi import APIRouter, File, HTTPException, Query, UploadFile

from app.config import settings
from app.models.schemas import ElaResponse
from app.services.ela import perform_ela

router = APIRouter()


@router.post("/ela", response_model=ElaResponse)
async def analyse_ela(
    file: UploadFile = File(...),
    quality: int = Query(default=90, ge=1, le=100),
) -> ElaResponse:
    """
    Perform Error Level Analysis on an uploaded image.

    Returns a heatmap (base64 PNG), statistical measures, and a
    manipulation score (0.0 = clean, 1.0 = highly manipulated).
    """
    image_bytes = await file.read()

    if len(image_bytes) == 0:
        raise HTTPException(status_code=400, detail="Empty file uploaded")

    if len(image_bytes) > settings.max_image_size:
        raise HTTPException(
            status_code=400,
            detail=f"File too large ({len(image_bytes)} bytes). Max: {settings.max_image_size}",
        )

    try:
        return perform_ela(image_bytes, quality=quality)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
