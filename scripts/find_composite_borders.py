#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Composite Border Detection

Walks an authentic image corpus and flags images that are likely
"stacked composite" marketing/editorial imagery (e.g. multiple photos
layered diagonally with visible borders behind the main image). These
images are poison data for the deepfake classifier:

- Sharp diagonal boundaries trip splice_boundary, segmented_ela, and
  edge_dir_entropy detectors
- They are technically composites being labelled as "authentic"
- CLIP semantic features see them as designed editorial content
  rather than candid photographs

Usage:
    python scripts/find_composite_borders.py \\
        --corpus "/Volumes/MAC SSD/Training Data/corpus/training/authentic" \\
        --output corpus/border_candidates.csv

    # Move flagged images to corpus/excluded/composite_borders/
    python scripts/find_composite_borders.py \\
        --corpus "/Volumes/MAC SSD/Training Data/corpus/training/authentic" \\
        --move \\
        --threshold 0.6

Detection signals (combined into a 0–1 composite_score):
    1. Hough diagonal lines    — straight lines at non-axis angles
       (5°–85° from horizontal) suggest rotated rectangular borders
       from layered photos
    2. Corner colour clustering — sample 4 corner patches; if a corner
       has 2+ distinct colour clusters with a sharp boundary, it likely
       contains a layered-photo edge
    3. Edge-density ratio       — Canny edges in the outer 10% rim vs
       the central 80%; composite layers create a higher rim density
