"""
Tests for the RAG claim verification service.

Ollama is not required — all HTTP calls are mocked via unittest.mock.patch.
The test suite covers:
  - Claim parsing / splitting
  - Verdict aggregation logic
  - Graceful degradation when Ollama is unavailable
  - Graceful degradation when the model is not pulled
  - Response schema validity
  - Positive path (mocked Ollama returning known verdicts)
  - Empty input handling
  - The /forensics/claim-check HTTP endpoint
"""

from __future__ import annotations

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from httpx import ASGITransport, AsyncClient

from app.models.schemas import ClaimCheckResponse, ClaimVerdict
from app.services.claim_checker import (
    _aggregate_verdicts,
    _build_summary,
    _parse_verdict,
    check_claims,
    split_claims,
)
from main import app


# ── Claim splitting ────────────────────────────────────────────────────────────


class TestSplitClaims:
    """Tests for split_claims — the text-to-claim-list parser."""

    def test_empty_string_returns_empty_list(self):
        assert split_claims("") == []

    def test_whitespace_only_returns_empty_list(self):
        assert split_claims("   \n\t  ") == []

    def test_single_line_returned_as_single_claim(self):
        result = split_claims("The photograph was taken in Paris.")
        assert result == ["The photograph was taken in Paris."]

    def test_newline_separated_claims_split_correctly(self):
        text = "The image shows a sunset.\nThe photo was taken in 2022.\nThe location is Scotland."
        result = split_claims(text)
        assert len(result) == 3
        assert "The image shows a sunset." in result
        assert "The photo was taken in 2022." in result
        assert "The location is Scotland." in result

    def test_sentence_boundary_splitting(self):
        text = "The photograph was taken in Rome. The subject is the Colosseum. It dates to 2020."
        result = split_claims(text)
        assert len(result) == 3

    def test_short_fragments_filtered_out(self):
        # Lines shorter than 10 chars are dropped
        text = "OK\nThe photograph was taken in London.\nYes"
        result = split_claims(text)
        assert len(result) == 1
        assert result[0] == "The photograph was taken in London."

    def test_multiple_newlines_handled(self):
        text = "First claim here.\n\nSecond claim here.\n\nThird claim here."
        result = split_claims(text)
        # Empty lines create empty fragments that are filtered
        assert all(len(c) >= 10 for c in result)

    def test_exclamation_and_question_marks_split(self):
        text = "This image is fake! How can anyone believe it? The metadata is forged."
        result = split_claims(text)
        assert len(result) == 3

    def test_returns_list_type(self):
        result = split_claims("A single claim about something important.")
        assert isinstance(result, list)

    def test_claims_are_strings(self):
        result = split_claims("Claim one.\nClaim two is here.")
        assert all(isinstance(c, str) for c in result)


# ── Verdict parsing ────────────────────────────────────────────────────────────


class TestParseVerdict:
    """Tests for _parse_verdict — raw Ollama output → (verdict, explanation, confidence)."""

    def test_supported_parsed(self):
        raw = "SUPPORTED The claim matches known historical records."
        verdict, explanation, confidence = _parse_verdict(raw)
        assert verdict == "consistent_with_kb"
        assert "historical" in explanation
        assert confidence > 0.5

    def test_disputed_parsed(self):
        raw = "DISPUTED This contradicts multiple verified sources."
        verdict, explanation, confidence = _parse_verdict(raw)
        assert verdict == "inconsistent_with_kb"
        assert confidence > 0.5

    def test_unverified_parsed(self):
        raw = "UNVERIFIED There is insufficient information to assess this claim."
        verdict, explanation, confidence = _parse_verdict(raw)
        assert verdict == "insufficient_context_in_kb"
        assert confidence <= 0.45

    def test_case_insensitive_parsing(self):
        raw = "supported this appears to be accurate."
        verdict, _, _ = _parse_verdict(raw)
        assert verdict == "consistent_with_kb"

    def test_unknown_returns_unverified(self):
        raw = "I cannot determine the answer to this question."
        verdict, _, confidence = _parse_verdict(raw)
        assert verdict == "insufficient_context_in_kb"
        assert confidence == 0.20

    def test_markdown_fencing_stripped(self):
        raw = "```\nSUPPORTED The evidence supports this.\n```"
        verdict, _, _ = _parse_verdict(raw)
        assert verdict == "consistent_with_kb"

    def test_leading_colon_stripped(self):
        raw = ": SUPPORTED The claim is well-supported by available data."
        verdict, explanation, _ = _parse_verdict(raw)
        assert verdict == "consistent_with_kb"
        assert explanation  # explanation should not be empty

    def test_explanation_truncated_at_300_chars(self):
        long_explanation = "x" * 400
        raw = f"SUPPORTED {long_explanation}"
        _, explanation, _ = _parse_verdict(raw)
        assert len(explanation) <= 303  # 300 + "..."

    def test_empty_raw_returns_unverified(self):
        verdict, _, confidence = _parse_verdict("")
        assert verdict == "insufficient_context_in_kb"
        assert confidence == 0.20

    def test_confidence_higher_for_long_explanation(self):
        short_raw = "SUPPORTED Yes."
        long_raw = "SUPPORTED The evidence for this claim is robust and well-documented."
        _, _, conf_short = _parse_verdict(short_raw)
        _, _, conf_long = _parse_verdict(long_raw)
        assert conf_long >= conf_short


