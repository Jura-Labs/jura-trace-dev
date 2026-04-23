#!/usr/bin/env python3
"""
Quick corpus detection scan — calls the sidecar deepfake endpoint directly.
Produces a summary table of AI/Authentic classification across the corpus.

Does NOT require the Tauri app or REST API — only the Python sidecar on :8200.

Usage:
    python scripts/corpus_scan.py
    python scripts/corpus_scan.py --max 50
    python scripts/corpus_scan.py --dir /Volumes/Samsung\ USB/Training\ Data/corpus/training
"""

import argparse
import json
import os
import sys
import time
from collections import defaultdict
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

SIDECAR_URL = os.getenv("JURA_SIDECAR_URL", "http://127.0.0.1:8200")
CORPUS_BASE = Path(
    os.getenv(
        "JURA_CORPUS_BASE",
        "/Volumes/MAC SSD/Training Data/corpus",
    )
)
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".tiff", ".tif", ".bmp"}


def check_sidecar() -> bool:
    try:
        req = Request(f"{SIDECAR_URL}/health", method="GET")
        resp = urlopen(req, timeout=5)
        data = json.loads(resp.read())
        print(f"  Sidecar: {data.get('status')} (v{data.get('version', '?')})")
        caps = data.get("capabilities", {})
        print(f"  Deepfake: {caps.get('deepfake')}, CLIP: {caps.get('clip_detect')}, ELA: {caps.get('ela')}")
        return data.get("status") == "ok"
    except Exception as e:
        print(f"  Sidecar not available: {e}")
        return False


def call_deepfake(filepath: Path) -> dict | None:
    """Call the sidecar deepfake endpoint with a file."""
    try:
        boundary = "----JuraCorpusScan"
        file_bytes = filepath.read_bytes()
        filename = filepath.name

        body_parts = []
        body_parts.append(f"--{boundary}\r\n".encode())
        body_parts.append(
            f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'.encode()
        )
        body_parts.append(b"Content-Type: application/octet-stream\r\n\r\n")
        body_parts.append(file_bytes)
        body_parts.append(b"\r\n")
        body_parts.append(f"--{boundary}--\r\n".encode())
        body = b"".join(body_parts)

        url = f"{SIDECAR_URL}/forensics/deepfake?mime_type=image%2Fjpeg&has_camera_exif=false&camera_authenticity_bonus=0.0"
        req = Request(url, data=body, method="POST")
        req.add_header("Content-Type", f"multipart/form-data; boundary={boundary}")

        resp = urlopen(req, timeout=60)
        return json.loads(resp.read())
    except Exception as e:
        return {"error": str(e)}


def call_clip(filepath: Path) -> dict | None:
    """Call the sidecar CLIP detection endpoint."""
    try:
        boundary = "----JuraCorpusScan"
        file_bytes = filepath.read_bytes()
        filename = filepath.name

        body_parts = []
        body_parts.append(f"--{boundary}\r\n".encode())
        body_parts.append(
            f'Content-Disposition: form-data; name="file"; filename="{filename}"\r\n'.encode()
        )
        body_parts.append(b"Content-Type: application/octet-stream\r\n\r\n")
        body_parts.append(file_bytes)
        body_parts.append(b"\r\n")
        body_parts.append(f"--{boundary}--\r\n".encode())
        body = b"".join(body_parts)

        url = f"{SIDECAR_URL}/forensics/clip-detect"
        req = Request(url, data=body, method="POST")
        req.add_header("Content-Type", f"multipart/form-data; boundary={boundary}")

        resp = urlopen(req, timeout=30)
        return json.loads(resp.read())
    except Exception as e:
        return {"error": str(e)}


def collect_files(base: Path, max_files: int) -> list[tuple[Path, str]]:
    """Collect image files with ground-truth labels from directory structure."""
    files = []
    for ext in IMAGE_EXTENSIONS:
        for f in base.rglob(f"*{ext}"):
            path_str = str(f).lower()
            if "ai_generated" in path_str or "ai-generated" in path_str:
                label = "ai"
            elif "authentic" in path_str:
                label = "authentic"
            else:
                label = "unknown"
            files.append((f, label))

    files.sort(key=lambda x: str(x[0]))
    return files[:max_files]


