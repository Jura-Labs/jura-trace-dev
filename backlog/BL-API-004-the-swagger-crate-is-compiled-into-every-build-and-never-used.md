# BL-API-004: the Swagger crate is compiled into every build and never used

**Status**: Open. Found 11 September 2026, reading the Axum surface while
planning the v1.2.0 headless API and CLI.
**Raised**: 11 September 2026
**Severity**: Low on its own. It matters because it changes the cost of a
fix that is already scheduled: the outbound `unpkg.com` call that
BL-CLAIM-001 row 5 is waiting on can be closed by deleting code rather than
by vendoring assets, and the dependency comes out with it.

## What is wrong

`src-tauri/Cargo.toml:104` declares `utoipa-swagger-ui`, and
`Cargo.toml:116` puts it in the `api` feature, which
`Cargo.toml:114` puts in `default`. So it is compiled into every build of
the desktop application on every platform.

Nothing uses it. `grep -rn "utoipa_swagger_ui\|SwaggerUi" src-tauri/src/`
returns no lines. The three files that reference `utoipa` at all
(`src-tauri/src/api/mod.rs`, `routes.rs`, `types.rs`) use the `utoipa`
derive macros for the OpenAPI document, which is the other crate at
`Cargo.toml:103`, and that one is genuinely used.

The page the crate exists to serve is instead a string literal.
`src-tauri/src/api/mod.rs:271-302`, `swagger_ui_html`, returns hand written
HTML that pulls its JavaScript and CSS from a third party:

```html
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" >
...
<script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"> </script>
```

`src-tauri/src/api/mod.rs:278` and `:282`, with no version in either URL.
That half is already filed. It is BL-CLAIM-001 row 5, SR-24, JTV-209, audit
`NEW-LOW-2` at `docs/security-audit-2026-05-16.md:300-310`, and item A2 of
`docs/release/v1.2.0-plan.md:105`. What none of those record is that the
crate which would serve those same assets from inside the binary is already
being compiled and paid for.

**What it costs.** `utoipa-swagger-ui` 8.1.0 pulls `zip`, `rust-embed`,
`regex`, `mime_guess`, `url`, `base64` and, because it has not moved to
axum 0.8, **a second major version of axum**. `src-tauri/Cargo.lock` has
`axum 0.7.9` at line 385 and `axum 0.8.8` at line 414. This crate is the
only reason 0.7.9 is in the tree, and `src-tauri/Cargo.toml:100` asks for
0.8. Two axum majors are compiled into every release build to support a
page served from a literal.

## Verified, not inferred

Read on `main` at `d9eb2da1` on 11 September 2026.

- `src-tauri/Cargo.toml:103-104`, both utoipa crates declared optional.
- `src-tauri/Cargo.toml:114` and `:116`, `api` is in `default` and pulls
  `utoipa-swagger-ui`.
- `grep -rn "utoipa_swagger_ui\|SwaggerUi" src-tauri/src/` returns nothing.
- `grep -rln utoipa src-tauri/src/` returns `api/types.rs`,
  `api/routes.rs`, `api/mod.rs`, all of which use the derive crate only.
- `src-tauri/src/api/mod.rs:271-302`, the literal HTML, with the two
  unpinned `unpkg.com` URLs at `:278` and `:282`.
- `src-tauri/Cargo.lock:8609-8624`, the crate's dependency list including
  `axum 0.7.9` and `zip 2.4.2`.
- `src-tauri/Cargo.lock:385` and `:414`, the two axum majors.
- `src-tauri/src/api/mod.rs:235-238` and `:242`, the three OpenAPI and Swagger routes
  are merged outside the `require_api_key` layer, so the page is served
  without authentication. That is stated in the audit and repeated here
  because it is what makes deletion attractive.

## Who it affects, and how badly

No user notices the dependency. Every user carries it.

The people it affects are whoever fixes BL-CLAIM-001 row 5. The v1.2.0 plan
prices A2 at half a day to pin or vendor the assets. Deleting the route and
the crate is smaller than that, removes an unauthenticated route from a
port that is always listening, removes the outbound call completely rather
than pinning it, and takes an axum major out of the build. The OpenAPI
document at `GET /openapi.json` is unaffected, because it is produced by
the other crate.

The argument against deletion is that a machine readable API document with
no human readable page is less useful to the audience the v1.2.0 CLI is
aimed at. That is a real argument and it is why this is a backlog item
rather than a patch.

## What to do, in order

1. **Decide between vendoring and deleting**, and decide it as part of
   BL-CLAIM-001 row 5 rather than separately, because the two share one
   fix. Deleting is recommended here: `/openapi.json` stays, the page goes,
   the outbound call and the unauthenticated route go with it, and anyone
   who wants a rendered page can point their own Swagger or Redoc at the
   document.
2. **If deleting**, remove `swagger_ui_html` and `swagger_ui_redirect`
   (`src-tauri/src/api/mod.rs:265-302`), the two routes at `:237-238`, and
   `utoipa-swagger-ui` from `Cargo.toml:104` and `:116`. Then confirm
   `axum 0.7.9` has left `Cargo.lock`, which is the check that proves the
   dependency was really unused rather than transitively required.
3. **If vendoring instead**, use the crate that is already compiled rather
   than embedding downloaded files by hand. That is what it is for, and it
   is the option that makes the current state make sense.
4. **Either way, say so in the v1.2.0 plan's A2 line**, which currently
   describes the work as pinning URLs.

## What not to do

- **Do not pin the `unpkg.com` URLs and stop there.** A pinned third party
  fetch from a local-first product is still a third party fetch, and
  BL-CLAIM-001 exists because the README says there are none.
- **Do not remove `utoipa` itself.** It generates the OpenAPI document the
  v1.2.0 CLI work depends on. Only the `-swagger-ui` crate is unused.
