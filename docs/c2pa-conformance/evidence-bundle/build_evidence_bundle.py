#!/usr/bin/env python3
"""Rebuild the evidence bundle with small source images and stripped
verify-response JSON (only C2PA-relevant fields, no ML heatmaps)."""

import json
import os
import shutil
import subprocess
import time
from pathlib import Path
from urllib.request import Request, urlopen

import pillow_heif
pillow_heif.register_heif_opener()
try:
    import pillow_avif  # noqa
except ImportError:
    pass
from PIL import Image

API = "http://127.0.0.1:8300"
# Local REST API key (Settings → API Keys). Never hardcode: this file is public.
HEADERS = {"Authorization": f"Bearer {os.environ['JURA_API_KEY']}"}
SOURCE = Path("/tmp/c2pa-interop/test_input.jpg")
BUNDLE = Path(__file__).resolve().parent

FORMATS = [
    ("image/jpeg", "jpg",  "JPEG"),
    ("image/png",  "png",  "PNG"),
    ("image/tiff", "tiff", "TIFF"),
    ("image/webp", "webp", "WEBP"),
]

# Small enough to make evidence quick to inspect, large enough to be a
# realistic format round-trip test.
MAX_DIM = 1024


def convert_small(src_jpg, out_path, pil_format):
    img = Image.open(src_jpg)
    img.thumbnail((MAX_DIM, MAX_DIM), Image.Resampling.LANCZOS)
    if pil_format == "JPEG" and img.mode != "RGB":
        img = img.convert("RGB")
    kwargs = {}
    if pil_format == "JPEG":
        kwargs["quality"] = 90
    elif pil_format == "WEBP":
        kwargs["quality"] = 90
    elif pil_format == "TIFF":
        kwargs["compression"] = "tiff_lzw"
    elif pil_format == "PNG":
        kwargs["optimize"] = True
    img.save(out_path, format=pil_format, **kwargs)


def build_multipart(fields, files):
    boundary = "----JuraC2PAEvidence"
    parts = []
    for name, value in fields.items():
        parts.append(f"--{boundary}\r\n".encode())
        parts.append(f'Content-Disposition: form-data; name="{name}"\r\n\r\n'.encode())
        parts.append(value.encode())
        parts.append(b"\r\n")
    for name, (filepath, mime) in files.items():
        parts.append(f"--{boundary}\r\n".encode())
        parts.append(f'Content-Disposition: form-data; name="{name}"; filename="{filepath.name}"\r\n'.encode())
        parts.append(f"Content-Type: {mime}\r\n\r\n".encode())
        parts.append(filepath.read_bytes())
        parts.append(b"\r\n")
    parts.append(f"--{boundary}--\r\n".encode())
    return b"".join(parts), f"multipart/form-data; boundary={boundary}"


def sign(src, out, mime):
    body, ctype = build_multipart(
        {"creator_name": "Jura Labs CIC", "license": "CC-BY-4.0"},
        {"file": (src, mime)},
    )
    req = Request(f"{API}/api/v1/protect/sign", data=body, method="POST",
                  headers={**HEADERS, "Content-Type": ctype})
    resp = urlopen(req, timeout=60)
    out.write_bytes(resp.read())
    return resp.status


def verify(path, mime):
    body, ctype = build_multipart(
        {"mode": "standard"},
        {"file": (path, mime)},
    )
    req = Request(f"{API}/api/v1/verify", data=body, method="POST",
                  headers={**HEADERS, "Content-Type": ctype})
    resp = urlopen(req, timeout=120)
    return json.loads(resp.read())


def strip_to_c2pa_fields(full_response):
    """Keep only C2PA-validator-relevant fields. Remove ML heatmaps and
    detector outputs that have nothing to do with C2PA conformance."""
    d = full_response.get("data", {})
    stripped = {
        "status": full_response.get("status"),
        "data": {
            "mode": d.get("mode"),
            "sourceType": d.get("sourceType"),
            "contentType": d.get("contentType"),
            "detectorsRun": d.get("detectorsRun"),
            "overallTrust": d.get("overallTrust"),
            "c2paSigned": d.get("c2paSigned"),
            "c2paValid": d.get("c2paValid"),
            "c2paManifest": d.get("c2paManifest"),
            "aiGenerator": d.get("aiGenerator"),
            "inputSha256": d.get("inputSha256"),
            "methodology": d.get("methodology"),
            # Omit: elaResult (base64 heatmap), deepfakeResult (base64 heatmap),
            # clipResult, noiseResult, etc. — these are ML detection outputs,
            # not C2PA conformance evidence.
        },
    }
    return stripped


def c2patool_selfqa(path):
    try:
        r = subprocess.run(["c2patool", str(path)], capture_output=True, text=True, timeout=30)
        if r.returncode != 0:
            return {"status": "error", "stderr": r.stderr[:200]}
        return {
            "note": "Internal QA only — the C2PA Conformance Programme reviewer is expected to run their own validator.",
            "c2patool_version": "0.26.47",
            "c2patool_raw_output": json.loads(r.stdout),
        }
    except Exception as e:
        return {"status": "error", "error": str(e)[:200]}


def main():
    print(f"Rebuilding lean bundle (max {MAX_DIM}px, stripped JSON)\n")

    for mime, ext, pil_fmt in FORMATS:
        key = mime.replace("/", "_")
        d = BUNDLE / key
        d.mkdir(exist_ok=True)

        source = d / f"source.{ext}"
        convert_small(SOURCE, source, pil_fmt)

        signed = d / f"jura_signed.{ext}"
        if signed.exists():
            signed.unlink()
        sign(source, signed, mime)

        resp = verify(signed, mime)
        stripped = strip_to_c2pa_fields(resp)
        (d / "jura_trace_verify_response.json").write_text(json.dumps(stripped, indent=2))
        (d / "c2patool_selfqa.json").write_text(json.dumps(c2patool_selfqa(signed), indent=2))

        data = stripped["data"]
        m = data.get("c2paManifest") or {}
        (d / "index.json").write_text(json.dumps({
            "mime_type": mime,
            "source_file": source.name,
            "source_bytes": source.stat().st_size,
            "signed_file": signed.name,
            "signed_bytes": signed.stat().st_size,
            "source_dimensions": f"max {MAX_DIM}px longest side",
            "primary_evidence": "jura_trace_verify_response.json",
            "primary_evidence_note": (
                "Unedited HTTP response body from POST /api/v1/verify, filtered to "
                "C2PA-relevant fields only. Full unfiltered response (including ML "
                "detector heatmap base64 data) is available from the endpoint — "
                "stripped here to keep the bundle size reasonable."
            ),
            "headline_values": {
                "c2paValid": data.get("c2paValid"),
                "c2paManifest_present": bool(data.get("c2paManifest")),
                "assertion_count": len(m.get("assertions", [])),
                "detectors_run": data.get("detectorsRun"),
                "manifest_title": m.get("title"),
                "signed_at": m.get("signedAt"),
            },
            "timestamp_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }, indent=2))

        print(f"  {mime:12s}  source={source.stat().st_size:>8,}  signed={signed.stat().st_size:>8,}  valid={data.get('c2paValid')}")


if __name__ == "__main__":
    main()
