"""Documentation specialist agent."""

from ..base_agent import BaseAgent


class DocumentationAgent(BaseAgent):
    name = "documentation"
    description = "User guides, API docs, contributor guides, compliance documentation, brand voice"
    prompt_file = "documentation.md"

    context_files = [
        "claude",
    ]
    context_sections = {
        "spec": ["Product Vision", "Target Audiences"],
        "brand": ["Tone", "Language Rules", "Tagline"],
    }
