#!/usr/bin/env python3
"""
Jura Trace — Watermark Robustness Testing

Tests how well the DWT-DCT-SVD watermark survives common image degradations:
JPEG recompression, resizing, cropping, and screenshot simulation.

Usage:
    python scripts/test_watermark_robustness.py --corpus corpus/authentic
    python scripts/test_watermark_robustness.py --corpus corpus/authentic --max 20
"""

import argparse
import base64
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

import cv2
import numpy as np

SIDECAR = "http://127.0.0.1:8200"
PAYLOAD = "JuralabsCIC"


def check_sidecar(base_url: str) -> bool:
    """Check if the sidecar is running."""
    try:
        req = Request(f"{base_url}/health", headers={"Accept": "application/json"})
        with urlopen(req, timeout=5) as resp:
            data = json.loads(resp.read())
            return data.get("status") == "ok"
    except Exception:
        return False


def multipart_post(url: str, file_bytes: bytes, filename: str = "image.png",
                   content_type: str = "image/png") -> dict | None:
    """Send a multipart/form-data POST request with a file field."""
    boundary = "----JuraTraceWatermarkTest"
    body = (
        f"--{boundary}\r\n"
        f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'
        f"Content-Type: {content_type}\r\n\r\n"
    ).encode() + file_bytes + f"\r\n--{boundary}--\r\n".encode()

    req = Request(
        url,
        data=body,
        headers={
            "Content-Type": f"multipart/form-data; boundary={boundary}",
            "Accept": "application/json",
        },
        method="POST",
    )

    try:
        with urlopen(req, timeout=120) as resp:
            return json.loads(resp.read())
    except (HTTPError, URLError, Exception) as e:
        return {"error": str(e)}


def embed_watermark(image_bytes: bytes, base_url: str, payload: str = PAYLOAD,
                    strength: str = "medium") -> bytes | None:
    """Embed watermark via sidecar, return watermarked PNG bytes or None."""
    params = urlencode({"payload": payload, "strength": strength})
    url = f"{base_url}/forensics/watermark/embed?{params}"
    result = multipart_post(url, image_bytes)

    if result and result.get("success") and result.get("watermarked_image_base64"):
        return base64.b64decode(result["watermarked_image_base64"])
    return None


def extract_watermark(image_bytes: bytes, base_url: str,
                      payload_length: int = None) -> str | None:
    """Extract watermark via sidecar, return extracted payload or None."""
    if payload_length is None:
        payload_length = len(PAYLOAD.encode("utf-8"))
    params = urlencode({"payload_length": payload_length})
    url = f"{base_url}/forensics/watermark/extract?{params}"
    result = multipart_post(url, image_bytes)

    if result and result.get("success") and result.get("has_watermark"):
        return result.get("extracted_payload")
    return None


# --- Degradation functions ---

def degrade_jpeg(img: np.ndarray, quality: int) -> tuple[np.ndarray, bytes]:
    """Recompress as JPEG at given quality."""
    _, buf = cv2.imencode(".jpg", img, [cv2.IMWRITE_JPEG_QUALITY, quality])
    decoded = cv2.imdecode(np.frombuffer(buf, np.uint8), cv2.IMREAD_COLOR)
    return decoded, buf.tobytes()


def degrade_resize_roundtrip(img: np.ndarray, scale: float) -> tuple[np.ndarray, bytes]:
    """Resize down by scale factor, then back to original size."""
    h, w = img.shape[:2]
    small = cv2.resize(img, (int(w * scale), int(h * scale)),
                       interpolation=cv2.INTER_AREA)
    restored = cv2.resize(small, (w, h), interpolation=cv2.INTER_CUBIC)
    _, buf = cv2.imencode(".png", restored)
    return restored, buf.tobytes()


def degrade_resize(img: np.ndarray, scale: float) -> tuple[np.ndarray, bytes]:
    """Resize by scale factor (no round-trip)."""
    h, w = img.shape[:2]
    resized = cv2.resize(img, (int(w * scale), int(h * scale)),
                         interpolation=cv2.INTER_AREA)
    _, buf = cv2.imencode(".png", resized)
    return resized, buf.tobytes()


