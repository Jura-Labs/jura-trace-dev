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

# Stub imwatermark.rivaGan to avoid the torch dependency.
#
# invisible-watermark's package __init__.py eagerly imports
# `from .rivaGan import RivaWatermark`, and rivaGan.py does `import torch`.
# Torch is intentionally excluded from this bundle (the sidecar excludes
# list in jura-sidecar.spec; the bundle would be 1.5 GB+ otherwise).
# Without this stub, `from imwatermark import WatermarkEncoder` fails
# at runtime with "library not installed" and the Tauri watermark embed
# path returns success=false.
#
# We only use the dwtDctSvd method (DWT-DCT-SVD watermarking via SVD
# decomposition). rivaGan is the deep-learning watermark method which
# we never call. Stubbing it at sys.modules level lets the package
# __init__ succeed without ever evaluating rivaGan.py.
#
# Must run before any application code imports imwatermark.
import types

_riva_stub = types.ModuleType("imwatermark.rivaGan")


class _RivaWatermarkStub:
    """Inert placeholder for the rivaGan watermark method.

    The application code path never instantiates this class. Present only
    so `from .rivaGan import RivaWatermark` in imwatermark/watermark.py
    succeeds during package import.
    """


_riva_stub.RivaWatermark = _RivaWatermarkStub
sys.modules["imwatermark.rivaGan"] = _riva_stub
