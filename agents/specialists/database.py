"""Database specialist agent."""

from ..base_agent import BaseAgent


class DatabaseAgent(BaseAgent):
    name = "database"
    description = (
        "SQLite schema, query optimisation, migration strategy, rusqlite patterns"
    )
    prompt_file = "database.md"

    context_files = [
        "rust_db",
        "rust_lib",
        "cargo_toml",
    ]
    context_sections = {
        "spec": ["Technical Architecture", "Content Type Support"],
    }
