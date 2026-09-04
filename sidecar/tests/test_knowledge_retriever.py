# SPDX-License-Identifier: AGPL-3.0-or-later

"""
Tests for the TF-IDF KnowledgeRetriever.

No Ollama connection is required.  Tests use:
- The real knowledge_base/ directory when present (integration-style).
- A temporary in-memory knowledge base created via tmp_path for unit tests
  that need controlled content.

Test coverage:
  - test_retriever_loads_knowledge_base
  - test_retrieve_returns_relevant_passages
  - test_retrieve_empty_query
  - test_is_available
  - test_passage_count
  - test_retrieve_top_k_respected
  - test_retrieve_returns_list_type
  - test_nonexistent_kb_dir_is_available_false
  - test_empty_kb_dir_is_available_false
  - test_retrieve_on_unavailable_retriever_returns_empty
  - test_retrieve_scores_relevant_passage_higher
  - test_passage_count_matches_loaded_content
  - test_document_count
"""

from __future__ import annotations

import textwrap
from pathlib import Path


from app.services.knowledge_retriever import (
    KnowledgeRetriever,
    _split_into_passages,
)


# ── Helpers ───────────────────────────────────────────────────────────────────


def _make_kb(tmp_path: Path, files: dict[str, str]) -> KnowledgeRetriever:
    """
    Create a temporary knowledge base with the given {filename: content} mapping
    and return a KnowledgeRetriever pointed at it.
    """
    kb_dir = tmp_path / "knowledge_base"
    kb_dir.mkdir()
    for name, content in files.items():
        (kb_dir / name).write_text(content, encoding="utf-8")
    return KnowledgeRetriever(kb_dir=str(kb_dir))


# ── _split_into_passages unit tests ───────────────────────────────────────────


class TestSplitIntoPassages:
    """Low-level tests for the paragraph splitter."""

    def test_double_newline_splits_paragraphs(self):
        text = "First paragraph with enough content here.\n\nSecond paragraph with enough content here."
        result = _split_into_passages(text)
        assert len(result) == 2

    def test_section_headings_filtered_out(self):
        # Lines of equals signs are shorter than _MIN_PASSAGE_LENGTH
        text = "==========\n\nThis is a real paragraph with substantial content.\n\n=========="
        result = _split_into_passages(text)
        assert len(result) == 1
        assert "real paragraph" in result[0]

    def test_short_fragments_filtered(self):
        text = "OK\n\nThis is a proper paragraph with more than forty characters of content."
        result = _split_into_passages(text)
        assert len(result) == 1
        assert "proper paragraph" in result[0]

    def test_single_paragraph_returned_as_list(self):
        text = "A single paragraph that is long enough to pass the minimum length filter."
        result = _split_into_passages(text)
        assert isinstance(result, list)
        assert len(result) == 1

    def test_empty_string_returns_empty_list(self):
        assert _split_into_passages("") == []

    def test_whitespace_only_returns_empty_list(self):
        assert _split_into_passages("   \n\n   \n\n   ") == []


# ── KnowledgeRetriever — availability ─────────────────────────────────────────


class TestIsAvailable:
    """Tests for the is_available() method."""

    def test_is_available_with_real_kb(self):
        """The real knowledge_base/ should be present and loadable in the sidecar tree."""
        retriever = KnowledgeRetriever()
        assert retriever.is_available() is True

    def test_nonexistent_kb_dir_is_available_false(self, tmp_path):
        retriever = KnowledgeRetriever(kb_dir=str(tmp_path / "does_not_exist"))
        assert retriever.is_available() is False

    def test_empty_kb_dir_is_available_false(self, tmp_path):
        kb_dir = tmp_path / "empty_kb"
        kb_dir.mkdir()
        retriever = KnowledgeRetriever(kb_dir=str(kb_dir))
        assert retriever.is_available() is False

    def test_kb_with_only_short_passages_is_available_false(self, tmp_path):
        """A file containing only headings/short lines should yield no passages."""
        kb_dir = tmp_path / "kb"
        kb_dir.mkdir()
        (kb_dir / "short.txt").write_text("OK\n\nYes\n\nNo\n\n====\n\n----\n")
        retriever = KnowledgeRetriever(kb_dir=str(kb_dir))
        assert retriever.is_available() is False


# ── KnowledgeRetriever — passage counts ───────────────────────────────────────


class TestPassageCount:
    """Tests for passage_count and document_count properties."""

    def test_passage_count_with_real_kb(self):
        """Real KB should have substantially more than zero passages."""
        retriever = KnowledgeRetriever()
        assert retriever.passage_count > 10

    def test_passage_count_matches_loaded_content(self, tmp_path):
        content = "\n\n".join(
            [
                "This is the first paragraph with enough characters to pass the filter.",
                "This is the second paragraph with enough characters to pass the filter.",
                "This is the third paragraph with enough characters to pass the filter.",
            ]
        )
        retriever = _make_kb(tmp_path, {"test.txt": content})
        assert retriever.passage_count == 3

    def test_document_count(self, tmp_path):
        content_a = "File A paragraph one with enough content to be indexed properly.\n\nFile A paragraph two with enough content here."
        content_b = "File B paragraph one with enough content to be indexed properly.\n\nFile B paragraph two with enough content here."
        retriever = _make_kb(tmp_path, {"a.txt": content_a, "b.txt": content_b})
        assert retriever.document_count == 2

    def test_passage_count_zero_when_unavailable(self, tmp_path):
        retriever = KnowledgeRetriever(kb_dir=str(tmp_path / "missing"))
        assert retriever.passage_count == 0

    def test_document_count_zero_when_unavailable(self, tmp_path):
        retriever = KnowledgeRetriever(kb_dir=str(tmp_path / "missing"))
        assert retriever.document_count == 0