# ── Verdict aggregation ────────────────────────────────────────────────────────


class TestAggregateVerdicts:
    """Tests for _aggregate_verdicts — list[str] → overall verdict string."""

    def test_all_supported_returns_supported(self):
        assert _aggregate_verdicts(["consistent_with_kb", "consistent_with_kb", "consistent_with_kb"]) == "consistent_with_kb"

    def test_any_disputed_returns_disputed(self):
        assert _aggregate_verdicts(["consistent_with_kb", "inconsistent_with_kb", "insufficient_context_in_kb"]) == "inconsistent_with_kb"

    def test_disputed_overrides_supported(self):
        assert _aggregate_verdicts(["consistent_with_kb", "inconsistent_with_kb"]) == "inconsistent_with_kb"

    def test_mix_of_supported_and_unverified_returns_mixed(self):
        assert _aggregate_verdicts(["consistent_with_kb", "insufficient_context_in_kb"]) == "mixed_kb_match"

    def test_all_unverified_returns_unverified(self):
        assert _aggregate_verdicts(["insufficient_context_in_kb", "insufficient_context_in_kb"]) == "insufficient_context_in_kb"

    def test_any_unavailable_returns_unavailable(self):
        assert _aggregate_verdicts(["consistent_with_kb", "unavailable"]) == "unavailable"

    def test_empty_list_returns_unverified(self):
        assert _aggregate_verdicts([]) == "insufficient_context_in_kb"

    def test_single_supported(self):
        assert _aggregate_verdicts(["consistent_with_kb"]) == "consistent_with_kb"

    def test_single_disputed(self):
        assert _aggregate_verdicts(["inconsistent_with_kb"]) == "inconsistent_with_kb"

    def test_single_unverified(self):
        assert _aggregate_verdicts(["insufficient_context_in_kb"]) == "insufficient_context_in_kb"

    def test_unavailable_dominates_disputed(self):
        # unavailable is checked first
        assert _aggregate_verdicts(["inconsistent_with_kb", "unavailable"]) == "unavailable"


# ── Summary building ───────────────────────────────────────────────────────────


class TestBuildSummary:
    """Tests for _build_summary — human-readable summary string."""

    def _make_verdict(self, verdict: str, claim: str = "Test claim text here.") -> ClaimVerdict:
        return ClaimVerdict(
            claim=claim,
            verdict=verdict,
            explanation="Explanation.",
            confidence=0.5,
        )

    def test_unavailable_summary_mentions_ollama(self):
        claims = [self._make_verdict("unavailable")]
        summary = _build_summary("unavailable", claims)
        assert "Ollama" in summary

    def test_supported_summary_includes_count(self):
        claims = [self._make_verdict("consistent_with_kb"), self._make_verdict("consistent_with_kb")]
        summary = _build_summary("consistent_with_kb", claims)
        assert "2" in summary

    def test_summary_is_non_empty_string(self):
        claims = [self._make_verdict("insufficient_context_in_kb")]
        summary = _build_summary("insufficient_context_in_kb", claims)
        assert isinstance(summary, str) and summary

    def test_mixed_summary_mentions_mixed(self):
        claims = [self._make_verdict("consistent_with_kb"), self._make_verdict("insufficient_context_in_kb")]
        summary = _build_summary("mixed_kb_match", claims)
        # Summary framing intentionally avoids machine-readable tokens in
        # user-facing prose — it must convey "some matched, some didn't" in
        # natural language without echoing the status constants. The new
        # vocabulary copy says "consistent with reference material" and
        # "partial coverage".
        low = summary.lower()
        assert "consistent" in low and "partial coverage" in low


