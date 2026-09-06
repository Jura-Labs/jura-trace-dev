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
