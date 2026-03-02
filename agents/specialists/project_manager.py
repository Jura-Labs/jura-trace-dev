"""Project Manager specialist agent."""

from ..base_agent import BaseAgent


class ProjectManagerAgent(BaseAgent):
    name = "project_manager"
    description = "Sprint planning, phase tracking, scope tradeoffs, KPI monitoring, dependency management"
    prompt_file = "project_manager.md"

    context_files = []
    context_sections = {
        "spec": [
            "Phased Delivery",
            "Success Metrics",
            "Risk Register",
            "Funding Strategy",
        ],
    }
