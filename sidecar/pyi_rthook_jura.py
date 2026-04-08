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
