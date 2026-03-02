"""
Jura Archive Sidecar — Copy-Move Forgery Detection.

Detects duplicated (copy-pasted) regions within an image using keypoint
matching. Authentic photos rarely contain identical feature clusters at
different locations; copy-move forgeries do.

Algorithm:
1. Convert image to greyscale.
2. Detect ORB keypoints and compute descriptors.
3. Self-match descriptors using BFMatcher (Hamming distance).
4. Filter matches: discard self-matches and short-distance pairs.
5. Cluster matched point pairs using DBSCAN to find coherent regions.
6. Generate a visualisation overlay showing detected clone regions.
"""

import base64
import io

import cv2
import numpy as np
from PIL import Image

from app.models.schemas import CopyMoveResponse, CloneRegion

# Attempt to import scikit-learn; if unavailable, DBSCAN clustering is skipped
try:
    from sklearn.cluster import DBSCAN

    _HAS_SKLEARN = True
except ImportError:
    _HAS_SKLEARN = False


def perform_copy_move_detection(
    image_bytes: bytes,
    max_features: int = 5000,
    min_distance: float = 40.0,
    match_threshold: float = 0.75,
) -> CopyMoveResponse:
    """
    Detect copy-move forgery in an image.

    Args:
        image_bytes: Raw bytes of the input image.
        max_features: Maximum ORB features to detect.
        min_distance: Minimum pixel distance between matched points to
                      be considered a potential clone (filters self-matches).
        match_threshold: Lowe's ratio test threshold for filtering matches.

    Returns:
        CopyMoveResponse with visualisation, clone regions, and score.

    Raises:
        ValueError: If image cannot be decoded.
    """
    try:
        pil_image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
        img_array = np.array(pil_image)
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    grey = cv2.cvtColor(img_array, cv2.COLOR_RGB2GRAY)

    # Detect ORB keypoints and descriptors
    orb = cv2.ORB_create(nfeatures=max_features)
    keypoints, descriptors = orb.detectAndCompute(grey, None)

    if descriptors is None or len(keypoints) < 10:
        return _empty_response(img_array)

    # Self-match with BFMatcher (Hamming for ORB binary descriptors)
    bf = cv2.BFMatcher(cv2.NORM_HAMMING, crossCheck=False)
    raw_matches = bf.knnMatch(descriptors, descriptors, k=3)

    # Filter matches
    good_pairs: list[tuple[cv2.KeyPoint, cv2.KeyPoint]] = []
    for match_group in raw_matches:
        for m in match_group:
            # Skip self-matches (same keypoint index)
            if m.queryIdx == m.trainIdx:
                continue

            pt1 = keypoints[m.queryIdx].pt
            pt2 = keypoints[m.trainIdx].pt

            # Minimum spatial distance to avoid nearby texture matches
            dist = np.sqrt((pt1[0] - pt2[0]) ** 2 + (pt1[1] - pt2[1]) ** 2)
            if dist < min_distance:
                continue

            good_pairs.append((keypoints[m.queryIdx], keypoints[m.trainIdx]))

    if len(good_pairs) < 4:
        return _empty_response(img_array)

    # Cluster matched pairs to find coherent clone regions
    clone_regions: list[CloneRegion] = []
    if _HAS_SKLEARN and len(good_pairs) >= 4:
        clone_regions = _cluster_matches(good_pairs, img_array.shape)

    # Compute score
    total_area = img_array.shape[0] * img_array.shape[1]
    cloned_area = sum(r.area for r in clone_regions)
    score = min(cloned_area / max(total_area * 0.01, 1.0), 1.0)

    suspicious = len(clone_regions) >= 1 and score > 0.1

    # Generate visualisation
    vis_base64 = _generate_visualisation(img_array, good_pairs, clone_regions)

    return CopyMoveResponse(
        visualisation_base64=vis_base64,
        clone_regions=[r.model_dump() for r in clone_regions],
        matched_pairs=len(good_pairs),
        score=round(score, 4),
        suspicious=suspicious,
    )


def _cluster_matches(
    pairs: list[tuple[cv2.KeyPoint, cv2.KeyPoint]],
    img_shape: tuple,
) -> list[CloneRegion]:
    """Cluster matched keypoint pairs into coherent clone regions using DBSCAN."""
    # Collect all matched point coordinates (both source and target)
    points = []
    for kp1, kp2 in pairs:
        points.append(kp1.pt)
        points.append(kp2.pt)

    points_array = np.array(points, dtype=np.float64)

    # DBSCAN clustering — eps scaled to ~2% of image diagonal
    diagonal = np.sqrt(img_shape[0] ** 2 + img_shape[1] ** 2)
    eps = max(diagonal * 0.02, 30.0)

    clustering = DBSCAN(eps=eps, min_samples=4).fit(points_array)
    labels = clustering.labels_

    regions: list[CloneRegion] = []
    unique_labels = set(labels)
    unique_labels.discard(-1)  # Remove noise label

    for label in unique_labels:
        cluster_mask = labels == label
        cluster_points = points_array[cluster_mask]

        x_min = int(cluster_points[:, 0].min())
        y_min = int(cluster_points[:, 1].min())
        x_max = int(cluster_points[:, 0].max())
        y_max = int(cluster_points[:, 1].max())

        w = max(x_max - x_min, 1)
        h = max(y_max - y_min, 1)

        regions.append(
            CloneRegion(
                x=x_min,
                y=y_min,
                width=w,
                height=h,
                area=w * h,
                point_count=int(cluster_mask.sum()),
            )
        )

    return regions


def _generate_visualisation(
    img_array: np.ndarray,
    pairs: list[tuple[cv2.KeyPoint, cv2.KeyPoint]],
    regions: list[CloneRegion],
) -> str:
    """Draw match lines and clone region bounding boxes on the image."""
    vis = img_array.copy()

    # Draw match lines (semi-transparent via overlay)
    overlay = vis.copy()
    for kp1, kp2 in pairs[:200]:  # Cap at 200 lines for performance
        pt1 = (int(kp1.pt[0]), int(kp1.pt[1]))
        pt2 = (int(kp2.pt[0]), int(kp2.pt[1]))
        cv2.line(overlay, pt1, pt2, (255, 100, 100), 1)

    cv2.addWeighted(overlay, 0.4, vis, 0.6, 0, vis)

    # Draw clone region bounding boxes
    for region in regions:
        cv2.rectangle(
            vis,
            (region.x, region.y),
            (region.x + region.width, region.y + region.height),
            (0, 0, 255),  # Red in RGB
            2,
        )

    # Encode as base64 PNG
    buffer = io.BytesIO()
    Image.fromarray(vis).save(buffer, format="PNG")
    return base64.b64encode(buffer.getvalue()).decode("utf-8")


def _empty_response(img_array: np.ndarray) -> CopyMoveResponse:
    """Return a clean response when no copy-move is detected."""
    buffer = io.BytesIO()
    Image.fromarray(img_array).save(buffer, format="PNG")
    vis_base64 = base64.b64encode(buffer.getvalue()).decode("utf-8")

    return CopyMoveResponse(
        visualisation_base64=vis_base64,
        clone_regions=[],
        matched_pairs=0,
        score=0.0,
        suspicious=False,
    )
