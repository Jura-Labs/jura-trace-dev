"""
Jura Archive Sidecar — Colour Temperature Segmentation.

Analyses white balance / colour temperature per region using CIELAB colour
space. Detects discontinuous colour casts that suggest composite images
where regions originate from sources with different white balance settings.

Algorithm:
1. Convert image to CIELAB colour space.
2. Compute global mean A and B (chrominance channels).
3. Divide into a 4x4 grid of regions.
4. For each region, compute mean A and B.
5. Flag regions with >8 LAB units Euclidean deviation from the global mean.
6. Check for spatial clustering of anomalous regions.
"""

import base64

import cv2
import numpy as np


def perform_colour_temperature(image_bytes: bytes) -> dict:
    """
    Analyse white balance / colour temperature per region in CIELAB space.
    Detect discontinuous colour casts suggesting composite images.

    Args:
        image_bytes: Raw bytes of the input image.

    Returns:
        Dict with heatmap, per-region colour data, anomaly counts, and summary.
    """
    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        return _neutral_result("Could not decode image")

    h, w = img.shape[:2]

    # Convert to CIELAB
    lab = cv2.cvtColor(img, cv2.COLOR_BGR2LAB).astype(np.float64)
    # OpenCV LAB: L [0,255], A [0,255], B [0,255] — centred at 128
    # Convert to standard LAB range: L [0,100], A [-128,127], B [-128,127]
    lab[:, :, 0] = lab[:, :, 0] * 100.0 / 255.0
    lab[:, :, 1] = lab[:, :, 1] - 128.0
    lab[:, :, 2] = lab[:, :, 2] - 128.0

    # Global mean A and B
    global_a = float(np.mean(lab[:, :, 1]))
    global_b = float(np.mean(lab[:, :, 2]))

    # 4x4 grid (coarser than ELA — colour temp is scene-level)
    grid_rows, grid_cols = 4, 4
    cell_h = h // grid_rows
    cell_w = w // grid_cols

    if cell_h < 1 or cell_w < 1:
        return _neutral_result("Image too small for colour temperature analysis")

    regions = []
    deviations = []

    for row in range(grid_rows):
        for col in range(grid_cols):
            y1 = row * cell_h
            x1 = col * cell_w
            y2 = min(y1 + cell_h, h)
            x2 = min(x1 + cell_w, w)

            cell = lab[y1:y2, x1:x2]
            mean_a = float(np.mean(cell[:, :, 1]))
            mean_b = float(np.mean(cell[:, :, 2]))

            # Euclidean distance in AB plane from global mean
            deviation = float(
                np.sqrt((mean_a - global_a) ** 2 + (mean_b - global_b) ** 2)
            )
            deviations.append(deviation)

            # Threshold: 8.0 LAB units (JND boundary)
            anomalous = deviation > 8.0

            regions.append({
                "x": int(x1),
                "y": int(y1),
                "width": int(x2 - x1),
                "height": int(y2 - y1),
                "mean_a": round(mean_a, 2),
                "mean_b": round(mean_b, 2),
                "deviation_from_global": round(deviation, 2),
                "anomalous": anomalous,
            })

    anomalous_count = sum(1 for r in regions if r["anomalous"])

    # Check for clusters of adjacent anomalous regions covering >10% area
    has_significant_cluster = _check_colour_clusters(
        regions, grid_rows, grid_cols, h, w
    )

    if anomalous_count == 0:
        score = 0.0
    elif has_significant_cluster:
        score = min(0.4 + (anomalous_count / len(regions)) * 1.5, 1.0)
    else:
        score = min((anomalous_count / len(regions)) * 1.0, 0.5)

    suspicious = score > 0.4 and has_significant_cluster

    heatmap = _generate_colour_heatmap(lab, h, w, global_a, global_b)

    summary = f"Global colour: A={global_a:.1f}, B={global_b:.1f}. "
    if anomalous_count > 0:
        summary += (
            f"{anomalous_count}/{len(regions)} regions deviate >8 LAB units"
        )
        if has_significant_cluster:
            summary += " with significant spatial clustering"
        if suspicious:
            summary += (
                ". Pattern suggests possible colour temperature mismatch "
                "from compositing"
            )
    else:
        summary += (
            f"All {len(regions)} regions within normal colour temperature range"
        )

    return {
        "heatmap_base64": heatmap,
        "regions": regions,
        "anomalous_regions": anomalous_count,
        "total_regions": len(regions),
        "global_mean_a": round(global_a, 2),
        "global_mean_b": round(global_b, 2),
        "score": round(score, 4),
        "suspicious": suspicious,
        "summary": summary,
    }


def _check_colour_clusters(
    regions: list[dict], rows: int, cols: int, img_h: int, img_w: int
) -> bool:
    """Check if anomalous regions form clusters covering >10% of image."""
    grid = np.zeros((rows, cols), dtype=bool)
    for i, r in enumerate(regions):
        row, col = divmod(i, cols)
        grid[row, col] = r["anomalous"]

    visited = np.zeros_like(grid)
    total_area = img_h * img_w

    def flood_fill(r: int, c: int) -> int:
        if r < 0 or r >= rows or c < 0 or c >= cols:
            return 0
        if visited[r, c] or not grid[r, c]:
            return 0
        visited[r, c] = True
        idx = r * cols + c
        area = regions[idx]["width"] * regions[idx]["height"]
        return (
            area
            + flood_fill(r + 1, c)
            + flood_fill(r - 1, c)
            + flood_fill(r, c + 1)
            + flood_fill(r, c - 1)
        )

    for r in range(rows):
        for c in range(cols):
            if grid[r, c] and not visited[r, c]:
                cluster_area = flood_fill(r, c)
                if cluster_area > total_area * 0.10:
                    return True
    return False


def _generate_colour_heatmap(
    lab: np.ndarray, h: int, w: int, global_a: float, global_b: float
) -> str:
    """Visualise colour temperature deviation as heatmap."""
    # Compute per-pixel deviation from global mean in AB plane
    dev_a = lab[:, :, 1] - global_a
    dev_b = lab[:, :, 2] - global_b
    deviation = np.sqrt(dev_a**2 + dev_b**2)

    # Normalise to 0-255
    max_dev = (
        np.percentile(deviation, 99) if np.max(deviation) > 0 else 1.0
    )
    normalised = np.clip(deviation / max_dev * 255, 0, 255).astype(np.uint8)
    heatmap = cv2.applyColorMap(normalised, cv2.COLORMAP_JET)
    _, buf = cv2.imencode(".png", heatmap)
    return base64.b64encode(buf).decode("utf-8")


def _neutral_result(summary: str) -> dict:
    return {
        "heatmap_base64": None,
        "regions": [],
        "anomalous_regions": 0,
        "total_regions": 0,
        "global_mean_a": 0.0,
        "global_mean_b": 0.0,
        "score": 0.0,
        "suspicious": False,
        "summary": summary,
    }
