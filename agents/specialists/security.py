"""Security specialist agent."""

from ..base_agent import BaseAgent


class SecurityAgent(BaseAgent):
    name = "security"
    description = "Threat modelling, CSP hardening, audit trail integrity, supply chain, Tauri permissions"
    prompt_file = "security.md"

    context_files = [
        "tauri_conf",
        "rust_lib",
        "cargo_toml",
    ]
    context_sections = {
        "spec": ["Technical Architecture", "Risk Register"],
    }
