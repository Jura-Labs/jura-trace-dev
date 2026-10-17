# SPDX-License-Identifier: AGPL-3.0-or-later

"""Probe 1, decode: SHA-256 of decoded RGB buffer + decoder class + format."""

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
    res = {"versions": versions(), "plugins": plug, "images": {}}
    with WarnCapture() as wc:
        for e in load_golden():
            rec = {"format": e["format"]}
            try:
                with Image.open(e["path"]) as im:
                    rec["reported_format"] = im.format
                    rec["decoder_class"] = type(im).__name__
                    rec["mode"] = im.mode
                    rec["size"] = list(im.size)
                    rgb = im.convert("RGB")
                    arr = np.asarray(rgb)
                    rec["rgb_sha256"] = sha_array(arr)
                    rec["shape"] = list(arr.shape)
            except Exception as exc:  # noqa: BLE001
                rec["error"] = f"{type(exc).__name__}: {exc}"
            res["images"][e["id"]] = rec
    res["warnings"] = wc.unique()
    d = out_dir()
    write_json(os.path.join(d, "probe1_decode.json"), res)
    errs = sum(1 for r in res["images"].values() if "error" in r)
    print(f"probe1 done: {len(res['images'])} images, {errs} errors -> {d}")


if __name__ == "__main__":
    sys.exit(main())
