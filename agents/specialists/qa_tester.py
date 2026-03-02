"""QA/Testing specialist agent."""

from ..base_agent import BaseAgent


class QATesterAgent(BaseAgent):
    name = "qa_tester"
    description = "Test strategies (cargo test, vitest, pytest), E2E, accessibility, security testing"
    prompt_file = "qa_tester.md"

    context_files = [
        "rust_lib",
        "cargo_toml",
        "package_json",
    ]
    context_sections = {
        "spec": ["Success Metrics", "Core Capabilities"],
    }
