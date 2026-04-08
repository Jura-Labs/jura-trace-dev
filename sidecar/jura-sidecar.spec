# -*- mode: python ; coding: utf-8 -*-
#
# Jura Trace — PyInstaller spec for the Python ML sidecar
#
# Cross-platform: macOS (arm64 / x86_64), Windows (AMD64), Linux (x86_64).
# Build on the target host runner — PyInstaller produces a native binary for
# the machine it runs on.
#
# Usage (from sidecar/ directory):
#
#   python -m PyInstaller jura-sidecar.spec --noconfirm
#
# Output binary: dist/jura-sidecar  (dist/jura-sidecar.exe on Windows)
# Build artefacts: build/jura-sidecar/
#
# Distribution model (as of Sprint 18):
#   - The frozen binary is a "core" build: torch / open_clip / faster-whisper
#     are EXCLUDED. CLIP detection reports model_available=False (graceful
#     degradation). Transcription reports unavailable.
#   - Model files (deepfake_classifier.joblib, univfd_probe.joblib) are NOT
#     bundled inside the binary. Ship them alongside it in a models/ directory
#     and set JURA_MODELS_DIR to that path — or rely on the Rust sidecar
#     launcher to set the env var via the runtime hook below.
#   - knowledge_base/*.txt IS bundled via datas so the TF-IDF RAG index works
#     inside the frozen binary. The runtime hook sets JURA_KB_DIR to
#     sys._MEIPASS/knowledge_base so knowledge_retriever.py finds the files.
#   - FFmpeg / ffprobe are system dependencies — not bundled. The platform
#     installer must ensure they are present. The health endpoint correctly
#     reports their unavailability when absent.
#
# Open issues (see docs/pyinstaller-spike-findings.md for full details):
#  1. Model files ship alongside the binary (not inside it) to allow
#     independent updates. The Rust backend sets JURA_MODELS_DIR when spawning.
#  2. FFmpeg is not bundled; health endpoint gracefully reports unavailability.
#  3. macOS code-signing: set codesign_identity below for notarised builds.
#  4. Cold-start: onefile extracts ~250 MB on first run (~1-2 s on Apple Silicon).
#     Consider --onedir for production (faster start, larger footprint on disk).
#  5. Windows + Linux: smoke-tested in CI only; manual QA pass required.

import platform
import sys
from pathlib import Path
from PyInstaller.utils.hooks import collect_all, collect_data_files

# SPEC is set by PyInstaller to the absolute path of this spec file.
_spec_dir = Path(SPEC).parent  # noqa: F821 — SPEC injected by PyInstaller

# ── Binary name ───────────────────────────────────────────────────────────────
# PyInstaller automatically appends .exe on Windows; we use a plain name here.
_name = "jura-sidecar"

# ── Architecture detection ────────────────────────────────────────────────────
# On macOS, pass the host machine's architecture so the binary is correctly
# tagged (arm64 on Apple Silicon, x86_64 on Intel). On Windows and Linux,
# PyInstaller builds for the host arch implicitly; target_arch is macOS-only.
_host_arch = platform.machine()   # 'arm64', 'x86_64', or 'AMD64'

if sys.platform == "darwin":
    # Normalise: platform.machine() returns 'arm64' on Apple Silicon
    _target_arch = _host_arch  # 'arm64' or 'x86_64'
else:
    # target_arch is not meaningful on Windows / Linux — must be None
    _target_arch = None

# ── UPX excludes (platform-conditional) ──────────────────────────────────────
# UPX compresses the binary (~30% smaller) but cannot handle certain native
# libraries. Exclude fragile libs per platform to avoid corruption at runtime.
# Set upx=False during development for faster iteration (avoids ~2-3 s cold-
# start decompression cost even when UPX runs cleanly).
if sys.platform == "darwin":
    _upx_exclude = [
        "libtorch*.dylib",
        "libc10.dylib",
        "libopencv*.dylib",
        "libopenblas*.dylib",
    ]
elif sys.platform == "win32":
    _upx_exclude = [
        "torch*.dll",
        "opencv*.dll",
        "python3*.dll",
        "vcruntime*.dll",
        "msvcp*.dll",
        "openblas*.dll",
    ]
else:
    # Linux
    _upx_exclude = [
        "libtorch*.so",
        "libopencv*.so",
        "libpython3*.so",
        "libopenblas*.so",
    ]

datas = []
binaries = []
hiddenimports = []

# ── collect_all for dynamically-imported framework packages ──────────────────
# These packages use dynamic import patterns that PyInstaller's static analyser
# cannot resolve (e.g. plugin discovery, metaclass-based registration).
for package in ["uvicorn", "fastapi", "pydantic", "pydantic_settings", "starlette"]:
    d, b, h = collect_all(package)
    datas += d; binaries += b; hiddenimports += h

