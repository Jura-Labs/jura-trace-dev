#!/usr/bin/env python3
"""
Jura Trace — DeepFake-Eval-2024 Benchmark Download Agent

Downloads the DeepFake-Eval-2024 benchmark dataset (Chandra et al., 2025,
arXiv 2503.02857) from HuggingFace for zero-shot benchmark evaluation of
the Jura Trace deepfake detection pipeline.

Dataset terms (accepted before download):
  - CC-BY-SA 4.0 licence
  - "Evaluation use only — NOT for training"
  - Not for development or improvement of harmful technologies
  - Users responsible for commercial law compliance

Jura Labs governance alignment:
  - Option C decision DEC-2026-04-07-001: single commercial-safe
    production model + OpenRAIL-M research artefact.
  - DFE-2024 is stored in a SEPARATE benchmark-only directory
    (corpus/training/benchmark/deepfake_eval_2024/), parallel to
    production/ and research/ paths.
  - A .benchmark_only sentinel file marks the directory so the training
    script firewall will refuse to include these files in any training
    corpus (production or research).

Dataset structure (from HuggingFace repo):
  - audio-data/       Audio deepfake samples (56.5 hours)
  - image-data/       Image deepfake samples (1,975 images)
  - video-data/       Video deepfake samples (44 hours)
  - examples/         Example files
  - *-metadata-publish*.csv    Metadata for each split

Required environment:
  - HF_TOKEN=<your_read_token>  (from huggingface.co/settings/tokens)
  - JURA_CORPUS_BASE=/path/to/corpus/training  (optional, defaults to
    repo-relative path via scripts.agents.config)

Citation (must be included in all derived work):
  Chandra, N. A. et al. (2025). Deepfake-Eval-2024: A Multi-Modal
  In-the-Wild Benchmark of Deepfakes Circulated in 2024.
  arXiv:2503.02857.

Usage:
    # Full download (all three splits)
    export HF_TOKEN=hf_...
    export JURA_CORPUS_BASE="/Volumes/MAC SSD/Training Data/corpus/training"
    python -m scripts.agents.crawl_deepfake_eval_2024

    # Images only (fastest, ~1 GB, useful for smoke test)
    python -m scripts.agents.crawl_deepfake_eval_2024 --splits image

    # Metadata only (manifest-first mode, ~3 MB, no media)
    python -m scripts.agents.crawl_deepfake_eval_2024 --metadata-only

    # Audio + image (skip video for disk-constrained runs)
    python -m scripts.agents.crawl_deepfake_eval_2024 --splits image,audio
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent))

from scripts.agents import config

# ── Dataset identifiers ─────────────────────────────────────────────────
DATASET_REPO = "nuriachandra/Deepfake-Eval-2024"
DATASET_REF = "main"
LOCAL_SUBDIR = "benchmark/deepfake_eval_2024"

SPLIT_FOLDERS = {
    "image": "image-data",
    "audio": "audio-data",
    "video": "video-data",
}

METADATA_FILES = [
    "README.md",
    "audio-metadata-publish.csv",
    "audio-metadata-publish-with-links.csv",
    "image-metadata-publish.csv",
    "image-metadata-publish-with-links.csv",
    "video-metadata-publish-with-links.csv",
]

# ── Disk safety thresholds ──────────────────────────────────────────────
MIN_FREE_SPACE_GB = 5.0  # abort if free space drops below this
SIZE_CAP_GB_DEFAULT = 40.0  # abort if total probed size exceeds this

# ── Citation and terms file ─────────────────────────────────────────────
CITATION_TEXT = """\
# DeepFake-Eval-2024 — Citation and Terms

## Citation (required in all derived publications)

Chandra, N. A., Murtfeldt, R., Qiu, L., Karmakar, A., Lee, H.,
Tanumihardja, E., Farhat, K., Caffee, B., Paik, S., Lee, C., Choi, J.,
Kim, A., & Etzioni, O. (2025). Deepfake-Eval-2024: A Multi-Modal
In-the-Wild Benchmark of Deepfakes Circulated in 2024.
arXiv:2503.02857. https://arxiv.org/abs/2503.02857

## Licence

CC BY-SA 4.0 (Creative Commons Attribution-ShareAlike 4.0 International)

## Terms of use (accepted on download)

