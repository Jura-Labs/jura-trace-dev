"""
Jura Trace Sidecar — Content-type classification schema.

Lightweight heuristic classifier that categorises an image as photograph,
screenshot, document, artwork, or unknown. Used by the verification pipeline
to gate AI-detection models (UnivFD, GBM) away from non-photographic content
that would otherwise trigger false positives.
"""

from typing import Literal

from pydantic import BaseModel, Field


ContentCategory = Literal["photograph", "screenshot", "document", "artwork", "unknown"]


class ContentTypeResult(BaseModel):
    """Result of the content-type heuristic classifier."""

    category: ContentCategory = Field(
        ...,
        description="Top-level content category.",
    )
    confidence: float = Field(
        ...,
        ge=0.0,
        le=1.0,
        description="Classifier confidence in the chosen category (0-1).",
    )
    ai_detection_suitable: bool = Field(
        ...,
        description=(
            "True when AI-detection models should be run on this image. "
            "False for screenshot/document/unknown — running UnivFD/GBM on "
            "those content types produces unreliable false-positive verdicts."
        ),
    )
    signals: dict = Field(
        default_factory=dict,
        description="Contributing evidence (per-signal booleans / numerics).",
    )
    reasoning: str = Field(
        ...,
        description="One-line human-readable explanation.",
    )
