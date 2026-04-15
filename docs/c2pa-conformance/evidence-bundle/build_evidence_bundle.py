#!/usr/bin/env python3
"""
Build a C2PA Conformance Programme evidence bundle.

For each of the 7 claimed MIME types:
 1. Convert a clean source JPEG into the target format
 2. Sign with Jura Trace REST API
 3. Verify the Jura-signed file parses through Jura Trace verify
 4. Also run c2patool externally on the signed output for cross-validation

Outputs everything to docs/c2pa-conformance/evidence-bundle/.
"""

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path
from urllib.request import Request, urlopen

import pillow_heif
pillow_heif.register_heif_opener()
try:
    import pillow_avif  # noqa: F401
except ImportError:
    pass
from PIL import Image

API = "http://127.0.0.1:8300"
API_KEY = "jt_corpus_scan_f1ef13848615459a"
HEADERS = {"Authorization": f"Bearer {API_KEY}"}

SOURCE = Path("/tmp/c2pa-interop/test_input.jpg")
BUNDLE = Path("/Users/paulgriffiths/Downloads/ecoadvisor/juralabs/docs/c2pa-conformance/evidence-bundle")
BUNDLE.mkdir(parents=True, exist_ok=True)

# 7 MIME types Jura Trace claims to validate
FORMATS = [
    # (mime, extension, Pillow format string)
    ("image/jpeg",  "jpg",  "JPEG"),
    ("image/png",   "png",  "PNG"),
    ("image/tiff",  "tiff", "TIFF"),
    ("image/webp",  "webp", "WEBP"),
    ("image/heic",  "heic", "HEIF"),  # HEIC = HEIF brand
    ("image/heif",  "heif", "HEIF"),
    ("image/avif",  "avif", "AVIF"),
]


def convert_source(src_jpg: Path, out_path: Path, pil_format: str):
    """Convert source JPEG to target format."""
    img = Image.open(src_jpg)
    # Strip original EXIF/metadata for a clean signing baseline
    clean = Image.new(img.mode, img.size)
    clean.putdata(list(img.getdata()))
    # Convert RGB for formats that don't like alpha/unusual modes
    if pil_format in ("JPEG", "HEIF", "AVIF") and clean.mode != "RGB":
        clean = clean.convert("RGB")
    save_kwargs = {}
    if pil_format == "JPEG":
        save_kwargs["quality"] = 90
    elif pil_format == "WEBP":
        save_kwargs["quality"] = 90
    elif pil_format == "TIFF":
        save_kwargs["compression"] = "tiff_lzw"
    clean.save(out_path, format=pil_format, **save_kwargs)


def sign_with_jura(src_path: Path, out_path: Path, mime: str) -> dict:
    """Call Jura Trace REST sign endpoint."""
    import mimetypes
    mimetypes.add_type(mime, f".{src_path.suffix}")
    boundary = "----JuraC2PAEvidence"
    body_parts = []

    def add_field(name, value):
        body_parts.append(f"--{boundary}\r\n".encode())
        body_parts.append(f'Content-Disposition: form-data; name="{name}"\r\n\r\n'.encode())
        body_parts.append(value.encode() if isinstance(value, str) else value)
        body_parts.append(b"\r\n")

    def add_file(name, filepath, mime_t):
        body_parts.append(f"--{boundary}\r\n".encode())
        body_parts.append(
            f'Content-Disposition: form-data; name="{name}"; filename="{filepath.name}"\r\n'.encode()
        )
        body_parts.append(f"Content-Type: {mime_t}\r\n\r\n".encode())
        body_parts.append(filepath.read_bytes())
        body_parts.append(b"\r\n")

    add_file("file", src_path, mime)
    add_field("creator_name", "Jura Labs CIC")
    add_field("license", "CC-BY-4.0")
    body_parts.append(f"--{boundary}--\r\n".encode())
    body = b"".join(body_parts)

    req = Request(f"{API}/api/v1/protect/sign", data=body, method="POST",
                  headers={**HEADERS, "Content-Type": f"multipart/form-data; boundary={boundary}"})
    try:
        resp = urlopen(req, timeout=60)
        signed = resp.read()
        out_path.write_bytes(signed)
        return {"status": "ok", "bytes": len(signed), "http": resp.status}
    except Exception as e:
        return {"status": "error", "error": str(e)[:200]}


