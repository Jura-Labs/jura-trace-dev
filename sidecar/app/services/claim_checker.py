"""
Jura Archive Sidecar — RAG Claim Verification Service.

Verifies claims associated with an image (from captions, EXIF descriptions,
C2PA assertions, or user-provided text) using a local TF-IDF knowledge base
for retrieval, augmented by a local Ollama LLM for reasoning.

No external API calls are made — all retrieval and inference is performed
locally.  If Ollama is unavailable the service degrades gracefully and
returns an "unavailable" verdict.

Algorithm:
1. Split claims_text into individual claims (by sentence / newline).
2. For each claim, retrieve the most relevant passages from the knowledge
   base using TF-IDF cosine similarity (KnowledgeRetriever).
3. Build a grounded prompt that includes the retrieved passages as reference
   material, instructing the LLM to reason from the provided text rather
   than from parametric recall.
4. Send the grounded prompt to Ollama at temperature 0.1.
5. Parse the verdict token (SUPPORTED / DISPUTED / UNVERIFIED) from the
   response, together with a brief explanation.
6. Aggregate individual verdicts into an overall verdict:
     - All supported               → supported
     - Any disputed                → disputed
     - Mix of supported/unverified → mixed
     - All unverified              → unverified
     - Ollama unavailable          → unavailable
7. Return a ClaimCheckResponse with per-claim verdicts, an aggregated overall
   verdict, methodology disclosure (including knowledge base stats), and a
   human-readable summary.
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

_VALID_VERDICTS = {"supported", "disputed", "unverified"}

# Grounded prompt: LLM is instructed to use only the retrieved reference
# material, which limits hallucination to the scope of the knowledge base.
_PROMPT_TEMPLATE_RAG = textwrap.dedent("""\
    You are a fact-checking assistant. Use ONLY the reference material below to \
evaluate the claim. If the reference material does not contain relevant \
information, respond with UNVERIFIED. Respond with ONLY one of: SUPPORTED, \
DISPUTED, UNVERIFIED, followed by a brief explanation (1-2 sentences). Do not \
add any other commentary.

    Reference material:
    {retrieved_passages}

    Claim: {claim}
    Context: {context}

    Verdict:\
""")

# Fallback prompt used when no knowledge base passages are available for a
# claim.  Explicitly warns the model (and downstream readers) that the result
# depends on parametric recall.
_PROMPT_TEMPLATE_FALLBACK = textwrap.dedent("""\
    You are a fact-checking assistant. No reference material is available for \
this claim — your response will be based on your training data only, which may \
be incomplete or inaccurate. Respond with ONLY one of: SUPPORTED, DISPUTED, \
UNVERIFIED, followed by a brief explanation (1-2 sentences). Do not add any \
other commentary.

    Claim: {claim}
    Context: {context}

    Verdict:\
""")

# Methodology templates — filled in dynamically with knowledge base stats.
_METHODOLOGY_RAG_TEMPLATE = (
    "Claims verified against local knowledge base ({n_docs} documents, "
    "{n_passages} passages) using TF-IDF retrieval + Ollama {model}. "
    "Retrieved passages grounded the verification — the LLM was instructed to "
    "reason from provided reference material rather than parametric recall. "
    "Temperature was set to 0.1 for deterministic output. "
    "This is AI-assisted analysis — results should be interpreted as indicative, "
    "not conclusive."
)

_METHODOLOGY_NO_KB = (
    "WARNING: No knowledge base available. Verification relies on model training "
    "data only (parametric recall). Results may be unreliable. "
    "Each claim was evaluated using a local Ollama LLM at temperature 0.1. "
    "This is AI-assisted analysis — results should be interpreted as indicative, "
    "not conclusive."
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
    Extract a (verdict, explanation, confidence) triple from Ollama's raw output.

    The model is prompted to start with SUPPORTED / DISPUTED / UNVERIFIED.
    We search for one of these tokens (case-insensitive) and treat everything
    after it as the explanation.

    Confidence heuristic:
      - SUPPORTED or DISPUTED with explanation ≥ 20 chars: 0.75
      - SUPPORTED or DISPUTED with short explanation:      0.55
      - UNVERIFIED with any explanation:                   0.40
      - Token not found (parse failure):                   0.20 (→ unverified)

    Args:
        raw: The raw text response from Ollama.

    Returns:
        Tuple of (verdict_str, explanation_str, confidence_float).
    """
    # Strip markdown fence delimiters (``` lines) but keep the content inside.
    # Some models wrap their response in a code block; we want the text within.
    cleaned = re.sub(r"^```[^\n]*\n?", "", raw.strip())  # opening fence
    cleaned = re.sub(r"\n?```$", "", cleaned)             # closing fence
    cleaned = re.sub(r"^[:\s]+", "", cleaned).strip()

    pattern = re.compile(
        r"\b(SUPPORTED|DISPUTED|UNVERIFIED)\b(.*)$",
        re.IGNORECASE | re.DOTALL,
    )
    match = pattern.search(cleaned)
    if not match:
        return "unverified", cleaned[:200] if cleaned else "Could not parse model response.", 0.20

    verdict = match.group(1).lower()
    explanation = match.group(2).strip().lstrip(":.–—-").strip()

    # Trim explanation to a reasonable length
    if len(explanation) > 300:
        explanation = explanation[:297] + "..."

    if not explanation:
        explanation = f"Model returned verdict: {verdict.upper()} without further explanation."

    if verdict in ("supported", "disputed"):
        confidence = 0.75 if len(explanation) >= 20 else 0.55
    else:
        confidence = 0.40

    return verdict, explanation, confidence


