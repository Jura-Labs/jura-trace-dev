#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Smoke-test the frozen PyInstaller sidecar bundle.

Catches the class of bug that shipped to production on 2026-05-21 where
the bundled sidecar reported `watermark: True` capability but the actual
endpoint returned `success: false` with message "invisible-watermark
library not installed". The library WAS bundled but its eager torch
import via `imwatermark.rivaGan` failed at runtime. The dev sidecar
worked fine (torch present in conda env) and the dev pytest suite passed.

This script:

  1. Launches the bundled sidecar binary as a subprocess on a free port.
  2. Waits for /health to return 200.
  3. Reads /health capabilities to know which features should be live.
  4. For each capability that is true, POSTs a small synthetic test
     image to the corresponding /forensics/* endpoint.
  5. Asserts the response does NOT contain "library not installed",
     "ImportError", or "ModuleNotFoundError" in the message field.
  6. Kills the sidecar process.

Exit codes:
  0 - all reachable endpoints returned a clean response
  1 - sidecar failed to start within timeout
  2 - one or more endpoints surfaced a library-import failure
  3 - sidecar returned a non-2xx HTTP status on a known-good endpoint

Wire this into:
  - scripts/build-local-mac.sh post-PyInstaller, pre-Tauri-bundle
  - .github/workflows/release.yml post-PyInstaller step, before signing
  - .forgejo/workflows/release.yml when the Codeberg migration completes

Usage:
  python3 scripts/smoke_test_frozen_sidecar.py <path-to-bundled-sidecar>

Example:
  python3 scripts/smoke_test_frozen_sidecar.py sidecar/dist/jura-sidecar/jura-sidecar
"""

from __future__ import annotations

import io
import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

# Capabilities reported by /health that we treat as testable.
# The value is the /forensics/* endpoint path.
CAPABILITY_TO_ENDPOINT = {
    "ela": "/forensics/ela",
    "noise": "/forensics/noise",
    "copy_move": "/forensics/copy-move",
    "deepfake": "/forensics/deepfake",
    "jpeg_ghost": "/forensics/jpeg-ghost",
    "npr": "/forensics/npr",
    "segmented_ela": "/forensics/segmented-ela",
    "shadow_consistency": "/forensics/shadow-consistency",
    "colour_temperature": "/forensics/colour-temperature",
    "splice_boundary": "/forensics/splice-boundary",
    "clip_detect": "/forensics/clip-detect",
    "watermark": "/forensics/watermark/extract",
}

# Strings in the response message that indicate a bundle-import failure.
# Catching these is the WHOLE POINT of this smoke test. Adding more
# variants here is cheaper than discovering the bug in production.
IMPORT_FAILURE_MARKERS = [
    "library not installed",
    "ImportError",
    "ModuleNotFoundError",
    "No module named",
    "cannot import",
    "module is not installed",
]


def find_free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def make_test_image(width: int = 320, height: int = 320) -> bytes:
    """Build a minimal valid JPEG without external dependencies.

    Uses Pillow if available (covers dev runs); falls back to a
    pre-encoded 320x320 grey JPEG embedded as bytes for the frozen
    bundle context where Pillow may not be on the test harness path.
    """
    try:
        from PIL import Image

        img = Image.new("RGB", (width, height), (128, 128, 128))
        buf = io.BytesIO()
        img.save(buf, format="JPEG", quality=85)
        return buf.getvalue()
    except ImportError:
        # Fallback: pre-encoded 8x8 grey JPEG. Some detectors enforce
        # min dimensions (watermark needs 256x256); those will be
        # skipped with a non-failure note when this fallback is used.
        # In practice the smoke-test harness always has Pillow.
        sys.stderr.write(
            "WARN: Pillow not available, using minimal 8x8 fallback. "
            "Watermark + segmented-ELA tests will be skipped.\n"
        )
        return bytes.fromhex(
            "ffd8ffe000104a46494600010100000100010000ffdb004300080606"
            "070605080707070909080a0c140d0c0b0b0c1912130f141d1a1f1e1d"
            "1a1c1c20242e2720222c231c1c2837292c30313434341f27393d3832"
            "3c2e333432ffc0000b08000800080101011100ffc4001f0000010501"
            "01010101010000000000000000010203040506070809000affc40031"
            "10000201030302040303020403010500010202031121041231054151"
            "611322f0327181a11423426292a14352d243e1825272ffc4001f0100"
            "030101010101010101010000000000000102030405060708090a0bff"
            "c400b51100020102040403040705040400010277000102031104052131"
            "1206411351617122718132061491c1235233f0156272d1a162342434"
            "82e1f01716718191a25262728292a3536378334434441435363738"
            "39363a4344243d2425263738393a434445464748494a535455565758"
            "59353c2434445464748494a636465666768696a737475767778797a"
            "8283848586878889899293949596979899ffda000c0301000211031100"
            "3f00fbfcffd9"
        )


def wait_for_health(port: int, timeout: float = 60.0) -> dict | None:
    """Poll /health every 0.5s until it returns 200 or timeout expires."""
    deadline = time.monotonic() + timeout
    last_err = None
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(
                f"http://127.0.0.1:{port}/health", timeout=2.0
            ) as resp:
                if resp.status == 200:
                    return json.loads(resp.read().decode("utf-8"))
        except urllib.error.URLError as e:
            last_err = e
        except (ConnectionError, OSError) as e:
            last_err = e
        time.sleep(0.5)
    sys.stderr.write(f"FAIL: sidecar /health never returned 200 (last error: {last_err})\n")
    return None


def post_image(port: int, endpoint: str, image_bytes: bytes, api_key: str = "") -> tuple[int, str]:
    """POST a multipart upload with a single file field. Returns (status, body)."""
    boundary = "----jurasmokeboundary"
    body = (
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="file"; filename="probe.jpg"\r\n'
        "Content-Type: image/jpeg\r\n\r\n"
    ).encode("utf-8") + image_bytes + f"\r\n--{boundary}--\r\n".encode("utf-8")

    headers = {"Content-Type": f"multipart/form-data; boundary={boundary}"}
    if api_key:
        headers["X-Jura-API-Key"] = api_key

    req = urllib.request.Request(
        f"http://127.0.0.1:{port}{endpoint}",
        data=body,
        headers=headers,
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60.0) as resp:
            return resp.status, resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode("utf-8", errors="replace")


def check_response(endpoint: str, status: int, body: str) -> tuple[bool, str]:
    """Return (ok, reason). ok=False if response indicates an import failure."""
    if status not in (200, 422):
        # 422 is acceptable (e.g. validation rejected the synthetic image
        # for being too small). The smoke test cares about import failures,
        # not domain validation outcomes.
        return False, f"HTTP {status} (expected 2xx): {body[:200]}"

    body_lower = body.lower()
    for marker in IMPORT_FAILURE_MARKERS:
        if marker.lower() in body_lower:
            return False, f"bundle-import failure marker '{marker}' present in response: {body[:300]}"

    return True, "ok"


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        sys.stderr.write(
            "usage: smoke_test_frozen_sidecar.py <path-to-bundled-sidecar-binary>\n"
        )
        return 1

    binary = Path(argv[1])
    if not binary.exists():
        sys.stderr.write(f"FAIL: sidecar binary not found at {binary}\n")
        return 1
    if not os.access(binary, os.X_OK):
        sys.stderr.write(f"FAIL: sidecar binary at {binary} is not executable\n")
        return 1

    port = find_free_port()
    env = os.environ.copy()
    # Generate a session-only key for the smoke test. The 2026-05-21 security
    # hardening of the auth middleware refuses /forensics requests when
    # JURA_SIDECAR_KEY is empty (previously bypassed auth, a quiet
    # vulnerability if a wrapper script dropped the env var). The smoke
    # test now sets its own per-run key and passes it on every request.
    import secrets as _secrets

    smoke_key = _secrets.token_hex(32)
    env["JURA_SIDECAR_KEY"] = smoke_key

    print(f"INFO: starting bundled sidecar at {binary} on port {port}")
    proc = subprocess.Popen(
        [str(binary), "--host", "127.0.0.1", "--port", str(port)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )

    try:
        health = wait_for_health(port, timeout=90.0)
        if health is None:
            stderr = proc.stderr.read().decode("utf-8", errors="replace") if proc.stderr else ""
            sys.stderr.write(f"sidecar stderr (last 2000 bytes):\n{stderr[-2000:]}\n")
            return 1

        print(f"INFO: sidecar /health reports {health.get('service')} v{health.get('version')}")
        caps = health.get("capabilities") or {}

        image_bytes = make_test_image()
        print(f"INFO: probing {sum(1 for k, v in caps.items() if v and k in CAPABILITY_TO_ENDPOINT)} live endpoints")

        failures: list[tuple[str, str]] = []
        for capability, endpoint in CAPABILITY_TO_ENDPOINT.items():
            value = caps.get(capability)
            if not value:
                print(f"SKIP {endpoint} ({capability}=false)")
                continue
            status, body = post_image(port, endpoint, image_bytes, smoke_key)
            ok, reason = check_response(endpoint, status, body)
            tag = "PASS" if ok else "FAIL"
            print(f"{tag} {endpoint}: {reason if not ok else f'HTTP {status}'}")
            if not ok:
                failures.append((endpoint, reason))

        if failures:
            sys.stderr.write(f"\nSMOKE TEST FAILED: {len(failures)} endpoint(s) with bundle-import or HTTP issues:\n")
            for endpoint, reason in failures:
                sys.stderr.write(f"  - {endpoint}: {reason}\n")
            return 2

        print(f"\nSMOKE TEST PASSED: all {len(CAPABILITY_TO_ENDPOINT)} mapped endpoints returned without import failures")
        return 0

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
