# BL-SILENT-001: mechanisms that report success while doing nothing

**Status**: Open. Raised 4 September 2026, after five defects of one shape
were found in a single day.
**Severity**: High as a class, even though each instance is now fixed.
Patching five instances does not stop the sixth.

## The class

Five defects, unrelated by subsystem, identical in shape. In every case a
mechanism reported success, or appeared to be running, while doing nothing.

| # | What it claimed | What it did | Undetected for |
|---|---|---|---|
| 1 | Update manifest served correctly, HTTP 200, valid JSON | Put each `.sig` **URL** in the field where the signature belongs, so every update was rejected | 18 Jun to 4 Sep, 145 users |
| 2 | Rust CI job | Could not compile at all: a resource glob pointing at gitignored build output | 16 May to 4 Sep |
| 3 | Weekly supply-chain scan, appearing in Actions every Monday | Exited on a flag the scanner does not define, wrote no SARIF; `continue-on-error` made a missing report read as a clean one | 6 Jul to 4 Sep |
| 4 | CI guard "prevents regression" of stray model files | Searched the wrong directory with a glob that cannot match the real filename | since it was written |
| 5 | Test suite named `check-for-updates` | Documents in its own header that it cannot reach the updater | throughout |

Three more of the same shape found the same week: `cargo install --locked
cargo-audit || true` swallowing install failures; `pip-audit` running
before `pytest` so a vulnerability finding skipped 435 tests; and
`health.py` computing an `ffmpeg_available` probe, discarding it, and
carrying a comment claiming the capability "is still reported truthfully".

## Four root causes, not five

Two of these were visible from the original five defects. The third and
fourth were added as later work found them, and both sections say when.

**Cause A: effective CI cadence was zero.** `ci.yml` triggered only on
`pull_request`, and a solo developer committing straight to `main` means it
never ran. Defects 2 and 3 are both invisible only because of this.

**Cause B: success was defined as "the step exited 0", not "the expected
artefact exists and is valid".** Defects 1, 3 and 4 all pass that weaker
test. So does a scanner that scans nothing and a guard that matches
nothing.

Close those two properties and most of the class closes with them. Causes C
and D below are narrower but not covered by closing A and B.

## A correction to the obvious fix, from this repo's own evidence

The natural prescription is a scheduled run plus a deadman check, on the
reasoning that GitHub disables cron workflows after 60 days of repository
inactivity, so the schedule itself can lie by omission.

**That is not what happened here.** The OSV cron fired every single Monday
from 22 June to 31 August, which is 73 days after the last commit on 19
June. It was never disabled. It ran faithfully, ten times, and scanned
nothing on every one of them.

So the deadman check is still worth having, but for a better reason than
the one usually given, and with a different assertion. **Check for a recent
*successful* run, not a recent run.** There were zero successful OSV runs
in that window and eleven scheduled ones. A freshness check on runs would
have said everything was fine; a freshness check on *successes* would have
screamed from the first week.

That is Cause B applied to the schedule itself.

## Twelve more, found by hunting for the shape

A systematic sweep on 4 September found the class is much wider than the
five. The worst are written up separately; the rest are inventoried here so
they are tracked rather than rediscovered.

**Written up elsewhere.** Windows signatures invalidated after verification,
and the updater UI mapping 404 to "up to date" (both added to BL-REL-002).
The exported PDF asserting OCSP and CRL checks that never run
(BL-CLAIM-003).

**Still to file, ranked.**