- Evaluation use only — NOT for training
- Not for development or improvement of harmful technologies
- Attribution required in all derived work
- Share-alike on any derivative works
- Users responsible for commercial law compliance

## Jura Labs use declaration

This dataset is used exclusively for zero-shot benchmark evaluation of
the Jura Trace forensic detection pipeline (image, video, and audio
deepfake detectors). It is stored under `corpus/training/benchmark/` in
strict segregation from the production (`corpus/training/production/`)
and research (`corpus/training/research/`) training corpora.

Under Jura Labs CIC decision DEC-2026-04-07-001 (Option C corpus
strategy), a training-script firewall refuses to include any file from
this directory in any training run, regardless of model variant.

## Source

https://huggingface.co/datasets/nuriachandra/Deepfake-Eval-2024
https://github.com/nuriachandra/Deepfake-Eval-2024
"""

SENTINEL_TEXT = """\
Benchmark-only corpus directory.

Files under this path MUST NOT be used in any training corpus (production
or research artefact) under Jura Labs CIC decision DEC-2026-04-07-001
(Option C corpus strategy).

The training-script firewall enforces this by refusing to enumerate any
directory containing a .benchmark_only sentinel file. Do not remove this
sentinel unless the governing decision is formally revoked.

Dataset: DeepFake-Eval-2024 (Chandra et al., 2025, arXiv 2503.02857)
Licence: CC BY-SA 4.0
Terms: Evaluation use only. Not for training.
"""


# ── Disk safety helpers ────────────────────────────────────────────────
def _free_gb(path: Path) -> float:
    """Return free space at path in GB (base 2)."""
    usage = shutil.disk_usage(str(path))
    return usage.free / (1024**3)


def _abort_if_low_disk(path: Path, needed_gb: float = MIN_FREE_SPACE_GB) -> None:
    free = _free_gb(path)
    if free < needed_gb:
        print(f"  ERROR: Free space at {path} is {free:.1f} GB, below threshold {needed_gb} GB")
        print("  Aborting before media download to prevent disk exhaustion.")
        sys.exit(2)


# ── HuggingFace helpers ────────────────────────────────────────────────
def _ensure_hf_token() -> str:
    token = os.environ.get("HF_TOKEN") or os.environ.get("HUGGING_FACE_HUB_TOKEN")
    if not token:
        print("  ERROR: HF_TOKEN not set.")
        print("  Get a read token at https://huggingface.co/settings/tokens")
        print("  Then: export HF_TOKEN=hf_...")
        print("  You must also accept the dataset terms at:")
        print("    https://huggingface.co/datasets/nuriachandra/Deepfake-Eval-2024")
        sys.exit(1)
    return token


def _load_huggingface_hub():
    try:
        from huggingface_hub import HfApi, hf_hub_download, list_repo_files
        from huggingface_hub.errors import GatedRepoError, HfHubHTTPError
    except ImportError:
        print("  ERROR: huggingface_hub not installed.")
        print("  Run: pip install huggingface_hub")
        sys.exit(1)
    return HfApi, hf_hub_download, list_repo_files, GatedRepoError, HfHubHTTPError


# ── Manifest and size probe ────────────────────────────────────────────
def _probe_repo_files(repo_id: str, token: str):
    """List all files in the dataset repo with approximate sizes."""
    HfApi, _, _, GatedRepoError, HfHubHTTPError = _load_huggingface_hub()
    api = HfApi(token=token)

    try:
        info = api.dataset_info(repo_id, files_metadata=True)
    except GatedRepoError:
        print(f"  ERROR: Dataset {repo_id} is gated and access has not been granted.")
        print("  Accept the terms of use at:")
        print(f"    https://huggingface.co/datasets/{repo_id}")
        print("  Then re-run this script.")
        sys.exit(3)
    except HfHubHTTPError as e:
        print(f"  ERROR: HuggingFace API error: {e}")
        sys.exit(4)

    files = []
    total_bytes = 0
    for sibling in info.siblings:
        rfilename = sibling.rfilename
        size = getattr(sibling, "size", None) or 0
        files.append((rfilename, size))
        total_bytes += size

    return files, total_bytes


def _classify_file(rfilename: str) -> str:
    """Return split name ('image', 'audio', 'video', 'metadata', 'other')."""
    lower = rfilename.lower()
    if lower.startswith("image-data/"):
        return "image"
    if lower.startswith("audio-data/"):
        return "audio"
    if lower.startswith("video-data/"):
        return "video"
    if lower.startswith("examples/"):
        return "metadata"  # treat examples as metadata — tiny and always included
    if lower.endswith(".csv") or lower.endswith(".md") or lower.endswith(".json"):
        return "metadata"
    return "other"


# ── Download loop ──────────────────────────────────────────────────────
def _download_file(
    repo_id: str,
    filename: str,
    output_dir: Path,
    token: str,
) -> Path | None:
    """Download a single file from HF to output_dir, returning the local path."""
    _, hf_hub_download, _, _, HfHubHTTPError = _load_huggingface_hub()

    # Abort if disk is low before each download
    _abort_if_low_disk(output_dir)

    try:
        local_path = hf_hub_download(
            repo_id=repo_id,
            filename=filename,
            repo_type="dataset",
            revision=DATASET_REF,
            local_dir=str(output_dir),
            token=token,
            # local_dir_use_symlinks=False is default in modern huggingface_hub
        )
        return Path(local_path)
    except HfHubHTTPError as e:
        print(f"    Warning: HF HTTP error on {filename}: {e}")
        return None
    except Exception as e:
        print(f"    Warning: download failed for {filename}: {e}")
        return None


def _write_sentinel_and_citation(output_dir: Path) -> None:
    """Write the .benchmark_only sentinel and the CITATION.txt to output_dir."""
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / ".benchmark_only").write_text(SENTINEL_TEXT)
    (output_dir / "CITATION.txt").write_text(CITATION_TEXT)
    print(f"  Wrote .benchmark_only sentinel to {output_dir}/.benchmark_only")
    print(f"  Wrote CITATION.txt to {output_dir}/CITATION.txt")


# ── Main entry point ───────────────────────────────────────────────────
def main() -> None:
    parser = argparse.ArgumentParser(
        description="Jura Trace — DeepFake-Eval-2024 Benchmark Download Agent",
    )
    parser.add_argument(
        "--splits",
        default="image,audio,video",
        help="Comma-separated splits to download (image,audio,video) — default: all",
    )
    parser.add_argument(
        "--metadata-only",
        action="store_true",
        help="Download only the metadata CSVs and README, not the media files",
    )
    parser.add_argument(
        "--size-cap-gb",
        type=float,
        default=SIZE_CAP_GB_DEFAULT,
        help=f"Abort if total probed size exceeds this many GB (default {SIZE_CAP_GB_DEFAULT})",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help=f"Output directory (default: $JURA_CORPUS_BASE/{LOCAL_SUBDIR})",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Probe sizes and show the download plan without downloading any media",
    )
    args = parser.parse_args()

    print("=" * 60)
    print("  Jura Trace — DeepFake-Eval-2024 Download Agent")
    print("=" * 60)

    requested_splits = {s.strip().lower() for s in args.splits.split(",") if s.strip()}
    valid_splits = set(SPLIT_FOLDERS.keys())
    unknown = requested_splits - valid_splits
    if unknown:
        print(f"  ERROR: unknown splits: {unknown}. Valid: {valid_splits}")
        sys.exit(1)

    output_dir = args.output or (config.CORPUS_BASE / LOCAL_SUBDIR)
    output_dir = Path(output_dir)
    print(f"  Output:           {output_dir}")
    print(f"  Splits requested: {sorted(requested_splits)}")
    print(f"  Size cap:         {args.size_cap_gb:.1f} GB")
    print(f"  Metadata only:    {args.metadata_only}")
    print(f"  Dry run:          {args.dry_run}")

    # Ensure output parent exists and check free space
    output_dir.parent.mkdir(parents=True, exist_ok=True)
    free = _free_gb(output_dir.parent)
    print(f"  Free space:       {free:.1f} GB at {output_dir.parent}")

    if free < MIN_FREE_SPACE_GB:
        print(f"  ERROR: Free space {free:.1f} GB is below minimum {MIN_FREE_SPACE_GB} GB.")
        sys.exit(2)

    # Verify HF token is set
    token = _ensure_hf_token()
    print("  HF_TOKEN:         set")

    # Probe the dataset file list
    print("\n  Probing dataset file list from HuggingFace...")
    files, total_bytes = _probe_repo_files(DATASET_REPO, token)
    total_gb = total_bytes / (1024**3)
    print(f"  Total files:      {len(files)}")
    print(f"  Total size:       {total_gb:.2f} GB")

    # Group by split
    by_split: dict[str, list[tuple[str, int]]] = {k: [] for k in ["image", "audio", "video", "metadata", "other"]}
    for rfilename, size in files:
        by_split[_classify_file(rfilename)].append((rfilename, size))

    print("\n  Split breakdown:")
    for split_name, split_files in by_split.items():
        split_size = sum(size for _, size in split_files)
        split_gb = split_size / (1024**3)
        print(f"    {split_name:>10}: {len(split_files):>6} files, {split_gb:>7.2f} GB")

    # Size cap check (before download)
    if total_gb > args.size_cap_gb:
        print(f"\n  ERROR: Total dataset size {total_gb:.1f} GB exceeds size cap {args.size_cap_gb:.1f} GB.")
        print("  Options: raise --size-cap-gb, download fewer splits, or free disk space.")
        sys.exit(2)

    if total_gb > free - MIN_FREE_SPACE_GB:
        print(f"\n  ERROR: Total dataset size {total_gb:.1f} GB does not fit in {free:.1f} GB free (need {MIN_FREE_SPACE_GB} GB headroom).")
        sys.exit(2)

    # Decide which files to actually download
    to_download: list[tuple[str, int]] = []
    # Always include metadata (small)
    to_download.extend(by_split["metadata"])
    # Always include "other" if tiny (< 10 MB each)
    to_download.extend([(f, s) for f, s in by_split["other"] if s < 10 * 1024 * 1024])

    if not args.metadata_only:
        for split in requested_splits:
            to_download.extend(by_split[split])

    planned_bytes = sum(size for _, size in to_download)
    planned_gb = planned_bytes / (1024**3)
    print(f"\n  Planned download: {len(to_download)} files, {planned_gb:.2f} GB")

    if args.dry_run:
        print("\n  DRY RUN — no files will be downloaded. Exiting.")
        return

    # Write sentinel and citation BEFORE downloading any media, so the firewall
    # is in place from the moment the first file lands on disk.
    _write_sentinel_and_citation(output_dir)

    # Download loop
    print("\n  Starting download...")
    downloaded = 0
    failed = 0
    manifest_entries = []

    for i, (rfilename, size) in enumerate(to_download, start=1):
        size_mb = size / (1024**2)
        print(f"  [{i}/{len(to_download)}] {rfilename} ({size_mb:.1f} MB)")
        local_path = _download_file(DATASET_REPO, rfilename, output_dir, token)
        if local_path is None:
            failed += 1
            continue
        downloaded += 1
        manifest_entries.append({
            "rfilename": rfilename,
            "local_path": str(local_path.relative_to(output_dir)),
            "size_bytes": size,
            "split": _classify_file(rfilename),
        })

    # Write manifest
    manifest_path = output_dir / "jura_download_manifest.json"
    manifest = {
        "dataset": DATASET_REPO,
        "arxiv_id": "2503.02857",
        "citation_file": "CITATION.txt",
        "licence": "CC-BY-SA-4.0",
        "terms": "Evaluation use only — NOT for training",
        "governing_decision": "DEC-2026-04-07-001 (Option C)",
        "downloaded_at": datetime.now(timezone.utc).isoformat(),
        "total_files": len(manifest_entries),
        "total_bytes": sum(e["size_bytes"] for e in manifest_entries),
        "failed_count": failed,
        "entries": manifest_entries,
    }
    manifest_path.write_text(json.dumps(manifest, indent=2))

    # Final summary
    print("\n" + "=" * 60)
    print(f"  Download complete")
    print(f"  Files downloaded:  {downloaded}")
    print(f"  Files failed:      {failed}")
    print(f"  Total size:        {manifest['total_bytes'] / (1024**3):.2f} GB")
    print(f"  Output directory:  {output_dir}")
    print(f"  Manifest:          {manifest_path}")
    print(f"  Sentinel:          {output_dir / '.benchmark_only'}")
    print(f"  Citation:          {output_dir / 'CITATION.txt'}")
    print("  Reminder: this dataset is evaluation-only. DO NOT use in training.")
    print("=" * 60)

    if failed > 0:
        sys.exit(5)


if __name__ == "__main__":
    main()
