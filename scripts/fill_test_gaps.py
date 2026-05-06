#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fill gaps in the test set where COCO IDs returned 404."""

import json
import random
import time
from datetime import datetime
from io import BytesIO
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

from PIL import Image, ImageDraw, ImageFilter

BASE_DIR = Path(__file__).resolve().parent.parent
TEST_SET_DIR = BASE_DIR / "corpus" / "test_set"
MANIFEST_PATH = TEST_SET_DIR / "manifest.json"

USER_AGENT = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)


def fetch(url, timeout=15):
    req = Request(url, headers={"User-Agent": USER_AGENT, "Accept": "*/*"})
    try:
        with urlopen(req, timeout=timeout) as resp:
            return resp.read()
    except (HTTPError, URLError, TimeoutError):
        return None


def try_coco_ids(ids, subdir, prefix, label, verdict, notes_fn, quality=90, transform_fn=None):
    """Try a list of COCO IDs until we get the needed count."""
    existing = set(f.name for f in (TEST_SET_DIR / subdir).iterdir()) if (TEST_SET_DIR / subdir).exists() else set()
    # Count how many we need based on prefix
    current = sum(1 for f in existing if f.startswith(prefix))

    new_entries = []
    idx = current
    for cid in ids:
        url = f"http://images.cocodataset.org/val2017/{cid:012d}.jpg"
        data = fetch(url)
        if data and len(data) > 5000:
            img = Image.open(BytesIO(data)).convert("RGB")
            if transform_fn:
                img = transform_fn(img, idx)
            filename = f"{prefix}{idx:03d}_coco{cid}.jpg"
            outpath = TEST_SET_DIR / subdir / filename
            if img.mode in ("RGBA", "P"):
                img = img.convert("RGB")
            img.save(str(outpath), format="JPEG", quality=quality, subsampling=0)
            new_entries.append({
                "filename": filename,
                "path": f"{subdir}/{filename}",
                "source": f"coco-val2017-{cid}",
                "expected_verdict": verdict,
                "category": label,
                "notes": notes_fn(cid, idx),
            })
            print(f"  OK {subdir}/{filename}")
            idx += 1
        time.sleep(0.5)
    return new_entries


# Known-good COCO val2017 IDs (verified from common datasets)
SPARE_IDS = [
    139, 785, 872, 1268, 1503, 1761, 2006, 2261, 2587, 2685,
    3156, 3553, 4134, 5001, 5503, 6040, 6723, 7281, 7816, 8277,
    8690, 9378, 9772, 10092, 10583, 11122, 11615, 12120, 12639, 13177,
]

print("Filling test set gaps...\n")

all_new = []

# Social media: need 4 more
def social_transform(img, idx):
    w, h = img.size
    ratio = min(1200 / w, 1200 / h)
    if ratio < 1:
        img = img.resize((int(w * ratio), int(h * ratio)), Image.LANCZOS)
    return img

all_new += try_coco_ids(
    SPARE_IDS[:6], "authentic/social_media", "social_",
    "social_media", "authentic",
    lambda cid, i: f"COCO-{cid} re-encoded Q75 social media sim",
    quality=75,
    transform_fn=social_transform,
)

# Stock: need 3 more
all_new += try_coco_ids(
    SPARE_IDS[6:12], "authentic/stock", "stock_",
    "stock", "authentic",
    lambda cid, i: f"COCO-{cid} stock/press proxy",
    quality=90,
)

# Edge cases: need 2 more (1 crop, 1 noexif)
def crop_transform(img, idx):
    w, h = img.size
    cw, ch = int(w * 0.6), int(h * 0.6)
    left, top = (w - cw) // 2, (h - ch) // 2
    img = img.crop((left, top, left + cw, top + ch))
    new_w = 800
    new_h = int(new_w * ch / cw)
    return img.resize((new_w, new_h), Image.LANCZOS)

