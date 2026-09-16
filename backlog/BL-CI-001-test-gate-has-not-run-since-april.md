# BL-CI-001: the test gate has not run since 30 April, and nothing noticed

**Status**: Closed 8 September 2026. Every item in "What would fix it" is on `main` and has run there. See "Update, 8 September 2026" at the foot of this file. Found 3 September 2026.
**Raised**: 3 September 2026
**Severity**: High. This is the item that blocks BL-DEPS-001. Twenty
dependency updates cannot be merged with confidence because there is no
green build to merge them behind, and the June release work was never
tested by CI at all.

## What is wrong

Two separate problems that combine into one hole.

**The gate only fires on pull requests.** `.github/workflows/ci.yml:3-5`:

```yaml
on:
  pull_request:
    branches: [main]
  workflow_dispatch:
```

There is no `push` trigger. Every commit that reached `main` directly
bypassed CI. The last eleven commits, `bdee0a5` through `aa6f562` (16 to 19
June 2026), all landed straight on `main` during the release scramble, and
CI saw none of them.

**What partly covers the gap, and what it misses.** `.githooks/pre-commit`
runs on this machine (`core.hooksPath` is set) and does a real job:
`cargo fmt --check`, `cargo check --all-targets`, `cargo clippy -D
warnings`, `svelte-check` and the vitest unit tests. It passed on the
commit that added this file. So the June work was not unchecked; it was
checked by a hook that runs on one laptop and that nothing enforces.

What the hook does not run is the part that matters most:

- **`cargo test`.** All 596 Rust tests are outside it.
- **The Python sidecar, entirely.** No ruff, none of the 435 tests in
  `sidecar/tests/`. The sidecar is where every forensic detector lives.
- **Playwright.** The ten end-to-end specs never run anywhere.

A contributor without `core.hooksPath` set has no checks at all, and CI is
the only thing that would catch that.

**The one job that does run, fails.** The last CI run was on 30 April 2026,
against `dependabot/npm_and_yarn/ui/vitest-4.1.2`. Rust, Frontend and Repo
hygiene passed. Python failed, at the step named `Lint`
(`.github/workflows/ci.yml:151-152`):

```yaml
      - name: Lint
        run: cd sidecar && ruff check .
```

Reproduced locally on 3 September 2026 with ruff 0.14.2:

| Rule | Count | What it is |
|---|---|---|
| E402 | 35 | module import not at top of file |
| F401 | 26 | unused import |
| F841 | 4 | unused variable |
| F811 | 2 | redefined while unused |

67 errors, 27 of them fixable by `ruff check --fix`.

## How it showed up

Triaging the dependabot backlog. Every open dependency PR shows either an
unknown or an unstable merge state, and the reason is the same: the only
check that would confirm a bump is safe has been red since April, so no
bump can be merged on evidence rather than hope.

## Why it matters more than a lint failure looks

**The linter is not pinned.** `.github/workflows/ci.yml:148-149` runs
`pip install ruff` with no version. A ruff release that adds or tightens a
rule turns the build red with no change to this repository. That is what
appears to have happened: 35 of the 67 findings are E402, a rule about
import placement that is common in files which set environment variables
before importing a heavy module. The code did not break. The tool moved.

**A red gate that nobody can fix trains people to route around it.** The
June release did route around it, by committing to `main` where CI does not
run. v1.0.0 shipped on 18 June 2026 with no CI run against any of the code
in it, and with the Rust and Python test suites unexecuted by anything.

**The register already knows.** This is SR-18 in the brain security
register, recorded as "Trace ci.yml gates nothing (last ran 30 Apr,
failing)". This file is the repo-side write-up of the same fact, with the
cause identified.

## What would fix it

Roughly in order of cost.

1. **Pin ruff** to an exact version in `ci.yml`, so the linter cannot
   change under a build that has not changed. Pin it to the version the
   fixes below are made against.

2. **Clear the 67 findings.** 27 are automatic. The 35 E402 cases need a
   judgement per file: either move the import, or add a scoped `# noqa:
   E402` with a one-line reason where the import genuinely must follow a
   side effect. Do not blanket-ignore E402 across the sidecar; that hides
   the cases where the ordering is accidental.

3. **Add a `push` trigger on `main`**, so that work which lands directly,
   as release work does, is still tested. The gate is worth little if the
   only path it watches is the one nobody used in June.

   `ci.yml` already runs `cargo test` (line 81) and `pytest tests/ -v`
   (line 172). Those are the two suites nothing else runs. Repairing the
   trigger is what turns them back on.

4. **Then** re-run the dependency PRs against it. See BL-DEPS-001.

## What not to do

Do not delete the Lint step or set `continue-on-error: true` on it to get
the build green. A green build that checks nothing is worse than a red one,
because it is believed. The same reasoning applies to BL-CI-002, where an
`continue-on-error: true` is exactly why a broken scanner went unnoticed
for two months.

Do not fix the lint findings in the same branch as any dependency bump.
Keep the gate repair separate, so that when a bump does break something the
diff that broke it is obvious.

## Update, 8 September 2026

Reconciled against `origin/main` at `d0ca411d` and the GitHub Actions
history of `Jura-Labs/jura-trace-dev`, after the fourteen merges of
8 September. Pull request numbers below are `jura-trace-dev` numbers unless
marked `jura-archive`; the two repositories number from 1 independently and
collide (BL-DEPS-003's "PR #28" is the jura-archive openssl bump, not the
cargo-audit clearance below).

### The four fixes, checked one by one

1. **Pin ruff.** Done. `.github/workflows/ci.yml:277` runs
   `pip install ruff==0.14.2`, with the reason in the comment above it.
   Landed in `d605acd6` (jura-archive PR #27, merged `643c4dd3`,
   5 September).
