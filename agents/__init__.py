"""Jura Archive — Advisory Agent System.

A multi-agent system providing specialist development guidance
for the Jura Archive content protection and verification platform.
"""

__version__ = "0.1.0"

from .specialists.backend import BackendAgent
from .specialists.database import DatabaseAgent
from .specialists.devops import DevOpsAgent
from .specialists.documentation import DocumentationAgent
from .specialists.end_user import EndUserAgent
from .specialists.knowledge import KnowledgeAgent
from .specialists.legislation import LegislationAgent
from .specialists.project_manager import ProjectManagerAgent
from .specialists.qa_tester import QATesterAgent
from .specialists.security import SecurityAgent
from .specialists.ux_frontend import UXFrontendAgent

AGENT_REGISTRY: dict[str, type] = {
    "database": DatabaseAgent,
    "ux_frontend": UXFrontendAgent,
    "legislation": LegislationAgent,
    "backend": BackendAgent,
    "devops": DevOpsAgent,
    "qa_tester": QATesterAgent,
    "end_user": EndUserAgent,
    "knowledge": KnowledgeAgent,
    "project_manager": ProjectManagerAgent,
    "security": SecurityAgent,
    "documentation": DocumentationAgent,
}


def get_agent(name: str):
    """Get an instantiated agent by name."""
    cls = AGENT_REGISTRY.get(name)
    if cls is None:
        raise ValueError(
            f"Unknown agent: {name}. Available: {list(AGENT_REGISTRY.keys())}"
        )
    return cls()


def list_agents() -> dict[str, str]:
    """Return a mapping of agent names to their descriptions."""
    return {name: cls.description for name, cls in AGENT_REGISTRY.items()}
