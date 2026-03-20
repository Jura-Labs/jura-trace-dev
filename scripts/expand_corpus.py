#!/usr/bin/env python3
"""
Jura Trace — Corpus Expansion Script

Downloads AI-generated images from HuggingFace datasets and authentic
images from public sources to build a larger training corpus.

Usage:
    python scripts/expand_corpus.py
"""

import hashlib
import json
import os
import sys
from datetime import datetime
from io import BytesIO
from pathlib import Path


def download_ai_from_huggingface(output_dir: Path, max_images: int = 200):
    """Download AI-generated images from HuggingFace diffusiondb."""
    from datasets import load_dataset

    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "manifest.json"

    print(f"  Downloading from poloclub/diffusiondb (max {max_images})...")

    # Use the small random 1K subset — fast to download
    try:
        ds = load_dataset("poloclub/diffusiondb", "random_1k", split="train")
    except Exception as e:
        print(f"  Warning: could not load diffusiondb: {e}")
        print("  Trying alternative: Falah/Stable_Diffusion_Datasets...")
        try:
            ds = load_dataset("Falah/Stable_Diffusion_Datasets", split="train")
        except Exception as e2:
            print(f"  Warning: could not load alternative: {e2}")
            return 0

    downloaded = 0
    entries = []

    for i, item in enumerate(ds):
        if downloaded >= max_images:
            break

        img = item.get("image")
        if img is None:
            continue

        # Convert to bytes
        buf = BytesIO()
        img.save(buf, format="PNG")
        img_bytes = buf.getvalue()

        if len(img_bytes) < 5000:
            continue

        # Save
        img_hash = hashlib.md5(img_bytes).hexdigest()[:10]
        filename = f"diffusiondb_{img_hash}.png"
        filepath = output_dir / filename

        if filepath.exists():
            continue

        filepath.write_bytes(img_bytes)
        downloaded += 1

        entries.append({
            "filename": filename,
            "source": "diffusiondb-random-1k",
            "label": "ai_generated",
            "generator": "stable_diffusion",
            "downloaded_at": datetime.now().isoformat(),
        })

        if downloaded % 20 == 0:
            print(f"    {downloaded}/{max_images}...")

    # Merge with existing manifest
    existing = []
    if manifest_path.exists():
        try:
            old = json.loads(manifest_path.read_text())
            existing = old.get("images", [])
        except Exception:
            pass

    manifest = {
        "corpus_name": "ai-generated-mixed",
        "description": "AI-generated images from multiple sources for classifier training.",
        "created_at": datetime.now().isoformat(),
        "total_images": len(existing) + len(entries),
        "label": "ai_generated",
        "images": existing + entries,
    }
    manifest_path.write_text(json.dumps(manifest, indent=2))

    print(f"    Downloaded {downloaded} from HuggingFace")
    return downloaded


def copy_local_ai_images(source_dir: Path, output_dir: Path):
    """Copy local AI images to the corpus."""
    output_dir.mkdir(parents=True, exist_ok=True)

    if not source_dir.exists():
        print(f"  Local AI folder not found: {source_dir}")
        return 0

    copied = 0
    extensions = {".jpg", ".jpeg", ".png", ".webp"}

    for f in sorted(source_dir.iterdir()):
        if f.suffix.lower() not in extensions:
            continue

        dest = output_dir / f.name
        if not dest.exists():
            dest.write_bytes(f.read_bytes())
            copied += 1

    print(f"  Copied {copied} local AI images")
    return copied


