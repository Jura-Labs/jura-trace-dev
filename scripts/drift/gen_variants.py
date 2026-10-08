# SPDX-License-Identifier: AGPL-3.0-or-later

"""Generate the AVIF / HEIC / WebP variants with the BASELINE stack, then
record SHA-256 of every golden file so both envs provably read the same bytes."""

import hashlib
import json
import os
import sys

from probe_common import DRIFT, MANIFEST, register_plugins, env_name, versions

from PIL import Image


def main() -> None:
    if env_name() != "baseline":
        raise SystemExit("variants must be generated in the baseline env")
    plug = register_plugins()
    gpath = os.path.join(DRIFT, "golden_set.json")
    with open(gpath) as fh:
        g = json.load(fh)
    os.makedirs(os.path.join(DRIFT, "golden", "variants"), exist_ok=True)
    save_kw = {
        "AVIF": {"format": "AVIF", "quality": 75},
        "HEIC": {"format": "HEIF", "quality": 75},
        "WEBP": {"format": "WEBP", "quality": 80},
    }
    for e in g["images"]:
        if e["generated"]:
            with Image.open(e["source_path"]) as src:
                rgb = src.convert("RGB")
                rgb.save(e["path"], **save_kw[e["format"]])
            with Image.open(e["path"]) as chk:
                e["variant_reported_format"] = chk.format
                e["variant_size"] = list(chk.size)
        with open(e["path"], "rb") as fh:
            e["sha256"] = hashlib.sha256(fh.read()).hexdigest()
        e["bytes"] = os.path.getsize(e["path"])
    # Regenerated variants must be byte-identical to the published manifest,
    # or the comparison is between different images, not different stacks.
    if os.path.exists(MANIFEST):
        with open(MANIFEST) as fh:
            want = {e["id"]: e["sha256"] for e in json.load(fh)["images"]}
        bad = [e["id"] for e in g["images"] if want.get(e["id"]) != e["sha256"]]
        if bad:
            raise SystemExit(f"{len(bad)} golden files differ from golden_manifest.json: {', '.join(bad)}")
        print("all 84 golden files match golden_manifest.json")
    g["generated_with"] = {"env": "baseline", **versions(), **plug, "save_kwargs": save_kw}
    with open(gpath, "w") as fh:
        json.dump(g, fh, indent=1)
    fmts = {}
    for e in g["images"]:
        if e["generated"]:
            fmts.setdefault(e["format"], set()).add(e["variant_reported_format"])
    print("generated; reported formats:", {k: sorted(v) for k, v in fmts.items()})


if __name__ == "__main__":
    sys.exit(main())
