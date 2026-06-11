---
title: "Verifying Jura Trace Downloads — SHA-256 Checksums"
description: "How to verify the integrity of Jura Trace installer files using SHA-256 checksums before installation."
last-updated: 25 March 2026
status: internal
---

# Verifying Jura Trace Downloads — SHA-256 Checksums

Before installing any software, it is good practice to confirm that the file you downloaded is identical to the file Juralabs published. SHA-256 checksums are a standard method for doing this. A checksum is a fixed-length fingerprint computed from the file's contents — if even a single byte has changed (due to a corrupted download, a network error, or tampering in transit), the fingerprint will not match. Verifying it takes under a minute and provides a firm bedrock of trust before you run the installer on your machine.

---

## Contents

1. [Where to find the official checksums](#where-to-find-the-official-checksums)
2. [Verifying on macOS](#verifying-on-macos)
3. [Verifying on Windows](#verifying-on-windows)
4. [Verifying on Linux](#verifying-on-linux)
5. [If the hash does not match](#if-the-hash-does-not-match)

---

## Where to find the official checksums

Each Jura Trace release includes a `SHA256SUMS.txt` file published alongside the installers on the GitHub Releases page:

```
https://github.com/juralabs/jura-archive/releases
```

On the release page, scroll to the **Assets** section. You will see the installer files listed together with `SHA256SUMS.txt`. Download `SHA256SUMS.txt` to the same folder as your installer before running the verification commands below.

> **Note**: Always download `SHA256SUMS.txt` directly from the GitHub Releases page. Do not accept checksums sent by email or shared via other channels — the releases page is the authoritative source.

---

## Verifying on macOS

Open **Terminal** (you can find it in **Applications → Utilities**, or search for it with Spotlight).

Navigate to your Downloads folder, then run:

```bash
shasum -a 256 Jura-Trace_X.X.X_aarch64.dmg
```

Replace `X.X.X` with the version number you downloaded, and adjust the filename if you downloaded the Intel build (`_x64.dmg`).

The command prints a 64-character hex string followed by the filename. Compare this string to the corresponding line in `SHA256SUMS.txt`.

**To verify all files at once** using the checksums file:

```bash
shasum -a 256 -c SHA256SUMS.txt
```

Each file listed in `SHA256SUMS.txt` that is present in the current directory will be checked. A passing result shows `OK` next to each filename.

---

## Verifying on Windows

Two methods are available. Both produce the same result.

### Method 1 — Command Prompt (certutil)

Open **Command Prompt** (press `Windows + R`, type `cmd`, press Enter).

Navigate to your Downloads folder:

```
cd %USERPROFILE%\Downloads
```

Run the checksum command:

```
certutil -hashfile Jura-Trace_X.X.X_x64_en-US.msi SHA256
```

Replace `X.X.X` with the version number and adjust the filename if you downloaded the NSIS setup file (`_x64-setup.exe`).

The command prints the SHA-256 hash on a single line. Compare it to the corresponding entry in `SHA256SUMS.txt`.

### Method 2 — PowerShell (Get-FileHash)

Open **PowerShell** (press `Windows + S`, search for "PowerShell", press Enter).

```powershell
Get-FileHash "Jura-Trace_X.X.X_x64_en-US.msi" -Algorithm SHA256 | Select-Object Hash
```

PowerShell prints the hash in uppercase. The value should match the corresponding entry in `SHA256SUMS.txt` (the comparison is case-insensitive).

---

## Verifying on Linux

Open a terminal and navigate to the directory containing the downloaded file.

```bash
sha256sum Jura-Trace_X.X.X_amd64.AppImage
```

Replace `X.X.X` with the version number. Adjust the filename if you downloaded the `.deb` package.

**To verify against the checksums file:**

```bash
sha256sum -c SHA256SUMS.txt
```

Each listed file present in the current directory is checked. A passing result shows `OK` next to each filename. Lines for files not present in the directory print a warning — this is expected if you only downloaded one installer format.

---

## If the hash does not match

If the hash you calculate does not match the value in `SHA256SUMS.txt`:

1. **Do not install the file.**
2. Delete the downloaded installer.
3. Re-download the installer from the GitHub Releases page.
4. Re-run the verification command before proceeding.

If the hash still does not match after a fresh download, contact your Juralabs pilot coordinator at pilot@juralabs.org. Include the hash you calculated, the filename, and the version number. Do not run the installer until the discrepancy is resolved.

---

*Jura Trace is developed by Juralabs Community Interest Company (UK). Licenced under AGPL-3.0-or-later. Local-first — no data leaves your machine.*
