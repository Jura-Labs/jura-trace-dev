"""Abstract base class for all advisory agents."""

from abc import ABC

import anthropic

from . import config
from .context import build_context
from .tools import TOOL_DEFINITIONS, execute_tool


class BaseAgent(ABC):
    """Base class that all specialist agents inherit from.

    Handles system prompt loading, context injection, and the
    Anthropic API call loop with tool use.
    """

    # Subclasses must set these
    name: str = ""
    description: str = ""
    prompt_file: str = ""

    # Project files this agent needs for context
    context_files: list[str] = []
    context_sections: dict[str, list[str]] = {}

    def __init__(self) -> None:
        self.client = anthropic.Anthropic(api_key=config.ANTHROPIC_API_KEY)

    def load_system_prompt(self) -> str:
        """Load the agent's system prompt from its markdown file."""
        prompt_path = config.PROMPTS_DIR / self.prompt_file
        if not prompt_path.exists():
            return f"You are the {self.name} agent for the Jura Trace project."
        return prompt_path.read_text(encoding="utf-8")

    def build_system_message(self) -> str:
        """Assemble the full system message: prompt + project context."""
        prompt = self.load_system_prompt()

        if self.context_files:
            context = build_context(self.context_files, self.context_sections)
            return f"{prompt}\n\n---\n\n## Project Context\n\n{context}"

        return prompt

    def query(self, user_message: str, supporting_context: str = "") -> str:
        """Send a query to the agent and return the response.

        Args:
            user_message: The user's question or request.
            supporting_context: Optional context from a prior agent's response
                (used in multi-agent dispatch).

        Returns:
            The agent's text response.
        """
        system = self.build_system_message()
        messages = []

        if supporting_context:
            messages.append(
                {
                    "role": "user",
                    "content": (
                        f"[Context from another specialist agent]:\n\n"
                        f"{supporting_context}\n\n---\n\n"
                        f"Now answer this query:\n{user_message}"
                    ),
                }
            )
        else:
            messages.append({"role": "user", "content": user_message})

        # Tool-use loop: keep calling until the model stops using tools
        for _ in range(10):  # safety limit on tool rounds
            response = self.client.messages.create(
                model=config.AGENT_MODEL,
                max_tokens=config.MAX_RESPONSE_TOKENS,
                system=system,
                messages=messages,
                tools=TOOL_DEFINITIONS,
            )

            # Collect text and tool-use blocks
            text_parts = []
            tool_uses = []

            for block in response.content:
                if block.type == "text":
                    text_parts.append(block.text)
                elif block.type == "tool_use":
                    tool_uses.append(block)

            # If no tool calls, we're done
            if not tool_uses:
                return "\n".join(text_parts)

            # Append assistant message with all content blocks
            messages.append({"role": "assistant", "content": response.content})

            # Execute each tool and build tool results
            tool_results = []
            for tool_use in tool_uses:
                result = execute_tool(tool_use.name, tool_use.input)
                tool_results.append(
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use.id,
                        "content": result,
                    }
                )

            messages.append({"role": "user", "content": tool_results})

        # If we exhausted the loop, return whatever text we have
        return (
            "\n".join(text_parts)
            if text_parts
            else "Agent reached tool-use limit without completing."
        )
