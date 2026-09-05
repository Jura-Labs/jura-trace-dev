# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — CLIP-based AI Image Detection.

Uses CLIP ViT-B/32 embeddings (via ONNX runtime) with zero-shot
classification AND a trained UnivFD LogReg probe to detect AI-generated
images. The probe (6 KB joblib, AUC 0.9933, FP 4.12%, recall 95.70%)
is the load-bearing detection signal; zero-shot text similarity is the
auxiliary readout shown as the 5-bar class breakdown in Expert View.

**JTV-143 (3 May 2026): switched from PyTorch + open_clip to ONNX runtime.**
The PyInstaller bundle no longer ships PyTorch (~700 MB) or open_clip; the
ONNX vision + text encoders (FP32, ~579 MB combined) and the standalone
BPE tokeniser (`clip_tokenizer.py`) replace them. The trained UnivFD probe
operates against the ONNX-derived embeddings — perfect cosine match
validated on the corpus (mean cosine 1.000000, max drift 0.000163;
see docs/calibration/univfd-v9-onnx-validation.md).

If ``onnxruntime`` is not installed or the ONNX model files are absent
under JURA_MODELS_DIR, the service gracefully degrades and returns a
response with ``model_available=False``.
"""

import gc
import hashlib
import io
import logging
import os
import time
from typing import Optional

import numpy as np
from PIL import Image

from app.models.schemas import ClipDetectionResponse, VerdictThresholds

logger = logging.getLogger(__name__)

# Singleton ONNX session state — loaded once, reused across requests.
# JTV-143: `_vision_session` / `_text_session` are onnxruntime.InferenceSession
# instances; `_text_prompt_cache` holds pre-encoded prompt embeddings so the
# (relatively expensive) tokenise + text-encode round only runs once per
# process lifetime. `_model_load_attempted` retains its prior semantics —
# don't retry every request after a clean failure.
_vision_session = None
_text_session = None
_text_prompt_cache: Optional[np.ndarray] = None
_model_load_attempted = False

# UnivFD probe state — lazy-loaded from models/univfd_probe.joblib
_univfd_probe = None
_univfd_probe_loaded = False

# Last-used timestamp — updated every time the model is actually invoked.
# Read by the /forensics/clip-status endpoint so the Rust idle-watcher
# can decide when to call /forensics/unload-clip.
_last_used_ts: float = 0.0

_UNIVFD_PROBE_SHA256 = (
    "0534a9e80e352a5bd8af5fc447d03e37be2e1aa68a05d81f05736d6ef8956a86"
)

# Text prompts for zero-shot classification.
# Prompts 0-1 describe real photographs; 2-4 describe AI/synthetic content.
_TEXT_PROMPTS = [
    "a photograph taken by a camera",
    "a real photograph of a real scene",
    "an AI-generated image",
    "a synthetic image created by artificial intelligence",
    "a digitally manipulated photograph",
]

# Verdict thresholds.  These are the load-bearing decision boundaries
# for the CLIP/UnivFD verdict; the trained UnivFD probe (AUC 0.9933)
# produces the headline score and these constants determine which
# verdict_level applies.  Surfaced on every ClipDetectionResponse via
# `verdict_thresholds` so the UI consumes live values rather than
# hardcoding.  When the UnivFD probe is retrained (next: v10) bump
# _MODEL_VERSION and adjust constants together.
_SYNTHETIC_THRESHOLD = 0.60
_AUTHENTIC_THRESHOLD = 0.35
_MODEL_VERSION = "univfd-probe-v10onnx"
_THRESHOLD_BASIS = (
    "UnivFD probe v10onnx: LogisticRegression on CLIP ViT-B/32 embeddings "
    "extracted via the production ONNX runtime (NOT PyTorch+open_clip — "
    "see docs/calibration/univfd-v10onnx-divergence-fix.md). "
    "AUC 0.9929, FP 3.87% on photographic content, recall 95.77%, "
    "validated on 56,344 samples including platform-forwarded and "
    "multi-format augmentation (PNG/TIFF/WebP/HEIC re-encodes). "
    "Per-format AUC: PNG 0.998, TIFF 0.995, WebP 0.993, HEIC 0.990. "
    "Trained 2026-05-11 after diagnosis of PyTorch↔ONNX preprocess "
    "divergence (mean cos sim ~0.996, fixed by aligning training "
    "embeddings to the production PIL+ONNX path)."
)


def get_last_used_ts() -> float:
    """Return the Unix timestamp of the last CLIP model invocation (0.0 if never used)."""
    return _last_used_ts


def unload_clip_model() -> dict:
    """Release the CLIP ONNX sessions from memory to reclaim RAM.

    Sets all session globals to None and runs a GC cycle so Python
    can return the ~150 MB ONNX runtime + ~580 MB ONNX model footprint
    back to the OS. The next request will re-load via ``_ensure_model()``
    as normal.

    Returns a dict with ``unloaded`` (bool) and ``previously_loaded`` (bool).
    """
    global _vision_session, _text_session, _text_prompt_cache, _model_load_attempted

    previously_loaded = _vision_session is not None
    _vision_session = None
    _text_session = None
    _text_prompt_cache = None
    _model_load_attempted = False

    gc.collect()
    logger.info("clip ONNX sessions unloaded for RAM reclamation")
    return {"unloaded": True, "previously_loaded": previously_loaded}


def _verdict_thresholds() -> VerdictThresholds:
    """Build the VerdictThresholds payload from the module constants."""
    return VerdictThresholds(
        synthetic_min=_SYNTHETIC_THRESHOLD,
        authentic_max=_AUTHENTIC_THRESHOLD,
        model_version=_MODEL_VERSION,
        threshold_basis=_THRESHOLD_BASIS,
    )


# CLIP normalisation constants (identical to OpenCLIP / OpenAI CLIP).
# Mean and std are applied per-channel after dividing pixel values by 255.
_CLIP_MEAN = np.array([0.48145466, 0.4578275, 0.40821073], dtype=np.float32).reshape(
    3, 1, 1
)
_CLIP_STD = np.array([0.26862954, 0.26130258, 0.27577711], dtype=np.float32).reshape(
    3, 1, 1
)
_CLIP_INPUT_SIZE = 224


def _preprocess_image(img: Image.Image) -> np.ndarray:
    """Replicate the OpenCLIP ViT-B/32 preprocess pipeline using PIL + numpy.

    Steps mirror `open_clip.create_model_and_transforms("ViT-B-32")`:
        1. Convert to RGB (handle palette / alpha-channel inputs)
        2. Resize shorter edge to 224 with bicubic interpolation
        3. Centre-crop to 224×224
        4. To float32 in [0, 1]
        5. HWC → CHW
        6. Per-channel CLIP normalisation
        7. Add batch axis → (1, 3, 224, 224)

    Verified against the open_clip reference transform on 100 corpus images
    on 2026-05-03 — cosine similarity of resulting embeddings is 1.000000
    (see docs/calibration/univfd-v9-onnx-validation.md). Drop-in replacement
    for the torchvision Compose previously held in `_preprocess`.
    """
    if img.mode != "RGB":
        img = img.convert("RGB")
    w, h = img.size
    if w < h:
        new_w, new_h = _CLIP_INPUT_SIZE, int(round(h * _CLIP_INPUT_SIZE / w))
    else:
        new_w, new_h = int(round(w * _CLIP_INPUT_SIZE / h)), _CLIP_INPUT_SIZE
    img = img.resize((new_w, new_h), Image.BICUBIC)
    left = (new_w - _CLIP_INPUT_SIZE) // 2
    top = (new_h - _CLIP_INPUT_SIZE) // 2
    img = img.crop((left, top, left + _CLIP_INPUT_SIZE, top + _CLIP_INPUT_SIZE))
    arr = np.asarray(img, dtype=np.float32) / 255.0
    arr = arr.transpose(2, 0, 1)
    arr = (arr - _CLIP_MEAN) / _CLIP_STD
    return arr[np.newaxis, ...]


def _models_dir() -> str:
    """Resolve the directory holding the ONNX model files.

    Honours JURA_MODELS_DIR (set by the Rust spawn so a packaged .app points
    at the resource bundle's models/ folder) and falls back to the repo
    `models/` for `make dev` / direct uvicorn launches.
    """
    env = os.environ.get("JURA_MODELS_DIR")
    if env and os.path.isdir(env):
        return env
    return os.path.normpath(
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "models")
    )


def _ensure_model() -> bool:
    """Lazy-load the CLIP ONNX sessions. Returns True if both are ready."""
    global _vision_session, _text_session, _model_load_attempted

    if _vision_session is not None:
        return True

    if _model_load_attempted:
        # Already tried and failed — don't retry every request.
        return False

    _model_load_attempted = True

    try:
        import onnxruntime as ort
    except ImportError:
        logger.warning(
            "onnxruntime not installed — CLIP detection unavailable. "
            "Install with: pip install onnxruntime"
        )
        return False

    models_dir = _models_dir()
    vision_path = os.path.join(models_dir, "clip-vit-b32-vision.onnx")
    text_path = os.path.join(models_dir, "clip-vit-b32-text.onnx")

    if not os.path.exists(vision_path) or not os.path.exists(text_path):
        logger.warning(
            "CLIP ONNX model files not found in %s. Expected "
            "clip-vit-b32-vision.onnx + clip-vit-b32-text.onnx (+ .data files). "
            "Run scripts/export_clip_onnx.py to generate them.",
            models_dir,
        )
        return False

    try:
        logger.info("Loading CLIP ONNX vision encoder from %s ...", vision_path)
        _vision_session = ort.InferenceSession(
            vision_path, providers=["CPUExecutionProvider"]
        )
        logger.info("Loading CLIP ONNX text encoder from %s ...", text_path)
        _text_session = ort.InferenceSession(
            text_path, providers=["CPUExecutionProvider"]
        )
        logger.info("CLIP ONNX sessions loaded successfully.")
        return True
    except Exception:
        logger.exception("Failed to load CLIP ONNX sessions")
        _vision_session = None
        _text_session = None
        return False


def _load_univfd_probe():
    """Lazy-load the UnivFD linear probe from models/univfd_probe.joblib.

    Returns the probe (sklearn LogisticRegression) or None if unavailable.
    """
    global _univfd_probe, _univfd_probe_loaded

    if _univfd_probe_loaded:
        return _univfd_probe

    _univfd_probe_loaded = True

    try:
        import joblib

        # Honour JURA_MODELS_DIR (set by the Tauri Rust backend at spawn time
        # — lib.rs:6009 — and by the PyInstaller rthook when a models/ dir
        # ships next to the executable). Same dev/frozen pattern as
        # deepfake.py:_resolve_models_dir(). Without this, a frozen binary
        # resolves __file__ inside _MEIPASS and the relative ../../../models
        # path points into the extracted bundle dir where the joblib was
        # never shipped (per jura-sidecar.spec) — UnivFD then silently
        # reports unavailable in every build, gating off the v10onnx probe.
        env_dir = os.environ.get("JURA_MODELS_DIR")
        if env_dir and os.path.isdir(env_dir):
            probe_path = os.path.join(env_dir, "univfd_probe.joblib")
        else:
            probe_path = os.path.normpath(
                os.path.join(
                    os.path.dirname(__file__),
                    "..",
                    "..",
                    "..",
                    "models",
                    "univfd_probe.joblib",
                )
            )

        if os.path.exists(probe_path):
            h = hashlib.sha256()
            with open(probe_path, "rb") as fh:
                for chunk in iter(lambda: fh.read(65536), b""):
                    h.update(chunk)
            if h.hexdigest() != _UNIVFD_PROBE_SHA256:
                logger.warning(
                    "univfd_probe.joblib failed SHA-256 integrity check — "
                    "refusing to load. Re-train or restore from a trusted source."
                )
                _univfd_probe = None
            else:
                _univfd_probe = joblib.load(probe_path)
                logger.info("UnivFD probe loaded from %s", probe_path)
        else:
            logger.info(
                "UnivFD probe not found at %s — using zero-shot only", probe_path
            )
    except Exception:
        logger.exception("Failed to load UnivFD probe")
        _univfd_probe = None

    return _univfd_probe


def _encode_image(image: Image.Image) -> np.ndarray:
    """Run an image through the ONNX vision encoder, return l2-normalised 512-d vector."""
    image_input = _preprocess_image(image)
    out = _vision_session.run(
        None, {_vision_session.get_inputs()[0].name: image_input}
    )[0]
    features = out[0]
    norm = float(np.linalg.norm(features)) + 1e-12
    return (features / norm).astype(np.float32)


def _encode_text_prompts() -> np.ndarray:
    """Tokenise + encode the fixed _TEXT_PROMPTS into l2-normalised 512-d vectors.

    Cached on first call so repeated zero-shot scoring incurs only the image
    encoder cost. The text encoder ONNX exported from PyTorch 2.10 has a
    reshape op fixed to batch=1, so we loop one prompt at a time —
    negligible since the result is cached after first call.
    """
    global _text_prompt_cache
    if _text_prompt_cache is not None:
        return _text_prompt_cache

    from app.services.clip_tokenizer import tokenize  # noqa: PLC0415

    text_input_name = _text_session.get_inputs()[0].name
    embeddings = []
    for prompt in _TEXT_PROMPTS:
        tokens = tokenize([prompt])  # int32 (1, 77)
        out = _text_session.run(None, {text_input_name: tokens})[0]
        emb = out[0]
        norm = float(np.linalg.norm(emb)) + 1e-12
        embeddings.append((emb / norm).astype(np.float32))
    _text_prompt_cache = np.stack(embeddings)  # (5, 512)
    logger.info("CLIP text prompt embeddings cached (%d prompts)", len(_TEXT_PROMPTS))
    return _text_prompt_cache


def _score_univfd_probe(image: Image.Image) -> float | None:
    """Score an image using the UnivFD linear probe on CLIP embeddings.

    Returns the AI-generated probability [0, 1] or None if unavailable.
    JTV-143: ONNX-derived embeddings validated bit-identical to the
    PyTorch reference the probe was trained against (cosine 1.000000).
    """
    probe = _load_univfd_probe()
    if probe is None:
        return None

    try:
        embedding = _encode_image(image).reshape(1, -1)
        proba = probe.predict_proba(embedding)[0]
        # proba[1] = probability of class 1 (ai_generated)
        return float(proba[1])
    except Exception:
        logger.exception("UnivFD probe scoring failed")
        return None


def _classify_zero_shot(image: Image.Image) -> tuple[float, dict[str, float]]:
    """Compare image embedding to text prompts via CLIP zero-shot.

    Returns:
        (ai_probability, class_probabilities) where ai_probability is a
        float in [0, 1] and class_probabilities maps prompt labels to
        their softmax probabilities.
    """
    image_features = _encode_image(image)  # (512,)
    text_features = _encode_text_prompts()  # (5, 512)

    similarity = text_features @ image_features  # (5,)
    # Numerically stable softmax
    s = similarity - similarity.max()
    exp = np.exp(s)
    probs = exp / exp.sum()

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
        model_name="ViT-B-32 (laion2b_s34b_b79k, ONNX FP32)",
        model_available=False,
        summary=(
            "CLIP model not available — onnxruntime missing or "
            "clip-vit-b32-vision.onnx / clip-vit-b32-text.onnx absent from "
            "models/ directory."
        ),
        verdict_thresholds=_verdict_thresholds(),
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
    global _last_used_ts

    if not _ensure_model():
        return _unavailable_response()

    # Stamp last-used timestamp now that the model is confirmed loaded and
    # about to be invoked — used by the idle-watcher to decide eviction.
    _last_used_ts = time.time()

    try:
        pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    ai_score, class_probs = _classify_zero_shot(pil_image)

    # Try the UnivFD probe (trained linear classifier on CLIP embeddings)
    univfd_score = _score_univfd_probe(pil_image)
    univfd_available = univfd_score is not None

    # Use probe score as primary when available; fall back to zero-shot
    if univfd_available:
        score = max(0.0, min(1.0, univfd_score))
    else:
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

    # Summary.
    #
    # When the trained UnivFD probe is available it is the load-bearing
    # signal: a binary logistic regression on CLIP embeddings (AUC 0.9933)
    # producing the headline `score`.  The zero-shot `class_probs` are
    # softmax over CLIP text-similarity to the label prompts and are not
    # arithmetically related to the probe score — quoting them next to the
    # probe score implies a relationship that does not exist (a 0.87 probe
    # output and ~0.20 per-class zero-shot scores are perfectly consistent).
    # So we only quote class_probs when zero-shot is the source.
    if univfd_available:
        if verdict_level == "synthetic":
            summary = (
                f"CLIP UnivFD probe classifies this image as likely "
                f"AI-generated (probe score={score:.2f}, AUC 0.9933). "
                f"Zero-shot CLIP labels below are auxiliary text-similarity "
                f"scores and are not arithmetically related to the probe score."
            )
        elif verdict_level == "authentic":
            summary = (
                f"CLIP UnivFD probe classifies this image as likely a real "
                f"photograph (probe score={score:.2f}, AUC 0.9933). "
                f"Zero-shot CLIP labels below are auxiliary text-similarity "
                f"scores and are not arithmetically related to the probe score."
            )
        else:
            summary = (
                f"CLIP UnivFD probe classification is inconclusive "
                f"(probe score={score:.2f})"
            )
    else:
        # Zero-shot is the only signal — class_probs do explain the score.
        if verdict_level == "synthetic":
            summary = (
                f"CLIP zero-shot classifies this image as likely AI-generated "
                f"(score={score:.2f}, ai_generated={class_probs.get('ai_generated', 0):.2f}, "
                f"synthetic={class_probs.get('synthetic', 0):.2f})"
            )
        elif verdict_level == "authentic":
            summary = (
                f"CLIP zero-shot classifies this image as likely a real "
                f"photograph "
                f"(score={score:.2f}, photograph={class_probs.get('photograph', 0):.2f}, "
                f"real_scene={class_probs.get('real_scene', 0):.2f})"
            )
        else:
            summary = (
                f"CLIP zero-shot classification is inconclusive "
                f"(score={score:.2f}) — zero-shot has limited discriminative "
                f"power; a trained UnivFD linear probe is needed for "
                f"reliable results."
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
        univfd_score=round(univfd_score, 4) if univfd_score is not None else None,
        univfd_available=univfd_available,
        verdict_thresholds=_verdict_thresholds(),
    )
