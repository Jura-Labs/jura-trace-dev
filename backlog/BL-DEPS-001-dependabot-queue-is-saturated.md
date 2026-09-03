# BL-DEPS-001: the dependency queue is full, so nothing new can be reported

**Status**: Open. Found 3 September 2026.
**Raised**: 3 September 2026
**Severity**: High, and higher than the individual bumps suggest. Combined
with BL-CI-002, Jura Trace currently has no working channel through which a
newly disclosed dependency vulnerability would reach anybody.

## What is wrong

Twenty dependabot pull requests are open on `Jura-Labs/jura-archive`. They
are not spread evenly. They sit at exactly five in each of the four active
ecosystems:

```
   5 cargo
   5 github_actions
   5 npm_and_yarn
   5 pip
```

`.github/dependabot.yml` sets no `open-pull-requests-limit`, so the default
of five per ecosystem applies. **Every ecosystem is at its cap.** Dependabot
cannot open a pull request for anything new, including a security advisory,
until existing ones are closed or merged.

The oldest are from 20 March 2026, the newest from 3 April. The last
"Dependabot Updates" run was 29 June 2026, four days after the last commit
to the repository. Nothing has moved since.

## Why it matters

The three ways this repository would learn that a dependency has a known
vulnerability are all currently silent:

1. **Dependabot** cannot open new pull requests. The queue is full.
2. **OSV-Scanner** has not actually scanned since July. See BL-CI-002.
3. **`pip-audit` and `cargo-audit`** live in `ci.yml`, which has not run
   since 30 April. See BL-CI-001.

What a hand-run `npm audit` finds today is in BL-DEPS-002: one critical and
four high findings, none of which reached anyone through any of the three.

## The twenty, triaged

Checked against the manifests as they stand on 3 September 2026.

### Close without merging (2)

Both target a dependency that no longer appears in any requirements file.
`grep -rn "chromadb\|sentence-transformers" sidecar/requirements*` returns
nothing.

| PR | Package | Proposed |
|---|---|---|
| #2 | chromadb `==0.5.*` to `==1.5.*` | Close. Not a dependency any more |
| #3 | sentence-transformers `==3.*` to `==5.*` | Close. Not a dependency any more |

Closing these two frees two pip slots immediately and costs nothing.

### Lock-file only, no manifest change (2)

The manifest range already admits the new version, so these are a
`cargo update -p` away and carry no API risk.

| PR | Package | Change | Manifest today |
|---|---|---|---|
| #4 | image | 0.25.9 to 0.25.10 | `image = "0.25"` (Cargo.toml:60) |
| #9 | tauri-build | 2.5.5 to 2.5.6 | `tauri-build = { version = "2" }` (Cargo.toml:34) |

### Build-chain hygiene, low blast radius (6)

None of these ship to a user. Four of the five GitHub Actions bumps also
address the Node 20 deprecation that is now warning on every run.

| PR | Package | Change | Note |
|---|---|---|---|
| #13 | actions/checkout | 4 to 6 | mergeable |
| #7 | actions/setup-node | 4 to 6 | mergeable |
| #10 | actions/setup-python | 5 to 6 | conflicting, needs rebase |
| #11 | actions/cache | 4 to 5 | conflicting, needs rebase |
| #8 | actions/github-script | 7 to 8 | conflicting, needs rebase |
| #18 | postcss | 8.5.6 to 8.5.8 | patch; also clears a high advisory |

The three conflicts are because `ci.yml` and `release.yml` were edited in
June after the pull requests were opened. Rebasing is mechanical.

### Needs a build and a smoke test (4)

Each of these touches code that runs in front of a user. None should be
merged on the version number alone.

| PR | Package | Change | What it touches |
|---|---|---|---|
| #14 | rcgen | 0.13.2 to 0.14.7 | The per-install local CA behind Sovereign-mode signing. A behaviour change here changes certificates users have already signed against |
| #16 | rusqlite | 0.31 to 0.39 | Eight minor versions of the database layer, including the bundled SQLite. Expect API churn |
| #24 | Pillow | 11.2.1 to 12.2.0 | The pixel path under ELA, noise, copy-move and JPEG ghost. A decode difference moves detector outputs, which moves verdicts |
| #19 | infer | 0.16 to 0.19 | File type sniffing, which is what routes a file to a pipeline |

Pillow is the one to be most careful with. The detector thresholds were
calibrated against images decoded by the current version. Re-run the
calibration corpus before accepting it, and record the before and after.

### Majors that are a decision, not a bump (6)

| PR | Package | Change | Why it is a decision |
|---|---|---|---|
| #15 | tailwindcss | 3.4.19 to 4.2.2 | Tailwind 4 replaces the config file with CSS-native configuration. This is a UI migration, not an upgrade |
| #17 | @sveltejs/vite-plugin-svelte | 5.1.1 to 7.0.0 | Two majors; coupled to the vite bump below |
| #26 | vite | 6.4.1 to 8.0.10 | Two majors; clears a high advisory |
| #25 | vitest | 2.1.9 to 4.1.5 | Two majors; clears the one critical advisory. `npm audit` names this as the fix for both |
| #23 | fastapi | 0.115.12 to 0.135.3 | Twenty minor versions of the sidecar's web framework |
| #22 | python-dotenv | 1.1.0 to 1.2.2 | Small, but the sidecar reads its key material through it |

#17, #25 and #26 should move together or not at all. They share a
dependency graph and splitting them produces a build that resolves but does
not run.

## What would fix it

In this order, because each step makes the next one safer.

1. **Fix the gate first.** BL-CI-001. Merging any of these without a green
   build is guessing.
2. **Close #2 and #3.** No build needed.
3. **Set `open-pull-requests-limit`** in `.github/dependabot.yml`, to 10 for
   cargo, npm and pip. A cap of five silently converts "you have unread
   security news" into "you have no security news".
4. **Merge the lock-file-only and build-chain group** as one branch, behind
   the repaired gate. Eight pull requests close.
5. **Then the four that need a smoke test**, one branch each, with the
   calibration corpus re-run for Pillow.
6. **Then take the six majors to a decision**, as scheduled work with an ID,
   not as a dependency chore.

## What not to do

Do not merge the majors to clear the count. Four of the six are development
dependencies that never reach a user, and the count is not the problem. The
saturated queue is the problem, and closing two stale pull requests and
raising the limit fixes that in ten minutes.

Do not merge anything to `main` directly. Rule 4: code goes to a branch.
Note that `ci.yml` only runs on pull requests, so a direct push would also
skip the gate that step 1 exists to repair.
