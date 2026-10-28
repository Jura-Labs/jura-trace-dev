# BL-A11Y-001: no accessibility audit since May, and no accessibility statement

**Status**: Open. Target v1.3.0 (Paul, 22 October 2026).
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: Medium. Archives and public-sector buyers in the UK and EU
expect WCAG 2.2 AA and an accessibility statement.

## What is wrong

- 44 files under `ui/src` carry ARIA attributes or roles, from an
  accessibility pass around rc.23 (May 2026). No audit has been run since,
  and v1.1.0 and v1.2.0 added screens (Verification Record export, API
  keys, release-news card, the second-opinion note).
- No automated accessibility check runs in CI: no axe dependency in
  `ui/package.json`.
- No accessibility statement exists for the app.

The review reports EN 301 549 v4.1.1 (published September 2026) moving to
WCAG 2.2, and WCAG 2.2 as ISO/IEC 40500:2025; not checked against the
standards bodies' pages.

## What would fix it

1. Add axe checks to the end-to-end suite for the main screens (verify,
   result, protect, settings, help), failing on serious and critical
   issues.
2. A manual pass with a screen reader (VoiceOver on macOS, NVDA on
   Windows) and keyboard only, through first run, a check and a record
   export.
3. Fix what is found, starting with the verify result, which carries the
   most meaning in colour and badges.
4. Publish an accessibility statement: what was tested, against what,
   when, and the known gaps. Do not claim conformance that was not tested.

## What not to do

- Do not claim "WCAG compliant". State what was tested and what is known
  to fail.
