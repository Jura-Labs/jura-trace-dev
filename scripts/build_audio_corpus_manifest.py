"""
build_audio_corpus_manifest.py — Sprint 35 audio corpus manifest builder (authentic side).

Walks the LibriSpeech, LJSpeech, and Common Voice directories on USB, computes
per-clip metadata, and writes/appends to models/audio_corpus_manifest.json.

Usage:
    # After LibriSpeech + LJSpeech have been extracted to USB:
    python scripts/build_audio_corpus_manifest.py \
        --librispeech "/Volumes/MAC SSD/Training Data/corpus/audio/authentic/librispeech/LibriSpeech" \
        --ljspeech    "/Volumes/MAC SSD/Training Data/corpus/audio/authentic/ljspeech/LJSpeech-1.1/wavs" \
        --manifest    models/audio_corpus_manifest.json \
        --max-librispeech 3000 \
        --max-ljspeech 500 \
        --append-manifest

    # Dry run (count clips, do not write):
    python scripts/build_audio_corpus_manifest.py ... --dry-run

Requirements: standard library only (hashlib, json, pathlib, wave).
"""

import argparse
import hashlib
import json
import os
import random
import time
import wave
from pathlib import Path

SKIP_PREFIXES = ("._",)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def wav_duration(path: Path) -> float:
    """Return duration in seconds for a WAV file."""
    try:
        with wave.open(str(path)) as wf:
            frames = wf.getnframes()
            rate = wf.getframerate()
            return round(frames / rate, 3) if rate > 0 else 0.0
    except Exception:
        return 0.0


def flac_duration_approx(path: Path) -> float:
    """Approximate FLAC duration from file size (rough: 16kHz mono ~32KB/s)."""
    size = path.stat().st_size
    return round(size / 32000, 2)


def walk_files(root: Path, extensions: tuple[str, ...]) -> list[Path]:
    results = []
    for dirpath, _, filenames in os.walk(root):
        for fn in filenames:
            if any(fn.startswith(p) for p in SKIP_PREFIXES):
                continue
            if fn.lower().endswith(extensions):
                results.append(Path(dirpath) / fn)
    return results


def build_librispeech_records(librispeech_root: Path, max_clips: int,
                               seed: int = 42) -> list[dict]:
    """Walk LibriSpeech FLAC files, sample up to max_clips."""
    print(f"  Scanning LibriSpeech: {librispeech_root}")
    all_files = walk_files(librispeech_root, (".flac",))
    print(f"    Found {len(all_files)} FLAC files")
    random.seed(seed)
    random.shuffle(all_files)
    selected = all_files[:max_clips]
    records = []
    for i, p in enumerate(selected):
        if i % 500 == 0:
            print(f"    Processing {i}/{len(selected)}...")
        # Speaker ID is the top directory under the root
        parts = p.relative_to(librispeech_root).parts
        speaker_id = parts[0] if len(parts) >= 1 else "unknown"
        duration = flac_duration_approx(p)
        records.append({
            "file": str(p),
            "source": "librispeech_test_clean",
            "licence": "CC-BY 4.0 — https://www.openslr.org/12",
            "label": "authentic",
            "speaker_id": speaker_id,
            "language": "en",
            "duration_s": duration,
            "size_bytes": p.stat().st_size,
            "sha256": sha256_file(p),
            "indexed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        })
    return records


def build_ljspeech_records(ljspeech_wavs_root: Path, max_clips: int,
                            seed: int = 42) -> list[dict]:
    """Walk LJSpeech WAV files, sample up to max_clips."""
    print(f"  Scanning LJSpeech: {ljspeech_wavs_root}")
    all_files = walk_files(ljspeech_wavs_root, (".wav",))
    print(f"    Found {len(all_files)} WAV files")
    random.seed(seed + 1)
    random.shuffle(all_files)
    selected = all_files[:max_clips]
    records = []
    for i, p in enumerate(selected):
        if i % 100 == 0:
            print(f"    Processing {i}/{len(selected)}...")
        duration = wav_duration(p)
        records.append({
            "file": str(p),
            "source": "ljspeech_1.1",
            "licence": "Public domain (CC0) — https://keithito.com/LJ-Speech-Dataset/",
            "label": "authentic",
            "speaker_id": "LJ",
            "language": "en",
            "duration_s": duration,
            "size_bytes": p.stat().st_size,
            "sha256": sha256_file(p),
            "indexed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        })
    return records


def load_existing_manifest(path: Path) -> list[dict]:
    if path.exists():
        with open(path) as f:
            return json.load(f)
    return []


def write_manifest(path: Path, records: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(records, f, indent=2)
    print(f"Manifest written: {path} ({len(records)} entries)")


def print_summary(records: list[dict]) -> None:
    by_label: dict[str, int] = {}
    by_source: dict[str, int] = {}
    by_licence: dict[str, int] = {}
    for r in records:
        lbl = r.get("label", "unknown")
        src = r.get("source", "unknown")
        lic = r.get("licence", "unknown")[:40]
        by_label[lbl] = by_label.get(lbl, 0) + 1
        by_source[src] = by_source.get(src, 0) + 1
        by_licence[lic] = by_licence.get(lic, 0) + 1
    print("\n=== Corpus summary ===")
    for lbl, count in sorted(by_label.items()):
        print(f"  {lbl}: {count} clips")
    print("\nBy source:")
    for src, count in sorted(by_source.items()):
        print(f"  {src}: {count}")
    print("\nBy licence:")
    for lic, count in sorted(by_licence.items()):
        print(f"  {lic}: {count}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Build authentic-side audio corpus manifest")
    parser.add_argument("--librispeech", help="Path to LibriSpeech root (containing speaker dirs)")
    parser.add_argument("--ljspeech", help="Path to LJSpeech wavs/ directory")
    parser.add_argument("--manifest", required=True, help="Output manifest JSON path")
    parser.add_argument("--max-librispeech", type=int, default=3000)
    parser.add_argument("--max-ljspeech", type=int, default=500)
    parser.add_argument("--append-manifest", action="store_true")
    parser.add_argument("--dry-run", action="store_true",
                        help="Count clips without writing")
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    new_records: list[dict] = []

    if args.librispeech:
        lbs_root = Path(args.librispeech)
        if lbs_root.exists():
            recs = build_librispeech_records(lbs_root, args.max_librispeech, args.seed)
            print(f"  LibriSpeech: {len(recs)} clips indexed")
            new_records.extend(recs)
        else:
            print(f"  Warning: LibriSpeech path not found: {lbs_root}")

    if args.ljspeech:
        ljs_root = Path(args.ljspeech)
        if ljs_root.exists():
            recs = build_ljspeech_records(ljs_root, args.max_ljspeech, args.seed)
            print(f"  LJSpeech: {len(recs)} clips indexed")
            new_records.extend(recs)
        else:
            print(f"  Warning: LJSpeech path not found: {ljs_root}")

    manifest_path = Path(args.manifest)
    existing = load_existing_manifest(manifest_path) if args.append_manifest else []
    all_records = existing + new_records

    print_summary(all_records)

    if args.dry_run:
        print(f"\nDry run — manifest not written. Would contain {len(all_records)} entries.")
    else:
        write_manifest(manifest_path, all_records)


if __name__ == "__main__":
    main()
