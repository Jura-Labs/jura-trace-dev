# BL-TEST-003: 314 end-to-end tests, and nothing runs them

**Status**: Partly fixed 5 September 2026 — 296 of 314 now run in CI. The
18 visual-regression tests remain unrun on any machine but a Mac.
**Severity**: Medium. No defect is known to have escaped through this gap,
but the gap is total: the largest test suite in the repository has been
providing zero assurance since it was written.
**Raised**: 5 September 2026, after a local run reported 308 failures.

## How it surfaced

A local `npx playwright test` reported **308 failed**. That looks like a
catastrophic regression from the change under test. It was not: the
Playwright browser binary was not installed on the machine, and every
failure read `Executable doesn't exist at .../chrome-headless-shell`.

Checking the actual error rather than assuming is the only reason this is a
backlog item instead of a day spent bisecting a working change.

## Four compounding reasons the suite was dead

**1. No workflow runs it.** `ci.yml` explicitly skipped Playwright, with a
comment saying the tests were "better suited to a dedicated nightly
workflow". That workflow was never written. The repository contains
`ci.yml`, `osv-scanner.yml` and `release.yml` and nothing else.

This is the same shape as the "nightly workflow" reference already noted in
BL-SILENT-001: a comment that defers work to a mechanism which does not
exist reads, to every later reader, as though the work is covered.

**2. The browser install is documented nowhere.** Not in `README.md`, not
in `CONTRIBUTING.md`, not in the `Makefile`, not in `CLAUDE.md` — which
nonetheless listed `npx playwright test` as a development command. On a
fresh checkout that command fails every test, 100% of the time, for a
reason that has nothing to do with the code.

**3. Every visual baseline is macOS-only.** All 18 committed snapshots are
`*-darwin.png`. Playwright puts the platform in the snapshot filename, so
an Ubuntu runner looks for `*-linux.png`, finds nothing, and fails. "Just
add it to CI" was therefore never a one-line change, which is probably why
it kept not happening.

**4. The pull-request template asked for a guarantee nobody could give.**
`.forgejo/pull_request_template.md:42` asks contributors to tick "Existing
Playwright e2e tests pass on the affected surfaces". No contributor could
have substantiated that without discovering all of the above first.

## What was fixed on 5 September

The e2e suite now runs in `ci.yml`'s Frontend job, with
`npx playwright install --with-deps chromium` ahead of it.

**296 of 314 tests run.** The nine visual tests (eighteen across the two
browser projects) are **deselected** with `--grep-invert "visual:"`, so
they report as not-run.

They are deselected rather than ignored on purpose. `--ignore-snapshots`
would make every `toHaveScreenshot` assertion pass without comparing
anything, which is precisely the failure mode the workflow exists to catch.
A test that cannot run should say so.

Cost is negligible and worth stating, because it is the argument for having
done this sooner: the suite finishes in **under thirty seconds** against a
plain Vite server, with no Rust build and no sidecar. Against the Rust
job's forty-five minutes, this is the cheapest coverage in the repository.

## What is still open

**Generate and commit Linux baselines**, so the 18 visual tests run in CI
too. Playwright supports per-platform snapshots natively; the baselines
just have to be produced once on Linux and committed alongside the darwin
ones. The mechanical route is a one-off CI run with `--update-snapshots`
and the results downloaded from the artifact.

Worth a moment's thought first, though, on what the visual tests are for.
Cross-platform font rendering differs, so Linux baselines will not be
pixel-identical to the macOS ones and should not be expected to be. Two
independent baselines is the correct outcome, not a problem to engineer
away with a tolerance high enough to hide real regressions.

**Decide what the PR template should say.** Either the checkbox becomes
"CI runs these" and stops asking the contributor, or it stays and points at
the documented install command. Asking for an unverifiable attestation is
worse than asking for nothing.

## A smaller finding, recorded because it steers every session

`CLAUDE.md`'s test counts had drifted on every single line:

| Claimed | Actual |
|---|---|
| 148 Playwright e2e | 314 |
| 15 Vitest | 182 |
| 492 Rust lib | 568 |
| 15 Rust API integration | 20 |

Corrected in the same commit. The file is the first thing every session
reads, so wrong numbers there are not a documentation nicety: they set the
baseline against which somebody decides whether a suite looks healthy.
