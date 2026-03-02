"""Configuration for the Jura Archive advisory agent system."""

import os
from pathlib import Path

from dotenv import load_dotenv

# Load .env from project root
PROJECT_ROOT = Path(__file__).resolve().parent.parent
load_dotenv(PROJECT_ROOT / ".env")

# Anthropic API
ANTHROPIC_API_KEY = os.getenv("ANTHROPIC_API_KEY", "")

# Model selection
ORCHESTRATOR_MODEL = "claude-haiku-4-5-20251001"
AGENT_MODEL = "claude-sonnet-4-20250514"

# Token limits
MAX_CONTEXT_TOKENS = 4096
MAX_RESPONSE_TOKENS = 4096

# Paths
AGENTS_DIR = Path(__file__).resolve().parent
PROMPTS_DIR = AGENTS_DIR / "prompts"
SPECIALISTS_DIR = AGENTS_DIR / "specialists"

# Project files commonly referenced by agents
PROJECT_FILES = {
    "spec": PROJECT_ROOT / "PROJECT_SPEC.md",
    "claude": PROJECT_ROOT / "CLAUDE.md",
    "architecture": PROJECT_ROOT / "docs" / "ARCHITECTURE.md",
    "brand": PROJECT_ROOT / "docs" / "BRAND_GUIDELINES.md",
    "tauri_conf": PROJECT_ROOT / "src-tauri" / "tauri.conf.json",
    "rust_lib": PROJECT_ROOT / "src-tauri" / "src" / "lib.rs",
    "rust_db": PROJECT_ROOT / "src-tauri" / "src" / "db.rs",
    "rust_format_router": PROJECT_ROOT / "src-tauri" / "src" / "format_router.rs",
    "rust_metadata": PROJECT_ROOT / "src-tauri" / "src" / "metadata.rs",
    "cargo_toml": PROJECT_ROOT / "src-tauri" / "Cargo.toml",
    "layout": PROJECT_ROOT / "ui" / "src" / "routes" / "+layout.svelte",
    "makefile": PROJECT_ROOT / "Makefile",
    "docker_compose": PROJECT_ROOT / "docker-compose.yml",
    "gitignore": PROJECT_ROOT / ".gitignore",
    "package_json": PROJECT_ROOT / "package.json",
}

# Read-only tool configuration
ALLOWED_EXTENSIONS = {
    ".rs",
    ".toml",
    ".lock",
    ".svelte",
    ".ts",
    ".js",
    ".css",
    ".html",
    ".py",
    ".txt",
    ".cfg",
    ".md",
    ".json",
    ".yaml",
    ".yml",
    ".sql",
    ".sh",
    ".dockerfile",
    ".env.example",
}

MAX_FILE_SIZE_BYTES = 100_000  # 100KB limit for file reads
MAX_GREP_RESULTS = 30
