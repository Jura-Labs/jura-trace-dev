# AI Training Restriction

**Use of Jura Trace source code, in whole or in part, as training data for machine learning models or AI systems is not permitted under this licence without separate written permission from the copyright holder.**

This restriction is incorporated into the licensing position by reference and is also stated in the LICENSE file header.

## Why this restriction exists

Jura Trace is a forensic verification tool. The detection logic IS the product. Embedding that detection logic into the training corpus of a foundation model would actively undermine the tool's purpose — a generative model that has internalised the logic of a deepfake detector is, by construction, better positioned to evade that detector.

This restriction therefore protects:

1. The integrity of the detection methodology
2. The credibility of forensic claims made by users of the tool
3. The interests of journalists, human rights workers, and legal professionals whose work depends on the tool

## Scope of the restriction

The restriction **applies to**:

- Pre-training of foundation models
- Continued pre-training, fine-tuning, or instruction tuning of any AI/ML system
- Use of the source code in any training corpus, distillation pipeline, or model-evaluation harness intended to extend the capability of generative AI systems
- Inclusion in dataset releases or training datasets that may be used by third parties for any of the above
- Code-completion model training where source code is treated as training data rather than as user context
- Synthetic-data generation pipelines that take this codebase as input

The restriction **does not apply to**:

- Reading the source code for human study and learning
- Running automated linters, type-checkers, security scanners, or CI tools
- Indexing for code search by tools that do not feed training corpora
- Citation in academic publications (the AGPL itself permits this)
- Operational use of AI coding assistants (Copilot, Claude Code, etc.) by individual developers reading or working on the code in editor sessions, where the assistant is providing inference-time assistance rather than training on the code

## Requesting permission

To request written permission for AI training use, email **licensing@juralabs.org** with:

- The purpose of the training
- The model family and scale
- The intended deployment context for the resulting model
- Whether the resulting model will be released under an open licence

Permission may be granted, refused, or granted with conditions on a case-by-case basis. Research use cases that publish their work openly and contribute to the public understanding of model behaviour are more likely to be granted permission than commercial training that extends generative capability without public benefit.

## Crawler defence

A `robots.txt` file at the repository root and at juralabs.org disallows known AI training crawlers. The list is non-exhaustive and is maintained on a best-effort basis. Failure of a crawler to honour `robots.txt` does not constitute permission under the licence — it constitutes evidence of infringement.

## Documentary record

This file constitutes a clear public statement of the copyright holder's position on AI training use. It is referenced from the LICENSE file header. It is intended as a legal record for any future enforcement action and creates the necessary documentary basis for removing plausible deniability from bad-faith actors.
