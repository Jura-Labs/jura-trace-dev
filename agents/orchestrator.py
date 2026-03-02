"""Orchestrator: routes queries to specialist agents.

Two-stage routing:
1. Fast pattern match — keyword/regex against agent domains (no API call)
2. LLM classification — when ambiguous, uses claude-haiku to classify
"""

import json
import re

import anthropic

from . import config
from .base_agent import BaseAgent

# Keyword patterns mapped to agent names (checked in order)
KEYWORD_PATTERNS: list[tuple[str, list[str]]] = [
    (
        "database",
        [
            r"\bsqlite\b",
            r"\bschema\b",
            r"\bmigrat",
            r"\brusqlite\b",
            r"\bquery\b",
            r"\bindex(?:es)?\b",
            r"\btable\b",
            r"\bjoin\b",
            r"\bforeign.key\b",
            r"\bhamming\b",
        ],
    ),
    (
        "ux_frontend",
        [
            r"\bsveltekit\b",
            r"\bsvelte\b",
            r"\btailwind\b",
            r"\bcss\b",
            r"\bui\b",
            r"\bux\b",
            r"\bfrontend\b",
            r"\bfront.end\b",
            r"\bcomponent\b",
            r"\bwcag\b",
            r"\baccessib",
            r"\bdark.mode\b",
            r"\blayout\b",
            r"\bdesign\b",
            r"\bbrand\b",
            r"\bnavigat",
        ],
    ),
    (
        "legislation",
        [
            r"\bgdpr\b",
            r"\beu.ai.act\b",
            r"\bcopyright\b",
            r"\blicen[cs]",
            r"\bcomplian",
            r"\bregulat",
            r"\blegal\b",
            r"\bdata.protection\b",
            r"\bprivacy\b",
            r"\bright",
            r"\blaw\b",
            r"\blegislat",
        ],
    ),
    (
        "backend",
        [
            r"\brust\b",
            r"\btauri\b",
            r"\bc2pa\b",
            r"\bipc\b",
            r"\bsidecar\b",
            r"\bhash(?:ing)?\b",
            r"\bwatermark",
            r"\bformat.router\b",
            r"\bcargo\b",
            r"\bcrate\b",
            r"\bcommand\b.*tauri",
            r"\bfingerprint",
        ],
    ),
    (
        "devops",
        [
            r"\bci/?cd\b",
            r"\bdocker\b",
            r"\bgithub.action",
            r"\bpipeline\b",
            r"\bbuild\b.*\b(?:cross|platform|release)\b",
            r"\bollama\b.*deploy",
            r"\binstaller\b",
            r"\bdmg\b",
            r"\bmsi\b",
            r"\bappimage\b",
        ],
    ),
    (
        "qa_tester",
        [
            r"\btest(?:ing|s)?\b",
            r"\bcargo.test\b",
            r"\bvitest\b",
            r"\bpytest\b",
            r"\be2e\b",
            r"\bplaywright\b",
            r"\bcoverage\b",
            r"\bqa\b",
            r"\bquality\b",
        ],
    ),
    (
        "end_user",
        [
            r"\bpersona\b",
            r"\buser.stor",
            r"\bcurator\b",
            r"\bjournalist\b",
            r"\bfact.check",
            r"\bit.manager\b",
            r"\busability\b",
            r"\bworkflow\b.*user",
            r"\bonboarding\b",
        ],
    ),
    (
        "knowledge",
        [
            r"\bdeepfake\b.*(?:method|technique|detect)",
            r"\bperceptual.hash",
            r"\bela\b.*(?:analys|forensic)",
            r"\bai.algorithm\b",
            r"\bc2pa.ecosystem\b",
            r"\bfake.news\b",
            r"\bmisinfo",
            r"\bip.law\b",
            r"\bintellectual.propert",
        ],
    ),
    (
        "project_manager",
        [
            r"\bsprint\b",
            r"\bphase\b.*\d",
            r"\bkpi\b",
            r"\bmilestone\b",
            r"\bscope\b",
            r"\bpriori",
            r"\bbacklog\b",
            r"\bschedul",
            r"\bdependenc",
            r"\bdeliverable\b",
            r"\bweek\s*\d",
        ],
    ),
    (
        "security",
        [
            r"\bthreat\b",
            r"\bcsp\b",
            r"\baudit.trail\b",
            r"\bpermission\b",
            r"\bsupply.chain\b",
            r"\bvulnerab",
            r"\bsandbox\b",
            r"\btauri.*permission",
            r"\bsecur",
        ],
    ),
    (
        "documentation",
        [
            r"\bdoc(?:umentation|s)?\b",
            r"\bguide\b",
            r"\breadme\b",
            r"\bapi.doc",
            r"\bcontribut",
            r"\bchangelog\b",
            r"\bbrand.voice\b",
            r"\bwriting\b",
        ],
    ),
]


def _pattern_match(query: str) -> dict[str, int]:
    """Score each agent by keyword pattern matches.

    Returns:
        Dict of agent_name -> match count, only for agents with matches.
    """
    query_lower = query.lower()
    scores: dict[str, int] = {}

    for agent_name, patterns in KEYWORD_PATTERNS:
        count = sum(1 for p in patterns if re.search(p, query_lower))
        if count > 0:
            scores[agent_name] = count

    return scores


