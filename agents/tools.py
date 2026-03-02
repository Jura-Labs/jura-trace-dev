"""Read-only tools available to advisory agents.

Agents can read project files and search the codebase but cannot
write or modify anything.
"""

import subprocess
from pathlib import Path

from . import config


def _validate_path(path: str) -> Path:
    """Resolve and validate a file path is within the project root."""
    resolved = (config.PROJECT_ROOT / path).resolve()
    if not resolved.is_relative_to(config.PROJECT_ROOT):
        raise ValueError(f"Path escapes project root: {path}")
    return resolved


def read_file(path: str) -> str:
    """Read a file from the project directory.

    Args:
        path: Relative path from project root.

    Returns:
        File contents as a string.
    """
    resolved = _validate_path(path)

    if not resolved.exists():
        return f"Error: File not found: {path}"

    if not resolved.is_file():
        return f"Error: Not a file: {path}"

    suffix = resolved.suffix.lower()
    # Allow extensionless files like Makefile, Dockerfile
    if suffix and suffix not in config.ALLOWED_EXTENSIONS:
        return f"Error: File type not allowed: {suffix}"

    if resolved.stat().st_size > config.MAX_FILE_SIZE_BYTES:
        return f"Error: File too large (>{config.MAX_FILE_SIZE_BYTES} bytes): {path}"

    return resolved.read_text(encoding="utf-8", errors="replace")


def grep_codebase(pattern: str, file_glob: str = "") -> str:
    """Search the codebase using grep.

    Args:
        pattern: Regex pattern to search for.
        file_glob: Optional glob to filter files (e.g. '*.rs', '*.svelte').

    Returns:
        Matching lines with file paths and line numbers.
    """
    cmd = [
        "grep",
        "-rn",
        "--include",
        file_glob if file_glob else "*",
        "-E",
        pattern,
        str(config.PROJECT_ROOT),
    ]

    # Exclude common non-source directories
    for exclude in [
        "node_modules",
        "target",
        ".git",
        "__pycache__",
        ".svelte-kit",
        "build",
    ]:
        cmd.insert(3, f"--exclude-dir={exclude}")

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except subprocess.TimeoutExpired:
        return "Error: Search timed out after 10 seconds."

    output = result.stdout.strip()
    if not output:
        return f"No matches found for pattern: {pattern}"

    # Limit results
    lines = output.split("\n")
    if len(lines) > config.MAX_GREP_RESULTS:
        lines = lines[: config.MAX_GREP_RESULTS]
        lines.append(
            f"... (truncated, showing first {config.MAX_GREP_RESULTS} results)"
        )

    # Make paths relative to project root
    project_str = str(config.PROJECT_ROOT) + "/"
    return "\n".join(line.replace(project_str, "") for line in lines)


# Tool definitions for the Anthropic API tool-use protocol
TOOL_DEFINITIONS = [
    {
        "name": "read_file",
        "description": (
            "Read a file from the Jura Archive project. "
            "Path is relative to the project root. "
            "Use this to examine source code, configuration, documentation, or schema files."
        ),
        "input_schema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path from project root (e.g. 'src-tauri/src/db.rs')",
                },
            },
            "required": ["path"],
        },
    },
    {
        "name": "grep_codebase",
        "description": (
            "Search the Jura Archive codebase using regex pattern matching. "
            "Returns matching lines with file paths and line numbers. "
            "Use this to find function definitions, usage patterns, imports, or configuration."
        ),
        "input_schema": {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression pattern to search for",
                },
                "file_glob": {
                    "type": "string",
                    "description": "Optional glob to filter files (e.g. '*.rs', '*.svelte', '*.ts')",
                },
            },
            "required": ["pattern"],
        },
    },
]


def execute_tool(name: str, arguments: dict) -> str:
    """Execute a tool by name with the given arguments."""
    if name == "read_file":
        return read_file(arguments["path"])
    elif name == "grep_codebase":
        return grep_codebase(arguments["pattern"], arguments.get("file_glob", ""))
    else:
        return f"Error: Unknown tool: {name}"
