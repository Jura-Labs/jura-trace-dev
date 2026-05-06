# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Jura Trace Sidecar — Knowledge Base Retrieval (investigative aid).

This module is NOT a fact-checker and does NOT produce verdicts about the
truth or falsity of any claim. It matches analyst-entered text against a
preliminary local reference corpus and returns one of three statuses:

- ``consistent_with_kb``          — retrieved passages are consistent with the claim
- ``inconsistent_with_kb``        — retrieved passages contradict the claim
- ``insufficient_context_in_kb``  — the corpus does not cover the claim

See the in-app model card at /help/model-cards (#kb-retrieval) for the
full scope, limitations, non-warranty notice, and out-of-scope use list.

No external API calls are made — all retrieval and inference is performed
locally.  If Ollama is unavailable the service degrades gracefully and
returns an "unavailable" status.

Algorithm:
1. Split claims_text into individual claims (by sentence / newline).
2. For each claim, retrieve the most relevant passages from the knowledge
   base using TF-IDF cosine similarity (KnowledgeRetriever).
3. **Fail-closed:** if no relevant passages are retrieved, return
   ``insufficient_context_in_kb`` immediately without invoking the language
   model. This is deliberate — it prevents the model from reasoning from
   parametric memory about claims the corpus cannot support. The parametric
   fallback path was removed in April 2026 following a defamation-exposure
   review (see .claude/projects/.../memory/project_tech_debt_audit_apr2026.md).
4. Otherwise, build a grounded prompt that includes the retrieved passages as
   reference material, instructing the LLM to assess consistency with the
   provided reference text only.
5. Send the grounded prompt to Ollama at temperature 0.1.
6. Parse the status token (CONSISTENT / INCONSISTENT / INSUFFICIENT) from the
   response, together with a brief explanation.
7. Aggregate individual statuses into an overall status:
     - All consistent_with_kb                      → consistent_with_kb
     - Any inconsistent_with_kb                    → inconsistent_with_kb
     - Mix of consistent_with_kb / insufficient    → mixed_kb_match
     - All insufficient_context_in_kb              → insufficient_context_in_kb
     - Ollama unavailable                          → unavailable
8. Return a ClaimCheckResponse with per-claim statuses, aggregated overall
   status, methodology disclosure, and a human-readable summary that makes
   clear the output is a retrieval match, not a factual verdict.
"""

from __future__ import annotations

import logging
import re
import textwrap

import httpx

from app.models.schemas import ClaimCheckResponse, ClaimVerdict
from app.services.knowledge_retriever import KnowledgeRetriever

# Module-level retriever instance — shared across all requests, built once.
_retriever = KnowledgeRetriever()

logger = logging.getLogger(__name__)

# ── Constants ─────────────────────────────────────────────────────────────────

# Status vocabulary. Note the deliberate absence of "verdict" — this tool
# does not emit verdicts about the truth or falsity of any claim. It reports
# whether analyst-entered text is consistent with the retrieved reference
# passages, or whether the preliminary corpus lacks coverage of the subject.
CONSISTENT = "consistent_with_kb"
INCONSISTENT = "inconsistent_with_kb"
INSUFFICIENT = "insufficient_context_in_kb"
UNAVAILABLE = "unavailable"
MIXED = "mixed_kb_match"

_VALID_STATUSES = {CONSISTENT, INCONSISTENT, INSUFFICIENT}

# Grounded prompt. The LLM is explicitly instructed NOT to act as a
# fact-checker and NOT to reason from its own training data — it assesses
# consistency with the provided reference passages only. There is no
# parametric-memory fallback: if no passages are retrieved, the caller
# returns INSUFFICIENT without invoking the model.
_PROMPT_TEMPLATE_RAG = textwrap.dedent("""\
    You are a reference-retrieval assessment tool. You are NOT a fact-checker \
and MUST NOT use any information outside the reference material below. Your \
task is to report whether the reference material is consistent with, \
inconsistent with, or insufficient to assess the claim.

    Respond with ONLY one of: CONSISTENT, INCONSISTENT, INSUFFICIENT, followed \
by a brief explanation (1-2 sentences) that cites the reference material. Do \
NOT add commentary about the real-world truth of the claim — only about \
consistency with the provided reference passages. If the reference material \
does not contain information relevant to the claim, respond INSUFFICIENT.

    Reference material:
    {retrieved_passages}

    Claim: {claim}
    Context: {context}

    Response:\
""")

# Methodology template — filled in dynamically with knowledge base stats.
# Framed as a retrieval match, not a fact-check.
_METHODOLOGY_RAG_TEMPLATE = (
    "Knowledge base retrieval match against local preliminary corpus "
    "({n_docs} documents, {n_passages} passages) using TF-IDF + local "
    "Ollama {model} (temperature 0.1). The language model was instructed to "
    "assess consistency with the retrieved reference passages only. This is "
    "a preliminary investigative aid, not a fact-checker — see the model "
    "card (/help/model-cards#kb-retrieval) for scope, limitations, and "
    "non-warranty notice. Results must not be cited as authority for the "
    "truth or falsity of any claim."
)

_METHODOLOGY_NO_KB = (
    "Knowledge base unavailable — no passages could be retrieved. No claim "
    "assessment was performed. This tool never reasons from parametric "
    "model memory about claims the corpus cannot support. See the model "
    "card (/help/model-cards#kb-retrieval) for scope and limitations."
)


def _build_methodology(model: str) -> str:
    """
    Build the methodology string based on knowledge base availability.

    Args:
        model: The Ollama model name used for inference.

    Returns:
        A methodology disclosure string for inclusion in ClaimCheckResponse.
    """
    if _retriever.is_available():
        return _METHODOLOGY_RAG_TEMPLATE.format(
            n_docs=_retriever.document_count,
            n_passages=_retriever.passage_count,
            model=model,
        )
    return _METHODOLOGY_NO_KB

# ── Claim splitting ───────────────────────────────────────────────────────────


def split_claims(text: str) -> list[str]:
    """
    Split a block of text into individual claim strings.

    Strategy (in order of priority):
    1. Split on newlines — each non-empty line is treated as a distinct claim.
    2. If only a single line was found, split further on sentence boundaries
       (terminal punctuation followed by whitespace).

    Each resulting fragment is stripped and must be at least 10 characters long
    to be included (filters out stray punctuation / whitespace artefacts).

    Args:
        text: Raw claims text from the caller.

    Returns:
        List of individual claim strings, deduplicated and ordered.
    """
    if not text or not text.strip():
        return []

    # Prefer newline splitting — callers often supply one claim per line.
    lines = [ln.strip() for ln in text.splitlines() if ln.strip()]
    if len(lines) > 1:
        return [ln for ln in lines if len(ln) >= 10]

    # Single line: split on sentence boundaries.
    sentences = re.split(r"(?<=[.!?])\s+", text.strip())
    result = [s.strip() for s in sentences if len(s.strip()) >= 10]
    return result if result else [text.strip()]


# ── Ollama call ───────────────────────────────────────────────────────────────


async def _call_ollama(
    prompt: str,
    ollama_base_url: str,
    model: str,
    timeout: float = 30.0,
) -> str:
    """
    Send a generation request to Ollama and return the raw response text.

    Uses the /api/generate endpoint with stream=False so the full response
    arrives in a single JSON object.

    Args:
        prompt:          The complete prompt string.
        ollama_base_url: Base URL of the Ollama instance (no trailing slash).
        model:           Model name, e.g. "qwen2.5:7b-instruct".
        timeout:         HTTP timeout in seconds.

    Returns:
        The model's response string, stripped of leading/trailing whitespace.

    Raises:
        httpx.HTTPError:    On network or HTTP-level failure.
        httpx.TimeoutException: If the request exceeds ``timeout``.
        KeyError:           If the response JSON is missing the "response" key.
    """
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "options": {
            "temperature": 0.1,
            "num_predict": 150,
        },
    }
    async with httpx.AsyncClient(timeout=timeout) as client:
        resp = await client.post(
            f"{ollama_base_url}/api/generate",
            json=payload,
        )
        resp.raise_for_status()
        data = resp.json()
        return data["response"].strip()


# ── Response parsing ──────────────────────────────────────────────────────────


def _parse_verdict(raw: str) -> tuple[str, str, float]:
    """
    Extract a (status, explanation, confidence) triple from Ollama's raw output.

    The model is prompted to start with CONSISTENT / INCONSISTENT / INSUFFICIENT.
    We search for one of these tokens (case-insensitive) and treat everything
    after it as the explanation. Legacy verdict tokens
    (SUPPORTED / DISPUTED / UNVERIFIED) are accepted for backwards compatibility
    with users running older locally-cached prompts or intermediate builds, and
    mapped onto the new vocabulary.

    Confidence heuristic (reflects retrieval-match certainty, NOT factual truth):
      - CONSISTENT or INCONSISTENT with explanation ≥ 20 chars: 0.75
      - CONSISTENT or INCONSISTENT with short explanation:      0.55
      - INSUFFICIENT with any explanation:                      0.40
      - Token not found (parse failure):                        0.20 (→ INSUFFICIENT)

    Args:
        raw: The raw text response from Ollama.

    Returns:
        Tuple of (status_str, explanation_str, confidence_float) where
        status_str is one of the new-vocabulary constants.
    """
    # Strip markdown fence delimiters (``` lines) but keep the content inside.
    # Some models wrap their response in a code block; we want the text within.
    cleaned = re.sub(r"^```[^\n]*\n?", "", raw.strip())  # opening fence
    cleaned = re.sub(r"\n?```$", "", cleaned)             # closing fence
    cleaned = re.sub(r"^[:\s]+", "", cleaned).strip()

    # Accept both new and legacy tokens. New vocabulary first so it wins on
    # ambiguous responses where the model prints both.
    pattern = re.compile(
        r"\b(CONSISTENT|INCONSISTENT|INSUFFICIENT|SUPPORTED|DISPUTED|UNVERIFIED)\b(.*)$",
        re.IGNORECASE | re.DOTALL,
    )
    match = pattern.search(cleaned)
    if not match:
        return (
            INSUFFICIENT,
            cleaned[:200] if cleaned else "Could not parse model response.",
            0.20,
        )

    raw_token = match.group(1).upper()
    # Map legacy tokens onto the new vocabulary.
    token_map = {
        "CONSISTENT": CONSISTENT,
        "SUPPORTED": CONSISTENT,
        "INCONSISTENT": INCONSISTENT,
        "DISPUTED": INCONSISTENT,
        "INSUFFICIENT": INSUFFICIENT,
        "UNVERIFIED": INSUFFICIENT,
    }
    status = token_map[raw_token]
    explanation = match.group(2).strip().lstrip(":.–—-").strip()

    # Trim explanation to a reasonable length
    if len(explanation) > 300:
        explanation = explanation[:297] + "..."

    if not explanation:
        explanation = (
            f"Model returned {raw_token} without further explanation."
        )

    if status in (CONSISTENT, INCONSISTENT):
        confidence = 0.75 if len(explanation) >= 20 else 0.55
    else:
        confidence = 0.40

    return status, explanation, confidence


# ── Verdict aggregation ───────────────────────────────────────────────────────


def _aggregate_verdicts(statuses: list[str]) -> str:
    """
    Combine per-claim retrieval-match statuses into a single overall status.

    Rules (evaluated in priority order):
      - Any "unavailable"                            → "unavailable"
      - Any "inconsistent_with_kb"                   → "inconsistent_with_kb"
      - Mix of consistent + insufficient             → "mixed_kb_match"
      - All "consistent_with_kb"                     → "consistent_with_kb"
      - All "insufficient_context_in_kb" (or empty)  → "insufficient_context_in_kb"

    Args:
        statuses: List of per-claim status strings (new vocabulary).

    Returns:
        A single overall status string.
    """
    if not statuses:
        return INSUFFICIENT

    status_set = set(statuses)

    if UNAVAILABLE in status_set:
        return UNAVAILABLE
    if INCONSISTENT in status_set:
        return INCONSISTENT
    if CONSISTENT in status_set and INSUFFICIENT in status_set:
        return MIXED
    if status_set == {CONSISTENT}:
        return CONSISTENT
    return INSUFFICIENT


def _build_summary(overall: str, claims: list[ClaimVerdict]) -> str:
    """Build a human-readable summary string for the ClaimCheckResponse.

    Framing is deliberately retrieval-match, not fact-check. The phrase
    "verdict" does not appear in user-facing output.
    """
    n = len(claims)
    if overall == UNAVAILABLE:
        return (
            "Knowledge base retrieval could not be performed because the Ollama "
            "service is unavailable. Install and start Ollama to enable this "
            "investigative aid. This tool is not a fact-checker — see the "
            "model card for scope and limitations."
        )
    counts: dict[str, int] = {}
    for c in claims:
        counts[c.verdict] = counts.get(c.verdict, 0) + 1

    label_map = {
        CONSISTENT: "consistent with reference material",
        INCONSISTENT: "inconsistent with reference material",
        INSUFFICIENT: "no relevant reference material found",
    }
    parts = []
    for status in (CONSISTENT, INCONSISTENT, INSUFFICIENT):
        if counts.get(status, 0):
            parts.append(f"{counts[status]} {label_map[status]}")

    count_str = "; ".join(parts) if parts else "0 claims"
    overall_labels = {
        CONSISTENT: (
            "All analyst-entered claims are consistent with the local "
            "reference corpus. This is a retrieval match only — it does "
            "NOT establish that the claims are factually true."
        ),
        INCONSISTENT: (
            "One or more claims appear inconsistent with the local reference "
            "corpus. This is a retrieval match only — it does NOT establish "
            "that the claims are factually false."
        ),
        MIXED: (
            "Some claims matched reference material, others did not. The "
            "knowledge base has only partial coverage of the subject."
        ),
        INSUFFICIENT: (
            "The local reference corpus does not cover these claims. No "
            "assessment was performed beyond retrieval."
        ),
    }
    base = overall_labels.get(overall, f"Overall status: {overall}.")
    return (
        f"Analysed {n} claim{'s' if n != 1 else ''}. {count_str}. {base}"
    )


# ── Public API ────────────────────────────────────────────────────────────────


async def check_claims(
    claims_text: str,
    context: str = "",
    ollama_base_url: str = "http://127.0.0.1:11434",
    model: str = "qwen2.5:7b-instruct",
) -> ClaimCheckResponse:
    """
    Verify claims in a block of text using a local Ollama LLM.

    Args:
        claims_text:      Text containing one or more claims to verify.
                          Claims may be separated by newlines or sentence
                          boundaries.
        context:          Optional additional context (e.g. image metadata,
                          EXIF description, C2PA assertions) that may help the
                          model evaluate the claims.
        ollama_base_url:  Base URL of the local Ollama instance.
        model:            Ollama model name to use for inference.

    Returns:
        ClaimCheckResponse with per-claim verdicts, an aggregated overall
        verdict, and methodology disclosure.

    Notes:
        - If Ollama is unreachable, returns a graceful "unavailable" response.
        - If the model is not pulled, returns "unavailable" with guidance.
        - All LLM calls use temperature 0.1 for reproducibility.
        - This function never raises — errors are captured in the response.
    """
    claims = split_claims(claims_text)

    # ── Empty input ───────────────────────────────────────────────────────────
    if not claims:
        return ClaimCheckResponse(
            overall_verdict=INSUFFICIENT,
            claims=[],
            model_used=model,
            methodology=_build_methodology(model),
            summary="No claims were provided for knowledge base retrieval.",
        )

    # ── Check Ollama availability ─────────────────────────────────────────────
    ollama_available = await _check_ollama_available(ollama_base_url)
    if not ollama_available:
        unavailable_claims = [
            ClaimVerdict(
                claim=c,
                verdict=UNAVAILABLE,
                explanation=(
                    "Ollama is not available. Start the Ollama service to enable "
                    "knowledge base retrieval. This tool is an investigative aid, "
                    "not a fact-checker."
                ),
                confidence=0.0,
            )
            for c in claims
        ]
        return ClaimCheckResponse(
            overall_verdict=UNAVAILABLE,
            claims=unavailable_claims,
            model_used=model,
            methodology=_build_methodology(model),
            summary=_build_summary(UNAVAILABLE, unavailable_claims),
        )

    # ── Check model availability ──────────────────────────────────────────────
    model_available = await _check_model_available(ollama_base_url, model)
    if not model_available:
        unavailable_claims = [
            ClaimVerdict(
                claim=c,
                verdict=UNAVAILABLE,
                explanation=(
                    f"Model '{model}' is not available in Ollama. "
                    f"Pull it with: ollama pull {model}"
                ),
                confidence=0.0,
            )
            for c in claims
        ]
        return ClaimCheckResponse(
            overall_verdict=UNAVAILABLE,
            claims=unavailable_claims,
            model_used=model,
            methodology=_build_methodology(model),
            summary=_build_summary(UNAVAILABLE, unavailable_claims),
        )

    # ── Retrieve and assess each claim ───────────────────────────────────────
    assessed: list[ClaimVerdict] = []
    for claim in claims:
        # Retrieve relevant passages from the knowledge base for this claim.
        # The retrieval query combines the claim with any caller-provided context
        # so that forensic metadata (EXIF, C2PA) influences passage selection.
        retrieval_query = f"{claim} {context}".strip() if context else claim
        passages = _retriever.retrieve(retrieval_query, top_k=3)

        # Fail-closed: if no relevant passages were retrieved, return
        # INSUFFICIENT immediately without calling the language model. The
        # parametric-memory fallback path was removed in April 2026 — this
        # tool must never reason from training data about claims the corpus
        # cannot support, because that is the pathway to confidently-wrong
        # outputs and defamation exposure. See the model card at
        # /help/model-cards#kb-retrieval.
        if not passages:
            logger.debug(
                "No knowledge base passages retrieved for claim: %.80s", claim
            )
            assessed.append(
                ClaimVerdict(
                    claim=claim,
                    verdict=INSUFFICIENT,
                    explanation=(
                        "No relevant passages found in the local reference "
                        "corpus. This tool only reports on coverage present "
                        "in its knowledge base — see the model card for the "
                        "current corpus scope."
                    ),
                    confidence=0.40,
                )
            )
            continue

        retrieved_text = "\n\n".join(passages)
        prompt = _PROMPT_TEMPLATE_RAG.format(
            retrieved_passages=retrieved_text,
            claim=claim,
            context=context or "None provided.",
        )

        try:
            raw = await _call_ollama(prompt, ollama_base_url, model)
            status, explanation, confidence = _parse_verdict(raw)
        except httpx.TimeoutException:
            logger.warning(
                "Ollama timed out during retrieval assessment for claim: %.80s",
                claim,
            )
            status, explanation, confidence = (
                INSUFFICIENT,
                "Ollama request timed out. The model may be loading — please retry.",
                0.0,
            )
        except Exception as exc:  # noqa: BLE001
            logger.warning("Ollama error during retrieval assessment: %s", exc)
            status, explanation, confidence = (
                INSUFFICIENT,
                f"Retrieval assessment failed: {exc}",
                0.0,
            )

        assessed.append(
            ClaimVerdict(
                claim=claim,
                verdict=status,
                explanation=explanation,
                confidence=round(confidence, 4),
            )
        )

    overall = _aggregate_verdicts([v.verdict for v in assessed])
    return ClaimCheckResponse(
        overall_verdict=overall,
        claims=assessed,
        model_used=model,
        methodology=_build_methodology(model),
        summary=_build_summary(overall, assessed),
    )


# ── Availability helpers ──────────────────────────────────────────────────────


async def _check_ollama_available(ollama_base_url: str) -> bool:
    """Return True if the Ollama /api/tags endpoint responds within 2 seconds."""
    try:
        async with httpx.AsyncClient(timeout=2.0) as client:
            resp = await client.get(f"{ollama_base_url}/api/tags")
            return resp.status_code == 200
    except Exception:
        return False


async def _check_model_available(ollama_base_url: str, model: str) -> bool:
    """
    Return True if ``model`` appears in Ollama's model list.

    The /api/tags response is a JSON object with a "models" list, each entry
    having a "name" field.  We check for an exact match or a match on the
    base name (without tag suffix) to handle "qwen2.5:latest" vs "qwen2.5".
    """
    try:
        async with httpx.AsyncClient(timeout=2.0) as client:
            resp = await client.get(f"{ollama_base_url}/api/tags")
            if resp.status_code != 200:
                return False
            data = resp.json()
            model_names = {m["name"] for m in data.get("models", [])}
            if model in model_names:
                return True
            # Also match on base name without tag
            base = model.split(":")[0]
            return any(m.split(":")[0] == base for m in model_names)
    except Exception:
        return False
