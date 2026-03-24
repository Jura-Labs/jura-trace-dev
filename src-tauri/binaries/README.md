# Sidecar Binaries

This directory holds the frozen Python ML sidecar binary produced by PyInstaller.
Tauri bundles and launches the binary automatically in production builds.

## Binary naming

Tauri v2 requires platform-specific suffixes on sidecar binaries. The `externalBin`
entry in `tauri.conf.json` is `"binaries/jura-sidecar"` — Tauri maps this to the
following filenames at bundle time, based on the current compilation target:

| Platform        | Expected filename                                 |
|-----------------|---------------------------------------------------|
| macOS ARM       | `jura-sidecar-aarch64-apple-darwin`               |
| macOS Intel     | `jura-sidecar-x86_64-apple-darwin`                |
| Windows x86-64  | `jura-sidecar-x86_64-pc-windows-msvc.exe`         |
| Linux x86-64    | `jura-sidecar-x86_64-unknown-linux-gnu`           |

To find the exact triple for the machine you are building on:

```bash
rustc -vV | grep host
```

## Adding the binary for a release build

1. Build the PyInstaller bundle from `sidecar/`:

   ```bash
   cd sidecar
   pyinstaller jura-sidecar.spec
   ```

2. Copy the output binary into this directory with the correct platform suffix:

   ```bash
   # Example for macOS ARM:
   cp dist/jura-sidecar \
       src-tauri/binaries/jura-sidecar-aarch64-apple-darwin
   chmod +x src-tauri/binaries/jura-sidecar-aarch64-apple-darwin
   ```

3. Run `cargo tauri build` as normal.

## Development workflow

In development (`cargo tauri dev`) the Rust `run()` function attempts to spawn the
binary but falls back gracefully when it is absent. Start the sidecar manually
instead:

```bash
cd sidecar
uvicorn main:app --host 127.0.0.1 --port 8200 --reload
```

## `.gitignore`

The binary files in this directory are excluded from version control by the root
`.gitignore` (they are built artefacts). Only this README is tracked.
