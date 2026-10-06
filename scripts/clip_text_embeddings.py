#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Build-time CLIP prompt embeddings (BL-SIZE-001).

The CLIP text encoder (242 MB) only ever encoded five fixed prompts. This
script computes them once, with the sidecar's own code, so the installers can
ship a 10 KB array instead of the encoder:

    clip_text_embeddings.py generate --models-dir models
        Writes models/clip-vit-b32-text-prompts.npy and .json from
        models/clip-vit-b32-text.onnx. Run it when _TEXT_PROMPTS or the
        encoder changes, and commit both files.

    clip_text_embeddings.py check --models-dir DIR
        Encodes the prompts again with the encoder in DIR, in THIS Python's
        onnxruntime, and fails unless the result equals the committed array
        bit for bit and the record names that encoder. The release runs it on
        every platform, with the environment the sidecar is frozen from,
        before leaving the encoder out of the bundle.

Run from the repository root, in an environment with the sidecar's
requirements installed.
"""

import argparse
import datetime
import hashlib
import json
import os
import platform
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(REPO, "sidecar"))

import numpy as np  # noqa: E402
import onnxruntime as ort  # noqa: E402

from app.services import clip_detector as cd  # noqa: E402

TEXT_FILES = ("clip-vit-b32-text.onnx", "clip-vit-b32-text.onnx.data")


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def encode(models_dir):
    text = os.path.join(models_dir, TEXT_FILES[0])
    if not os.path.exists(text):
        sys.exit(f"clip_text_embeddings: no text encoder at {text}")
    session = ort.InferenceSession(text, providers=["CPUExecutionProvider"])
    return cd.encode_prompts_with(session)


def generate(models_dir, out_dir):
    arr = encode(models_dir)
    npy = os.path.join(out_dir, cd.PRECOMPUTED_EMBEDDINGS)
    np.save(npy, arr, allow_pickle=False)
    record = {
        "what": "L2-normalised CLIP ViT-B/32 text embeddings of the sidecar's "
        "_TEXT_PROMPTS, computed at build time so the text encoder need not ship "
        "(BL-SIZE-001).",
        "prompts": cd._TEXT_PROMPTS,
        "shape": list(arr.shape),
        "dtype": str(arr.dtype),
        "npy_sha256": sha256(npy),
        "text_encoder_sha256": {f: sha256(os.path.join(models_dir, f)) for f in TEXT_FILES},
        "onnxruntime": ort.__version__,
        "platform": f"{platform.system()} {platform.machine()}",
        "generated": datetime.date.today().isoformat(),
        "regenerate_with": "scripts/clip_text_embeddings.py generate --models-dir models",
    }
    with open(os.path.join(out_dir, cd.PRECOMPUTED_RECORD), "w", encoding="utf-8") as f:
        json.dump(record, f, indent=2)
        f.write("\n")
    print(f"wrote {npy} ({os.path.getsize(npy)} bytes) and {cd.PRECOMPUTED_RECORD}")
    return 0


def check(models_dir, embeddings_dir):
    problems = []
    rec_path = os.path.join(embeddings_dir, cd.PRECOMPUTED_RECORD)
    with open(rec_path, encoding="utf-8") as f:
        record = json.load(f)
    for name, want in record["text_encoder_sha256"].items():
        got = sha256(os.path.join(models_dir, name))
        if got != want:
            problems.append(f"{name} in {models_dir} is {got}, the record names {want}")
    shipped = cd.load_precomputed_prompt_embeddings(embeddings_dir)
    if shipped is None:
        problems.append(f"{cd.PRECOMPUTED_EMBEDDINGS} in {embeddings_dir} is missing or unusable (see the log)")
    else:
        live = encode(models_dir)
        if not np.array_equal(shipped, live):
            diff = float(np.max(np.abs(shipped.astype(np.float64) - live.astype(np.float64))))
            problems.append(
                f"live encoding differs from the shipped array (max abs difference {diff:.3e}) "
                f"under onnxruntime {ort.__version__} on {platform.system()} {platform.machine()}"
            )
    if problems:
        print("clip_text_embeddings check: FAIL")
        for p in problems:
            print(f"  - {p}")
        return 1
    print(
        f"clip_text_embeddings check: OK, bit-identical under onnxruntime {ort.__version__} "
        f"on {platform.system()} {platform.machine()} ({len(cd._TEXT_PROMPTS)} prompts)"
    )
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("generate")
    g.add_argument("--models-dir", default=os.path.join(REPO, "models"))
    g.add_argument("--out-dir", default=None)
    c = sub.add_parser("check")
    c.add_argument("--models-dir", required=True, help="where the text encoder is")
    c.add_argument("--embeddings-dir", default=os.path.join(REPO, "models"))
    args = ap.parse_args()
    if args.cmd == "generate":
        return generate(args.models_dir, args.out_dir or args.models_dir)
    return check(args.models_dir, args.embeddings_dir)


if __name__ == "__main__":
    sys.exit(main())