def download_authentic_from_coco(output_dir: Path, max_images: int = 150):
    """Download authentic photos from COCO validation set via direct URLs."""
    import urllib.request

    output_dir.mkdir(parents=True, exist_ok=True)

    # COCO val2017 image list — first N images from the validation set
    # These are known real photographs with full provenance
    coco_base = "http://images.cocodataset.org/val2017/"

    # Known COCO val2017 image IDs (first 200, zero-padded to 12 digits)
    # These are real photographs from Flickr with CC licences
    print(f"  Downloading from COCO val2017 (max {max_images})...")

    # Get the image list
    try:
        list_url = "http://images.cocodataset.org/annotations/image_info_test2017.json"
        # Actually, let's use a simpler approach — sequential IDs from val2017
        # Known valid val2017 filenames
        val_ids = [
            "000000000139", "000000000285", "000000000632", "000000000724",
            "000000000776", "000000000785", "000000000802", "000000000872",
            "000000000885", "000000001000", "000000001268", "000000001296",
            "000000001353", "000000001425", "000000001490", "000000001503",
            "000000001532", "000000001584", "000000001675", "000000001761",
            "000000001818", "000000001993", "000000002006", "000000002149",
            "000000002153", "000000002157", "000000002261", "000000002299",
            "000000002431", "000000002473", "000000002532", "000000002587",
            "000000002592", "000000002685", "000000002923", "000000002933",
            "000000003156", "000000003255", "000000003501", "000000003553",
            "000000003845", "000000003934", "000000004134", "000000004395",
            "000000004495", "000000004572", "000000004601", "000000004795",
            "000000004830", "000000005001", "000000005037", "000000005060",
            "000000005193", "000000005248", "000000005503", "000000005529",
            "000000005577", "000000005600", "000000005602", "000000005638",
            "000000005992", "000000006040", "000000006471", "000000006614",
            "000000006723", "000000006763", "000000006771", "000000006818",
            "000000007088", "000000007278", "000000007281", "000000007386",
            "000000007574", "000000007784", "000000007816", "000000007918",
            "000000008021", "000000008211", "000000008277", "000000008344",
            "000000008532", "000000008690", "000000008762", "000000008844",
            "000000009014", "000000009378", "000000009400", "000000009448",
            "000000009483", "000000009590", "000000009769", "000000009772",
            "000000009891", "000000009914", "000000010092", "000000010125",
            "000000010175", "000000010205", "000000010211", "000000010244",
            "000000010363", "000000010386", "000000010434", "000000010526",
            "000000010583", "000000010707", "000000010764", "000000010791",
            "000000010862", "000000010977", "000000011051", "000000011122",
            "000000011149", "000000011197", "000000011511", "000000011615",
            "000000011699", "000000011760", "000000011813", "000000011947",
            "000000012062", "000000012120", "000000012166", "000000012252",
            "000000012280", "000000012484", "000000012576", "000000012639",
            "000000012667", "000000012670", "000000012748", "000000013004",
            "000000013177", "000000013201", "000000013291", "000000013348",
            "000000013546", "000000013597", "000000013729", "000000013774",
            "000000013923", "000000014007", "000000014038", "000000014226",
            "000000014380", "000000014439", "000000014473", "000000014831",
            "000000014888", "000000015079", "000000015254", "000000015278",
            "000000015335", "000000015338", "000000015440", "000000015497",
        ]
    except Exception:
        val_ids = []

    downloaded = 0
    for img_id in val_ids[:max_images]:
        filename = f"coco_{img_id}.jpg"
        filepath = output_dir / filename

        if filepath.exists():
            downloaded += 1
            continue

        url = f"{coco_base}{img_id}.jpg"
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = resp.read()
                if len(data) > 5000:
                    filepath.write_bytes(data)
                    downloaded += 1
        except Exception:
            pass

        if downloaded % 20 == 0 and downloaded > 0:
            print(f"    {downloaded}/{max_images}...")

        import time
        time.sleep(0.1)

    print(f"  Downloaded {downloaded} COCO images")
    return downloaded


def main():
    base = Path(__file__).parent.parent
    ai_dir = base / "corpus" / "ai_generated"
    authentic_dir = base / "corpus" / "authentic"
    local_ai = Path("/Users/paulgriffiths/Desktop/Fake Images AI")

    print("Jura Trace — Corpus Expansion")
    print()

    # Step 1: Copy local AI images
    print("1. Local AI images:")
    copy_local_ai_images(local_ai, ai_dir)

    # Step 2: Download from HuggingFace
    print("\n2. HuggingFace AI images:")
    download_ai_from_huggingface(ai_dir, max_images=180)

    # Step 3: Download COCO authentic images
    print("\n3. COCO authentic images:")
    download_authentic_from_coco(authentic_dir, max_images=150)

    # Summary
    ai_count = len([f for f in ai_dir.iterdir() if f.suffix.lower() in {".jpg", ".jpeg", ".png"}]) if ai_dir.exists() else 0
    auth_count = len([f for f in authentic_dir.iterdir() if f.suffix.lower() in {".jpg", ".jpeg", ".png"}]) if authentic_dir.exists() else 0

    print(f"\nCorpus summary:")
    print(f"  Authentic: {auth_count} images ({authentic_dir})")
    print(f"  AI-generated: {ai_count} images ({ai_dir})")
    print(f"  Total: {auth_count + ai_count}")
    print(f"\nNext: retrain the classifier:")
    print(f"  python3 scripts/train_classifier.py")


if __name__ == "__main__":
    main()
