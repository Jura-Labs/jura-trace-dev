# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Configuration.

Uses pydantic-settings to load from environment variables with sensible defaults.
"""

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    sidecar_port: int = 8200
    sidecar_log_level: str = "INFO"
    # Use 127.0.0.1 explicitly: macOS resolves `localhost` to `::1` (IPv6) by
    # default and Ollama only binds IPv4 (127.0.0.1) unless OLLAMA_HOST=0.0.0.0
    # is set.  An IPv6-first lookup against IPv4-only Ollama produces a
    # connection-refused that the /health endpoint surfaces as
    # "Ollama unavailable" even when Ollama is running.  Force IPv4 by literal.
    ollama_base_url: str = "http://127.0.0.1:11434"
    ela_quality: int = 90
    # 200 MB — matches the Rust importer's MAX_IMPORT_FILE_SIZE_BYTES so any
    # file the desktop accepts also reaches the sidecar. The previous 20 MB
    # cap silently rejected typical 42 MP DSLR JPEGs (Sony A7R-series _DSC*),
    # which the Rust layer then counted as a sidecar-detector failure; the
    # user saw "Insufficient signal — only 2 of 13 detectors ran" (EXIF +
    # C2PA in Rust only) on a clean image.
    max_image_size: int = 200_000_000  # 200 MB
    # LLM settings — shared Ollama instance with ROOTED sibling app.
    # Primary model: qwen2.5:7b-instruct (likely already pulled by ROOTED users).
    # Override with JURA_LLM_MODEL env var if desired.
    llm_model: str = "qwen2.5:7b-instruct"
    llm_fallback_model: str = "llama3.1:8b-instruct-q4_K_M"
    # Shared-secret API key for sidecar authentication.
    # Set JURA_SIDECAR_KEY in the environment to enforce authentication.
    # If empty (default), the sidecar accepts all requests — suitable for
    # development but not recommended for shared workstations.
    sidecar_key: str = ""

    model_config = {"env_prefix": "JURA_"}


settings = Settings()
