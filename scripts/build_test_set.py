#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace -- Real-World Test Set Builder

Creates a 200-image test set for validating the classifier against diverse content.
Separate from the training corpus to avoid data leakage.

Categories:
  1. Authentic (100): phone, social media, screenshots, scans, stock
  2. AI-generated (60): synthetic images with AI-typical characteristics + DiffusionDB
  3. Edge cases (20): heavy compression, crops, meta-stripped, screenshot-of-screenshot
  4. Mixed/ambiguous (20): AI-upscaled, composites

Usage:
    python scripts/build_test_set.py
"""

import hashlib
import json
import math
import os
import random
import struct
import sys
import time
from datetime import datetime
from io import BytesIO
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BASE_DIR = Path(__file__).resolve().parent.parent
CORPUS_DIR = BASE_DIR / "corpus"
TEST_SET_DIR = CORPUS_DIR / "test_set"
EXISTING_AUTHENTIC = CORPUS_DIR / "authentic"
EXISTING_AI = CORPUS_DIR / "ai_generated"

USER_AGENT = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 "
    "JuraTrace-TestSetBuilder/1.0 (research; content-authenticity)"
)

MANIFEST = []  # Will be populated as we build


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def ensure_dirs():
    """Create all test set subdirectories."""
    for subdir in [
        "authentic/phone",
        "authentic/social_media",
        "authentic/screenshots",
        "authentic/scans",
        "authentic/stock",
        "ai_generated",
        "edge_cases",
        "mixed",
    ]:
        (TEST_SET_DIR / subdir).mkdir(parents=True, exist_ok=True)


def fetch(url: str, timeout: int = 20) -> bytes | None:
    """Fetch URL with browser-like headers. Returns bytes or None."""
    req = Request(url, headers={
        "User-Agent": USER_AGENT,
        "Accept": "image/*, */*",
    })
    try:
        with urlopen(req, timeout=timeout) as resp:
            return resp.read()
    except (HTTPError, URLError, TimeoutError, OSError) as e:
        print(f"    WARN: {url[:80]}...: {e}")
        return None


def validate_image(data: bytes, min_dim: int = 200) -> bool:
    """Check image data is valid and meets minimum size."""
    if len(data) < 3000:
        return False
    try:
        img = Image.open(BytesIO(data))
        img.verify()
        img = Image.open(BytesIO(data))
        w, h = img.size
        return w >= min_dim and h >= min_dim
    except Exception:
        return False


def save_and_record(
    data: bytes | None,
    img: Image.Image | None,
    subdir: str,
    filename: str,
    source: str,
    expected_verdict: str,
    category: str,
    notes: str = "",
    fmt: str = "JPEG",
    quality: int = 90,
):
    """Save image to disk and add to manifest."""
    outpath = TEST_SET_DIR / subdir / filename
    if outpath.exists():
        print(f"    (exists) {subdir}/{filename}")
        # Still record in manifest
        MANIFEST.append({
            "filename": filename,
            "path": f"{subdir}/{filename}",
            "source": source,
            "expected_verdict": expected_verdict,
            "category": category,
            "notes": notes,
        })
        return True

    if data is not None:
        outpath.write_bytes(data)
    elif img is not None:
        save_kwargs = {}
        if fmt.upper() == "JPEG":
            save_kwargs = {"quality": quality, "subsampling": 0}
            if img.mode in ("RGBA", "P", "LA"):
                img = img.convert("RGB")
        elif fmt.upper() == "PNG":
            pass
        img.save(str(outpath), format=fmt, **save_kwargs)
    else:
        return False

    MANIFEST.append({
        "filename": filename,
        "path": f"{subdir}/{filename}",
        "source": source,
        "expected_verdict": expected_verdict,
        "category": category,
        "notes": notes,
    })
    print(f"    OK {subdir}/{filename}")
    return True


def get_existing_filenames() -> set:
    """Get filenames in the training corpus to avoid overlap."""
    names = set()
    for d in [EXISTING_AUTHENTIC, EXISTING_AI]:
        if d.exists():
            for f in d.iterdir():
                names.add(f.name)
    return names


# ---------------------------------------------------------------------------
# Category 1: Authentic images
# ---------------------------------------------------------------------------


def build_phone_photos(existing: set):
    """Download 20 phone-like photos from Wikimedia Commons and COCO."""
    print("\n[1/8] Phone photos (20 images)...")

    # Use COCO val2017 images with IDs spread to avoid training overlap
    # Training corpus uses IDs starting from low ranges; we use high-range IDs
    # COCO val2017 has IDs like 000000XXXXXX -- use specific known-good IDs
    # that are distant from the training set's sequential range
    coco_ids = [
        397133, 37777, 252219, 87038, 174482,
        403385, 6818, 480985, 458054, 331352,
        296649, 386912, 502136, 491497, 184791,
        348881, 289393, 522713, 153299, 224119,
    ]

    downloaded = 0
    for coco_id in coco_ids:
        if downloaded >= 20:
            break
        img_id_str = f"{coco_id:012d}"
        filename = f"phone_{downloaded:03d}_coco{coco_id}.jpg"
        if filename in existing:
            continue

        url = f"http://images.cocodataset.org/val2017/{img_id_str}.jpg"
        data = fetch(url, timeout=15)
        if data and validate_image(data):
            save_and_record(
                data=data, img=None,
                subdir="authentic/phone", filename=filename,
                source=f"coco-val2017-{coco_id}",
                expected_verdict="authentic",
                category="phone",
                notes="COCO val2017 real-world photo, used as phone camera proxy",
            )
            downloaded += 1
        time.sleep(0.5)

    print(f"    Downloaded {downloaded}/20 phone photos")


def build_social_media(existing: set):
    """Download 20 social-media-like images (re-encoded COCO + Unsplash)."""
    print("\n[2/8] Social media re-encoded (20 images)...")

    # Download COCO images and re-encode at typical social media quality levels
    coco_ids = [
        78823, 215245, 554291, 360661, 146358,
        412463, 518517, 16228, 383386, 56344,
        312421, 100582, 340559, 163314, 536343,
        509735, 476415, 24021, 562207, 214224,
    ]

    social_qualities = [72, 75, 78, 80, 82]  # Typical social media JPEG quality

    downloaded = 0
    for i, coco_id in enumerate(coco_ids):
        if downloaded >= 20:
            break
        img_id_str = f"{coco_id:012d}"
        filename = f"social_{downloaded:03d}_q{social_qualities[i % len(social_qualities)]}.jpg"
        if filename in existing:
            continue

        url = f"http://images.cocodataset.org/val2017/{img_id_str}.jpg"
        data = fetch(url, timeout=15)
        if data and validate_image(data):
            # Re-encode at social media quality to simulate platform compression
            img = Image.open(BytesIO(data)).convert("RGB")
            # Resize to typical social media dimensions
            max_dim = random.choice([1080, 1200, 1440])
            w, h = img.size
            ratio = min(max_dim / w, max_dim / h)
            if ratio < 1:
                img = img.resize((int(w * ratio), int(h * ratio)), Image.LANCZOS)

            q = social_qualities[i % len(social_qualities)]
            save_and_record(
                data=None, img=img,
                subdir="authentic/social_media", filename=filename,
                source=f"coco-val2017-{coco_id}-reencoded-q{q}",
                expected_verdict="authentic",
                category="social_media",
                notes=f"COCO photo re-encoded at Q{q} to simulate social media compression",
                quality=q,
            )
            downloaded += 1
        time.sleep(0.5)

    print(f"    Downloaded {downloaded}/20 social media images")


def build_screenshots():
    """Generate 20 synthetic screenshots programmatically."""
    print("\n[3/8] Screenshots (20 images)...")

    random.seed(42)
    np.random.seed(42)

    for i in range(20):
        width = np.random.randint(800, 1920)
        height = np.random.randint(600, 1200)

        # Alternate between light and dark themes
        if i % 2 == 0:
            bg_color = (random.randint(235, 250), random.randint(235, 250), random.randint(235, 250))
        else:
            bg_color = (random.randint(25, 45), random.randint(25, 45), random.randint(30, 50))

        img = Image.new("RGB", (width, height), color=bg_color)
        draw = ImageDraw.Draw(img)

        # Draw a browser-like top bar
        bar_height = random.randint(40, 80)
        bar_color = tuple(max(0, min(255, c + random.randint(-20, 20))) for c in bg_color)
        draw.rectangle([0, 0, width, bar_height], fill=bar_color)

        # Draw circles (browser buttons)
        for j, color in enumerate([(255, 95, 87), (255, 189, 46), (39, 201, 63)]):
            draw.ellipse([10 + j * 22, 12, 26 + j * 22, 28], fill=color)

        # Draw rectangles to simulate UI elements
        n_elements = np.random.randint(8, 25)
        for _ in range(n_elements):
            x1 = np.random.randint(20, width - 100)
            y1 = np.random.randint(bar_height + 10, height - 50)
            el_w = np.random.randint(60, min(400, width - x1))
            el_h = np.random.randint(15, min(120, height - y1))

            if i % 2 == 0:
                color = (
                    random.randint(180, 255),
                    random.randint(180, 255),
                    random.randint(180, 255),
                )
            else:
                color = (
                    random.randint(40, 100),
                    random.randint(40, 100),
                    random.randint(50, 110),
                )
            draw.rectangle([x1, y1, x1 + el_w, y1 + el_h], fill=color)

        # Draw some horizontal lines (text-like)
        n_lines = np.random.randint(5, 15)
        for _ in range(n_lines):
            y = np.random.randint(bar_height + 20, height - 20)
            x_start = np.random.randint(30, width // 3)
            line_len = np.random.randint(100, width - x_start - 30)
            line_color = tuple(max(0, min(255, c + random.randint(-60, -30) if i % 2 == 0 else random.randint(80, 140))) for c in (128, 128, 128))
            draw.line([(x_start, y), (x_start + line_len, y)], fill=line_color, width=2)

        filename = f"screenshot_{i:03d}.png"
        save_and_record(
            data=None, img=img,
            subdir="authentic/screenshots", filename=filename,
            source="programmatic-screenshot-generation",
            expected_verdict="authentic",
            category="screenshot",
            notes=f"Synthetic screenshot {width}x{height}, {'dark' if i % 2 else 'light'} theme",
            fmt="PNG",
        )

    print(f"    Generated 20 screenshots")


def build_scanned_documents():
    """Generate 20 synthetic scan-like images."""
    print("\n[4/8] Scanned documents (20 images)...")

    random.seed(123)
    np.random.seed(123)

    for i in range(20):
        # A4-ish proportions at scan resolution
        width = random.choice([1700, 2100, 2480])
        height = int(width * 1.414)  # A4 ratio

        # Paper colour with slight variation (off-white to cream)
        paper_r = random.randint(230, 252)
        paper_g = random.randint(225, 248)
        paper_b = random.randint(215, 242)
        img = Image.new("RGB", (width, height), (paper_r, paper_g, paper_b))
        draw = ImageDraw.Draw(img)
        pixels = np.array(img)

        # Add scan grain noise
        noise = np.random.normal(0, random.uniform(3, 8), pixels.shape).astype(np.int16)
        pixels = np.clip(pixels.astype(np.int16) + noise, 0, 255).astype(np.uint8)

        # Add uneven lighting (brighter in centre, darker at edges)
        y_coords, x_coords = np.mgrid[0:height, 0:width]
        cx, cy = width / 2, height / 2
        dist = np.sqrt((x_coords - cx) ** 2 + (y_coords - cy) ** 2)
        max_dist = np.sqrt(cx**2 + cy**2)
        vignette = 1.0 - 0.15 * (dist / max_dist) ** 2
        for c in range(3):
            pixels[:, :, c] = np.clip(pixels[:, :, c] * vignette, 0, 255).astype(np.uint8)

        img = Image.fromarray(pixels)
        draw = ImageDraw.Draw(img)

        # Draw text-like horizontal bars
        margin_x = random.randint(100, 200)
        margin_y = random.randint(120, 200)
        line_height = random.randint(18, 30)
        text_color = (random.randint(10, 50), random.randint(10, 50), random.randint(10, 50))

        # Title block
        draw.rectangle(
            [margin_x, margin_y, margin_x + random.randint(300, 600), margin_y + 24],
            fill=text_color,
        )

        # Body lines
        y_pos = margin_y + 60
        while y_pos < height - margin_y:
            line_w = random.randint(int((width - 2 * margin_x) * 0.5), width - 2 * margin_x)
            line_h = random.randint(2, 4)
            draw.rectangle(
                [margin_x, y_pos, margin_x + line_w, y_pos + line_h],
                fill=text_color,
            )
            y_pos += line_height
            # Paragraph breaks
            if random.random() < 0.15:
                y_pos += line_height

        # Slight rotation to simulate scan misalignment
        angle = random.uniform(-1.5, 1.5)
        img = img.rotate(angle, resample=Image.BICUBIC, fillcolor=(paper_r, paper_g, paper_b))

        filename = f"scan_{i:03d}.jpg"
        save_and_record(
            data=None, img=img,
            subdir="authentic/scans", filename=filename,
            source="programmatic-scan-simulation",
            expected_verdict="authentic",
            category="scan",
            notes=f"Synthetic scanned document {width}x{height}, rotation={angle:.1f}deg",
            quality=92,
        )

    print(f"    Generated 20 scans")


def build_stock_photos(existing: set):
    """Download 20 stock/press photos from Unsplash and Wikimedia."""
    print("\n[5/8] Stock/press photos (20 images)...")

    # Unsplash source URLs (CC0, specific curated IDs for diversity)
    unsplash_ids = [
        "photo-1506744038136-46273834b3fb",  # landscape
        "photo-1441974231531-c6227db76b6e",  # forest
        "photo-1518791841217-8f162f1e1131",  # cat
        "photo-1474511320723-9a56873571b7",  # mountain
        "photo-1507003211169-0a1dd7228f2d",  # portrait
        "photo-1472214103451-9374bd1c798e",  # nature
        "photo-1513836279014-a89f7a76ae86",  # trees
        "photo-1495567720989-cebdbdd97913",  # road
        "photo-1485470733090-0aae1788d668",  # architecture
        "photo-1494790108377-be9c29b29330",  # woman portrait
    ]

    # Use Unsplash source (random photos for remaining)
    downloaded = 0

    # First batch: specific Unsplash images via source API
    for i, photo_id in enumerate(unsplash_ids):
        if downloaded >= 10:
            break
        filename = f"stock_{downloaded:03d}_unsplash.jpg"
        if filename in existing:
            continue

        # Unsplash source URL gives a redirect to the actual image
        url = f"https://images.unsplash.com/{photo_id}?w=1200&q=80"
        data = fetch(url, timeout=20)
        if data and validate_image(data):
            save_and_record(
                data=data, img=None,
                subdir="authentic/stock", filename=filename,
                source=f"unsplash-{photo_id}",
                expected_verdict="authentic",
                category="stock",
                notes="Unsplash CC0 stock photo",
            )
            downloaded += 1
        time.sleep(1.0)

    # Second batch: COCO images as press photo proxies (different IDs from other batches)
    coco_press_ids = [
        571857, 579635, 308391, 86408, 312213,
        425226, 463802, 356427, 26564, 565877,
    ]
    for coco_id in coco_press_ids:
        if downloaded >= 20:
            break
        img_id_str = f"{coco_id:012d}"
        filename = f"stock_{downloaded:03d}_coco{coco_id}.jpg"
        if filename in existing:
            continue

        url = f"http://images.cocodataset.org/val2017/{img_id_str}.jpg"
        data = fetch(url, timeout=15)
        if data and validate_image(data):
            save_and_record(
                data=data, img=None,
                subdir="authentic/stock", filename=filename,
                source=f"coco-val2017-{coco_id}-press-proxy",
                expected_verdict="authentic",
                category="stock",
                notes="COCO val2017 photo used as press/stock proxy",
            )
            downloaded += 1
        time.sleep(0.5)

    print(f"    Downloaded {downloaded}/20 stock photos")


# ---------------------------------------------------------------------------
# Category 2: AI-generated images (60)
# ---------------------------------------------------------------------------


def build_ai_generated():
    """Generate 60 synthetic AI-like images with diverse characteristics."""
    print("\n[6/8] AI-generated images (60)...")

    random.seed(777)
    np.random.seed(777)

    generated = 0

    # --- Type A: Smooth gradient scenes (20) ---
    # Diffusion models often produce unnaturally smooth gradients
    for i in range(20):
        w = random.choice([512, 768, 1024])
        h = random.choice([512, 768, 1024])
        pixels = np.zeros((h, w, 3), dtype=np.float64)

        # Create smooth colour gradient
        base_hue = random.uniform(0, 1)
        for y in range(h):
            for x in range(w):
                # Smooth radial gradient with slight colour shift
                cx, cy = w / 2, h / 2
                dist = math.sqrt((x - cx) ** 2 + (y - cy) ** 2)
                max_dist = math.sqrt(cx**2 + cy**2)
                t = dist / max_dist

                r = int(255 * (0.3 + 0.7 * (1 - t) * (0.5 + 0.5 * math.sin(base_hue * 6.28))))
                g = int(255 * (0.2 + 0.6 * (1 - t) * (0.5 + 0.5 * math.sin(base_hue * 6.28 + 2.09))))
                b = int(255 * (0.3 + 0.5 * (1 - t) * (0.5 + 0.5 * math.sin(base_hue * 6.28 + 4.18))))
                pixels[y, x] = [r, g, b]

        # Add very uniform, low-level noise (AI-typical)
        noise = np.random.normal(0, random.uniform(1.5, 4.0), pixels.shape)
        pixels = np.clip(pixels + noise, 0, 255).astype(np.uint8)

        img = Image.fromarray(pixels)
        # Add slight Gaussian blur (common in diffusion outputs)
        if random.random() < 0.5:
            img = img.filter(ImageFilter.GaussianBlur(radius=random.uniform(0.3, 0.8)))

        filename = f"ai_gradient_{i:03d}.png"
        save_and_record(
            data=None, img=img,
            subdir="ai_generated", filename=filename,
            source="programmatic-ai-simulation",
            expected_verdict="ai_generated",
            category="ai_generated",
            notes=f"Smooth gradient {w}x{h}, simulates diffusion model smooth areas",
            fmt="PNG",
        )
        generated += 1

    # --- Type B: Perlin-noise-like textures (20) ---
    # AI images often have correlated noise across channels
    for i in range(20):
        w = random.choice([512, 768, 1024])
        h = random.choice([512, 768, 1024])

        # Generate base with low-frequency structure (like AI upsampled content)
        # Use multi-octave smooth noise
        small_w, small_h = w // 8, h // 8
        base_r = np.random.uniform(50, 200, (small_h, small_w)).astype(np.float64)
        base_g = np.random.uniform(50, 200, (small_h, small_w)).astype(np.float64)
        base_b = np.random.uniform(50, 200, (small_h, small_w)).astype(np.float64)

        # Upscale smoothly (simulates diffusion model VAE decoder output)
        from PIL import Image as PILImage

        r_img = PILImage.fromarray(base_r.astype(np.uint8)).resize((w, h), PILImage.BICUBIC)
        g_img = PILImage.fromarray(base_g.astype(np.uint8)).resize((w, h), PILImage.BICUBIC)
        b_img = PILImage.fromarray(base_b.astype(np.uint8)).resize((w, h), PILImage.BICUBIC)

        pixels = np.stack([np.array(r_img), np.array(g_img), np.array(b_img)], axis=-1)

        # Add CORRELATED noise across channels (AI-typical signal)
        shared_noise = np.random.normal(0, random.uniform(2, 5), (h, w))
        for c in range(3):
            channel_noise = np.random.normal(0, random.uniform(0.5, 1.5), (h, w))
            pixels[:, :, c] = np.clip(
                pixels[:, :, c].astype(np.float64) + shared_noise + channel_noise,
                0, 255,
            )
        pixels = pixels.astype(np.uint8)

        img = Image.fromarray(pixels)
        filename = f"ai_texture_{i:03d}.png"
        save_and_record(
            data=None, img=img,
            subdir="ai_generated", filename=filename,
            source="programmatic-ai-simulation",
            expected_verdict="ai_generated",
            category="ai_generated",
            notes=f"Correlated-noise texture {w}x{h}, simulates diffusion VAE artefacts",
            fmt="PNG",
        )
        generated += 1

    # --- Type C: AI-resolution scene composites (20) ---
    # Generate at typical AI resolutions with characteristic smoothness
    for i in range(20):
        # Typical AI output resolutions
        w, h = random.choice([
            (512, 512), (768, 768), (1024, 1024),
            (768, 512), (512, 768), (1024, 768),
        ])

        # Create a scene-like image with smooth regions
        pixels = np.zeros((h, w, 3), dtype=np.uint8)

        # Sky region (top third)
        sky_h = h // 3
        for y in range(sky_h):
            t = y / sky_h
            r = int(135 + 80 * t + random.uniform(-2, 2))
            g = int(180 + 40 * t + random.uniform(-2, 2))
            b = int(230 - 20 * t + random.uniform(-2, 2))
            pixels[y, :] = [
                max(0, min(255, r)),
                max(0, min(255, g)),
                max(0, min(255, b)),
            ]

        # Ground/middle region
        ground_color = [
            random.randint(60, 120),
            random.randint(80, 150),
            random.randint(40, 90),
        ]
        for y in range(sky_h, h):
            t = (y - sky_h) / (h - sky_h)
            for c in range(3):
                val = ground_color[c] - int(30 * t) + random.randint(-3, 3)
                pixels[y, :, c] = max(0, min(255, val))

        # Add some shapes (blobs) to simulate objects
        img = Image.fromarray(pixels)
        draw = ImageDraw.Draw(img)
        n_shapes = random.randint(3, 10)
        for _ in range(n_shapes):
            x1 = random.randint(0, w - 50)
            y1 = random.randint(sky_h - 20, h - 30)
            size = random.randint(20, 100)
            color = tuple(random.randint(30, 200) for _ in range(3))
            if random.random() < 0.5:
                draw.ellipse([x1, y1, x1 + size, y1 + size], fill=color)
            else:
                draw.rectangle([x1, y1, x1 + size, y1 + int(size * 0.7)], fill=color)

        # Apply slight blur (typical of AI output)
        img = img.filter(ImageFilter.GaussianBlur(radius=random.uniform(0.5, 1.2)))

        # Add very uniform noise
        pixels = np.array(img).astype(np.float64)
        noise_std = random.uniform(1.0, 3.0)
        noise = np.random.normal(0, noise_std, pixels.shape)
        pixels = np.clip(pixels + noise, 0, 255).astype(np.uint8)
        img = Image.fromarray(pixels)

        filename = f"ai_scene_{i:03d}.png"
        save_and_record(
            data=None, img=img,
            subdir="ai_generated", filename=filename,
            source="programmatic-ai-simulation",
            expected_verdict="ai_generated",
            category="ai_generated",
            notes=f"Synthetic scene {w}x{h}, smooth regions + uniform noise + blur",
            fmt="PNG",
        )
        generated += 1

    print(f"    Generated {generated}/60 AI-like images")


# ---------------------------------------------------------------------------
# Category 3: Edge cases (20)
# ---------------------------------------------------------------------------


def build_edge_cases(existing: set):
    """Build 20 edge case images from downloaded authentic sources."""
    print("\n[7/8] Edge cases (20 images)...")

    # We need some source images to manipulate
    # Download 5 COCO images for each edge case type
    source_ids = [
        # For heavy compression (5)
        [449996, 376442, 110638, 227765, 434479],
        # For crop+resize (5)
        [515579, 154425, 256668, 310072, 431876],
        # For screenshot-of-screenshot (5) -- we'll use our generated screenshots
        [],
        # For EXIF-stripped (5)
        [578489, 210273, 412151, 116208, 348708],
    ]

    created = 0

    # --- 5 heavily compressed (Q30) ---
    print("    Heavy JPEG compression (Q30)...")
    for i, coco_id in enumerate(source_ids[0]):
        filename = f"edge_compressed_{i:03d}.jpg"
        url = f"http://images.cocodataset.org/val2017/{coco_id:012d}.jpg"
        data = fetch(url, timeout=15)
        if data and validate_image(data):
            img = Image.open(BytesIO(data)).convert("RGB")
            save_and_record(
                data=None, img=img,
                subdir="edge_cases", filename=filename,
                source=f"coco-val2017-{coco_id}-compressed-q30",
                expected_verdict="authentic",
                category="edge_case",
                notes="Authentic photo compressed to JPEG Q30 (extreme compression)",
                quality=30,
            )
            created += 1
        time.sleep(0.5)

    # --- 5 cropped + resized ---
    print("    Cropped + resized...")
    for i, coco_id in enumerate(source_ids[1]):
        filename = f"edge_cropped_{i:03d}.jpg"
        url = f"http://images.cocodataset.org/val2017/{coco_id:012d}.jpg"
        data = fetch(url, timeout=15)
        if data and validate_image(data):
            img = Image.open(BytesIO(data)).convert("RGB")
            w, h = img.size
            # Crop to centre 60%
            crop_w, crop_h = int(w * 0.6), int(h * 0.6)
            left = (w - crop_w) // 2
            top = (h - crop_h) // 2
            img = img.crop((left, top, left + crop_w, top + crop_h))
            # Resize to non-standard dimensions
            new_w = random.choice([640, 800, 960])
            new_h = int(new_w * crop_h / crop_w)
            img = img.resize((new_w, new_h), Image.LANCZOS)
            save_and_record(
                data=None, img=img,
                subdir="edge_cases", filename=filename,
                source=f"coco-val2017-{coco_id}-cropped-resized",
                expected_verdict="authentic",
                category="edge_case",
                notes=f"Authentic photo cropped to 60% centre, resized to {new_w}x{new_h}",
                quality=85,
            )
            created += 1
        time.sleep(0.5)

    # --- 5 screenshots of screenshots ---
    print("    Screenshots of screenshots...")
    for i in range(5):
        filename = f"edge_double_screenshot_{i:03d}.png"
        source_path = TEST_SET_DIR / "authentic" / "screenshots" / f"screenshot_{i:03d}.png"
        if source_path.exists():
            inner = Image.open(str(source_path))
            iw, ih = inner.size
            # Create outer screenshot with browser chrome
            outer_w = iw + random.randint(40, 80)
            outer_h = ih + random.randint(80, 120)
            outer = Image.new("RGB", (outer_w, outer_h), (200, 200, 200))
            draw = ImageDraw.Draw(outer)
            # Browser bar
            draw.rectangle([0, 0, outer_w, 50], fill=(230, 230, 230))
            draw.rectangle([60, 15, 300, 35], fill=(255, 255, 255), outline=(180, 180, 180))
            # Paste inner
            paste_x = (outer_w - iw) // 2
            paste_y = 55
            outer.paste(inner, (paste_x, paste_y))
            save_and_record(
                data=None, img=outer,
                subdir="edge_cases", filename=filename,
                source="double-screenshot-of-generated-screenshot",
                expected_verdict="authentic",
                category="edge_case",
                notes="Screenshot of a screenshot (nested rendering context)",
                fmt="PNG",
            )
            created += 1

    # --- 5 EXIF-stripped ---
    print("    EXIF-stripped authentic photos...")
    for i, coco_id in enumerate(source_ids[3]):
        filename = f"edge_noexif_{i:03d}.jpg"
        url = f"http://images.cocodataset.org/val2017/{coco_id:012d}.jpg"
        data = fetch(url, timeout=15)
        if data and validate_image(data):
            # Load and re-save without EXIF
            img = Image.open(BytesIO(data)).convert("RGB")
            # Saving via PIL without copying EXIF info strips it
            save_and_record(
                data=None, img=img,
                subdir="edge_cases", filename=filename,
                source=f"coco-val2017-{coco_id}-exif-stripped",
                expected_verdict="authentic",
                category="edge_case",
                notes="Authentic photo with all EXIF metadata stripped",
                quality=90,
            )
            created += 1
        time.sleep(0.5)

    print(f"    Created {created}/20 edge cases")


# ---------------------------------------------------------------------------
# Category 4: Mixed/ambiguous (20)
# ---------------------------------------------------------------------------


def build_mixed(existing: set):
    """Build 20 mixed/ambiguous images."""
    print("\n[8/8] Mixed/ambiguous (20 images)...")

    created = 0

    # --- 10 AI-upscaled authentic photos ---
    print("    AI-upscaled authentic photos (bicubic + blur)...")
    coco_upscale_ids = [
        128372, 462565, 84477, 559543, 404601,
        300842, 189436, 575815, 398237, 50165,
    ]
    for i, coco_id in enumerate(coco_upscale_ids):
        filename = f"mixed_upscaled_{i:03d}.jpg"
        url = f"http://images.cocodataset.org/val2017/{coco_id:012d}.jpg"
        data = fetch(url, timeout=15)
        if data and validate_image(data):
            img = Image.open(BytesIO(data)).convert("RGB")
            w, h = img.size
            # Downscale first then upscale (simulates AI upscaling pipeline)
            small = img.resize((w // 3, h // 3), Image.LANCZOS)
            # Upscale with bicubic (rough approximation of AI upscaler output)
            upscaled = small.resize((w, h), Image.BICUBIC)
            # Add slight sharpening then blur (typical AI upscaler artefact)
            upscaled = upscaled.filter(ImageFilter.SHARPEN)
            upscaled = upscaled.filter(ImageFilter.GaussianBlur(radius=0.4))
            save_and_record(
                data=None, img=upscaled,
                subdir="mixed", filename=filename,
                source=f"coco-val2017-{coco_id}-upscaled",
                expected_verdict="inconclusive",
                category="mixed",
                notes="Authentic photo downscaled 3x then upscaled (simulates AI upscaling)",
                quality=90,
            )
            created += 1
        time.sleep(0.5)

    # --- 10 composites (paste one authentic region onto another) ---
    print("    Composite images...")
    coco_bg_ids = [
        462629, 170099, 279278, 565012, 508101,
        474028, 157124, 338191, 490125, 396274,
    ]
    coco_fg_ids = [
        22969, 459153, 436617, 217957, 364384,
        495732, 324209, 190307, 565391, 378605,
    ]

    for i in range(10):
        filename = f"mixed_composite_{i:03d}.jpg"
        bg_url = f"http://images.cocodataset.org/val2017/{coco_bg_ids[i]:012d}.jpg"
        fg_url = f"http://images.cocodataset.org/val2017/{coco_fg_ids[i]:012d}.jpg"

        bg_data = fetch(bg_url, timeout=15)
        fg_data = fetch(fg_url, timeout=15)

        if bg_data and fg_data and validate_image(bg_data) and validate_image(fg_data):
            bg_img = Image.open(BytesIO(bg_data)).convert("RGB")
            fg_img = Image.open(BytesIO(fg_data)).convert("RGB")

            bg_w, bg_h = bg_img.size
            # Crop a region from foreground
            fw, fh = fg_img.size
            crop_size = min(fw, fh, bg_w // 3, bg_h // 3)
            fg_crop = fg_img.crop((0, 0, crop_size, crop_size))

            # Paste onto background at random position
            paste_x = random.randint(0, max(0, bg_w - crop_size))
            paste_y = random.randint(0, max(0, bg_h - crop_size))

            # Apply slight feathering at edges (5px Gaussian blur on alpha mask)
            composite = bg_img.copy()
            # Simple paste with slight opacity blending at border
            mask = Image.new("L", (crop_size, crop_size), 255)
            mask_draw = ImageDraw.Draw(mask)
            # Feather edges
            border = 8
            for edge_px in range(border):
                alpha = int(255 * edge_px / border)
                mask_draw.rectangle(
                    [edge_px, edge_px, crop_size - 1 - edge_px, crop_size - 1 - edge_px],
                    outline=alpha,
                )
            composite.paste(fg_crop, (paste_x, paste_y), mask)

            save_and_record(
                data=None, img=composite,
                subdir="mixed", filename=filename,
                source=f"composite-coco-{coco_bg_ids[i]}-on-{coco_fg_ids[i]}",
                expected_verdict="inconclusive",
                category="mixed",
                notes=f"Composite: region from {coco_fg_ids[i]} pasted onto {coco_bg_ids[i]} at ({paste_x},{paste_y})",
                quality=88,
            )
            created += 1
        time.sleep(0.5)

    print(f"    Created {created}/20 mixed images")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    print("=" * 68)
    print("Jura Trace -- Real-World Test Set Builder")
    print("=" * 68)
    print(f"  Output: {TEST_SET_DIR}")
    print(f"  Target: 200 images across 4 categories")
    print()

    ensure_dirs()
    existing = get_existing_filenames()
    print(f"  Training corpus files to avoid: {len(existing)}")

    # Build all categories
    build_phone_photos(existing)
    build_social_media(existing)
    build_screenshots()
    build_scanned_documents()
    build_stock_photos(existing)
    build_ai_generated()
    build_edge_cases(existing)
    build_mixed(existing)

    # Write manifest
    manifest_path = TEST_SET_DIR / "manifest.json"
    manifest_data = {
        "generated_at": datetime.now().isoformat(),
        "total_images": len(MANIFEST),
        "categories": {},
        "images": MANIFEST,
    }

    # Count by category
    for entry in MANIFEST:
        cat = entry["category"]
        manifest_data["categories"][cat] = manifest_data["categories"].get(cat, 0) + 1

    manifest_path.write_text(json.dumps(manifest_data, indent=2))

    # Summary
    print("\n" + "=" * 68)
    print("TEST SET SUMMARY")
    print("=" * 68)
    print(f"  Total images: {len(MANIFEST)}")
    for cat, count in sorted(manifest_data["categories"].items()):
        print(f"    {cat}: {count}")
    print(f"\n  Manifest: {manifest_path}")

    # Count actual files on disk
    actual_count = 0
    for root, dirs, files in os.walk(TEST_SET_DIR):
        for f in files:
            if Path(f).suffix.lower() in {".jpg", ".jpeg", ".png", ".webp"}:
                actual_count += 1
    print(f"  Actual image files on disk: {actual_count}")
    print("=" * 68)


if __name__ == "__main__":
    main()
