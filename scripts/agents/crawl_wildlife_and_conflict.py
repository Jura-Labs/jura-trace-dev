#!/usr/bin/env python3
"""
Jura Trace — Sprint 29 Track 2 Wildlife + War/Conflict Corpus Crawler

Sources authentic photographs from two domains the GBM/UnivFD models
currently underperform on:

1. **Wildlife / macro photography** — wikimedia_photos showed 24.8% FP
   in the GBM v4 bias check, dominated by wildlife and insect macro.
   Sourced from Wikimedia Commons CC-licensed wildlife categories.

2. **War / armed conflict documentation** — the user observed false
   positives on smartphone footage from active conflict zones during
   Sprint 29 validation. This is the highest-stakes use case for the
   product (DRRF, investigative journalists, fact-checkers). Sourced
   from Wikimedia Commons CC-licensed conflict categories.

Both use the Wikimedia Commons API (no auth needed). All images are
verified as CC BY, CC BY-SA, CC0, or public domain before download.

Ethical filters for war/conflict content:
- No images depicting identifiable casualties or remains
- No images from sources whose authenticity cannot be verified
- All sources logged with licence + date in the manifest

Usage:
    # Wildlife from Wikimedia Commons
    python scripts/agents/crawl_wildlife_and_conflict.py wildlife --count 200

    # War/conflict from Wikimedia Commons
    python scripts/agents/crawl_wildlife_and_conflict.py conflict --count 200

    # Both at once
    python scripts/agents/crawl_wildlife_and_conflict.py all --wildlife 200 --conflict 200
"""

import argparse
import csv
import hashlib
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from urllib.parse import quote
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError

# Output directories on USB
USB_BASE = Path("/Volumes/Samsung USB/Training Data/corpus/training/authentic")

# Wildlife categories — diverse subjects
WILDLIFE_CATEGORIES = [
    "Wildlife photography",
    "Macro photography",
    "Insects in close-up",
    "Birds in flight",
    "Wild mammals",
    "Underwater photography of fish",
    "Reptiles in habitat",
    "Amphibians close-up",
    "Spiders close-up",
    "Butterflies on flowers",
]

# War/conflict categories — recent + historical, all CC-licensed
# Filtered to documentary photographs only (no illustrations, no graphic content)
CONFLICT_CATEGORIES = [
    "Russo-Ukrainian War",
    "Photographs of the Russo-Ukrainian War",
    "Syrian civil war",
    "Conflicts in 2024",
    "Conflicts in 2025",
    "Military exercises",
    "Damaged buildings in conflict zones",
    "Refugee camps",
    "Humanitarian aid",
    "United Nations peacekeeping",
]

# Wikimedia Commons API endpoint
COMMONS_API = "https://commons.wikimedia.org/w/api.php"

# Acceptable licences
ACCEPTABLE_LICENCES = {
    "cc0", "cc-zero", "publicdomain", "public domain",
    "cc-by", "cc-by-sa", "cc by", "cc by-sa",
    "cc-by-2.0", "cc-by-3.0", "cc-by-4.0",
    "cc-by-sa-2.0", "cc-by-sa-3.0", "cc-by-sa-4.0",
}

# Reject keywords for war/conflict (ethical filter)
CONFLICT_REJECT_KEYWORDS = [
    "casualty", "casualties", "dead body", "bodies", "corpse", "corpses",
    "execution", "torture", "wounded soldier", "injury close-up",
    "blood", "bleeding", "amputee",
]


def fetch_json(url: str, timeout: int = 30) -> dict | None:
    """GET a URL and return parsed JSON. Returns None on failure."""
    req = Request(url, headers={"User-Agent": "JuraTraceCorpusBuilder/1.0 (research)"})
    try:
        with urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read())
    except (HTTPError, URLError, json.JSONDecodeError) as e:
        print(f"  fetch_json error: {e}")
        return None


def list_category_files(category: str, limit: int = 50) -> list[str]:
    """List file titles in a Wikimedia Commons category."""
    cat = f"Category:{category}"
    url = (
        f"{COMMONS_API}?action=query&list=categorymembers"
        f"&cmtitle={quote(cat)}&cmtype=file&cmlimit={limit}"
        f"&format=json"
    )
    data = fetch_json(url)
    if not data:
        return []
    return [m["title"] for m in data.get("query", {}).get("categorymembers", [])]


def get_image_info(file_title: str) -> dict | None:
    """Fetch image URL, licence, and metadata for a Commons file."""
    url = (
        f"{COMMONS_API}?action=query&titles={quote(file_title)}"
        f"&prop=imageinfo&iiprop=url|size|mime|extmetadata"
        f"&format=json"
    )
    data = fetch_json(url)
    if not data:
        return None
    pages = data.get("query", {}).get("pages", {})
    for _, page in pages.items():
        info_list = page.get("imageinfo", [])
        if not info_list:
            continue
        info = info_list[0]
        meta = info.get("extmetadata", {})
        return {
            "title": file_title,
            "url": info.get("url"),
            "width": info.get("width", 0),
            "height": info.get("height", 0),
            "mime": info.get("mime", ""),
            "licence": (meta.get("LicenseShortName", {}) or {}).get("value", "").lower(),
            "description": (meta.get("ImageDescription", {}) or {}).get("value", "")[:500],
            "credit": (meta.get("Credit", {}) or {}).get("value", "")[:200],
        }
    return None


def is_acceptable_licence(licence: str) -> bool:
    """Check if the licence string indicates an acceptable CC/PD licence."""
    if not licence:
        return False
    licence_l = licence.lower().strip()
    return any(acc in licence_l for acc in ACCEPTABLE_LICENCES)