# ── Graceful degradation: Ollama unavailable ──────────────────────────────────


class TestOllamaUnavailable:
    """check_claims must degrade gracefully when Ollama is not running."""

    @pytest.mark.asyncio
    async def test_returns_unavailable_when_ollama_down(self):
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            result = await check_claims("The photograph was taken in 2022.")
        assert result.overall_verdict == "unavailable"
        assert all(v.verdict == "unavailable" for v in result.claims)
        assert result.model_used  # model name still present
        assert result.methodology  # methodology still present

    @pytest.mark.asyncio
    async def test_confidence_zero_when_unavailable(self):
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            result = await check_claims("Any claim text goes here for testing.")
        for claim in result.claims:
            assert claim.confidence == 0.0

    @pytest.mark.asyncio
    async def test_explanation_mentions_ollama_when_unavailable(self):
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            result = await check_claims("Any claim text goes here for testing.")
        for claim in result.claims:
            assert "Ollama" in claim.explanation


# ── Graceful degradation: model not pulled ────────────────────────────────────


class TestModelNotAvailable:
    """check_claims must degrade gracefully when the model is not pulled."""

    @pytest.mark.asyncio
    async def test_returns_unavailable_when_model_missing(self):
        with (
            patch(
                "app.services.claim_checker._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.claim_checker._check_model_available",
                new_callable=AsyncMock,
                return_value=False,
            ),
        ):
            result = await check_claims("The photograph was taken in Edinburgh.")
        assert result.overall_verdict == "unavailable"
        for claim in result.claims:
            assert "ollama pull" in claim.explanation

    @pytest.mark.asyncio
    async def test_model_name_in_explanation_when_missing(self):
        test_model = "qwen2.5:7b-instruct"
        with (
            patch(
                "app.services.claim_checker._check_ollama_available",
                new_callable=AsyncMock,
                return_value=True,
            ),
            patch(
                "app.services.claim_checker._check_model_available",
                new_callable=AsyncMock,
                return_value=False,
            ),
        ):
            result = await check_claims(
                "The photograph shows a mountain landscape.",
                model=test_model,
            )
        assert all(test_model in c.explanation for c in result.claims)


# ── Empty input handling ──────────────────────────────────────────────────────


class TestEmptyInput:
    """check_claims must handle empty / whitespace-only input gracefully."""

    @pytest.mark.asyncio
    async def test_empty_string_returns_no_claims(self):
        result = await check_claims("")
        assert result.overall_verdict == "insufficient_context_in_kb"
        assert result.claims == []

    @pytest.mark.asyncio
    async def test_whitespace_only_returns_no_claims(self):
        result = await check_claims("   \n  ")
        assert result.overall_verdict == "insufficient_context_in_kb"
        assert result.claims == []

    @pytest.mark.asyncio
    async def test_empty_input_summary_informative(self):
        result = await check_claims("")
        assert "No claims" in result.summary


# ── Positive path: mocked Ollama returning known responses ────────────────────


