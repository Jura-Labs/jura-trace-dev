# SPDX-License-Identifier: AGPL-3.0-or-later

"""Probe 3, classifier load + predict.

1. _load_classifier() via the sidecar (SHA-256 gate at deepfake.py:241),
   warnings captured verbatim.
2. predict_proba on the BASELINE feature vectors (results/baseline/
   probe2_features.npy), NaN -> 0, trimmed to clf.n_features_in_, called
   row by row exactly as deepfake.py:1052 does (`clf.predict_proba([vec])`).
3. predict_proba on THIS env's own feature vectors (end-to-end GBM number).
4. perform_deepfake_detection_with_features end-to-end per image (what a
   user sees), heatmap replaced by its hash.
"""

import os
import sys

import numpy as np

from probe_common import (
    DRIFT,
    MIME,
    WarnCapture,
    load_golden,
    out_dir,
    register_plugins,
    sha_array,
    sha_str,
    versions,
    write_json,
)


def _predict_rows(clf, X: np.ndarray) -> np.ndarray:
    expected = getattr(clf, "n_features_in_", X.shape[1])
    out = np.zeros((X.shape[0], 2), dtype=np.float64)
    for i in range(X.shape[0]):
        vec = [0.0 if np.isnan(v) else float(v) for v in X[i]]
        if expected < len(vec):
            vec = vec[:expected]
        out[i] = clf.predict_proba([vec])[0]
    return out


def main() -> None:
    plug = register_plugins()
    from app.services import deepfake as df  # noqa: PLC0415

    golden = load_golden()
    d = out_dir()
    res = {"versions": versions(), "plugins": plug}

    with WarnCapture() as wc:
        clf = df._load_classifier()
    res["load_warnings"] = wc.unique()
    if clf is None:
        # _load_classifier swallows the exception (bare except at deepfake.py
        # 285), so re-raise it here verbatim for the record, then still run
        # the end-to-end path: that is what a user would see with the GBM
        # silently absent.
        res["loaded"] = False
        import joblib, traceback  # noqa: E401,PLC0415

        try:
            joblib.load(os.path.join(df.MODELS_DIR, "deepfake_classifier.joblib"))
        except Exception:  # noqa: BLE001
            res["load_error_traceback"] = traceback.format_exc()
        res["images"] = {}
        with WarnCapture() as wc:
            for e in golden:
                rec = res["images"].setdefault(e["id"], {})
                try:
                    with open(e["path"], "rb") as fh:
                        b = fh.read()
                    resp, _feats = df.perform_deepfake_detection_with_features(
                        b, mime_type=MIME[e["format"]], has_camera_exif=False
                    )
                    dump = resp.model_dump()
                    dump["heatmap_sha256"] = sha_str(dump.pop("heatmap_base64", ""))
                    dump["signals"] = sorted(s["name"] for s in dump["signals"] if s["triggered"])
                    dump.pop("verdict_thresholds", None)
                    rec["e2e"] = dump
                except Exception as exc:  # noqa: BLE001
                    rec["e2e_error"] = f"{type(exc).__name__}: {exc}"
        res["e2e_warnings"] = wc.unique()
        write_json(os.path.join(d, "probe3_clf.json"), res)
        print("probe3: classifier did NOT load; e2e recorded without GBM")
        return
    res["loaded"] = True
    res["clf_type"] = f"{type(clf).__module__}.{type(clf).__name__}"
    res["n_features_in_"] = int(getattr(clf, "n_features_in_", -1))
    res["pickled_sklearn_version"] = getattr(clf, "_sklearn_version", None)
    for attr in ("n_estimators", "n_estimators_", "learning_rate", "max_depth", "max_iter"):
        if hasattr(clf, attr):
            res[attr] = getattr(clf, attr)

    # (2) baseline feature vectors -> proba under this env's sklearn
    base_path = os.path.join(DRIFT, "results", "baseline", "probe2_features.npy")
    if not os.path.exists(base_path):
        raise SystemExit("run probe2 in the baseline env first")
    Xb = np.load(base_path)
    with WarnCapture() as wc:
        Pb = _predict_rows(clf, Xb)
    res["predict_warnings_baseline_feats"] = wc.unique()
    np.save(os.path.join(d, "probe3_proba_on_baseline_feats.npy"), Pb)

    # (3) own env's feature vectors -> proba
    own_path = os.path.join(d, "probe2_features.npy")
    Xo = np.load(own_path)
    Po = _predict_rows(clf, Xo)
    np.save(os.path.join(d, "probe3_proba_on_own_feats.npy"), Po)

    res["images"] = {}
    for i, e in enumerate(golden):
        res["images"][e["id"]] = {
            "proba_on_baseline_feats": Pb[i].tolist(),
            "proba_on_baseline_feats_sha256": sha_array(Pb[i]),
            "proba_on_own_feats": Po[i].tolist(),
            "proba_on_own_feats_sha256": sha_array(Po[i]),
        }

    # (4) end-to-end
    with WarnCapture() as wc:
        for e in golden:
            rec = res["images"][e["id"]]
            try:
                with open(e["path"], "rb") as fh:
                    b = fh.read()
                resp, _feats = df.perform_deepfake_detection_with_features(
                    b, mime_type=MIME[e["format"]], has_camera_exif=False
                )
                dump = resp.model_dump()
                dump["heatmap_sha256"] = sha_str(dump.pop("heatmap_base64", ""))
                dump["signals"] = sorted(s["name"] for s in dump["signals"] if s["triggered"])
                dump.pop("verdict_thresholds", None)
                rec["e2e"] = dump
            except Exception as exc:  # noqa: BLE001
                rec["e2e_error"] = f"{type(exc).__name__}: {exc}"
    res["e2e_warnings"] = wc.unique()
    write_json(os.path.join(d, "probe3_clf.json"), res)
    print(
        f"probe3 done: loaded={res['loaded']} type={res['clf_type']} "
        f"n_in={res['n_features_in_']} pickled_sk={res['pickled_sklearn_version']} "
        f"load_warnings={len(res['load_warnings'])} -> {d}"
    )


if __name__ == "__main__":
    sys.exit(main())