def degrade_crop_centre(img: np.ndarray, fraction: float = 0.2) -> tuple[np.ndarray, bytes]:
    """Centre crop, removing `fraction` from each edge pair."""
    h, w = img.shape[:2]
    margin_h = int(h * fraction / 2)
    margin_w = int(w * fraction / 2)
    cropped = img[margin_h:h - margin_h, margin_w:w - margin_w]
    _, buf = cv2.imencode(".png", cropped)
    return cropped, buf.tobytes()


def degrade_screenshot(img: np.ndarray) -> tuple[np.ndarray, bytes]:
    """Simulate screenshot: slight resize + JPEG Q80."""
    h, w = img.shape[:2]
    scaled = cv2.resize(img, (int(w * 0.95), int(h * 0.95)),
                        interpolation=cv2.INTER_AREA)
    _, buf = cv2.imencode(".jpg", scaled, [cv2.IMWRITE_JPEG_QUALITY, 80])
    decoded = cv2.imdecode(np.frombuffer(buf, np.uint8), cv2.IMREAD_COLOR)
    return decoded, buf.tobytes()


# Degradation test suite
DEGRADATIONS = [
    ("Original (PNG)", None),
    ("JPEG Q95", lambda img: degrade_jpeg(img, 95)),
    ("JPEG Q85", lambda img: degrade_jpeg(img, 85)),
    ("JPEG Q70", lambda img: degrade_jpeg(img, 70)),
    ("JPEG Q60", lambda img: degrade_jpeg(img, 60)),
    ("Resize 50%->100%", lambda img: degrade_resize_roundtrip(img, 0.5)),
    ("Resize 75%", lambda img: degrade_resize(img, 0.75)),
    ("Crop 20%", lambda img: degrade_crop_centre(img, 0.2)),
    ("Screenshot sim", lambda img: degrade_screenshot(img)),
]


