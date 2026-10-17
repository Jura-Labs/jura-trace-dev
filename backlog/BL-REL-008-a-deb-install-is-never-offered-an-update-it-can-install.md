# BL-REL-008: a .deb install is never offered an update it can install

**Status**: Open. Found 8 October 2026, building the Linux live updater leg
(v1.2.0 plan item A11, PR #159).
**Raised**: 13 October 2026
**Severity**: Medium. Nobody loses data and nothing is installed that should
not be. But 40 of the 337 downloads to 12 October were `.deb` files (27 of
v1.0.0, 13 of v1.1.0), and those installs have no working route to a fix.

## What is wrong

Jura Trace publishes two Linux installers, an AppImage and a `.deb`. The
update manifest carries one Linux entry, `linux-x86_64`, and it points at
the AppImage (`.github/workflows/release.yml:2334-2340`).

`tauri-plugin-updater` 2.10.0 chooses its installer from the bundle type the
running app was built as. A `.deb` install looks in the manifest for
`linux-x86_64-deb`, falls back to `linux-x86_64`, downloads what that entry
names, and then refuses to install it because the bytes are not a `.deb`.

So an app installed from the `.deb` is told an update exists, downloads an
AppImage it cannot use, and fails. What the user sees at that point has not
been observed. Nobody has run it.

## Verified, not inferred

- The manifest has no `linux-x86_64-deb` key. Read from `release.yml`, and
  the completeness gates at `:2521` and `:2691` expect exactly
  `darwin-aarch64`, `linux-x86_64` and `windows-x86_64`.
- The plugin's selection order was read from its source at 2.10.0 while
  writing the live leg. It is recorded in the header of
  `.github/workflows/updater-live-leg-linux.yml`, under "DOES NOT PROVE".
- **Not verified**: the behaviour on a real `.deb` install. This item is
  from reading, not from a run.

## Who it affects, and how badly

Anybody who installed the `.deb`. v1.1.0 to v1.2.0 is the first Linux update
that can exist, so the fault has not yet fired for anyone. It fires on the
day v1.2.0 is published.

## What to do, in order

1. **Reproduce it.** Install the v1.1.0 `.deb` on Ubuntu 22.04 after v1.2.0
   is published, press Check for Updates, and write down what the app says.
   The live leg script takes `--appimage`; a `.deb` variant is a small
   change to the same script.
2. **Choose one of two honest answers.**
   - Add a `linux-x86_64-deb` entry to the manifest, with the `.deb` and its
     signature. The `.deb.sig` is already a release asset. Installing it
     needs root, so check what the plugin does about that before promising
     it works.
   - Or say so in the app: on a `.deb` install, show the new version and a
     link to the download page, and do not attempt the install.
3. **Whichever is chosen, add it to the live leg**, so the answer is tested
   on the installed product and not only read.

## What not to do

- Do not point `linux-x86_64` at the `.deb`. That breaks the AppImage, which
  is the path that works.
- Do not ship the manifest entry without a run on a real install. A second
  entry that fails differently is not an improvement.
- Do not drop the `.deb` to make the problem go away without asking Paul. It
  is 12% of downloads.
