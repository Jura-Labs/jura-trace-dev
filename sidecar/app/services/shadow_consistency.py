"""
Jura Trace Sidecar — Shadow/Lighting Direction Consistency Analysis.

Estimates the dominant light direction across image regions using gradient
analysis. Detects when regions have incompatible shadow/light directions,
which may indicate compositing from multiple source images with different
lighting conditions.

Algorithm:
1. Convert to greyscale, compute Sobel gradients.
2. Compute magnitude-weighted circular mean of gradient angles globally.
3. Segment foreground into connected components via Otsu threshold.
4. For each component, compute local light direction.
5. Flag components with >80 degree deviation from the global direction.
"""

import base64

import cv2
import numpy as np


def perform_shadow_consistency(image_bytes: bytes) -> dict:
    """
    Estimate dominant light direction across image regions using gradient
    analysis. Detect when regions have incompatible shadow/light directions.

    Args:
        image_bytes: Raw bytes of the input image.

    Returns:
        Dict with heatmap, per-region directions, inconsistency counts, and summary.
    """
    nparr = np.frombuffer(image_bytes, np.uint8)
    img = cv2.imdecode(nparr, cv2.IMREAD_COLOR)
    if img is None:
        return _neutral_result("Could not decode image")

    h, w = img.shape[:2]
    grey = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY).astype(np.float64)

    # Compute gradients
    grad_x = cv2.Sobel(grey, cv2.CV_64F, 1, 0, ksize=3)
    grad_y = cv2.Sobel(grey, cv2.CV_64F, 0, 1, ksize=3)
    magnitude = np.sqrt(grad_x**2 + grad_y**2)
    angle = np.arctan2(grad_y, grad_x) * 180 / np.pi  # -180 to 180

    # Global dominant light direction (magnitude-weighted circular mean)
    global_direction = _weighted_circular_mean(angle, magnitude)

    # Segment into foreground components using Otsu threshold
    grey_u8 = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    _, binary = cv2.threshold(
        grey_u8, 0, 255, cv2.THRESH_BINARY + cv2.THRESH_OTSU
    )

    # Find connected components
    num_labels, labels, stats, centroids = cv2.connectedComponentsWithStats(
        binary, connectivity=8
    )

    regions = []
    min_area = (h * w) * 0.03  # Minimum 3% of image area

    for label_id in range(1, num_labels):  # Skip background (0)
        area = stats[label_id, cv2.CC_STAT_AREA]
        if area < min_area:
            continue

        x = int(stats[label_id, cv2.CC_STAT_LEFT])
        y = int(stats[label_id, cv2.CC_STAT_TOP])
        rw = int(stats[label_id, cv2.CC_STAT_WIDTH])
        rh = int(stats[label_id, cv2.CC_STAT_HEIGHT])

        # Extract region mask
        mask = labels[y : y + rh, x : x + rw] == label_id
        region_angle = angle[y : y + rh, x : x + rw][mask]
        region_mag = magnitude[y : y + rh, x : x + rw][mask]

        if len(region_angle) < 100:
            continue

        # Regional light direction
        region_direction = _weighted_circular_mean(region_angle, region_mag)

        # Circular deviation from global
        deviation = abs(_circular_difference(region_direction, global_direction))

        inconsistent = deviation > 80.0

        regions.append({
            "x": x,
            "y": y,
            "width": rw,
            "height": rh,
            "area": int(area),
            "gradient_angle_mean": round(region_direction, 2),
            "deviation_from_global": round(deviation, 2),
            "inconsistent": inconsistent,
        })

    inconsistent_count = sum(1 for r in regions if r["inconsistent"])
    total = len(regions)

    if total < 3:
        return _neutral_result(
            "Insufficient foreground components for shadow analysis"
        )

    score = (
        min(inconsistent_count / max(total, 1) * 0.6, 1.0)
        if inconsistent_count > 0
        else 0.0
    )
    suspicious = inconsistent_count >= 2 and score > 0.4

    # Generate heatmap showing gradient directions
    heatmap = _generate_shadow_heatmap(angle, magnitude, h, w)

    summary = f"Global light direction: {global_direction:.0f} degrees. "
    if inconsistent_count > 0:
        summary += (
            f"{inconsistent_count}/{total} regions show inconsistent "
            f"shadow direction (>80 degree deviation)"
        )
        if suspicious:
            summary += ". Pattern suggests possible composite"
    else:
        summary += f"All {total} foreground regions consistent"

    return {
        "heatmap_base64": heatmap,
        "global_light_direction": round(global_direction, 2),
        "regions": regions,
        "inconsistent_regions": inconsistent_count,
        "total_regions": total,
        "score": round(score, 4),
        "suspicious": suspicious,
        "summary": summary,
    }


def _weighted_circular_mean(
    angles: np.ndarray, weights: np.ndarray
) -> float:
    """Compute magnitude-weighted circular mean of angles in degrees."""
    rad = np.radians(angles)
    w = weights / (np.sum(weights) + 1e-10)
    sin_mean = np.sum(w * np.sin(rad))
    cos_mean = np.sum(w * np.cos(rad))
    return float(np.degrees(np.arctan2(sin_mean, cos_mean)))


def _circular_difference(a: float, b: float) -> float:
    """Compute smallest circular difference between two angles."""
    diff = (a - b + 180) % 360 - 180
    return diff


def _generate_shadow_heatmap(
    angle: np.ndarray, magnitude: np.ndarray, h: int, w: int
) -> str:
    """Visualise gradient direction as HSV heatmap."""
    # Normalise angle to 0-180 for HSV hue
    hue = ((angle + 180) / 360 * 179).astype(np.uint8)
    sat = np.full_like(hue, 255)
    val = np.clip(
        magnitude / (np.max(magnitude) + 1e-10) * 255, 0, 255
    ).astype(np.uint8)

    hsv = np.stack([hue, sat, val], axis=-1)
    bgr = cv2.cvtColor(hsv, cv2.COLOR_HSV2BGR)
    _, buf = cv2.imencode(".png", bgr)
    return base64.b64encode(buf).decode("utf-8")


def _neutral_result(summary: str) -> dict:
    return {
        "heatmap_base64": None,
        "global_light_direction": 0.0,
        "regions": [],
        "inconsistent_regions": 0,
        "total_regions": 0,
        "score": 0.0,
        "suspicious": False,
        "summary": summary,
    }
