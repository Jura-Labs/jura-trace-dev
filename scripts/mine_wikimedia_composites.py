#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""
Jura Trace — Wikimedia Commons composite miner (Track D of splice benchmark)

Mines Wikimedia Commons for real-world photomontages and composite
photographs with commercial-use-compatible licences (CC-BY, CC-BY-SA,
CC0, Public Domain) to supplement the synthetic splice corpus built by
scripts/build_splice_corpus_v2.py.

Why: CASIA v1/v2 were rejected 2026-04-11 as non-commercial-licensed.
Wikimedia Commons is one of the few sources of real-world composites
with commercial-compatible licensing. See
docs/decisions/splice-benchmark-longterm.md (Option D).

Output: models/splice_wikimedia_v1/
  images/       — downloaded JPEG composites
  manifest.json — per-image licence, attribution, dims, sha256

All downloaded images are labelled `spliced` (they are real composites
per Commons categorisation). Per-pixel masks are NOT available; bbox is
null. Downstream sweep can still compute image-level trust-score AUC.

Usage:
    python scripts/mine_wikimedia_composites.py --dry-run --max 10
    python scripts/mine_wikimedia_composites.py --max 20
    python scripts/mine_wikimedia_composites.py --max 100

Dependencies: stdlib only (urllib, json, hashlib). No external packages.
Respects Wikimedia API etiquette: 1 req/sec, honest User-Agent, no auth.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

API_ENDPOINT = "https://commons.wikimedia.org/w/api.php"
USER_AGENT = (
    "JuraTraceCalibration/1.0 "
    "(https://juralabs.org; research@juralabs.org)"
)
REQUEST_DELAY_SEC = 1.0  # polite rate limit
REQUEST_TIMEOUT = 30

# Categories to harvest. Primary is Photomontages; others are exploratory.
CATEGORIES = [
    "Category:Photomontages",
    "Category:Composite_photographs",
    "Category:Digitally_manipulated_photographs",
    "Category:Fake_images",
    # Category:Image_editing is mostly meta; skipped as instructed.
]

# JTV-127 — Global Majority handset corpus for UnivFD v11 retrain.
# Wikimedia Commons indexes user-uploaded camera photos by handset model;
# per-file licence is in `extmetadata` (CC-BY / CC-BY-SA / CC0 are accepted,
# any NC/ND variant is rejected by the existing licence filter).
# See open-corpus-sweep-may-2026.md (rank-1 recommendation) for category scope
# and yield estimates (300-600 licence-clean images expected).
#
# Category-name conventions on Commons vary by vendor:
# - "Photos taken with X" (Xiaomi)
# - "Taken with X" (Infinix)
# - direct vendor name with model subcategories (Realme, Tecno)
# Verify each category exists at https://commons.wikimedia.org/wiki/<Category>
# before a non-dry-run sweep — the API silently returns 0 results for
# misspelled category names. Override via --categories for ad-hoc lists.
HANDSET_CATEGORIES = [
    "Category:Photos_taken_with_Xiaomi_mobile_phones",
    "Category:Taken_with_Infinix_mobile_phones",
    "Category:Photographs_taken_with_Tecno_mobile_phones",
    "Category:Photographs_taken_with_Realme_mobile_phones",
    "Category:Photographs_taken_with_Samsung_Galaxy_A_series",
    "Category:Photographs_taken_with_Vivo_mobile_phones",
    "Category:Photographs_taken_with_Oppo_mobile_phones",
    "Category:Photographs_taken_with_Huawei_P_series",
    # Realme sub-models: Commons indexes individual phones too. Direct categories
    # by model give a cleaner per-device sweep when needed; these can be added
    # via --categories on a per-run basis.
    # Examples: "Category:Realme_7", "Category:Realme_GT_Master_Edition"
]

# Image filters (default values — overridden per --preset; see PRESETS below)
MIN_SHORT_EDGE = 512
MAX_FILE_BYTES = 15 * 1024 * 1024  # 15 MB
ACCEPTED_MIME = {"image/jpeg"}

