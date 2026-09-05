# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Segmented Error Level Analysis (Segmented ELA).

Divides an image into an 8x8 grid and computes ELA per cell, flagging
anomalous regions where compression artefacts differ significantly from
the image-wide average. Clusters of 3+ adjacent anomalous cells are
particularly suspicious as they suggest a spliced region.

Algorithm:
1. Load image, recompress as JPEG at target quality.
2. Compute pixel-wise absolute difference.
3. Divide into 8x8 grid; compute mean difference per cell.
4. Flag cells >2 sigma above the global cell mean as anomalous.
5. Detect clusters of adjacent anomalous cells via flood-fill.
6. Score based on anomaly count and clustering.
"""

import base64

import cv2
import numpy as np
from PIL import Image
from io import BytesIO


def perform_segmented_ela(image_bytes: bytes, quality: int = 90) -> dict:
    """
    Segment image into grid cells and compute ELA per cell.
    Anomalous cells identified by 2-sigma deviation from cell mean.
    Clusters of 3+ adjacent anomalous cells are flagged.

    Args:
        image_bytes: Raw bytes of the input image.
        quality: JPEG recompression quality (1-100).

    Returns:
        Dict with heatmap, per-region scores, anomaly counts, and summary.
    """
    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        return _neutral_result("Could not decode image")

    h, w = img.shape[:2]

    # Check if JPEG — for non-JPEG formats return a neutral result so that
    # no spurious score reaches the Rust pipeline. PIL cannot open AVIF/HEIC
    # (raises UnidentifiedImageError), which the original code swallowed in
    # the `except Exception: pass` branch and then fell through to the full
    # OpenCV ELA computation, producing a bogus score (Finding 2). The fix:
    # treat any PIL open failure as "not JPEG" and return neutral immediately.
    # AVIF/HEIC magic bytes are also checked explicitly as a belt-and-braces
    # guard for environments where PIL might gain AVIF support in future.
    _AVIF_MAGIC = b"ftyp"  # bytes 4-8 of AVIF/HEIF container
    _HEIC_BRANDS = {
        b"heic",
        b"heix",
        b"hevc",
        b"hevx",
        b"heim",
        b"heis",
        b"hevm",
        b"hevs",
        b"mif1",
        b"msf1",
    }
    if len(image_bytes) >= 12 and image_bytes[4:8] == _AVIF_MAGIC:
        brand = image_bytes[8:12].lower()
        if brand in _HEIC_BRANDS or b"avif" in image_bytes[8:12].lower():
            return _neutral_result(
                "Segmented ELA is not applicable for AVIF/HEIC images"
            )
    try:
        pil_img = Image.open(BytesIO(image_bytes))
        if pil_img.format and pil_img.format.upper() not in ("JPEG", "JPG"):
            return _neutral_result(
                "Segmented ELA is not applicable for non-JPEG images"
            )
    except Exception:
        # PIL could not open the image (e.g. AVIF/HEIC without a plugin, or
        # corrupt data). Treat as non-JPEG — return neutral rather than
        # proceeding to OpenCV and generating a spurious score.
        return _neutral_result(
            "Segmented ELA is not applicable: could not identify image format"
        )

    # Recompress at target quality
    encode_param = [int(cv2.IMWRITE_JPEG_QUALITY), quality]
    _, encoded = cv2.imencode(".jpg", img, encode_param)
    recompressed = cv2.imdecode(np.frombuffer(encoded, np.uint8), cv2.IMREAD_COLOR)

    # Compute absolute difference
    diff = cv2.absdiff(img, recompressed).astype(np.float64)

    # Grid parameters
    grid_rows, grid_cols = 8, 8
    cell_h = h // grid_rows
    cell_w = w // grid_cols

    if cell_h < 1 or cell_w < 1:
        return _neutral_result("Image too small for segmented ELA")

    regions = []
    ela_scores = []

    for row in range(grid_rows):
        for col in range(grid_cols):
            y1 = row * cell_h
            x1 = col * cell_w
            y2 = min(y1 + cell_h, h)
            x2 = min(x1 + cell_w, w)

            cell_diff = diff[y1:y2, x1:x2]
            cell_score = float(np.mean(cell_diff))
            ela_scores.append(cell_score)

            regions.append(
                {
                    "x": int(x1),
                    "y": int(y1),
                    "width": int(x2 - x1),
                    "height": int(y2 - y1),
                    "ela_score": round(cell_score, 4),
                    "anomalous": False,  # Set below
                }
            )

    # Statistical anomaly detection — 2-sigma
    mean_score = np.mean(ela_scores)
    std_score = np.std(ela_scores)
    threshold = mean_score + 2.0 * std_score if std_score > 0 else mean_score + 1.0

    for i, region in enumerate(regions):
        region["anomalous"] = bool(ela_scores[i] > threshold)

    anomalous_count = sum(1 for r in regions if r["anomalous"])

    # Check for clusters of 3+ adjacent anomalous cells
    has_cluster = _check_anomalous_clusters(regions, grid_rows, grid_cols)

    # Inter-region variance
    inter_variance = float(np.var(ela_scores))

    # Score: based on cluster presence and anomaly count
    if anomalous_count == 0:
        score = 0.0
    elif has_cluster:
        score = min(0.3 + (anomalous_count / len(regions)) * 2.0, 1.0)
    else:
        score = min((anomalous_count / len(regions)) * 1.5, 0.6)

    suspicious = score > 0.4 and has_cluster

    # Generate heatmap
    heatmap = _generate_ela_heatmap(diff, h, w)

    summary = f"{anomalous_count}/{len(regions)} regions show anomalous ELA levels"
    if has_cluster:
        summary += " with spatial clustering"
    if suspicious:
        summary += ". Pattern suggests possible composite"
    else:
        summary += ". No significant regional inconsistency detected"

    return {
        "heatmap_base64": heatmap,
        "regions": regions,
        "anomalous_regions": anomalous_count,
        "total_regions": len(regions),
        "inter_region_variance": round(inter_variance, 4),
        "score": round(score, 4),
        "suspicious": suspicious,
        "summary": summary,
    }


def _check_anomalous_clusters(regions: list[dict], rows: int, cols: int) -> bool:
    """Check if anomalous cells form clusters of 3+."""
    grid = np.zeros((rows, cols), dtype=bool)
    for i, r in enumerate(regions):
        row, col = divmod(i, cols)
        grid[row, col] = r["anomalous"]

    visited = np.zeros_like(grid)

    def flood_fill(r: int, c: int) -> int:
        if r < 0 or r >= rows or c < 0 or c >= cols:
            return 0
        if visited[r, c] or not grid[r, c]:
            return 0
        visited[r, c] = True
        return (
            1
            + flood_fill(r + 1, c)
            + flood_fill(r - 1, c)
            + flood_fill(r, c + 1)
            + flood_fill(r, c - 1)
        )

    for r in range(rows):
        for c in range(cols):
            if grid[r, c] and not visited[r, c]:
                cluster_size = flood_fill(r, c)
                if cluster_size >= 3:
                    return True
    return False


def _generate_ela_heatmap(diff: np.ndarray, h: int, w: int) -> str:
    """Generate a heatmap from the ELA difference image."""
    grey = np.mean(diff, axis=2).astype(np.float64)
    # Amplify for visibility
    grey = np.clip(grey * 20, 0, 255).astype(np.uint8)
    heatmap = cv2.applyColorMap(grey, cv2.COLORMAP_JET)
    _, buf = cv2.imencode(".png", heatmap)
    return base64.b64encode(buf).decode("utf-8")


def _neutral_result(summary: str) -> dict:
    return {
        "heatmap_base64": None,
        "regions": [],
        "anomalous_regions": 0,
        "total_regions": 0,
        "inter_region_variance": 0.0,
        "score": 0.0,
        "suspicious": False,
        "summary": summary,
    }