class TestPositivePath:
    """Mocked Ollama returning expected verdicts."""

    def _make_mock_response(self, text: str) -> MagicMock:
        """Build a mock httpx response object returning ``text`` as the Ollama response."""
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = {"response": text}
        mock_resp.raise_for_status = MagicMock()
        return mock_resp

    def _mock_tags_response(self, model: str = "qwen2.5:7b-instruct") -> MagicMock:
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = {"models": [{"name": model}]}
        return mock_resp

    @pytest.mark.asyncio
    async def test_supported_claim_returns_supported(self):
        tags_resp = self._mock_tags_response()
        generate_resp = self._make_mock_response(
            "SUPPORTED The claim is consistent with verified historical data."
        )

        async def fake_get(url, **kwargs):
            return tags_resp

        async def fake_post(url, **kwargs):
            return generate_resp

        mock_client = AsyncMock()
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_client.get = AsyncMock(side_effect=fake_get)
        mock_client.post = AsyncMock(side_effect=fake_post)

        with patch("httpx.AsyncClient", return_value=mock_client):
            result = await check_claims(
                "The Eiffel Tower was built in 1889.",
                model="qwen2.5:7b-instruct",
            )

        assert result.overall_verdict == "consistent_with_kb"
        assert result.claims[0].verdict == "consistent_with_kb"

    @pytest.mark.asyncio
    async def test_disputed_claim_returns_disputed(self):
        tags_resp = self._mock_tags_response()
        generate_resp = self._make_mock_response(
            "DISPUTED This claim contradicts documented historical records."
        )

        async def fake_get(url, **kwargs):
            return tags_resp

        async def fake_post(url, **kwargs):
            return generate_resp

        mock_client = AsyncMock()
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_client.get = AsyncMock(side_effect=fake_get)
        mock_client.post = AsyncMock(side_effect=fake_post)

        with patch("httpx.AsyncClient", return_value=mock_client):
            result = await check_claims(
                "The photograph was taken on the Moon in 1960.",
                model="qwen2.5:7b-instruct",
            )

        assert result.overall_verdict == "inconsistent_with_kb"

    @pytest.mark.asyncio
    async def test_multiple_claims_aggregated(self):
        """Two supported claims → overall supported."""
        tags_resp = self._mock_tags_response()
        generate_resp = self._make_mock_response(
            "SUPPORTED This is consistent with known facts about this location."
        )

        async def fake_get(url, **kwargs):
            return tags_resp

        async def fake_post(url, **kwargs):
            return generate_resp

        mock_client = AsyncMock()
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_client.get = AsyncMock(side_effect=fake_get)
        mock_client.post = AsyncMock(side_effect=fake_post)

        text = "The image shows London.\nThe photo was taken in summer.\nThe sky is clear."

        with patch("httpx.AsyncClient", return_value=mock_client):
            result = await check_claims(text, model="qwen2.5:7b-instruct")

        assert len(result.claims) == 3
        assert result.overall_verdict == "consistent_with_kb"

    @pytest.mark.asyncio
    async def test_context_passed_to_prompt(self):
        """The context string should be forwarded to Ollama."""
        tags_resp = self._mock_tags_response()
        generate_resp = self._make_mock_response("UNVERIFIED No information available.")

        captured_payloads: list[dict] = []

        async def fake_get(url, **kwargs):
            return tags_resp

        async def fake_post(url, json=None, **kwargs):
            if json:
                captured_payloads.append(json)
            return generate_resp

        mock_client = AsyncMock()
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_client.get = AsyncMock(side_effect=fake_get)
        mock_client.post = AsyncMock(side_effect=fake_post)

        with patch("httpx.AsyncClient", return_value=mock_client):
            await check_claims(
                "The image was captured in Edinburgh.",
                context="EXIF data shows GPS coordinates in Scotland.",
                model="qwen2.5:7b-instruct",
            )

        assert any(
            "Edinburgh" in p.get("prompt", "") and "Scotland" in p.get("prompt", "")
            for p in captured_payloads
        )

    @pytest.mark.asyncio
    async def test_response_schema_valid(self):
        """check_claims must always return a ClaimCheckResponse-compatible structure."""
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            result = await check_claims("A claim about something specific.")

        # Validate via Pydantic (will raise if schema is wrong)
        validated = ClaimCheckResponse.model_validate(result.model_dump())
        assert validated.overall_verdict in (
            "consistent_with_kb", "inconsistent_with_kb", "insufficient_context_in_kb", "mixed_kb_match", "unavailable"
        )
        for claim in validated.claims:
            assert 0.0 <= claim.confidence <= 1.0
            assert claim.verdict in ("consistent_with_kb", "inconsistent_with_kb", "insufficient_context_in_kb", "unavailable")


# ── API endpoint tests ─────────────────────────────────────────────────────────


