# SPDX-License-Identifier: AGPL-3.0-or-later

"""Compare baseline vs candidate probe outputs; write comparison.json and
print the N/84 table. Also compares baseline vs baseline_run2 for the
unseeded copy-move path."""

import json
import os

import numpy as np

from probe_common import DRIFT  # noqa: E402
R = os.path.join(DRIFT, "results")


def load(env, name):
    p = os.path.join(R, env, name)
    if not os.path.exists(p):
        return None
    with open(p) as fh:
        return json.load(fh)


def npy(env, name):
    p = os.path.join(R, env, name)
    return np.load(p) if os.path.exists(p) else None


def get(d, path):
    for k in path:
        if d is None:
            return None
        d = d.get(k) if isinstance(d, dict) else None
    return d


def compare_field(a, b, path, label, out, ids):
    ident, diff, missing = 0, [], []
    for i in ids:
        va = get(a["images"].get(i), path)
        vb = get(b["images"].get(i), path)
        if va is None or vb is None:
            missing.append(i)
        elif va == vb:
            ident += 1
        else:
            diff.append(i)
    out[label] = {"identical": ident, "of": len(ids), "differ": diff, "missing": missing}


def array_delta(A, B, ids, label, out, names=None):
    if A is None or B is None:
        out[label] = {"error": "array missing"}
        return
    both_nan = np.isnan(A) & np.isnan(B)
    one_nan = np.isnan(A) ^ np.isnan(B)
    eq = (A == B) | both_nan
    rows_ident = int(eq.all(axis=1).sum()) if A.ndim == 2 else int(eq.sum())
    d = np.abs(A - B)
    d[both_nan] = 0.0
    d[one_nan] = 0.0
    finite_max = float(d.max()) if d.size else 0.0
    entry = {"identical_rows": rows_ident, "of": len(ids), "max_abs_delta": finite_max}
    if one_nan.any():
        rows = np.where(one_nan.any(axis=1))[0] if A.ndim == 2 else np.where(one_nan)[0]
        entry["nan_mismatch_images"] = [ids[i] for i in rows]
        entry["note"] = "one env produced NaN (e.g. decode failure) where the other produced a value; excluded from max_abs_delta"
    if A.ndim == 2:
        bad_rows = np.where(~eq.all(axis=1))[0]
        entry["differing_images"] = [ids[i] for i in bad_rows]
        bad_cols = np.where(~eq.all(axis=0))[0]
        entry["differing_feature_indices"] = [int(c) for c in bad_cols]
        if names is not None:
            entry["differing_feature_names"] = [names[c] for c in bad_cols]
        entry["max_abs_delta_per_index"] = {int(c): float(np.nanmax(d[:, c])) for c in bad_cols}
        per_img = {}
        for i in bad_rows:
            per_img[ids[i]] = {"max_abs_delta": float(np.nanmax(d[i])), "argmax_index": int(np.nanargmax(d[i]))}
        entry["per_image"] = per_img
        # relative delta, ignoring zeros
        with np.errstate(divide="ignore", invalid="ignore"):
            rel = d / np.maximum(np.abs(A), 1e-300)
        rel[eq] = 0.0
        entry["max_rel_delta"] = float(np.nanmax(rel)) if rel.size else 0.0
    else:
        bad = np.where(~eq)[0]
        entry["differing_images"] = [ids[i] for i in bad]
        entry["per_image"] = {ids[i]: float(d[i]) for i in bad}
    out[label] = entry


