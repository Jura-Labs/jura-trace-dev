# SPDX-License-Identifier: AGPL-3.0-or-later

"""Probe 5, OpenCV-specific paths plus NPR (scipy.stats) and GAN fingerprint
(np.linalg.lstsq).

Usage: python probe5_cv.py [run_tag]   (run_tag e.g. 'run2' for the repeat)

a. copy_move.perform_copy_move_detection: matched_pairs, score, suspicious,
   clone_regions, visualisation hash. RANSAC is unseeded in the code (no
   setRNGSeed anywhere in sidecar/), so the baseline is run twice.
b. SIFT detectAndCompute alone (deterministic part of copy-move) so a
   copy-move difference can be split into SIFT-vs-RANSAC.
c. cv2 primitives that feed the GBM features, on the PIL-decoded RGB array:
   deepfake._resize (INTER_AREA to 512), cvtColor RGB2BGR / RGB2GRAY /
   BGR2HSV, Sobel x/y CV_64F k3, Laplacian CV_64F, GaussianBlur (0,0) s=2,
   medianBlur 3, bilateralFilter (9,75,75), pyrDown, calcHist.
d. npr.perform_npr_analysis (returned rounded fields) plus the raw
   scipy.stats.kurtosis / skew stats replicated from npr.py:95-104.
e. gan_fingerprint.visualise_gan_fingerprint plus the raw lstsq fit
   (_radial_profile + _fit_one_over_f) replicated from lines 208-222.
"""

import os
import sys

import cv2
import numpy as np
from PIL import Image

from probe_common import (
    WarnCapture,
    load_golden,
    out_dir,
    register_plugins,
    sha_array,
    sha_str,
    versions,
    write_json,
)


def cv_primitives(rgb: np.ndarray, df) -> dict:
    out = {}
    r = df._resize(rgb, df.ANALYSIS_SIZE)
    out["resize_inter_area"] = sha_array(r)
    bgr = cv2.cvtColor(r, cv2.COLOR_RGB2BGR)
    grey = cv2.cvtColor(r, cv2.COLOR_RGB2GRAY)
    out["cvtColor_rgb2bgr"] = sha_array(bgr)
    out["cvtColor_rgb2gray"] = sha_array(grey)
    out["cvtColor_bgr2hsv"] = sha_array(cv2.cvtColor(bgr, cv2.COLOR_BGR2HSV))
    gf = grey.astype(np.float64)
    out["sobel_x_64f"] = sha_array(cv2.Sobel(gf, cv2.CV_64F, 1, 0, ksize=3))
    out["sobel_y_64f"] = sha_array(cv2.Sobel(gf, cv2.CV_64F, 0, 1, ksize=3))
    out["laplacian_64f"] = sha_array(cv2.Laplacian(gf, cv2.CV_64F))
    out["gaussianblur_s2"] = sha_array(cv2.GaussianBlur(gf, (0, 0), sigmaX=2.0))
    out["medianblur_3"] = sha_array(cv2.medianBlur(grey, 3))
    out["bilateral_9_75_75"] = sha_array(
        cv2.bilateralFilter(grey, d=9, sigmaColor=75, sigmaSpace=75)
    )
    out["pyrdown_64f"] = sha_array(cv2.pyrDown(gf))
    hists = [cv2.calcHist([bgr], [i], None, [256], [0, 256]).flatten() for i in range(3)]
    out["calchist_bgr"] = sha_array(np.concatenate(hists))
    return out


def sift_only(rgb: np.ndarray) -> dict:
    grey = cv2.cvtColor(rgb, cv2.COLOR_RGB2GRAY)
    sift = cv2.SIFT_create(nfeatures=12000)
    kps, desc = sift.detectAndCompute(grey, None)
    if desc is None:
        return {"n_keypoints": 0, "descriptors_sha256": None, "keypoints_sha256": None}
    pts = np.array([(k.pt[0], k.pt[1], k.size, k.angle, k.response, k.octave) for k in kps], dtype=np.float64)
    return {
        "n_keypoints": int(len(kps)),
        "descriptors_sha256": sha_array(desc),
        "keypoints_sha256": sha_array(pts),
    }


def npr_raw(rgb: np.ndarray, npr) -> np.ndarray:
    from scipy.stats import kurtosis, skew  # noqa: PLC0415

    img = npr._resize(rgb, npr._ANALYSIS_SIZE)
    grey = cv2.cvtColor(img, cv2.COLOR_RGB2GRAY).astype(np.float64)
    maps = [
        grey[:, 1:] - grey[:, :-1],
        grey[1:, :] - grey[:-1, :],
        grey[1:, 1:] - grey[:-1, :-1],
        grey[1:, :-1] - grey[:-1, 1:],
    ]
    vals = []
    for dm in maps:
        flat = dm.flatten()
        vals += [
            float(np.mean(flat)),
            float(np.std(flat)),
            float(kurtosis(flat, bias=True)),
            float(skew(flat, bias=True)),
        ]
    return np.asarray(vals, dtype=np.float64)


