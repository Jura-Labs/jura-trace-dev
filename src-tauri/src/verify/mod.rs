//! Verification sub-modules.
//!
//! This module tree holds the building blocks of the verify pipeline.
//! More submodules arrive in step A3 (pipeline orchestration, result types).
//!
//! # Sub-modules
//! - [`trust`] — trust-score computation (`compute_trust`, `document_trust`, all named constants).
//!   This is the AGPL reproducibility anchor cited in the methodology page and PDF reports.
//! - [`input_quality`] — pre-pipeline input quality assessment (`assess_input_quality`,
//!   `estimate_jpeg_quality`, `detect_screenshot`).

pub(crate) mod input_quality;
pub(crate) mod pipeline;
pub(crate) mod trust;
