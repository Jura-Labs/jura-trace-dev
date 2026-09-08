# BL-DEPS-002: twelve frontend advisories, and only one of them ships

**Status**: Open, partly resolved. The count is 10, down from 12, after the ui-minor-patch group merged on 8 September; none of the ten reaches a user. Step 1 is roughly half done and step 3 is deferred by policy. See "Update, 8 September 2026" at the foot of this file. Found 3 September 2026.
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

## Update, 8 September 2026

Reconciled against `ui/package-lock.json` on `origin/main` at `d0ca411d`.
The count below is from `npm audit --package-lock-only --json` run against
that lockfile on 8 September, which reads the registry and installs
nothing; the same figure appears in the `npm ci` output of the Frontend job
on CI run `34284960670` ("10 vulnerabilities (4 low, 4 moderate, 1 high,
1 critical)"). Pull request numbers are `jura-trace-dev` numbers; the
`#25`, `#26` and `#17` cited in step 3 above were `jura-archive` numbers
for the same vitest, vite and vite-plugin-svelte majors.

### What merged

- **#14** (`b18c5182`, 20:00 UTC), the `ui-minor-patch` Dependabot group,
  15 updates. This is most of step 1 done by another route: browserslist,
  nanoid and postcss are gone from the audit.
- **#15** (`40101446`, 21:44 UTC), `@testing-library/jest-dom` 6.9.1 to
  7.0.1. A test dependency; no audit effect.
- **#16**, typescript 5.9.3 to 7.0.2, **closed** without merging: SvelteKit
  2.62.0 declares `peerOptional typescript "^5.3.3 || ^6.0.0"`, so the tree
  cannot resolve, and TypeScript 7 drops `tsserver`, which `svelte-check`
  needs. An upstream change is required first. Not an advisory either way.

### The ten that remain

```
{'info': 0, 'low': 4, 'moderate': 4, 'high': 1, 'critical': 1, 'total': 10}
```

| Package | Severity | Fix | Ships to a user? |
|---|---|---|---|
| vitest | critical | vitest 4.1.11, major | No, test runner |
| vite | high | via vitest 4, major | No, build tool |
| @vitest/mocker, esbuild, vite-node | moderate | via vitest 4, major | No |
| dompurify | moderate | **non-major, available** | No (see the 4 September correction above) |
| postcss-selector-parser | low | **non-major, available** | No |
| @sveltejs/kit, @sveltejs/adapter-static, cookie | low | none clean; `npm audit` proposes a downgrade of kit to 0.0.30, which is not a fix | No |

Was 12 with 4 high; now 10 with 1 high. The three lows under
`@sveltejs/kit` are new to the list, through `cookie`. Still zero reachable
from a shipped artefact; the correction of 4 September stands.

### What remains of the three steps

1. **`npm audit fix` without `--force`.** Half done by #14. dompurify and
   postcss-selector-parser still have a non-major fix and are not applied;
   the `cookie` lows have no non-major fix and wait on SvelteKit. Two
   packages, one small branch.
2. **Confirm the PDF still renders** after the dompurify bump. Not
   recorded as done anywhere in the repository. Do it with step 1, since
   that bump moves jspdf's sanitiser even though `ui/src/lib/pdf.ts` never
   calls it.
3. **The vitest 4 group.** Deferred, not decided: the tailwindcss, vite,
   vitest and @sveltejs/vite-plugin-svelte majors are in Dependabot's
   `ignore` (`.github/dependabot.yml:97-104`) with a comment that the
   vitest entry is "the entry most worth removing soon". BL-DEPS-001 step 6
   holds the decision.
