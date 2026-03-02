"""Backend/Rust specialist agent."""

from ..base_agent import BaseAgent


class BackendAgent(BaseAgent):
    name = "backend"
    description = (
        "Rust/Tauri v2, C2PA, hashing, watermarking, IPC, sidecar communication"
    )
    prompt_file = "backend.md"

    context_files = [
        "rust_lib",
        "rust_db",
        "rust_format_router",
        "rust_metadata",
        "cargo_toml",
        "tauri_conf",
    ]
    context_sections = {
        "spec": ["Technical Architecture", "Core Capabilities"],
    }