| # | What it claims | What it does | Since |
|---|---|---|---|
| 1 | A corrupt C2PA manifest is reported | `verify/pipeline.rs:836` uses `.ok().flatten()`, so a parse **error** becomes `None`, indistinguishable from "no manifest". Trust score shows "No provenance data" and the detector lists as run. For a provenance product, "failed to check" reads as "checked, unsigned" | 2 Mar |
| 2 | The audit chain-of-custody check is load-bearing | `db.rs:1393` documents that NULL-hash rows return false; the code `continue`s past them, so nulling both hash columns on a tampered row passes the restore check at `lib.rs:3317`. Deleting trailing rows also passes. Separately, every production `log_action` and `insert_verification` call is `let _ =`, so entries may never be written at all | 23 Mar |
| 3 | The AI-watermark detector checked the image | `deepfake.py:293-356` returns `[]` on ImportError, and `imwatermark` is deliberately absent from every runtime bundle. `watermarks: []` is indistinguishable from "checked, none found", and unlike the GBM and UnivFD paths there is no availability flag | 21 May |
| 4 | Six GBM feature extractors measured the image | `deepfake.py:1568, 1597, 1627, 1656, 1691, 1726` return camera-plausible **constants** on exception, injecting "real camera" evidence and biasing toward authentic, unlogged. The Sprint-29 extractors get this right and return NaN | 20 Mar |
| 5 | The frozen-sidecar smoke test passed | `scripts/smoke_test_frozen_sidecar.py:289-307` skips any capability reporting false, then prints "PASSED: all N mapped endpoints" where N is the dictionary size rather than the number probed. The all-capabilities-false failure it was written for passes | |
| 6 | `latest.json` is published with the release | `release.yml:1312` runs `publish-release` on matrix **failure**, which is the normal macOS flow, so the primary endpoint can go live without `darwin-aarch64` and with SHA256SUMS missing the DMG the release body tells users to verify. The SCP step is skip-silent and `continue-on-error` | |
| 7 | The macOS release channel has CI's guards | `scripts/build-local-mac.sh:246-248` copies models with `2>/dev/null \|\| warn` then reports ok against whatever stale files are already present, and has no empty-`.sig` gate where CI has one. This is the real release channel for macOS | |
| 8 | The licence appendix is complete | `scripts/generate-licenses.sh:56-63` swallows failures across the Python ecosystem and prints "Done.", shipping a legally incomplete appendix | |

Lower blast radius, noted: the FP-report modal fabricates `mock-fp-*`
success on real database failures (`api.ts:520-532`, part of a wider
pattern there of rendering backend errors as plausible empty data);
re-sign ingredient preservation is `let _ =` at `c2pa.rs:599`, undoing the
intent of the 22 May audit fix; watchlist error events vanish
(`monitor_scheduler.rs:388-407`); no Playwright spec runs in any workflow,
and the "nightly workflow" that `ci.yml:117` refers to does not exist;
`db.rs:359` swallows migration errors.

**Correction, 8 September 2026.** The requirements sync guard was listed
below as checked and clean. That was right about the drift it was written
for, a package name present in one manifest and missing from another, and
wrong about versions. `scripts/check_requirements_sync.py:28` says it
compares names only, and it never opens `requirements.lock` at all, so it
passes a change that makes CI test one version while the release ships
another. That is Cause B of this very item. See BL-DEPS-004.

**Checked and clean**, which is worth recording so it is not re-hunted: the
feature flags genuinely gate, the model SHA-256 pin verifies against both
copies, `describe_image` and
`claim_checker` report honest failures, and the signing gates in
`release.sh` and the macOS build's Phase 5b and 6.5 fail loudly.

## Cause C, added 4 September 2026: verification ordered before the artefact is final

Drawn from the sweep above rather than from the original five. That is the
Windows signing fault and the `latest.json` publication fault, and it is not
covered by cadence or by prove-it-ran assertions. The rule it implies is
that a gate must run on the bytes that ship, last, after every mutation.

## Cause D, added 5 September 2026: absence of an error string read as success

Found in this session's own tooling, which is why it is written down rather
than quietly fixed.

A shell check was written to de-risk the merge queue by testing whether the
three branches that all rewrite `release.yml` conflict with each other. It
reported all three pairs clean. They were clean, but the check had not
established that: `set -- $pair` does not word-split in zsh, so
`git merge-tree` received one malformed argument, failed, printed nothing to
stdout, and the `grep -q '^<<<<<<<'` that followed found no conflict marker
in empty output and therefore reported no conflict.

The pattern is `cmd | grep -q BADNESS || echo fine`, and it returns "fine"
for both of the two very different situations where BADNESS is absent
because the thing is good, and where BADNESS is absent because the command
never ran. It is the same shape as the `>/dev/null 2>&1` that hides the
macOS codesign failure and surfaces only "sort: Broken pipe", already
recorded as a known trap in this repository.

The rule this implies, and it applies to CI steps as much as to a throwaway
loop: **a check that greps for failure must first assert that the command
that produces the output actually succeeded.** Exit status first, pattern
second. In practice that means `set -o pipefail`, capturing output to a
variable and testing the command's own status before inspecting it, rather
than piping straight into `grep`.

Worth a specific note for anyone writing these checks on this machine: the
default shell here is zsh, which does not word-split unquoted parameter
expansions the way bash does. A snippet that is correct in bash can be
silently wrong when pasted into a zsh session, and it fails in the
direction of looking fine.

## Three more, 9 September 2026, found by verifying a build by hand

The local v1.1.0 macOS build succeeded and printed a green tick at every
phase. Opening its artefacts by hand rather than reading the ticks found the
DMG sound and the updater archive not, and the script unable to tell. All
three were fixed in PR #49; they are recorded here because each is an
instance of a cause already on this list.

