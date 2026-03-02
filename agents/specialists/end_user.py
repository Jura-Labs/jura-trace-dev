"""End User specialist agent with 4 personas."""

from ..base_agent import BaseAgent


class EndUserAgent(BaseAgent):
    name = "end_user"
    description = "4 personas (curator, journalist, fact-checker, IT manager), user stories, usability review"
    prompt_file = "end_user.md"

    context_files = [
        "layout",
        "tauri_conf",
    ]
    context_sections = {
        "spec": ["Target Audiences", "Core Capabilities", "Product Vision"],
    }
