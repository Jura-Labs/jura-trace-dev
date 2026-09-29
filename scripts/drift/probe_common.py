# SPDX-License-Identifier: AGPL-3.0-or-later

"""Shared harness for the five-bump drift probes.

Sets up the sidecar import path the same way the sidecar itself runs (cwd =
sidecar/, `app.*` importable), points JURA_MODELS_DIR at the canonical
models/ directory, and registers the format plugins in the same order as
sidecar/main.py's startup hook (pillow_heif.register_heif_opener() first,
then `import pillow_avif`).
"""

import hashlib
import json
import os
import sys
import warnings

# The repository is the one this script sits in (scripts/drift/), so a run
# from a worktree tests that worktree. Generated files (golden variants,
# results, the per-run golden set with its real paths, the virtual
# environments) go to a work directory outside the repository: they are
# derived from the corpus, which is not published.
REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
DRIFT = os.path.abspath(
    os.path.expanduser(os.environ.get("JURA_DRIFT_WORK", "~/.cache/jura-drift"))
)
HERE = os.path.dirname(os.path.abspath(__file__))
MANIFEST = os.path.join(HERE, "golden_manifest.json")
SIDECAR = os.path.join(REPO, "sidecar")
MODELS = os.path.join(REPO, "models")

if SIDECAR not in sys.path:
    sys.path.insert(0, SIDECAR)
os.environ.setdefault("JURA_MODELS_DIR", MODELS)

MIME = {
    "PNG": "image/png",
    "JPEG": "image/jpeg",
    "AVIF": "image/avif",
    "HEIC": "image/heic",
    "WEBP": "image/webp",
}


def register_plugins() -> dict:
    """Mirror sidecar/main.py lines 46-68."""
    info = {}
    import pillow_heif  # noqa: PLC0415

    pillow_heif.register_heif_opener()
    info["pillow_heif"] = pillow_heif.__version__
    import pillow_avif  # noqa: F401,PLC0415

    info["pillow_avif"] = getattr(pillow_avif, "__version__", "?")
    from PIL import Image, features  # noqa: PLC0415

    info["PIL_native_avif"] = bool(features.check("avif"))
    info["ext_avif"] = Image.registered_extensions().get(".avif")
    info["ext_heic"] = Image.registered_extensions().get(".heic")
    return info


def versions() -> dict:
    import numpy, scipy, sklearn, cv2, PIL  # noqa: E401,PLC0415

    v = {
        "python": sys.version.split()[0],
        "numpy": numpy.__version__,
        "scipy": scipy.__version__,
        "scikit-learn": sklearn.__version__,
        "opencv": cv2.__version__,
        "Pillow": PIL.__version__,
    }
    try:
        import onnxruntime  # noqa: PLC0415

        v["onnxruntime"] = onnxruntime.__version__
    except ImportError:
        v["onnxruntime"] = None
    try:
        import skimage  # noqa: PLC0415

        v["scikit-image"] = skimage.__version__
    except ImportError:
        v["scikit-image"] = None
    return v


def env_name() -> str:
    # The environment's name is its directory under $JURA_DRIFT_WORK/envs/,
    # so a new single-variable environment (onnxruntime_only, say) needs a
    # requirements file in envs/ and nothing here.
    prefix = os.path.abspath(sys.prefix)
    if os.path.basename(os.path.dirname(prefix)) == "envs":
        return os.path.basename(prefix)
    raise SystemExit(f"cannot infer env from {prefix}: run with $JURA_DRIFT_WORK/envs/<name>/bin/python")


def out_dir(run_tag: str | None = None) -> str:
    name = env_name()
    if run_tag:
        name = f"{name}_{run_tag}"
    d = os.path.join(DRIFT, "results", name)
    os.makedirs(d, exist_ok=True)
    return d


def load_golden() -> list[dict]:
    with open(os.path.join(DRIFT, "golden_set.json")) as fh:
        g = json.load(fh)
    imgs = sorted(g["images"], key=lambda e: e["id"])
    if len(imgs) != 84:
        raise SystemExit(f"golden set has {len(imgs)} images, expected 84")
    return imgs


def sha_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def sha_array(arr) -> str:
    import numpy as np  # noqa: PLC0415

    a = np.ascontiguousarray(arr)
    h = hashlib.sha256()
    h.update(str(a.dtype).encode())
    h.update(str(a.shape).encode())
    h.update(a.tobytes())
    return h.hexdigest()


def sha_str(s: str) -> str:
    return hashlib.sha256(s.encode()).hexdigest()


def write_json(path: str, obj) -> None:
    with open(path, "w") as fh:
        json.dump(obj, fh, indent=1, sort_keys=True, default=_default)


def _default(o):
    import numpy as np  # noqa: PLC0415

    if isinstance(o, (np.floating,)):
        return float(o)
    if isinstance(o, (np.integer,)):
        return int(o)
    if isinstance(o, np.ndarray):
        return o.tolist()
    return str(o)


class WarnCapture:
    """Record every warning verbatim (category, message, filename:lineno)."""

    def __init__(self):
        self.records = []

    def __enter__(self):
        self._cm = warnings.catch_warnings(record=True)
        self._list = self._cm.__enter__()
        warnings.simplefilter("always")
        return self

    def __exit__(self, *exc):
        for w in self._list:
            self.records.append(
                {
                    "category": w.category.__name__,
                    "message": str(w.message),
                    "where": f"{w.filename}:{w.lineno}",
                }
            )
        self._cm.__exit__(*exc)
        return False

    def unique(self) -> list[dict]:
        seen = {}
        for r in self.records:
            key = (r["category"], r["message"])
            if key not in seen:
                seen[key] = dict(r, count=0)
            seen[key]["count"] += 1
        return list(seen.values())
