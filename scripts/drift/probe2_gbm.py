# SPDX-License-Identifier: AGPL-3.0-or-later

"""Probe 2, GBM feature vector via the sidecar's own extractor.

Uses app.services.deepfake.extract_features_for_training, which runs the
identical extractor sequence to _perform_deepfake_detection_impl (lines
996-1030) minus the size guard and screenshot pre-classifier, followed by
extract_feature_vector (FEATURE_NAMES order, 84 values). NaN -> 0.0 as the
impl does at line 1043. Raw floats (NaN preserved) saved to .npy.
"""

import os
import sys

import numpy as np

from probe_common import (
    MIME,
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
    from app.services import deepfake as df  # noqa: PLC0415

    golden = load_golden()
    n = len(df.FEATURE_NAMES)
    feats = np.full((len(golden), n), np.nan, dtype=np.float64)
    res = {
        "versions": versions(),
        "plugins": plug,
        "feature_names": df.FEATURE_NAMES,
        "n_features": n,
        "images": {},
        "order": [e["id"] for e in golden],
    }
    with WarnCapture() as wc:
        for i, e in enumerate(golden):
            rec = {"format": e["format"], "mime_type": MIME[e["format"]]}
            try:
                with open(e["path"], "rb") as fh:
                    b = fh.read()
                features, codec = df.extract_features_for_training(
                    b, mime_type=MIME[e["format"]], has_camera_exif=False
                )
                vec = df.extract_feature_vector(features)
                raw = np.asarray(vec, dtype=np.float64)
                feats[i] = raw
                clean = np.where(np.isnan(raw), 0.0, raw)
                rec["codec_class"] = codec
                rec["n_nan"] = int(np.isnan(raw).sum())
                rec["vec_clean_sha256"] = sha_array(clean)
                rec["vec80_clean_sha256"] = sha_array(clean[:80])
                h, signals = df._heuristic_score(features, codec, has_camera_exif=False)
                rec["heuristic_score"] = float(h)
                rec["heuristic_signals_triggered"] = sorted(
                    s.name for s in signals if s.triggered
                )
            except Exception as exc:  # noqa: BLE001
                rec["error"] = f"{type(exc).__name__}: {exc}"
            res["images"][e["id"]] = rec
    res["warnings"] = wc.unique()
    d = out_dir()
    np.save(os.path.join(d, "probe2_features.npy"), feats)
    write_json(os.path.join(d, "probe2_gbm.json"), res)
    errs = sum(1 for r in res["images"].values() if "error" in r)
    print(f"probe2 done: {len(golden)} images, {errs} errors -> {d}")


if __name__ == "__main__":
    sys.exit(main())