def main():
    g = json.load(open(os.path.join(DRIFT, "golden_set.json")))
    ids = sorted(e["id"] for e in g["images"])
    C = {"n_images": len(ids), "probes": {}}

    # Probe 1
    a, b = load("baseline", "probe1_decode.json"), load("candidate", "probe1_decode.json")
    p1 = {}
    if a and b:
        compare_field(a, b, ["rgb_sha256"], "decoded_rgb_sha256", p1, ids)
        compare_field(a, b, ["decoder_class"], "decoder_class", p1, ids)
        compare_field(a, b, ["reported_format"], "reported_format", p1, ids)
        p1["errors"] = {e: [i for i, r in x["images"].items() if "error" in r] for e, x in (("baseline", a), ("candidate", b))}
        p1["plugins"] = {"baseline": a["plugins"], "candidate": b["plugins"]}
        p1["warnings"] = {"baseline": a["warnings"], "candidate": b["warnings"]}
    C["probes"]["p1_decode"] = p1

    # Probe 2
    a, b = load("baseline", "probe2_gbm.json"), load("candidate", "probe2_gbm.json")
    p2 = {}
    if a and b:
        compare_field(a, b, ["vec_clean_sha256"], "gbm_vector_84_sha256", p2, ids)
        compare_field(a, b, ["vec80_clean_sha256"], "gbm_vector_80_sha256", p2, ids)
        compare_field(a, b, ["codec_class"], "codec_class", p2, ids)
        compare_field(a, b, ["heuristic_score"], "heuristic_score", p2, ids)
        compare_field(a, b, ["heuristic_signals_triggered"], "heuristic_signals_triggered", p2, ids)
        order = a["order"]
        assert order == b["order"] == ids
        array_delta(npy("baseline", "probe2_features.npy"), npy("candidate", "probe2_features.npy"), ids, "gbm_features_delta", p2, a["feature_names"])
        p2["errors"] = {e: [i for i, r in x["images"].items() if "error" in r] for e, x in (("baseline", a), ("candidate", b))}
        p2["warnings"] = {"baseline": a["warnings"], "candidate": b["warnings"]}
    C["probes"]["p2_gbm_features"] = p2

    # Probe 3
    a, b = load("baseline", "probe3_clf.json"), load("candidate", "probe3_clf.json")
    p3 = {}
    if a and b:
        for k in ("loaded", "clf_type", "n_features_in_", "pickled_sklearn_version", "load_warnings", "predict_warnings_baseline_feats", "e2e_warnings"):
            p3[k] = {"baseline": a.get(k), "candidate": b.get(k)}
        p3["candidate_load_error_traceback"] = b.get("load_error_traceback")
        p3["classifier_loaded"] = {"identical": int(bool(a.get("loaded")) == bool(b.get("loaded"))) * len(ids), "of": len(ids), "differ": [] if a.get("loaded") == b.get("loaded") else ["ALL (baseline loaded=%s, candidate loaded=%s)" % (a.get("loaded"), b.get("loaded"))], "missing": []}
        if a.get("loaded") and b.get("loaded"):
            compare_field(a, b, ["proba_on_baseline_feats_sha256"], "predict_proba_on_baseline_features", p3, ids)
            compare_field(a, b, ["proba_on_own_feats_sha256"], "predict_proba_on_own_env_features", p3, ids)
            array_delta(npy("baseline", "probe3_proba_on_baseline_feats.npy"), npy("candidate", "probe3_proba_on_baseline_feats.npy"), ids, "proba_baseline_feats_delta", p3)
            array_delta(npy("baseline", "probe3_proba_on_own_feats.npy"), npy("candidate", "probe3_proba_on_own_feats.npy"), ids, "proba_own_feats_delta", p3)
        for f in ("score", "classifier_score", "classifier_available", "univfd_score", "verdict_level", "suspicious", "signals", "heatmap_sha256"):
            compare_field(a, b, ["e2e", f], f"e2e_{f}", p3, ids)
        # end-to-end score shift when GBM is absent
        shifts = {}
        for i in ids:
            sa, sb = get(a["images"].get(i), ["e2e", "score"]), get(b["images"].get(i), ["e2e", "score"])
            va, vb = get(a["images"].get(i), ["e2e", "verdict_level"]), get(b["images"].get(i), ["e2e", "verdict_level"])
            if sa is not None and sb is not None and (sa != sb or va != vb):
                shifts[i] = {"baseline": sa, "candidate": sb, "delta": round(sb - sa, 6), "verdict_baseline": va, "verdict_candidate": vb}
        p3["e2e_score_shifts"] = shifts
        if shifts:
            ds = [v["delta"] for v in shifts.values()]
            p3["e2e_score_shift_summary"] = {"n_images_shifted": len(shifts), "max_abs_delta": max(abs(x) for x in ds), "mean_delta": sum(ds) / len(ds), "n_verdict_changed": sum(1 for v in shifts.values() if v["verdict_baseline"] != v["verdict_candidate"])}
        p3["e2e_errors"] = {e: [i for i, r in x["images"].items() if "e2e_error" in r] for e, x in (("baseline", a), ("candidate", b))}
    C["probes"]["p3_classifier"] = p3

    # Probe 4
    a, b = load("baseline", "probe4_univfd.json"), load("candidate", "probe4_univfd.json")
    p4 = {}
    if a and b:
        for k in ("onnx_available", "probe_loaded", "probe_type", "probe_pickled_sklearn_version", "probe_load_warnings", "ensure_model_warnings", "warnings"):
            p4[k] = {"baseline": a.get(k), "candidate": b.get(k)}
        compare_field(a, b, ["clip_input_sha256"], "clip_input_array_sha256", p4, ids)
        compare_field(a, b, ["embedding_sha256"], "clip_embedding_sha256", p4, ids)
        compare_field(a, b, ["univfd_score"], "univfd_probe_score", p4, ids)
        for f in ("score", "univfd_score", "verdict_level", "class_probabilities", "suspicious"):
            compare_field(a, b, ["clip_detection", f], f"clip_detection_{f}", p4, ids)
        array_delta(npy("baseline", "probe4_embeddings.npy"), npy("candidate", "probe4_embeddings.npy"), ids, "embedding_delta", p4)
        array_delta(npy("baseline", "probe4_univfd_scores.npy"), npy("candidate", "probe4_univfd_scores.npy"), ids, "univfd_score_delta", p4)
        p4["errors"] = {e: [i for i, r in x["images"].items() if "error" in r] for e, x in (("baseline", a), ("candidate", b))}
    C["probes"]["p4_univfd"] = p4

    # Probe 5 (baseline vs candidate, and baseline vs baseline_run2)
    def p5_compare(x, y, out):
        for f in ("matched_pairs", "score", "suspicious", "clone_regions", "visualisation_sha256"):
            compare_field(x, y, ["copy_move", f], f"copy_move_{f}", out, ids)
        for f in ("n_keypoints", "descriptors_sha256", "keypoints_sha256"):
            compare_field(x, y, ["sift", f], f"sift_{f}", out, ids)
        cv_keys = sorted(next(iter(x["images"].values())).get("cv", {}).keys())
        for k in cv_keys:
            compare_field(x, y, ["cv", k], f"cv_{k}", out, ids)
        for f in ("score", "suspicious", "hv_correlation", "diff_variance_ratio", "hf_energy_ratio", "heatmap_sha256"):
            compare_field(x, y, ["npr", f], f"npr_{f}", out, ids)
        compare_field(x, y, ["npr_raw_stats_sha256"], "npr_raw_kurtosis_skew_stats_sha256", out, ids)
        for f in ("n_peaks", "spectral_peaks", "has_periodic_artefacts", "model_attribution", "confidence", "spectrum_sha256", "error"):
            compare_field(x, y, ["gan", f], f"gan_{f}", out, ids)
        for f in ("imdecode", "avg_magnitude_sha256", "radial_profile_sha256", "lstsq_coeffs_sha256", "fitted_sha256"):
            compare_field(x, y, ["gan_raw", f], f"gan_raw_{f}", out, ids)

    a, b, a2 = load("baseline", "probe5_cv.json"), load("candidate", "probe5_cv.json"), load("baseline_run2", "probe5_cv.json")
    p5 = {}
    if a and b:
        vs = {}
        p5_compare(a, b, vs)
        array_delta(npy("baseline", "probe5_npr_raw_stats.npy"), npy("candidate", "probe5_npr_raw_stats.npy"), ids, "npr_raw_stats_delta", vs)
        array_delta(npy("baseline", "probe5_gan_lstsq_coeffs.npy"), npy("candidate", "probe5_gan_lstsq_coeffs.npy"), ids, "gan_lstsq_coeffs_delta", vs)
        # copy-move numeric deltas
        cmd = {}
        for i in ids:
            xa, xb = get(a["images"].get(i), ["copy_move"]), get(b["images"].get(i), ["copy_move"])
            if xa and xb and (xa["score"] != xb["score"] or xa["matched_pairs"] != xb["matched_pairs"]):
                cmd[i] = {"baseline": {"matched_pairs": xa["matched_pairs"], "score": xa["score"], "regions": len(xa["clone_regions"])}, "candidate": {"matched_pairs": xb["matched_pairs"], "score": xb["score"], "regions": len(xb["clone_regions"])}}
        vs["copy_move_numeric_deltas"] = cmd
        vs["gan_imdecode_failures"] = {e: [i for i, r in x["images"].items() if r.get("gan_raw", {}).get("imdecode") is None] for e, x in (("baseline", a), ("candidate", b))}
        vs["errors"] = {e: [i for i, r in x["images"].items() if "error" in r] for e, x in (("baseline", a), ("candidate", b))}
        vs["warnings"] = {"baseline": a["warnings"], "candidate": b["warnings"]}
        p5["baseline_vs_candidate"] = vs
    if a and a2:
        rr = {}
        for f in ("matched_pairs", "score", "suspicious", "clone_regions", "visualisation_sha256"):
            compare_field(a, a2, ["copy_move", f], f"copy_move_{f}", rr, ids)
        for f in ("n_keypoints", "descriptors_sha256"):
            compare_field(a, a2, ["sift", f], f"sift_{f}", rr, ids)
        cmd = {}
        for i in ids:
            xa, xb = get(a["images"].get(i), ["copy_move"]), get(a2["images"].get(i), ["copy_move"])
            if xa and xb and (xa["score"] != xb["score"] or xa["matched_pairs"] != xb["matched_pairs"]):
                cmd[i] = {"run1": {"matched_pairs": xa["matched_pairs"], "score": xa["score"]}, "run2": {"matched_pairs": xb["matched_pairs"], "score": xb["score"]}}
        rr["copy_move_numeric_deltas"] = cmd
        p5["baseline_run1_vs_run2"] = rr
    C["probes"]["p5_opencv_npr_gan"] = p5

    with open(os.path.join(DRIFT, "comparison.json"), "w") as fh:
        json.dump(C, fh, indent=1, sort_keys=True)

    # print table
    def rows(d, prefix=""):
        for k, v in d.items():
            if isinstance(v, dict) and "identical" in v and "of" in v:
                flag = "" if v["identical"] == v["of"] and not v["missing"] else "  <-- " + (f"differ={len(v['differ'])}" if v["differ"] else "") + (f" missing={len(v['missing'])}" if v["missing"] else "")
                print(f"{prefix}{k:55s} {v['identical']:3d} / {v['of']}{flag}")
            elif isinstance(v, dict) and "identical_rows" in v:
                print(f"{prefix}{k:55s} {v['identical_rows']:3d} / {v['of']}  max_abs_delta={v['max_abs_delta']:.3e}")
    for pk, pv in C["probes"].items():
        print(f"== {pk}")
        if pk == "p5_opencv_npr_gan":
            for sub, sv in pv.items():
                print(f"  -- {sub}")
                rows(sv, "  ")
        else:
            rows(pv)


if __name__ == "__main__":
    main()
