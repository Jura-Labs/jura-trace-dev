# BL-CLAIM-007: a result does not say what kind of evidence it rests on

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of legislation.
**Severity**: Critical. The EU's Code of Practice under AI Act Article 50
now states that forensic detection is not mature enough to meet the Act's
quality requirements, and asks that results say which technique they rest
on. Trace's strongest AI headline rests on forensic and ML signals alone.

## What is wrong

`ui/src/lib/components/SimpleVerdict.svelte`:

- `:153-154`: a result is in the "ai-generated" category when a watermark
  is found **or** the ML detectors say AI (`isAiGenerated`).
- `:164-166`: the headline is "AI-Generated Content Detected" (watermark),
  "Likely AI-Generated" (high-confidence ML) or "Possible AI Generation".
- `:380-381`: the next step for every result in that category, including
  ML-only ones, is "Treat as AI-generated content. Do not present as a
  camera photograph."

The headline does not say whether it rests on a signed provenance record,
a watermark, or statistical detectors.

## The source

Code of Practice on Transparency of AI-Generated Content (final, June
2026; PDF from ec.europa.eu/newsroom/dae/redirection/document/129555, read
22 October 2026):

- Measure 2.2: "At the time of publication of the Code, forensic detection
  mechanisms were not deemed mature enough" to meet the Article 50 quality
  requirements.
- Measure 2.3: results should state whether they rest on metadata,
  watermark marking, forensic detection or other techniques.
- Sub-measure 2.1.2: results downloadable "in a digitally signed format,
  including at least a hash of the content submitted for detection, a URL
  or other identifier of the detection solution, and a timestamp".
- Measure 3.5(d) encourages "publicly available detection solutions that
  can be run locally, without reliance on third-party detection services".

These measures bind signatories (AI providers), not Trace. They are the
best current statement of what a reader should expect a detection result to
say.

## What would fix it

1. Every headline names its basis in plain words: "Content Credentials say
   this was made with an AI tool", "An AI watermark was found", or "Our
   statistical checks found patterns often seen in AI images".
2. ML-only results do not tell the user to treat the image as
   AI-generated. Suggested: "Our checks suggest this may be AI-generated.
   Look for other evidence before relying on it."
3. The Verification Record already holds the file's hash and the engine
   version. It has no independent timestamp
   (`docs/VERIFICATION_RECORD.md:73`). Record that as a known gap next to
   2.1.2; it is not a v1.3.0 requirement.
4. Check the help, report and PDF wording for the same pattern.

## What not to do

- Do not say or imply Trace makes anyone compliant with the AI Act, or
  that it implements the Code. voice.md bans "compliant" about Trace.
- Do not describe Trace as an "AI detection tool" in US-facing copy.
  California's AI Transparency Act (SB 942) defines that term for the
  tools generative-AI providers must offer, and a 2026 amendment reportedly
  renames it "disclosure verification tool" (secondary sources only, not
  checked against the statute).
