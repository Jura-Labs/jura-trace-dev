#!/usr/bin/env python3
"""
Jura Trace — Calibration Corpus Builder

Downloads known-authentic press photographs from The Guardian's
"Ten best photographs of the day" series for detector calibration.

These images are used solely for internal threshold calibration and
are not redistributed. All images remain copyright of their respective
photographers / The Guardian.

Usage:
    python scripts/build_corpus.py --days 10 --output corpus/authentic
    python scripts/build_corpus.py --days 30 --output corpus/authentic --max-images 200
"""

import argparse
import hashlib
import json
import os
import re
import sys
import time
from datetime import datetime, timedelta
from pathlib import Path
from urllib.request import Request, urlopen
from urllib.error import HTTPError, URLError


GUARDIAN_SERIES_URL = "https://www.theguardian.com/news/series/ten-best-photographs-of-the-day"
USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
SIGNED_IMAGE_PATTERN = re.compile(r'(https://i\.guim\.co\.uk/img/media/([a-f0-9]+)/[^\s"]+)', re.IGNORECASE)


def fetch_page(url: str) -> str:
    """Fetch a page with browser-like headers."""
    req = Request(url, headers={"User-Agent": USER_AGENT, "Accept": "text/html"})
    try:
        with urlopen(req, timeout=15) as resp:
            return resp.read().decode("utf-8", errors="replace")
    except (HTTPError, URLError) as e:
        print(f"  Warning: could not fetch {url}: {e}")
        return ""


def extract_gallery_urls(days: int) -> list[str]:
    """Get gallery page URLs for the last N days."""
    urls = []
    # Fetch the series index to find actual gallery links
    for page_num in range(1, (days // 20) + 2):
        index_url = GUARDIAN_SERIES_URL if page_num == 1 else f"{GUARDIAN_SERIES_URL}?page={page_num}"
        html = fetch_page(index_url)
        # Find date-based gallery links
        gallery_links = re.findall(
            r'/news/series/ten-best-photographs-of-the-day/\d{4}/\w+/\d{2}/all',
            html
        )
        for link in gallery_links:
            full_url = f"https://www.theguardian.com{link}"
            if full_url not in urls:
                urls.append(full_url)
        if len(urls) >= days:
            break
        time.sleep(0.5)  # Be polite
    return urls[:days]


def extract_image_urls(gallery_html: str) -> list[tuple[str, str]]:
    """Extract signed image URLs from a gallery page.
    Returns list of (signed_url, media_hash) tuples."""
    import html as html_mod
    decoded = html_mod.unescape(gallery_html)
    matches = SIGNED_IMAGE_PATTERN.findall(decoded)
    seen = set()
    results = []
    for full_url, media_hash in matches:
        if media_hash not in seen:
            seen.add(media_hash)
            # Clean trailing srcset descriptors like " 920w"
            clean_url = re.sub(r'\s+\d+w$', '', full_url).strip()
            if clean_url:
                results.append((clean_url, media_hash[:12]))
    return results


def download_image(url: str, output_dir: Path, prefix: str) -> str | None:
    """Download an image using its signed URL, return the local filename or None."""
    # Detect extension from URL path (before query string)
    path_part = url.split("?")[0]
    ext = "jpg" if path_part.lower().endswith((".jpg", ".jpeg")) else "png"
    # Use media hash for deduplication
    url_hash = hashlib.md5(url.encode()).hexdigest()[:8]
    filename = f"{prefix}_{url_hash}.{ext}"
    filepath = output_dir / filename

    if filepath.exists():
        return filename  # Already downloaded

    # Use the signed URL as-is (Guardian CDN validates the signature)
    req = Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urlopen(req, timeout=30) as resp:
            data = resp.read()
            if len(data) < 5000:
                print(f"  Warning: tiny response for {filename}, skipping")
                return None
            filepath.write_bytes(data)
            return filename
    except (HTTPError, URLError) as e:
        print(f"  Warning: could not download {filename}: {e}")
        return None


def main():
    parser = argparse.ArgumentParser(description="Build calibration corpus from Guardian photographs")
    parser.add_argument("--days", type=int, default=10, help="Number of gallery days to fetch (default: 10)")
    parser.add_argument("--output", type=str, default="corpus/authentic", help="Output directory (default: corpus/authentic)")
    parser.add_argument("--max-images", type=int, default=100, help="Maximum images to download (default: 100)")
    parser.add_argument("--manifest", type=str, default=None, help="Manifest JSON path (default: <output>/manifest.json)")
    args = parser.parse_args()

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = Path(args.manifest) if args.manifest else output_dir / "manifest.json"

    print(f"Jura Trace — Calibration Corpus Builder")
    print(f"  Output: {output_dir}")
    print(f"  Days: {args.days}")
    print(f"  Max images: {args.max_images}")
    print()

    # Step 1: Find gallery pages
    print("Finding gallery pages...")
    gallery_urls = extract_gallery_urls(args.days)
    print(f"  Found {len(gallery_urls)} gallery pages")
    print()

    # Step 2: Extract image URLs from each gallery
    all_images = []
    for i, gallery_url in enumerate(gallery_urls):
        date_match = re.search(r'/(\d{4})/(\w+)/(\d{2})/', gallery_url)
        date_str = f"{date_match.group(1)}-{date_match.group(2)}-{date_match.group(3)}" if date_match else f"gallery-{i}"

        print(f"  [{i+1}/{len(gallery_urls)}] {date_str}...", end=" ", flush=True)
        html = fetch_page(gallery_url)
        images = extract_image_urls(html)
        print(f"{len(images)} images")

        for url, media_hash in images:
            all_images.append({
                "url": url,
                "media_hash": media_hash,
                "gallery_date": date_str,
                "source": "guardian-ten-best",
                "label": "authentic",
            })

        if len(all_images) >= args.max_images:
            break
        time.sleep(0.5)

    all_images = all_images[:args.max_images]
    print(f"\n  Total: {len(all_images)} images to download")
    print()

    # Step 3: Download images
    print("Downloading images...")
    manifest_entries = []
    downloaded = 0

    for i, img in enumerate(all_images):
        prefix = img["gallery_date"].replace("-", "")
        filename = download_image(img["url"], output_dir, prefix)

        if filename:
            downloaded += 1
            manifest_entries.append({
                "filename": filename,
                "source_url": img["url"],
                "gallery_date": img["gallery_date"],
                "source": img["source"],
                "label": img["label"],
                "downloaded_at": datetime.now().isoformat(),
            })
            if downloaded % 10 == 0:
                print(f"  {downloaded}/{len(all_images)} downloaded...")
        time.sleep(0.3)

    # Step 4: Write manifest
    manifest = {
        "corpus_name": "guardian-ten-best-authentic",
        "description": "Known-authentic press photographs from The Guardian's Ten Best Photographs of the Day series. For internal calibration only.",
        "created_at": datetime.now().isoformat(),
        "total_images": len(manifest_entries),
        "label": "authentic",
        "copyright_notice": "All images copyright their respective photographers / The Guardian. Used solely for internal detector calibration, not redistributed.",
        "images": manifest_entries,
    }

    manifest_path.write_text(json.dumps(manifest, indent=2))

    print(f"\nDone.")
    print(f"  Downloaded: {downloaded} images")
    print(f"  Location: {output_dir}/")
    print(f"  Manifest: {manifest_path}")
    print(f"\nNext: run the calibration pipeline:")
    print(f"  python scripts/calibrate.py --corpus {output_dir}")


if __name__ == "__main__":
    main()