def main():
    parser = argparse.ArgumentParser(description="Jura Trace — Quick Corpus Detection Scan")
    parser.add_argument("--dir", type=Path, default=CORPUS_BASE, help="Corpus root directory")
    parser.add_argument("--max", type=int, default=9999, help="Max files to scan")
    parser.add_argument("--subdirs", nargs="+", default=["training", "test_set", "sprint29_validation"],
                        help="Subdirectories to scan")
    parser.add_argument("--output", type=Path, default=None, help="Output JSON path")
    parser.add_argument("--clip", action="store_true", help="Also run CLIP/UnivFD detection")
    args = parser.parse_args()

    print("=" * 65)
    print("  Jura Trace — Corpus Detection Scan (sidecar-direct)")
    print("=" * 65)

    if not check_sidecar():
        sys.exit(1)

    # Collect files from specified subdirectories
    all_files = []
    for subdir in args.subdirs:
        d = args.dir / subdir
        if d.exists():
            files = collect_files(d, args.max - len(all_files))
            print(f"  {subdir}: {len(files)} images")
            all_files.extend(files)
        else:
            print(f"  {subdir}: not found at {d}")

    if not all_files:
        print("\n  No files found.")
        sys.exit(1)

    print(f"\n  Total: {len(all_files)} images to scan")
    print("-" * 65)

    # Run detection
    results = []
    stats = defaultdict(lambda: defaultdict(int))
    errors = 0
    t_start = time.time()

    for i, (filepath, label) in enumerate(all_files):
        t0 = time.time()
        df_result = call_deepfake(filepath)
        clip_result = call_clip(filepath) if args.clip else None
        elapsed = time.time() - t0

        if df_result and "error" not in df_result:
            score = df_result.get("score", 0)
            verdict = df_result.get("confidence", "unknown")
            suspicious = df_result.get("suspicious", False)

            # Determine predicted class
            if suspicious:
                predicted = "ai"
            else:
                predicted = "authentic"

            # Track stats
            stats[label][predicted] += 1

            # Determine correctness
            if label == "ai" and predicted == "ai":
                status = "TP"  # true positive (correctly detected AI)
            elif label == "authentic" and predicted == "authentic":
                status = "TN"  # true negative (correctly passed authentic)
            elif label == "authentic" and predicted == "ai":
                status = "FP"  # false positive (authentic flagged as AI)
            elif label == "ai" and predicted == "authentic":
                status = "FN"  # false negative (AI missed)
            else:
                status = "??"

            clip_score = None
            clip_verdict = None
            if clip_result and "error" not in clip_result:
                clip_score = clip_result.get("score")
                clip_verdict = clip_result.get("verdict")

            row = {
                "file": str(filepath.relative_to(args.dir)),
                "label": label,
                "predicted": predicted,
                "status": status,
                "deepfake_score": round(score, 4),
                "deepfake_verdict": verdict,
                "clip_score": round(clip_score, 4) if clip_score is not None else None,
                "clip_verdict": clip_verdict,
                "time_ms": round(elapsed * 1000),
            }
            results.append(row)

            marker = {"TP": "+", "TN": ".", "FP": "!", "FN": "x"}.get(status, "?")
            if (i + 1) % 20 == 0 or status in ("FP", "FN"):
                print(f"  [{i+1:4d}/{len(all_files)}] {status} {marker} {score:.3f} {filepath.name[:40]:<40} ({elapsed:.1f}s)")
        else:
            errors += 1
            err_msg = df_result.get("error", "unknown") if df_result else "no response"
            if (i + 1) % 50 == 0:
                print(f"  [{i+1:4d}/{len(all_files)}] ERR {filepath.name[:40]} — {err_msg[:60]}")
            results.append({
                "file": str(filepath.relative_to(args.dir)),
                "label": label,
                "predicted": "error",
                "status": "ERR",
                "error": err_msg,
            })

    total_time = time.time() - t_start

    # Summary
    print("\n" + "=" * 65)
    print("  DETECTION SUMMARY")
    print("=" * 65)

    tp = stats["ai"]["ai"]
    fn = stats["ai"]["authentic"]
    tn = stats["authentic"]["authentic"]
    fp = stats["authentic"]["ai"]
    unk_ai = stats["unknown"]["ai"]
    unk_auth = stats["unknown"]["authentic"]

    total_ai = tp + fn
    total_auth = tn + fp
    recall = tp / total_ai * 100 if total_ai > 0 else 0
    fpr = fp / total_auth * 100 if total_auth > 0 else 0
    precision = tp / (tp + fp) * 100 if (tp + fp) > 0 else 0
    accuracy = (tp + tn) / (tp + tn + fp + fn) * 100 if (tp + tn + fp + fn) > 0 else 0

    print(f"\n  AI images:        {total_ai:4d}  (TP: {tp}, FN: {fn})")
    print(f"  Authentic images: {total_auth:4d}  (TN: {tn}, FP: {fp})")
    if unk_ai + unk_auth > 0:
        print(f"  Unknown label:    {unk_ai + unk_auth:4d}  (→AI: {unk_ai}, →Auth: {unk_auth})")
    print(f"  Errors:           {errors:4d}")
    print(f"\n  AI Recall:        {recall:6.1f}%  (of {total_ai} AI images correctly detected)")
    print(f"  Authentic FPR:    {fpr:6.1f}%  (of {total_auth} authentic images wrongly flagged)")
    print(f"  Precision:        {precision:6.1f}%")
    print(f"  Accuracy:         {accuracy:6.1f}%")
    print(f"\n  Total time:       {total_time:.0f}s ({total_time/len(all_files):.1f}s/image)")
    print(f"  Images scanned:   {len(all_files)}")

    # FP and FN details
    fps = [r for r in results if r.get("status") == "FP"]
    fns = [r for r in results if r.get("status") == "FN"]

    if fps:
        print(f"\n  FALSE POSITIVES ({len(fps)}):")
        for r in fps[:20]:
            print(f"    {r['deepfake_score']:.3f}  {r['file']}")

    if fns:
        print(f"\n  FALSE NEGATIVES ({len(fns)}):")
        for r in fns[:20]:
            print(f"    {r['deepfake_score']:.3f}  {r['file']}")

    # Save results
    output = args.output or (args.dir / "results" / f"corpus_scan_{int(time.time())}.json")
    output.parent.mkdir(parents=True, exist_ok=True)
    summary = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "corpus_dir": str(args.dir),
        "total_files": len(all_files),
        "errors": errors,
        "ai_images": total_ai,
        "authentic_images": total_auth,
        "tp": tp, "fn": fn, "tn": tn, "fp": fp,
        "recall_pct": round(recall, 2),
        "fpr_pct": round(fpr, 2),
        "precision_pct": round(precision, 2),
        "accuracy_pct": round(accuracy, 2),
        "total_time_s": round(total_time, 1),
        "results": results,
    }
    output.write_text(json.dumps(summary, indent=2))
    print(f"\n  Results saved to: {output}")
    print("=" * 65)


if __name__ == "__main__":
    main()
