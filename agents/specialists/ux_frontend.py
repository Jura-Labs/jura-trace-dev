"""UX/Frontend specialist agent."""

from ..base_agent import BaseAgent


class UXFrontendAgent(BaseAgent):
    name = "ux_frontend"
    description = "SvelteKit/Tailwind, WCAG 2.2 AA, dark mode, geology brand identity"
    prompt_file = "ux_frontend.md"

    context_files = [
        "layout",
        "tauri_conf",
        "package_json",
    ]
    context_sections = {
        "spec": ["Product Vision", "Target Audiences"],
        "brand": ["Colour Palette", "Typography", "Design Principles"],
    }
