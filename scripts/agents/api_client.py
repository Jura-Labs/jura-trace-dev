"""
Jura Trace — REST API Client for Corpus Agents

Thin wrapper around the local REST API (port 8300) and Python sidecar
(port 8200). Uses only stdlib (urllib) to avoid extra dependencies.
"""

import http.client
import json
import mimetypes
import os
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

from . import config


def _auth_headers() -> dict:
    """Return Authorization header if an API key is configured."""
    headers = {"Accept": "application/json"}
    if config.JURA_API_KEY:
        headers["Authorization"] = f"Bearer {config.JURA_API_KEY}"
    return headers


def _multipart_encode(fields: dict, files: dict) -> tuple[bytes, str]:
    """Build a multipart/form-data body from fields and files.

    Args:
        fields: {name: value} for text fields.
        files:  {name: (filename, bytes, content_type)} for file fields.

    Returns:
        (body_bytes, content_type_header)
    """
    boundary = "----JuraTraceCorpusAgent"
    parts = []

    for name, value in fields.items():
        parts.append(f"--{boundary}\r\n".encode())
        parts.append(f'Content-Disposition: form-data; name="{name}"\r\n\r\n'.encode())
        parts.append(f"{value}\r\n".encode())

    for name, (filename, data, ctype) in files.items():
        parts.append(f"--{boundary}\r\n".encode())
        parts.append(
            f'Content-Disposition: form-data; name="{name}"; filename="{filename}"\r\n'.encode()
        )
        parts.append(f"Content-Type: {ctype}\r\n\r\n".encode())
        parts.append(data)
        parts.append(b"\r\n")

    parts.append(f"--{boundary}--\r\n".encode())
    body = b"".join(parts)
    content_type = f"multipart/form-data; boundary={boundary}"
    return body, content_type


# ── Health checks ────────────────────────────────────────────────────────

def check_api_health() -> dict | None:
    """Check the Jura Trace REST API (port 8300)."""
    try:
        req = Request(f"{config.JURA_API_URL}/api/v1/health", headers=_auth_headers())
        with urlopen(req, timeout=5) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


def check_sidecar_health() -> dict | None:
    """Check the Python ML sidecar (port 8200)."""
    try:
        req = Request(f"{config.SIDECAR_URL}/health", headers={"Accept": "application/json"})
        with urlopen(req, timeout=5) as resp:
            return json.loads(resp.read())
    except Exception:
        return None


# ── Verify ───────────────────────────────────────────────────────────────

def verify_file(file_path: Path, mode: str = "standard") -> dict | None:
    """Verify a file via POST /api/v1/verify.

    Returns the full API response dict, or None on failure.
    """
    data = file_path.read_bytes()
    if not data:
        return None

    mime = mimetypes.guess_type(str(file_path))[0] or "application/octet-stream"
    body, ctype = _multipart_encode(
        fields={"mode": mode},
        files={"file": (file_path.name, data, mime)},
    )

    headers = _auth_headers()
    headers["Content-Type"] = ctype

    try:
        req = Request(
            f"{config.JURA_API_URL}/api/v1/verify",
            data=body,
            headers=headers,
            method="POST",
        )
        with urlopen(req, timeout=120) as resp:
            return json.loads(resp.read())
    except Exception as e:
        print(f"  [verify] Error for {file_path.name}: {e}")
        return None


# ── Protect: fingerprint ─────────────────────────────────────────────────

def fingerprint_file(file_path: Path) -> dict | None:
    """Fingerprint a file via POST /api/v1/protect/fingerprint.

    Returns the API response dict with hashes, or None on failure.
    """
    data = file_path.read_bytes()
    mime = mimetypes.guess_type(str(file_path))[0] or "application/octet-stream"
    body, ctype = _multipart_encode(
        fields={},
        files={"file": (file_path.name, data, mime)},
    )

    headers = _auth_headers()
    headers["Content-Type"] = ctype

    try:
        req = Request(
            f"{config.JURA_API_URL}/api/v1/protect/fingerprint",
            data=body,
            headers=headers,
            method="POST",
        )
        with urlopen(req, timeout=60) as resp:
            return json.loads(resp.read())
    except Exception as e:
        print(f"  [fingerprint] Error for {file_path.name}: {e}")
        return None


# ── Protect: watermark ───────────────────────────────────────────────────

def watermark_embed_sidecar(file_path: Path, payload: str, strength: str = "medium") -> bytes | None:
    """Embed a watermark via the sidecar POST /forensics/watermark/embed.

    Returns watermarked PNG bytes, or None on failure.
    """
    data = file_path.read_bytes()
    mime = mimetypes.guess_type(str(file_path))[0] or "image/png"
    body, ctype = _multipart_encode(
        fields={"payload": payload, "strength": strength},
        files={"file": (file_path.name, data, mime)},
    )

    headers = {"Accept": "application/json", "Content-Type": ctype}
    if config.JURA_API_KEY:
        headers["X-Jura-API-Key"] = config.JURA_API_KEY

    try:
        req = Request(
            f"{config.SIDECAR_URL}/forensics/watermark/embed",
            data=body,
            headers=headers,
            method="POST",
        )
        with urlopen(req, timeout=60) as resp:
            result = json.loads(resp.read())
            if result.get("success") and result.get("watermarked_image_base64"):
                import base64
                return base64.b64decode(result["watermarked_image_base64"])
    except Exception as e:
        print(f"  [watermark] Error for {file_path.name}: {e}")
    return None


# ── Protect: C2PA sign ───────────────────────────────────────────────────

def sign_c2pa_sidecar(file_path: Path, creator_name: str) -> bytes | None:
    """C2PA-sign a file via the sidecar or REST API.

    Note: C2PA signing is a Rust-side operation exposed via Tauri IPC.
    For corpus agents, we use the REST API POST /api/v1/protect/sign.
    Returns signed file bytes, or None on failure.
    """
    data = file_path.read_bytes()
    mime = mimetypes.guess_type(str(file_path))[0] or "image/png"
    body, ctype = _multipart_encode(
        fields={"creator_name": creator_name},
        files={"file": (file_path.name, data, mime)},
    )

    headers = _auth_headers()
    headers["Content-Type"] = ctype

    try:
        req = Request(
            f"{config.JURA_API_URL}/api/v1/protect/sign",
            data=body,
            headers=headers,
            method="POST",
        )
        with urlopen(req, timeout=60) as resp:
            return resp.read()
    except Exception as e:
        print(f"  [c2pa_sign] Error for {file_path.name}: {e}")
    return None


# ── Sidecar: run detector ────────────────────────────────────────────────

def run_sidecar_detector(endpoint: str, file_path: Path) -> dict | None:
    """Run a single sidecar detector against a file.

    endpoint: e.g. "/forensics/deepfake"
    Returns the parsed JSON response or None.
    """
    data = file_path.read_bytes()
    mime = mimetypes.guess_type(str(file_path))[0] or "image/png"
    body, ctype = _multipart_encode(
        fields={},
        files={"file": (file_path.name, data, mime)},
    )

    headers = {"Accept": "application/json", "Content-Type": ctype}

    try:
        req = Request(
            f"{config.SIDECAR_URL}{endpoint}",
            data=body,
            headers=headers,
            method="POST",
        )
        with urlopen(req, timeout=60) as resp:
            return json.loads(resp.read())
    except Exception as e:
        print(f"  [detector] Error for {file_path.name} on {endpoint}: {e}")
    return None
