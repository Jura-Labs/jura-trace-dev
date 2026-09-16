# BL-CI-002: the weekly supply-chain scan has scanned nothing since July

**Status**: Closed 8 September 2026. All four fix items are on `main` and the scanner has produced a SARIF file on every push since. What remains (a package-count assertion, and `continue-on-error` on the scan step) is the release plan's gate item 5, not this item. See "Update, 8 September 2026" at the foot of this file. Found 3 September 2026.
**Raised**: 3 September 2026
**Severity**: High. The scan appears in the Actions list every Monday and
has found nothing for two months, because it never ran. An absent finding
has been reading as a clean result.

## What is wrong

`.github/workflows/osv-scanner.yml` passes a flag the scanner no longer
defines. From the run of 31 August 2026:

```
Incorrect Usage: flag provided but not defined: -skip-git
```

The scanner then prints its usage text and exits non-zero. Because the step
carries `continue-on-error: true`, the job continues to the upload step,
which fails differently:

```
##[error]Path does not exist: osv-results.sarif
```

No SARIF file was written, because no scan ran. The offending arguments are
at `.github/workflows/osv-scanner.yml:41-47`:

```yaml
          scan-args: |-
            --recursive
            --skip-git
            --format=sarif
            --output=osv-results.sarif
            ./
```

`--skip-git` was valid for osv-scanner v1. The action is pinned at
`google/osv-scanner-action/osv-scanner-action@v2.0.0`, and under v2 the
`scan source` subcommand does not define it.

## How it showed up

Every scheduled run since at least 6 July 2026 has the conclusion
`failure`: 6 and 13 and 20 and 27 July, 3 and 10 and 17 and 24 and 31
August. Nine consecutive weeks. The failure was assumed to be the SARIF
upload, which is a familiar and harmless-looking failure on repositories
without code scanning enabled. It is not that. The upload fails because the
scan never produced a file.

## Why it matters

The repository's supply-chain position is currently unmeasured by this
tool. The scanner was added as defence in depth alongside `pip-audit` and
`cargo-audit`, and the header comment in the workflow explains why: "any
one feed could miss an advisory". Since July, all three have been silent,
because `pip-audit` and `cargo-audit` live in `ci.yml`, which has not run
since April either (BL-CI-001).

What is actually known, checked by hand on 3 September 2026 with `npm
audit` on `ui/`, is in BL-DEPS-002: one critical and four high findings in
the frontend chain. None of them reached anybody through this workflow.

`continue-on-error: true` on the scan step is what made this quiet. It was
put there so an advisory would not block a merge, which is reasonable. The
cost is that a scanner which cannot start looks the same as a scanner which
found nothing.

## What would fix it

1. **Remove `--skip-git`.** Under v2, git repository scanning is not the
   default behaviour that flag was suppressing. Check the output of a
   manual `workflow_dispatch` run afterwards, and confirm the SARIF file
   exists before trusting the next scheduled run.

2. **Fail the job if the SARIF file is missing.** Keep
   `continue-on-error: true` on the scan itself, so that a real advisory
   does not block work, but add a step that errors when
   `osv-results.sarif` was not written. The distinction to encode is
   "found nothing" versus "did not run".

3. **Confirm the SARIF upload target exists.** `jura-archive` is private.
   Code scanning upload on a private repository needs GitHub Advanced
   Security. If it is not available on this plan, the SARIF has nowhere to
   go and the results should be attached as a build artefact and read
   there instead, or the scan output switched to `table` and read in the
   log.

4. **While in the file**, note that `actions/checkout@v4` and
   `github/codeql-action/upload-sarif@v3` both target Node 20, which
   GitHub has deprecated and is now force-running on Node 24. That is the
   same class of drift as BL-DEPS-001 items 7, 8, 10, 11 and 13.

## What not to do

Do not silence the weekly failure notification without fixing the cause.
The notification is the only thing that was still working.

## Update, 8 September 2026

Reconciled against `origin/main` at `d0ca411d` and the `OSV-Scanner`
workflow runs on `Jura-Labs/jura-trace-dev`.

### The fix items, checked

1. **Remove `--skip-git`.** Done. `.github/workflows/osv-scanner.yml:54-58`
   passes `--recursive`, `--format=sarif`, `--output=osv-results.sarif`,
   `./` and nothing else. Landed in `d605acd6` (jura-archive PR #27,
   5 September). The SARIF exists on every push run since: run
   `34284960681` on `d0ca411d` (22:15 UTC) logged "SARIF written: 773035
   bytes" and scanned seven lockfiles for 1,451 packages in total
   (`agents/requirements.txt` 5, the worker's `package-lock.json` 91,
   `sidecar/requirements-build.txt` 23, `requirements-ci.txt` 23,
   `requirements.txt` 20, `src-tauri/Cargo.lock` 921,
   `ui/package-lock.json` 368).
2. **Fail the job if the SARIF is missing.** Done. `osv-scanner.yml:64-72`
   exits 1 on an absent or empty `osv-results.sarif`.
3. **Confirm the upload target exists.** Done both ways. The "Upload SARIF
   to code-scanning" step concluded `success` on run 34284960681, and the
   SARIF is also kept as a build artefact (`osv-scanner.yml:86-92`) so the
   result survives if the upload ever stops working.
4. **Node 20 actions.** Every action in both workflows is now pinned to a
   commit SHA (`d4097532`, 6 September) and the Repo hygiene job fails on
   an unpinned `uses:` (`ci.yml:382-395`). `github/codeql-action/upload-sarif`
   still logs the Node 20 deprecation on each run; its bump is one of the
   five in Dependabot #13, which is open.

### Also changed by #34 (`bd534866`)

The `pull_request: branches: [main]` filter is gone (`osv-scanner.yml:19-23`),
for the reason given in the comment there: a pull request against any other
base got no scan, and a missing scan reads as a clean one.

### What remains, and where it lives

The plan's automated gate item 5 in `docs/release/v1.1.0-plan.md` asks for
OSV "verified by a nonzero package count in the run log rather than by the
job being green". That is **not implemented**. The scanner prints
"Scanned ... found N packages" per lockfile, and nothing asserts on those
lines; the only assertion is the SARIF size at `osv-scanner.yml:64-72`.
Related, BL-SILENT-001's plan item 3 asks for `continue-on-error` to come
off the scan step and stay only on the upload. It is still on the scan step
at `osv-scanner.yml:59`. On run 34284960681 the scanner exited 1 (findings
present) and the job was green because of that line. Both belong to the
plan and to BL-SILENT-001, not to this item, whose own fix section is met.

No scheduled run has fired in `jura-trace-dev` yet. The repository was
created at 11:57 UTC on 8 September, a Tuesday; the first cron is Monday
14 September at 03:17 UTC. Fix item 1 said to confirm the SARIF exists
before trusting the next scheduled run: it has existed on every one of the
ten push runs on `main` since the repository was created.

Resolved: 2026-09-08 `bd534866` (#34), on top of `643c4dd3` (jura-archive
PR #27, 2026-09-05). `--skip-git` removed, SARIF-present assertion, upload
confirmed working, artefact kept, actions SHA-pinned, `branches` filter
removed.
