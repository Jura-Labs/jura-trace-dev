# Jura Archive — Python ML Sidecar

This is a FastAPI service providing ML capabilities that require Python libraries.
It runs as a managed sidecar process launched by the Tauri desktop application.

## Status

Phase 2+ — not required for MVP. The MVP (PROTECT pipeline) runs entirely
in Rust via the Tauri backend.

## Architecture

```
Tauri App (Rust) --HTTP--> Python Sidecar (Port 8200)
                               |
                               ├── /forensics/ela      (Error Level Analysis)
                               ├── /forensics/noise     (Noise pattern analysis)
                               ├── /forensics/metadata  (Deep EXIF inspection)
                               ├── /deepfake/detect     (DeepSafe ensemble)
                               └── /rag/check-claim     (Fact-check RAG)
```

## Setup

```bash
cd sidecar
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
uvicorn main:app --host 127.0.0.1 --port 8200 --reload
```

## Key Libraries

- **Sherloq** — Open-source digital image forensics toolset
- **ImageHash** — Perceptual hashing (pHash, aHash, dHash, wHash)
- **DeepSafe** — Containerised deepfake detection ensemble
- **ChromaDB** — Vector store for RAG fact-checking pipeline
