"""Rich terminal output formatting for agent responses."""

from rich.console import Console
from rich.markdown import Markdown
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

console = Console()

# Brand colours (mapped to closest Rich styles)
COLOURS = {
    "obsidian": "grey93",
    "graphite": "grey62",
    "lapis": "dodger_blue2",
    "malachite": "green3",
    "amber": "dark_orange",
    "cinnabar": "red3",
}


def print_header() -> None:
    """Print the Jura Trace advisory system header."""
    console.print()
    console.print(
        Panel(
            Text("Jura Trace — Advisory Agents", justify="center", style="bold"),
            subtitle="Know What's Real",
            border_style=COLOURS["lapis"],
        )
    )
    console.print()


def print_agent_response(agent_name: str, response: str) -> None:
    """Format and print an agent's response with a branded panel."""
    md = Markdown(response)
    console.print(
        Panel(
            md,
            title=f"[bold]{agent_name}[/bold]",
            border_style=COLOURS["malachite"],
            padding=(1, 2),
        )
    )
    console.print()


def print_routing(primary: str, supporting: list[str], reasoning: str) -> None:
    """Print the orchestrator's routing decision."""
    console.print(
        f"  [dim]Routing:[/dim] [bold]{primary}[/bold]",
        end="",
    )
    if supporting:
        console.print(f" [dim]+ {', '.join(supporting)}[/dim]", end="")
    console.print()
    console.print(f"  [dim]{reasoning}[/dim]")
    console.print()


def print_agents_table(agents: dict) -> None:
    """Print a table of available agents."""
    table = Table(title="Available Agents", border_style=COLOURS["graphite"])
    table.add_column("#", style="dim", width=3)
    table.add_column("Agent", style="bold")
    table.add_column("Description")

    for i, (name, agent) in enumerate(agents.items(), 1):
        table.add_row(str(i), name, agent.description)

    console.print(table)
    console.print()


def print_error(message: str) -> None:
    """Print an error message."""
    console.print(
        f"  [bold {COLOURS['cinnabar']}]Error:[/bold {COLOURS['cinnabar']}] {message}"
    )
    console.print()


def print_info(message: str) -> None:
    """Print an informational message."""
    console.print(f"  [dim]{message}[/dim]")


def print_thinking() -> None:
    """Print a 'thinking' indicator."""
    console.print("  [dim]Thinking...[/dim]", end="\r")