# scikit-image and sklearn ship data files (lookup tables, datasets, joblib
# models). collect_data_files handles these; collect_all is not needed because
# the hidden imports list below covers the runtime-loaded submodules.
datas += collect_data_files("skimage")
datas += collect_data_files("sklearn")

# chromadb and sentence-transformers are optional heavy dependencies.
# collect_all is wrapped in try/except so CI builds without them don't abort.
for _optional_pkg in ["chromadb", "sentence_transformers"]:
    try:
        d, b, h = collect_all(_optional_pkg)
        datas += d; binaries += b; hiddenimports += h
    except Exception:
        pass

# certifi — SSL CA bundle needed for outbound HTTPS requests
try:
    datas += collect_data_files("certifi")
except Exception:
    pass

# invisible-watermark (package name differs from pip install name)
d, b, h = collect_all("imwatermark")
datas += d; binaries += b; hiddenimports += h

# ── Bundle knowledge_base text files ─────────────────────────────────────────
# knowledge_retriever.py resolves knowledge_base/ relative to __file__. In a
# frozen binary __file__ points into _MEIPASS, so we bundle the .txt files
# there and set JURA_KB_DIR in the runtime hook below.
_kb_dir = _spec_dir / "knowledge_base"
if _kb_dir.exists():
    datas += [(str(f), "knowledge_base") for f in sorted(_kb_dir.glob("*.txt"))]

# ── Hidden imports ────────────────────────────────────────────────────────────
# Anything that is imported at runtime via a string, getattr, or inside an
# 'if TYPE_CHECKING' block that PyInstaller cannot resolve statically.
hiddenimports += [
    # uvicorn internals — dynamic event loop selection and protocol loading
    "uvicorn.logging",
    "uvicorn.lifespan",
    "uvicorn.lifespan.on",
    "uvicorn.protocols",
    "uvicorn.protocols.http",
    "uvicorn.protocols.http.auto",
    "uvicorn.protocols.http.h11_impl",
    "anyio._backends._asyncio",       # uvicorn async backend

    # Pillow lazy image plugin loading — PIL loads plugins on demand
    "PIL.JpegImagePlugin",
    "PIL.PngImagePlugin",
    "PIL.BmpImagePlugin",
    "PIL.WebPImagePlugin",
    "PIL.TiffImagePlugin",
    "PIL.GifImagePlugin",

    # scikit-learn internals loaded by joblib when deserialising the GBM model
    "sklearn.ensemble._gb",
    "sklearn.ensemble._gradient_boosting",
    "sklearn.tree._classes",
    "sklearn.utils._weight_vector",
    "sklearn.utils._bunch",

    # scipy sub-modules not auto-detected by static analysis
    "scipy.special._cdflib",
    "scipy.fft._pocketfft",

    # imagehash — used by noise/ELA services for perceptual hash comparisons
    "imagehash",

    # httpx transport — used by claim_checker when calling Ollama
    "httpx._transports.default",
    "httpx._transports.async_",

    # pydantic-settings env-var loading
    "pydantic_settings.env_settings",

    # python-multipart — FastAPI imports this dynamically for form/file uploads
    "multipart",
    "multipart.multipart",

    # ── App service modules ───────────────────────────────────────────────────
    # FastAPI router discovery cannot be traced statically; list every module
    # that is imported (directly or via the router) at request time.
    "app.services.ela",
    "app.services.noise_analysis",
    "app.services.copy_move",
    "app.services.deepfake",
    "app.services.jpeg_ghost",
    "app.services.npr",
    "app.services.segmented_ela",
    "app.services.shadow_consistency",
    "app.services.colour_temperature",
    "app.services.splice_boundary",
    "app.services.clip_detector",
    "app.services.watermark",
    "app.services.claim_checker",
    "app.services.knowledge_retriever",
    "app.services.transcription",
    "app.services.describe_image",
    "app.services.video_metadata",
    "app.services.audio_metadata",
    "app.services.video_frames",
    "app.services.video_deepfake",

    # Sprint 21-26 additions — must be listed explicitly
    "app.services.noise_visualisation",
    "app.services.clahe",
    "app.services.frequency_visualisation",
    "app.services.jpeg_grid",
    "app.services.roi_analysis",
    "app.services.gan_fingerprint",

    # scipy submodule used by gan_fingerprint.py
    "scipy.ndimage",

    # certifi — SSL CA bundle bundled by httpx; retained even though all
    # outbound httpx calls are to 127.0.0.1, because httpx imports certifi
    # at module load time regardless of target host.
    "certifi",

    # ── App framework modules ─────────────────────────────────────────────────
    "app.api.forensics",
    "app.api.health",
    "app.models.schemas",
    "app.config",
]

