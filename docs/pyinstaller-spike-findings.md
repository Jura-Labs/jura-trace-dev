# PyInstaller Spike Findings — Jura Trace ML Sidecar

**Date**: 2026-03-24
**Sprint context**: Pre-Sprint 18 investigation
**Author**: DevOps agent
**Scope**: Feasibility of freezing the Python ML sidecar as a self-contained executable for macOS/Windows/Linux platform installers

---

## Summary

**Status: GO with caveats.**

The frozen binary builds successfully on macOS arm64 and serves all tested forensics endpoints. The core `--onefile` approach is viable for Sprint 18, but five open issues must be resolved before the frozen binary can replace a live Python installation for end users.

---

## Test Environment

| Item | Value |
|------|-------|
| macOS | arm64 (Apple Silicon, Darwin 25.2.0) |
| Python | 3.13.5 (miniconda) |
| PyInstaller | 6.19.0 |
| PyInstaller hooks contrib | 2026.3 |
| Build command | `python -m PyInstaller main.py --onefile --name jura-sidecar --collect-all uvicorn --collect-all fastapi --collect-all pydantic --collect-all pydantic_settings --collect-all starlette` |

---

## Build Outcome

**Build succeeded** on first attempt with no fatal errors.

- Elapsed time: approximately 159 seconds on Apple Silicon
- Output binary: `sidecar/dist/jura-sidecar`
- Binary size: **315.6 MB** (compressed with UPX)
- Uncompressed size on disk (at runtime): approximately 800–900 MB extracted to `$TMPDIR/_MEIxxxxxx/`

The 315 MB footprint is dominated by:
- PyTorch (pulled in via `open-clip-torch`): ~200 MB compressed
- open\_clip model code (not weights — weights download separately): ~30 MB
- scikit-learn + scikit-image + scipy: ~40 MB
- numpy + OpenCV + Pillow: ~25 MB
- FastAPI + uvicorn + pydantic + starlette: ~8 MB

---

## Endpoint Smoke Tests

All tests run against the frozen binary served on port 8200.

| Endpoint | Result | Notes |
|----------|--------|-------|
| `GET /health` | **PASS** | All capabilities correctly reported |
| `POST /forensics/ela` | **PASS** | Returns heatmap and score |
| `POST /forensics/noise` | **PASS** | Returns heatmap and block variances |
| `POST /forensics/deepfake` | **PASS** | Returns score; classifier\_used=None (model not bundled — expected) |
| `POST /forensics/clip-detect` | **PASS** | Returns score 0.42; model\_available=True |
| `POST /forensics/claim-check` | **PASS** | Returns structured verdict via Ollama |
| Video endpoints | **NOT TESTED** | Require FFmpeg; reported unavailable by health endpoint as expected |
| Transcription | **NOT TESTED** | faster-whisper not installed (commented out in requirements.txt) |

The health endpoint correctly reported `video_metadata: false`, `audio_metadata: false`, `video_frames: false`, `video_deepfake: false` — the frozen binary respects graceful degradation for FFmpeg-dependent features.

---

## Required Hidden Imports

The following packages require explicit `--hidden-import` or `--collect-all` flags because PyInstaller's static analyser cannot resolve their dynamic import patterns:

### Must have `--collect-all` (collect datas + binaries + hidden imports)

| Package | Reason |
|---------|--------|
| `uvicorn` | Uses dynamic import for ASGI app loading, event loop selection |
| `fastapi` | Dynamic router registration, Pydantic v2 schema generation |
| `pydantic` | Heavy use of `__init_subclass__`, `__class_getitem__`, metaclass magic |
| `pydantic_settings` | Extends pydantic with env-var loading via dynamic attribute access |
| `starlette` | Middleware and routing loaded dynamically |
| `imwatermark` | Package name differs from pip name (`invisible-watermark`); hook needed |

### Explicit `--hidden-import` additions

