# Generative AI Use Policy

**Effective:** 2026-05-30
**Version:** 1.0
**Applies to:** all contributors to Jura Trace and related Jura Labs CIC codebases.

This policy describes how Generative AI tools (large language models, code assistants) are used in this project and the responsibilities of human contributors. It is published in accordance with the [NLnet Foundation's Policy on the use of Generative Artificial Intelligence for NLnet-funded projects](https://nlnet.nl/genai/) (effective 8 December 2025, version 1.1 of 26 January 2026).

The policy applies whether or not the project is currently funded by NLnet. Where grant-specific obligations differ (per-commit logging for funded work), those are noted explicitly.

---

## Principles

### 1. Human responsibility is non-delegable

AI tools may assist, but human contributors remain accountable for the correctness, security, reproducibility, and licensing of every contribution. The AI assistant is not a contributor of record. A human signs off; a human is answerable.

### 2. Architecture and design decisions are human-led

AI may be consulted on trade-offs and option-comparison. The choice between options is made by a human contributor who understands and can defend the decision in writing.

### 3. Tests cannot be fully implemented by AI

Test cases, regression-test design, recursive-test coverage, and detector calibration are not delegated to AI. Generated test scaffolding (argument-parsing stubs, table-driven fixtures, mock-object skeletons) is acceptable as a starting point, but **the test logic, the assertions, and the coverage decisions must be human-authored and human-reviewed**. AI cannot sign off on test coverage.

### 4. Sources cannot be fully verified by AI

Any claim about a source must be human-checked against the primary source. This explicitly includes:

- Academic paper citations and statistical claims
- Calibration figures, false-positive rates, recall numbers, AUC scores
- Corpus provenance entries
- Vendor specification references (for example, C2PA spec section numbers)
- Trust-list certificate fingerprints
- Conformance-programme record identifiers
- Press claims and competitor-landscape statements
- Licence terms of third-party dependencies

AI search and AI summarisation are starting points. They are not sufficient evidence on their own. The maintainer's working assumption is that any AI-stated fact is a hypothesis until it is verified against a primary source.

### 5. Security-sensitive code receives explicit human review

Code touching signing, key handling, network boundaries, file-system access, IPC permissions, certificate handling, or trust evaluation is reviewed by a human contributor before merge regardless of how it was drafted. AI suggestions in these areas are treated as drafts requiring human-verified correctness against the relevant specification.

### 6. AI output is a draft, not a deliverable

Generated code is treated as a starting point. It is not merged until reviewed, tested, and accepted by a human contributor. The reviewing human must be able to explain what the generated code does and why it is correct.

### 7. No misrepresentation

Contributors do not claim AI-generated work as their own without disclosure. Where AI involvement materially affects a contribution, that involvement is disclosed (see "Disclosure" below).

---

## What AI is used for in this codebase

- Boilerplate generation (struct serialisation, error wrappers, repetitive transforms)
- Documentation drafting (README sections, in-source comments, user-facing guides, changelog entries)
- Refactoring suggestions and code-review pre-checks
- Accessibility fixes (ARIA attribute placement, focus handling, contrast adjustment)
- Test scaffolding (the structure of a test file, not the assertions)
- Drafting commit messages and pull-request descriptions

## What AI is not used for in this codebase

- Architectural choices between competing approaches
- Detector threshold setting, calibration sign-off, or model-evaluation conclusions
- Test logic, test coverage decisions, or recursive and regression test sign-off
- Source citation, fact-checking, or verification of vendor specifications
- Security review of signing, key handling, or trust-evaluation code
- Corpus collection, annotation, or quality control
- The runtime forensic verdict pipeline (no large language model is in the detection path that produces user-visible verdicts)

---

## Disclosure

### Project-level disclosure

This policy and the README's "On the use of Generative AI in this codebase" section are the project-level disclosure. They are kept current. Material changes to either are accompanied by a commit-message note in the changelog.

### Per-contribution disclosure

Contributors using AI assistance for code or documentation should disclose:

- The model used (with version where available, for example "Claude Opus 4.x")
- The nature of the assistance (drafting, refactoring, documentation, accessibility, etc.)
- Specific prompts or prompt themes where reproducibility matters

Acceptable disclosure formats:

- **Commit-message body or footer** is preferred for substantive generated contributions, especially under grant-funded work.
- **An entry in `docs/genai-provenance.md`** is acceptable for batched disclosure of routine assistance, with each entry referenced by commit hash.
- **A `Co-Authored-By` trailer** is acceptable as a minimum marker for light assistance, but is not sufficient on its own where prompts and outputs materially affect the contribution.

For pull requests, the `AI-Disclosure` field in the PR template should be completed.

---

## Pre-grant codebase

Code committed before a specific grant award begins is governed by this policy at the project level (the README declaration and these principles describe the practice). Per-commit retroactive logging is not applied to pre-grant code.

## Funded-work codebase

For code authored under a specific grant deliverable (for example, an NLnet NGI Zero Commons Fund award), per-commit AI-use disclosure is the working practice. The provenance log in `docs/genai-provenance.md` is the canonical record for the funded work. The `AI-Disclosure` field in the PR template is mandatory for funded-work pull requests.

Where the NLnet policy and this policy differ, the NLnet policy takes precedence for the duration and scope of any NLnet-funded work.

---

## Review and updates

This policy is reviewed at minimum each major release and updated as the practice evolves. Material updates are versioned (this is version 1.0).

| Version | Date | Notes |
|---------|------|-------|
| 1.0 | 2026-05-30 | Initial release. Aligned with NLnet GenAI policy v1.1. |

---

## Contact

| Subject | Email |
|---------|-------|
| Questions about this policy | paul@juralabs.org |
| Security concerns about AI-assisted code in this project | security@juralabs.org |
| Commercial-use questions | licensing@juralabs.org |

---

*This policy is licensed under CC BY-SA 4.0 and may be adapted by other open-source projects.*
