"""DevOps specialist agent."""

from ..base_agent import BaseAgent


class DevOpsAgent(BaseAgent):
    name = "devops"
    description = (
        "CI/CD, Docker, cross-platform builds, Ollama deployment, sidecar orchestration"
    )
    prompt_file = "devops.md"

    context_files = [
        "makefile",
        "docker_compose",
        "cargo_toml",
        "package_json",
        "tauri_conf",
    ]
    context_sections = {
        "spec": ["Technical Architecture", "Phased Delivery"],
    }