def main():
    parser = argparse.ArgumentParser(
        description="Watermark robustness testing against image degradations"
    )
    parser.add_argument(
        "--corpus", type=str, default="corpus/authentic",
        help="Path to corpus directory (default: corpus/authentic)"
    )
    parser.add_argument(
        "--sidecar", type=str, default=SIDECAR,
        help="Sidecar URL (default: http://127.0.0.1:8200)"
    )
    parser.add_argument(
        "--max", type=int, default=20,
        help="Max images to process (default: 20)"
    )
    parser.add_argument(
        "--payload", type=str, default=PAYLOAD,
        help="Watermark payload string (default: JuralabsCIC)"
    )
    parser.add_argument(
        "--strength", type=str, default="medium",
        choices=["low", "medium", "high"],
        help="Watermark embedding strength (default: medium)"
    )
    parser.add_argument(
        "--output", type=str, default="docs/watermark-robustness-report.json",
        help="Output JSON report path"
    )
    args = parser.parse_args()

    corpus_dir = Path(args.corpus)
    if not corpus_dir.exists():
        print(f"Error: corpus directory not found: {corpus_dir}")
        sys.exit(1)

    payload = args.payload
    payload_length = len(payload.encode("utf-8"))

    # Header
    print()
    print("Jura Trace — Watermark Robustness Testing")
    print("=" * 50)
    print(f"  Corpus:   {corpus_dir}")
    print(f"  Sidecar:  {args.sidecar}")
    print(f"  Payload:  \"{payload}\" ({payload_length} bytes)")
    print(f"  Strength: {args.strength}")

    # Check sidecar
    if not check_sidecar(args.sidecar):
        print(f"\nError: sidecar not responding at {args.sidecar}")
        print("Start it with: cd sidecar && uvicorn main:app --host 127.0.0.1 --port 8200")
        sys.exit(1)
    print("  Sidecar:  connected")

    # Find images
    image_extensions = {".jpg", ".jpeg", ".png", ".tiff", ".webp"}
    images = sorted([
        f for f in corpus_dir.iterdir()
        if f.suffix.lower() in image_extensions
    ])

    if args.max > 0:
        images = images[:args.max]

    total = len(images)
    print(f"  Images:   {total}")
    print()

    if total == 0:
        print("No images found in corpus.")
        sys.exit(1)

    # Results tracking: {degradation_name: [bool, ...]}
    results = {name: [] for name, _ in DEGRADATIONS}
    per_image_results = []
    errors = []

    for i, image_path in enumerate(images):
        print(f"  [{i + 1}/{total}] {image_path.name}", flush=True)

        # Read original image
        with open(image_path, "rb") as f:
            original_bytes = f.read()

        # Step 1: embed watermark
        watermarked_png = embed_watermark(
            original_bytes, args.sidecar, payload=payload, strength=args.strength
        )
        if watermarked_png is None:
            print(f"         SKIP — embedding failed")
            errors.append({"file": image_path.name, "error": "embedding failed"})
            continue

        # Decode watermarked PNG for degradation
        wm_arr = np.frombuffer(watermarked_png, np.uint8)
        wm_img = cv2.imdecode(wm_arr, cv2.IMREAD_COLOR)
        if wm_img is None:
            print(f"         SKIP — could not decode watermarked image")
            errors.append({"file": image_path.name, "error": "decode failed"})
            continue

        image_result = {"file": image_path.name, "degradations": {}}

        for deg_name, deg_fn in DEGRADATIONS:
            try:
                if deg_fn is None:
                    # Test the original watermarked PNG
                    test_bytes = watermarked_png
                else:
                    _, test_bytes = deg_fn(wm_img)

                extracted = extract_watermark(
                    test_bytes, args.sidecar, payload_length=payload_length
                )
                survived = extracted is not None and payload in extracted
                results[deg_name].append(survived)
                image_result["degradations"][deg_name] = {
                    "survived": survived,
                    "extracted": extracted,
                }
            except Exception as e:
                results[deg_name].append(False)
                image_result["degradations"][deg_name] = {
                    "survived": False,
                    "error": str(e),
                }

        per_image_results.append(image_result)

        # Print per-image summary
        survived_count = sum(
            1 for name, _ in DEGRADATIONS
            if image_result["degradations"].get(name, {}).get("survived", False)
        )
        print(f"         {survived_count}/{len(DEGRADATIONS)} degradations survived")

        time.sleep(0.1)

    # Summary table
    tested = len(per_image_results)
    print()
    print()
    print("WATERMARK ROBUSTNESS RESULTS")
    print("=" * 50)
    print(f"Corpus: {tested} images (of {total} selected)")
    print(f"Payload: \"{payload}\"")
    print(f"Strength: {args.strength}")
    print()
    print(f"  {'Degradation':<22s}  {'Survived':>10s}  {'Rate':>6s}")
    print(f"  {'-' * 22}  {'-' * 10}  {'-' * 6}")

    summary_rows = []
    for deg_name, _ in DEGRADATIONS:
        deg_results = results[deg_name]
        if deg_results:
            survived = sum(deg_results)
            total_tested = len(deg_results)
            rate = survived / total_tested * 100
            print(f"  {deg_name:<22s}  {survived:>3d}/{total_tested:<4d}     {rate:5.1f}%")
            summary_rows.append({
                "degradation": deg_name,
                "survived": survived,
                "total": total_tested,
                "rate_pct": round(rate, 1),
            })
        else:
            print(f"  {deg_name:<22s}  {'N/A':>10s}  {'N/A':>6s}")
            summary_rows.append({
                "degradation": deg_name,
                "survived": 0,
                "total": 0,
                "rate_pct": 0.0,
            })

    if errors:
        print()
        print(f"Errors/skips: {len(errors)}")
        for err in errors:
            print(f"  {err['file']}: {err['error']}")

    # Save JSON report
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    report = {
        "generated_at": datetime.now().isoformat(),
        "corpus_dir": str(corpus_dir),
        "corpus_size": tested,
        "payload": payload,
        "payload_length_bytes": payload_length,
        "strength": args.strength,
        "summary": summary_rows,
        "per_image": per_image_results,
        "errors": errors,
    }
    output_path.write_text(json.dumps(report, indent=2))
    print()
    print(f"Report saved: {output_path}")


if __name__ == "__main__":
    main()