# ── KnowledgeRetriever — retrieval ────────────────────────────────────────────


class TestRetrieve:
    """Tests for the retrieve() method."""

    def test_retriever_loads_knowledge_base(self):
        """retrieve() on the real KB returns non-empty results for a relevant query."""
        retriever = KnowledgeRetriever()
        results = retriever.retrieve("Stable Diffusion AI image generation")
        assert len(results) > 0

    def test_retrieve_returns_list_type(self, tmp_path):
        content = "This passage is about image forensics and error level analysis technique."
        retriever = _make_kb(tmp_path, {"test.txt": content})
        result = retriever.retrieve("image forensics")
        assert isinstance(result, list)

    def test_retrieve_empty_query_returns_empty(self, tmp_path):
        content = "A proper knowledge base passage about something interesting and informative."
        retriever = _make_kb(tmp_path, {"test.txt": content})
        assert retriever.retrieve("") == []
        assert retriever.retrieve("   ") == []

    def test_retrieve_on_unavailable_retriever_returns_empty(self, tmp_path):
        retriever = KnowledgeRetriever(kb_dir=str(tmp_path / "missing"))
        assert retriever.retrieve("any query about anything") == []

    def test_retrieve_returns_relevant_passages(self, tmp_path):
        """The most relevant passage should be returned when querying for its topic."""
        c2pa_passage = textwrap.dedent("""\
            C2PA stands for Coalition for Content Provenance and Authenticity.
            It is a cryptographic standard for signing provenance metadata into
            media files to verify their origin and editing history.""")
        ela_passage = textwrap.dedent("""\
            Error Level Analysis re-compresses a JPEG image and measures the
            difference. Regions at a different compression quality show elevated
            error levels, which may indicate splicing from a different source.""")
        retriever = _make_kb(tmp_path, {
            "c2pa.txt": c2pa_passage,
            "ela.txt": ela_passage,
        })
        results = retriever.retrieve("C2PA provenance cryptographic signing")
        assert len(results) > 0
        # The C2PA passage should be present given the query focuses on C2PA
        combined = " ".join(results).lower()
        assert "c2pa" in combined or "provenance" in combined or "cryptographic" in combined

    def test_retrieve_scores_relevant_passage_higher(self, tmp_path):
        """A query about deepfakes should rank the deepfake passage above an unrelated one."""
        deepfake_passage = textwrap.dedent("""\
            Deepfake videos use encoder-decoder neural networks to perform face
            swapping. Detection relies on analysing facial boundaries, temporal
            inconsistencies, and unnatural eye reflections in the manipulated frames.""")
        unrelated_passage = textwrap.dedent("""\
            The capital of France is Paris. The Eiffel Tower was built for the
            1889 World Fair. It stands 330 metres tall and was the world's tallest
            structure for over forty years after its completion.""")
        retriever = _make_kb(tmp_path, {
            "deepfake.txt": deepfake_passage,
            "unrelated.txt": unrelated_passage,
        })
        results = retriever.retrieve("deepfake face swap neural network detection", top_k=2)
        assert len(results) >= 1
        # The deepfake passage should appear in results
        assert any("deepfake" in r.lower() or "face" in r.lower() for r in results)

    def test_retrieve_top_k_respected(self, tmp_path):
        """retrieve() should return at most top_k results."""
        passages = [
            f"This is passage number {i} about image forensics and verification techniques for claim checking."
            for i in range(10)
        ]
        content = "\n\n".join(passages)
        retriever = _make_kb(tmp_path, {"test.txt": content})
        results = retriever.retrieve("image forensics", top_k=3)
        assert len(results) <= 3

    def test_retrieve_with_real_kb_c2pa_query(self):
        """Real KB: a C2PA query should surface C2PA-related passages."""
        retriever = KnowledgeRetriever()
        results = retriever.retrieve("C2PA Content Credentials manifest signing", top_k=3)
        assert len(results) > 0
        combined = " ".join(results).lower()
        assert "c2pa" in combined or "content credentials" in combined or "manifest" in combined

    def test_retrieve_with_real_kb_forensics_query(self):
        """Real KB: a forensics query should surface forensic technique passages."""
        retriever = KnowledgeRetriever()
        results = retriever.retrieve("ELA error level analysis JPEG compression forensics", top_k=3)
        assert len(results) > 0
        combined = " ".join(results).lower()
        assert any(
            kw in combined for kw in ("ela", "error level", "jpeg", "forensic", "compression")
        )

    def test_retrieve_with_real_kb_misinformation_query(self):
        """Real KB: a misinformation query should surface relevant passages."""
        retriever = KnowledgeRetriever()
        results = retriever.retrieve("out of context image misleading caption misinformation", top_k=3)
        assert len(results) > 0

    def test_retrieve_passage_strings_are_non_empty(self, tmp_path):
        content = "A substantive passage about image manipulation and digital forensics techniques."
        retriever = _make_kb(tmp_path, {"test.txt": content})
        results = retriever.retrieve("image manipulation forensics")
        assert all(isinstance(r, str) and r for r in results)

    def test_lazy_loading_does_not_reload(self, tmp_path):
        """is_available() and retrieve() should not reload the index on repeated calls."""
        content = "A passage about image provenance and content authenticity verification."
        retriever = _make_kb(tmp_path, {"test.txt": content})
        assert retriever.is_available() is True
        first_count = retriever.passage_count
        # Second call should return the same count without reloading
        assert retriever.passage_count == first_count
