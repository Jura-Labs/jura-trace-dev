# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Forensics endpoints.
"""

import time

from fastapi import APIRouter, File, HTTPException, Query, UploadFile

from app.config import settings
from app.models.schemas import (
    ClaimCheckResponse,
    ClipDetectionResponse,
    ColourTemperatureResponse,
    CopyMoveResponse,
    DctAnalysisResponse,
    DeepfakeResponse,
    ElaResponse,
    FourierAnalysisResponse,
    ImageDescribeResponse,
    JpegGhostResponse,
    NoiseAnalysisResponse,
    NprResponse,
    PlatformFingerprintResponse,
    SegmentedElaResponse,
    ShadowConsistencyResponse,
    SpliceBoundaryResponse,
    WatermarkEmbedResponse,
    WatermarkExtractResponse,
)
from app.services.describe_image import describe_image as _describe_image
from app.services.describe_image import extract_text_from_image as _extract_text_from_image
from app.services.claim_checker import check_claims as _check_claims
from app.services.clip_detector import (
    get_last_used_ts,
    perform_clip_detection,
    unload_clip_model,
)
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
from app.services.frequency_visualisation import perform_frequency_visualisation
from app.services.jpeg_grid import perform_jpeg_grid_visualisation
# JTV-138 (2026-05-02): audio/video routes and their service imports were
# removed for v1.0 (audio_metadata, transcription, video_deepfake,
# video_frames, video_metadata, audio_deepfake, enf_analysis). The Rust
# pipeline gates ENABLE_VIDEO_DEEPFAKE_GROUP and ENABLE_AUDIO_GROUP to
# false, and removing the routes closes a reachable-but-ungated HTTP
# surface on the local sidecar port. JTV-139 restores everything by
# reverting the pre-launch cleanup commit (2026-06-11) in one PR.
from app.services.watermark import perform_watermark_embed, perform_watermark_extract
from app.services.dct_analysis import analyse_dct
from app.services.fourier_analysis import analyse_fourier
from app.services.platform_fingerprint import analyse_platform

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
    camera_authenticity_bonus: float = Query(default=0.0, ge=0.0, le=1.0),
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

    The ``camera_authenticity_bonus`` (0.0–1.0) is the MakerNote-derived
    confidence that the file came from a real camera. Sprint 29 Track 1:
    when a vendor-recognised MakerNote is present, the deepfake score is
    suppressed proportionally to mitigate false positives on computational
    photography output (Pixel HDR+, iPhone Deep Fusion, drone ISPs).
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
            image_bytes,
            mime_type=mime_type,
            has_camera_exif=has_camera_exif,
            camera_authenticity_bonus=camera_authenticity_bonus,
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


# JTV-138 (2026-05-02): five video/audio routes lived here (video/metadata,
# audio/metadata, video/deepfake, video/frames, transcribe). Removed for
# v1.0 to close a reachable-but-ungated HTTP surface on the local sidecar
# port; the Rust pipeline gates ENABLE_VIDEO_DEEPFAKE_GROUP and
# ENABLE_AUDIO_GROUP to false so nothing in the v1.0 verify path calls
# them. JTV-139 restores by reverting the pre-launch cleanup commit
# (2026-06-11).


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


@router.post("/frequency-visualisation")
async def frequency_visualisation(
    file: UploadFile = File(...),
):
    """Return 2D FFT magnitude spectrum and DCT block heatmap."""
    image_bytes = await _read_and_validate(file)

    try:
        return perform_frequency_visualisation(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


@router.post("/jpeg-grid")
async def jpeg_grid_visualisation(
    file: UploadFile = File(...),
):
    """Return JPEG 8x8 block boundary artefact heatmap and Q-table."""
    image_bytes = await _read_and_validate(file)

    try:
        return perform_jpeg_grid_visualisation(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc


# Pre-launch cleanup (2026-06-11): the roi-analysis and gan-fingerprint
# routes were removed. Neither had a Rust caller or any UI rendering, so
# they were reachable-but-unused HTTP surface (same rationale as the
# JTV-138 audio/video route removal). The services and their unit tests
# remain in app/services/ pending a post-launch promote-or-remove
# decision; restore the routes by reverting this commit.


# JTV-138 (2026-05-02): the audio/deepfake route lived here. It was a
# Sprint 35 skeleton with no deployed model; JTV-110 / JTV-113 v1.1
# retraining on ASVspoof + WaveFake will restore it (revert the
# pre-launch cleanup commit, 2026-06-11).


@router.post("/dct-analysis", response_model=DctAnalysisResponse)
async def dct_analysis(file: UploadFile = File(...)) -> DctAnalysisResponse:
    """Compute 8x8 block DCT statistics to detect mixed compression levels.

    Returns a heatmap of AC energy distribution across all 8x8 DCT blocks
    and a coefficient of variation score. A high CV indicates that different
    image regions were compressed at different quality levels — a strong
    indicator of splice or composite forgery.

    Only runs in deep verification mode.
    Accepted formats: JPEG, PNG, WebP, TIFF, BMP, GIF.
    """
    image_bytes = await _read_and_validate(file)

    try:
        result = analyse_dct(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return DctAnalysisResponse(
        heatmap_base64=result["heatmapBase64"],
        dc_std=result["dcStd"],
        ac_mean=result["acMean"],
        ac_std=result["acStd"],
        ac_coefficient_of_variation=result["acCoefficientOfVariation"],
        suspicious=result["suspicious"],
        score=result["score"],
        summary=result["summary"],
    )


@router.post("/fourier-analysis", response_model=FourierAnalysisResponse)
async def fourier_analysis(file: UploadFile = File(...)) -> FourierAnalysisResponse:
    """Detect periodic patterns via 2D FFT magnitude spectrum analysis.

    Returns the log-magnitude spectrum as a PNG heatmap and a count of
    spectral peaks above a 3-sigma threshold. Discrete peaks away from
    the DC component indicate periodic artefacts from GAN upsampling,
    screen recapture (moire), or resampling during compositing.

    Only runs in deep verification mode.
    Accepted formats: JPEG, PNG, WebP, TIFF, BMP, GIF.
    """
    image_bytes = await _read_and_validate(file)

    try:
        result = analyse_fourier(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return FourierAnalysisResponse(
        spectrum_base64=result["spectrumBase64"],
        peak_count=result["peakCount"],
        suspicious=result["suspicious"],
        score=result["score"],
        summary=result["summary"],
    )


# ── Platform fingerprinting ──────────────────────────────────────────────────


@router.post("/platform-fingerprint", response_model=PlatformFingerprintResponse)
async def platform_fingerprint(
    file: UploadFile = File(...),
) -> PlatformFingerprintResponse:
    """Identify which social media platform processed an uploaded image.

    Analyses JPEG compression quality, maximum dimension, EXIF stripping,
    and resolution patterns to match against known platform signatures
    (WhatsApp, Telegram, Facebook, Instagram, Twitter/X, Signal, WeChat).

    Returns ranked candidates with confidence scores. Useful for
    establishing an image's distribution chain — e.g. confirming that a
    photo was forwarded via WhatsApp before being submitted for verification.

    Accepted formats: JPEG (strongest signals), PNG, WebP (limited matching).
    """
    image_bytes = await _read_and_validate(file)

    try:
        result = analyse_platform(image_bytes)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    return PlatformFingerprintResponse(**result)


# JTV-138 (2026-05-02): the enf-analysis route (mains-hum ENF on WAV)
# lived here. Audio-only, out of v1.0 scope; restored alongside audio
# deepfake under JTV-110/JTV-113 (revert the pre-launch cleanup commit,
# 2026-06-11).


# ── CLIP model lifecycle management ──────────────────────────────────────────


@router.post("/unload-clip")
async def unload_clip():
    """Release the CLIP ViT-B/32 model from memory to reclaim RAM.

    The model (~600-700 MB) is dropped immediately; the next CLIP
    detection request will re-load it from the local open_clip cache.

    Called by the Rust idle-watcher after 10 minutes of no CLIP usage.
    """
    return unload_clip_model()


@router.get("/clip-status")
async def clip_status():
    """Return CLIP model load state and idle time for diagnostic purposes.

    The Rust idle-watcher polls this endpoint every 60 s and triggers
    unload when ``idle_seconds`` exceeds the configured threshold.
    """
    import app.services.clip_detector as _mod

    # JTV-143 (3 May 2026): renamed from `_model` to `_vision_session` after
    # the ONNX backend swap. The vision session is the one the Rust idle
    # watcher cares about (memory-resident inference state).
    loaded = _mod._vision_session is not None
    last_used = get_last_used_ts()
    idle_seconds = (time.time() - last_used) if last_used > 0.0 else -1.0
    return {"loaded": loaded, "last_used_ts": last_used, "idle_seconds": idle_seconds}