2. **Clear the 67 findings.** Done. Same commit, plus `9f0702a7` for
   `ruff format`. The Python job on `main` run `34284960670` (22:15 UTC,
   `d0ca411d`) passes both `ruff check .` (`ci.yml:280`) and
   `ruff format --check .` (`ci.yml:283`).
3. **Add a `push` trigger on `main`.** Done. `ci.yml:13-14`. `cargo test`
   runs at `ci.yml:134` and `python -m pytest tests/ -v` at `ci.yml:309`,
   and both ran green on every successful push run listed below.
4. **Re-run the dependency PRs against it.** Handed to BL-DEPS-001, which
   records what merged.

### What the gate actually did on main, 8 September

| Run | Commit | UTC | Result | Why |
|---|---|---|---|---|
| 34236052149 | `abc219a6` | 14:05 | failure | `cargo-audit`: "error: 14 vulnerabilities found!" over 898 crates. A true finding, cleared by #28 and #29 |
| 34257110330 | `cc0cbc8a` (#28) | 17:26 | cancelled | cancelled by the next merge's run |
| 34257247545 | `2fc09258` (#30) | 17:27 | cancelled | same |
| 34258510910 | `3ac697c7` (#32) | 17:40 | **success** | first green push run on `main` since the repository was created at 11:57 UTC |
| 34272269646 | `64003ee0` (#21) | 20:00 | cancelled | cancelled by the next merge's run |
| 34272283628 | `b18c5182` (#14) | 20:00 | failure | `cargo-audit`: "binary `cargo-audit` already exists in destination". No advisory involved |
| 34276798329 | `bd534866` (#34) | 20:46 | success | |
| 34282219516 | `bb81abee` (#36) | 21:44:09 | success | |
| 34282224860 | `9a177fd5` (#33) | 21:44:12 | cancelled | zero jobs started; see below |
| 34282231338 | `3ca2896f` (#35) | 21:44:17 | cancelled | zero jobs started; see below |
| 34282237255 | `40101446` (#15) | 21:44:21 | success | |
| 34283175083 | `3f66df5c` (#37) | 21:55 | success | |
| 34283829777 | `c718ca25` (#38) | 22:02 | success | |
| 34284960670 | `d0ca411d` (#39) | 22:15 | success | Rust, Frontend, Python, Repo hygiene all green |

One correction to the account given in #34's description and in the
comment at `ci.yml:17-31`: `main` did not go "from the cutover to that
evening without a single completed CI run". Run 34236052149 completed at
14:05 with a failure. It went without a single *successful* run until
17:40, which is the fact that matters and the more precise way to say it.

### Four gaps this item did not foresee, fixed in #34 (`bd534866`)

- `cancel-in-progress: true` applied to `main` as well as to branches, so
  each merge's run cancelled the previous merge's run. Now
  `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}`
  (`ci.yml:34`).
- Three of those cancellations showed a red X against `cargo test` that was
  the cancellation, not a failure. Recorded in BL-SILENT-001 as a mirror
  instance.
- `pull_request: branches: [main]` meant a stacked pull request (#29 based
  on #28) got no CI at all. The filter is gone from `ci.yml:9` and from
  `osv-scanner.yml:23`.
- `cargo install --locked cargo-audit` refused to overwrite the binary the
  Cargo cache had restored, so every branch and `main` went red on the
  `cargo-audit` step with no advisory involved (run 34272283628 above). The
  step now skips the install when the binary is present (`ci.yml:165-173`).

### One residual, not covered by #34, for the plan rather than this item

Two runs on `main` were still cancelled after #34 merged: 34282224860
(`9a177fd5`, merge of #33) and 34282231338 (`3ca2896f`, merge of #35), both
carrying the fixed `ci.yml`, both cancelled within six seconds of being
created, both with zero jobs. Four merges landed between 21:44:06 and
21:44:19. GitHub allows one running and one pending run per concurrency
group and cancels the older pending run when a third arrives, whatever
`cancel-in-progress` says. So `concurrency: group: ci-${{ github.ref }}`
still drops a `main` run whenever three merges land inside one run's
duration. Those two commits have no CI verdict of their own; their content
is covered by `40101446`'s green run, which contains both. The fix is a
per-commit group on `main` (for example
`ci-${{ github.ref }}-${{ github.ref == 'refs/heads/main' && github.sha || '' }}`),
and it belongs with the plan's item on `concurrency` under "Regression
couplings worth naming" in `docs/release/v1.1.0-plan.md`, not here.

### Release gate item 1

The plan's automated gate item 1, the same SHA green on a pull request and
again on push-to-main, is a release-time check on the release commit. It
belongs in `docs/release/v1.1.0-plan.md`, where it already is. What this
item can say is that the mechanism it relies on exists and has been seen to
work: #37, #38 and #39 each went green on the pull request and then green
on the push run of the merge commit, within the same hour.

### Two more things now true that the original text said were not

- Branch protection is live on `main`: required status checks Rust,
  Frontend, Python, Repo hygiene and Scan lockfiles; `strict: false`; no
  required reviews; `enforce_admins: false`, so an administrator can still
  push past it. Verified with
  `gh api repos/Jura-Labs/jura-trace-dev/branches/main/protection`.
- `cargo audit` on `main` reports 0 vulnerabilities over 921 crates, with
  16 allowed warnings (10 unmaintained, 4 unsound, 2 yanked). `pip-audit`
  reports "No known vulnerabilities found". Both from run 34284960670.

Resolved: 2026-09-08 `bd534866` (#34), on top of `643c4dd3` (jura-archive
PR #27, 2026-09-05). Push trigger, pinned ruff, 67 lint findings cleared,
`cargo test` and `pytest` running on every push to `main`; four further
configuration gaps closed.