def gan_raw(b: bytes, gf) -> tuple[dict, np.ndarray | None]:
    nparr = np.frombuffer(b, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        return {"imdecode": None}, None
    img_rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
    channels = cv2.split(img_rgb.astype(np.float32))
    mags = []
    for ch in channels:
        f = np.fft.fft2(ch)
        mags.append(np.log1p(np.abs(np.fft.fftshift(f))))
    avg = np.mean(mags, axis=0)
    radii, profile = gf._radial_profile(avg)
    fitted = gf._fit_one_over_f(radii, profile)
    log_r = np.log(radii.astype(np.float64) + 1e-8)
    log_p = np.log(profile + 1e-8)
    A = np.vstack([log_r, np.ones_like(log_r)]).T
    coeffs, _, _, _ = np.linalg.lstsq(A, log_p, rcond=None)
    return {
        "imdecode": sha_array(img),
        "avg_magnitude_sha256": sha_array(avg),
        "radial_profile_sha256": sha_array(profile),
        "lstsq_coeffs": coeffs.tolist(),
        "lstsq_coeffs_sha256": sha_array(coeffs),
        "fitted_sha256": sha_array(fitted),
    }, coeffs


def main() -> None:
    tag = sys.argv[1] if len(sys.argv) > 1 else None
    plug = register_plugins()
    from app.services import copy_move as cm  # noqa: PLC0415
    from app.services import deepfake as df  # noqa: PLC0415
    from app.services import gan_fingerprint as gf  # noqa: PLC0415
    from app.services import npr  # noqa: PLC0415

    golden = load_golden()
    d = out_dir(tag)
    res = {"versions": versions(), "plugins": plug, "has_sklearn_dbscan": cm._HAS_SKLEARN, "images": {}}
    npr_stats = np.full((len(golden), 16), np.nan)
    gan_coeffs = np.full((len(golden), 2), np.nan)
    with WarnCapture() as wc:
        for i, e in enumerate(golden):
            rec = {"format": e["format"]}
            try:
                with open(e["path"], "rb") as fh:
                    b = fh.read()
                with Image.open(e["path"]) as im:
                    rgb = np.array(im.convert("RGB"))
                # a. copy-move end-to-end
                r = cm.perform_copy_move_detection(b)
                rec["copy_move"] = {
                    "matched_pairs": r.matched_pairs,
                    "score": r.score,
                    "suspicious": r.suspicious,
                    "clone_regions": sorted(
                        (c["x"], c["y"], c["width"], c["height"], c["area"], c["point_count"])
                        for c in r.clone_regions
                    ),
                    "visualisation_sha256": sha_str(r.visualisation_base64),
                }
                # b. SIFT alone
                rec["sift"] = sift_only(rgb)
                # c. primitives
                rec["cv"] = cv_primitives(rgb, df)
                # d. NPR
                n = npr.perform_npr_analysis(b)
                rec["npr"] = {
                    "score": n.score,
                    "suspicious": n.suspicious,
                    "hv_correlation": n.hv_correlation,
                    "diff_variance_ratio": n.diff_variance_ratio,
                    "hf_energy_ratio": n.hf_energy_ratio,
                    "heatmap_sha256": sha_str(n.heatmap_base64),
                }
                raw = npr_raw(rgb, npr)
                npr_stats[i] = raw
                rec["npr_raw_stats_sha256"] = sha_array(raw)
                # e. GAN fingerprint
                try:
                    g = gf.visualise_gan_fingerprint(b)
                    rec["gan"] = {
                        "n_peaks": len(g["spectral_peaks"]),
                        "spectral_peaks": g["spectral_peaks"],
                        "has_periodic_artefacts": g["has_periodic_artefacts"],
                        "model_attribution": g["model_attribution"],
                        "confidence": g["confidence"],
                        "spectrum_sha256": sha_str(g["spectrum_base64"]),
                    }
                except ValueError as exc:
                    rec["gan"] = {"error": str(exc)}
                graw, coeffs = gan_raw(b, gf)
                rec["gan_raw"] = graw
                if coeffs is not None:
                    gan_coeffs[i] = coeffs
            except Exception as exc:  # noqa: BLE001
                rec["error"] = f"{type(exc).__name__}: {exc}"
            res["images"][e["id"]] = rec
    res["warnings"] = wc.unique()
    np.save(os.path.join(d, "probe5_npr_raw_stats.npy"), npr_stats)
    np.save(os.path.join(d, "probe5_gan_lstsq_coeffs.npy"), gan_coeffs)
    write_json(os.path.join(d, "probe5_cv.json"), res)
    errs = sum(1 for r in res["images"].values() if "error" in r)
    gan_fail = sum(1 for r in res["images"].values() if r.get("gan_raw", {}).get("imdecode") is None)
    print(f"probe5 done: errors={errs} gan_imdecode_failures={gan_fail} -> {d}")


if __name__ == "__main__":
    sys.exit(main())
