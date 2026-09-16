# BL-API-002: the API server announces a port it has not bound, and reports success when it fails to bind

**Status**: Open. Found 11 September 2026, reading the Axum surface while
planning the v1.2.0 headless API and CLI.
**Raised**: 11 September 2026
**Severity**: Medium today, because the only common trigger is a second
copy of the application. High from v1.2.0, because the headless binary and
the desktop application are designed to contend for the same port, so the
condition this code cannot report becomes the normal condition.

## What is wrong

Two faults in eleven lines of `src-tauri/src/api/mod.rs`, and they
compound.

`src-tauri/src/api/mod.rs:150` logs that the server is listening. The bind
is attempted at `:152`, two lines later.

```rust
    log::info!("API server listening on http://{addr}");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            log::warn!(
                "API server: could not bind to {addr}: {e}. \
                 The REST API will be unavailable for this session."
            );
            return Ok(());
        }
    };
```

On failure the log therefore carries a success line followed by a warning,
in that order, which reads as a server that started and then had a problem
rather than one that never started.

The second fault is the `return Ok(())` at `:159`. The caller is
`src-tauri/src/lib.rs:4675`:

```rust
                    if let Err(e) = api::start_server(api_state, 8300).await {
                        log::error!("API server error: {e}");
                    }
```

It reacts only to `Err`. A bind failure is an `Ok`, so nothing is logged at
that level, nothing is surfaced to the window, and no state anywhere in the
application records that the REST API is absent. The application runs
normally in every visible respect and has no API.

The doc comment at `:135-138` documents this as intended:

```rust
/// If the port is already in use (e.g. another Jura Trace instance), a warning
/// is logged and the function returns normally — the Tauri app continues without
/// the API server.
```

Continuing without the API server is a defensible choice. Reporting that
choice as a successful start is not, and that is the part this item is
about.

## Verified, not inferred

Read on `main` at `d9eb2da1` on 11 September 2026.

- `src-tauri/src/api/mod.rs:150`, the `log::info!` line, precedes
  `src-tauri/src/api/mod.rs:152`, the `TcpListener::bind` call.
- `src-tauri/src/api/mod.rs:155-159`, `log::warn!` then `return Ok(())`.
- `src-tauri/src/lib.rs:4675` matches only on `Err`.
- `src-tauri/Cargo.toml:114` reads `default = ["custom-protocol", "api"]`,
  so the server is compiled into every build. There is no setting that
  turns it off.
- The port is the literal `8300` at `src-tauri/src/lib.rs:4675`. Nothing
  reads a configured port, so a collision cannot be worked around by the
  user.

## Who it affects, and how badly

Anyone who opens a second copy of Jura Trace, and anyone whose machine
already has something on 8300. They get an application that looks correct
and whose REST API does not answer. Because the log asserts the opposite,
the first hour of diagnosing that is spent looking somewhere else.

It also affects this repository's own tooling. `scripts/agents/config.py:14`,
`scripts/build-sample-pack.sh:33` and the C2PA evidence-bundle script all
assume 8300 is answering, so a silent non-bind turns into a confusing
failure one layer away from its cause.

The blast radius is small today because the API has no documented consumer,
which is BL-API-001. It stops being small in v1.2.0: the design in
`docs/design/v1.2.0-headless-api-and-cli.md` section 4 has a headless binary
and the desktop application both wanting 8300, and section 13 Q2 leaves the
desktop behaviour unchanged, so the collision is designed in rather than
accidental.

## This is a BL-SILENT-001 instance

It is Cause B in its plainest form: success defined as the function
returning, rather than as the listener existing. It is recorded in the
dated update to `BL-SILENT-001` of 11 September 2026 as well as here,
because that file is the argument for the guards and it gets stronger with
each instance.

## What to do, in order

1. **Move the log line below the bind**, so the message that says a port is
   being served is only emitted once a port is being served. One line moved.
   This is worth doing on its own even if nothing else here is taken.
2. **Return the error.** Change `return Ok(())` at `:159` to return the
   bind error, and let `src-tauri/src/lib.rs:4675` log it at `error!`.
   The signature already returns a `Result`, so this is not a new failure
   path, it is the existing one being used. Update the doc comment at
   `:135-138`, which currently documents the swallow as the contract.
3. **Record the state, do not only log it.** Put an `api_listening: bool`,
   or the bound address, on `AppState`, so the Settings page can say the
   local API is not running and why. A log line nobody opens is the same
   as no report for a desktop user. This is the step that makes the defect
   visible rather than merely honest, and it is the one worth pairing with
   BL-API-003, which is about the same panel.
4. **Decide the port question with v1.2.0, not before.** Whether the
   desktop should fail loudly on a collision, pick another port, or stop
   starting the API unconditionally is Q2 of the v1.2.0 design and needs
   the answer to BL-CLAIM-001's always-on-listener question first. Steps 1
   to 3 are correct under every answer.

## What not to do

- **Do not make the bind failure fatal to the application.** A machine with
  8300 occupied should still verify images. The fix is to report, not to
  refuse.
- **Do not add a retry loop on the port.** Silently moving to 8301 gives
  every consumer a port it cannot predict, and it is the same defect in a
  new shape: the log would then name a port nobody was told about.
