#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Assertion A3 (docs/design/v1.2.0-headless-api-and-cli.md, section 11):
the analysis sidecar really ran.

Reads one verification response body (the API's JSON, as `jura verify
--format json` prints it) from a file or stdin, and exits 0 only if:

  - `degraded` is false,
  - `data.detectorsRun` contains ela, noise and deepfake, which only the
    sidecar produces,
  - in deep mode, at least --min-deep detectors ran (default 10);
  - and, with --require-clip, CLIP ran with its model loaded
    (`clip` in detectorsRun, `data.clipResult.modelAvailable` true).

--require-clip is for installed builds. CLIP missing its model files does
not fail anything else: the sidecar starts, answers and analyses, and the
verdict is computed from fewer detectors (design v1.2.0-windows-onedir-and-
size section 4.2, the B4 failure mode). A CI macOS build shipped exactly
that until 5 October 2026, because its release job never fetched the CLIP
models.

A degraded run fails. A run that produced only the Rust detectors fails.
This is the floor `scripts/smoke_test_frozen_sidecar.py` lacked when it
printed "PASSED" for endpoints it never probed (BL-SILENT-001).

The input must be a JPEG's result: ELA only runs on JPEG.

Exit codes: 0 the sidecar ran; 1 it did not (reasons on stderr); 2 the
input was not a verification response.
"""

import argparse
import json
import sys

SIDECAR_ONLY = ("ela", "noise", "deepfake")


def check(body: dict, min_deep: int, require_clip: bool = False) -> list[str]:
    problems = []
    data = body.get("data")
    if not isinstance(data, dict) or "detectorsRun" not in data:
        raise ValueError("no data.detectorsRun: not a verification response")
    ran = data.get("detectorsRun") or []
    if body.get("degraded") is not False:
        problems.append(f"degraded is {body.get('degraded')!r}, not false")
    missing = [d for d in SIDECAR_ONLY if d not in ran]
    if missing:
        problems.append(f"sidecar detectors missing from detectorsRun: {', '.join(missing)}")
    mode = data.get("mode")
    if mode == "deep" and len(ran) < min_deep:
        problems.append(f"only {len(ran)} detectors ran in deep mode, the floor is {min_deep}")
    if require_clip:
        clip = data.get("clipResult") or {}
        if "clip" not in ran or clip.get("modelAvailable") is not True:
            problems.append(
                f"CLIP did not run with its model (clip in detectorsRun: {'clip' in ran}, "
                f"clipResult.modelAvailable: {clip.get('modelAvailable')!r}); are the CLIP files in the bundle's models/?"
            )
    return problems


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("result", nargs="?", help="response JSON file; stdin if omitted")
    ap.add_argument("--min-deep", type=int, default=10)
    ap.add_argument("--require-clip", action="store_true", help="fail unless CLIP ran with its model")
    args = ap.parse_args()
    raw = open(args.result).read() if args.result else sys.stdin.read()
    try:
        body = json.loads(raw)
        problems = check(body, args.min_deep, args.require_clip)
    except (ValueError, AttributeError) as e:
        print(f"assert_sidecar_ran: {e}", file=sys.stderr)
        return 2
    ran = body["data"].get("detectorsRun") or []
    if problems:
        print("assert_sidecar_ran: FAIL, the sidecar did not run:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    print(f"assert_sidecar_ran: OK, {len(ran)} detectors ran: {', '.join(ran)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
