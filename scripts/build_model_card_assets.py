#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Build the model-card metadata files attached to each release.

The help page's model cards point readers at "the model-card metadata JSON
shipped alongside each release" (BL-CLAIM-002). This writes those files from
the training metadata in models/, with two things removed:

- generator names, replaced by the public family labels in
  models/generator_family_labels.json. A key with no label is an error, so
  a retrain that adds a generator cannot publish its name by accident;
- local filesystem paths from the training corpus record. Counts are kept.

Each file also records the SHA-256 of the model file the installer ships, so
a reader can tie the card to the artefact. For the UnivFD probe the training
metadata carries its own hash, and a mismatch is an error.

Standard library only, so it runs on a bare CI runner.

Usage: python3 scripts/build_model_card_assets.py <output-dir>
"""

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MODELS = ROOT / "models"
SHIPPED = ROOT / "src-tauri" / "models"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def gbm_card() -> dict:
    meta = load(MODELS / "deepfake_classifier_v4_meta.json")
    shipped = SHIPPED / "deepfake_classifier.joblib"
    return {
        "model": "GBM classifier v4",
        "shipped_file": "deepfake_classifier.joblib",
        "shipped_sha256": sha256(shipped),
        "trained_at": meta["trained_at"],
        "n_samples": meta["n_samples"],
        "n_authentic": meta["n_authentic"],
        "n_ai_generated": meta["n_ai_generated"],
        "n_features": meta["n_features"],
        "feature_names": meta["feature_names"],
        "cv_folds": meta["cv_folds"],
        "cv_auc_roc": meta["cv_auc_roc"],
        "model_params": meta["model_params"],
        "top_features": meta["top_features"],
        "per_generator_recall": None,
        "note": (
            "Per-generator recall was not recorded when this model was trained. "
            "It is reported for the UnivFD probe."
        ),
    }


def univfd_card(labels: dict) -> dict:
    meta = load(MODELS / "univfd_probe_v10onnx_meta.json")
    shipped = SHIPPED / "univfd_probe.joblib"
    shipped_hash = sha256(shipped)
    if shipped_hash != meta["sha256"]:
        raise SystemExit(
            f"src-tauri/models/univfd_probe.joblib is {shipped_hash}, but "
            f"univfd_probe_v10onnx_meta.json describes {meta['sha256']}. "
            "The card would describe a model the installer does not ship."
        )

    unlabelled = sorted(k for k in meta["per_generator_recall"] if k not in labels)
    if unlabelled:
        raise SystemExit(
            f"{len(unlabelled)} generator key(s) have no public label in "
            "models/generator_family_labels.json. Add one for each before "
            "publishing; the key itself is not written out."
        )

    families = [
        {"family": labels[k], **v}
        for k, v in sorted(
            meta["per_generator_recall"].items(),
            key=lambda kv: (-kv[1]["n"], labels[kv[0]]),
        )
    ]
    corpus = meta["training_corpus"]
    return {
        "model": f"UnivFD linear probe {meta['model_version']}",
        "shipped_file": "univfd_probe.joblib",
        "shipped_sha256": shipped_hash,
        "trained_at": meta["trained_at"],
        "approach": meta["approach"],
        "clip_model": meta["clip_model"],
        "clip_pretrained": meta["clip_pretrained"],
        "embedding_dim": meta["embedding_dim"],
        "random_seed": meta["random_seed"],
        "probe_params": meta["probe_params"],
        "training_corpus": {
            k: corpus[k]
            for k in ("n_train", "n_test", "n_train_authentic", "n_train_ai")
        },
        "evaluation": {
            k: v for k, v in meta["evaluation"].items() if k != "split_json"
        },
        "per_variant_auc": meta["per_variant_auc"],
        "per_generator_family_recall": families,
        "regression_gates": meta["regression_gates"],
    }


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit(__doc__.strip().splitlines()[-1])
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)

    labels = load(MODELS / "generator_family_labels.json")["labels"]
    cards = {
        "model-card-gbm.json": gbm_card(),
        "model-card-univfd.json": univfd_card(labels),
    }

    for name, card in cards.items():
        text = json.dumps(card, indent=2, ensure_ascii=False) + "\n"
        for key in labels:
            if key.strip("_") and f'"{key}"' in text:
                raise SystemExit(f"{name} still contains a withheld generator key.")
        if "/Volumes/" in text or "/Users/" in text:
            raise SystemExit(f"{name} contains a local filesystem path.")
        (out / name).write_text(text, encoding="utf-8")
        print(f"wrote {out / name}")


if __name__ == "__main__":
    main()
