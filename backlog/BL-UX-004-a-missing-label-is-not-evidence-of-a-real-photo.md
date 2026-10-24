# BL-UX-004: a missing label is not evidence of a real photo

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of legislation.
**Severity**: Medium. Labelling duties are new, so readers will start to
treat "no label" as "real". Trace's result is silent on that.

## What is wrong

When a file has no Content Credentials, the verify page says only "No
Content Credentials attached." (`ui/src/routes/verify/+page.svelte:3540`;
`ContentCredentialsSeal.svelte:45`). Nothing says what that does and does
not mean.

From 2 August 2026, EU AI Act Article 50 requires providers of generative
AI to mark their output, and the review reports that systems already on
the market have until 2 December 2026. California's AI Transparency Act
(SB 942) is reported operative from 2 August 2026 too. These come from the
review's reading, partly of law-firm summaries; not checked against the
legislation. Either way, most images in circulation were made before any
marking duty, and marks are stripped by screenshots, re-encoding and most
social platforms.

## What would fix it

Under "No Content Credentials attached", one plain line:

> Most images have none, real or AI-made. Credentials are often removed
> when an image is shared, so their absence tells you nothing either way.

Put the same sentence in the help page on Content Credentials and in the
PDF report where the C2PA section is empty.

## What not to do

- Do not let the absence of credentials or a watermark move the trust
  score down. That would be a change to scoring, not wording, and is out of
  scope here.
- Do not name the AI Act in the app. It is context for this item, not a
  claim about the law.