def verify_with_jura(path: Path) -> dict:
    """Call Jura Trace REST verify endpoint and extract key fields."""
    import mimetypes
    mime = mimetypes.guess_type(str(path))[0] or "application/octet-stream"
    boundary = "----JuraC2PAVerify"
    body_parts = []

    def add_file(name, filepath, mime_t):
        body_parts.append(f"--{boundary}\r\n".encode())
        body_parts.append(
            f'Content-Disposition: form-data; name="{name}"; filename="{filepath.name}"\r\n'.encode()
        )
        body_parts.append(f"Content-Type: {mime_t}\r\n\r\n".encode())
        body_parts.append(filepath.read_bytes())
        body_parts.append(b"\r\n")

    def add_field(name, value):
        body_parts.append(f"--{boundary}\r\n".encode())
        body_parts.append(f'Content-Disposition: form-data; name="{name}"\r\n\r\n'.encode())
        body_parts.append(value.encode())
        body_parts.append(b"\r\n")

    add_file("file", path, mime)
    add_field("mode", "standard")
    body_parts.append(f"--{boundary}--\r\n".encode())
    body = b"".join(body_parts)

    req = Request(f"{API}/api/v1/verify", data=body, method="POST",
                  headers={**HEADERS, "Content-Type": f"multipart/form-data; boundary={boundary}"})
    try:
        resp = urlopen(req, timeout=120)
        data = json.loads(resp.read())
        d = data.get("data", {})
        m = d.get("c2paManifest") or {}
        return {
            "status": "ok",
            "c2paValid": d.get("c2paValid"),
            "manifest_present": bool(d.get("c2paManifest")),
            "detectors_run": d.get("detectorsRun"),
            "title": m.get("title"),
            "claim_generator": m.get("claimGenerator"),
            "assertions_count": len(m.get("assertions", [])),
            "signed_at": m.get("signedAt"),
        }
    except Exception as e:
        return {"status": "error", "error": str(e)[:200]}


def verify_with_c2patool(path: Path) -> dict:
    """Run c2patool externally for cross-validation."""
    try:
        r = subprocess.run(["c2patool", str(path)], capture_output=True, text=True, timeout=30)
        if r.returncode != 0:
            return {"status": "error", "stderr": r.stderr[:200]}
        d = json.loads(r.stdout)
        am = d.get("active_manifest")
        m = d["manifests"][am] if am else {}
        sig = m.get("signature_info", {})
        return {
            "status": "ok",
            "validation_state": d.get("validation_state"),
            "active_manifest": am,
            "issuer": sig.get("issuer"),
            "time": sig.get("time"),
            "assertions_count": len(m.get("assertions", [])),
            "assertion_labels": [a.get("label") for a in m.get("assertions", [])],
        }
    except Exception as e:
        return {"status": "error", "error": str(e)[:200]}


def main():
    print(f"Building evidence bundle at {BUNDLE}\n")
    results = {}

    for mime, ext, pil_fmt in FORMATS:
        key = mime.replace("/", "_")
        print(f"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
        print(f"  {mime}")
        print(f"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

        fmt_dir = BUNDLE / key
        fmt_dir.mkdir(exist_ok=True)

        # 1. Convert source → target format
        source = fmt_dir / f"source.{ext}"
        try:
            convert_source(SOURCE, source, pil_fmt)
            print(f"  ✓ source converted: {source.stat().st_size:,} bytes")
        except Exception as e:
            print(f"  ✗ conversion failed: {e}")
            results[mime] = {"conversion": str(e)}
            continue

        # 2. Sign with Jura
        signed = fmt_dir / f"jura_signed.{ext}"
        if signed.exists():
            signed.unlink()
        sign_result = sign_with_jura(source, signed, mime)
        if sign_result["status"] == "ok":
            print(f"  ✓ Jura signed: {sign_result['bytes']:,} bytes")
        else:
            print(f"  ✗ Jura sign failed: {sign_result['error']}")
            results[mime] = {"conversion": "ok", "sign": sign_result}
            continue

        # 3. Verify Jura-signed via Jura Trace
        jura_verify = verify_with_jura(signed)
        if jura_verify["status"] == "ok":
            verdict = "PASS" if jura_verify.get("c2paValid") else "FAIL"
            print(f"  ✓ Jura verify: {verdict} | valid={jura_verify['c2paValid']} | assertions={jura_verify['assertions_count']}")
        else:
            print(f"  ✗ Jura verify error: {jura_verify['error']}")

        # 4. Cross-validate with c2patool
        ext_verify = verify_with_c2patool(signed)
        if ext_verify["status"] == "ok":
            verdict = ext_verify.get("validation_state", "unknown")
            print(f"  ✓ c2patool verify: {verdict} | assertions={ext_verify.get('assertions_count')}")
        else:
            print(f"  ✗ c2patool error: {ext_verify.get('error') or ext_verify.get('stderr')}")

        # Persist per-format evidence JSON
        evidence = {
            "mime_type": mime,
            "extension": ext,
            "pillow_format": pil_fmt,
            "source_file": str(source.name),
            "signed_file": str(signed.name),
            "sign_result": sign_result,
            "jura_verify_result": jura_verify,
            "c2patool_verify_result": ext_verify,
            "timestamp_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
        (fmt_dir / "evidence.json").write_text(json.dumps(evidence, indent=2))
        results[mime] = evidence
        print()

    # Overall summary
    summary_path = BUNDLE / "SUMMARY.json"
    summary_path.write_text(json.dumps({
        "generated": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "applicant": "Jura Labs CIC",
        "product": "Jura Trace",
        "record_id": "019d8d83-ed1c-787c-920c-8fad67b55cbe",
        "c2pa_rs_version": "0.79",
        "c2patool_version": "0.26.47",
        "results_by_mime_type": results,
    }, indent=2))

    print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
    print(f"  Evidence bundle at: {BUNDLE}")
    print(f"  Summary JSON: {summary_path}")
    print("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")


if __name__ == "__main__":
    main()
