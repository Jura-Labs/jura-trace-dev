# SPDX-License-Identifier: AGPL-3.0-or-later

"""Select the 84-image golden set with a fixed seed.

Run in the BASELINE env (needs PIL to reject undecodable / sub-128px files).

Format mix matches docs/calibration/pillow-12-decode-drift-sep2026.md:
24 PNG, 18 JPEG (corpus originals), 14 AVIF, 14 HEIC, 14 WebP (generated
from distinct corpus originals). Stratified across the four class dirs;
`protected` holds no JPEG so its JPEG quota is redistributed.
"""

import json
import os
import random
import sys

from PIL import Image

SEED = 20260908
CORPUS = os.environ.get("JURA_DRIFT_CORPUS")
if not CORPUS:
    raise SystemExit("set JURA_DRIFT_CORPUS to the corpus root (the directory holding authentic/, ai_generated/, test_set/, protected/)")
CLASSES = ["authentic", "ai_generated", "test_set", "protected"]
from probe_common import DRIFT  # noqa: E402

QUOTA = {
    "PNG": {"authentic": 6, "ai_generated": 6, "test_set": 6, "protected": 6},
    "JPEG": {"authentic": 7, "ai_generated": 4, "test_set": 7, "protected": 0},
    "AVIF": {"authentic": 4, "ai_generated": 3, "test_set": 4, "protected": 3},
    "HEIC": {"authentic": 4, "ai_generated": 3, "test_set": 4, "protected": 3},
    "WEBP": {"authentic": 4, "ai_generated": 3, "test_set": 4, "protected": 3},
}
MIN_SIDE = 128


def usable(path: str) -> tuple[bool, str]:
    try:
        with Image.open(path) as im:
            fmt = im.format
            w, h = im.size
        if min(w, h) < MIN_SIDE:
            return False, fmt
        return fmt in ("PNG", "JPEG"), fmt
    except Exception:
        return False, ""


def main() -> None:
    rng = random.Random(SEED)
    pools: dict[str, dict[str, list[str]]] = {}
    for cls in CLASSES:
        files = []
        for root, _dirs, names in os.walk(os.path.join(CORPUS, cls)):
            for n in sorted(names):
                if n.lower().rsplit(".", 1)[-1] in ("jpg", "jpeg", "png"):
                    files.append(os.path.join(root, n))
        files.sort()
        rng.shuffle(files)
        pools[cls] = {"PNG": [], "JPEG": []}
        for f in files:
            ok, fmt = usable(f)
            if ok:
                pools[cls][fmt].append(f)

    images = []
    counters: dict[str, int] = {}

    def take(cls: str, fmt_pool: str) -> str:
        pool = pools[cls][fmt_pool]
        if not pool:
            raise SystemExit(f"pool exhausted: {cls}/{fmt_pool}")
        return pool.pop(0)

    # Originals first.
    for fmt in ("PNG", "JPEG"):
        for cls in CLASSES:
            for _ in range(QUOTA[fmt][cls]):
                p = take(cls, fmt)
                k = f"{fmt.lower()}_{cls}"
                counters[k] = counters.get(k, 0) + 1
                images.append(
                    {
                        "id": f"{fmt.lower()}_{cls}_{counters[k]:02d}",
                        "format": fmt,
                        "class": cls,
                        "path": p,
                        "source_path": None,
                        "generated": False,
                    }
                )

    # Variants: source drawn from whichever original pool still has files,
    # alternating so the sources mix PNG and JPEG parents.
    for fmt in ("AVIF", "HEIC", "WEBP"):
        for cls in CLASSES:
            for i in range(QUOTA[fmt][cls]):
                order = ("PNG", "JPEG") if i % 2 == 0 else ("JPEG", "PNG")
                src = None
                for fp in order:
                    if pools[cls][fp]:
                        src = pools[cls][fp].pop(0)
                        break
                if src is None:
                    raise SystemExit(f"no source left for {fmt}/{cls}")
                k = f"{fmt.lower()}_{cls}"
                counters[k] = counters.get(k, 0) + 1
                ident = f"{fmt.lower()}_{cls}_{counters[k]:02d}"
                ext = {"AVIF": ".avif", "HEIC": ".heic", "WEBP": ".webp"}[fmt]
                images.append(
                    {
                        "id": ident,
                        "format": fmt,
                        "class": cls,
                        "path": os.path.join(DRIFT, "golden", "variants", ident + ext),
                        "source_path": src,
                        "generated": True,
                    }
                )

    assert len(images) == 84, len(images)
    out = {
        "seed": SEED,
        "corpus_root": CORPUS,
        "min_side_px": MIN_SIDE,
        "format_counts": {
            f: sum(1 for e in images if e["format"] == f)
            for f in ("PNG", "JPEG", "AVIF", "HEIC", "WEBP")
        },
        "class_counts": {c: sum(1 for e in images if e["class"] == c) for c in CLASSES},
        "pool_sizes_after_filter": {
            c: {k: len(v) + sum(1 for e in images if e["class"] == c and (e["source_path"] or e["path"]) and (e["format"] == k or (e["generated"] and Image.open(e["source_path"]).format == k))) for k, v in pools[c].items()}
            for c in CLASSES
        },
        "images": images,
    }
    with open(os.path.join(DRIFT, "golden_set.json"), "w") as fh:
        json.dump(out, fh, indent=1)
    print(json.dumps({k: out[k] for k in ("format_counts", "class_counts")}, indent=1))
    print("sample:", images[0]["path"])
    print("sample variant:", images[-1]["path"], "<-", images[-1]["source_path"])


if __name__ == "__main__":
    sys.exit(main())
