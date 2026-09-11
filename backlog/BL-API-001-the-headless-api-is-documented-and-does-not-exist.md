# BL-API-001: the headless API deployment story is documented and does not exist

**Status**: Open. Found 3 September 2026, while assessing what would most
increase adoption.
**Raised**: 3 September 2026
**Severity**: Medium as a documentation fault. High as a blocker, because
it sits directly under the CLI, which is the change most likely to bring
new users.

## What is wrong

`docs/API_WRAPPER.md:95-109` describes two ways to run the REST API:

> - **Separate binary**: `jura-trace-api` compiled from the same workspace,
>   started independently of the desktop application.
> - **Feature flag in the main binary**: `jura-trace --api --port 8300`
>   starts the desktop application and the API server together.
>
> The separate binary approach is preferred for enterprise deployments
> where the API must run in a headless server context (CI/CD pipelines,
> background services) without launching a desktop window.

With shell examples:

```bash
jura-trace-api --port 8300 --config /etc/jura-trace/api.toml
jura-trace --api --port 8300
```

**Neither exists.**

- `src-tauri/Cargo.toml:28-31` declares exactly one extra binary target,
  `gen-detectors`. There is no `jura-trace-api`.
- The desktop binary parses no command-line arguments at all. There is no
  `clap` or `argh` dependency and no `std::env::args` handling, so
  `--api --port 8300` is silently ignored.
- The API server starts in exactly one place, `src-tauri/src/lib.rs:4658`,
  inside the Tauri setup path. It runs when the desktop window runs, and
  not otherwise.

The `api` cargo feature does exist (`Cargo.toml:113-115`), so the code is
modular enough that a second binary target is plausible. It has simply
never been made.

## Why it matters now

Two independent reviews on 3 September pointed at a CLI as the single
change most likely to bring new users, on the reasoning that curated OSINT
and fact-checking toolkits index things that can be scripted, and a
desktop-only tool is invisible to all of them. The locked CLI architecture
is a thin REST client over this API.

A thin client over an API that only exists while a GUI window is open is
not scriptable in the way those toolkits mean. So the headless path is not
a nicety attached to the CLI; it is the CLI's precondition.

There is a second cost. The sidecar, which provides every forensic
detector, is spawned by the Tauri setup path
(`src-tauri/src/state.rs`, `spawn_sidecar`). A headless API has to own that
process lifecycle itself: start it, wait for readiness, shut it down. That
is the real work in this item, not the Axum server, which already runs.

## What would fix it

1. **Add a `jura-trace-api` binary target** behind the existing `api`
   feature, owning sidecar spawn, readiness and shutdown. This is the
   substance of the item.
2. **Decide about `--api` on the desktop binary.** Either implement the
   argument or delete it from the documentation. Implementing it means
   adding argument parsing to a binary that has none, which is more than it
   sounds; deleting it is a one-line documentation fix.
3. **Correct `docs/API_WRAPPER.md`** either way, so it describes what
   exists. Until then it is telling enterprise evaluators that a deployment
   mode is available when it is not, which is the same class of fault as
   BL-CLAIM-002 and BL-DOC-001, and this is the third instance found in one
   day.

## A related decision that has gone stale

`docs/decisions/jura-cli-binary-name-reservation.md` (11 May 2026, accepted)
reserves the binary name `jura` and the repository
`codeberg.org/jura-labs/jura-cli`, on the reasoning that "Codeberg is the
primary code host". W37-1 retires Codeberg. The binary name is unaffected
and still good; the repository half of that decision needs restating
against GitHub before anyone acts on it.

## What not to do

Do not ship a CLI that requires the desktop application to be open, and
describe it as automation-ready. The audience for a CLI is precisely the
audience that will find that out immediately.

## Update, 11 September 2026: two corrections and one non-defect, from the v1.2.0 design

`docs/design/v1.2.0-headless-api-and-cli.md` was written on 10 and 11
September and answers most of this item. Three things from it belong here
rather than in a new file, because all three are about the same document
this item exists to correct, `docs/API_WRAPPER.md`, or about the contract
the CLI inherits.

**The exit-code contract is published twice and disagrees with itself.**
`docs/API_WRAPPER.md:683-696` and `:1027-1040` both define it, and two rows
differ:

| Code | First table, `:694` | Second table, `:1038` |
|---|---|---|
| 5 | "Server rejected the input as an unsupported format (**HTTP 422**)" | "Server returned **400 Bad Request** ... unsupported MIME type" |
| 6 | "Server-side error (HTTP 5xx) **or partial-availability degraded response**" | "Server returned 5xx ... sidecar crash, internal panic, database error" |

Both tables claim stability. The first says "stable across all v1.x
releases" and "existing codes are never re-purposed"; the second says
"stable from v1.0.1" and that existing assignments do not change. A
contract that is published twice, differently, and described as immutable
in both places, is worse than no contract, because a script author has no
way to know which half they read. `docs/design/v1.2.0-headless-api-and-cli.md`
section 7.3 already decides which is right for v1.2.0; what remains is to
delete the duplicate rather than leave both in the file, and to do it in
the same commit as step 3 of this item.

**The trust band exists only in the frontend.** `ui/src/lib/types.ts:1195-1200`
is `getTrustLevel`, three thresholds at 0.7 and 0.4, and
`ui/src/lib/components/VerdictSummary.svelte:20-27` adjusts it for an
inconclusive deepfake verdict. `grep -rn "trust_band\|trustBand" src-tauri/src/ sidecar/`
returns nothing. So the band a score falls into is computed nowhere except
in one Svelte component, and the PDF path, the database, the REST API and
anything built on the API must each re-derive it. That is not a defect in
the shipped product, which has exactly one consumer. It becomes one the
moment a second consumer exists, which is what this item is for. Decide
where the band lives before the CLI prints one.

**And one thing that is recorded as a defect and is not.** Section 12 item
9 of the design document says there is no `busy_timeout` on the SQLite
connection. There is. `src-tauri/src/db.rs:31` opens through
`rusqlite::Connection::open`, and rusqlite 0.31.0 calls
`sqlite3_busy_timeout(db, 5000)` on every open, at
`inner_connection.rs:121` in the vendored source. Nothing in this
repository sets or clears one, and all four open sites under
`src-tauri/src/` use that constructor, so five seconds is in force
everywhere. Whether five seconds is the right number once a headless binary
shares the file is a real question for the design and a different one from
the defect as written. Recorded here rather than edited into the design
document, per the constraint on the deliverable that found it.
