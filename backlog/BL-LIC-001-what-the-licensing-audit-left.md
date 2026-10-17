# BL-LIC-001: what the licensing audit left

**Status**: Open. Found 12 October 2026, auditing the licensing terms on the
GitHub organisation profile, both repositories and the public README.
**Raised**: 13 October 2026
**Severity**: Low. The licence itself is stated correctly everywhere a user
or a funder would look: `LICENSE`, both READMEs, the Cargo manifests and,
since 12 October, the organisation profile and the public `TRAINING.md` and
`NOTICE`. These are the loose ends.

## What is wrong

1. **`TRAINING.md:49` still reads as a licence term.** "Failure of a crawler
   to honour `robots.txt` does not constitute permission under the licence —
   it constitutes evidence of infringement." The rest of the file was
   rewritten as a request and a statement of position, because AGPL-3.0
   section 7 does not allow an added restriction on use. This sentence
   survived, in this repository and in the public one.
2. **`ui/package.json` has no `license` field.** It has `"private": true`,
   so nothing is published from it, but licence scanners report the frontend
   as unlicensed.
3. **51 of 161 source files have no SPDX header.** Counted on `main` across
   `src-tauri/src`, `sidecar/app` and `ui/src` (`.rs`, `.py`, `.ts`,
   `.svelte`): 110 carry `SPDX-License-Identifier`. The audit of 12 October
   counted 53 of 203 over a wider set of files.
4. **Five files under `docs/` still mention PolyForm**, the licence Jura
   Trace left in May 2026. Not read individually; some may be history that
   should stay.
5. **`Jura-Labs/jura-trace-dev` has no repository description.**

## Why it matters

Item 1 is the only one with a consequence. A sentence that claims a
licence-backed prohibition the licence cannot carry is the thing an OSI or
OpenSSF reviewer would quote. The others are hygiene that a funder's
automated scan will list.

## What would fix it

1. Replace the sentence at `TRAINING.md:49` with one that states the
   position without invoking the licence. Paul should approve the wording.
   The public repository's copy needs the same edit and his explicit
   approval to push.
2. Add `"license": "AGPL-3.0-or-later"` to `ui/package.json`.
3. Add the SPDX header to the 51 files, and add a check to the repo hygiene
   job so a new file without one fails.
4. Read the five `docs/` files and correct or leave each deliberately.
5. Set the description with `gh repo edit`.

## What not to do

- Do not reintroduce "additional terms" language anywhere. The position on
  AI training is a request, and `NOTICE` now says so.
- Do not rewrite history to remove old PolyForm references. The change of
  licence is part of the record.