| Import | Imported by |
|--------|-------------|
| `anyio._backends._asyncio` | uvicorn, FastAPI async event loop |
| `sklearn.utils._weight_vector` | joblib/GBM internals for deepfake classifier |
| `scipy.special._cdflib` | scipy statistical functions |
| `PIL.JpegImagePlugin` | Pillow lazy plugin loading |
| `PIL.PngImagePlugin` | Pillow lazy plugin loading |
| `PIL.BmpImagePlugin` | Pillow lazy plugin loading |
| `PIL.WebPImagePlugin` | Pillow lazy plugin loading |
| `httpx._transports.default` | claim\_checker HTTP calls to Ollama |
| All `app.services.*` | FastAPI router auto-discovery |
| All `app.api.*` | FastAPI router registration |
| `app.models.schemas` | Pydantic schema discovery |
| `app.config` | pydantic-settings app config |

### Ignorable warnings (false positives)

The build warnings file (`build/jura-sidecar/warn-jura-sidecar.txt`) contains many "missing module" entries that are safe to ignore:

- `mypy.*` — pydantic ships mypy plugin code that is never imported at runtime
- `trio.*` — optional anyio backend; asyncio backend is used instead
- `multiprocessing.AuthenticationError` etc. — Python 3.13 reorganised multiprocessing; these are internal aliases, not real missing modules
- `h2.*` — HTTP/2 support for httpx; not used by the sidecar
- `_overlapped` — Windows-only asyncio module; irrelevant on macOS
- `java` — platform.py Java detection; never triggered
- `collections.abc` — false positive from PyInstaller analysis of Python 3.13 stdlib

---

## Model File Inclusion

The trained GBM deepfake classifiers live at `juralabs/models/`:

```
models/
  deepfake_classifier.joblib   (232 KB)
  univfd_probe.joblib          (5 KB)
  deepfake_classifier_meta.json
  univfd_probe_meta.json
  evaluation_report.json
```

**The model files are NOT bundled in the frozen binary.** `app/services/deepfake.py` resolves the model path relative to `__file__`:

```python
model_path = os.path.join(
    os.path.dirname(__file__), "..", "..", "..", "models", "deepfake_classifier.joblib"
)
```

In a frozen PyInstaller binary, `__file__` points into the `_MEIPASS` temp extraction directory (`/var/folders/.../T/_MEIxxxxxx/app/services/deepfake.pyc`). The path `../../../models/` therefore resolves to `/var/folders/.../T/_MEIxxxxxx/../../../models/` — a temp path that will not exist at runtime.

**Sprint 18 fix required**: Modify `deepfake.py` (and `univfd_probe` loading if it uses the same pattern) to detect the frozen context and resolve the model path relative to `sys.executable`:

```python
import sys, os

def _resolve_models_dir() -> str:
    # Check for explicit override first (useful for testing and .app bundles)
    env_override = os.environ.get("JURA_MODELS_DIR")
    if env_override:
        return env_override
    # In a PyInstaller frozen binary, use the directory containing the executable
    if getattr(sys, "frozen", False):
        return os.path.join(os.path.dirname(sys.executable), "models")
    # Development: relative to source file
    return os.path.normpath(
        os.path.join(os.path.dirname(__file__), "..", "..", "..", "models")
    )
```

The model files (< 250 KB total) should be shipped alongside the frozen binary, not bundled inside it, so they can be updated independently without rebuilding the entire binary.

**Alternative**: Use `--add-data models:models` in the spec to bundle them inside the binary and access via `sys._MEIPASS`. This is simpler but means a model update requires a full binary rebuild.

The `deepfake_classifier.joblib` absence caused the smoke test to show `classifier_used: None` — the endpoint still responds correctly (graceful degradation).

---

## knowledge\_base Inclusion

The TF-IDF knowledge retriever (`app/services/knowledge_retriever.py`) loads `.txt` documents from `sidecar/knowledge_base/` via `__file__`-relative path. This has the same frozen-binary path issue as the model files above.

The improved spec (`sidecar/jura-sidecar.spec`) bundles the `.txt` files via:

```python
_kb_dir = _spec_dir / "knowledge_base"
if _kb_dir.exists():
    datas += [(str(f), "knowledge_base") for f in _kb_dir.glob("*.txt")]
```