# Licence filtering. Match is done case-insensitively on LicenseShortName
# (and falls back to License field). Acceptance is by regex because
# Commons uses a wide variety of short names.
ACCEPT_LICENCE_PATTERNS = [
    # cc-by / CC BY 4.0 / cc by 2.0 etc. (space or hyphen separators)
    re.compile(r"^cc[- ]by([- ]\d(\.\d)?)?$", re.I),
    # cc-by-sa-* / CC BY-SA 4.0 / CC BY SA 3.0
    re.compile(r"^cc[- ]by[- ]sa([- ]\d(\.\d)?)?$", re.I),
    # cc-zero / cczero
    re.compile(r"^cc[- ]?zero$", re.I),
    # cc0, cc0-1.0
    re.compile(r"^cc0(-\d(\.\d)?)?$", re.I),
    re.compile(r"^public domain$", re.I),
    # pd, pd-us, pd-old
    re.compile(r"^pd([- ].*)?$", re.I),
    # flickr commons / LOC
    re.compile(r"^no restrictions$", re.I),
]

REJECT_LICENCE_PATTERNS = [
    # cc-by-nc, CC BY-NC-SA 4.0, etc. — word-boundary NC.
    re.compile(r"\bnc(\b|[- ])", re.I),
    # cc-by-nd, CC BY-ND 4.0 — word-boundary ND.
    re.compile(r"\bnd(\b|[- ])", re.I),
    re.compile(r"fair[- ]use", re.I),
    re.compile(r"copyright", re.I),
    re.compile(r"all rights reserved", re.I),
]

OUTPUT_DIR = Path("models/splice_wikimedia_v1")
IMAGES_DIR = OUTPUT_DIR / "images"
MANIFEST_PATH = OUTPUT_DIR / "manifest.json"

# Per-preset settings. `apply_preset(name)` mutates the module-level constants
# above based on the chosen preset so the rest of the script doesn't need to
# carry config through every function signature. Default values above match
# the photomontage preset.
PRESETS: dict[str, dict] = {
    "photomontage": {
        "categories": CATEGORIES,
        "output_dir": Path("models/splice_wikimedia_v1"),
        "manifest_label": "spliced",
        "accepted_mime": {"image/jpeg"},
        "min_short_edge": 512,
        "description": "Photomontage / composite training data (existing default)",
    },
    "handset": {
        "categories": HANDSET_CATEGORIES,
        "output_dir": Path("models/handset_wikimedia_v1"),
        "manifest_label": "authentic",
        # Handset photos sometimes upload as PNG (Pixel/Samsung native PNG mode,
        # or HEIC→PNG transcoding by uploaders). JPEG dominates but allow PNG.
        "accepted_mime": {"image/jpeg", "image/png"},
        # 480px lower bound captures more legitimate handset uploads
        # (Wikimedia thumbnail policies sometimes downsize uploads below 512).
        "min_short_edge": 480,
        "description": "Global Majority handset corpus for JTV-127 / UnivFD v11 retrain",
    },
}


def apply_preset(preset_name: str) -> dict:
    """Mutate module-level constants to match the named preset.

    Returns the resolved preset config dict so the caller can also read
    derived values (categories list, manifest label) without re-resolving.
    """
    if preset_name not in PRESETS:
        raise ValueError(
            f"Unknown preset {preset_name!r}; valid: {sorted(PRESETS)}"
        )
    cfg = PRESETS[preset_name]
    g = globals()
    g["OUTPUT_DIR"] = cfg["output_dir"]
    g["IMAGES_DIR"] = cfg["output_dir"] / "images"
    g["MANIFEST_PATH"] = cfg["output_dir"] / "manifest.json"
    g["MIN_SHORT_EDGE"] = cfg["min_short_edge"]
    g["ACCEPTED_MIME"] = cfg["accepted_mime"]
    return cfg


# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------


class PoliteClient:
    """Minimal rate-limited urllib client with honest UA."""

    def __init__(self, delay: float = REQUEST_DELAY_SEC):
        self.delay = delay
        self._last_request_at = 0.0

    def _wait(self):
        now = time.monotonic()
        gap = now - self._last_request_at
        if gap < self.delay:
            time.sleep(self.delay - gap)
        self._last_request_at = time.monotonic()

    def get_json(self, url: str) -> dict | None:
        self._wait()
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
                data = resp.read()
                return json.loads(data.decode("utf-8"))
        except urllib.error.HTTPError as e:
            print(f"    HTTP {e.code} on {url[:80]}", file=sys.stderr)
            return None
        except urllib.error.URLError as e:
            print(f"    URL error: {e}", file=sys.stderr)
            return None
        except Exception as e:  # noqa: BLE001
            print(f"    Unexpected error: {e}", file=sys.stderr)
            return None

    def download(self, url: str, dest: Path) -> bytes | None:
        self._wait()
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
                data = resp.read()
                dest.write_bytes(data)
                return data
        except Exception as e:  # noqa: BLE001
            print(f"    download error: {e}", file=sys.stderr)
            return None


