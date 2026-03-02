"""
Jura Archive Sidecar — Configuration.

Uses pydantic-settings to load from environment variables with sensible defaults.
"""

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    sidecar_port: int = 8200
    sidecar_log_level: str = "INFO"
    ollama_base_url: str = "http://localhost:11434"
    ela_quality: int = 90
    max_image_size: int = 20_000_000  # 20 MB

    model_config = {"env_prefix": "JURA_"}


settings = Settings()
