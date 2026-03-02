"""Project context loader with intelligent section pruning.

Each agent declares which project files it needs. This module loads
and optionally prunes large files to control token usage.
"""

import re
from pathlib import Path

from . import config


def _extract_sections(content: str, headings: list[str]) -> str:
    """Extract specific markdown sections from a document.

    Args:
        content: Full markdown document.
        headings: List of heading texts to extract (case-insensitive partial match).

    Returns:
        Concatenated matching sections.
    """
    if not headings:
        return content

    sections = []
    lines = content.split("\n")
    capturing = False
    capture_level = 0
    current_section: list[str] = []

    for line in lines:
        heading_match = re.match(r"^(#{1,6})\s+(.+)", line)

        if heading_match:
            level = len(heading_match.group(1))
            title = heading_match.group(2).strip()

            # If we were capturing, check if this heading ends the section
            if capturing and level <= capture_level:
                sections.append("\n".join(current_section))
                current_section = []
                capturing = False

            # Check if this heading matches any requested heading
            title_lower = title.lower()
            for h in headings:
                if h.lower() in title_lower:
                    capturing = True
                    capture_level = level
                    current_section = [line]
                    break
        elif capturing:
            current_section.append(line)

    # Flush last section
    if capturing and current_section:
        sections.append("\n".join(current_section))

    return "\n\n---\n\n".join(sections) if sections else content[:2000]


def load_file(path: Path, sections: list[str] | None = None) -> str | None:
    """Load a project file, optionally extracting specific sections.

    Args:
        path: Absolute path to the file.
        sections: Optional list of heading texts to extract from markdown files.

    Returns:
        File contents (possibly pruned), or None if the file doesn't exist.
    """
    if not path.exists():
        return None

    try:
        content = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None

    # Prune markdown files if sections specified
    if sections and path.suffix.lower() == ".md":
        content = _extract_sections(content, sections)

    # Truncate very large files
    if len(content) > config.MAX_CONTEXT_TOKENS * 4:  # rough char-to-token ratio
        content = content[: config.MAX_CONTEXT_TOKENS * 4]
        content += "\n\n[... truncated for context window ...]"

    return content


def build_context(
    file_keys: list[str],
    section_map: dict[str, list[str]] | None = None,
) -> str:
    """Build a context string from project files.

    Args:
        file_keys: Keys from config.PROJECT_FILES to load.
        section_map: Optional mapping of file_key -> list of section headings to extract.

    Returns:
        Formatted context string with file contents.
    """
    section_map = section_map or {}
    parts = []

    for key in file_keys:
        path = config.PROJECT_FILES.get(key)
        if path is None:
            continue

        sections = section_map.get(key)
        content = load_file(path, sections)

        if content is None:
            continue

        rel_path = path.relative_to(config.PROJECT_ROOT)
        parts.append(f"### File: {rel_path}\n\n```\n{content}\n```")

    if not parts:
        return "No project context files were loaded."

    return "\n\n---\n\n".join(parts)