# ---------------------------------------------------------------------------
# API queries
# ---------------------------------------------------------------------------


def list_category_members(
    client: PoliteClient,
    category: str,
    limit: int,
) -> list[str]:
    """Return titles of files in a Commons category (paginated)."""
    titles: list[str] = []
    cmcontinue: str | None = None
    per_page = min(500, max(50, limit * 3))

    while len(titles) < limit * 3:  # fetch headroom for filtering
        params = {
            "action": "query",
            "list": "categorymembers",
            "cmtitle": category,
            "cmtype": "file",
            "cmlimit": str(per_page),
            "format": "json",
        }
        if cmcontinue:
            params["cmcontinue"] = cmcontinue
        url = f"{API_ENDPOINT}?{urllib.parse.urlencode(params)}"
        data = client.get_json(url)
        if not data:
            break
        members = data.get("query", {}).get("categorymembers", [])
        for m in members:
            if m.get("ns") == 6:  # File namespace
                titles.append(m["title"])
        cont = data.get("continue", {}).get("cmcontinue")
        if not cont:
            break
        cmcontinue = cont
        if len(titles) >= limit * 3:
            break
    return titles


def fetch_imageinfo(
    client: PoliteClient,
    titles: list[str],
) -> dict[str, dict]:
    """Batch imageinfo fetch. Returns {title: imageinfo_dict}."""
    results: dict[str, dict] = {}
    # Commons API accepts up to 50 titles per request for imageinfo.
    BATCH = 50
    for i in range(0, len(titles), BATCH):
        batch = titles[i : i + BATCH]
        params = {
            "action": "query",
            "prop": "imageinfo",
            "iiprop": "extmetadata|url|size|mime",
            "titles": "|".join(batch),
            "format": "json",
        }
        url = f"{API_ENDPOINT}?{urllib.parse.urlencode(params)}"
        data = client.get_json(url)
        if not data:
            continue
        pages = data.get("query", {}).get("pages", {})
        for _pid, page in pages.items():
            title = page.get("title")
            infos = page.get("imageinfo") or []
            if title and infos:
                results[title] = infos[0]
    return results


# ---------------------------------------------------------------------------
# Licence + usability filtering
# ---------------------------------------------------------------------------


def extract_licence(extmeta: dict) -> tuple[str | None, str | None]:
    """Return (short_name, raw_source_field) best-effort from extmetadata."""
    short = extmeta.get("LicenseShortName", {}).get("value")
    lic = extmeta.get("License", {}).get("value")
    # Prefer LicenseShortName; fall back to License.
    return (short or lic, short or lic)


def licence_verdict(licence_str: str | None) -> tuple[bool, str]:
    """Return (accept, reason). Reason is the normalised licence or rejection code."""
    if not licence_str:
        return (False, "no_licence")
    s = licence_str.strip()
    # Reject patterns have priority (nc/nd appear inside otherwise-CC names).
    for rx in REJECT_LICENCE_PATTERNS:
        if rx.search(s):
            return (False, f"reject:{s.lower()}")
    for rx in ACCEPT_LICENCE_PATTERNS:
        if rx.match(s):
            return (True, s.lower())
    return (False, f"unknown:{s.lower()}")


def usability_verdict(info: dict) -> tuple[bool, str]:
    mime = info.get("mime", "")
    if mime not in ACCEPTED_MIME:
        return (False, f"mime:{mime or 'unknown'}")
    w = info.get("width", 0) or 0
    h = info.get("height", 0) or 0
    if min(w, h) < MIN_SHORT_EDGE:
        return (False, "too_small")
    size = info.get("size", 0) or 0
    if size > MAX_FILE_BYTES:
        return (False, "too_large")
    return (True, "ok")


def sanitise_filename(title: str) -> str:
    """Commons title -> filesystem-safe stem."""
    # Strip "File:" prefix
    if title.startswith("File:"):
        title = title[5:]
    # Replace unsafe chars
    safe = re.sub(r"[^A-Za-z0-9._-]+", "_", title)
    return safe[:120]  # keep filesystem-safe length


def strip_html(s: str | None) -> str:
    if not s:
        return ""
    return re.sub(r"<[^>]+>", "", s).strip()


# ---------------------------------------------------------------------------
# Main mining loop
# ---------------------------------------------------------------------------


def load_existing_manifest() -> dict:
    if MANIFEST_PATH.exists():
        try:
            return json.loads(MANIFEST_PATH.read_text())
        except Exception:
            pass
    return {
        "generated_at": None,
        "total_candidates_scanned": 0,
        "total_accepted": 0,
        "total_rejected": 0,
        "rejection_reasons": {},
        "categories_queried": [],
        "licence_breakdown": {},
        "images": [],
    }


