#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Golden-verdict baseline for packaging changes that must not change results.

docs/design/v1.2.0-windows-onedir-and-size.md section 5.2: a green build
proves nothing when the failure mode is a product that works and answers
differently. This captures what a running Jura Trace says about a fixed set
of images, and compares a later run against it.

    golden_verdicts.py extract  RAW.ndjson  > golden.jsonl
    golden_verdicts.py compare  BASELINE.jsonl  AFTER.jsonl  --expect N

`extract` reads `jura verify --format ndjson` output and keeps the fields
that decide or display a verdict. `compare` fails unless:

  - both runs examined exactly N images, the same ones, with no failures;
  - CLIP ran on every image in both runs (clip in detectorsRun and
    modelAvailable true), so a detector that was off in both cannot pass;
  - overallTrust, the verdict band, the deepfake score and verdict level,
    and the CLIP score, verdict level, UnivFD score and five class
    probabilities are equal (within --tolerance, default 0: exact).

Images are keyed by the SHA-256 of their bytes, not their path, so the two
runs may use different folders.
"""

import argparse
import json
import sys

FIELDS = (
    ("overallTrust", ("data", "overallTrust")),
    ("band", ("data", "verdict", "band")),
    ("deepfakeScore", ("data", "deepfakeResult", "score")),
    ("deepfakeVerdict", ("data", "deepfakeResult", "verdictLevel")),
    ("clipScore", ("data", "clipResult", "score")),
    ("clipVerdict", ("data", "clipResult", "verdictLevel")),
    ("clipAvailable", ("data", "clipResult", "modelAvailable")),
    ("univfdScore", ("data", "clipResult", "univfdScore")),
    ("clipClassProbs", ("data", "clipResult", "classProbs")),
)


def dig(obj, path):
    for key in path:
        if not isinstance(obj, dict):
            return None
        obj = obj.get(key)
    return obj


def extract(raw_path):
    rows = []
    with open(raw_path) as f:
        for n, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            resp = rec.get("response")
            row = {"input": rec.get("input"), "exit": rec.get("exit")}
            if resp is None:
                row["error"] = (rec.get("error") or {}).get("message", "no response")
            else:
                row["sha256"] = dig(resp, ("data", "inputSha256"))
                row["detectorsRun"] = dig(resp, ("data", "detectorsRun"))
                row["degraded"] = resp.get("degraded")
                row["engine"] = dig(resp, ("data", "provenance", "engineVersion"))
                row["sidecar"] = dig(resp, ("data", "provenance", "sidecarVersion"))
                row["modelHashes"] = dig(resp, ("data", "provenance", "modelHashes"))
                for name, path in FIELDS:
                    row[name] = dig(resp, path)
            rows.append(row)
    for row in rows:
        print(json.dumps(row, sort_keys=True))
    return 0


def load(path):
    with open(path) as f:
        return [json.loads(l) for l in f if l.strip()]


def close(a, b, tol):
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        return abs(a - b) <= tol
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(close(a[k], b[k], tol) for k in a)
    return a == b


def compare(base_path, after_path, expect, tol):
    problems = []
    runs = {}
    for label, path in (("baseline", base_path), ("after", after_path)):
        rows = load(path)
        failed = [r for r in rows if "error" in r or r.get("exit") not in (0, 20)]
        for r in failed:
            problems.append(f"{label}: {r.get('input')} produced no analysis: {r.get('error') or r.get('exit')}")
        good = [r for r in rows if r not in failed]
        if len(rows) != expect:
            problems.append(f"{label}: {len(rows)} images, expected {expect}")
        no_clip = [r["input"] for r in good if not r.get("clipAvailable") or "clip" not in (r.get("detectorsRun") or [])]
        if no_clip:
            problems.append(f"{label}: CLIP did not run on {len(no_clip)} image(s), e.g. {no_clip[:3]}")
        runs[label] = {r["sha256"]: r for r in good}

    base, after = runs["baseline"], runs["after"]
    if base.keys() != after.keys():
        problems.append(
            f"different images: {len(base.keys() - after.keys())} only in the baseline, "
            f"{len(after.keys() - base.keys())} only after"
        )
    compared = 0
    for sha in sorted(base.keys() & after.keys()):
        b, a = base[sha], after[sha]
        compared += 1
        for name, _ in FIELDS:
            if not close(b.get(name), a.get(name), tol):
                problems.append(f"{b['input']}: {name} {b.get(name)!r} -> {a.get(name)!r}")

    print(f"compared {compared} images (expected {expect}), tolerance {tol}")
    if compared != expect:
        problems.append(f"compared {compared} images, expected {expect}")
    if problems:
        print(f"FAIL: {len(problems)} difference(s) or problem(s):")
        for p in problems[:60]:
            print(f"  - {p}")
        return 1
    print("OK: every verdict, score and CLIP readout is unchanged, and CLIP ran on every image in both runs")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    e = sub.add_parser("extract")
    e.add_argument("raw")
    c = sub.add_parser("compare")
    c.add_argument("baseline")
    c.add_argument("after")
    c.add_argument("--expect", type=int, required=True)
    c.add_argument("--tolerance", type=float, default=0.0)
    args = ap.parse_args()
    if args.cmd == "extract":
        return extract(args.raw)
    return compare(args.baseline, args.after, args.expect, args.tolerance)


if __name__ == "__main__":
    sys.exit(main())
