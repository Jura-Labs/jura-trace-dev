# SPDX-License-Identifier: AGPL-3.0-or-later

"""
generate_synthetic_speech.py — Sprint 35 audio deepfake corpus builder (synthetic side).

Generates synthetic speech clips using edge-tts (Microsoft Edge TTS, free, no API key,
commercial-use OK per Microsoft service terms). Reads sentences from a text corpus
(Project Gutenberg public-domain source) and distributes generation across multiple
voices to maximise speaker and accent diversity.

Licence of generated output: owned by Jura Labs CIC — no third-party licence
encumbrance on the synthesised audio.

Usage:
    python scripts/generate_synthetic_speech.py \
        --output "/Volumes/MAC SSD/Training Data/corpus/audio/synthetic/edge_tts" \
        --n-clips 2000 \
        --voices-per-locale 2 \
        --manifest models/audio_corpus_manifest.json \
        --append-manifest

Requirements:
    edge-tts >= 7.0 (already installed: 7.2.8)
"""

import argparse
import asyncio
import hashlib
import json
import os
import random
import re
import time
import urllib.request
from pathlib import Path

import edge_tts

# ---------------------------------------------------------------------------
# Voice selection — representative sample across locales and genders.
# Chosen to cover: English (AU, CA, GB, HK, IN, NG, NZ, PH, SG, TZ, US),
# plus a handful of non-English locales for language diversity.
# ---------------------------------------------------------------------------
TARGET_VOICES = [
    # English — multiple accents
    "en-AU-WilliamMultilingualNeural",
    "en-AU-NatashaNeural",
    "en-CA-LiamNeural",
    "en-CA-ClaraNeural",
    "en-GB-RyanNeural",
    "en-GB-SoniaNeural",
    "en-GB-LibbyNeural",
    "en-HK-SamNeural",
    "en-HK-YanNeural",
    "en-IN-PrabhatNeural",
    "en-IN-NeerjaNeural",
    "en-NG-AbeoNeural",
    "en-NZ-MitchellNeural",
    "en-PH-JamesNeural",
    "en-SG-WayneNeural",
    "en-TZ-ElimuNeural",
    "en-US-ChristopherNeural",
    "en-US-JennyNeural",
    "en-US-EricNeural",
    "en-US-AriaNeural",
    "en-US-GuyNeural",
    "en-US-MonicaNeural",
    # Non-English — language diversity sample
    "fr-FR-HenriNeural",
    "fr-FR-DeniseNeural",
    "de-DE-ConradNeural",
    "de-DE-KatjaNeural",
    "es-ES-AlvaroNeural",
    "es-ES-ElviraNeural",
    "pt-BR-FranciscaNeural",
    "ar-SA-HamedNeural",
    "zh-CN-YunxiNeural",
    "zh-CN-XiaoxiaoNeural",
    "hi-IN-MadhurNeural",
    "ja-JP-KeitaNeural",
    "ko-KR-InJoonNeural",
    "sw-KE-RafikiNeural",
]

# ---------------------------------------------------------------------------
# Text corpus — Project Gutenberg public-domain sentences.
# We fetch a handful of books and extract sentences of 50–200 chars.
# ---------------------------------------------------------------------------
GUTENBERG_URLS = [
    # Pride and Prejudice
    "https://www.gutenberg.org/files/1342/1342-0.txt",
    # Moby Dick
    "https://www.gutenberg.org/files/2701/2701-0.txt",
    # The Adventures of Sherlock Holmes
    "https://www.gutenberg.org/files/1661/1661-0.txt",
    # A Tale of Two Cities
    "https://www.gutenberg.org/files/98/98-0.txt",
    # Jane Eyre
    "https://www.gutenberg.org/files/1260/1260-0.txt",
]


def fetch_gutenberg_sentences(urls: list[str], min_len: int = 50, max_len: int = 180,
                               target: int = 20000) -> list[str]:
    """Download Gutenberg texts and extract clean sentences."""
    sentences: list[str] = []
    for url in urls:
        if len(sentences) >= target:
            break
        try:
            print(f"  Fetching text corpus: {url.split('/')[-1]}")
            with urllib.request.urlopen(url, timeout=30) as r:
                raw = r.read().decode("utf-8", errors="ignore")
            # Strip Gutenberg header/footer
            start = raw.find("*** START OF")
            end = raw.find("*** END OF")
            if start != -1:
                raw = raw[start:]
            if end != -1:
                raw = raw[:end]
            # Split into sentences (rough heuristic)
            raw = re.sub(r"\r\n", "\n", raw)
            raw = re.sub(r"[ \t]+", " ", raw)
            chunks = re.split(r"(?<=[.!?])\s+", raw)
            for chunk in chunks:
                chunk = chunk.strip()
                chunk = re.sub(r"\s+", " ", chunk)
                if min_len <= len(chunk) <= max_len and chunk[0].isupper():
                    sentences.append(chunk)
        except Exception as exc:
            print(f"  Warning: could not fetch {url}: {exc}")
    random.shuffle(sentences)
    return sentences


