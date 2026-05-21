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

# ── JTV-143 (3 May 2026): CLIP via ONNX runtime ──────────────────────────────
# The CLIP detector now loads ViT-B/32 vision + text encoders through
# onnxruntime instead of PyTorch + open_clip. onnxruntime ships native
# .dylib / .so / .dll files that need collect_all to be staged correctly.
# torch + open_clip stay in `excludes` below — the size delta would be
# ~1.5 GB (torch) + ~70 MB (open_clip wheel) on top of the ONNX bundle.
d, b, h = collect_all("onnxruntime")
datas += d; binaries += b; hiddenimports += h

# CLIP ONNX models (~579 MB combined: 1.2 MB vision graph + 335 MB vision
# weights + 1.0 MB text graph + 242 MB text weights). The .onnx file is
# the protobuf graph; the .onnx.data sidecar holds tensor weights via the
# ONNX external-data convention. Both files must sit in the same directory
# at runtime — onnxruntime auto-resolves the .data file by relative name.
#
# JTV-184 (2026-05-16): CLIP ONNX files are NO LONGER bundled inside the
# PyInstaller sidecar binary. They were previously added to `datas` here,
# AND ship as Tauri resources at Contents/Resources/models/ via the
# `resources: ["models/*"]` declaration in src-tauri/tauri.conf.json —
# meaning the same ~580 MB was carried in the .app twice. The runtime
# loader at clip_detector.py:_models_dir() honours JURA_MODELS_DIR (set by
# the Tauri Rust shell at spawn time to point at the bundled Resources
# directory), so the PyInstaller copy was never reached at runtime in the
# Tauri-spawned production path — it was pure dead weight that doubled the
# sidecar binary size from ~150 MB to ~731 MB and pushed PyInstaller
# --onefile cold-extract from ~30 s to 4+ min on a clean .app install.
# That cold-extract budget overran Tauri's wait_for_sidecar_ready window
# (~140 s) and produced the "Analysis Engine offline" symptom on every
# fresh install. Removing this datas block restores the sidecar to its
# pre-JTV-143 size envelope and brings cold-extract back below Tauri's
# probe budget.
#
# For dev mode (uvicorn launched directly), clip_detector.py falls back to
# `<repo>/models/clip-vit-b32-*` via __file__-relative resolution — that
# path is untouched by this change.
#
# Re-add this block ONLY if a deployment context emerges where CLIP must
# live inside the PyInstaller bundle (e.g. a standalone CLI distribution
# of the sidecar with no Tauri Resources beside it). In that case, also
# remove the `models/*` glob from src-tauri/tauri.conf.json to avoid the
# duplicate-shipping regression.

# Standalone CLIP BPE tokeniser ships its vocab gz file alongside the
# Python module (sidecar/app/services/bpe_simple_vocab_16e6.txt.gz, ~1.3 MB).
# clip_tokenizer.py resolves it via __file__-relative path which translates
# to _MEIPASS/app/services/ inside the frozen binary.
_bpe_path = _spec_dir / "app" / "services" / "bpe_simple_vocab_16e6.txt.gz"
if _bpe_path.exists():
    datas += [(str(_bpe_path), "app/services")]

# certifi — SSL CA bundle needed for outbound HTTPS requests
try:
    datas += collect_data_files("certifi")
except Exception:
    pass

