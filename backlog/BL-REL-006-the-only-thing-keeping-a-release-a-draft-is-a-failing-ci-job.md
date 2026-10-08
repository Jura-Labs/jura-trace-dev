# BL-REL-006: the only thing keeping a release a draft is a failing CI job

**Status**: Open. Found 11 September 2026, reading `release.yml` while
planning the v1.2.0 release work. It is what remains of item 6 of
BL-SILENT-001's "still to file, ranked" list. That item's first half, an
incomplete `latest.json` reaching the live endpoint, was closed by the
assertions added between 7 and 10 September. Its second half, the publish
decision itself, is this item.
**Raised**: 11 September 2026
**Severity**: High. It is latent and it fires on the day somebody fixes an
unrelated thing, which is the worst day to discover it.

## What is wrong

`release.yml` publishes a release to the public repository when this is
true, at `.github/workflows/release.yml:2204`:

```yaml
        if: needs.build.result == 'success' && (github.event_name != 'workflow_dispatch' || inputs.platforms == 'all')
```

On a tag push, `github.event_name` is `push`, so the second clause is
always true and the condition reduces to `needs.build.result == 'success'`.

Today that is false on every release, and the workflow's own comment says
exactly why, at `:2185-2188`:

```
        # Only auto-publish when EVERY platform built in CI. macOS is built
        # locally and uploaded after this run (its CI job fails at the known
        # Phase 5 codesign step), so in normal operation this step is skipped
        # and the release stays a DRAFT for manual publish once the macOS
        # artefacts are attached.
```

So the property "a release stays a draft until a person looks at it" is
produced by a job that fails, not by an assertion that something is
required. Nothing states the requirement. Fix the macOS CI job, which is a
desirable thing to do and is tracked in
`project_mac_local_release_build.md`, and the next tag publishes itself.

**What it would publish has never shipped.** The macOS CI path has failed
at Phase 5 on every release, so no artefact it produces has ever reached a
user. The local script is the real macOS release channel
(`docs/release/mac-release-runbook.md`), and BL-REL-005 shows the two
channels do not even produce the same sidecar.

**And the CI path carries a fault the local script had fixed.**
`.github/workflows/release.yml:1053-1063`, "Phase 5 regenerate DMG +
updater payload", runs:

```sh
          cargo tauri bundle --bundles dmg,updater --target aarch64-apple-darwin
```

The comment above it says this regenerates the `.app.tar.gz` now that the
nested signing is correct. On macOS `updater` is not a bundle target;
the archive is a by-product of `app`. That is the exact fault found in
`scripts/build-local-mac.sh` on 9 September 2026 and fixed in PR #49,
recorded in BL-SILENT-001 under "The warning nobody read": Tauri emits
"no updater-enabled targets were built" as a Warn line and the archive on
disk stays the one written before the signing. The step's own `ls` at
`:1062-1063` lists `dmg/` and `macos/` and would not show it either way.

The gate that would catch a stale archive exists, and macOS does not have
it. The Windows gate at `:1389-1393` is explicit about why:

```
      # ── Empty-signature and freshness gate (Windows) ─────────────────────
      # Runs LAST, on the bytes that ship. Checks each .sig exists, is
      # non-empty, and is NEWER than the installer it signs. That last
      # assertion is the one that encodes the bug: a signature older than
      # its artefact cannot possibly describe it.
```

The macOS and Linux gate at `:1125-1149` checks only that each `.sig`
exists and is not zero bytes. A `.app.tar.gz` and `.sig` pair left over
from before the re-signing passes it.

So the first green macOS CI run would publish, automatically and with no
person in the loop, a release whose updater archive is likely to contain
the `.app` from before the nested signing was applied and before
notarisation and stapling.

## Verified, not inferred

Read on `main` at `d9eb2da1` on 11 September 2026.

- `.github/workflows/release.yml:2204`, the condition, quoted in full
  above.
- `:2185-2203`, the comment, including the sentence naming the macOS CI
  failure as the reason the step is normally skipped.
- `:1053-1063`, the regenerate step and its `cargo tauri bundle
  --bundles dmg,updater`.
