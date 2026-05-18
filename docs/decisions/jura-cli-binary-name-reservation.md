# Decision: Reserve `jura` Binary Name + Codeberg Namespace

**Date**: 2026-05-11
**Status**: Accepted
**Owner**: Paul (Jura Labs CIC)
**Tickets**: JTV-181 (v1.0 groundwork), JTV-182 (v1.0.1 CLI surface)
**Related decision**: 2026-05-11 CLI architecture lock (memory `project_cli_v101_locked.md`)

## Decision

Reserve the binary name `jura` and the Codeberg repo namespace `codeberg.org/jura-labs/jura-cli` for the v1.0.1 CLI client.

- **Binary name**: `jura` (not `jura-cli`, not `jt`, not `juratrace`) — short, memorable, matches the brand prefix without competing with the desktop binary `jura-trace`.
- **Codeberg repo**: `codeberg.org/jura-labs/jura-cli` (namespace `jura-labs` already exists; sub-repo does not yet exist as of 2026-05-11).

## Rationale

1. **Distinct from `jura-trace`**: the desktop installer registers binaries under the `jura-trace` family. `jura` is a clean parent-namespace command that does not collide with `jura-trace`, `jura-sidecar`, or any future Codeberg-namespaced binary.
2. **Short for daily use**: forensic personas (Fatima Al-Rashid, Amara Diallo, James Okafor — top 3 per the 2026-05-11 persona-testing synthesis) will invoke this binary frequently in shell scripts and case-file evidence chains. A four-character command keeps audit trails compact.
3. **Codeberg-first**: per `project_hosting_migration_plan.md` (2026-05-05), Codeberg is the primary code host. The CLI lives in its own repo so the desktop monorepo (`jura-archive`) stays self-contained and the CLI's release cadence is independent.
4. **Matches the v1.0.1 architecture**: the CLI is a thin Rust client (Option B in the 2026-05-11 design), so a standalone Cargo workspace at `codeberg.org/jura-labs/jura-cli` is the natural home.

## What's reserved vs. created today

- **Reserved (this decision, no infrastructure change)**:
  - Binary name `jura`
  - Codeberg repo path `codeberg.org/jura-labs/jura-cli`
  - Crate name `jura` on crates.io (will be claimed at v1.0.1 cut)
- **NOT created today** (deferred until v1.0.1 implementation kicks off):
  - The Codeberg repo itself
  - A Cargo workspace member
  - Any binary artefact

The decision is recorded here so any future change to the CLI binary name requires re-opening this document rather than being a silent rename.

## What v1.0 ships (JTV-181 — done in this session)

1. `methodology.univfd_probe_model_hash` added to `VerificationResult` so the future `jura` CLI can pin the exact CLIP probe used.
2. CLI exit-code contract documented in `docs/API_WRAPPER.md` (0 success / 1 usage / 2 unreachable / 3 auth / 4 file / 5 format / 6 server). Pre-locked so newsroom CI authored against this contract today will work against the v1.0.1 CLI on day one.
3. Schema-as-public-API-contract policy documented in `docs/API_WRAPPER.md`. No breaking changes within v1.x; deprecation cycle of one minor release before any field removal.

## What v1.0.1 ships (JTV-182 — 3 engineer-days post-launch)

- Cargo workspace member `cli/` (or, more likely, separate Codeberg repo as a thin client)
- Commands: `jura verify`, `jura sign`, `jura version`, `jura auth set-key`, `jura auth show-key`
- Cross-platform binaries (mac / linux / windows), ~5 MB each
- Independent auto-updater

## What v1.1 ships (post-NLnet funding)

- `jura serve` headless mode (unlocks museum Linux deployment)
- `jura batch <glob>` parallel verification with NDJSON output
- `jura watch` Watched Locations integration
- `--no-network` / `--ephemeral` flags (Amara/Aisha field-ops requirements)
- Berkeley Protocol / AI Act export formats

## Reversibility

- Renaming the binary post-v1.0.1 release would break shell scripts and audit trails — high reversibility cost. Decide now, not later.
- The Codeberg repo URL is part of the auto-updater endpoint contract; renaming is an updater-pipeline migration. Cost: ~half-day; not free.

## References

- Memory `project_cli_v101_locked.md` — full architecture lock
- Memory `project_hosting_migration_plan.md` — Codeberg-primary policy
- `docs/API_WRAPPER.md` — exit-code contract + schema contract
- Plane: JTV-181 (v1.0 groundwork), JTV-182 (v1.0.1 deliverable)