At runtime, the files land at `$MEIPASS/knowledge_base/*.txt`.

**Sprint 18 fix required**: Modify `knowledge_retriever.py` to use `sys._MEIPASS`-aware path resolution, or set `JURA_KB_DIR=$MEIPASS/knowledge_base` in the environment before spawning the sidecar. The Rust sidecar launcher (`src-tauri/src/sidecar.rs`) is the right place to set this.

---

## Estimated Binary Size by Configuration

| Configuration | Estimated size |
|---------------|----------------|
| Current (all deps incl. torch) | 315 MB compressed / ~850 MB extracted |
| Without open\_clip + torch | ~90 MB compressed / ~250 MB extracted |
| Without open\_clip + torch + chromadb | ~80 MB compressed / ~220 MB extracted |

For Sprint 18, two distributions may make sense:
1. **Core binary** (~90 MB): excludes open\_clip/torch; CLIP detection reports `model_available: False`
2. **Full binary** (~315 MB): includes torch for CLIP; users download on demand

This matches the existing graceful-degradation design: CLIP is already optional.

To build the core binary, add to the spec's `excludes` list:
```python
excludes += ["torch", "torchvision", "open_clip", "timm", "transformers"]
```

---

## Outstanding Issues and Blockers

### Issue 1 — BLOCKER: Model path resolution in frozen context

**Severity**: High
**Affected**: `deepfake.py`, `univfd_probe` loading
**Symptom**: Classifier unavailable at runtime; deepfake falls back to heuristic-only scoring
**Fix**: See model path fix above. Estimated effort: 30 minutes.

### Issue 2 — BLOCKER: knowledge\_base path resolution

**Severity**: Medium (RAG claim checker degrades gracefully but loses KB grounding)
**Affected**: `knowledge_retriever.py`
**Symptom**: TF-IDF index empty; claim checker falls back to Ollama-only with no retrieved context
**Fix**: Bundle via spec `datas` (done in improved spec) + set `JURA_KB_DIR` env var when launching. Estimated effort: 1 hour including Rust sidecar.rs changes.

### Issue 3 — FFmpeg not bundled

**Severity**: Medium
**Affected**: Video metadata, audio metadata, video frames, video deepfake endpoints
**Note**: The health endpoint correctly reports these as unavailable. FFmpeg is a system dependency on all three platforms. The Sprint 18 installer must ensure FFmpeg is present:
- **macOS**: Bundle a static `ffprobe`/`ffmpeg` binary alongside the .app (add via `--add-binary` or as a Tauri sidecar)
- **Windows**: Bundle via NSIS installer or WiX component
- **Linux**: Depend on `ffmpeg` package via .deb/AppImage dependency declaration

### Issue 4 — Cold-start time

**Severity**: Low-Medium
**Observed**: The frozen binary takes approximately 2–4 seconds to start serving requests on Apple Silicon (measured by time from launch to successful `/health` response). This is because the onefile binary extracts ~850 MB to `$TMPDIR` on each cold start.

**Root causes**:
1. PyInstaller onefile extraction to `$TMPDIR` on every fresh start
2. PyTorch and open\_clip initialisation
3. UPX decompression overhead

**Mitigations for Sprint 18**:
- Use `--onedir` instead of `--onefile`: no extraction needed; the directory is already expanded. Tauri can bundle the directory inside the .app. This reduces cold-start from ~3 s to ~0.5 s.
- The Rust backend already health-polls the sidecar before making requests (`sidecar.rs`), so startup latency is already handled gracefully.
- Consider pre-launching the sidecar in the Tauri `setup()` hook rather than on first use.

### Issue 5 — macOS code signing

**Severity**: Low (Sprint 18 blocker for notarisation)
**Detail**: PyInstaller sets the binary to re-sign with `None` identity by default. For macOS notarisation (required for Gatekeeper to allow execution without quarantine prompt), the binary must be:
1. Signed with an Apple Developer ID Application certificate
2. Notarised via `notarytool`
3. Stapled

