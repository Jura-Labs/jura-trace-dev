# Capabilities — scope rationale

Tauri v2 capability files (`*.json`) are strict JSON, so per-entry comments
can't live in `default.json`. This sidecar README documents *why* each
permission and scope entry is in the allow-list, so future audits don't
have to reverse-engineer the rationale from grep output.

Pair with `default.json`. Update both files in the same commit when adding
or removing permissions.

## `core:default` + `updater:default`

Tauri-baseline event / window / path / app permissions and the Tauri
updater plugin. Required for the application to function.

## `shell:allow-open`

Lets the frontend open external URLs in the user's default browser via
`@tauri-apps/plugin-shell`'s `open()` function. Used by the Help pages and
the Settings → Ollama "manual download" links.

Scope: URLs only — file paths are not accepted by `open()`. The
`shell:allow-execute` permission below is the one to audit for path /
argument injection.

## `shell:allow-execute` (per-shape allow-list)

Replaced 2026-05-17 (JTV-184 / audit NEW-HIGH-2) with per-shape entries —
no more `args: true` wildcards. Each entry pins exact argv shape:

| Name | Cmd | Purpose | Args |
|---|---|---|---|
| `brew-arm-version` / `brew-intel-version` | `/opt/homebrew/bin/brew` or `/usr/local/bin/brew` | Ollama install probe — checks Homebrew is present before attempting `brew install ollama` | `["--version"]` |
| `brew-arm-install` / `brew-intel-install` | same | Ollama install via Homebrew | `["install", <formula validator>]` |
| `winget-version` | `winget` | Ollama install probe on Windows | `["--version"]` |
| `winget-install-ollama` | `winget` | Ollama install on Windows | exact 5-arg argv pinned in the capability |
| `open` | `open` | Reveal-in-Finder on macOS | `["-R", <path validator>]` |
| `xdg-open` | `xdg-open` | Reveal directory on Linux | `[<path validator>]` |
| `explorer` | `explorer` | Reveal-in-Explorer on Windows | `["/select,", <path validator>]` |

Path validator `^[/A-Za-z].+$` rejects leading hyphens — blocks the
flag-confusion class where an attacker-controlled "path" could be
interpreted as a CLI flag by `open` / `xdg-open` / `explorer`.

Formula validator `^[a-z][a-z0-9._-]{0,63}$` matches Homebrew formula
naming convention (lowercase + dot/underscore/hyphen, ≤64 chars).

## `dialog:allow-open` + `dialog:allow-save`

Native file-picker dialogs for the verify, protect, monitor, and settings
flows. No scope restriction — the user explicitly selects each file in
the OS-native dialog; the path returned to the frontend is whatever the
user chose. The downstream `fs:allow-read-file` capability gates what the
frontend can do with the chosen path.

## `fs:allow-read-file`

Frontend reads via `@tauri-apps/plugin-fs`'s `readFile()` /
`readTextFile()`. Today these are not used by any frontend code (verified
2026-05-18, security audit MED-2 / JTV-184 quick-win) — the scope is
defence-in-depth for any future feature that needs to read user-selected
files outside the IPC verify pipeline.

| Scope | Why |
|---|---|
| `$APPDATA/**` | App's own data directory — `Application Support/Jura Trace/` on macOS, `%AppData%\Jura Trace\` on Windows. Needed for reading the app's SQLite database from the frontend if/when a future browse-history feature is added. |
| `$DESKTOP/**` | Common user save location. Verified files often live here. |
| `$DOCUMENT/**` | Common user save location. |
| `$DOWNLOAD/**` | Common user save location. The most likely place a user drops a media asset for verification. |
| `$PICTURE/**` | OS-default media library directory on macOS / Windows / Linux. |
| `$VIDEO/**` | OS-default media library directory. |
| `$AUDIO/**` | OS-default media library directory. (Audio verification is v1.0.x — keep the scope for re-enablement.) |

**Removed 2026-05-18 (JTV-184):** `$TEMP/**` was in the allow-list but
verified to have zero frontend reads. Removed to reduce defence-in-depth
surface. Rust `tempfile::tempdir()` uses are all in test code and bypass
the Tauri fs plugin entirely — they're irrelevant to this capability.

**Open follow-up (v1.1, JTV-184 refactor half):** investigate Tauri v2's
scoped-dialog pattern where `fs:allow-read-file` is only activated for
paths selected via the native file picker, not blanket directory access.
This would tie filesystem reads to explicit user action rather than
granting an always-on capability surface. Material reduction in
post-XSS blast radius if the refactor proves practical.

## `fs:allow-write-file`

Frontend writes via `@tauri-apps/plugin-fs`'s `writeFile()` /
`writeTextFile()`. Used by the PDF / ZIP export flows
(`ui/src/lib/blob.ts`, `ui/src/routes/verify/+page.svelte`) which write
exports to user-selected paths.

| Scope | Why |
|---|---|
| `$APPDATA/**` | App's own data directory — used by Backup/Restore (`ui/src/routes/settings/+page.svelte`) for the SQLite snapshot file. |
| `$DESKTOP/**` to `$AUDIO/**` | Same OS-default user directories as read; user explicitly picks the save location via `dialog:allow-save`. |

`$TEMP/**` is NOT in the write allow-list. Frontend never writes to
temp directories; Rust handles all transient file work via the
`tempfile` crate in server-side code paths.
