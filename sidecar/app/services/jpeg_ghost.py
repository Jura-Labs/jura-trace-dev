# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — JPEG Ghost Detection.

JPEG ghost detection identifies splice/composite forgeries by analysing
double JPEG compression artefacts. The method exploits the fact that when
a JPEG region is re-compressed at its original quality level, the
difference (error) is minimised — producing a "ghost". Spliced regions
originating from a differently-compressed source show a ghost at a
different quality level than the rest of the image.

Algorithm:
1. Load image as RGB.
2. For each quality level q in range(50, 100, 5):
   a. Re-save as JPEG at quality q.
   b. Compute pixel-wise absolute difference with original.
   c. Compute mean difference per 16x16 block.
3. For each block, find the quality that minimises the difference
   (the "ghost quality").
4. Compute statistics: mode ghost quality, variance, deviating blocks.
5. Generate a colour-coded heatmap of per-block ghost qualities.
6. Score based on variance and deviating block count.

References:
- Farid, H. (2009) — "Exposing digital forgeries from JPEG ghosts"
- https://doi.org/10.1109/TIFS.2009.2018622
"""

import base64
import io
from collections import Counter

import numpy as np
from PIL import Image

from app.models.schemas import JpegGhostResponse

# Quality levels to test
QUALITY_RANGE = list(range(50, 100, 5))

# Block size for local analysis
BLOCK_SIZE = 16

# Threshold: blocks whose ghost quality deviates by more than this many
# quality steps from the image mode are considered suspicious.
QUALITY_DEVIATION_THRESHOLD = 10


def perform_jpeg_ghost_detection(image_bytes: bytes) -> JpegGhostResponse:
    """
    Perform JPEG ghost detection on an image.

    Args:
        image_bytes: Raw bytes of the input image.

    Returns:
        JpegGhostResponse with ghost quality map, statistics, and score.

    Raises:
        ValueError: If image cannot be decoded.
    """
    # PNG images have no JPEG compression history — ghost analysis is not applicable
    if image_bytes[:4] == b'\x89PNG':
        return JpegGhostResponse(
            score=0.0,
            suspicious=False,
            ghost_quality=QUALITY_RANGE[len(QUALITY_RANGE) // 2],
            quality_variance=0.0,
            deviating_blocks=0,
            total_blocks=0,
            heatmap_base64="",
            summary="JPEG ghost analysis is not applicable for PNG images",
        )

    try:
        original = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    except Exception as exc:
        raise ValueError(f"Cannot decode image: {exc}") from exc

    orig_array = np.array(original, dtype=np.float64)
    h, w = orig_array.shape[:2]

    # Number of complete blocks in each dimension
    blocks_y = h // BLOCK_SIZE
    blocks_x = w // BLOCK_SIZE

    if blocks_y < 1 or blocks_x < 1:
        # Image too small for block analysis — return neutral result
        return _neutral_result(original)

    # Crop to exact block grid
    crop_h = blocks_y * BLOCK_SIZE
    crop_w = blocks_x * BLOCK_SIZE
    orig_cropped = orig_array[:crop_h, :crop_w]

    # ── Step 1: Compute block-wise mean difference at each quality ────
    # Shape: (num_qualities, blocks_y, blocks_x)
    block_diffs = np.zeros((len(QUALITY_RANGE), blocks_y, blocks_x), dtype=np.float64)

    for qi, q in enumerate(QUALITY_RANGE):
        # Re-compress at quality q
        buf = io.BytesIO()
        original.save(buf, format="JPEG", quality=q)
        buf.seek(0)
        recompressed = Image.open(buf).convert("RGB")
        recomp_array = np.array(recompressed, dtype=np.float64)[:crop_h, :crop_w]

        # Pixel-wise absolute difference, averaged across channels
        diff = np.abs(orig_cropped - recomp_array).mean(axis=2)

        # Block-wise mean difference
        for by in range(blocks_y):
            for bx in range(blocks_x):
                y0 = by * BLOCK_SIZE
                x0 = bx * BLOCK_SIZE
                block_diffs[qi, by, bx] = diff[y0:y0 + BLOCK_SIZE, x0:x0 + BLOCK_SIZE].mean()

    # ── Step 2: Per-block ghost quality (quality that minimises diff) ─
    ghost_quality_indices = np.argmin(block_diffs, axis=0)  # (blocks_y, blocks_x)
    ghost_qualities = np.array(QUALITY_RANGE)[ghost_quality_indices]  # actual Q values

    # ── Step 3: Statistics ────────────────────────────────────────────
    total_blocks = blocks_y * blocks_x
    flat_qualities = ghost_qualities.flatten()

    # Mode: most common ghost quality
    quality_counts = Counter(flat_qualities.tolist())
    mode_quality = int(quality_counts.most_common(1)[0][0])

    # Variance of ghost qualities across blocks
    quality_variance = float(np.var(flat_qualities))

    # Blocks deviating from mode by more than threshold
    deviating_mask = np.abs(ghost_qualities.astype(np.float64) - mode_quality) > QUALITY_DEVIATION_THRESHOLD
    deviating_blocks = int(deviating_mask.sum())

    # ── Step 4: Score ─────────────────────────────────────────────────
    # Two components:
    #   1. Normalised variance (high variance → possible splice)
    #   2. Fraction of deviating blocks
    # Max variance for range 50-95 is ~506 (half at 50, half at 95)
    max_possible_variance = 506.0
    variance_score = min(quality_variance / max_possible_variance, 1.0)
    deviation_fraction = deviating_blocks / total_blocks if total_blocks > 0 else 0.0

    score = float(np.clip(0.4 * variance_score + 0.6 * deviation_fraction, 0.0, 1.0))
    suspicious = score > 0.3

    # ── Step 5: Heatmap ───────────────────────────────────────────────
    heatmap_base64 = _generate_heatmap(ghost_qualities, mode_quality)

    # ── Step 6: Summary ───────────────────────────────────────────────
    if deviating_blocks == 0:
        summary = (
            f"Uniform compression detected. All {total_blocks} blocks show ghost "
            f"quality near Q{mode_quality}, consistent with single JPEG compression."
        )
    elif suspicious:
        summary = (
            f"Potential splice detected. {deviating_blocks}/{total_blocks} blocks "
            f"show ghost quality deviating from the dominant Q{mode_quality}. "
            f"Quality variance: {quality_variance:.1f}."
        )
    else:
        summary = (
            f"Minor compression variation detected. {deviating_blocks}/{total_blocks} "
            f"blocks deviate from dominant Q{mode_quality}. "
            f"Quality variance: {quality_variance:.1f}. Likely benign."
        )

    return JpegGhostResponse(
        score=round(score, 4),
        suspicious=suspicious,
        ghost_quality=mode_quality,
        quality_variance=round(quality_variance, 2),
        deviating_blocks=deviating_blocks,
        total_blocks=total_blocks,
        heatmap_base64=heatmap_base64,
        summary=summary,
    )


def _neutral_result(original: Image.Image) -> JpegGhostResponse:
    """Return a neutral result for images too small to analyse."""
    # Generate a tiny placeholder heatmap
    placeholder = Image.new("RGB", (1, 1), (128, 128, 128))
    buf = io.BytesIO()
    placeholder.save(buf, format="PNG")
    heatmap_b64 = base64.b64encode(buf.getvalue()).decode("utf-8")

    return JpegGhostResponse(
        score=0.0,
        suspicious=False,
        ghost_quality=QUALITY_RANGE[len(QUALITY_RANGE) // 2],
        quality_variance=0.0,
        deviating_blocks=0,
        total_blocks=0,
        heatmap_base64=heatmap_b64,
        summary="Image too small for JPEG ghost analysis.",
    )


def _generate_heatmap(
    ghost_qualities: np.ndarray,
    mode_quality: int,
) -> str:
    """
    Generate a colour-coded heatmap of per-block ghost qualities.

    Blocks matching the mode are shown in cool blue. Blocks deviating
    from the mode are shown in warm colours (yellow to red) proportional
    to deviation magnitude.

    Returns:
        Base64-encoded PNG string.
    """
    blocks_y, blocks_x = ghost_qualities.shape

    # Compute deviation from mode
    deviation = np.abs(ghost_qualities.astype(np.float64) - mode_quality)
    max_dev = max(float(deviation.max()), 1.0)

    # Normalise deviation to 0-1
    norm_dev = deviation / max_dev

    # Map to colours: blue (0) → yellow (0.5) → red (1.0)
    heatmap_rgb = np.zeros((blocks_y, blocks_x, 3), dtype=np.uint8)

    for by in range(blocks_y):
        for bx in range(blocks_x):
            t = norm_dev[by, bx]
            if t < 0.5:
                # Blue to yellow
                s = t * 2.0
                r = int(255 * s)
                g = int(255 * s)
                b = int(255 * (1.0 - s))
            else:
                # Yellow to red
                s = (t - 0.5) * 2.0
                r = 255
                g = int(255 * (1.0 - s))
                b = 0
            heatmap_rgb[by, bx] = [r, g, b]

    # Scale up so each block is visible (4x4 pixels per block)
    scale = 4
    heatmap_img = Image.fromarray(heatmap_rgb)
    heatmap_img = heatmap_img.resize(
        (blocks_x * scale, blocks_y * scale),
        resample=Image.NEAREST,
    )

    buf = io.BytesIO()
    heatmap_img.save(buf, format="PNG")
    return base64.b64encode(buf.getvalue()).decode("utf-8")
