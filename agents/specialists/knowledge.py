"""Domain knowledge specialist agent."""

from ..base_agent import BaseAgent


class KnowledgeAgent(BaseAgent):
    name = "knowledge"
    description = "IP law, fake news detection, AI algorithms, C2PA ecosystem, perceptual hashing, deepfake methods"
    prompt_file = "knowledge.md"

    context_files = [
        "rust_lib",
    ]
    context_sections = {
        "spec": [
            "Core Capabilities",
            "Content Type Support",
            "Technical Architecture",
            "Competitive Landscape",
        ],
    }