all_new += try_coco_ids(
    SPARE_IDS[12:15], "edge_cases", "edge_cropped_",
    "edge_case", "authentic",
    lambda cid, i: f"COCO-{cid} cropped 60% centre + resized",
    quality=85,
    transform_fn=crop_transform,
)

all_new += try_coco_ids(
    SPARE_IDS[15:18], "edge_cases", "edge_noexif_",
    "edge_case", "authentic",
    lambda cid, i: f"COCO-{cid} EXIF stripped",
    quality=90,
)

# Mixed: need 3 more (1 upscale, 2 composite)
def upscale_transform(img, idx):
    w, h = img.size
    small = img.resize((w // 3, h // 3), Image.LANCZOS)
    up = small.resize((w, h), Image.BICUBIC)
    up = up.filter(ImageFilter.SHARPEN)
    return up.filter(ImageFilter.GaussianBlur(radius=0.4))

all_new += try_coco_ids(
    SPARE_IDS[18:21], "mixed", "mixed_upscaled_",
    "mixed", "inconclusive",
    lambda cid, i: f"COCO-{cid} downscaled 3x then upscaled",
    quality=90,
    transform_fn=upscale_transform,
)

# For composites, we need pairs
comp_bgs = SPARE_IDS[21:25]
comp_fgs = SPARE_IDS[25:29]
existing_composites = sum(1 for f in (TEST_SET_DIR / "mixed").iterdir() if f.name.startswith("mixed_composite"))

for idx_offset, (bg_id, fg_id) in enumerate(zip(comp_bgs, comp_fgs)):
    bg_data = fetch(f"http://images.cocodataset.org/val2017/{bg_id:012d}.jpg")
    fg_data = fetch(f"http://images.cocodataset.org/val2017/{fg_id:012d}.jpg")
    if bg_data and fg_data and len(bg_data) > 5000 and len(fg_data) > 5000:
        bg = Image.open(BytesIO(bg_data)).convert("RGB")
        fg = Image.open(BytesIO(fg_data)).convert("RGB")
        bw, bh = bg.size
        fw, fh = fg.size
        cs = min(fw, fh, bw // 3, bh // 3)
        crop = fg.crop((0, 0, cs, cs))
        px = random.randint(0, max(0, bw - cs))
        py = random.randint(0, max(0, bh - cs))
        mask = Image.new("L", (cs, cs), 255)
        md = ImageDraw.Draw(mask)
        for e in range(8):
            a = int(255 * e / 8)
            md.rectangle([e, e, cs - 1 - e, cs - 1 - e], outline=a)
        bg.paste(crop, (px, py), mask)
        ci = existing_composites + idx_offset
        fname = f"mixed_composite_{ci:03d}_coco{bg_id}.jpg"
        outpath = TEST_SET_DIR / "mixed" / fname
        bg.save(str(outpath), format="JPEG", quality=88, subsampling=0)
        all_new.append({
            "filename": fname,
            "path": f"mixed/{fname}",
            "source": f"composite-coco-{bg_id}-on-{fg_id}",
            "expected_verdict": "inconclusive",
            "category": "mixed",
            "notes": f"Composite: {fg_id} pasted onto {bg_id}",
        })
        print(f"  OK mixed/{fname}")
    time.sleep(0.5)

# Update manifest
manifest = json.loads(MANIFEST_PATH.read_text())
manifest["images"].extend(all_new)
manifest["total_images"] = len(manifest["images"])
# Recount categories
cats = {}
for e in manifest["images"]:
    c = e["category"]
    cats[c] = cats.get(c, 0) + 1
manifest["categories"] = cats
manifest["updated_at"] = datetime.now().isoformat()
MANIFEST_PATH.write_text(json.dumps(manifest, indent=2))

print(f"\nAdded {len(all_new)} images. New total: {manifest['total_images']}")
for cat, count in sorted(cats.items()):
    print(f"  {cat}: {count}")

# Count actual files
import os
actual = sum(1 for r, d, fs in os.walk(TEST_SET_DIR) for f in fs if Path(f).suffix.lower() in {".jpg", ".png"})
print(f"Actual files on disk: {actual}")
