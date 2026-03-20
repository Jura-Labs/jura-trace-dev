"""
Jura Trace Sidecar — Splice Boundary Detection.

Detects splice boundaries using three orthogonal signals:
1. JPEG block grid alignment — spliced edges often align with 8x8 block
   boundaries from the source image's compression grid.
2. Noise asymmetry — spliced regions typically have different noise levels
   on each side of the boundary.
3. Feathering/blur profile — compositing tools often apply feathering or
   gaussian blur along splice edges, producing unnaturally uniform gradient
   profiles.

An edge needs 2 of 3 signals to be classified as a splice candidate,
reducing false positives from natural edges.
"""

import base64

import cv2
import numpy as np


def perform_splice_boundary(image_bytes: bytes) -> dict:
    """
    Detect splice boundaries using three signals: JPEG grid alignment,
    noise asymmetry, and feathering profile.

    Args:
        image_bytes: Raw bytes of the input image.

    Returns:
        Dict with heatmap, detected boundaries, scores, and summary.
    """
    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        return _neutral_result("Could not decode image")

    h, w = img.shape[:2]
    grey = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)

    # Canny edge detection
    edges = cv2.Canny(grey, 50, 150)

    # Find contours from edges
    contours, _ = cv2.findContours(
        edges, cv2.RETR_LIST, cv2.CHAIN_APPROX_SIMPLE
    )

    # Filter to significant contours (>20px arc length)
    min_length = 20
    significant_contours = [
        c for c in contours if cv2.arcLength(c, False) >= min_length
    ]

    boundaries = []

    # Cap at 100 to limit processing time
    for contour in significant_contours[:100]:
        x, y, cw, ch = cv2.boundingRect(contour)

        if cw < 5 or ch < 5:
            continue

        # Signal 1: JPEG block grid alignment
        jpeg_aligned = _check_jpeg_grid_alignment(contour)

        # Signal 2: Noise asymmetry
        noise_asym = _check_noise_asymmetry(grey, contour, h, w)

        # Signal 3: Feathering detection
        feathered = _check_feathering(grey, contour, h, w)

        signals = int(jpeg_aligned) + int(noise_asym) + int(feathered)

        if signals >= 2:  # Two-of-three criterion
            confidence = signals / 3.0
            boundaries.append({
                "x": int(x),
                "y": int(y),
                "width": int(cw),
                "height": int(ch),
                "jpeg_grid_aligned": jpeg_aligned,
                "noise_asymmetric": noise_asym,
                "feathering_detected": feathered,
                "signals_triggered": signals,
                "confidence": round(confidence, 2),
            })

    suspicious_count = len(boundaries)
    total_checked = len(significant_contours[:100])

    if total_checked == 0:
        return _neutral_result("No significant edge contours found")

    score = (
        min(suspicious_count / max(total_checked, 1) * 5.0, 1.0)
        if suspicious_count > 0
        else 0.0
    )
    suspicious = suspicious_count >= 2 and score > 0.3

    heatmap = _generate_boundary_heatmap(img, boundaries, edges)

    summary = f"Checked {total_checked} edge contours. "
    if suspicious_count > 0:
        summary += (
            f"{suspicious_count} potential splice boundaries detected "
            "(2+ of 3 signals each)"
        )
        if suspicious:
            summary += ". Pattern suggests possible image compositing"
    else:
        summary += "No splice boundary candidates found"

    return {
        "heatmap_base64": heatmap,
        "boundaries": boundaries,
        "suspicious_boundaries": suspicious_count,
        "total_boundaries_checked": total_checked,
        "score": round(score, 4),
        "suspicious": suspicious,
        "summary": summary,
    }


def _check_jpeg_grid_alignment(contour: np.ndarray) -> bool:
    """Check if contour aligns with JPEG 8x8 block grid."""
    points = contour.reshape(-1, 2)
    total = len(points)
    if total == 0:
        return False

    # Count points within 2px of a multiple-of-8 coordinate
    x_coords = points[:, 0]
    y_coords = points[:, 1]

    # Distance to nearest grid line for each coordinate
    x_dist = np.mod(x_coords, 8)
    x_dist = np.minimum(x_dist, 8 - x_dist)
    y_dist = np.mod(y_coords, 8)
    y_dist = np.minimum(y_dist, 8 - y_dist)

    aligned_x = int(np.sum(x_dist <= 2))
    aligned_y = int(np.sum(y_dist <= 2))

    alignment_ratio = (aligned_x + aligned_y) / (2 * total)
    return alignment_ratio > 0.4  # 40% of points near grid lines


