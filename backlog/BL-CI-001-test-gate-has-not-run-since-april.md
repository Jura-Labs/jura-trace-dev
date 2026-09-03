# BL-CI-001: the test gate has not run since 30 April, and nothing noticed

**Status**: Open. Found 3 September 2026.
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
