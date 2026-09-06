# BL-DEPS-002: twelve frontend advisories, and only one of them ships

**Status**: Open. Found 3 September 2026.
**Raised**: 3 September 2026
**Severity**: Medium. The headline numbers are alarming and mostly are not.
The one that matters is a moderate, not the critical.

## What is wrong

`npm audit` in `ui/`, run 3 September 2026:

```
{'info': 0, 'low': 2, 'moderate': 5, 'high': 4, 'critical': 1, 'total': 12}
```

Broken out, with what each one actually is:

| Package | Severity | Ships to a user? | Fix |
|---|---|---|---|
| dompurify | moderate | No, see the correction below | Non-major |
| vitest, @vitest/mocker | critical, moderate | No, test runner | vitest 4, major |
| vite, vite-node, esbuild | high, moderate, moderate | No, build tool | vitest 4, major |
| browserslist | high | No, build tool | Non-major |
| nanoid | high | No, build tool | Non-major |
| postcss, postcss-selector-parser | high, low | No, build tool | Non-major |
| @sveltejs/kit | moderate | Build and dev server | Non-major |
| cookie | low | No | Non-major |

**Correction, 4 September 2026: no npm finding reaches a user, including
dompurify.** This file previously said dompurify was the one that did. That
was wrong, and the title of this item is wrong with it.

dompurify 3.4.7 is in the production dependency graph, under `jspdf ^4.2.1`
(`ui/package.json`), so it looked reachable. But jspdf only invokes it from
`doc.html()`, and `ui/src/lib/pdf.ts` never calls that: a search for
`doc.html` and for dompurify in the frontend returns nothing. The report is
built through jspdf's drawing API, which does not touch the sanitiser. Its
advisories are moderate and low in any case.

So the honest position is stronger than the one recorded here first: **zero
of the twelve npm findings are reachable in a shipped artefact.** All of
them are build and test chain. This is the DOMPurify half of SR-22 in the
brain security register, and SR-22 should be re-graded on this evidence
rather than carried as a user-facing risk.

The lesson worth keeping: presence in the production dependency graph is
not reachability. Checking the call path took one search and moved this
item from "the one that matters" to "none of them do".

Everything else is the build chain. Jura Trace is a Tauri desktop
application; vite, vitest and esbuild are not present in the installed
product. They matter for the integrity of the machine that builds releases,
which is a real concern, but it is a different concern from the one the word
"critical" suggests.

## Why the counts were not known

The three mechanisms that should have surfaced these are all silent. See
BL-CI-001, BL-CI-002 and BL-DEPS-001. This audit was run by hand.

## What would fix it

1. **`npm audit fix`, without `--force`.** It clears browserslist, nanoid,
   postcss, postcss-selector-parser, cookie, @sveltejs/kit and dompurify.
   Seven of twelve, no major versions, one branch, one review. Do this
   first and separately, so the diff is readable. Note that it is now
   build-chain hygiene rather than a user-facing fix, so it is no longer
   the urgent item in this file. **The urgent frontend work is nothing in
   this list**: see BL-DEPS-003 for the advisories that do reach users, all
   of which are in Rust and Python.

2. **Confirm the PDF still renders** after the bump anyway. dompurify is
   not on the report's call path today, but jspdf's own version moves with
   it, and the forensic report is the artefact a user hands to somebody
   else. Generate a report for a C2PA-signed image and for a
   deepfake-flagged one, before and after, and look at both. `html2canvas`
   is a jspdf peer and an audit fix that bumps it changes report
   screenshots.

3. **The remaining five** (vitest, @vitest/mocker, vite, vite-node,
   esbuild) all resolve to the same major upgrade, which is
   BL-DEPS-001 items #25, #26 and #17. Handle them there, as a decision.

## What not to do

Do not run `npm audit fix --force`. It will pull the vitest and vite
majors in behind a command that reads like a routine fix, and those two
change the build.

Do not report "one critical vulnerability in Jura Trace" anywhere public
without the qualifier. It is in the test runner. Saying otherwise would be
the same class of error the claims register exists to prevent, in the
opposite direction.