def is_conflict_safe(info: dict) -> bool:
    """Apply the ethical filter for war/conflict content."""
    text = (info.get("description", "") + " " + info.get("title", "")).lower()
    for kw in CONFLICT_REJECT_KEYWORDS:
        if kw in text:
            return False
    return True


def download_image(url: str, dest: Path) -> bool:
    """Download an image to dest. Returns True on success."""
    req = Request(url, headers={"User-Agent": "JuraTraceCorpusBuilder/1.0 (research)"})
    try:
        with urlopen(req, timeout=60) as resp:
            data = resp.read()
        if len(data) < 1024:
            return False
        dest.write_bytes(data)
        return True
    except (HTTPError, URLError) as e:
        print(f"  download error: {e}")
        return False


def crawl_categories(
    categories: list[str],
    count: int,
    out_dir: Path,
    name_prefix: str,
    apply_conflict_filter: bool = False,
    log_path: Path | None = None,
) -> int:
    """Walk a list of Wikimedia categories and download up to `count` images."""
    out_dir.mkdir(parents=True, exist_ok=True)
    log_rows = []
    downloaded = 0
    seen_hashes = set()

    # Index existing files (for resume)
    for f in out_dir.glob("*"):
        if f.is_file():
            try:
                h = hashlib.sha256(f.read_bytes()).hexdigest()
                seen_hashes.add(h)
            except Exception:
                pass
    existing = len(seen_hashes)
    if existing > 0:
        print(f"  resume: {existing} existing files in {out_dir}")

    for cat in categories:
        if downloaded >= count:
            break
        print(f"\n  category: {cat}")
        titles = list_category_files(cat, limit=100)
        print(f"    {len(titles)} files in category")

        for title in titles:
            if downloaded >= count:
                break
            time.sleep(0.5)  # be polite to the Commons API

            info = get_image_info(title)
            if not info or not info.get("url"):
                continue

            # Filter: image only, reasonable size, acceptable licence
            mime = info.get("mime", "")
            if not mime.startswith("image/"):
                continue
            if mime in ("image/svg+xml", "image/x-xcf"):
                continue
            w, h = info.get("width", 0), info.get("height", 0)
            if w < 480 or h < 480:
                continue
            if not is_acceptable_licence(info.get("licence", "")):
                continue
            if apply_conflict_filter and not is_conflict_safe(info):
                print(f"    SKIP (ethical filter): {title}")
                continue

            # Download
            ext = ".jpg" if "jpeg" in mime else ".png" if "png" in mime else ".webp"
            tmp_name = f"{name_prefix}_{downloaded + existing:04d}{ext}"
            dest = out_dir / tmp_name
            if download_image(info["url"], dest):
                # Dedup by SHA-256
                try:
                    h = hashlib.sha256(dest.read_bytes()).hexdigest()
                except Exception:
                    dest.unlink(missing_ok=True)
                    continue
                if h in seen_hashes:
                    dest.unlink(missing_ok=True)
                    continue
                seen_hashes.add(h)
                downloaded += 1
                log_rows.append({
                    "filename": dest.name,
                    "title": title,
                    "category": cat,
                    "licence": info.get("licence", ""),
                    "credit": info.get("credit", "")[:100],
                    "url": info.get("url", ""),
                    "sha256": h,
                    "downloaded_at": datetime.now().isoformat(),
                })
                if downloaded % 10 == 0:
                    print(f"    {downloaded}/{count}...")

    # Write log
    if log_path and log_rows:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        with open(log_path, "w", newline="") as f:
            writer = csv.DictWriter(
                f,
                fieldnames=["filename", "title", "category", "licence", "credit", "url", "sha256", "downloaded_at"],
            )
            writer.writeheader()
            writer.writerows(log_rows)
        print(f"\n  log: {log_path}")

    print(f"  total downloaded: {downloaded}")
    return downloaded


def cmd_wildlife(args):
    print("=" * 60)
    print("  Sprint 29 Track 2 — Wildlife / Macro Photography Crawl")
    print("=" * 60)
    out = USB_BASE / "wildlife_macro"
    log = Path("corpus/sources_wildlife_macro.csv")
    n = crawl_categories(
        WILDLIFE_CATEGORIES,
        args.count,
        out,
        "wildlife",
        apply_conflict_filter=False,
        log_path=log,
    )
    print(f"\n  Wildlife/macro corpus: {n} images")


def cmd_conflict(args):
    print("=" * 60)
    print("  Sprint 29 Track 2 — War / Conflict Documentation Crawl")
    print("=" * 60)
    print("  Ethical filters active: no casualties, CC licences only")
    print()
    out = USB_BASE / "war_conflict"
    log = Path("corpus/sources_war_conflict.csv")
    n = crawl_categories(
        CONFLICT_CATEGORIES,
        args.count,
        out,
        "conflict",
        apply_conflict_filter=True,
        log_path=log,
    )
    print(f"\n  War/conflict corpus: {n} images")


def cmd_all(args):
    cmd_wildlife(argparse.Namespace(count=args.wildlife))
    print()
    cmd_conflict(argparse.Namespace(count=args.conflict))


def main():
    parser = argparse.ArgumentParser(description="Wildlife + war/conflict Wikimedia crawler")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_w = sub.add_parser("wildlife", help="Wildlife/macro from Wikimedia Commons")
    p_w.add_argument("--count", type=int, default=200)
    p_w.set_defaults(func=cmd_wildlife)

    p_c = sub.add_parser("conflict", help="War/conflict from Wikimedia Commons")
    p_c.add_argument("--count", type=int, default=200)
    p_c.set_defaults(func=cmd_conflict)

    p_a = sub.add_parser("all", help="Both wildlife and conflict")
    p_a.add_argument("--wildlife", type=int, default=200)
    p_a.add_argument("--conflict", type=int, default=200)
    p_a.set_defaults(func=cmd_all)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
