"""
Jura Trace — Agent Configuration

Central configuration for all corpus agents. Reads from environment
variables with sensible defaults for local development.
"""

import os
from pathlib import Path

# ── API endpoints ────────────────────────────────────────────────────────
JURA_API_URL = os.getenv("JURA_API_URL", "http://127.0.0.1:8300")
SIDECAR_URL = os.getenv("JURA_SIDECAR_URL", "http://127.0.0.1:8200")
JURA_API_KEY = os.getenv("JURA_API_KEY", "")

# ── Paths ────────────────────────────────────────────────────────────────
_ROOT = Path(__file__).resolve().parent.parent.parent
CORPUS_BASE = _ROOT / "corpus" / "training"
CORPUS_AI = CORPUS_BASE / "ai_generated"
CORPUS_AUTHENTIC = CORPUS_BASE / "authentic"
CORPUS_VIDEO = CORPUS_BASE / "video"
PROTECTED_BASE = _ROOT / "corpus" / "protected"
RESULTS_BASE = _ROOT / "corpus" / "results"

# ── Constraint thresholds ────────────────────────────────────────────────
# AI-generated content must ALWAYS score below this trust value, even
# after fingerprinting, watermarking, and C2PA signing.
AI_TRUST_CEILING = 0.70

# ── Crawling defaults ────────────────────────────────────────────────────
DEFAULT_MAX_PER_SOURCE = 200
REQUEST_DELAY_SECONDS = 0.5
REQUEST_TIMEOUT_SECONDS = 30

# ── Protection defaults ──────────────────────────────────────────────────
WATERMARK_PAYLOAD_HEX = "4a757261547261636554657374416700"  # "JuraTraceTestAg\0"
WATERMARK_STRENGTH = 2  # medium
C2PA_CREATOR_NAME = "Jura Trace Corpus Agent"

# ── Supported image extensions ───────────────────────────────────────────
IMAGE_EXTENSIONS = {".jpg", ".jpeg", ".png", ".webp", ".tiff", ".tif", ".bmp"}
VIDEO_EXTENSIONS = {".mp4", ".mov", ".avi", ".mkv", ".webm"}
