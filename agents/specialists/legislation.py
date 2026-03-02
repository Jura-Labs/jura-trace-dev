"""Legislation and compliance specialist agent."""

from ..base_agent import BaseAgent


class LegislationAgent(BaseAgent):
    name = "legislation"
    description = "EU AI Act, GDPR, UK Data Protection, copyright, C2PA standards, cultural heritage law"
    prompt_file = "legislation.md"

    context_files = []
    context_sections = {
        "spec": ["Problem Statement", "Product Vision", "Risk Register"],
    }
