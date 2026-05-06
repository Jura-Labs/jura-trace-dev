# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Configuration.

Uses pydantic-settings to load from environment variables with sensible defaults.
"""

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    sidecar_port: int = 8200
    sidecar_log_level: str = "INFO"
    ollama_base_url: str = "http://localhost:11434"
    ela_quality: int = 90
    max_image_size: int = 20_000_000  # 20 MB
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
