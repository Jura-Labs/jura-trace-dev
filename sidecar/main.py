"""
Jura Trace — Python ML Sidecar

Provides image forensics, deepfake detection, and RAG pipeline
capabilities that require Python ML libraries.

Usage:
    uvicorn main:app --host 127.0.0.1 --port 8200 --reload
"""

from fastapi import FastAPI

from app.api import forensics, health

app = FastAPI(
    title="Jura Trace ML Sidecar",
    version="0.2.0",
    description="Local ML services for content forensics and verification",
)

app.include_router(health.router, tags=["health"])
app.include_router(forensics.router, prefix="/forensics", tags=["forensics"])

if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="127.0.0.1", port=8200, log_level="info")
