"""
Jura Archive — Python ML Sidecar

Provides image forensics, deepfake detection, and RAG pipeline
capabilities that require Python ML libraries.

Phase 2+ — not required for MVP.

Usage:
    uvicorn main:app --host 127.0.0.1 --port 8200 --reload
"""

from fastapi import FastAPI

app = FastAPI(
    title="Jura Archive ML Sidecar",
    version="0.1.0",
    description="Local ML services for content forensics and verification",
)


@app.get("/health")
async def health():
    return {"status": "ok", "version": "0.1.0"}


# Phase 2: Image forensics endpoints
# @app.post("/forensics/ela")
# @app.post("/forensics/noise")
# @app.post("/forensics/metadata")

# Phase 2: Deepfake detection
# @app.post("/deepfake/detect")

# Phase 2: RAG claim checking
# @app.post("/rag/check-claim")