"""

import argparse
import csv
import shutil
import sys
from pathlib import Path

try:
    import cv2
    import numpy as np
except ImportError:
    print("ERROR: opencv-python and numpy required.")
    print("  pip install opencv-python numpy")
    sys.exit(1)


IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".tiff", ".tif"}


def collect_images(directory: Path) -> list[Path]:
    """Walk the corpus and return all image file paths."""
    if not directory.exists():
        return []
    return sorted(
        f for f in directory.rglob("*")
        if f.is_file() and f.suffix.lower() in IMAGE_EXTENSIONS
    )


def inset_quadrilateral_score(grey: np.ndarray, img: np.ndarray) -> float:
    """
    Detect a quadrilateral contour that is clearly INSET from the image
    border — the unique signature of a composite where a foreground photo
    sits on top of background layers.

    Real photos may contain rectangles (windows, signs, doors) but those
    are small relative to the image and don't form a single inset
    quadrilateral covering most of the frame.

    Returns a score in [0, 1]:
        0.0 = no inset quadrilateral found
        1.0 = strong inset quadrilateral with very different
              inside-vs-outside content
    """
    h, w = grey.shape
    img_area = h * w

    # Strong edge map — looking for the structural rectangle border
    edges = cv2.Canny(grey, 50, 150)
    # Dilate slightly to close small gaps in border lines
    edges_dilated = cv2.dilate(edges, np.ones((3, 3), np.uint8), iterations=1)

    contours, _ = cv2.findContours(
        edges_dilated, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE
    )
    if not contours:
        return 0.0

    best_score = 0.0
    for contour in contours:
        area = cv2.contourArea(contour)
        # Quadrilateral must cover 30%-90% of the image area
        if area < img_area * 0.30 or area > img_area * 0.90:
            continue

        # Approximate the contour to a polygon
        epsilon = 0.02 * cv2.arcLength(contour, True)
        approx = cv2.approxPolyDP(contour, epsilon, True)

        # Must be a quadrilateral (4 vertices)
        if len(approx) != 4:
            continue

        # Get bounding rect and check it's clearly inset
        x, y, rw, rh = cv2.boundingRect(approx)
        inset_top = y / h
        inset_left = x / w
        inset_right = (w - x - rw) / w
        inset_bottom = (h - y - rh) / h

        # At least 2 sides must be inset by >5% (foreground photo
        # sitting on visible background layers)
        inset_count = sum(1 for i in [inset_top, inset_left, inset_right, inset_bottom] if i > 0.05)
        if inset_count < 2:
            continue

        # Average inset depth (more inset = more visible background = stronger signal)
        avg_inset = sum([inset_top, inset_left, inset_right, inset_bottom]) / 4.0

        # Compare inside-vs-outside colour statistics: composites have
        # very different content above/below the rectangle
        mask = np.zeros((h, w), dtype=np.uint8)
        cv2.drawContours(mask, [approx], -1, 255, -1)
        inside_pixels = img[mask > 0]
        outside_pixels = img[mask == 0]

        if len(inside_pixels) < 100 or len(outside_pixels) < 100:
            continue

        inside_mean = np.mean(inside_pixels, axis=0)
        outside_mean = np.mean(outside_pixels, axis=0)
        inside_std = np.std(inside_pixels, axis=0).mean()
        outside_std = np.std(outside_pixels, axis=0).mean()

        # Mean colour distance between inside/outside (max ~441 for RGB)
        colour_diff = float(np.linalg.norm(inside_mean - outside_mean))

        # Score combines inset depth + colour difference
        # avg_inset of 0.10 + colour_diff of 60 = ~0.5
        # avg_inset of 0.20 + colour_diff of 100 = ~1.0
        inset_component = min(1.0, avg_inset / 0.20)
        colour_component = min(1.0, colour_diff / 100.0)
        score = 0.5 * inset_component + 0.5 * colour_component

        if score > best_score:
            best_score = score

    return best_score


def corner_colour_score(img: np.ndarray) -> float:
    """
    Sample the four corner regions and check for multiple distinct
    colour clusters.

    A natural photograph corner shows continuous colour variation.
    A composite layered corner shows two distinct colour bands meeting
    at a sharp boundary (the layer edge).

    Returns a score in [0, 1] = average of the four corner anomaly scores.
    """
    h, w = img.shape[:2]
    patch_size = max(48, min(h, w) // 12)
    if patch_size > min(h, w) // 2:
        return 0.0

    corners = [
        img[0:patch_size, 0:patch_size],                      # top-left
        img[0:patch_size, w - patch_size:w],                  # top-right
        img[h - patch_size:h, 0:patch_size],                  # bottom-left
        img[h - patch_size:h, w - patch_size:w],              # bottom-right
    ]

    scores = []
    for patch in corners:
        if patch.size == 0:
            continue
        # Compress the patch and compute colour entropy via histogram peaks
        small = cv2.resize(patch, (32, 32))
        flat = small.reshape(-1, 3).astype(np.float32)

        # k=2 clustering: are there 2 distinct colour modes?
        try:
            _, labels, centres = cv2.kmeans(
                flat,
                K=2,
                bestLabels=None,
                criteria=(cv2.TERM_CRITERIA_EPS + cv2.TERM_CRITERIA_MAX_ITER, 10, 1.0),
                attempts=3,
                flags=cv2.KMEANS_PP_CENTERS,
            )
        except cv2.error:
            scores.append(0.0)
            continue

        # Distance between the two cluster centres in RGB space (max ~441)
        cluster_dist = np.linalg.norm(centres[0] - centres[1])

        # Balance of cluster sizes (50/50 = max anomaly, 95/5 = no signal)
        labels_flat = labels.flatten()
        c0 = float(np.sum(labels_flat == 0)) / len(labels_flat)
        c1 = 1.0 - c0
        balance = 1.0 - abs(c0 - c1)  # 1.0 when 50/50, 0.0 when 100/0

        # A natural corner has clusters that are CLOSE in colour space
        # (continuous variation). A composite corner has FAR clusters
        # that are also reasonably balanced.
        # Threshold: distance > 80 + balance > 0.4 = anomaly
        if cluster_dist > 80 and balance > 0.4:
            score = min(1.0, (cluster_dist / 200.0) * balance)
        else:
            score = 0.0
        scores.append(score)

    if not scores:
        return 0.0
    return float(np.mean(scores))


def rim_edge_density_score(grey: np.ndarray) -> float:
    """
    Compare edge density in the outer 10% rim vs the central 80% area.

    Authentic photographs have edges distributed roughly uniformly
    (or concentrated in the centre on the subject). Composite borders
    create a high concentration of edges in the rim.

    Returns a score in [0, 1].
    """
    h, w = grey.shape
    edges = cv2.Canny(grey, 80, 200)

    rim_h = max(1, h // 10)
    rim_w = max(1, w // 10)

    rim_pixels = (
        int(np.sum(edges[:rim_h, :]))
        + int(np.sum(edges[h - rim_h:, :]))
        + int(np.sum(edges[rim_h:h - rim_h, :rim_w]))
        + int(np.sum(edges[rim_h:h - rim_h, w - rim_w:]))
    )
    total_pixels = int(np.sum(edges))

    if total_pixels == 0:
        return 0.0

    rim_area = (h * rim_h * 2) + ((h - 2 * rim_h) * rim_w * 2)
    total_area = h * w

    rim_density = rim_pixels / max(rim_area, 1)
    overall_density = total_pixels / max(total_area, 1)

    if overall_density == 0:
        return 0.0

    ratio = rim_density / overall_density
    # Ratio > 1.5 = rim has 50% more edges per pixel than the average
    if ratio < 1.3:
        return 0.0
    # Ratio of 2.5+ = strong signal (3.0 maps to 1.0)
    return min(1.0, (ratio - 1.3) / 1.7)


def composite_score(image_path: Path) -> dict | None:
    """
    Compute the composite-border score for a single image.

    Returns a dict with sub-scores and the combined score, or None
    if the image cannot be loaded.
    """
    try:
        img = cv2.imread(str(image_path), cv2.IMREAD_COLOR)
        if img is None:
            return None
    except Exception:
        return None

    h, w = img.shape[:2]
    if h < 64 or w < 64:
        return None

    # Resize if very large for performance (max 1024 long edge)
    long_edge = max(h, w)
    if long_edge > 1024:
        scale = 1024.0 / long_edge
        new_w = int(w * scale)
        new_h = int(h * scale)
        img = cv2.resize(img, (new_w, new_h), interpolation=cv2.INTER_AREA)

    grey = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)

    quad_score = inset_quadrilateral_score(grey, img)
    c_score = corner_colour_score(img)
    r_score = rim_edge_density_score(grey)

    # Combined: inset quadrilateral is the strongest signal — it is
    # specific to the composite-layered pattern. Corner colour and
    # rim edge density act as supporting evidence.
    combined = (0.70 * quad_score) + (0.20 * c_score) + (0.10 * r_score)

    return {
        "filename": image_path.name,
        "path": str(image_path),
        "inset_quadrilateral": round(quad_score, 3),
        "corner_colour": round(c_score, 3),
        "rim_edge_density": round(r_score, 3),
        "composite_score": round(combined, 3),
    }


def main():
    parser = argparse.ArgumentParser(
        description="Detect composite-border images in an authentic corpus"
    )
    parser.add_argument(
        "--corpus",
        type=str,
        default="/Volumes/MAC SSD/Training Data/corpus/training/authentic",
        help="Path to the authentic image corpus",
    )
    parser.add_argument(
        "--output",
        type=str,
        default="corpus/border_candidates.csv",
        help="Output CSV path",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.45,
        help="composite_score threshold for flagging (default 0.45)",
    )
    parser.add_argument(
        "--move",
        action="store_true",
        help="Move flagged images to corpus/excluded/composite_borders/",
    )
    parser.add_argument(
        "--quarantine-dir",
        type=str,
        default="/Volumes/MAC SSD/Training Data/corpus/excluded/composite_borders",
        help="Where to move flagged images (with --move)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=0,
        help="Process only the first N images (0 = all)",
    )
    parser.add_argument(
        "--html",
        type=str,
        default=None,
        help="Optional HTML preview path for visual review",
    )
    args = parser.parse_args()

    corpus_dir = Path(args.corpus)
    images = collect_images(corpus_dir)
    if not images:
        print(f"ERROR: no images found in {corpus_dir}")
        sys.exit(1)

    if args.limit > 0:
        images = images[: args.limit]

    print(f"  Scanning {len(images)} images in {corpus_dir}")
    print(f"  Threshold: {args.threshold}")
    if args.move:
        print(f"  Move mode ON → {args.quarantine_dir}")
    print()

    rows = []
    flagged = []
    for i, img_path in enumerate(images, start=1):
        result = composite_score(img_path)
        if result is None:
            continue
        rows.append(result)
        if result["composite_score"] >= args.threshold:
            flagged.append(result)
        if i % 200 == 0:
            print(f"  [{i}/{len(images)}] processed, {len(flagged)} flagged so far...")

    # Write CSV
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    rows.sort(key=lambda r: r["composite_score"], reverse=True)
    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "filename",
                "path",
                "composite_score",
                "inset_quadrilateral",
                "corner_colour",
                "rim_edge_density",
            ],
        )
        writer.writeheader()
        writer.writerows(rows)

    print(f"\n  CSV written: {output_path}")
    print(f"  Total processed: {len(rows)}")
    print(f"  Flagged (>={args.threshold}): {len(flagged)}")

    if flagged:
        print(f"\n  Top 15 flagged candidates:")
        for r in flagged[:15]:
            print(
                f"    {r['composite_score']:.3f}  "
                f"[q={r['inset_quadrilateral']:.2f} c={r['corner_colour']:.2f} r={r['rim_edge_density']:.2f}]  "
                f"{r['filename']}"
            )

        # Per-source breakdown
        from collections import Counter
        sources = Counter()
        for r in flagged:
            parts = Path(r["path"]).parts
            # Find the directory after "authentic"
            try:
                idx = parts.index("authentic")
                source = parts[idx + 1] if idx + 1 < len(parts) else "(root)"
            except ValueError:
                source = "(unknown)"
            sources[source] += 1
        print("\n  Flagged by source directory:")
        for source, count in sources.most_common():
            total_in_source = sum(
                1 for r in rows
                if Path(r["path"]).parts[Path(r["path"]).parts.index("authentic") + 1] == source
                if "authentic" in Path(r["path"]).parts
            )
            pct = (count / total_in_source * 100) if total_in_source > 0 else 0
            print(f"    {source:25s} {count:5d} flagged / {total_in_source:5d} total  ({pct:.1f}%)")

    if args.html and flagged:
        # Generate HTML preview using base64 thumbnails (no external paths)
        import base64
        from io import BytesIO

        html_path = Path(args.html)
        html_path.parent.mkdir(parents=True, exist_ok=True)

        thumbs_html = []
        for r in flagged:
            try:
                img = cv2.imread(r["path"], cv2.IMREAD_COLOR)
                if img is None:
                    continue
                # Resize to 240px max edge for the preview
                h, w = img.shape[:2]
                long_edge = max(h, w)
                if long_edge > 240:
                    scale = 240.0 / long_edge
                    img = cv2.resize(img, (int(w * scale), int(h * scale)))
                _, buf = cv2.imencode(".jpg", img, [cv2.IMWRITE_JPEG_QUALITY, 75])
                b64 = base64.b64encode(buf.tobytes()).decode("ascii")
                thumbs_html.append(
                    f'<div class="card"><img src="data:image/jpeg;base64,{b64}"/>'
                    f'<div class="meta"><strong>{r["filename"]}</strong><br>'
                    f'score: {r["composite_score"]:.3f}<br>'
                    f'q={r["inset_quadrilateral"]:.2f} '
                    f'c={r["corner_colour"]:.2f} '
                    f'r={r["rim_edge_density"]:.2f}</div></div>'
                )
            except Exception as e:
                print(f"  WARN: thumbnail failed for {r['filename']}: {e}")

        html = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>Border Candidates Review</title>
<style>
body {{ font-family: -apple-system, sans-serif; background: #1E2128; color: #EDEAE4; padding: 20px; }}
h1 {{ color: #5A85B5; }}
.grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px; }}
.card {{ background: #272B34; border-radius: 8px; overflow: hidden; padding: 8px; border: 1px solid #3a3f4a; }}
.card img {{ width: 100%; border-radius: 4px; }}
.meta {{ font-size: 12px; padding: 8px 4px 4px; color: #9B9890; line-height: 1.5; }}
.meta strong {{ color: #EDEAE4; word-break: break-all; }}
</style></head><body>
<h1>Composite Border Candidates ({len(flagged)} flagged)</h1>
<p>Threshold: {args.threshold} · Sorted by composite_score (highest first)</p>
<div class="grid">
{"".join(thumbs_html)}
</div></body></html>
"""
        html_path.write_text(html)
        print(f"\n  HTML preview written: {html_path}")

    if args.move and flagged:
        quarantine = Path(args.quarantine_dir)
        quarantine.mkdir(parents=True, exist_ok=True)
        moved = 0
        for r in flagged:
            src = Path(r["path"])
            dst = quarantine / src.name
            try:
                shutil.move(str(src), str(dst))
                moved += 1
            except Exception as e:
                print(f"  WARN: failed to move {src.name}: {e}")
        print(f"\n  Moved {moved} images to {quarantine}")


if __name__ == "__main__":
    main()
