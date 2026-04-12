"""
Jura Trace Sidecar — Audio Deepfake Detection (Sprint 35 skeleton).

Two-stage ensemble detector for AI-generated speech, voice cloning, and TTS:

    Stage 1  MFCC features → GradientBoostingClassifier (fast, ~20 ms)
    Stage 2  Wav2Vec2-Base embeddings → LogisticRegression (~150 ms)
    Ensemble final_score = 0.3 × stage1 + 0.7 × stage2

**CURRENT STATE**: No trained probes are deployed yet.  The service will
perform feature extraction and return ``model_loaded=False`` until the
Sprint 35 corpus (LibriSpeech authentic + ASVspoof synthetic) is collected
and ``scripts/train_audio_deepfake.py`` has been run.
"""

import logging
import os
import time
import tempfile
from pathlib import Path

import numpy as np

from app.models.schemas import AudioDeepfakeResponse

logger = logging.getLogger(__name__)

# ── Model directory ────────────────────────────────────────────────────────────

def _resolve_models_dir() -> str:
    """Resolve the models directory (dev and frozen/PyInstaller contexts)."""
    env_dir = os.environ.get("JURA_MODELS_DIR")
    if env_dir and os.path.isdir(env_dir):
        return env_dir
    # sidecar/app/services/audio_deepfake.py → ../../.. → sidecar/ → ../models/
    return os.path.normpath(
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "models")
    )


MODELS_DIR: str = _resolve_models_dir()

_STAGE1_PROBE_FILENAME = "audio_deepfake_stage1.joblib"
_STAGE2_PROBE_FILENAME = "audio_deepfake_stage2.joblib"

# ── Singleton Wav2Vec2 state ───────────────────────────────────────────────────

_wav2vec2_processor = None
_wav2vec2_model = None
_wav2vec2_load_attempted = False

# ── Feature extraction ─────────────────────────────────────────────────────────

_MIN_DURATION_SECONDS = 0.5
_TARGET_SR = 16_000
_N_MFCC = 40


def extract_mfcc_features(audio_path: str, sr: int = _TARGET_SR, n_mfcc: int = _N_MFCC) -> np.ndarray | None:
    """Extract MFCC feature vector from an audio file.

    Loads audio via librosa (preferred) or falls back to
    ``scipy.io.wavfile`` for WAV files when librosa is not installed.
    Resamples to 16 kHz mono.  Computes n_mfcc MFCCs over the full
    duration, then concatenates mean and std across time for both MFCCs
    and their deltas.

    Feature layout (160-dim):
        [mean_mfcc_0..39, std_mfcc_0..39, delta_mean_0..39, delta_std_0..39]

    Returns ``None`` if the file cannot be loaded or is shorter than 0.5 s.
    """
    try:
        audio, actual_sr = _load_audio(audio_path, target_sr=sr)
    except Exception as exc:
        logger.warning("MFCC: failed to load %s — %s", audio_path, exc)
        return None

    if audio is None or len(audio) == 0:
        return None

    duration = len(audio) / sr
    if duration < _MIN_DURATION_SECONDS:
        logger.debug("MFCC: audio too short (%.3f s < %.1f s)", duration, _MIN_DURATION_SECONDS)
        return None

    try:
        import librosa  # type: ignore[import-untyped]

        mfccs = librosa.feature.mfcc(y=audio, sr=sr, n_mfcc=n_mfcc)   # (n_mfcc, T)
        deltas = librosa.feature.delta(mfccs)                            # (n_mfcc, T)
    except ImportError:
        # librosa not available — fall back to a simple DCT-based MFCC via scipy
        mfccs, deltas = _compute_mfcc_scipy_fallback(audio, sr, n_mfcc)
        if mfccs is None:
            return None

    mfcc_mean = np.mean(mfccs, axis=1)   # (n_mfcc,)
    mfcc_std  = np.std(mfccs, axis=1)    # (n_mfcc,)
    delta_mean = np.mean(deltas, axis=1) # (n_mfcc,)
    delta_std  = np.std(deltas, axis=1)  # (n_mfcc,)

    features = np.concatenate([mfcc_mean, mfcc_std, delta_mean, delta_std])  # (160,)
    return features.astype(np.float32)


