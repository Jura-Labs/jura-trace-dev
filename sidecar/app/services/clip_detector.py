"""
Jura Archive Sidecar — CLIP-based AI Image Detection.

Uses CLIP ViT-B/32 embeddings with zero-shot classification to detect
AI-generated images. Compares image embedding against text prompts
describing real photographs vs AI-generated content.

**CALIBRATION STATUS (March 2026):**
Zero-shot classification with the generic LAION-trained model produces
near-uniform probabilities (~20% per class) — it does NOT reliably
discriminate between real photos and AI-generated images. The CLIP
infrastructure is sound, but a trained UnivFD linear probe (6KB weights
on top of CLIP features, trained on a real-vs-AI dataset) is needed
for production-grade detection.

The endpoint is available as an EXPERIMENTAL feature. Results are
informational and should NOT be used as a primary detection signal
until a trained probe is integrated.

The model downloads on first use (~350MB) and is cached locally.
If ``open_clip`` is not installed, the service gracefully degrades
and returns a response with ``model_available=False``.
"""

import io
import logging

from PIL import Image

from app.models.schemas import ClipDetectionResponse

logger = logging.getLogger(__name__)

# Singleton model state — loaded once, reused across requests.
_model = None
_preprocess = None
_tokenizer = None
_model_load_attempted = False

# Text prompts for zero-shot classification.
# Prompts 0-1 describe real photographs; 2-4 describe AI/synthetic content.
_TEXT_PROMPTS = [
    "a photograph taken by a camera",
    "a real photograph of a real scene",
    "an AI-generated image",
    "a synthetic image created by artificial intelligence",
    "a digitally manipulated photograph",
]

# Verdict thresholds
_SYNTHETIC_THRESHOLD = 0.60
_AUTHENTIC_THRESHOLD = 0.35


def _ensure_model() -> bool:
    """Lazy-load the CLIP model. Returns True if model is ready."""
    global _model, _preprocess, _tokenizer, _model_load_attempted

    if _model is not None:
        return True

    if _model_load_attempted:
        # Already tried and failed — don't retry every request.
        return False

    _model_load_attempted = True

    try:
        import open_clip
        import torch  # noqa: F401 — needed at runtime by open_clip

        logger.info("Loading CLIP ViT-B-32 model (first use may download ~350MB)...")
        _model, _, _preprocess = open_clip.create_model_and_transforms(
            "ViT-B-32", pretrained="laion2b_s34b_b79k"
        )
        _model.eval()
        _tokenizer = open_clip.get_tokenizer("ViT-B-32")
        logger.info("CLIP model loaded successfully.")
        return True
    except ImportError:
        logger.warning(
            "open_clip or torch not installed — CLIP detection unavailable. "
            "Install with: pip install open-clip-torch"
        )
        return False
    except Exception:
        logger.exception("Failed to load CLIP model")
        return False


def _classify_zero_shot(image: Image.Image) -> tuple[float, dict[str, float]]:
    """Compare image embedding to text prompts via CLIP zero-shot.

    Returns:
        (ai_probability, class_probabilities) where ai_probability is a
        float in [0, 1] and class_probabilities maps prompt labels to
        their softmax probabilities.
    """
    import torch

    image_input = _preprocess(image).unsqueeze(0)
    text_tokens = _tokenizer(_TEXT_PROMPTS)

    with torch.no_grad():
        image_features = _model.encode_image(image_input)
        text_features = _model.encode_text(text_tokens)

        image_features = image_features / image_features.norm(dim=-1, keepdim=True)
        text_features = text_features / text_features.norm(dim=-1, keepdim=True)

        similarity = (image_features @ text_features.T).squeeze(0)
        probs = similarity.softmax(dim=-1).numpy()

    # Prompts 0-1 are "real", 2-3 are "AI", 4 is "manipulated"
    ai_prob = float(probs[2] + probs[3])
    manip_prob = float(probs[4])

    # Manipulation is weighted at 0.5 — it could be either real-edited or AI
    combined_ai_score = ai_prob + manip_prob * 0.5

    class_probabilities = {
        "photograph": float(probs[0]),
        "real_scene": float(probs[1]),
        "ai_generated": float(probs[2]),
        "synthetic": float(probs[3]),
        "manipulated": float(probs[4]),
    }

    return combined_ai_score, class_probabilities


def _unavailable_response() -> ClipDetectionResponse:
    """Return a response indicating the CLIP model is not available."""
    return ClipDetectionResponse(
        score=0.0,
        suspicious=False,
        verdict_level="inconclusive",
        confidence="low",
        class_probabilities={},
        model_name="ViT-B-32 (laion2b_s34b_b79k)",
        model_available=False,
        summary="CLIP model not available — install open-clip-torch for AI image detection.",
    )


def perform_clip_detection(image_bytes: bytes) -> ClipDetectionResponse:
    """Detect AI-generated images using CLIP zero-shot classification.

    Args:
        image_bytes: Raw bytes of the input image.

    Returns:
        ClipDetectionResponse with score, verdict, and class probabilities.

    Raises:
        ValueError: If image cannot be decoded.
    """
    if not _ensure_model():
        return _unavailable_response()

    try:
        pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    ai_score, class_probs = _classify_zero_shot(pil_image)

    # Clamp score to [0, 1]
    score = max(0.0, min(1.0, ai_score))

    # Three-way verdict
    if score > _SYNTHETIC_THRESHOLD:
        verdict_level = "synthetic"
    elif score < _AUTHENTIC_THRESHOLD:
        verdict_level = "authentic"
    else:
        verdict_level = "inconclusive"

    # Confidence based on distance from thresholds
    if score > 0.75 or score < 0.20:
        confidence = "high"
    elif score > 0.55 or score < 0.30:
        confidence = "medium"
    else:
        confidence = "low"

    # Summary
    if verdict_level == "synthetic":
        summary = (
            f"CLIP classifies this image as likely AI-generated "
            f"(score={score:.2f}, ai_generated={class_probs.get('ai_generated', 0):.2f}, "
            f"synthetic={class_probs.get('synthetic', 0):.2f})"
        )
    elif verdict_level == "authentic":
        summary = (
            f"CLIP classifies this image as likely a real photograph "
            f"(score={score:.2f}, photograph={class_probs.get('photograph', 0):.2f}, "
            f"real_scene={class_probs.get('real_scene', 0):.2f})"
        )
    else:
        summary = (
            f"CLIP classification is inconclusive "
            f"(score={score:.2f}) — zero-shot classification has limited discriminative power. "
            f"A trained UnivFD linear probe is needed for reliable results."
        )

    return ClipDetectionResponse(
        score=round(score, 4),
        suspicious=score > 0.5,
        verdict_level=verdict_level,
        confidence=confidence,
        class_probabilities={k: round(v, 4) for k, v in class_probs.items()},
        model_name="ViT-B-32 (laion2b_s34b_b79k)",
        model_available=True,
        summary=summary,
    )