def _check_noise_asymmetry(
    grey: np.ndarray, contour: np.ndarray, h: int, w: int
) -> bool:
    """Check for noise level difference across the contour boundary."""
    # Create mask for the contour region
    mask = np.zeros((h, w), dtype=np.uint8)
    cv2.drawContours(mask, [contour], 0, 255, thickness=16)  # 16px strip

    # Split into inner and outer by dilating/eroding
    inner_mask = np.zeros_like(mask)
    cv2.drawContours(
        inner_mask, [contour], 0, 255, thickness=-1
    )  # Fill inside
    outer_mask = mask & ~inner_mask
    inner_strip = mask & inner_mask

    if np.sum(inner_strip > 0) < 50 or np.sum(outer_mask > 0) < 50:
        return False

    # Compute Laplacian variance (noise estimate) for each side
    laplacian = cv2.Laplacian(grey, cv2.CV_64F)

    inner_noise = np.var(laplacian[inner_strip > 0])
    outer_noise = np.var(laplacian[outer_mask > 0])

    if outer_noise == 0 or inner_noise == 0:
        return False

    ratio = max(inner_noise, outer_noise) / min(inner_noise, outer_noise)
    return bool(ratio > 2.0)


def _check_feathering(
    grey: np.ndarray, contour: np.ndarray, h: int, w: int
) -> bool:
    """Detect unnatural feathering/blur profile at the boundary."""
    # Sample gradient magnitude along the contour
    points = contour.reshape(-1, 2)
    if len(points) < 10:
        return False

    # Sample every 3rd point to reduce computation
    sample_points = points[::3]

    grad_x = cv2.Sobel(grey, cv2.CV_64F, 1, 0, ksize=3)
    grad_y = cv2.Sobel(grey, cv2.CV_64F, 0, 1, ksize=3)
    grad_mag = np.sqrt(grad_x**2 + grad_y**2)

    edge_gradients = []
    for pt in sample_points:
        x, y = int(pt[0]), int(pt[1])
        if 2 <= x < w - 2 and 2 <= y < h - 2:
            # Sample a 5x5 patch gradient profile
            patch = grad_mag[y - 2 : y + 3, x - 2 : x + 3]
            edge_gradients.append(np.std(patch))

    if len(edge_gradients) < 5:
        return False

    # Feathered edges have unusually smooth gradient profiles
    # (low variance of gradient STDs along the contour)
    gradient_cv = np.std(edge_gradients) / (np.mean(edge_gradients) + 1e-10)

    # Very uniform gradient profile suggests artificial feathering
    return gradient_cv < 0.3


def _generate_boundary_heatmap(
    img: np.ndarray, boundaries: list[dict], edges: np.ndarray
) -> str:
    """Visualise detected splice boundaries on the image."""
    vis = img.copy()
    # Draw edges faintly
    edge_overlay = np.zeros_like(img)
    edge_overlay[edges > 0] = [64, 64, 64]
    vis = cv2.addWeighted(vis, 0.7, edge_overlay, 0.3, 0)

    # Draw suspicious boundaries in red
    for b in boundaries:
        x, y, bw, bh = b["x"], b["y"], b["width"], b["height"]
        colour = (
            (0, 0, 220) if b["signals_triggered"] >= 3 else (0, 100, 220)
        )
        cv2.rectangle(vis, (x, y), (x + bw, y + bh), colour, 2)

    _, buf = cv2.imencode(".png", vis)
    return base64.b64encode(buf).decode("utf-8")


def _neutral_result(summary: str) -> dict:
    return {
        "heatmap_base64": None,
        "boundaries": [],
        "suspicious_boundaries": 0,
        "total_boundaries_checked": 0,
        "score": 0.0,
        "suspicious": False,
        "summary": summary,
    }