def save_manifest(manifest: dict) -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    MANIFEST_PATH.write_text(json.dumps(manifest, indent=2, sort_keys=False))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--max", type=int, default=100,
                        help="Maximum accepted files to download (default 100)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Query API and report; do not download")
    parser.add_argument("--preset", default="photomontage",
                        choices=sorted(PRESETS),
                        help="Category preset (default: photomontage)")
    parser.add_argument("--categories", nargs="+", default=None,
                        help="Override preset categories with explicit list")
    args = parser.parse_args()

    cfg = apply_preset(args.preset)
    categories_to_query = args.categories if args.categories else cfg["categories"]
    manifest_label = cfg["manifest_label"]

    print(f"Wikimedia Commons miner — preset={args.preset!r} ({cfg['description']})")
    print(f"  dry-run: {args.dry_run}")
    print(f"  max accepted: {args.max}")
    print(f"  categories: {categories_to_query}")
    print(f"  output_dir: {OUTPUT_DIR}")
    print(f"  manifest_label: {manifest_label}")
    print(f"  min_short_edge: {MIN_SHORT_EDGE}")
    print(f"  accepted_mime: {sorted(ACCEPTED_MIME)}")
    print()

    client = PoliteClient()
    manifest = load_existing_manifest()
    existing_titles = {img["commons_title"] for img in manifest["images"]}

    if not args.dry_run:
        IMAGES_DIR.mkdir(parents=True, exist_ok=True)

    total_scanned = 0
    total_accepted = 0
    rejection_reasons: dict[str, int] = dict(manifest.get("rejection_reasons", {}))
    licence_breakdown: dict[str, int] = dict(manifest.get("licence_breakdown", {}))
    new_images: list[dict] = []

    for category in categories_to_query:
        if total_accepted >= args.max:
            break
        print(f"[*] Listing {category}")
        titles = list_category_members(client, category, args.max)
        print(f"    Found {len(titles)} file titles")
        if not titles:
            continue

        # Skip titles we've already processed
        fresh_titles = [t for t in titles if t not in existing_titles]
        print(f"    {len(fresh_titles)} fresh (excluding already-manifest'd)")

        total_scanned += len(fresh_titles)

        print(f"    Fetching imageinfo in batches...")
        infos = fetch_imageinfo(client, fresh_titles)
        print(f"    Got imageinfo for {len(infos)} titles")

        for title, info in infos.items():
            if total_accepted >= args.max:
                break

            # Usability filter
            ok_use, reason_use = usability_verdict(info)
            if not ok_use:
                rejection_reasons[reason_use] = rejection_reasons.get(reason_use, 0) + 1
                continue

            # Licence filter
            extmeta = info.get("extmetadata", {}) or {}
            lic_str, _ = extract_licence(extmeta)
            ok_lic, lic_reason = licence_verdict(lic_str)
            if not ok_lic:
                rejection_reasons[lic_reason] = rejection_reasons.get(lic_reason, 0) + 1
                continue

            # Accepted!
            author = strip_html(extmeta.get("Artist", {}).get("value"))
            attribution = strip_html(
                extmeta.get("Attribution", {}).get("value")
                or extmeta.get("Credit", {}).get("value")
            )
            # JTV-127 — capture EXIF Make/Model from Commons extmetadata when
            # the uploader preserved them. Used by the handset preset to build
            # per-vendor / per-model coverage tables for the UnivFD v11 retrain.
            exif_make = strip_html(extmeta.get("Make", {}).get("value")) or None
            exif_model = strip_html(extmeta.get("Model", {}).get("value")) or None
            file_url = info.get("url")
            width = info.get("width", 0)
            height = info.get("height", 0)
            mime = info.get("mime", "")
            page_url = info.get("descriptionurl") or (
                f"https://commons.wikimedia.org/wiki/"
                f"{urllib.parse.quote(title.replace(' ', '_'))}"
            )

            licence_breakdown[lic_reason] = licence_breakdown.get(lic_reason, 0) + 1

            if args.dry_run:
                print(f"    [DRY] accept: {title[:70]}  [{lic_reason}]  "
                      f"{width}x{height}  {info.get('size', 0)//1024}KB")
                total_accepted += 1
                new_images.append({
                    "filename": None,
                    "commons_title": title,
                    "commons_url": page_url,
                    "licence": lic_reason,
                    "author": author,
                    "attribution_text": attribution,
                    "width": width,
                    "height": height,
                    "mime": mime,
                    "size_bytes": info.get("size", 0),
                    "sha256": None,
                    "category_source": category,
                    "fetched_at": None,
                    "label": manifest_label,
                    "bbox": None,
                    "mask_available": False,
                    "exif_make": exif_make,
                    "exif_model": exif_model,
                })
                continue

            # Real download
            if not file_url:
                rejection_reasons["no_url"] = rejection_reasons.get("no_url", 0) + 1
                continue

            # Compute target filename with short hash prefix for collision safety
            stem = sanitise_filename(title)
            title_hash = hashlib.sha1(title.encode("utf-8")).hexdigest()[:8]
            out_name = f"{title_hash}_{stem}"
            # Extension is MIME-driven so PNG-accepting presets keep .png
            # rather than getting forced to .jpg (which would corrupt the file).
            if mime == "image/png":
                if not out_name.lower().endswith(".png"):
                    out_name += ".png"
            elif not out_name.lower().endswith((".jpg", ".jpeg")):
                out_name += ".jpg"
            out_path = IMAGES_DIR / out_name

            if out_path.exists():
                # Resume: verify hash, skip if matches
                existing_bytes = out_path.read_bytes()
                existing_sha = hashlib.sha256(existing_bytes).hexdigest()
                print(f"    [skip-existing] {out_name}")
                new_images.append({
                    "filename": out_name,
                    "commons_title": title,
                    "commons_url": page_url,
                    "licence": lic_reason,
                    "author": author,
                    "attribution_text": attribution,
                    "width": width,
                    "height": height,
                    "mime": mime,
                    "size_bytes": len(existing_bytes),
                    "sha256": existing_sha,
                    "category_source": category,
                    "fetched_at": datetime.now(timezone.utc).isoformat(),
                    "label": manifest_label,
                    "bbox": None,
                    "mask_available": False,
                    "exif_make": exif_make,
                    "exif_model": exif_model,
                })
                total_accepted += 1
                continue

            print(f"    [fetch] {title[:70]}  [{lic_reason}]")
            blob = client.download(file_url, out_path)
            if blob is None:
                rejection_reasons["download_failed"] = (
                    rejection_reasons.get("download_failed", 0) + 1
                )
                continue

            sha = hashlib.sha256(blob).hexdigest()
            new_images.append({
                "filename": out_name,
                "commons_title": title,
                "commons_url": page_url,
                "licence": lic_reason,
                "author": author,
                "attribution_text": attribution,
                "width": width,
                "height": height,
                "mime": mime,
                "size_bytes": len(blob),
                "sha256": sha,
                "category_source": category,
                "fetched_at": datetime.now(timezone.utc).isoformat(),
                "label": manifest_label,
                "bbox": None,
                "mask_available": False,
            })
            total_accepted += 1

    # Update manifest
    manifest["generated_at"] = datetime.now(timezone.utc).isoformat()
    manifest["total_candidates_scanned"] = (
        manifest.get("total_candidates_scanned", 0) + total_scanned
    )
    manifest["total_accepted"] = (
        manifest.get("total_accepted", 0) + total_accepted
    )
    manifest["total_rejected"] = sum(rejection_reasons.values())
    manifest["rejection_reasons"] = rejection_reasons
    manifest["licence_breakdown"] = licence_breakdown
    cats = set(manifest.get("categories_queried", []))
    cats.update(categories_to_query)
    manifest["categories_queried"] = sorted(cats)
    if not args.dry_run:
        manifest["images"].extend(new_images)
        save_manifest(manifest)

    print()
    print("=" * 60)
    print(f"Run summary (this invocation):")
    print(f"  scanned:   {total_scanned}")
    print(f"  accepted:  {total_accepted}")
    print(f"  rejected:  {sum(rejection_reasons.values())}")
    print(f"  licence breakdown: {json.dumps(licence_breakdown, indent=2)}")
    print(f"  rejection reasons: {json.dumps(rejection_reasons, indent=2)}")
    if args.dry_run:
        print()
        print(f"DRY RUN: no files written. Sample accepted entries:")
        for entry in new_images[:5]:
            print(f"  - {entry['commons_title'][:70]}")
            print(f"      licence: {entry['licence']}")
            print(f"      author:  {entry['author'][:80]}")
            print(f"      dims:    {entry['width']}x{entry['height']}")
    else:
        print(f"  manifest:  {MANIFEST_PATH}")
        print(f"  images:    {IMAGES_DIR}")


if __name__ == "__main__":
    main()
