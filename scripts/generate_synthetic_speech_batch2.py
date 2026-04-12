"""
generate_synthetic_speech_batch2.py — Sprint 35 audio deepfake corpus builder (synthetic batch 2).

Generates 3,000 more synthetic clips using:
- 30 new voices not used in batch 1 (new locales: Irish, Kenyan, South African,
  New Zealand, Irish English, Italian, Dutch, Indonesian, French-Canadian, German-Austrian)
- Different Project Gutenberg texts to avoid sentence overlap with batch 1
- Prosody variation: +10%, -10%, +20%, -20% speaking rates on ~40% of clips

Output: /Volumes/Samsung USB/Training Data/corpus/audio/synthetic/edge_tts_batch2/
Appends to existing manifest: models/audio_corpus_manifest.json

Licence: edge-tts output owned by generator (Jura Labs CIC).
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
# New voices — not in batch 1. Prioritise underrepresented accents / locales.
# ---------------------------------------------------------------------------
BATCH2_VOICES = [
    # Additional English — Ireland, Kenya, South Africa, New Zealand
    "en-IE-ConnorNeural",
    "en-IE-EmilyNeural",
    "en-KE-AsiliaNeural",
    "en-KE-ChilembaNeural",
    "en-ZA-LeahNeural",
    "en-ZA-LukeNeural",
    "en-NZ-MollyNeural",
    "en-NG-EzinneNeural",
    "en-TZ-ImaniNeural",
    "en-GB-MaisieNeural",
    "en-GB-ThomasNeural",
    "en-US-AvaNeural",
    "en-US-AndrewNeural",
    "en-US-EmmaNeural",
    "en-US-BrianNeural",
    "en-US-MichelleNeural",
    "en-US-RogerNeural",
    "en-US-SteffanNeural",
    # French — Belgian, Canadian
    "fr-BE-CharlineNeural",
    "fr-BE-GerardNeural",
    "fr-CA-SylvieNeural",
    "fr-CA-AntoineNeural",
    # German — Austrian, Swiss
    "de-AT-IngridNeural",
    "de-AT-JonasNeural",
    "de-CH-LeniNeural",
    # Italian
    "it-IT-DiegoNeural",
    "it-IT-ElsaNeural",
    # Dutch
    "nl-NL-ColetteNeural",
    "nl-NL-MaartenNeural",
    # Indonesian
    "id-ID-ArdiNeural",
    "id-ID-GadisNeural",
]

# ---------------------------------------------------------------------------
# New Gutenberg texts — different books from batch 1.
# Batch 1 used: Pride and Prejudice, Moby Dick, Sherlock Holmes,
#               A Tale of Two Cities, Jane Eyre.
# ---------------------------------------------------------------------------
GUTENBERG_URLS = [
    # Frankenstein
    "https://www.gutenberg.org/files/84/84-0.txt",
    # Dracula
    "https://www.gutenberg.org/files/345/345-0.txt",
    # The War of the Worlds
    "https://www.gutenberg.org/files/36/36-0.txt",
    # The Picture of Dorian Gray
    "https://www.gutenberg.org/files/174/174-0.txt",
    # Treasure Island
    "https://www.gutenberg.org/files/120/120-0.txt",
    # The Odyssey (Butcher & Lang translation)
    "https://www.gutenberg.org/files/1727/1727-0.txt",
]

# ---------------------------------------------------------------------------
# Prosody rate schedule — 40% of clips get rate variation.
# ---------------------------------------------------------------------------
RATES = ["+0%"] * 6 + ["+10%", "-10%", "+20%", "-20%"]  # 60% normal, 10% each variant


def fetch_gutenberg_sentences(urls: list[str], min_len: int = 50, max_len: int = 180,
                               target: int = 20000) -> list[str]:
    sentences: list[str] = []
    for url in urls:
        if len(sentences) >= target:
            break
        try:
            print(f"  Fetching: {url.split('/')[-1]}")
            with urllib.request.urlopen(url, timeout=30) as r:
                raw = r.read().decode("utf-8", errors="ignore")
            start = raw.find("*** START OF")
            end = raw.find("*** END OF")
            if start != -1:
                raw = raw[start:]
            if end != -1:
                raw = raw[:end]
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


async def generate_clip(text: str, voice: str, out_path: Path,
                         rate: str = "+0%") -> dict | None:
    try:
        communicate = edge_tts.Communicate(text, voice, rate=rate)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        await communicate.save(str(out_path))
        sha256 = hashlib.sha256(out_path.read_bytes()).hexdigest()
        size_bytes = out_path.stat().st_size
        duration_s = round(size_bytes / 16000, 2)
        locale = voice.split("-")[0] + "-" + voice.split("-")[1]
        return {
            "file": str(out_path),
            "source": "edge_tts_batch2",
            "licence": "Microsoft Edge TTS — output owned by generator (Jura Labs CIC); "
                       "commercial use permitted per Microsoft Azure Cognitive Services Terms",
            "label": "synthetic",
            "voice": voice,
            "locale": locale,
            "rate": rate,
            "gender": "unknown",
            "text_source": "Project Gutenberg (public domain)",
            "duration_s": duration_s,
            "size_bytes": size_bytes,
            "sha256": sha256,
            "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        }
    except Exception as exc:
        print(f"  [WARN] Failed {voice} rate={rate}: {exc}")
        return None


async def generate_batch(texts: list[str], voices: list[str], out_dir: Path,
                          n_clips: int, concurrency: int = 8) -> list[dict]:
    records: list[dict] = []
    clips_per_voice = max(1, n_clips // len(voices))
    semaphore = asyncio.Semaphore(concurrency)

    async def bounded(text: str, voice: str, out_path: Path, rate: str) -> dict | None:
        async with semaphore:
            return await generate_clip(text, voice, out_path, rate)

    tasks = []
    idx = 0
    for voice in voices:
        voice_dir = out_dir / voice
        voice_dir.mkdir(parents=True, exist_ok=True)
        for clip_i in range(clips_per_voice):
            if idx >= len(texts):
                idx = 0  # cycle texts if needed
            text = texts[idx]
            idx += 1
            rate = random.choice(RATES)
            rate_tag = rate.replace("+", "p").replace("-", "m").replace("%", "")
            out_path = voice_dir / f"clip_{clip_i:05d}_r{rate_tag}.mp3"
            if out_path.exists():
                print(f"  [SKIP] {out_path.name} already exists")
                continue
            tasks.append(bounded(text, voice, out_path, rate))

    # Fill remaining slots to hit n_clips exactly
    remaining = n_clips - clips_per_voice * len(voices)
    for extra in range(max(0, remaining)):
        if idx >= len(texts):
            idx = 0
        voice = voices[extra % len(voices)]
        rate = random.choice(RATES)
        rate_tag = rate.replace("+", "p").replace("-", "m").replace("%", "")
        out_path = out_dir / voice / f"clip_extra_{extra:04d}_r{rate_tag}.mp3"
        if not out_path.exists():
            tasks.append(bounded(texts[idx], voice, out_path, rate))
            idx += 1

    print(f"Generating {len(tasks)} clips across {len(voices)} voices "
          f"({concurrency} concurrent)...")
    done = 0
    for coro in asyncio.as_completed(tasks):
        result = await coro
        if result:
            records.append(result)
        done += 1
        if done % 200 == 0:
            print(f"  Progress: {done}/{len(tasks)} dispatched, "
                  f"{len(records)} succeeded")

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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True)
    parser.add_argument("--n-clips", type=int, default=3000)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--append-manifest", action="store_true")
    parser.add_argument("--concurrency", type=int, default=8)
    parser.add_argument("--seed", type=int, default=99)
    args = parser.parse_args()

    random.seed(args.seed)
    out_dir = Path(args.output)
    manifest_path = Path(args.manifest)

    print("=== Batch 2 synthetic speech generation (edge-tts) ===")
    print(f"Target: {args.n_clips} clips | Output: {out_dir}")
    print(f"Voices: {len(BATCH2_VOICES)} | Rate variants: {sorted(set(RATES))}")

    print("\n[1/3] Fetching Project Gutenberg text corpus...")
    sentences = fetch_gutenberg_sentences(GUTENBERG_URLS, target=args.n_clips * 3)
    print(f"  {len(sentences)} candidate sentences loaded")
    if len(sentences) < args.n_clips:
        print("  Warning: fewer sentences than clips — cycling texts")
        sentences = (sentences * (args.n_clips // len(sentences) + 2))[:args.n_clips * 3]

    print(f"\n[2/3] Generating {args.n_clips} clips...")
    t0 = time.time()
    records = asyncio.run(generate_batch(
        sentences, BATCH2_VOICES, out_dir, args.n_clips, args.concurrency
    ))
    elapsed = time.time() - t0
    print(f"  Done: {len(records)} clips in {elapsed:.1f}s "
          f"({elapsed / max(1, len(records)):.2f}s/clip)")

    print("\n[3/3] Writing manifest...")
    existing = load_existing_manifest(manifest_path) if args.append_manifest else []
    all_records = existing + records
    write_manifest(manifest_path, all_records)

    print(f"\n=== Summary ===")
    print(f"  Batch 2 clips generated: {len(records)}")
    print(f"  Total manifest entries: {len(all_records)}")
    by_label: dict[str, int] = {}
    by_source: dict[str, int] = {}
    for r in all_records:
        lbl = r.get("label", "unknown")
        src = r.get("source", "unknown")
        by_label[lbl] = by_label.get(lbl, 0) + 1
        by_source[src] = by_source.get(src, 0) + 1
    for lbl, count in sorted(by_label.items()):
        print(f"    {lbl}: {count}")
    print("  By source:")
    for src, count in sorted(by_source.items()):
        print(f"    {src}: {count}")


if __name__ == "__main__":
    main()