# ── Excludes ──────────────────────────────────────────────────────────────────
# Packages that must not be bundled in the core binary:
#   - torch / torchvision / open_clip: ~200 MB compressed; CLIP is optional
#     and degrades gracefully (ClipDetectionResponse.model_available=False).
#   - faster_whisper / ctranslate2: optional transcription; requires native
#     ctranslate2 libs that are non-trivial to freeze cross-platform.
#   - tkinter / _tkinter: no GUI needed in a headless daemon sidecar.
#   - trio: anyio alternative backend; uvicorn uses asyncio.
#   - mypy: pydantic ships a mypy plugin that is never imported at runtime.
#   - pytest / gradio / IPython: dev/test tooling.
#
# NOTE: To build a "full" binary with CLIP support (~315 MB), remove torch,
# torchvision, and open_clip from this list and run a separate CI matrix job.
excludes = [
    "torch",
    "torchvision",
    "open_clip",
    "timm",
    "transformers",
    "faster_whisper",
    "ctranslate2",
    "tkinter",
    "_tkinter",
    "trio",
    "mypy",
    "pytest",
    "gradio",
    "IPython",
    "notebook",
    "jupyter",
    "jedi",
    "matplotlib",
]

# ── Runtime hook ──────────────────────────────────────────────────────────────
# The runtime hook runs inside the frozen process before any app code imports.
# It sets the two path env vars that the sidecar services use to locate data
# files that cannot be resolved via __file__ in a frozen binary:
#
#   JURA_MODELS_DIR — absolute path to models/*.joblib files.
#     Set to <executable_dir>/models so the model files shipped alongside the
#     binary are found. The Rust sidecar launcher (sidecar.rs) may override
#     this via the environment if the models live elsewhere.
#
#   JURA_KB_DIR — absolute path to knowledge_base/*.txt files.
#     Set to sys._MEIPASS/knowledge_base so the TF-IDF index finds the .txt
#     files bundled inside the frozen binary via the datas list above.
#
# This hook is written to a temporary file that PyInstaller injects at startup.
import tempfile
import os as _os

_hook_src = '''\
import os
import sys

# Models directory: look next to the executable first (models shipped alongside
# the binary); fall back to the _MEIPASS extraction directory.
if not os.environ.get("JURA_MODELS_DIR"):
    _exe_dir = os.path.dirname(sys.executable)
    _models_candidate = os.path.join(_exe_dir, "models")
    if os.path.isdir(_models_candidate):
        os.environ["JURA_MODELS_DIR"] = _models_candidate
    elif hasattr(sys, "_MEIPASS"):
        _meipass_models = os.path.join(sys._MEIPASS, "models")
        if os.path.isdir(_meipass_models):
            os.environ["JURA_MODELS_DIR"] = _meipass_models

# Knowledge base directory: always inside _MEIPASS (bundled via spec datas).
if not os.environ.get("JURA_KB_DIR") and hasattr(sys, "_MEIPASS"):
    _kb_candidate = os.path.join(sys._MEIPASS, "knowledge_base")
    if os.path.isdir(_kb_candidate):
        os.environ["JURA_KB_DIR"] = _kb_candidate
'''

_hook_file = _os.path.join(_os.path.dirname(_os.path.abspath(SPEC)), "pyi_rthook_jura.py")  # noqa: F821
with open(_hook_file, "w", encoding="utf-8") as _fh:
    _fh.write(_hook_src)

# ── Analysis ──────────────────────────────────────────────────────────────────
a = Analysis(
    ["main.py"],
    pathex=[str(_spec_dir)],   # lets PyInstaller find app/__init__.py
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[_hook_file],
    excludes=excludes,
    noarchive=False,
    optimize=0,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name=_name,
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    # UPX compresses the binary (saves ~30%) but adds ~2-3 s cold-start cost
    # while decompressing. Set upx=False for faster development iteration.
    upx=True,
    upx_exclude=_upx_exclude,
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,  # not needed for a CLI/daemon sidecar
    target_arch=_target_arch,
    # Code signing:
    #   macOS: set to "Developer ID Application: Jura Labs CIC (<TEAMID>)"
    #          for notarised builds (required for Gatekeeper acceptance).
    #          The Tauri release workflow should inject this via the
    #          APPLE_SIGNING_IDENTITY secret.
    #   Windows: Authenticode signing is handled post-build via signtool in CI.
    codesign_identity=None,
    entitlements_file=None,
)
