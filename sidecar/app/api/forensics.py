"""
Jura Archive Sidecar — Forensics endpoints.
"""

from fastapi import APIRouter, File, HTTPException, Query, UploadFile

from app.config import settings
from app.models.schemas import (
    CaResponse,
    ClaimCheckResponse,
    ClipDetectionResponse,
    ColourTemperatureResponse,
    CopyMoveResponse,
    DeepfakeResponse,
    ElaResponse,
    JpegGhostResponse,
    NoiseAnalysisResponse,
    NprResponse,
    SegmentedElaResponse,
    ShadowConsistencyResponse,
    SpliceBoundaryResponse,
)
from app.services.chromatic_aberration import perform_ca_analysis
from app.services.claim_checker import check_claims as _check_claims
from app.services.clip_detector import perform_clip_detection
from app.services.colour_temperature import perform_colour_temperature
from app.services.copy_move import perform_copy_move_detection
from app.services.deepfake import perform_deepfake_detection
from app.services.ela import perform_ela
from app.services.jpeg_ghost import perform_jpeg_ghost_detection
from app.services.noise_analysis import perform_noise_analysis
from app.services.npr import perform_npr_analysis
from app.services.segmented_ela import perform_segmented_ela
from app.services.shadow_consistency import perform_shadow_consistency
from app.services.splice_boundary import perform_splice_boundary

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


@router.post("/segmented-ela", response_model=SegmentedElaResponse)
async def segmented_ela(
    file: UploadFile = File(...),
    quality: int = Query(default=90, ge=1, le=100),
) -> SegmentedElaResponse:
    """
    Perform Segmented Error Level Analysis on an uploaded image.

    Divides the image into an 8x8 grid, computes ELA per cell, and flags
    anomalous regions. Clusters of 3+ adjacent anomalous cells suggest
    composite manipulation.

    Returns a heatmap (base64 PNG), per-region scores, anomaly counts,
    and a manipulation score (0.0 = clean, 1.0 = highly manipulated).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_segmented_ela(image_bytes, quality=quality)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/shadow-consistency", response_model=ShadowConsistencyResponse)
async def shadow_consistency(
    file: UploadFile = File(...),
) -> ShadowConsistencyResponse:
    """
    Analyse shadow/lighting direction consistency in an uploaded image.

    Estimates dominant light direction across image regions using gradient
    analysis and flags regions with incompatible shadow directions.

    Returns a heatmap (base64 PNG), per-region light directions, and a
    score (0.0 = consistent, 1.0 = highly inconsistent).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_shadow_consistency(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/colour-temperature", response_model=ColourTemperatureResponse)
async def colour_temperature(
    file: UploadFile = File(...),
) -> ColourTemperatureResponse:
    """
    Analyse colour temperature consistency in an uploaded image.

    Segments the image into a 4x4 grid and checks for discontinuous colour
    casts in CIELAB space that suggest compositing from multiple sources.

    Returns a heatmap (base64 PNG), per-region colour data, anomaly counts,
    and a score (0.0 = uniform, 1.0 = significant mismatch).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_colour_temperature(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/splice-boundary", response_model=SpliceBoundaryResponse)
async def splice_boundary(
    file: UploadFile = File(...),
) -> SpliceBoundaryResponse:
    """
    Detect splice boundaries in an uploaded image using three signals:
    JPEG block grid alignment, noise asymmetry, and feathering profile.

    An edge needs 2 of 3 signals to be classified as a splice candidate.

    Returns a heatmap (base64 PNG), detected boundaries, and a score
    (0.0 = no splices, 1.0 = strong evidence of compositing).
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_splice_boundary(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/clip-detect", response_model=ClipDetectionResponse)
async def detect_clip(
    file: UploadFile = File(...),
) -> ClipDetectionResponse:
    """
    Detect AI-generated images using CLIP ViT-B/32 zero-shot classification.

    Compares the image embedding against text prompts describing real
    photographs vs AI-generated content. Returns a score (0.0 = authentic,
    1.0 = synthetic), class probabilities, and a three-way verdict.

    If the CLIP model is not installed, returns a response with
    ``model_available=False`` and score 0.0.
    """
    image_bytes = await _read_and_validate(file)

    try:
        return perform_clip_detection(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/claim-check", response_model=ClaimCheckResponse)
async def claim_check(
    claims_text: str = Query(..., description="Text containing claims to verify"),
    context: str = Query(default="", description="Optional context (EXIF description, C2PA assertions, etc.)"),
) -> ClaimCheckResponse:
    """
    Verify claims associated with an image using a local Ollama LLM.

    Accepts free-form text containing one or more claims (separated by newlines
    or sentence boundaries) and optional supporting context such as EXIF
    descriptions or C2PA assertion data.

    Returns a structured verdict for each claim
    (supported / disputed / unverified) plus an aggregated overall verdict.

    Requires Ollama to be running locally with a compatible model pulled.
    If Ollama is unavailable, returns ``overall_verdict="unavailable"`` —
    the endpoint always responds with HTTP 200 so the frontend can display
    a graceful degradation message.

    This is AI-assisted analysis. Results are indicative, not conclusive.
    """
    return await _check_claims(
        claims_text=claims_text,
        context=context,
        ollama_base_url=settings.ollama_base_url,
        model=settings.llm_model,
    )
