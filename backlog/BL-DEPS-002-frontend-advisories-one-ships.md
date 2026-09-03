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
| dompurify | moderate | **Yes** | Non-major |
| vitest, @vitest/mocker | critical, moderate | No, test runner | vitest 4, major |
| vite, vite-node, esbuild | high, moderate, moderate | No, build tool | vitest 4, major |
| browserslist | high | No, build tool | Non-major |
| nanoid | high | No, build tool | Non-major |
| postcss, postcss-selector-parser | high, low | No, build tool | Non-major |
| @sveltejs/kit | moderate | Build and dev server | Non-major |
| cookie | low | No | Non-major |

**dompurify is the only finding in code that reaches a user.** It arrives
through `jspdf ^4.2.1` (`ui/package.json`), which is the production path for
the forensic report PDF. This is the DOMPurify half of SR-22 in the brain
security register.

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
   postcss, postcss-selector-parser, cookie, @sveltejs/kit and, most
   importantly, dompurify. Seven of twelve, no major versions, one branch,
   one review. Do this first and separately, so the diff is readable.

2. **Confirm the PDF still renders** after the dompurify bump. The forensic
   report is the artefact a user takes to somebody else, and it is the one
   place where a sanitiser change could silently strip content. Generate a
   report before and after and compare.

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
