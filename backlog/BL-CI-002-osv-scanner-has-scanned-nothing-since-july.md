# BL-CI-002: the weekly supply-chain scan has scanned nothing since July

**Status**: Open. Found 3 September 2026.
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