# ── Verdict aggregation ───────────────────────────────────────────────────────


def _aggregate_verdicts(verdicts: list[str]) -> str:
    """
    Combine per-claim verdicts into a single overall verdict.

    Rules (evaluated in priority order):
      - Any "unavailable"       → "unavailable"
      - Any "disputed"          → "disputed"
      - Mix of supported + unverified → "mixed"
      - All "supported"         → "supported"
      - All "unverified"        → "unverified"
      - Empty list              → "unverified"

    Args:
        verdicts: List of individual verdict strings.

    Returns:
        A single verdict string.
    """
    if not verdicts:
        return "unverified"

    verdict_set = set(verdicts)

    if "unavailable" in verdict_set:
        return "unavailable"
    if "disputed" in verdict_set:
        return "disputed"
    if "supported" in verdict_set and "unverified" in verdict_set:
        return "mixed"
    if verdict_set == {"supported"}:
        return "supported"
    return "unverified"


def _build_summary(overall: str, claims: list[ClaimVerdict]) -> str:
    """Build a human-readable summary string for the ClaimCheckResponse."""
    n = len(claims)
    if overall == "unavailable":
        return (
            "Claim verification could not be completed because the Ollama service "
            "is unavailable. Install and start Ollama to enable AI-assisted claim checking."
        )
    counts: dict[str, int] = {}
    for c in claims:
        counts[c.verdict] = counts.get(c.verdict, 0) + 1

    parts = []
    for v in ("supported", "disputed", "unverified"):
        if counts.get(v, 0):
            parts.append(f"{counts[v]} {v}")

    count_str = ", ".join(parts) if parts else "0 claims"
    verdict_labels = {
        "supported": "All claims are supported by available knowledge.",
        "disputed": "One or more claims are disputed.",
        "mixed": "Claims show mixed support — some supported, some unverifiable.",
        "unverified": "Insufficient information to verify the claims.",
    }
    base = verdict_labels.get(overall, f"Overall verdict: {overall}.")
    return f"Analysed {n} claim{'s' if n != 1 else ''}. {count_str}. {base}"


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
            overall_verdict="unverified",
            claims=[],
            model_used=model,
            methodology=_build_methodology(model),
            summary="No claims were provided for verification.",
        )

    # ── Check Ollama availability ─────────────────────────────────────────────
    ollama_available = await _check_ollama_available(ollama_base_url)
    if not ollama_available:
        unavailable_claims = [
            ClaimVerdict(
                claim=c,
                verdict="unavailable",
                explanation=(
                    "Ollama is not available. Start the Ollama service to enable "
                    "AI-assisted claim verification."
                ),
                confidence=0.0,
            )
            for c in claims
        ]
        return ClaimCheckResponse(
            overall_verdict="unavailable",
            claims=unavailable_claims,
            model_used=model,
            methodology=_build_methodology(model),
            summary=_build_summary("unavailable", unavailable_claims),
        )

    # ── Check model availability ──────────────────────────────────────────────
    model_available = await _check_model_available(ollama_base_url, model)
    if not model_available:
        unavailable_claims = [
            ClaimVerdict(
                claim=c,
                verdict="unavailable",
                explanation=(
                    f"Model '{model}' is not available in Ollama. "
                    f"Pull it with: ollama pull {model}"
                ),
                confidence=0.0,
            )
            for c in claims
        ]
        return ClaimCheckResponse(
            overall_verdict="unavailable",
            claims=unavailable_claims,
            model_used=model,
            methodology=_build_methodology(model),
            summary=_build_summary("unavailable", unavailable_claims),
        )

    # ── Verify each claim ─────────────────────────────────────────────────────
    verified: list[ClaimVerdict] = []
    for claim in claims:
        # Retrieve relevant passages from the knowledge base for this claim.
        # The retrieval query combines the claim with any caller-provided context
        # so that forensic metadata (EXIF, C2PA) influences passage selection.
        retrieval_query = f"{claim} {context}".strip() if context else claim
        passages = _retriever.retrieve(retrieval_query, top_k=3)

        if passages:
            retrieved_text = "\n\n".join(passages)
            prompt = _PROMPT_TEMPLATE_RAG.format(
                retrieved_passages=retrieved_text,
                claim=claim,
                context=context or "None provided.",
            )
        else:
            # No passages retrieved — fall back to parametric recall with warning.
            logger.debug(
                "No knowledge base passages retrieved for claim: %.80s", claim
            )
            prompt = _PROMPT_TEMPLATE_FALLBACK.format(
                claim=claim,
                context=context or "None provided.",
            )

        try:
            raw = await _call_ollama(prompt, ollama_base_url, model)
            verdict, explanation, confidence = _parse_verdict(raw)
        except httpx.TimeoutException:
            logger.warning("Ollama timed out while verifying claim: %.80s", claim)
            verdict, explanation, confidence = (
                "unverified",
                "Ollama request timed out. The model may be loading — please retry.",
                0.0,
            )
        except Exception as exc:  # noqa: BLE001
            logger.warning("Ollama error while verifying claim: %s", exc)
            verdict, explanation, confidence = (
                "unverified",
                f"Verification failed: {exc}",
                0.0,
            )

        verified.append(
            ClaimVerdict(
                claim=claim,
                verdict=verdict,
                explanation=explanation,
                confidence=round(confidence, 4),
            )
        )

    overall = _aggregate_verdicts([v.verdict for v in verified])
    return ClaimCheckResponse(
        overall_verdict=overall,
        claims=verified,
        model_used=model,
        methodology=_build_methodology(model),
        summary=_build_summary(overall, verified),
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
