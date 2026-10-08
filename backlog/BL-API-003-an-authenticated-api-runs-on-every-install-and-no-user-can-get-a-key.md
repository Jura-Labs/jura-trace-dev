# BL-API-003: an authenticated API runs on every install and no user can get a key for it

**Status**: Open. Found 11 September 2026, reading the Axum surface and the
settings page while planning the v1.2.0 headless API and CLI.
**Raised**: 11 September 2026
**Severity**: Medium. Nothing is exposed and no user is harmed, because the
credential is unobtainable by construction, which is the only reason the
severity is not higher. It is the precondition for the v1.2.0 CLI, so it is
a blocker on the work most likely to bring new users, exactly as BL-API-001
is.

## What is wrong

Every Jura Trace install runs an authenticated REST API on `127.0.0.1:8300`
from first launch. The application mints one API key on that first launch,
stores only its SHA-256 hash, logs eight characters of it, discards the
rest, and hides the only surface that could mint another. From the user's
side there is a server, a lock, and no key.

**The server starts unconditionally.** `src-tauri/Cargo.toml:114` has
`api` in `default`, and `src-tauri/src/lib.rs:4675` starts it inside the
Tauri setup path with the port as a literal. There is no setting, flag or
argument that prevents it.

**Every route requires a key.** `src-tauri/src/api/mod.rs:204-232` builds
the `/api/v1/...` sub-router and layers `auth::require_api_key` over all of
it, including `POST /v1/auth/keys` at `:221`. Creating a key over HTTP
requires a key you already have.

**The bootstrap key is discarded at birth.**
`src-tauri/src/db.rs:2169-2181` generates a raw key, hashes it with SHA-256,
stores the hash, and returns the raw value once.
`src-tauri/src/lib.rs:4657-4663` logs a truncated prefix and nothing else:

```rust
                                let prefix = &key[..key.len().min(8)];
                                log::info!(
                                    "API server bootstrap key created (jt_{prefix}...). \
                                     Retrieve the full key from Settings → API Keys."
                                );
```

The raw value is then dropped. It exists nowhere on disk.

**The panel that sentence names does not render.**
`ui/src/lib/featureFlags.ts:53` is `export const V1_SHOW_API_KEYS = false;`
and `ui/src/routes/settings/+page.svelte:2919` gates the whole API Keys
section on it. The `create_api_key`, `list_api_keys` and `revoke_api_key`
IPC commands exist and are registered at
`src-tauri/src/lib.rs:4740-4742`, with no caller that a user can reach.

**There is a second gate underneath the first.**
`ui/src/routes/settings/+page.svelte:813` derives
`apiKeysAvailable` as `currentTier === 'professional' || currentTier === 'enterprise'`.
So even with the feature flag flipped, a Community user would see the
unavailable branch rather than the key list.

**The server is not tier gated.** `grep LicenceTier src-tauri/src/api/`
returns nothing. `LicenceTier` appears in `lib.rs`, `config.rs`, `state.rs`
and `monitor_scheduler.rs` and in none of the five files under
`src-tauri/src/api/`. The tier check exists only in the frontend, which is
the half that is hidden. So the API is not Community-restricted in any
enforceable sense. It is simply unusable by everyone.

## Verified, not inferred

Read on `main` at `d9eb2da1` on 11 September 2026.

- `src-tauri/Cargo.toml:114`, `default = ["custom-protocol", "api"]`.
- `src-tauri/src/lib.rs:4675`, `api::start_server(api_state, 8300)`.
- `src-tauri/src/api/mod.rs:221-232`, `POST /v1/auth/keys` inside the
  `require_api_key` layer.
- `src-tauri/src/db.rs:2173-2180`, SHA-256 of a UUID, only the hash stored.
- `src-tauri/src/lib.rs:4658`, `&key[..key.len().min(8)]`.
- `ui/src/lib/featureFlags.ts:53` and
  `ui/src/routes/settings/+page.svelte:2919`.
- `ui/src/routes/settings/+page.svelte:813`, the tier derivation.
- `grep -rn LicenceTier src-tauri/src/api/` returns no lines.

## Who it affects, and how badly

Nobody is harmed today, and that is worth stating plainly rather than
implying otherwise. The listener is on loopback
(`src-tauri/src/api/mod.rs:148` binds `127.0.0.1`), every route needs a
bearer token, and no token is obtainable, so the practical exposure of the
always-on server is small.

Two things are still wrong.

**It is a service the user did not ask for and cannot see.** For a product
sold on local-first behaviour, an always-listening authenticated port that
appears in no setting and no help page is the kind of thing a reviewer
finds rather than the kind of thing a product discloses. That question
belongs with BL-CLAIM-001, which is already rewriting what the product says
about itself, and it is Q2 of `docs/design/v1.2.0-headless-api-and-cli.md`
section 13, which correctly refuses to settle it without legal review.

**It blocks the v1.2.0 CLI.** A thin REST client needs a key. Today the
only way to obtain one is to edit a feature flag, rebuild, and be on a tier
the shipped build does not offer. Whatever the CLI does about
authentication, it needs an issuance path, and there is none.

## What to do, in order

1. **Decide whether the desktop keeps starting the API at all.** This is
   the question the rest depends on, it is Q2 of the v1.2.0 design, and it
   is not a decision this file can take. Until it is answered, do not
   build issuance on top of a listener that may be turned off by default.
2. **Give the key an issuance path that is not the hidden panel.** The
   cheapest is a first-run reveal: when `ensure_bootstrap_api_key` returns
   `Some`, surface the key once in the UI with a copy control, exactly as
   `ui/src/routes/settings/+page.svelte:2944-2960` already does for a
   newly created key. That code exists and is unreachable.
3. **Unhide the panel, or delete it.** Leaving `V1_SHOW_API_KEYS = false`
   with a live authenticated server is the state that produced this item.
   If the API is to have users, the panel ships; if it is not, the server
   should not be starting. The comment at
   `ui/src/routes/settings/+page.svelte:2913-2918` ties the unhide to a
   Pro tier that `project_v1_community_only_launch.md` deferred, so the
   flag is waiting on a plan that changed.
4. **Decide the tier question in Rust or not at all.** If the API is meant
   to be a paid surface, the check belongs in
   `src-tauri/src/api/auth.rs` where it can be enforced. A tier check that
   lives only in a hidden Svelte branch enforces nothing and makes the
   product look gated when it is not.
5. **Correct the copy.** The unavailable branch at
   `ui/src/routes/settings/+page.svelte:2935` states, among other things,
   that no key-authenticated REST endpoint exists. That sentence is
   handled as the thirteenth frozen version string in BL-CLAIM-002's
   update of 11 September 2026, and it cannot be corrected sensibly until
   steps 1 to 4 decide what the true sentence is.

## What not to do

- **Do not remove the authentication to make the API usable.** An
  unauthenticated local port that accepts file uploads and signing
  requests is a worse product than an unusable one.
- **Do not write the bootstrap key to a file to make it retrievable.** A
  plaintext credential on disk is how this gets written up in somebody
  else's audit. Reveal once in the UI, or issue on request.
- **Do not treat this as fixed by flipping the feature flag.** The tier
  derivation at `:813` is a second gate, and the copy behind it is wrong.
  Flipping the flag alone shows Community users a panel telling them the
  feature does not exist while their machine serves it.