**The release gate that never checked anything (Cause C, and the empty
set).** Phase 6.5 of `scripts/build-local-mac.sh` was written after rc.29
to catch mis-signed nested binaries before notarisation. It scanned
`$APP_PATH/Contents/Resources/sidecar-bundle`. Phase 6 removes that `.app`
once it has packaged it, so `find` ran against a missing directory with
stderr silenced, counted zero files, and the gate printed
"All 0 nested Mach-O binaries in final .app signed". Two faults in one
line. It was not verifying the bytes that ship, which is Cause C; and it
had no floor, so an empty scan read as a clean scan, which is the
prove-it-ran gap of section 3 below in its purest form. It now extracts
the updater archive, the artefact the release exists to deliver, and dies
if it checked nothing.

**The warning nobody read (Cause C again, by way of section 7).** Phase 6
ran `cargo tauri bundle --bundles dmg,updater`. On macOS `updater` is not a
bundle target; the archive is a by-product of `app`. Tauri said so in every
build, as a Warn line: "no updater-enabled targets were built. Please
enable one of these targets: app, appimage, msi, nsis." The archive on
disk was the one Phase 1 wrote, before any of the later signing, and the
comment above the phase said the opposite. A gate downstream of an ignored
warning is verifying the wrong artefact by construction.

**The gate that failed a good build (Cause D, mirrored).** The rewritten
Phase 6.5 first read any stderr from `codesign --verify --deep --strict
--verbose=2` as failure. With `--verbose`, codesign prints "valid on disk"
and "satisfies its Designated Requirement" on success, so the gate refused
a build whose archive was, by independent check, valid. Cause D is the
absence of an error string read as success; this is the presence of a
non-error string read as failure, and the rule is the same one: **exit
status first, output second.** A gate that fails good builds trains the
same reflex as one that passes bad ones, which is to stop reading it.

Two smaller things from the same day belong with Cause D's note on zsh.
`cc $CFLAGS` with an unquoted variable passed one argument to the compiler
under zsh and produced an "invalid integral value" error, and a `for` loop
over test variants counted "0 failures" because its `cd` had failed and
nothing ran. Both looked like results. Neither was.

## One more, 11 September 2026, and it is Cause D with the sign flipped

The Windows updater live leg, `updater-live-leg.yml`, spent four runs
observing nothing and saying nothing about why. The port answered, the MSI
installed, the application's own page was listed on the debugging port three
seconds after launch and stayed listed for the full ten minutes, and the
verdict it wrote was a column of nulls: `dom_read` false, `button_pressed`
false, `last_stage` null, `page_url` null.

The defect underneath was a PowerShell parsing fault. The probe's JavaScript
is built as a comma separated array of single quoted lines, and one line
interpolates a value with `'text' + $env:EXPECTED + 'text'`. In an array
literal the comma binds tighter than the plus, so that is the array so far,
plus a scalar, plus the rest of the array, and plus on an array appends. One
element became three, the join put newlines between them, and a JavaScript
string literal cannot contain a raw newline. Every evaluation since had come
back `Uncaught SyntaxError`. It belongs with Cause D's zsh note: a construct
that is correct in one language's array semantics is silently wrong in
another's, and it fails in the direction of looking like something else.

What made it cost four runs rather than one is the part that belongs in this
file. The helper that talks to the browser returned a bare `$null` for four
completely different outcomes: could not connect, sent and heard nothing
back, read forty frames and none was the reply, and the page threw while
being evaluated. The caller skipped past all four without a word. **A page
that was evaluated and came back empty was recorded identically to a page
that could never be evaluated at all.** That is the mirror of Cause D. Cause
D is the absence of an error string read as success; this is the absence of a
result read as an absence of content.

Two things fixed it and both are worth copying. The helper now always returns
a record saying how far it got, and the caller prints one line per distinct
outcome with the second it happened, so a run that fails while saying what it
saw is worth more than three that fail silently. And the expression is
asserted to be well formed before the application is even installed, which is
a prove-it-ran assertion in the sense of section 3 below, applied to an input
rather than to an output.

Fixed in PRs #81 and #82. Run 34581569486 is the first green one and observed
a pristine v1.0.0 fetch and install v1.1.0 from the live service in 119
seconds.

## The plan

Ordered by defects prevented per unit of cost. Items 1 to 4 belong in the
v1.1.0 window, and total roughly two days.

### 1. Trigger and cadence

`push: branches: [main]` is already added on `fix/ci-gate`. Add a weekly
`schedule` alongside it. At roughly 20 minutes a run at 1× on ubuntu, and
about twenty pushes plus four crons a month, that is around 500 charged
minutes against 3,000 included. This alone surfaces a defect like number 2
within a week of introduction instead of four months.