async def generate_clip(text: str, voice: str, out_path: Path, rate: str = "+0%",
                         volume: str = "+0%") -> dict | None:
    """Generate a single clip and return metadata dict, or None on failure."""
    try:
        communicate = edge_tts.Communicate(text, voice, rate=rate, volume=volume)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        await communicate.save(str(out_path))
        # Compute SHA-256
        sha256 = hashlib.sha256(out_path.read_bytes()).hexdigest()
        # Estimate duration via file size (edge-tts MP3 ~128kbps)
        size_bytes = out_path.stat().st_size
        duration_s = round(size_bytes / 16000, 2)  # rough: 128kbps = 16000 bytes/sec
        locale = voice.split("-")[0] + "-" + voice.split("-")[1]
        return {
            "file": str(out_path),
            "source": "edge_tts",
            "licence": "Microsoft Edge TTS — output owned by generator (Jura Labs CIC); "
                       "commercial use permitted per Microsoft Azure Cognitive Services Terms",
            "label": "synthetic",
            "voice": voice,
            "locale": locale,
            "gender": "unknown",  # could be enriched from voice list metadata
            "text_source": "Project Gutenberg (public domain)",
            "duration_s": duration_s,
            "size_bytes": size_bytes,
            "sha256": sha256,
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
    except Exception as exc:
        print(f"  [WARN] Failed {voice}: {exc}")
        return None


async def generate_batch(texts: list[str], voices: list[str], out_dir: Path,
                          n_clips: int, concurrency: int = 8) -> list[dict]:
    """Generate n_clips synthetic clips distributed across voices."""
    records: list[dict] = []
    clips_per_voice = max(1, n_clips // len(voices))
    semaphore = asyncio.Semaphore(concurrency)

    async def bounded(text: str, voice: str, out_path: Path) -> dict | None:
        async with semaphore:
            return await generate_clip(text, voice, out_path)

    tasks = []
    idx = 0
    for voice in voices:
        voice_dir = out_dir / voice
        voice_dir.mkdir(parents=True, exist_ok=True)
        for clip_i in range(clips_per_voice):
            if idx >= len(texts):
                break
            text = texts[idx]
            idx += 1
            out_path = voice_dir / f"clip_{clip_i:05d}.mp3"
            if out_path.exists():
                print(f"  [SKIP] {out_path.name} already exists")
                continue
            tasks.append(bounded(text, voice, out_path))

    # Fill remaining slots up to n_clips
    remaining = n_clips - clips_per_voice * len(voices)
    for extra in range(max(0, remaining)):
        if idx >= len(texts):
            break
        voice = voices[extra % len(voices)]
        out_path = out_dir / voice / f"clip_extra_{extra:04d}.mp3"
        if not out_path.exists():
            tasks.append(bounded(texts[idx], voice, out_path))
            idx += 1

    print(f"Generating {len(tasks)} clips across {len(voices)} voices "
          f"({concurrency} concurrent)...")
    done = 0
    for coro in asyncio.as_completed(tasks):
        result = await coro
        if result:
            records.append(result)
        done += 1
        if done % 100 == 0:
            print(f"  Progress: {done}/{len(tasks)} clips generated, "
                  f"{len(records)} succeeded")

    return records


def load_existing_manifest(manifest_path: Path) -> list[dict]:
    if manifest_path.exists():
        with open(manifest_path) as f:
            return json.load(f)
    return []


def write_manifest(manifest_path: Path, records: list[dict]) -> None:
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    with open(manifest_path, "w") as f:
        json.dump(records, f, indent=2)
    print(f"Manifest written: {manifest_path} ({len(records)} entries)")


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate synthetic speech for audio deepfake corpus")
    parser.add_argument("--output", required=True, help="Output directory for synthetic clips")
    parser.add_argument("--n-clips", type=int, default=2000, help="Number of clips to generate")
    parser.add_argument("--voices-per-locale", type=int, default=2,
                        help="Max voices per locale (not yet implemented — use TARGET_VOICES directly)")
    parser.add_argument("--manifest", required=True, help="Path to corpus manifest JSON")
    parser.add_argument("--append-manifest", action="store_true",
                        help="Append to existing manifest instead of overwriting")
    parser.add_argument("--concurrency", type=int, default=8,
                        help="Concurrent TTS requests")
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    random.seed(args.seed)
    out_dir = Path(args.output)
    manifest_path = Path(args.manifest)

    print("=== Sprint 35 synthetic speech generation (edge-tts) ===")
    print(f"Target: {args.n_clips} clips | Output: {out_dir}")
    print(f"Voices: {len(TARGET_VOICES)} selected")

    # Fetch text corpus
    print("\n[1/3] Fetching Project Gutenberg text corpus...")
    sentences = fetch_gutenberg_sentences(GUTENBERG_URLS, target=args.n_clips * 2)
    print(f"  {len(sentences)} candidate sentences loaded")
    if len(sentences) < args.n_clips:
        print("  Warning: fewer sentences than target clips — some texts will repeat")
        sentences = (sentences * (args.n_clips // len(sentences) + 2))[:args.n_clips * 2]

    # Generate clips
    print(f"\n[2/3] Generating {args.n_clips} synthetic clips...")
    t0 = time.time()
    records = asyncio.run(generate_batch(
        sentences, TARGET_VOICES, out_dir, args.n_clips, args.concurrency
    ))
    elapsed = time.time() - t0
    print(f"  Done: {len(records)} clips in {elapsed:.1f}s "
          f"({elapsed / max(1, len(records)):.2f}s/clip)")

    # Write manifest
    print(f"\n[3/3] Writing manifest...")
    existing = load_existing_manifest(manifest_path) if args.append_manifest else []
    all_records = existing + records
    write_manifest(manifest_path, all_records)

    print(f"\n=== Summary ===")
    print(f"  Synthetic clips generated: {len(records)}")
    print(f"  Total manifest entries: {len(all_records)}")
    by_label = {}
    for r in all_records:
        by_label[r.get("label", "unknown")] = by_label.get(r.get("label", "unknown"), 0) + 1
    for label, count in sorted(by_label.items()):
        print(f"    {label}: {count}")


if __name__ == "__main__":
    main()
