"""
Jura Trace Sidecar — Forensics endpoints.
"""

from fastapi import APIRouter, File, HTTPException, Query, UploadFile

from app.config import settings
from app.models.schemas import (
    AudioMetadataResponse,
    CaResponse,
    ClaimCheckResponse,
    ClipDetectionResponse,
    ColourTemperatureResponse,
    CopyMoveResponse,
    DeepfakeResponse,
    ElaResponse,
    ImageDescribeResponse,
    JpegGhostResponse,
    NoiseAnalysisResponse,
    NprResponse,
    SegmentedElaResponse,
    ShadowConsistencyResponse,
    SpliceBoundaryResponse,
    TranscriptionResponse,
    VideoDeepfakeResponse,
    VideoFramesResponse,
    VideoMetadataResponse,
    WatermarkEmbedResponse,
    WatermarkExtractResponse,
)
from app.services.chromatic_aberration import perform_ca_analysis
from app.services.describe_image import describe_image as _describe_image
from app.services.describe_image import extract_text_from_image as _extract_text_from_image
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
from app.services.noise_visualisation import perform_noise_visualisation
from app.services.clahe import perform_clahe
from app.services.audio_metadata import perform_audio_metadata
from app.services.transcription import perform_transcription
from app.services.video_deepfake import perform_video_deepfake_analysis
from app.services.video_frames import perform_frame_extraction
from app.services.video_metadata import perform_video_metadata
from app.services.watermark import perform_watermark_embed, perform_watermark_extract

router = APIRouter()


def _has_camera_exif(image_bytes: bytes) -> bool:
    """Check if image has camera-origin EXIF metadata.

    Only returns True when genuine camera make/model tags are present.
    This is a conservative check — it avoids false positives from images
    that have been processed through CDNs or social media platforms which
    strip camera EXIF.
    """
    try:
        from PIL import Image as _Image
        from io import BytesIO

        img = _Image.open(BytesIO(image_bytes))
        exif = img.getexif()
        if not exif:
            return False
        # Check for camera-specific EXIF tags
        MAKE = 0x010F   # Camera manufacturer
        MODEL = 0x0110  # Camera model
        return MAKE in exif or MODEL in exif
    except Exception:
        return False


async def _read_and_validate(file: UploadFile) -> bytes:
    """Read and validate an uploaded image file."""
    image_bytes = await file.read()

    if len(image_bytes) == 0:
        raise HTTPException(status_code=400, detail="Empty file uploaded")

    if len(image_bytes) > settings.max_image_size:
        raise HTTPException(
            status_code=400,
            detail=f"File too large ({len(image_bytes)} bytes). Max: {settings.max_image_size}",
        )

    return image_bytes


# Maximum sizes for media file uploads.
# Video/audio files are larger than images by design, but still need a hard
# upper limit to prevent the sidecar process from being OOM-killed by a
# client submitting a multi-gigabyte file via the /video/* or /audio/*
# endpoints, which historically bypassed the image-specific _read_and_validate
# helper.
_MAX_VIDEO_SIZE: int = 500 * 1024 * 1024   # 500 MB
_MAX_AUDIO_SIZE: int = 100 * 1024 * 1024   # 100 MB


