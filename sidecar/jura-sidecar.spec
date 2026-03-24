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
# Open issues (see docs/pyinstaller-spike-findings.md):
#  1. models/*.joblib are NOT bundled — ship alongside the binary and set
#     JURA_MODELS_DIR env var, or place at <executable_dir>/../models/.
#  2. knowledge_base/*.txt IS bundled via datas below. The Rust backend must
#     set JURA_KB_DIR=$MEIPASS/knowledge_base when spawning the sidecar.
#  3. FFmpeg/ffprobe are system deps — not bundled. The platform installer
#     must handle them; health endpoint gracefully reports unavailability.
#  4. open_clip weights (~350 MB) download at runtime, cached in user dir.
#  5. faster-whisper excluded (commented out in requirements.txt).

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
# tagged (arm64 on Apple Silicon, x86_64 on Intel).  On Windows and Linux,
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
    ]
elif sys.platform == "win32":
    _upx_exclude = [
        "torch*.dll",
        "opencv*.dll",
        "python3*.dll",
        "vcruntime*.dll",
        "msvcp*.dll",
    ]
else:
    # Linux
    _upx_exclude = [
        "libtorch*.so",
        "libopencv*.so",
        "libpython3*.so",
    ]

datas = []
binaries = []
hiddenimports = []

# ── collect_all for dynamically-imported framework packages ──────────────────
for package in ["uvicorn", "fastapi", "pydantic", "pydantic_settings", "starlette"]:
    d, b, h = collect_all(package)
    datas += d; binaries += b; hiddenimports += h

# scikit-image and sklearn ship data files (lookup tables, datasets)
datas += collect_data_files("skimage")
datas += collect_data_files("sklearn")

# invisible-watermark (package name differs from pip name)
d, b, h = collect_all("imwatermark")
datas += d; binaries += b; hiddenimports += h

# ── Bundle knowledge_base text files ─────────────────────────────────────────
# knowledge_retriever.py resolves knowledge_base/ via __file__, which in a
# frozen binary points to _MEIPASS. Bundle the .txt files so they are
# extracted alongside the rest of the package at runtime.
_kb_dir = _spec_dir / "knowledge_base"
if _kb_dir.exists():
    datas += [(str(f), "knowledge_base") for f in _kb_dir.glob("*.txt")]

# ── Hidden imports ────────────────────────────────────────────────────────────
hiddenimports += [
    "anyio._backends._asyncio",       # uvicorn async backend
    "sklearn.utils._weight_vector",   # joblib GBM internals
    "scipy.special._cdflib",          # scipy sub-module not auto-detected
    "PIL.JpegImagePlugin",
    "PIL.PngImagePlugin",
    "PIL.BmpImagePlugin",
    "PIL.WebPImagePlugin",
    "imagehash",
    "httpx._transports.default",      # used by claim_checker -> Ollama
    # App subpackages — FastAPI router discovery
    "app.services.ela",
    "app.services.noise_analysis",
    "app.services.copy_move",
    "app.services.deepfake",
    "app.services.jpeg_ghost",
    "app.services.npr",
    "app.services.chromatic_aberration",
    "app.services.segmented_ela",
    "app.services.shadow_consistency",
    "app.services.colour_temperature",
    "app.services.splice_boundary",
    "app.services.clip_detector",
    "app.services.watermark",
    "app.services.claim_checker",
    "app.services.knowledge_retriever",
    "app.services.transcription",
    "app.services.video_metadata",
    "app.services.audio_metadata",
    "app.services.video_frames",
    "app.services.video_deepfake",
    "app.api.forensics",
    "app.api.health",
    "app.models.schemas",
    "app.config",
]

# ── Excludes ──────────────────────────────────────────────────────────────────
excludes = [
    "tkinter", "_tkinter",  # no GUI
    "trio",                 # asyncio used, not trio
    "mypy",                 # dev-only type checker
    "pytest",               # test-only
    "gradio",               # pulled in by transformers deps but unused
    "IPython", "notebook", "jupyter",
]

# ── Analysis ──────────────────────────────────────────────────────────────────
a = Analysis(
    ["main.py"],
    pathex=[str(_spec_dir)],   # lets PyInstaller find app/__init__.py
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
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
    # while it decompresses. Set upx=False during development for faster iteration.
    upx=True,
    upx_exclude=_upx_exclude,
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,  # not needed for a CLI/daemon sidecar
    target_arch=_target_arch,
    # Code signing: set to Apple Developer ID string for signed macOS distribution.
    # Windows Authenticode signing is handled post-build via signtool in CI.
    # TODO Sprint 19: wire up APPLE_SIGNING_IDENTITY secret in release workflow.
    codesign_identity=None,
    entitlements_file=None,
)
