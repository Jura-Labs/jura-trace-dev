---
title: "Jura Trace — Audio/Video Corpus Methodology"
description: "Deep review of video and audio training material sources, collection methodology, legal considerations, and agent-based workflow for expanding the training corpus beyond images."
date: 7 April 2026
status: PARTIALLY SUPERSEDED — sections 3, 5.3, 6, 8, 10, 11 superseded by docs/av-corpus-revised-proposition.md and docs/decisions/option-c-corpus-strategy.md
prepared-by: Corpus Engineering
version: 1.0
related-documents:
  - docs/corpus-state-2026-04-07.md
  - docs/sprint-plans/tried-compliance-roadmap.md
  - docs/av-corpus-revised-proposition.md (REVISED PROPOSITIONS)
  - docs/decisions/option-c-corpus-strategy.md (FORMAL DECISION)
  - Jura Trace strategic review (internal consultation document, not in this repo)
  - WITNESS TRIED Benchmark (arxiv 2504.21489)
---

> **DECISION ADOPTED 7 APRIL 2026 — Option C is the binding corpus strategy.**
>
> A single commercial-safe production model is deployed to all Jura Trace tiers (Community, Professional, Team, Enterprise). A separate research artefact, trained on additional research-licensed data, may be released under OpenRAIL-M for academic and human rights research use.
>
> The "two-model strategy" originally proposed in this document is **REJECTED**. References below to maintaining separate research and commercial production model variants are obsolete.
>
> See `docs/decisions/option-c-corpus-strategy.md` for the formal decision memo (DEC-2026-04-07-001) and `docs/av-corpus-revised-proposition.md` for the four-agent synthesis that led to this decision. Other revised propositions in those documents include:
>
> - Audio detector architecture: **W2V2-Base + MFCC ensemble**, not XLS-R-300M (latency)
> - Audio corpus target: **22,000 + 20,000 clips**, not 5,000 + 5,000
> - Face-swap detection: **disclose limitation, scope to v1.1 face-crop preprocessing**
> - DeepFake-Eval-2024: **zero-shot benchmark only, never training**
> - Drop VoxCeleb (institutional friction) and In-the-Wild (GDPR Article 9 exposure)
> - Research forms: **submit Sprint 27 (this week)**, not Sprint 30

# Audio/Video Corpus Methodology

## 1. Current State

As of 7 April 2026, the Jura Trace corpus contains **10,091 images (5,004 AI + 5,087 authentic)** with zero video and zero audio files. Despite this, the sidecar already provides:

| Capability | Implementation | Training requirement |
|---|---|---|
| **Video deepfake detection** | `sidecar/app/services/video_deepfake.py` — extracts frames via FFmpeg, runs the image GBM classifier per-frame, aggregates with temporal signals (noise/spectral/LBP drift) | **None directly.** Inherits from the 10,091-image classifier. A video test set is required to validate the pipeline. |
| **Video metadata** | `sidecar/app/services/video_metadata.py` — ffprobe-based codec/resolution/FPS extraction | None. No ML. |
| **Video frame extraction** | `sidecar/app/services/video_frames.py` — FFmpeg thumbnail strip | None. |
| **Audio metadata** | `sidecar/app/services/audio_metadata.py` — ffprobe-based codec/sample rate/channels | None. No ML. |
| **Transcription** | `sidecar/app/services/transcription.py` — faster-whisper (optional) | Uses OpenAI Whisper weights. No training. |
| **Audio deepfake / voice clone detection** | **Does not exist** | **Required.** Strategic review: "Voice clone fraud is the fastest-growing deepfake category... the current audio capability is described internally as 'basic'." |

**Two distinct corpus needs emerge:**

1. **Video test set** (validation, not training) — 100–200 clips covering authentic, face-swap deepfake, face-reenactment, and generative AI video (Sora, Veo, Kling, Runway, SVD, etc.) to verify the inherited per-frame + temporal pipeline works correctly across modes.

2. **Audio training corpus** (genuine training need) — to build a voice clone / TTS deepfake detector as a new Python sidecar service. Requires authentic speech + synthetic speech across TTS systems and languages.

The methodology below addresses both, priority-ordered by the strategic review and TRIED benchmark alignment.

---

## 2. Dataset Deep Review

### 2.1 AI Video Datasets