def extract_wav2vec2_embedding(audio_path: str, sr: int = _TARGET_SR) -> np.ndarray | None:
    """Extract a Wav2Vec2-Base embedding from an audio file.

    Lazy-loads ``facebook/wav2vec2-base`` (~360 MB) on first call and
    caches it in module-level state.  Returns a 768-dim vector
    (mean-pooled over the time axis of the last hidden state).

    Returns ``None`` when:
    - transformers or torch is not installed
    - the model download has not been triggered yet
    - the audio file cannot be loaded or is too short
    """
    global _wav2vec2_processor, _wav2vec2_model, _wav2vec2_load_attempted

    if not _wav2vec2_load_attempted:
        _wav2vec2_load_attempted = True
        try:
            from transformers import Wav2Vec2Processor, Wav2Vec2Model  # type: ignore[import-untyped]
            import torch  # type: ignore[import-untyped]

            logger.info("Wav2Vec2: loading facebook/wav2vec2-base …")
            _wav2vec2_processor = Wav2Vec2Processor.from_pretrained("facebook/wav2vec2-base")
            _wav2vec2_model = Wav2Vec2Model.from_pretrained("facebook/wav2vec2-base")
            _wav2vec2_model.eval()
            logger.info("Wav2Vec2: model loaded")
        except Exception as exc:
            logger.warning("Wav2Vec2: could not load model — %s", exc)
            _wav2vec2_processor = None
            _wav2vec2_model = None

    if _wav2vec2_processor is None or _wav2vec2_model is None:
        return None

    try:
        import torch  # type: ignore[import-untyped]

        audio, _ = _load_audio(audio_path, target_sr=sr)
        if audio is None or len(audio) / sr < _MIN_DURATION_SECONDS:
            return None

        inputs = _wav2vec2_processor(
            audio, sampling_rate=sr, return_tensors="pt", padding=True
        )
        with torch.no_grad():
            outputs = _wav2vec2_model(**inputs)

        # Mean-pool over time axis of last hidden state → (768,)
        hidden = outputs.last_hidden_state.squeeze(0)   # (T, 768)
        embedding = hidden.mean(dim=0).numpy().astype(np.float32)  # (768,)
        return embedding
    except Exception as exc:
        logger.warning("Wav2Vec2: embedding extraction failed — %s", exc)
        return None


# ── Ensemble scorer ────────────────────────────────────────────────────────────

_STAGE1_WEIGHT = 0.3
_STAGE2_WEIGHT = 0.7

# Verdict thresholds
_THRESHOLD_SYNTHETIC  = 0.65
_THRESHOLD_AUTHENTIC  = 0.35