Then the deadman, per the correction above: assert the last **successful**
`ci.yml` run is under eight days old. The natural home is the brain's
`verify` skill, which already probes live systems read-only through
`bin/status-check.sh` and `bin/sr-verify.sh` and files nothing when nothing
changed. Zero CI cost. **Note that `bin/` is outside the Trace instance's
write scope**, so this is a request to Paul or the brain session, not
something this repo can land alone.

### 2. Guard self-tests

Every guard gets a planted failing fixture, and CI runs the guard against
the fixture expecting **red** before running it against the repository
expecting green. A `scripts/ci/fixtures/bad/` directory with a stray file
the hygiene guard should catch, and a wrapper that fails the job if the
guard *passes* on it.

This is the mutation test the QA plan asked for, automated at near-zero
cost and running inside every green build rather than as a separate red
job. A glob that cannot match anything then fails on day one, which is
exactly defect 4.

### 3. Prove-it-ran assertions

- **cargo-audit**: `|| true` already removed on `fix/ci-gate`. Add
  `--json` output and assert the dependency count is greater than zero, so
  an audit that inspected nothing cannot pass.
- **pip-audit**: same shape, `-f json -o` then assert it enumerated
  packages.
- **Manifest generation**: beyond the completeness assertion already
  landed, assert the served `latest.json` carries exactly the expected
  platform keys, that every signature passes `looksLikeSignature`, and that
  none contains `://`.
- **OSV**: the SARIF non-empty assertion has landed. Remove
  `continue-on-error` from the scan step itself and keep it only on the
  upload, with a comment saying why.

### 4. Scanner canaries

A pinned, known-vulnerable fixture lockfile, and a weekly assertion that
OSV and pip-audit each report at least one finding against it. This catches
the residue of defect 3 that the SARIF assertion does not: a scanner that
runs, produces a well-formed report, and detects nothing.

### 5. Extract the manifest generator, after v1.1.0

Defect 1 lived in inline `github-script` inside `release.yml`, a file only
exercisable by a 72-minute release run. Move it to
`scripts/release/generate-update-manifest.mjs` with unit tests, including
negative fixtures for the sig-URL-in-signature case and a missing platform.
Then the failure that stranded 145 users is caught by a twenty-second test
rather than on release day. Fold this into BL-TEST-001 if that work touches
the file anyway.

### 6. Cadence for the expensive builds

Nightly Windows is unaffordable: thirty runs at 144 charged minutes is
4,320 a month on its own. Split by what is platform-dependent. The manifest
and updater logic is not, so run BL-TEST-001's Linux updater end-to-end
**weekly**, and the Windows installer end-to-end **monthly and on every
tag**. Total added spend across the whole plan is roughly 800 to 900
minutes a month, comfortably inside the allowance.

### 7. The comment problem

Defect 1 was caused by a false comment that the implementation followed.
Defect 4's comment claimed it "prevents regression". Defect 5's comment
honestly disclaimed its own coverage and nobody read it.

Two rules, both review discipline rather than tooling. A comment claiming a
guarantee must name the assertion that enforces it, which items 2 and 3
make possible. And a disclaimer like defect 5's must carry a ticket ID
rather than being prose, because prose gaps are not tracked and tickets
are.

## What not to do

- **Nightly builds.** Budget-breaking, and weekly would have caught
  everything here.
- **A scheduled deliberately-failing job.** A routine red run trains alarm
  fatigue. Fixtures inside green runs prove the same thing silently.
- **Branch protection and mandatory pull requests.** Solo developer;
  self-approved PRs are ceremony. Push-to-main triggers give the coverage
  without the ritual.
- **Attestation frameworks or policy engines.** Wrong scale entirely.
  `actionlint` in CI is proportionate; anything heavier is not.
- **Building tooling for the `continue-on-error` policy.** There are two
  instances. A grep and a comment rule suffice.

## Update, 8 September 2026

Three more entries for the class, none of which changes the causes or the
plan above. Pull request numbers are `Jura-Labs/jura-trace-dev` numbers.

