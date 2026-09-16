# BL-DEPS-001: the dependency queue is full, so nothing new can be reported

**Status**: Open, largely resolved. The saturation this item is named for is gone: no ecosystem is at its cap, the cap is raised, and Dependabot opened five new pull requests within two minutes of the queue clearing. What remains is the triage work in steps 4 to 6, listed under "Update, 8 September 2026" at the foot of this file. The headline, "nothing new can be reported", was overstated from the start; see the same section. Found 3 September 2026.
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

## Update, 8 September 2026

Reconciled against `gh pr list --repo Jura-Labs/jura-trace-dev`, the
merge history of `origin/main` at `d0ca411d`, and `.github/dependabot.yml`.

### The premise was overstated, and the file should say so

This item's title and severity rest on the claim that a saturated queue
blocks security advisories. **It does not.** GitHub's documentation for
`open-pull-requests-limit` says security update pull requests "are not
subject to this limit and do not count toward it". The header of
`.github/dependabot.yml:7-11` was corrected to say so in #39 (`d0ca411d`),
and the 3 and 4 September text here inherits the same error. What the cap
blocked was version updates only, which is a real problem (five pip bumps
were starved until this evening, below) but a smaller one than "no channel
through which a newly disclosed vulnerability would reach anybody". The
other two channels in "Why it matters" were genuinely silent (BL-CI-001,
BL-CI-002); this one was not.

### Two repositories, two numberings

The twenty pull requests triaged above were on `Jura-Labs/jura-archive`.
`Jura-Labs/jura-trace-dev` was created at 11:57 UTC on 8 September and
Dependabot re-opened its queue there from #1, so every number below is a
`jura-trace-dev` number and none of them matches the table above. Match on
package name, not number.

### What merged on 8 September (UTC)

| PR | Package | Merged | Commit |
|---|---|---|---|
| #21 | base64 0.22.1 to 0.23.1 (cargo) | 20:00 | `64003ee0` |
| #14 | ui-minor-patch group, 15 updates (npm, `/ui`) | 20:00 | `b18c5182` |
| #15 | @testing-library/jest-dom 6.9.1 to 7.0.1 (npm, `/ui`) | 21:44 | `40101446` |

Two of the "needs a build and a smoke test" four are done by other routes:
Pillow 11.2.1 to 12.3.0 landed on 4 September with the decode-drift test
(`7700c054`, BL-DEPS-003 item 2), and python-dotenv is at 1.2.3 in all
three sidecar manifests.

### What closed without merging, and why

| PR | Package | Reason |
|---|---|---|
| #16 | typescript 5.9.3 to 7.0.2 | Cannot rebase green. `@sveltejs/kit@2.62.0` declares `peerOptional typescript "^5.3.3 \|\| ^6.0.0"`, so `npm ci` fails with ERESOLVE; TypeScript 7 also drops `tsserver`, which `svelte-check` needs. Upstream's move, not ours |
| #26 | c2pa 0.79.3 to 0.85.0 | Does not fix the advisories it appears to. c2pa 0.85 requires `quick-xml ^0.39.0`, still inside the RUSTSEC-2026-0194/0195 range; 0.90 is the first release requiring `^0.41`. Taken as 0.90.20 in #29 instead |
| #1 | opencv-python-headless 4.11.0.86 to 5.0.0.93 | Closed by Paul at 21:54 UTC. Drift test passed (#37); deferred to after v1.1.0 (13 November 2026) as one combined four-package bump |
| #4, #9, #10 | scipy 1.18.1, numpy 2.5.2, pillow-avif-plugin 1.6.0 | Closed as deferred, not rejected, 22:15 UTC. Same drift test, same pass, same deferral, same combined bump |
| #5 | scikit-learn 1.8.0 to 1.9.0 | Closed as **blocked**, 22:15 UTC. The shipped `deepfake_classifier.joblib` cannot be unpickled under 1.9.0 (`ModuleNotFoundError: No module named '_loss'`). Needs its own PR after v1.1.0 that re-serialises the GBM and updates both SHA-256 constants |

None of the five sidecar packages was added to `ignore`, on purpose:
an `ignore` entry mutes security updates as well, and closing the pull
request by hand does not (`.github/dependabot.yml:126-145`).

### The cap, and the evidence it was binding

`open-pull-requests-limit` is 10 for cargo, npm root, npm `/ui` and
github-actions, and **15 for pip from #39** (`.github/dependabot.yml:37,
55, 73, 149, 197`), with `groups` for minor and patch bumps in every
ecosystem. Step 3 above asked for 10; pip got 15 because the five held
bumps were occupying a third of the queue while the drift test ran.

Within two minutes of #39 merging and the five closures landing, Dependabot
opened #40 pytest, #41 anyio, #42 regex, #43 torch and #44 onnxruntime, all
between 22:17:05 and 22:17:31 UTC. Those five had been waiting behind the
cap. That is the saturation effect this item described, observed directly,
and it confirms the corrected scope: version updates were blocked, and only
those.

### Open Dependabot pull requests now, per ecosystem

| Ecosystem | Open | Limit | PRs |
|---|---|---|---|
| pip (`/sidecar`) | 10 | 15 | #6 torchvision, #7 pydantic, #8 uvicorn, #11 certifi, #12 pytest-asyncio, #40 pytest, #41 anyio, #42 regex, #43 torch, #44 onnxruntime |
| cargo (`/src-tauri`) | 7 | 10 | #18 infer, #19 rusqlite, #20 tower-http, #22 rcgen, #25 reqwest, #27 kamadak-exif, #31 cargo-minor-patch group (15 updates) |
| github-actions | 1 | 10 | #13 github-actions group (5 updates) |
| npm (`/ui` and root) | 0 | 10 | |

18 Dependabot pull requests open, none at a cap. Three human pull requests
are also open (#2, #3, #23) and are not this item's business.

### What remains of the six steps

1. Fix the gate. **Done**, BL-CI-001 closed.
2. Close chromadb and sentence-transformers. **Done**: neither was
   re-opened in the new repository.
3. Set the limit. **Done**, above.
4. Merge the lock-file-only and build-chain group. **Not done.** #13
   (actions) and #31 (cargo minor and patch, which subsumes the old
   `image` and `tauri-build` bumps) are open and unreviewed.
5. The four that need a smoke test. **Two of four done** (Pillow,
   python-dotenv). #22 rcgen and #19 rusqlite are open, as is #18 infer.
6. The majors as a decision. **Deferred rather than decided.** tailwindcss,
   vite, vitest and @sveltejs/vite-plugin-svelte majors are in `ignore`
   (`.github/dependabot.yml:97-104`) and fastapi minors too (`:164-165`),
   each with a comment saying the entry is temporary. vitest 4 is still the
   documented fix for the one critical in BL-DEPS-002, so that entry is the
   one to remove first.

One new constraint on the pip queue, from BL-DEPS-004: every open sidecar
pull request will fail the Repo hygiene job until its `requirements.lock`
is regenerated, because `scripts/check_requirements_sync.py` now compares
versions across all three manifests (#36, `bb81abee`). That is the correct
signal. Expect all ten pip pull requests to be red on that job.
