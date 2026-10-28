# BL-REL-010: release notes have no "Known limitations" section

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 backlog review.
**Severity**: Low to fix, medium in effect. Users learn about limits from
the middle of a bullet, or not at all.

## What is wrong

`CHANGELOG.md` has a "Known limitations" heading in older sections
(`:347`, `:1099`) but none in v1.2.0 (`:9`) or v1.1.0 (`:198`). In v1.2.0,
two things a user should know are inline, inside other entries:

- Files signed in Published mode between 22 May 2026 and v1.2.0 now show
  as not valid (`assertion.action.malformed`), and need re-signing.
- An installation kept off the startup disk still cannot update itself.

The release page body is built from the changelog, so it inherits the gap.

## What would fix it

1. Every release section ends with "Known limitations", even if it says
   "None known".
2. For v1.3.0, carry the two above forward if still true, and add any
   from this release (for example, BL-DET-001's measured results on 2026
   generators, if they are weak).
3. A check in `scripts/` or CI that fails a release whose changelog
   section lacks the heading.

## What not to do

- Do not rewrite the v1.2.0 section after the fact. Add the heading from
  v1.3.0 on.