| Dataset | Scale | Generators | Access | Licence | Priority |
|---|---|---|---|---|---|
| **DeepFake-Eval-2024** | 44h video, 56.5h audio, 1,975 images | In-the-wild from 88 websites, 52 languages | [arxiv 2503.02857](https://arxiv.org/abs/2503.02857) | Research | ★★★★★ |
| **AV-Deepfake1M++** | 1M+ clips | LLM-driven multi-modal (2025 extension) | [GitHub ControlNet/AV-Deepfake1M](https://github.com/ControlNet/AV-Deepfake1M) | Research agreement | ★★★★★ |
| **GenVideo / GenVidBench** | 6M clips | 20+ generators including Sora/Runway/Pika/SVD | [arxiv 2501.11340](https://arxiv.org/abs/2501.11340) | Research | ★★★★☆ |
| **FaceForensics++** | 1,000 real + 4,000 fake | DeepFakes, Face2Face, FaceSwap, NeuralTextures, FaceShifter | [github.com/ondyari/FaceForensics](https://github.com/ondyari/FaceForensics) | Research agreement (form) | ★★★★☆ |
| **Celeb-DF v2** | 590 real + 5,639 fake | Improved face-swap synthesis | [github.com/yuezunli/celeb-deepfakeforensics](https://github.com/yuezunli/celeb-deepfakeforensics) | Research (form) | ★★★☆☆ |
| **DFDC** | 119,197 clips | Multiple face-swap + GAN methods, 66 actors | [ai.meta.com/datasets/dfdc](https://ai.meta.com/datasets/dfdc/) | Free, registration | ★★★☆☆ |
| **DeepfakeBench** (meta) | 9-dataset aggregator | All of the above | [github.com/SCLBD/DeepfakeBench](https://github.com/SCLBD/DeepfakeBench) | Per underlying dataset | ★★★★☆ |
| **DeeperForensics-1.0** | 60,000 videos | Face manipulation with perturbations | Academic release | Research | ★★☆☆☆ |

**DeepFake-Eval-2024 is the single most important dataset** because it mirrors exactly what TRIED describes: "in-the-wild" content from real-world distribution channels in 52 languages, with documented performance drops of 45–50% AUC versus lab-trained models. Any TRIED self-assessment claim about Pillar 1 (Real-World Adaptability) needs to be measured against this benchmark.

### 2.2 Authentic Video Datasets

| Dataset | Scale | Content | Licence | Priority |
|---|---|---|---|---|
| **Kinetics-700** | ~650K clips, 700 action classes | YouTube action recognition | CC-BY 4.0 (Google) | ★★★★★ |
| **VoxCeleb2** | ~1M utterances, 6,112 speakers | YouTube celebrity interviews | Research agreement | ★★★★☆ |
| **Pexels / Pixabay** | 60,000+ CC videos | Stock footage, diverse subjects | Free, no attribution | ★★★★☆ |
| **WebVid-2M** | 2M clips with captions | Scraped web video | Research use | ★★★☆☆ |
| **AVA (Atomic Visual Actions)** | ~430 clips × 15 min | Movies annotated with actions | Research | ★★★☆☆ |
| **HowTo100M** | 136M clips | Instructional YouTube | Research | ★★☆☆☆ |
| **UCF101 / HMDB51** | 13K / 7K clips | Action recognition classics | Research | ★★☆☆☆ |

**Priority rationale:** Kinetics (CC-BY 4.0) is the easiest legal path and matches the subject diversity Jura Trace needs. Pexels gives us professional-grade stock footage with no attribution requirement. VoxCeleb2 matches the "talking head" content type that deepfakes most commonly manipulate — important for pairing authentic talking-head videos with FaceForensics++/Celeb-DF fake equivalents.

### 2.3 AI Audio Datasets (Voice Clone / TTS Deepfake)

| Dataset | Scale | TTS systems | Languages | Access | Priority |
|---|---|---|---|---|---|
| **MLAAD v9** | 687.4h, 140 TTS models | 140 | **51** | [HF: mueller91/MLAAD](https://huggingface.co/datasets/mueller91/MLAAD) | CC-BY-NC 4.0 (matches PolyForm NC) | ★★★★★ |
| **SpeechFake** | 3M clips, 3,000+ hours | 40 | **46** | [arxiv 2507.21463](https://arxiv.org/abs/2507.21463) | Research | ★★★★★ |
| **ASVspoof 5** (2024) | 32 attack types | Multiple commercial + academic TTS | English primary | [asvspoof.org](https://www.isca-archive.org/asvspoof_2024/index.html) | Research, free | ★★★★★ |
| **WaveFake** | 104,885 clips | 7 TTS vocoders | English, Japanese | [Zenodo 5642694](https://zenodo.org/records/5642694) | Research, free | ★★★★☆ |
| **In-the-Wild** | 58 speakers, real + synthetic | From online videos (politicians, celebrities) | English primary | [HF: mueller91/In-The-Wild](https://huggingface.co/datasets/mueller91/In-The-Wild) | Research | ★★★★☆ |
| **CVoiceFake** | Built for SafeEar | 4 languages | 4 | [Zenodo 11124319](https://zenodo.org/records/11124319) | Research | ★★★☆☆ |
| **FakeAVCeleb** | Celebrity deepfakes with matched cloned audio | 5 ethnicities | English | [GitHub DASH-Lab](https://github.com/DASH-Lab/FakeAVCeleb) | Research, form | ★★★★☆ |

**Critical alignment points:**

- **MLAAD v9 is the single most TRIED-aligned audio dataset.** It covers 51 languages, CC-BY-NC 4.0 licence (compatible with Jura Trace's PolyForm NC), and directly addresses TRIED Pillar 3 (Accessibility — "most tools prioritise English/Spanish") and Pillar 1 (real-world linguistic diversity).
- **SpeechFake** is the largest audio deepfake dataset and covers 46 languages. It fills the gap between English-only research datasets and the Global Majority users WITNESS advocates for.
- **ASVspoof 5** is the official benchmark used by every serious academic paper in the field. Any professional claim must be measured against this.
- **In-the-Wild** is the audio equivalent of DeepFake-Eval-2024 — real distribution conditions, not lab data.

### 2.4 Authentic Audio Datasets

| Dataset | Scale | Content | Licence | Priority |
|---|---|---|---|---|
| **LibriSpeech** | ~1,000 hours | Read audiobook speech | CC-BY 4.0 | ★★★★★ |
| **Common Voice 17.0** | 20,000+ hours, 100+ languages | Crowd-sourced | CC-0 (but now gated via Mozilla Data Collective as of Oct 2025) | ★★★★☆ |
| **LibriTTS** | 585 hours | Multi-speaker TTS-quality | CC-BY 4.0 | ★★★★☆ |
| **VCTK** | 44 hours, 110 speakers | English voice cloning corpus | ODC-BY | ★★★☆☆ |
| **GigaSpeech** | 10,000 hours | Audiobooks + podcasts + YouTube | Research licence | ★★★☆☆ |
| **VoxCeleb1** | 100,000 utterances, 1,251 speakers | YouTube celebrity | Research agreement | ★★★★☆ |
| **MUSAN** | Music + speech + noise | Background condition augmentation | CC-BY 4.0 | ★★★☆☆ |

**LibriSpeech + Common Voice 17.0 + MLAAD v9 is the minimum viable triple** for audio deepfake detection. LibriSpeech provides clean read speech (high-quality authentic), Common Voice provides multilingual crowdsourced speech (diversity), and MLAAD provides the multilingual synthetic counterpart.

### 2.5 Audio–Video Multimodal Datasets

| Dataset | What makes it special | Priority |
|---|---|---|
| **FakeAVCeleb** | Real YouTube videos of celebrities from 5 ethnicities, with matched cloned audio AND faked video — allows testing audio-visual consistency signals (lip-sync mismatch) | ★★★★★ |
| **AV-Deepfake1M++** | 1M+ clips with both audio and visual manipulation, LLM-driven prompts | ★★★★☆ |
| **DeepFake-Eval-2024** | Audio and video sections with the same sourcing methodology | ★★★★★ |

These are the datasets to use when we eventually add cross-modal consistency detection (lip-sync analysis, voice-face matching). Not immediate priority but essential for v1.1+.

---

## 3. Priority Sequencing

Not everything needs to happen at once. Priority is driven by three constraints:

1. **What pays for itself fastest** — test sets for the existing video pipeline (no ML training needed) deliver value in days
2. **What the strategic review identified as urgent** — audio deepfake is "fastest-growing category"
3. **What unlocks TRIED credibility** — DeepFake-Eval-2024 and MLAAD directly map to TRIED's in-the-wild and multilingual requirements

| Phase | Goal | Datasets | Effort | Cost |
|---|---|---|---|---|
| **Phase 1 — Video test set** | Validate existing video deepfake pipeline works on real content across authentic/face-swap/generative | Pexels (authentic), Kinetics-700 subset (authentic), FaceForensics++ (face manipulation), Celeb-DF-v2 (celebrity face-swap) | 3–5 days | £0 |
| **Phase 2 — AI video generation coverage** | Capture 2025–2026 generator outputs the strategic review flagged (Sora, Veo, Kling, Runway, SVD, Grok Imagine) | Manual generation via APIs + DeepFake-Eval-2024 scraping | 1 week + £50–100 API | £50–100 |
| **Phase 3 — Audio test set** | Validate transcription + metadata pipeline on diverse audio | LibriSpeech subset, Common Voice subset | 2 days | £0 |
| **Phase 4 — Audio training corpus** | Build the foundation for a new voice-clone detector | MLAAD v9 + SpeechFake + LibriSpeech + In-the-Wild + ASVspoof 5 | 1–2 weeks | £0 (all free) |
| **Phase 5 — Audio deepfake detector service** | Implement `sidecar/app/services/audio_deepfake.py` with RawNet3 or SSL-based architecture | Training compute 6–12 hours on M1 or £50 Hetzner GPU | 2–3 weeks engineering | £0–50 |
| **Phase 6 — Multimodal consistency** | Lip-sync and voice-face matching signals | FakeAVCeleb, AV-Deepfake1M++ | Research spike 1 week, then 2–4 weeks implementation | £0 |

Phases 1–3 can run in parallel and are target-achievable within the existing Sprint 30–31 window. Phases 4–5 are Sprint 33–35 work (aligned with the strategic review's audio improvement recommendation). Phase 6 is Phase B / v1.1.

---

## 4. Corpus Architecture

### 4.1 Directory Structure

Extend the existing `corpus/training/` layout with two new top-level categories:

```
/Volumes/Samsung USB/Training Data/corpus/training/
├── ai_generated/          (5,004 images — existing)
├── authentic/             (5,087 images — existing)
├── video/
│   ├── ai_generated/
│   │   ├── faceforensics/    # face manipulation classics
│   │   ├── celebdf/          # celebrity face-swap
│   │   ├── dfdc/             # Meta DFDC subset
│   │   ├── sora/             # OpenAI Sora 2
│   │   ├── veo/              # Google Veo 3
│   │   ├── kling/            # Kling 2.x
│   │   ├── runway/           # Runway Gen-4
│   │   ├── svd/              # Stable Video Diffusion
│   │   ├── grok_imagine/     # xAI Grok Imagine
│   │   ├── hunyuan/          # HunyuanVideo
│   │   ├── pika/             # Pika Labs
│   │   └── luma/             # Luma Dream Machine
│   ├── authentic/
│   │   ├── kinetics/         # action recognition CC-BY 4.0
│   │   ├── pexels/           # stock footage CC
│   │   ├── voxceleb/         # talking-head (pairs with face-swap)
│   │   └── manual_curated/   # edge cases, phone-recorded
│   └── deepfake_eval_2024/   # TRIED-aligned real-world benchmark
│       ├── authentic/
│       └── manipulated/
└── audio/
    ├── ai_generated/
    │   ├── mlaad/            # 140 TTS × 51 languages
    │   ├── speechfake/       # 40 TTS × 46 languages
    │   ├── wavefake/         # vocoder artefacts
    │   ├── asvspoof5/        # official benchmark
    │   ├── elevenlabs/       # commercial voice clone
    │   ├── openai_tts/       # GPT-4o voice
    │   └── bark/             # Suno open-source
    ├── authentic/
    │   ├── librispeech/      # read audiobook
    │   ├── common_voice/     # multilingual crowdsourced
    │   ├── libritts/         # multi-speaker TTS-quality authentic
    │   └── voxceleb/         # YouTube celebrity interviews
    └── in_the_wild/          # real-distribution benchmark
        ├── authentic/
        └── synthetic/
```

### 4.2 Metadata Standard

Every corpus file is accompanied by per-directory `manifest.json` entries with the following fields:

```json
{
  "filename": "mlaad_en_xtts_v2_00000.wav",
  "source": "mlaad",
  "dataset_version": "v9",
  "sha256": "...",
  "size_bytes": 245760,
  "duration_seconds": 5.12,
  "sample_rate": 16000,
  "channels": 1,
  "codec": "pcm_s16le",
  "label": "ai_generated",
  "generator": "coqui-xtts-v2",
  "language": "en",
  "source_utterance": "LJSpeech-0001.wav",
  "licence": "CC-BY-NC-4.0",
  "downloaded_at": "2026-04-07T10:00:00Z",
  "demographic_labels": {
    "speaker_gender": "female",
    "speaker_age_range": "30-40",
    "accent_region": "US"
  }
}
```

Video entries add `resolution`, `fps`, `has_audio_track`, `generator_version`, `prompt` (when available), and `source_video_id`.

### 4.3 Train / Validation / Test Split Discipline

The corpus must preserve a strict 80/10/10 split at the source level, not the file level:

- **Training (80%):** fed to the classifier
- **Validation (10%):** used during training for early stopping and hyperparameter tuning
- **Test (10%):** held out completely, used only for final metrics

**Critical rule:** Splits are computed on the *source* level (e.g., per-speaker for audio, per-video-ID for video) — not per-file — to prevent data leakage. If speaker `voxceleb_id00012` is in training, all their clips are in training. This matches academic best practice and is what the ASVspoof 5 protocol enforces.

For the in-the-wild benchmark subsets (DeepFake-Eval-2024, In-the-Wild), the provided train/test split is used unchanged so our results are directly comparable to published literature.

---

## 5. Collection Agent Design

### 5.1 Agent Inventory

Following the existing `scripts/agents/` pattern from the image corpus work:

| Agent | File | Purpose | Status |
|---|---|---|---|
| `crawl_authentic_video` | `scripts/agents/crawl_authentic_video.py` | Download Kinetics-700 subset, Pexels, VoxCeleb via HuggingFace | **To build** |
| `crawl_ai_video` | `scripts/agents/crawl_ai_video.py` | Download FaceForensics++, Celeb-DF, DFDC via research forms + HF | **To build** |
| `generate_ai_video` | `scripts/agents/generate_ai_video.py` | Generate closed-source commercial and open-weights video-generator samples via API | **To build** (requires API keys) |
| `crawl_authentic_audio` | `scripts/agents/crawl_authentic_audio.py` | Download LibriSpeech, Common Voice, VCTK via HuggingFace | **To build** |
| `crawl_ai_audio` | `scripts/agents/crawl_ai_audio.py` | Download MLAAD, SpeechFake, WaveFake, ASVspoof 5, In-the-Wild | **To build** |
| `generate_ai_audio` | `scripts/agents/generate_ai_audio.py` | Generate custom samples via ElevenLabs/OpenAI TTS/Bark | **To build** (requires API keys) |
| `crawl_deepfake_eval_2024` | `scripts/agents/crawl_deepfake_eval_2024.py` | Pull the in-the-wild benchmark (research form) | **To build** |

All agents must:

1. Honour `JURA_CORPUS_BASE` env var (inherited from `scripts/agents/config.py`)
2. Be resume-safe (count existing files, skip by SHA-256)
3. Write per-source manifest JSON with licence field populated
4. Support `--dry-run` for prompt inspection and cost estimation
5. Print per-source cost estimates before API calls
6. Respect rate limits with documented backoff strategy

### 5.2 Reference Implementation Pattern

Each agent follows the template established by `crawl_authentic_images.py` (now with resume-safe + dedup):

```python
def download_<source>(output_dir: Path, max_items: int) -> list[dict]:
    """Docstring with dataset name, licence, and priority rationale."""
    output_dir.mkdir(parents=True, exist_ok=True)
    start_idx = _existing_item_count(output_dir)
    existing_hashes = _existing_hashes(output_dir)
    print(f"[{source}] ... (existing {start_idx})")

    # Load via HuggingFace datasets or direct API
    try:
        ds = load_dataset(DATASET_ID, split=SPLIT, streaming=True)
    except Exception as e:
        print(f"[{source}] Failed: {e}")
        return []

    entries = []
    downloaded = 0
    skipped_dupes = 0

    for item in ds:
        if downloaded >= max_items:
            break
        # ... extract, hash-check, write with start_idx offset ...

    print(f"[{source}] {downloaded} added ({skipped_dupes} dupes skipped)")
    return entries
```

This is the same pattern already proven in the image crawler work on 7 April. The agents for video and audio inherit all the improvements (env var, resume-safe, SHA-256 dedup, per-generator labelling).

### 5.3 Protection Pipeline

Once corpus items are downloaded, they go through the same protection pipeline as the image corpus (`scripts/agents/apply_protections.py`):

- **Video:** C2PA signing, perceptual fingerprint (video pHash variant), watermark embedding (DWT-DCT-SVD applied per-frame or to key frames)
- **Audio:** C2PA signing, audio fingerprint (chromaprint or Whisper embedding), watermark embedding (imwatermark audio variant)

Protected versions are stored under `corpus/protected/video/` and `corpus/protected/audio/`, parallel to the existing `corpus/protected/` for images. This allows training on both raw and protected versions, matching the existing constraint validation (AI+protection must score below 0.70 trust).

---

## 6. Legal and Ethical Considerations

The audio and video datasets bring a higher level of legal complexity than images. Five issues need explicit handling:

### 6.1 Research Licences vs Production Use

Most deepfake datasets (FaceForensics++, Celeb-DF, DFDC, FakeAVCeleb) are released under **research-only** licences. They explicitly prohibit commercial use. This matters for Jura Trace because:

- **Training on research-licensed data is generally permitted** for non-commercial research purposes. Jura Labs CIC + PolyForm Noncommercial licence is compatible.
- **Deploying a model trained on research-licensed data commercially** is legally ambiguous in the UK but definitely prohibited in the EU under recent AI Act jurisprudence.
- **Mitigation:** maintain two model variants — a research-only model trained on the full corpus including research-licensed data, and a commercial-safe model trained only on CC-BY / CC-BY-NC sources (Kinetics, Pexels, MLAAD, LibriSpeech, LibriTTS). Document which model is deployed where.

### 6.2 CC-BY-NC Compatibility with PolyForm NC

MLAAD v9 is explicitly CC-BY-NC 4.0. PolyForm Noncommercial 1.0.0 is the Jura Trace application licence. These are compatible for non-commercial use (both restrict commercial use), but the model weights derived from MLAAD inherit the CC-BY-NC restriction. **For the future commercial licence extension (planned Sprint 31–32), the derived model weights must exclude MLAAD-only features** or the commercial licence must include a clear derived-weights carve-out.

### 6.3 Voice Cloning Ethics and TfGBV

A documented HuggingFace dataset (`Mtechlaw/TfGBV-Grok-NCII-Dataset`) contains 565 instances of people requesting non-consensual intimate imagery from Grok. This is real-world evidence that voice-clone and face-swap technology is used for gender-based violence. Jura Trace must:

1. **Never generate synthetic content for training using consent-ambiguous sources.** No scraping voices from YouTube interviews to clone. Only use datasets with documented consent (LibriSpeech, LibriTTS, Common Voice, ASVspoof 5 donors).
2. **Never train a detector on datasets of non-consensual content,** even if the purpose is detection. Use only academic datasets where speakers have explicitly consented.
3. **Publish a Responsible Use statement** in the model card for the audio deepfake detector, specifically addressing TfGBV and the difference between detection (Jura Trace's purpose) and generation.

### 6.4 Copyright and Fair Dealing (UK Research Exception)

The UK has a Text and Data Mining exception for non-commercial research (CDPA s.29A). This allows scraping copyrighted content for research purposes. Jura Labs CIC + PolyForm NC licence qualifies. This is the legal basis for downloading from HuggingFace datasets that aggregate copyrighted web content (WebVid-2M, GigaSpeech). **The exception does not extend to commercial use**, which re-enforces the two-model strategy in §6.1.

### 6.5 Dataset Contamination Prevention

Many audio and video deepfake datasets were built by fine-tuning on or extracting from public datasets. For example, Celeb-DF uses YouTube clips that also appear in VoxCeleb. If we train on Celeb-DF and test on VoxCeleb, we'll see artificially high performance because speakers overlap.

**Contamination audit is mandatory** before any training run. Specifically:
- Hash every item in the training set and every item in the test set
- For video, also hash every extracted frame (Celeb-DF speaker + frame may appear in VoxCeleb)
- For audio, hash every 2-second window (to catch partial overlaps)
- Reject training runs where > 0.1% of test items have training-set overlap

This check should be part of the retraining pipeline (S31-01).

---

## 7. Integration with Existing Sidecar Services

### 7.1 Video Deepfake Pipeline (No Changes Needed)

The existing `sidecar/app/services/video_deepfake.py` already extracts frames and runs the image classifier. When we retrain the image classifier on the expanded 10,091-image corpus (S30-03), the video pipeline automatically improves. No separate video training is needed for the per-frame component.

**What is needed:** a test script that runs the full video pipeline against the video test set and measures:
- Per-mode accuracy (standard 6 frames, deep 20 frames, archival 40 frames)
- Generator-specific detection rates (Sora, Veo, Kling, Runway, SVD, FaceForensics++, Celeb-DF)
- Temporal signal contribution (does noise/spectral/LBP drift improve over pure per-frame averaging?)
- False positive rate on authentic (Kinetics, Pexels)

### 7.2 Audio Deepfake Service (New)

A new `sidecar/app/services/audio_deepfake.py` service is required for voice clone detection. The architecture mirrors the image deepfake service:

```python
# Proposed API
def perform_audio_deepfake_detection(
    audio_bytes: bytes,
    sample_rate: int | None = None,
) -> AudioDeepfakeResponse:
    """
    Detect AI-generated / voice-cloned audio.

    Returns confidence score in [0, 1] plus signal breakdown:
    - spectral_artifacts: TTS vocoder fingerprints (AASIST-style)
    - prosody_analysis: Pause patterns (strategic review reference)
    - formant_analysis: Unnatural formant transitions
    - deep_embedding: Self-supervised model (wav2vec 2.0 or XLS-R) probe
    """
```

**Recommended architecture:** a LogisticRegression or lightweight MLP probe on top of a frozen XLS-R (wav2vec 2.0 XLS) embedding — the same pattern used successfully for the UnivFD image probe (28.7% → 2.7% FP rate).

This keeps the model small (~5 MB), the training cost low (~30 minutes on M1 Mac), and the inference fast (~200 ms per 5-second clip).

### 7.3 Multimodal Consistency (Phase B)

Once both image and audio deepfake detectors are operational, a third service can combine them for video analysis with audio tracks:

```python
def perform_av_consistency_check(
    video_bytes: bytes,
) -> AVConsistencyResponse:
    """
    Detect audio-visual inconsistencies that indicate lip-sync manipulation
    or voice clone paired with genuine face (common deepfake pattern).
    """
```

Signals: lip movement vs phoneme alignment, voice gender prediction vs face gender prediction, voice age prediction vs face age prediction, room acoustic consistency vs visual scene.

Deferred to v1.1+ — out of scope for this methodology but noted for roadmap continuity.

---

## 8. Sprint Integration

Mapping this work to the existing TRIED compliance roadmap (`docs/sprint-plans/tried-compliance-roadmap.md`):

| Roadmap Sprint | Existing theme | AV corpus addition |
|---|---|---|
| Sprint 30 (19 May – 1 Jun) | Corpus to 10,000 + Retrain | **+Video test set** (Phase 1): Pexels + Kinetics subset + FaceForensics++ via research form. Validates existing video pipeline without new ML. ~100 authentic, 100 faked = 200 videos, ~3 GB disk. |
| Sprint 31 (2–15 Jun) | Durability Infrastructure | **+Audio test set** (Phase 3): LibriSpeech + Common Voice subsets for testing existing transcription pipeline. 500 authentic clips, ~1 GB. Feeds into adversarial robustness testing (S31-03). |
| Sprint 32 (16–29 Jun) | Institutional Readiness | **+DeepFake-Eval-2024 benchmark** (ties TRIED self-assessment in S32-02 to a published in-the-wild dataset). Makes the self-assessment genuinely measurable rather than self-reported. |
| Sprint 33 (30 Jun – 13 Jul) | Consumer API Foundation | **+Audio training corpus download** (Phase 4): MLAAD v9 + SpeechFake + ASVspoof 5 in parallel with consumer API work. ~50 GB, free, background download. |
| Sprint 34 (14–27 Jul) | EMIF & Ecosystem | **+AI audio generation** (Phase 2 for video, overlaps with ecosystem work): closed-source commercial video generators (manual generation for test samples). Cost: £50–100. Same pattern as the image-side commercial-generator test subset. |
| Sprint 35+ (Aug–Sep) | Audio deepfake detector | **+Phase 5 build audio_deepfake.py** service and probe. New 2-sprint project. |

### Alternative: Concentrated Corpus Sprint

If the founder prefers a single focused sprint over spreading AV work across six sprints, an alternative is:

**Sprint 31.5 (optional insertion, 16–29 Jun — replaces Sprint 32's TRIED work):** "AV Corpus Sprint"
- 3 days: all authentic video + audio crawlers (Kinetics, Pexels, LibriSpeech, Common Voice, VCTK)
- 3 days: all research-form datasets (FaceForensics++, Celeb-DF, DFDC, MLAAD, ASVspoof 5)
- 2 days: DeepFake-Eval-2024 and In-the-Wild benchmark integration
- 2 days: protection pipeline extension for video/audio
- 2 days: corpus documentation and model card updates

This is the minimum time to achieve "has a credible AV corpus" status. It defers Sprint 32's institutional readiness items by two weeks but establishes the foundation for Phase 5 (audio deepfake detector) without delay.

Recommendation: distribute across Sprints 30–34 as in the primary mapping above. The concentrated sprint is higher risk (single point of failure) and delays institutional readiness items that are more time-sensitive (EU AI Act August 2026 deadline).

---

## 9. Immediate Actions

The following work can start today without any external decisions:

1. **Create the directory structure** on the USB drive under `corpus/training/video/` and `corpus/training/audio/` with the subfolder layout in §4.1. Update `docs/corpus-state-2026-04-07.md`.

2. **Stub the six collection agents** in `scripts/agents/` with skeleton code matching the existing pattern. This makes it easy to run a specific source in isolation later.

3. **Draft the research form requests** for FaceForensics++, Celeb-DF-v2, DFDC, FakeAVCeleb, and DeepFake-Eval-2024. These have 1–7 day approval windows, so submitting early in Sprint 30 means the data is available before the sprint ends.

4. **Verify HuggingFace access** for MLAAD (`mueller91/MLAAD`), In-the-Wild (`mueller91/In-The-Wild`), Common Voice 17.0 (`mozilla-foundation/common_voice_17_0`), and LibriSpeech (`openslr/librispeech_asr`). All four are free and should work with the `datasets` library identically to the image crawlers.

5. **Draft the model card template** for the future audio deepfake detector, even before the detector exists. This documents intent, training data plan, and known limitations upfront — matching the Sprint 27 model card approach for GBM + UnivFD.

Items 1, 2, 4, and 5 are £0 and can all be done in a day. Item 3 is a few hours of form-filling plus wait time.

---

## 10. Risks and Open Questions

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Research forms take weeks to approve | 40% | Sprint 30 test set delayed | Submit all forms in Sprint 27, not Sprint 30. Use Pexels + Kinetics + HuggingFace-hosted subsets as Plan B. |
| MLAAD or In-the-Wild HF dataset removed (like OpenImages was) | 20% | Audio training corpus gap | Archive immediately on first download; maintain local mirror |
| Video test set dataset size exceeds USB drive (43GB free) | 30% | Storage pressure | Cap per-source at reasonable limits (100 clips per generator); use 480p versions not 1080p; evict source datasets after feature extraction if needed |
| Legal exposure from research-licensed data | 15% | Commercial launch blocked | Two-model strategy (§6.1); legal review before commercial licence signing (Sprint 31) |
| Demographic imbalance in downloaded datasets matches existing bias | 40% | TRIED Pillar 4 claim weakened | Demographic audit of downloaded content before training; manual curation of Global Majority content |
| Training on stale audio dataset (ASVspoof 5 from 2024 may miss 2025+ voice clones) | 25% | Detection gap on newest generators | Quarterly retraining discipline (Sprint 31-01); MLAAD v9 released Jan 2026 already covers many 2025 TTS |
| ElevenLabs / commercial TTS API terms prohibit building a detector against their output | 10% | Cannot use their voices in training | Read terms before API use; prefer open-source generators for training data where possible |

### Open Questions for Founder Decision

1. **Research licence acceptance:** Is Jura Labs CIC comfortable signing research licences for FaceForensics++, Celeb-DF, DFDC, FakeAVCeleb? They permit non-commercial research use, which matches the current PolyForm NC licence, but create constraints for the future commercial licence.

2. **API budget for AI video generation:** Sora 2 API access is ~$0.20 per 10-second clip, Veo 3 similar, Kling slightly cheaper. 100 clips across 5 generators ≈ £80–120. Is this within scope for Sprint 30 or deferred to Sprint 34?

3. **Two-model strategy commitment:** Building and maintaining two model variants (research-trained + commercial-safe) doubles the retraining workload. Is this acceptable, or do we limit training to CC-BY / CC-BY-NC sources only from the start?

4. **Phase 5 scope:** Implementing the audio deepfake detector as a new service is 2–3 weeks of engineering that isn't in the current roadmap. Does it go into Sprint 35+, or pull forward by slipping consumer API work?

5. **Storage plan:** Full AV corpus (videos + audio training) could reach 100–200 GB. Current USB drive has 43 GB free. Do we expand to a larger drive, use selective downloads (test subsets only), or migrate to cloud storage (R2, B2)?

---

## 11. Success Metrics

The AV corpus work is successful when all of the following are true:

| Metric | Target | Measurement |
|---|---|---|
| Video test set size | ≥ 200 clips (100 authentic + 100 manipulated) | Filesystem count |
| Video generator coverage | ≥ 5 distinct 2025–2026 generators represented | Per-folder audit |
| Video test set runs through existing pipeline | 100% | Automated test |
| Audio training corpus size | ≥ 10,000 clips (5,000 authentic + 5,000 synthetic) | Filesystem count |
| Audio language coverage | ≥ 20 languages | MLAAD manifest |
| Audio speaker diversity | ≥ 100 unique speakers authentic, ≥ 50 TTS systems | Per-source manifest |
| DeepFake-Eval-2024 integrated | Measurable benchmark score | AUC vs published |
| Licence documentation | Every file has licence field in manifest | Automated check |
| Model cards draft | GBM + UnivFD + planned audio detector | Help system pages |
| Contamination audit | < 0.1% train/test overlap | Automated check in pipeline |
| TRIED Pillar 1 score | 3.0 after Sprint 31 (includes video testing) | Self-assessment |
| TRIED Pillar 3 score | 2.5 → 3.0 after audio multilingual (includes MLAAD 51 languages) | Self-assessment |

---

## 12. Conclusion

The path to a credible audio/video corpus is clearer than it first appears:

- **Video deepfake detection inherits from the image classifier.** No separate training is needed, only a validation test set. This can be assembled in days from free sources.
- **Audio deepfake detection is the genuine gap.** It requires a new sidecar service, but the training data exists (MLAAD, ASVspoof 5, SpeechFake, LibriSpeech), is mostly free, and can be downloaded over a single weekend once the HuggingFace access is verified.
- **Multimodal consistency (lip-sync, voice-face matching) is the v1.1+ horizon.** Foundation is laid by getting both detectors operational.
- **The strategic review's "audio deepfake is weak" finding has a clear resolution path.** This methodology provides the corpus layer. Phase 5 adds the detector service. Phase 6 adds multimodal. Each phase is independently useful.

Cost across all phases is under £200 in API spend and zero in dataset licensing fees (all free or academic-licensed). Storage is the only hard constraint — a 500 GB external drive is the realistic minimum for full corpus + test sets + derived features, suggesting a drive upgrade alongside Phase 4.

TRIED alignment is the single strongest argument for prioritising this work. DeepFake-Eval-2024 and MLAAD v9 exist precisely because the research community identified the same gap WITNESS identified: tools trained on lab data fail in the field, and they fail hardest in non-English and non-Western contexts. Adding these datasets moves Jura Trace from "competitive with lab benchmarks" to "competitive with in-the-wild benchmarks" — which is the positioning EMIF, Mozilla, and WITNESS partnerships all require.

---

*This methodology should be reviewed at Sprint 30 start and adjusted based on the actual state of research-licence approvals, API access, and storage capacity. It is a working document, not a fixed plan.*