def score_audio_deepfake(
    audio_path: str,
    mfcc_probe_path: str | None = None,
    wav2vec2_probe_path: str | None = None,
) -> AudioDeepfakeResponse:
    """Run the two-stage ensemble on an audio file.

    If neither probe file exists (the current state — no trained probes
    deployed yet), returns a response with ``model_loaded=False`` and
    ``verdict="model_not_loaded"``.  The caller should surface
    "Audio deepfake detection available after model training" to the user
    rather than treating this as an error.

    Args:
        audio_path: Absolute path to the audio file to analyse.
        mfcc_probe_path: Path to the Stage 1 ``*.joblib`` probe file.
            Defaults to ``<MODELS_DIR>/audio_deepfake_stage1.joblib``.
        wav2vec2_probe_path: Path to the Stage 2 ``*.joblib`` probe file.
            Defaults to ``<MODELS_DIR>/audio_deepfake_stage2.joblib``.

    Returns:
        ``AudioDeepfakeResponse`` with all fields populated.
    """
    t0 = time.perf_counter()

    stage1_probe_path = mfcc_probe_path or os.path.join(MODELS_DIR, _STAGE1_PROBE_FILENAME)
    stage2_probe_path = wav2vec2_probe_path or os.path.join(MODELS_DIR, _STAGE2_PROBE_FILENAME)

    stage1_available = os.path.isfile(stage1_probe_path)
    stage2_available = os.path.isfile(stage2_probe_path)

    # ── Audio file metadata ────────────────────────────────────────────────────
    duration_seconds: float | None = None
    sample_rate: int | None = None
    try:
        audio, sr_actual = _load_audio(audio_path, target_sr=_TARGET_SR)
        if audio is not None:
            duration_seconds = round(len(audio) / _TARGET_SR, 3)
            sample_rate = sr_actual
    except Exception:
        pass

    # ── No probes: return skeleton response ───────────────────────────────────
    if not stage1_available and not stage2_available:
        elapsed_ms = (time.perf_counter() - t0) * 1000.0
        mfcc_vec = extract_mfcc_features(audio_path)
        mfcc_ok = mfcc_vec is not None
        return AudioDeepfakeResponse(
            score=None,
            verdict="model_not_loaded",
            stage1_score=None,
            stage2_score=None,
            stages_available=[],
            model_loaded=False,
            duration_seconds=duration_seconds,
            sample_rate=sample_rate,
            mfcc_features_extracted=mfcc_ok,
            wav2vec2_embedding_extracted=False,
            processing_time_ms=round(elapsed_ms, 1),
        )

    # ── Stage 1: MFCC + GradientBoostingClassifier ────────────────────────────
    stage1_score: float | None = None
    mfcc_ok = False
    stages_available: list[str] = []

    if stage1_available:
        mfcc_vec = extract_mfcc_features(audio_path)
        if mfcc_vec is not None:
            mfcc_ok = True
            try:
                import joblib  # type: ignore[import-untyped]
                probe1 = joblib.load(stage1_probe_path)
                prob = probe1.predict_proba(mfcc_vec.reshape(1, -1))[0]
                # Class 1 = synthetic
                stage1_score = float(prob[1] if len(prob) > 1 else prob[0])
                stages_available.append("stage1")
            except Exception as exc:
                logger.warning("Stage 1 inference failed: %s", exc)

    # ── Stage 2: Wav2Vec2 + LogisticRegression ────────────────────────────────
    stage2_score: float | None = None
    wav2vec2_ok = False

    if stage2_available:
        emb = extract_wav2vec2_embedding(audio_path)
        if emb is not None:
            wav2vec2_ok = True
            try:
                import joblib  # type: ignore[import-untyped]
                probe2 = joblib.load(stage2_probe_path)
                prob = probe2.predict_proba(emb.reshape(1, -1))[0]
                stage2_score = float(prob[1] if len(prob) > 1 else prob[0])
                stages_available.append("stage2")
            except Exception as exc:
                logger.warning("Stage 2 inference failed: %s", exc)

    # ── Ensemble ───────────────────────────────────────────────────────────────
    final_score: float | None = None
    if stage1_score is not None and stage2_score is not None:
        final_score = _STAGE1_WEIGHT * stage1_score + _STAGE2_WEIGHT * stage2_score
    elif stage1_score is not None:
        final_score = stage1_score
    elif stage2_score is not None:
        final_score = stage2_score

    verdict = _score_to_verdict(final_score)
    model_loaded = bool(stages_available)

    elapsed_ms = (time.perf_counter() - t0) * 1000.0
    return AudioDeepfakeResponse(
        score=round(final_score, 4) if final_score is not None else None,
        verdict=verdict,
        stage1_score=round(stage1_score, 4) if stage1_score is not None else None,
        stage2_score=round(stage2_score, 4) if stage2_score is not None else None,
        stages_available=stages_available,
        model_loaded=model_loaded,
        duration_seconds=duration_seconds,
        sample_rate=sample_rate,
        mfcc_features_extracted=mfcc_ok,
        wav2vec2_embedding_extracted=wav2vec2_ok,
        processing_time_ms=round(elapsed_ms, 1),
    )


# ── Private helpers ────────────────────────────────────────────────────────────

