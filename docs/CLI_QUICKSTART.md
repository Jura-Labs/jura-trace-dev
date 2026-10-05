---
title: "Jura Trace from the command line"
description: "Run Jura Trace's analysis from a terminal or a script: the jura client and the jura-trace-api server, where the installers put them, getting a key, and a first verification."
last-updated: 4 October 2026
status: v1.2.0
---

# Jura Trace from the command line

From v1.2.0 every Jura Trace installer includes two command-line programs
beside the app:

- **`jura`**, the client. It sends a file to Jura Trace, waits for the
  analysis, prints the result, and exits with a code a script can act on.
- **`jura-trace-api`**, the analysis server without the app's window, for
  machines nobody sits at.

Everything is analysed on your own machine, as in the app. `jura` talks to
Jura Trace at `http://127.0.0.1:8300`, which is either the app (while it is
open) or `jura-trace-api`. Only one of the two can use that address at a
time.

A result is evidence for a person to weigh: a trust score, the band the app
shows, and what each detector found. It is not a finding that a file is
genuine or fake.

## 1. Find the programs

| | `jura` | `jura-trace-api` |
|---|---|---|
| macOS | `/Applications/Jura Trace.app/Contents/MacOS/jura` | `/Applications/Jura Trace.app/Contents/MacOS/jura-trace-api` |
| Windows (setup.exe or .msi) | `%LOCALAPPDATA%\Jura Trace\jura.exe` | `%LOCALAPPDATA%\Jura Trace\jura-trace-api.exe` |
| Linux, .deb | `/usr/bin/jura` | `/usr/bin/jura-trace-api` |
| Linux server tarball | `jura` in the folder you unpack | `jura-trace-api` beside it |

On Linux the .deb puts both on your `PATH`. The AppImage carries them inside
the image, where they are not easily run; use the .deb for command-line work.

**A Linux server with no desktop** can use the server tarball from the
release page, `JuraTrace-<version>-Linux-x86_64-server.tar.gz`. It holds
`jura-trace-api`, `jura`, the analysis engine and its models, needs no
desktop libraries (only glibc 2.35 or later), and has a `README.txt`.

**macOS**, to type `jura` rather than the full path, link both into a
directory on your `PATH`:

```bash
sudo mkdir -p /usr/local/bin
sudo ln -sf "/Applications/Jura Trace.app/Contents/MacOS/jura" /usr/local/bin/jura
sudo ln -sf "/Applications/Jura Trace.app/Contents/MacOS/jura-trace-api" /usr/local/bin/jura-trace-api
```

**Windows**, in PowerShell, to add the install folder to your own `PATH`.
Open a new terminal afterwards.

```powershell
$dir = "$env:LOCALAPPDATA\Jura Trace"
$user = [Environment]::GetEnvironmentVariable("Path", "User")
[Environment]::SetEnvironmentVariable("Path", "$user;$dir", "User")
```

Check:

```bash
jura --version
```

## 2. Create an API key

Every request needs a key. Create one; it is printed once, so keep it:

```bash
jura-trace-api keys add --name my-laptop
```

The key goes into the same database the app uses, so it works with the app
and with `jura-trace-api` alike. Then store it in your operating system's
keyring, where `jura` will find it. The `-` reads the key from what you
paste, which keeps it out of your shell history:

```bash
jura auth set-key -
jura auth status
```

In CI, or anywhere without a keyring, set `JURA_API_KEY` instead, and
`JURA_NO_KEYRING=1` to stop `jura` looking for one.

## 3. Start Jura Trace

Either open the app, or run the server on its own:

```bash
jura-trace-api
```

The analysis engine takes a few seconds to start (longer the first time
after installing). `jura verify --wait-ready` waits for it.

## 4. Verify a file

```bash
jura verify photo.jpg --wait-ready
```

```
photo.jpg
  verdict      uncertain (0.55)
  mode         deep
  detectors    10 run
  sha256       9f2c1e...
  engine       1.2.0   sidecar 1.2.0   deep   2026-11-02T09:15:44Z
  note         capped: the AI-image detector was inconclusive
```

That text is for people, and can change in any release. A script reads JSON,
which is the API's response, field for field:

```bash
jura verify photo.jpg --format json > photo.result.json
```

The fields are described in [API_WRAPPER.md](API_WRAPPER.md#the-verification-result).
Keep `provenance` with the result: it records the engine version and model
hashes that produced it.

## 5. In a script

`jura verify` exits 0 whenever an analysis was produced, including a
low-trust one. To act on the verdict:

```bash
jura verify "$f" --format json --fail-on uncertain > "$f.json"
case $? in
  0)  echo "trusted" ;;
  20) echo "uncertain or untrusted: review $f" ;;
  8)  echo "no usable verdict for $f (inconclusive, or the engine was not ready)" ;;
  2)  echo "Jura Trace is not running" ;;
  *)  echo "jura failed on $f" ;;
esac
```

Every exit code is in [API_WRAPPER.md](API_WRAPPER.md#exit-codes), with
`--require-complete` (exit 8 when some analysis services were unavailable)
and the rest of the options. `jura --help` and `jura verify --help` list them
too.

## Several files at once

```bash
jura verify *.jpg --format ndjson > results.ndjson
```

writes one line per file, each naming the file, its own exit code, and
either the full result or the error. The whole command exits non-zero if any
file failed.

## Signing

```bash
jura sign photo.jpg --creator "Ada Lovelace" --license CC-BY-4.0
```

adds Content Credentials naming you as the creator and writes
`photo_signed.jpg` beside the original, which is left unchanged. If Jura
Trace is in Standard network mode and nobody has chosen whether signatures
carry a trusted timestamp (which means a request to a timestamp server), add
`--timestamp` or `--no-timestamp`.

## A session that leaves nothing behind

```bash
jura-trace-api --ephemeral
```

starts a server with a new, empty database in the temporary directory and
prints a key for that session. When you stop it with Ctrl+C, the database and
everything the session wrote are deleted. It also uses Standard network mode,
so it makes no network request you did not ask for. A process that is killed
rather than stopped leaves its folder behind; the server prints where it is
when it starts.

## Running `jura-trace-api` as a service

It stays in the foreground and stops cleanly on Ctrl+C or SIGTERM, taking its
analysis engine with it. To run it at boot, use your system's own service
manager (launchd, systemd, or the Windows Task Scheduler) with the full path
from step 1. It listens on `127.0.0.1` only. `jura-trace-api --help` lists
its options, including `--port` and `--db`.
