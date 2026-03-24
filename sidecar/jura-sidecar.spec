# -*- mode: python ; coding: utf-8 -*-
#
# Jura Trace — PyInstaller spec for the Python ML sidecar
#
# Target: macOS arm64 (Apple Silicon). For cross-platform builds, change
# target_arch below and build on the corresponding host runner.
#
# Usage (from sidecar/ directory):
#
#   python -m PyInstaller jura-sidecar.spec --noconfirm
#
# Output binary: dist/jura-sidecar
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

from pathlib import Path
from PyInstaller.utils.hooks import collect_all, collect_data_files

# SPEC is set by PyInstaller to the absolute path of this spec file.
_spec_dir = Path(SPEC).parent  # noqa: F821 — SPEC injected by PyInstaller

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
    name="jura-sidecar",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    # UPX compresses the binary (saves ~30%) but adds ~2-3 s cold-start cost
    # while it decompresses. Set upx=False during development for faster iteration.
    upx=True,
    upx_exclude=[
        # These dylibs are fragile under UPX on macOS arm64
        "libtorch*.dylib",
        "libc10.dylib",
        "libopencv*.dylib",
    ],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,  # not needed for a CLI/daemon sidecar
    target_arch="arm64",   # change to "x86_64" for Intel, None for auto
    codesign_identity=None,   # set Apple Developer ID for signed distribution
    entitlements_file=None,
)
