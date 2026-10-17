# SPDX-License-Identifier: AGPL-3.0-or-later

"""Attribution: compare each single-bump env against baseline on the probes
that were run there, and write attribution.json."""

import json
import os

import numpy as np

from probe_common import DRIFT  # noqa: E402
R = os.path.join(DRIFT, "results")
ENVS = ["candidate", "numpy_only", "scipy_only", "opencv_only", "sklearn_only"]


def load(env, name):
    p = os.path.join(R, env, name)
    return json.load(open(p)) if os.path.exists(p) else None


def npy(env, name):
    p = os.path.join(R, env, name)
    return np.load(p) if os.path.exists(p) else None


def feat_delta(A, B, names):
    both_nan = np.isnan(A) & np.isnan(B)
    d = np.abs(A - B)
    d[both_nan] = 0
    eq = (A == B) | both_nan
    cols = np.where(~eq.all(axis=0))[0]
    return {
        "identical_images": int(eq.all(axis=1).sum()),
        "n_indices_differ": int(len(cols)),
        "max_abs_delta": float(d.max()),
        "indices_gt_1e-9": [names[c] for c in cols if d[:, c].max() > 1e-9],
        "max_per_index_gt_1e-9": {names[c]: float(d[:, c].max()) for c in cols if d[:, c].max() > 1e-9},
        "indices_ulp_only": [names[c] for c in cols if d[:, c].max() <= 1e-9],
    }


def field_ident(a, b, path):
    n = 0
    tot = 0
    for i, ra in a["images"].items():
        rb = b["images"].get(i)
        va, vb = ra, rb
        for k in path:
            va = va.get(k) if isinstance(va, dict) else None
            vb = vb.get(k) if isinstance(vb, dict) else None
        tot += 1
        n += int(va == vb and va is not None)
    return f"{n}/{tot}"


def main():
    base2 = load("baseline", "probe2_gbm.json")
    names = base2["feature_names"]
    A2 = npy("baseline", "probe2_features.npy")
    base3 = load("baseline", "probe3_clf.json")
    base5 = load("baseline", "probe5_cv.json")
    base4 = load("baseline", "probe4_univfd.json")
    out = {}
    for env in ENVS:
        o = {}
        b2 = load(env, "probe2_gbm.json")
        if b2:
            o["p2_gbm_features"] = feat_delta(A2, npy(env, "probe2_features.npy"), names)
            o["p2_heuristic_score_identical"] = field_ident(base2, b2, ["heuristic_score"])
        b3 = load(env, "probe3_clf.json")
        if b3 and base3:
            Pb = npy("baseline", "probe3_proba_on_baseline_feats.npy")
            Pe = npy(env, "probe3_proba_on_baseline_feats.npy")
            Po = npy(env, "probe3_proba_on_own_feats.npy")
            o["p3"] = {
                "loaded": b3["loaded"],
                "load_warnings": b3["load_warnings"],
                "proba_on_baseline_feats_identical": int((Pb == Pe).all(axis=1).sum()),
                "proba_on_baseline_feats_max_abs_delta": float(np.abs(Pb - Pe).max()),
                "proba_on_own_feats_identical_vs_baseline": int((Pb == Po).all(axis=1).sum()),
                "proba_on_own_feats_max_abs_delta": float(np.abs(Pb - Po).max()),
                "e2e_score_identical": field_ident(base3, b3, ["e2e", "score"]),
                "e2e_classifier_score_identical": field_ident(base3, b3, ["e2e", "classifier_score"]),
                "e2e_verdict_identical": field_ident(base3, b3, ["e2e", "verdict_level"]),
            }
        b5 = load(env, "probe5_cv.json")
        if b5 and base5:
            cv_keys = sorted(next(iter(base5["images"].values()))["cv"].keys())
            o["p5"] = {f"cv_{k}": field_ident(base5, b5, ["cv", k]) for k in cv_keys}
            o["p5"]["sift_descriptors"] = field_ident(base5, b5, ["sift", "descriptors_sha256"])
            o["p5"]["sift_n_keypoints"] = field_ident(base5, b5, ["sift", "n_keypoints"])
            o["p5"]["copy_move_score"] = field_ident(base5, b5, ["copy_move", "score"])
            o["p5"]["copy_move_matched_pairs"] = field_ident(base5, b5, ["copy_move", "matched_pairs"])
            o["p5"]["npr_raw_stats"] = field_ident(base5, b5, ["npr_raw_stats_sha256"])
            o["p5"]["npr_score"] = field_ident(base5, b5, ["npr", "score"])
            o["p5"]["gan_lstsq_coeffs"] = field_ident(base5, b5, ["gan_raw", "lstsq_coeffs_sha256"])
            o["p5"]["gan_radial_profile"] = field_ident(base5, b5, ["gan_raw", "radial_profile_sha256"])
            o["p5"]["gan_confidence"] = field_ident(base5, b5, ["gan", "confidence"])
            N = npy("baseline", "probe5_npr_raw_stats.npy")
            M = npy(env, "probe5_npr_raw_stats.npy")
            o["p5"]["npr_raw_stats_max_abs_delta"] = float(np.nanmax(np.abs(N - M)))
            G = npy("baseline", "probe5_gan_lstsq_coeffs.npy")
            H = npy(env, "probe5_gan_lstsq_coeffs.npy")
            o["p5"]["gan_lstsq_coeffs_max_abs_delta"] = float(np.nanmax(np.abs(G - H)))
        b4 = load(env, "probe4_univfd.json")
        if b4 and base4:
            o["p4"] = {
                "clip_input_identical": field_ident(base4, b4, ["clip_input_sha256"]),
                "embedding_identical": field_ident(base4, b4, ["embedding_sha256"]),
                "univfd_score_identical": field_ident(base4, b4, ["univfd_score"]),
                "probe_load_warnings": b4["probe_load_warnings"],
            }
        out[env] = o
    json.dump(out, open(os.path.join(DRIFT, "attribution.json"), "w"), indent=1, sort_keys=True)
    print(json.dumps(out, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