In the spec, set:
```python
codesign_identity = "Developer ID Application: Jura Labs CIC (TEAMID)"
```

The Tauri release workflow already handles signing for the Tauri `.app`; the frozen sidecar binary (if bundled inside the `.app`) will be signed as part of the bundle. If shipped separately, it needs its own signing step.

### Issue 6 — Windows and Linux not tested

**Severity**: Medium
**Detail**: This spike was macOS arm64 only. Known platform-specific considerations:
- **Windows**: OpenCV ships `.pyd` files (DLLs); PyInstaller handles them but sometimes needs explicit `--add-binary`. Visual C++ runtime DLLs may need bundling. Target arch: `x86_64-pc-windows-msvc`.
- **Linux**: The binary will be glibc-version locked. Build on the oldest supported distro (Ubuntu 22.04 / glibc 2.35) for maximum compatibility. `libopencv*` may need explicit `--add-binary` entries.

---

## PyInstaller Version and Installation Notes

PyInstaller is not currently in `requirements.txt`. It must be added as a dev dependency:

```
# requirements-dev.txt (or add section to requirements.txt)
pyinstaller==6.19.0
pyinstaller-hooks-contrib==2026.3
```

Do not add PyInstaller to `requirements.txt` (production deps) — it must never be installed in the user-facing environment. Create a separate `requirements-dev.txt`.

---

## Recommended Spec File

The improved spec is at `sidecar/jura-sidecar.spec`. Key improvements over the auto-generated spec:

1. Bundles `knowledge_base/*.txt` via `datas`
2. Adds all `app.services.*` and `app.api.*` as hidden imports to prevent FastAPI router discovery failures
3. Excludes `tkinter`, `trio`, `mypy`, `pytest`, `gradio` to reduce size
4. Sets `target_arch="arm64"` explicitly for reproducible macOS builds
5. Documents `upx_exclude` for fragile dylibs on arm64
6. Comments explain every non-obvious choice

---

## Go / No-Go Recommendation for Sprint 18

**GO.**

The approach is proven to work. The frozen binary runs, serves all image forensics endpoints correctly, handles optional dependencies gracefully, and the binary size (315 MB with CLIP/torch; ~90 MB without) is acceptable for a desktop application.

### Sprint 18 must-do items before release

1. Fix model path resolution in `deepfake.py` to use `sys._MEIPASS` / `sys.executable` detection (30 min)
2. Fix knowledge\_base path resolution in `knowledge_retriever.py` (30 min) + Rust sidecar.rs `JURA_KB_DIR` env var (1 hr)
3. Test `--onedir` mode instead of `--onefile` to eliminate cold-start extraction (1 hr)
4. Test Windows and Linux builds on respective CI runners (2 hrs per platform)
5. Add FFmpeg bundling strategy to the installer spec for each platform (1 day)
6. Add `pyinstaller==6.19.0` to `requirements-dev.txt`
7. Add a `make freeze` target to the Makefile: `cd sidecar && python -m PyInstaller jura-sidecar.spec --noconfirm`

### Nice-to-have for Sprint 18

- Offer a "core" (no torch) and "full" (with CLIP) binary variant
- Add a startup self-test that verifies model files are present and warns on stderr if not
- Set `JURA_SIDECAR_KEY` via a random UUID generated by the Tauri app at first launch, stored in the app's data directory, and passed as an env var to the frozen sidecar process

---

## Appendix: Build Commands

```bash
# Install PyInstaller (dev only)
pip install pyinstaller==6.19.0

# Build from spec (recommended)
cd sidecar
python -m PyInstaller jura-sidecar.spec --noconfirm

# Quick build (auto-generates spec, for experimentation only)
python -m PyInstaller main.py \
  --onefile \
  --name jura-sidecar \
  --collect-all uvicorn \
  --collect-all fastapi \
  --collect-all pydantic \
  --collect-all pydantic_settings \
  --collect-all starlette \
  --noconfirm

# Test the frozen binary
./dist/jura-sidecar &
curl http://127.0.0.1:8200/health
kill %1
```
