# SPDX-License-Identifier: AGPL-3.0-or-later

"""Probe 4, UnivFD: preprocess array hash, CLIP embedding, probe score,
and the full perform_clip_detection response. Uses the sidecar's own
clip_detector functions (SHA-256 gate on univfd_probe.joblib at
clip_detector.py:60)."""

import os
import sys

import numpy as np
from PIL import Image

from probe_common import (
    WarnCapture,
    load_golden,
    out_dir,
    register_plugins,
    sha_array,
    versions,
    write_json,
)


def main() -> None:
    plug = register_plugins()
    from app.services import clip_detector as cd  # noqa: PLC0415

    golden = load_golden()
    d = out_dir()
    res = {"versions": versions(), "plugins": plug, "images": {}}

    with WarnCapture() as wc:
        res["onnx_available"] = bool(cd._ensure_model())
    res["ensure_model_warnings"] = wc.unique()
    with WarnCapture() as wc:
        probe = cd._load_univfd_probe()
    res["probe_load_warnings"] = wc.unique()
    res["probe_loaded"] = probe is not None
    if probe is not None:
        res["probe_type"] = f"{type(probe).__module__}.{type(probe).__name__}"
        res["probe_pickled_sklearn_version"] = getattr(probe, "_sklearn_version", None)
        res["probe_n_features_in_"] = int(getattr(probe, "n_features_in_", -1))

    emb = np.zeros((len(golden), 512), dtype=np.float32)
    scores = np.full((len(golden),), np.nan, dtype=np.float64)
    with WarnCapture() as wc:
        for i, e in enumerate(golden):
            rec = {"format": e["format"]}
            try:
                with Image.open(e["path"]) as im:
                    im.load()
                    arr = cd._preprocess_image(im)
                    rec["clip_input_sha256"] = sha_array(arr)
                    rec["clip_input_shape"] = list(arr.shape)
                    if res["onnx_available"]:
                        v = cd._encode_image(im)
                        emb[i] = v
                        rec["embedding_sha256"] = sha_array(v)
                        if probe is not None:
                            s = cd._score_univfd_probe(im)
                            scores[i] = s if s is not None else np.nan
                            rec["univfd_score"] = s
                with open(e["path"], "rb") as fh:
                    b = fh.read()
                resp = cd.perform_clip_detection(b)
                dump = resp.model_dump()
                dump.pop("verdict_thresholds", None)
                rec["clip_detection"] = dump
            except Exception as exc:  # noqa: BLE001
                rec["error"] = f"{type(exc).__name__}: {exc}"
            res["images"][e["id"]] = rec
    res["warnings"] = wc.unique()
    np.save(os.path.join(d, "probe4_embeddings.npy"), emb)
    np.save(os.path.join(d, "probe4_univfd_scores.npy"), scores)
    write_json(os.path.join(d, "probe4_univfd.json"), res)
    errs = sum(1 for r in res["images"].values() if "error" in r)
    print(
        f"probe4 done: onnx={res['onnx_available']} probe={res['probe_loaded']} "
        f"errors={errs} -> {d}"
    )


if __name__ == "__main__":
    sys.exit(main())
