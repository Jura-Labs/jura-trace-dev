"""
Jura Archive Sidecar — Forensics endpoints.
"""

from fastapi import APIRouter, File, HTTPException, Query, UploadFile

from app.config import settings
from app.models.schemas import (
    CaResponse,
    CopyMoveResponse,
    DeepfakeResponse,
    ElaResponse,
    JpegGhostResponse,
    NoiseAnalysisResponse,
    NprResponse,
)
from app.services.chromatic_aberration import perform_ca_analysis
from app.services.copy_move import perform_copy_move_detection
from app.services.deepfake import perform_deepfake_detection
from app.services.ela import perform_ela
from app.services.jpeg_ghost import perform_jpeg_ghost_detection
from app.services.noise_analysis import perform_noise_analysis
from app.services.npr import perform_npr_analysis

router = APIRouter()


async def _read_and_validate(file: UploadFile) -> bytes:
    """Read and validate an uploaded file."""
    image_bytes = await file.read()

    if len(image_bytes) == 0:
        raise HTTPException(status_code=400, detail="Empty file uploaded")

    if len(image_bytes) > settings.max_image_size:
        raise HTTPException(
            status_code=400,
            detail=f"File too large ({len(image_bytes)} bytes). Max: {settings.max_image_size}",
        )

    return image_bytes


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
    image_bytes = await _read_and_validate(file)

    try:
        return perform_ela(image_bytes, quality=quality)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/noise", response_model=NoiseAnalysisResponse)
async def analyse_noise(
    file: UploadFile = File(...),
    block_size: int = Query(default=32, ge=8, le=128),
) -> NoiseAnalysisResponse:
    """
    Perform block-wise noise variance analysis on an uploaded image.

    Returns a heatmap (base64 PNG), per-block variance data, and an
    anomaly score (0.0 = uniform, 1.0 = highly inconsistent).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_noise_analysis(image_bytes, block_size=block_size)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/copy-move", response_model=CopyMoveResponse)
async def detect_copy_move(
    file: UploadFile = File(...),
    max_features: int = Query(default=5000, ge=100, le=20000),
    min_distance: float = Query(default=40.0, ge=10.0, le=500.0),
) -> CopyMoveResponse:
    """
    Detect copy-move forgery in an uploaded image.

    Returns a visualisation (base64 PNG), detected clone regions,
    and a forgery score (0.0 = clean, 1.0 = significant cloning).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_copy_move_detection(
            image_bytes,
            max_features=max_features,
            min_distance=min_distance,
        )
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/deepfake", response_model=DeepfakeResponse)
async def detect_deepfake(
    file: UploadFile = File(...),
    mime_type: str = Query(default="image/jpeg"),
    has_camera_exif: bool = Query(default=False),
) -> DeepfakeResponse:
    """
    Detect AI-generated or synthetic content in an uploaded image.

    Returns a score (0.0 = authentic, 1.0 = synthetic), interpretable
    signals from the feature ensemble, and a frequency spectrum heatmap.

    The ``mime_type`` parameter enables codec-aware threshold selection
    so that modern lossy codecs (AVIF, WebP, HEIC) do not trigger false
    positives due to their aggressive in-loop filtering.

    The ``has_camera_exif`` parameter signals whether the image carries
    camera-origin EXIF data. Images with rich camera EXIF are less likely
    to be AI-generated; the scoring midpoint is shifted accordingly.
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_deepfake_detection(
            image_bytes, mime_type=mime_type, has_camera_exif=has_camera_exif,
        )
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/jpeg-ghost", response_model=JpegGhostResponse)
async def detect_jpeg_ghost(
    file: UploadFile = File(...),
) -> JpegGhostResponse:
    """
    Detect splice/composite forgeries via JPEG ghost analysis.

    Re-compresses the image at multiple quality levels and identifies
    blocks whose compression ghost appears at a different quality than
    the dominant level — indicating content spliced from a differently-
    compressed source.

    Returns a heatmap (base64 PNG), per-block ghost quality statistics,
    and a manipulation score (0.0 = uniform compression, 1.0 = strong
    evidence of splicing).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_jpeg_ghost_detection(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/npr", response_model=NprResponse)
async def analyse_npr(
    file: UploadFile = File(...),
) -> NprResponse:
    """
    Perform Neighbouring Pixel Relationship (NPR) analysis on an uploaded image.

    Detects AI-generated images by analysing statistical relationships between
    adjacent pixels.  Camera sensors produce characteristic inter-pixel
    correlations; AI generators produce subtly different NPR statistics.

    Returns a score (0.0 = authentic, 1.0 = synthetic), per-feature breakdown,
    and a pixel-difference magnitude heatmap (base64 PNG).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_npr_analysis(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/chromatic-aberration", response_model=CaResponse)
async def analyse_chromatic_aberration(
    file: UploadFile = File(...),
) -> CaResponse:
    """
    Analyse chromatic aberration consistency in an uploaded image.

    Real camera lenses produce radial chromatic aberration — channel shifts
    that grow linearly with distance from the image centre.  AI generators
    lack a physical lens model, so this radial pattern is absent or random.

    Returns an R\u00b2 value for the radial fit, a score (0.0 = consistent CA /
    likely real, 1.0 = absent CA / likely AI), and a suspicious flag.
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_ca_analysis(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