# invisible-watermark removed from v1.0 bundle 2026-05-21.
# Rationale: imwatermark's __init__.py eagerly imports rivaGan -> torch.
# PyInstaller static analysis follows the chain at build time and bundles
# torch despite the spec excludes, doubling the bundle to 1.5 GB. The
# watermark UI is gated behind V1_SHOW_WATERMARK=false in the frontend
# (feature deferred to v1.1 per persona-testing + content-authenticity-
# expert + tech-debt-analyst agent consensus). Re-enable in v1.1 by
# either vendoring imwatermark with rivaGan lazy-import patched OR
# replacing with a pure-Rust DWT-DCT-SVD implementation that lets
# embed + extract use the same library without torch dependency.
# d, b, h = collect_all("imwatermark")
# datas += d; binaries += b; hiddenimports += h

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
    # JTV-143: standalone CLIP BPE tokeniser (replaces open_clip dependency)
    "app.services.clip_tokenizer",
    # `regex` is the third-party PCRE-compatible engine the BPE tokeniser uses
    # (bundled with `regex` PyPI pkg, not stdlib `re`). Hidden because it's
    # imported via `import regex as re` which static analysers sometimes miss.
    "regex",
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
#   - torch / torchvision / open_clip: JTV-143 (3 May 2026) — CLIP runs via
#     onnxruntime instead; the ONNX bundle is ~580 MB combined for vision +
#     text encoders, vs ~1.5 GB for the equivalent torch path. The standalone
#     CLIP BPE tokeniser at sidecar/app/services/clip_tokenizer.py replaces
#     open_clip's tokeniser. timm / transformers were never used at runtime
#     (transformers was an audio-deepfake Stage-2 candidate now deferred).
#   - faster_whisper / ctranslate2: JTV-138 (2 May 2026) — transcription
#     dropped from v1.0; restored under JTV-139 v1.0.x.
#   - tkinter / _tkinter: no GUI needed in a headless daemon sidecar.
#   - trio: anyio alternative backend; uvicorn uses asyncio.
#   - mypy: pydantic ships a mypy plugin that is never imported at runtime.
#   - pytest / gradio / IPython: dev/test tooling.
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

# JTV-184 Phase 4 — `--onedir` build mode toggle.
#
# Set `JURA_SIDECAR_ONEDIR=1` in the environment when invoking PyInstaller
# to produce a directory-tree bundle (`dist/jura-sidecar/`) instead of the
# default self-extracting single-file binary (`dist/jura-sidecar`). The
# directory mode eliminates the runtime extraction step entirely — the
# Python interpreter, native `.so` / `.dylib` extensions, and bundled
# data files sit on disk in `_internal/` ready to use, so cold launch
# drops from ~90 s (single-file extract) to ~5 s (bootloader + Python
# import).
#
# Trade-off: the `.app` grows by ~200 MB because there is no compression
# step for the bundle payload. The signing surface also grows from one
# Mach-O to ~150-300 nested `.dylib` / `.so` files — each must be
# Developer-ID-signed before notarisation (see the JTV-184 Phase 5 CI
# step for the recursive codesign pass).
#
# Spike-mode usage:
#     JURA_SIDECAR_ONEDIR=1 python -m PyInstaller jura-sidecar.spec --noconfirm
#
# CI-mode usage:
#     The .github/workflows/release.yml and .forgejo/workflows/release.yml
#     pipelines will gain the env var alongside the recursive signing step
#     in JTV-184 Phase 5. For now the default (env-var unset) preserves
#     the existing --onefile build so accidental local rebuilds do not
#     surprise the CI pipeline.
_onedir_mode = _os.environ.get("JURA_SIDECAR_ONEDIR", "0") == "1"

if _onedir_mode:
    # --onedir: EXE block carries only the script bootstrap (no binaries
    # or datas — those move to COLLECT). `exclude_binaries=True` is the
    # signal that tells the PyInstaller bootloader to look for its
    # payload in a sibling `_internal/` directory at runtime rather than
    # self-extracting from the EXE itself.
    exe = EXE(
        pyz,
        a.scripts,
        [],
        exclude_binaries=True,
        name=_name,
        debug=False,
        bootloader_ignore_signals=False,
        strip=False,
        upx=True,
        upx_exclude=_upx_exclude,
        console=True,
        disable_windowed_traceback=False,
        argv_emulation=False,
        target_arch=_target_arch,
        codesign_identity=None,
        entitlements_file=None,
    )
    coll = COLLECT(
        exe,
        a.binaries,
        a.datas,
        strip=False,
        upx=True,
        upx_exclude=_upx_exclude,
        name=_name,
    )
else:
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