def _llm_classify(query: str) -> dict:
    """Use Claude Haiku to classify the query when pattern matching is ambiguous.

    Returns:
        Dict with keys: primary, supporting, reasoning
    """
    agent_descriptions = "\n".join(
        f"- {name}: {patterns_desc}"
        for name, patterns_desc in [
            (
                "database",
                "SQLite schema, query optimisation, migration strategy, rusqlite patterns",
            ),
            (
                "ux_frontend",
                "SvelteKit/Tailwind, WCAG 2.2 AA, dark mode, geology brand identity",
            ),
            (
                "legislation",
                "EU AI Act, GDPR, UK Data Protection, copyright, C2PA standards",
            ),
            (
                "backend",
                "Rust/Tauri v2, C2PA, hashing, watermarking, IPC, sidecar communication",
            ),
            ("devops", "CI/CD, Docker, cross-platform builds, Ollama deployment"),
            (
                "qa_tester",
                "Test strategies, E2E, accessibility testing, security testing",
            ),
            ("end_user", "User personas, user stories, usability review"),
            (
                "knowledge",
                "IP law, fake news detection, AI algorithms, C2PA ecosystem, deepfake methods",
            ),
            (
                "project_manager",
                "Sprint planning, phase tracking, scope tradeoffs, KPI monitoring",
            ),
            (
                "security",
                "Threat modelling, CSP hardening, audit trail, Tauri permissions",
            ),
            ("documentation", "User guides, API docs, contributor guides, brand voice"),
        ]
    )

    client = anthropic.Anthropic(api_key=config.ANTHROPIC_API_KEY)

    response = client.messages.create(
        model=config.ORCHESTRATOR_MODEL,
        max_tokens=256,
        system=(
            "You are a query router for the Jura Archive project. "
            "Classify the user's query to the most relevant specialist agent. "
            "Return JSON only, no other text.\n\n"
            f"Available agents:\n{agent_descriptions}"
        ),
        messages=[
            {
                "role": "user",
                "content": (
                    f"Query: {query}\n\n"
                    'Return JSON: {"primary": "agent_name", '
                    '"supporting": ["agent_name", ...], '
                    '"reasoning": "brief explanation"}\n\n'
                    "supporting should have 0-2 agents that could add value. "
                    "Only include supporting agents if genuinely relevant."
                ),
            }
        ],
    )

    text = response.content[0].text.strip()

    # Extract JSON from response (handle markdown code blocks)
    json_match = re.search(r"\{.*\}", text, re.DOTALL)
    if json_match:
        try:
            return json.loads(json_match.group())
        except json.JSONDecodeError:
            pass

    return {
        "primary": "backend",
        "supporting": [],
        "reasoning": "Fallback: could not parse routing",
    }


def route(query: str) -> dict:
    """Route a query to the appropriate agent(s).

    Returns:
        Dict with keys: primary, supporting, reasoning
    """
    scores = _pattern_match(query)

    if not scores:
        # No pattern matches — use LLM classification
        return _llm_classify(query)

    sorted_agents = sorted(scores.items(), key=lambda x: x[1], reverse=True)
    primary = sorted_agents[0][0]
    primary_score = sorted_agents[0][1]

    # If the top match is strong (3+ hits) and well-separated, use it directly
    if primary_score >= 3 and (
        len(sorted_agents) < 2 or sorted_agents[1][1] < primary_score - 1
    ):
        supporting = [a for a, s in sorted_agents[1:3] if s >= 2]
        return {
            "primary": primary,
            "supporting": supporting,
            "reasoning": f"Pattern match: {primary} ({primary_score} hits)",
        }

    # If ambiguous (close scores or low confidence), use LLM
    if len(sorted_agents) >= 2 and sorted_agents[1][1] >= primary_score - 1:
        return _llm_classify(query)

    # Single weak match — use it but no supporting agents
    return {
        "primary": primary,
        "supporting": [],
        "reasoning": f"Pattern match: {primary} ({primary_score} hits)",
    }


def dispatch(
    query: str, agents: dict[str, BaseAgent], routing: dict | None = None
) -> list[tuple[str, str]]:
    """Dispatch a query to routed agents and collect responses.

    Args:
        query: The user's question.
        agents: Dict of agent_name -> instantiated agent.
        routing: Pre-computed routing dict (if None, will route automatically).

    Returns:
        List of (agent_name, response) tuples.
    """
    if routing is None:
        routing = route(query)

    primary_name = routing["primary"]
    supporting_names = routing.get("supporting", [])

    responses = []

    # Primary agent
    if primary_name in agents:
        primary_response = agents[primary_name].query(query)
        responses.append((primary_name, primary_response))
    else:
        responses.append((primary_name, f"Error: Agent '{primary_name}' not found."))
        return responses

    # Supporting agents get the primary response as context
    for name in supporting_names:
        if name in agents and name != primary_name:
            supporting_response = agents[name].query(
                query, supporting_context=primary_response
            )
            responses.append((name, supporting_response))

    return responses