**Another instance fixed: the classifier loader's bare `except`.**
`sidecar/app/services/deepfake.py:255`, `_load_classifier`, caught every
exception from `joblib.load` and returned `None`, and the caller fell back
to the heuristic score. Every verdict was still produced, so the only
evidence of a disabled classifier in production would have been a shift in
the verdict distribution. It surfaced on 8 September because the
numerical-stack drift test (#37) showed scikit-learn 1.9.0 cannot unpickle
the shipped GBM at all (`ModuleNotFoundError: No module named '_loss'`) and
the loader said nothing. #38 (`c718ca25`) replaces the bare `except` with
`logger.exception(...)` at `deepfake.py:278-295`, once per process, with
two tests in `sidecar/tests/test_deepfake.py`
(`test_unpickle_failure_returns_none_and_logs_exception`,
`test_failure_is_logged_once_not_per_image`). Cause B, in the shape of
"degrade gracefully and say nothing". Worth adding to the ranked table as a
sibling of item 3 (the watermark detector's `[]` on ImportError) and item 4
(the constant-returning extractors); it is the same file and the same
instinct.

**A mirror instance fixed: cancelled CI runs manufacturing failure.**
`ci.yml` had `cancel-in-progress: true` for every ref including `main`.
When four merges landed within fourteen minutes on 8 September, each run
cancelled the one before it, and three of the cancelled runs showed a red X
against `cargo test` that was the cancellation, not a test result (jobs API:
`conclusion: cancelled` on job and step). That is BL-TEST-002's direction,
a signal reporting failure while nothing was wrong, produced by the same
gate this item is about. #34 (`bd534866`) sets
`cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`
(`ci.yml:34`). One residual, recorded in BL-CI-001 for the plan: GitHub
still cancels the older *pending* run in a concurrency group when a third
run arrives, so two `main` runs (`9a177fd5`, `3ca2896f`) were cancelled
with zero jobs after #34 merged. A per-commit group on `main` closes it.

**"What not to do" has been partly overtaken.** The list above says no
branch protection. Branch protection on `main` went live on 8 September
with required status checks Rust, Frontend, Python, Repo hygiene and Scan
lockfiles, no required reviews, `strict: false` and `enforce_admins: false`
(`gh api repos/Jura-Labs/jura-trace-dev/branches/main/protection`). That
is the half of branch protection the paragraph was not arguing against:
no review ceremony, and an administrator can still push. It does add one
thing the push trigger alone did not, which is that a merge cannot land
while the gate is red. The advice stands as written for mandatory reviews.

For the plan: item 3's OSV line ("remove `continue-on-error` from the scan
step") is not done, `osv-scanner.yml:59` still carries it, and its
cargo-audit and pip-audit count assertions are not done either. BL-CI-001
and BL-CI-002 closed on 8 September on their own fix sections; those three
assertions remain here.

## Update, 10 September 2026: the pre-commit hook was never a gate

The clearest instance yet, and it was in the tooling meant to catch the
others. `.githooks/pre-commit` runs five checks. Four of them are shaped

```sh
(cd ui && npx svelte-check --threshold error 2>&1 | tail -1) || {
    echo "ERROR: svelte-check found errors."
    exit 1
}
```

A shell pipeline exits with the status of its last command. That command is
`tail`, which succeeds on any input, so `cargo check --all-targets`,
`cargo clippy -D warnings`, `svelte-check` and `vitest` could all fail and
the hook would print "Pre-commit: all checks passed" and let the commit
through. Only `cargo fmt --check`, the one check not piped anywhere, ever
blocked anything. The file has been in this shape since it was written, so
every commit made through it was unverified by four of its five checks.

Found while an agent drafting the v1.1.0 changelog noticed the hook print
`1 ERRORS` from a mid-run svelte-check line and report a pass in the same
breath. Reproduced in one line:

```sh
sh -c 'set -e; (false 2>&1 | tail -1) || { echo "guard fires"; exit 1; }; echo "all checks passed"'
# prints: all checks passed
```

Fixed by `set -o pipefail`, which needs bash rather than `/bin/sh`, so the
shebang changes too. Proven both ways rather than argued: a deliberately
broken TypeScript file under `ui/src/lib/` makes the hook exit 1 with
"ERROR: svelte-check found errors", and the same hook on a clean tree exits
0. The comment block at the top of the file carries the one-line
reproduction so the next person does not have to rediscover it.

Two things this does not fix, both listed here rather than assumed away:

- CI is unaffected. `ci.yml` runs each tool as its own step with no pipe, so
  the assertions that actually protect `main` were always real. The hook was
  a local convenience that lied; the remote gate did not.
- It says nothing about how long the hook has been passing over real
  failures, because a hook that does not fail leaves no record. The tree is
  clean today (`svelte-check` reports 580 files, 0 errors; clippy and the
  vitest suite pass), so nothing appears to have slipped through, but that
  is an observation about now, not a reconstruction of the past.

This is cause A in the list above, a guard whose success condition is not
the thing it claims to check, and it argues for guard self-tests (plan item
2) in the one place the plan did not think to look: the guards' own
plumbing.
