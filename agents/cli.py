"""CLI interface for the Jura Archive advisory agent system.

Supports three modes:
- Single query (default): python -m agents "your question"
- Direct agent: python -m agents --agent backend "your question"
- Interactive REPL: python -m agents --interactive
"""

import click

from . import AGENT_REGISTRY, __version__
from .formatting import (
    console,
    print_agent_response,
    print_agents_table,
    print_error,
    print_header,
    print_info,
    print_routing,
    print_thinking,
)
from .orchestrator import dispatch, route


def _instantiate_agents() -> dict:
    """Create instances of all registered agents."""
    return {name: cls() for name, cls in AGENT_REGISTRY.items()}


def _handle_query(query: str, agents: dict, agent_name: str | None = None) -> None:
    """Process a single query and print the response."""
    if agent_name:
        # Direct agent mode
        if agent_name not in agents:
            print_error(f"Unknown agent: {agent_name}")
            print_info(f"Available: {', '.join(agents.keys())}")
            return

        print_info(f"Querying {agent_name} agent...")
        print_thinking()
        response = agents[agent_name].query(query)
        print_agent_response(agent_name, response)
    else:
        # Orchestrator routing
        routing = route(query)
        print_routing(
            routing["primary"], routing.get("supporting", []), routing["reasoning"]
        )
        print_thinking()
        responses = dispatch(query, agents, routing)
        for name, response in responses:
            print_agent_response(name, response)


def _repl(agents: dict) -> None:
    """Run the interactive REPL."""
    print_header()
    print_info("Interactive mode. Type your question, or use commands:")
    print_info("  /agents    — List available agents")
    print_info("  /switch <name> — Talk directly to a specific agent")
    print_info("  /auto      — Return to orchestrator routing")
    print_info("  /quit      — Exit")
    console.print()

    current_agent: str | None = None

    while True:
        try:
            prompt_label = f"[{current_agent}]" if current_agent else "[auto]"
            query = console.input(f"  {prompt_label} > ").strip()
        except (EOFError, KeyboardInterrupt):
            console.print()
            print_info("Goodbye.")
            break

        if not query:
            continue

        # Handle commands
        if query.startswith("/"):
            cmd_parts = query.split(maxsplit=1)
            cmd = cmd_parts[0].lower()

            if cmd == "/quit" or cmd == "/exit" or cmd == "/q":
                print_info("Goodbye.")
                break
            elif cmd == "/agents":
                print_agents_table(agents)
            elif cmd == "/switch":
                if len(cmd_parts) < 2:
                    print_error("Usage: /switch <agent_name>")
                    continue
                name = cmd_parts[1].strip().lower()
                if name in agents:
                    current_agent = name
                    print_info(
                        f"Switched to {name} agent. Queries go directly to this agent."
                    )
                else:
                    print_error(f"Unknown agent: {name}")
                    print_info(f"Available: {', '.join(agents.keys())}")
            elif cmd == "/auto":
                current_agent = None
                print_info("Switched to auto-routing mode.")
            else:
                print_error(f"Unknown command: {cmd}")
            continue

        # Process query
        _handle_query(query, agents, agent_name=current_agent)


@click.command()
@click.argument("query", required=False)
@click.option(
    "--agent", "-a", type=str, default=None, help="Route directly to a specific agent."
)
@click.option("--interactive", "-i", is_flag=True, help="Start interactive REPL mode.")
@click.option("--list-agents", "-l", is_flag=True, help="List all available agents.")
@click.version_option(version=__version__, prog_name="jura-agents")
def main(
    query: str | None, agent: str | None, interactive: bool, list_agents: bool
) -> None:
    """Jura Archive advisory agent system.

    Ask specialist AI agents for development guidance on the
    Jura Archive content protection and verification platform.

    \b
    Examples:
      python -m agents "How should I design fingerprint lookup?"
      python -m agents --agent database "What indexes for Hamming distance?"
      python -m agents --interactive
    """
    from . import config

    if list_agents:
        # List agents doesn't require API key
        agents = _instantiate_agents()
        print_agents_table(agents)
        return

    if not config.ANTHROPIC_API_KEY:
        print_error(
            "ANTHROPIC_API_KEY not set. "
            "Add it to .env or export it as an environment variable."
        )
        raise SystemExit(1)

    agents = _instantiate_agents()

    if interactive:
        _repl(agents)
        return

    if query:
        _handle_query(query, agents, agent_name=agent)
    else:
        # No query and not interactive — show help
        ctx = click.get_current_context()
        click.echo(ctx.get_help())