- `:1125-1149`, the macOS and Linux signature gate: existence and
  non-empty only, no freshness comparison.
- `:1389-1393` and the step at `:1394`, the Windows freshness gate.
- `:1088-1111`, the notarise and staple step, which runs after the
  regenerate step, so a stale archive would predate stapling as well.
- `:1520`, the job-level condition, `always() && ... (needs.build.result
  == 'success' || needs.build.result == 'failure')`, which is why the job
  runs at all on a failing matrix.
- The 10 September v1.1.0 release is the control: the DMG, the updater
  archive, its signature, `SHA256SUMS.txt` and `latest.json` were all
  attached between 18:26:59Z and 18:57:51Z, and
  `docs/release/v1.2.0-plan.md:65-70` records that there is no Actions run
  in the repository after 17:59:33Z. All of it was done by hand.

**Stated as inference, not as fact:** that the CI regenerate step produces
no updater archive has not been observed on a runner, because the job has
never got that far. It is the same command on the same platform that was
proved to do nothing locally on 9 September. What would settle it is one
`workflow_dispatch` macOS run that prints `ls -la` of the `macos/`
directory with timestamps before and after the regenerate step. That is
step 3 below and it is cheap.

## Who it affects, and how badly

Nobody yet. The population is everyone who auto-updates, on the first tag
cut after the macOS CI job is repaired.

The failure mode is the one BL-REL-002 already cost this project 145 users
for: an updater archive that the updater downloads, checks and rejects, or
worse, one that installs an application whose nested code is not correctly
signed, on the platform where Gatekeeper is strictest.

The second-order cost is that a fix to the signing job, made for good
reasons by somebody who has read none of this, silently changes the release
process from "a person publishes" to "the workflow publishes". Nobody
decides that. It just becomes true.

## This is a BL-SILENT-001 instance

It is the surviving half of item 6 of the ranked list in that file, and it
is also Cause C, a
gate that does not run on the bytes that ship. The missing macOS freshness
assertion is Cause C in its plainest form, and it is the one that was
written for Windows and never mirrored. Recorded in the dated update to
BL-SILENT-001 of 11 September 2026 as well as here.

## What to do, in order

1. **State the requirement instead of relying on the failure.** Replace
   `needs.build.result == 'success'` with a check on what is actually
   attached to the release: all three installers, the macOS DMG, the
   updater archive and its signature, and a `latest.json` carrying all
   three platform keys. The completeness assertion at `:1949-1977` is the
   pattern; this is the same assertion applied to the publish decision.
   Until the requirement is stated, every other step here is optional.
2. **Add the freshness assertion to macOS and Linux**, mirroring the
   Windows gate at `:1394` onwards: each `.sig` newer than the artefact it
   signs, and each artefact newer than the last mutation of the `.app`.
   Fifteen lines, and it is the assertion that makes step 3 unnecessary to
   repeat by hand.
3. **Prove what the regenerate step does.** One dispatch run, `ls -la` with
   timestamps either side. If it produces no archive, fix it the way PR #49
   fixed the local script, by building the archive from the signed `.app`
   explicitly rather than asking for a bundle target that does not exist.
4. **Decide, deliberately, whether macOS should ship from CI at all.** The
   answer may be no, and `project_mac_local_release_build.md` has the
   reasons. If it is no, then the publish step should say so: gate on the
   macOS artefacts being present rather than on a job result, and let the
   local build be the thing that satisfies it. That is step 1 again, which
   is why step 1 is first.

## What not to do

- **Do not leave the guard as it is because it currently works.** A guard
  that depends on a failure persisting is not a guard, and the person who
  removes the failure will have no way to know they changed anything else.
- **Do not fix the macOS CI signing job before step 1.** That is the change
  that fires this. Landing them in the other order is a release published
  by accident.
- **Do not fold this into BL-REL-007.** They are both in `release.yml` and
  both fixed by asserting what is required, but one is a publish decision
  and one is a checksum file, and BL-REL-007 is cheap enough that it should
  not wait for this.
