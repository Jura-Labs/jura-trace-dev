# Sidecar binaries

Tauri's `externalBin` entry in `tauri.conf.json` is `binaries/jura-sidecar`.
At bundle time Tauri looks for a file in this directory named for the
compilation target, and places it at `Contents/MacOS/jura-sidecar` (macOS)
or the equivalent, and signs it as part of the app.

| Target | File | What it is |
|---|---|---|
| macOS Apple Silicon | `jura-sidecar-aarch64-apple-darwin` | **A compiled launcher**, built from `../sidecar-launcher/jura-sidecar-launcher.c`. Execs the real PyInstaller `--onedir` bootloader at `Contents/Resources/sidecar-bundle/jura-sidecar`, where its `_internal/` tree lives beside it. See the source for why it is a Mach-O binary and not a script. |
| macOS Intel | `jura-sidecar-x86_64-apple-darwin` | placeholder; no Intel release is built |
| Windows x64 | `jura-sidecar-x86_64-pc-windows-msvc.exe` | placeholder; the release workflow replaces it with the PyInstaller `--onefile` output |
| Linux x64 | `jura-sidecar-x86_64-unknown-linux-gnu` | placeholder; the release workflow replaces it with the PyInstaller `--onefile` output |

All four files are tracked, so that `cargo tauri dev`, `cargo check` and CI
have something at the path Tauri requires. The two Windows and Linux
placeholders are overwritten in CI and the real artefacts are never
committed; if you rebuild one locally, `git update-index --skip-worktree`
keeps it out of your diff (see the root `.gitignore`).

## The macOS launcher

`scripts/build-local-mac.sh` recompiles the launcher from source on every
build and proves the tracked file is that source compiled, using
`scripts/macho-equal.py`, which strips the linker's ad-hoc signature from
temporary copies of both, masks the random `LC_UUID`, and requires every
other byte to match. The build stops if they differ. It never overwrites the tracked file.

Byte-identical output is not available from Apple's linker: every link gets a
fresh random UUID, `-reproducible` does not change that, and `-no_uuid` yields
a binary `dyld` refuses to load. That is why the comparison is masked rather
than exact.

To rebuild it by hand after changing the source:

```bash
cc -O2 -Wall -Wextra -Werror -arch arm64 -mmacosx-version-min=13.0 \
   -o src-tauri/binaries/jura-sidecar-aarch64-apple-darwin \
   src-tauri/sidecar-launcher/jura-sidecar-launcher.c
# Then prove a second compile is the same code as the file you just committed:
cc -O2 -Wall -Wextra -Werror -arch arm64 -mmacosx-version-min=13.0 \
   -o /tmp/launcher-check src-tauri/sidecar-launcher/jura-sidecar-launcher.c
python3 scripts/macho-equal.py /tmp/launcher-check src-tauri/binaries/jura-sidecar-aarch64-apple-darwin
```

Commit the source and the binary together. Do not codesign the tracked
file; the linker's ad-hoc signature is enough for it to run, and Tauri signs
the copy it places inside the `.app` with the Developer ID identity.

## Why it is a Mach-O binary and not a script

Until 9 September 2026 the launcher was a 38-line shell script. A script's
code signature is stored in extended attributes. The DMG carries those, so
the DMG passed every check; the updater archive is a tar, Tauri's tar writer
drops extended attributes, and the client-side updater extracts with the
same library. The launcher reached a user's Mac unsigned and Gatekeeper
assessed the updated app as "rejected, no usable signature". A Mach-O binary
carries its signature inside the file. See BL-REL-002 and the build script's
Phase 6.5 for the gate that now proves it on every build.

## Development

In development (`cargo tauri dev`) `spawn_sidecar` returns early and this
directory is not used. Run the sidecar directly:

```bash
cd sidecar
uvicorn main:app --host 127.0.0.1 --port 8200 --reload
```