class TestClaimCheckEndpoint:
    """Integration tests for POST /forensics/claim-check via the ASGI test client."""

    @pytest.mark.asyncio
    async def test_endpoint_returns_200_when_ollama_unavailable(self):
        """The endpoint must return HTTP 200 even when Ollama is down."""
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            transport = ASGITransport(app=app)
            async with AsyncClient(transport=transport, base_url="http://test") as client:
                resp = await client.post(
                    "/forensics/claim-check",
                    params={"claims_text": "The photograph was taken in 2020."},
                )
        assert resp.status_code == 200

    @pytest.mark.asyncio
    async def test_endpoint_response_has_required_fields(self):
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            transport = ASGITransport(app=app)
            async with AsyncClient(transport=transport, base_url="http://test") as client:
                resp = await client.post(
                    "/forensics/claim-check",
                    params={"claims_text": "This image shows a real event."},
                )
        data = resp.json()
        assert "overall_verdict" in data
        assert "claims" in data
        assert "model_used" in data
        assert "methodology" in data
        assert "summary" in data

    @pytest.mark.asyncio
    async def test_endpoint_overall_verdict_unavailable_when_ollama_down(self):
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            transport = ASGITransport(app=app)
            async with AsyncClient(transport=transport, base_url="http://test") as client:
                resp = await client.post(
                    "/forensics/claim-check",
                    params={"claims_text": "This is a real photograph."},
                )
        assert resp.json()["overall_verdict"] == "unavailable"

    @pytest.mark.asyncio
    async def test_endpoint_missing_claims_text_returns_422(self):
        """Missing required claims_text query parameter should return 422."""
        transport = ASGITransport(app=app)
        async with AsyncClient(transport=transport, base_url="http://test") as client:
            resp = await client.post("/forensics/claim-check")
        assert resp.status_code == 422

    @pytest.mark.asyncio
    async def test_endpoint_accepts_context_parameter(self):
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            transport = ASGITransport(app=app)
            async with AsyncClient(transport=transport, base_url="http://test") as client:
                resp = await client.post(
                    "/forensics/claim-check",
                    params={
                        "claims_text": "The photograph was taken in Vienna.",
                        "context": "EXIF GPS data shows Austria.",
                    },
                )
        assert resp.status_code == 200

    @pytest.mark.asyncio
    async def test_endpoint_claim_confidence_in_range(self):
        with patch(
            "app.services.claim_checker._check_ollama_available",
            new_callable=AsyncMock,
            return_value=False,
        ):
            transport = ASGITransport(app=app)
            async with AsyncClient(transport=transport, base_url="http://test") as client:
                resp = await client.post(
                    "/forensics/claim-check",
                    params={"claims_text": "The image shows a document."},
                )
        data = resp.json()
        for claim in data["claims"]:
            assert 0.0 <= claim["confidence"] <= 1.0


# ── Health endpoint: rag capability flag ──────────────────────────────────────


class TestHealthRagFlag:
    """The rag capability flag in /health should reflect Ollama availability."""

    @pytest.mark.asyncio
    async def test_rag_false_when_ollama_unavailable(self):
        """When Ollama is unreachable, rag capability should be False."""
        # The health endpoint does its own httpx call — mock at the httpx level
        mock_resp = MagicMock()
        mock_resp.status_code = 503

        mock_client = AsyncMock()
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_client.get = AsyncMock(side_effect=Exception("Connection refused"))

        with patch("httpx.AsyncClient", return_value=mock_client):
            transport = ASGITransport(app=app)
            async with AsyncClient(transport=transport, base_url="http://test") as client:
                resp = await client.get("/health")

        data = resp.json()
        assert data["capabilities"]["rag"] is False

    @pytest.mark.asyncio
    async def test_rag_true_when_ollama_available(self):
        """When Ollama is reachable, rag capability should be True."""
        mock_resp = MagicMock()
        mock_resp.status_code = 200
        mock_resp.json.return_value = {"models": []}

        mock_client = AsyncMock()
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)
        mock_client.get = AsyncMock(return_value=mock_resp)

        with patch("httpx.AsyncClient", return_value=mock_client):
            transport = ASGITransport(app=app)
            async with AsyncClient(transport=transport, base_url="http://test") as client:
                resp = await client.get("/health")

        data = resp.json()
        assert data["capabilities"]["rag"] is True