def _load_audio(audio_path: str, target_sr: int = _TARGET_SR) -> tuple[np.ndarray | None, int]:
    """Load audio file as a mono float32 numpy array at target_sr.

    Tries librosa first (handles MP3/FLAC/OGG/M4A/etc.), then falls back
    to scipy.io.wavfile for WAV-only if librosa is not installed.

    Returns (audio_array, actual_sr_after_resample).
    Raises on load failure so callers can log the path-specific error.
    """
    try:
        import librosa  # type: ignore[import-untyped]
        audio, _ = librosa.load(audio_path, sr=target_sr, mono=True)
        return audio.astype(np.float32), target_sr
    except ImportError:
        pass

    # FFmpeg fallback: convert any audio format to 16-bit PCM WAV in a temp
    # file, then read with scipy. FFmpeg is already a runtime dependency for
    # the audio_metadata service and faster-whisper transcription. This path
    # handles MP3, FLAC, OGG, AAC, M4A, OPUS, AIFF — everything FFmpeg can
    # decode — without adding librosa as a dependency.
    import shutil
    import subprocess
    import tempfile

    ffmpeg_bin = shutil.which("ffmpeg")
    if ffmpeg_bin:
        try:
            with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
                tmp_path = tmp.name
            subprocess.run(
                [
                    ffmpeg_bin, "-y", "-i", audio_path,
                    "-ac", "1",                    # mono
                    "-ar", str(target_sr),          # target sample rate
                    "-sample_fmt", "s16",           # 16-bit PCM
                    "-f", "wav",                    # WAV output
                    tmp_path,
                ],
                capture_output=True, timeout=30, check=True,
            )
            from scipy.io import wavfile as _wavfile  # type: ignore[import-untyped]
            _, data = _wavfile.read(tmp_path)
            if data.ndim > 1:
                data = data.mean(axis=1)
            data = data.astype(np.float32) / (2 ** 15)
            return data, target_sr
        except Exception:
            raise
        finally:
            import os
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)

    # Last resort: scipy.io.wavfile direct read (WAV only)
    try:
        from scipy.io import wavfile as _wavfile  # type: ignore[import-untyped]
        native_sr, data = _wavfile.read(audio_path)
        if data.ndim > 1:
            data = data.mean(axis=1)
        data = data.astype(np.float32)
        if data.max() > 1.0:
            data = data / (2 ** 15)
        if native_sr != target_sr:
            new_length = int(len(data) * target_sr / native_sr)
            data = np.interp(
                np.linspace(0, len(data) - 1, new_length),
                np.arange(len(data)),
                data,
            ).astype(np.float32)
        return data, target_sr
    except Exception:
        raise


def _compute_mfcc_scipy_fallback(
    audio: np.ndarray, sr: int, n_mfcc: int
) -> tuple[np.ndarray | None, np.ndarray | None]:
    """Minimal MFCC computation using scipy when librosa is unavailable.

    Returns (mfccs, deltas) as (n_mfcc, T) arrays, or (None, None) on failure.
    This is a best-effort fallback — librosa is strongly preferred.
    """
    try:
        from scipy.signal import spectrogram  # type: ignore[import-untyped]
        from scipy.fftpack import dct         # type: ignore[import-untyped]

        frame_size = int(sr * 0.025)  # 25 ms
        hop_size   = int(sr * 0.010)  # 10 ms

        _, _, spec = spectrogram(audio, fs=sr, nperseg=frame_size, noverlap=frame_size - hop_size)
        log_spec = np.log(np.maximum(spec, 1e-10))
        mfccs = dct(log_spec, axis=0, norm="ortho")[:n_mfcc, :]

        # Simple first-order delta
        pad = np.pad(mfccs, ((0, 0), (1, 1)), mode="edge")
        deltas = (pad[:, 2:] - pad[:, :-2]) / 2.0

        return mfccs.astype(np.float32), deltas.astype(np.float32)
    except Exception as exc:
        logger.warning("scipy MFCC fallback failed: %s", exc)
        return None, None


def _score_to_verdict(score: float | None) -> str:
    """Convert an ensemble score to a verdict string."""
    if score is None:
        return "model_not_loaded"
    if score >= _THRESHOLD_SYNTHETIC:
        return "likely_synthetic"
    if score <= _THRESHOLD_AUTHENTIC:
        return "authentic"
    return "inconclusive"