async def _read_media(file: UploadFile, max_size: int, media_label: str) -> bytes:
    """Read and size-validate an uploaded video or audio file.

    Args:
        file:        The uploaded file from FastAPI.
        max_size:    Maximum accepted byte count.
        media_label: Human-readable label used in error messages (e.g. "video").

    Returns:
        Raw bytes of the file.

    Raises:
        HTTPException 400 if the file is empty.
        HTTPException 413 if the file exceeds *max_size*.
    """
    contents = await file.read()
    if len(contents) == 0:
        raise HTTPException(status_code=400, detail="Empty file uploaded")
    if len(contents) > max_size:
        raise HTTPException(
            status_code=413,
            detail=(
                f"{media_label.capitalize()} file too large "
                f"({len(contents):,} bytes). "
                f"Maximum accepted size is {max_size // (1024 * 1024)} MB."
            ),
        )
    return contents


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
    has_camera_exif: bool | None = Query(default=None),
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
    If not explicitly set, camera EXIF is auto-detected from the image.
    """
    image_bytes = await _read_and_validate(file)

    # Auto-detect mime type from file content if not specified or default
    if mime_type == "image/jpeg":
        if image_bytes[:4] == b'\x89PNG':
            mime_type = "image/png"
        elif image_bytes[:4] == b'RIFF' and image_bytes[8:12] == b'WEBP':
            mime_type = "image/webp"
        elif file.content_type and file.content_type != "application/octet-stream":
            mime_type = file.content_type

    # Auto-detect camera EXIF if not explicitly provided
    if has_camera_exif is None:
        has_camera_exif = _has_camera_exif(image_bytes)

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


@router.post("/describe", response_model=ImageDescribeResponse)
async def describe_image(
    file: UploadFile = File(...),
) -> ImageDescribeResponse:
    """
    Generate a natural-language description of an uploaded image using LLaVA.

    Sends the image to a local Ollama instance running the ``llava:7b``
    multimodal model.  The description is a 2-4 sentence summary of the
    image content, composition, and notable features.

    This endpoint always returns HTTP 200.  When Ollama is unavailable or
    the LLaVA model is not pulled, ``success`` is ``False`` and
    ``description`` is ``null`` — the caller should degrade gracefully.

    Requires Ollama to be running locally with ``llava:7b`` pulled:
        ollama pull llava:7b
    """
    image_bytes = await _read_and_validate(file)

    return await _describe_image(
        image_bytes=image_bytes,
        ollama_base_url=settings.ollama_base_url,
        model="llava:7b",
    )


@router.post("/extract-text", response_model=ImageDescribeResponse)
async def extract_text(
    file: UploadFile = File(...),
) -> ImageDescribeResponse:
    """
    Extract and transcribe all visible text from an uploaded image using LLaVA.

    Sends the image to a local Ollama instance running the ``llava:7b``
    multimodal model with a text-extraction-specific prompt.  Suitable for
    screenshots, memes, social media posts, and scanned document images.

    The response reuses the :class:`ImageDescribeResponse` schema: the
    ``description`` field contains the transcribed text.

    This endpoint always returns HTTP 200.  When Ollama is unavailable or
    the LLaVA model is not pulled, ``success`` is ``False`` and
    ``description`` is ``null`` — the caller should degrade gracefully.

    Requires Ollama to be running locally with ``llava:7b`` pulled:
        ollama pull llava:7b
    """
    image_bytes = await _read_and_validate(file)

    return await _extract_text_from_image(
        image_bytes=image_bytes,
        ollama_base_url=settings.ollama_base_url,
        model="llava:7b",
    )


@router.post("/watermark/embed", response_model=WatermarkEmbedResponse)
async def watermark_embed(
    file: UploadFile = File(...),
    payload: str = Query(default="JuraTrace"),
    strength: str = Query(default="medium"),
) -> WatermarkEmbedResponse:
    """
    Embed an invisible DWT-DCT-SVD watermark into an uploaded image.

    Encodes the given payload string into the frequency domain of the image.
    The watermarked image is returned as a base64-encoded PNG (lossless to
    preserve the watermark). Minimum image size is 256x256.

    The payload is truncated to 64 bytes if longer. Strength parameter
    accepts "low", "medium", or "high".
    """
    image_bytes = await _read_and_validate(file)

    result = perform_watermark_embed(image_bytes, payload, strength)
    return result


@router.post("/watermark/extract", response_model=WatermarkExtractResponse)
async def watermark_extract(
    file: UploadFile = File(...),
    payload_length: int = Query(default=64, ge=1, le=256),
) -> WatermarkExtractResponse:
    """
    Extract an invisible DWT-DCT-SVD watermark from an uploaded image.

    Attempts to decode an embedded payload of the specified byte length.
    The payload_length must match the length used during embedding for
    accurate extraction.

    Returns the extracted payload (if found), a hex representation of the
    raw bytes, and a confidence indicator.
    """
    image_bytes = await _read_and_validate(file)

    result = perform_watermark_extract(image_bytes, payload_length)
    return result


@router.post("/video/metadata", response_model=VideoMetadataResponse)
async def video_metadata(
    file: UploadFile = File(...),
) -> VideoMetadataResponse:
    """
    Extract video metadata using FFmpeg/ffprobe.

    Returns codec, resolution, frame rate, duration, audio stream info,
    bitrate, and file size. Requires FFmpeg to be installed on the system.
    """
    contents = await _read_media(file, _MAX_VIDEO_SIZE, "video")
    return perform_video_metadata(contents)


@router.post("/audio/metadata", response_model=AudioMetadataResponse)
async def audio_metadata(
    file: UploadFile = File(...),
) -> AudioMetadataResponse:
    """
    Extract audio metadata using FFmpeg/ffprobe.

    Returns codec, sample rate, channels, duration, bitrate, and file size.
    Requires FFmpeg to be installed on the system.
    """
    contents = await _read_media(file, _MAX_AUDIO_SIZE, "audio")
    return perform_audio_metadata(contents)


@router.post("/video/deepfake", response_model=VideoDeepfakeResponse)
async def analyse_video_deepfake(
    file: UploadFile = File(...),
    mode: str = Query(default="standard"),
) -> VideoDeepfakeResponse:
    """
    Analyse a video for AI-generated or manipulated frames.

    Extracts evenly-spaced frames and runs the deepfake detector on each.
    Per-frame scores are aggregated into a video-level verdict.

    Modes: ``standard`` (6 frames), ``deep`` (20), ``archival`` (40).
    Requires FFmpeg to be installed on the system.
    """
    if mode not in ("standard", "deep", "archival"):
        raise HTTPException(
            status_code=400,
            detail=f"Invalid mode '{mode}'. Must be one of: standard, deep, archival",
        )

    contents = await _read_media(file, _MAX_VIDEO_SIZE, "video")
    return perform_video_deepfake_analysis(contents, mode=mode)


@router.post("/video/frames", response_model=VideoFramesResponse)
async def video_frames(
    file: UploadFile = File(...),
    count: int = Query(default=6, ge=1, le=12),
) -> VideoFramesResponse:
    """
    Extract evenly-spaced frames from a video as base64 JPEG strings.

    Returns up to ``count`` frames sampled at equal intervals through the
    video duration. Requires FFmpeg to be installed on the system.
    """
    contents = await _read_media(file, _MAX_VIDEO_SIZE, "video")
    return perform_frame_extraction(contents, count=count)


@router.post("/transcribe", response_model=TranscriptionResponse)
async def transcribe(
    file: UploadFile = File(...),
    language: str | None = Query(default=None),
    model_size: str = Query(default="base"),
) -> TranscriptionResponse:
    """
    Transcribe speech from an audio or video file using Whisper.

    Accepts audio (WAV, MP3, FLAC) and video (MP4, MOV) files.
    For video files, the audio track is extracted via FFmpeg first.
    Uses faster-whisper for efficient CPU inference.

    The ``language`` parameter accepts an ISO 639-1 code (e.g. "en").
    Leave unset for automatic language detection.

    Model sizes: ``tiny`` (~75 MB), ``base`` (~150 MB), ``small`` (~500 MB).
    Models are downloaded on first use.

    Requires the optional ``faster-whisper`` package.
    If not installed, returns ``success=False`` with a descriptive message.

    File size is capped at the video limit (500 MB) since video files are the
    largest accepted media type.  This prevents OOM via crafted large uploads.
    """
    # SECURITY: Apply the video size limit because transcription accepts both
    # audio and video; without a limit an attacker could submit an arbitrarily
    # large file and exhaust process memory before any processing begins.
    contents = await _read_media(file, _MAX_VIDEO_SIZE, "media")

    if model_size not in ("tiny", "base", "small"):
        raise HTTPException(
            status_code=400,
            detail=f"Invalid model_size '{model_size}'. Must be one of: tiny, base, small",
        )

    result = perform_transcription(
        contents, language=language, model_size=model_size,
    )
    return TranscriptionResponse(**result)


@router.post("/noise-visualisation")
async def noise_visualisation(
    file: UploadFile = File(...),
):
    """Return noise residual and variance heatmap visualisations."""
    image_bytes = await _read_and_validate(file)

    try:
        return perform_noise_visualisation(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/clahe")
async def clahe_enhance(
    file: UploadFile = File(...),
    clip_limit: float = Query(default=2.0, ge=0.5, le=10.0),
):
    """Return CLAHE-enhanced image with configurable clip limit."""
    image_bytes = await _read_and_validate(file)

    try:
        return perform_clahe(image_bytes, clip_limit=clip_limit)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
