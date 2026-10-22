# BL-REL-011: no Intel Mac build

**Status**: Open. Target: later, not v1.3.0 (Paul, 22 October 2026).
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: Low until someone asks. No request is on record.

## What is wrong

The release matrix builds three targets: `aarch64-apple-darwin`,
`x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`
(`.github/workflows/release.yml:327-329`). The website FAQ says there are
no plans to support Intel Macs. Not-for-profits and Global Majority users
often run older hardware, so this may exclude part of the audience; that
is inferred, not evidenced.

## What would fix it

1. Record whether anyone asks (feedback@juralabs.org, partners).
2. If it goes ahead: an `x86_64-apple-darwin` leg, the PyInstaller sidecar
   and ONNX models built for Intel, signing and notarisation, and a
   `darwin-x86_64` entry in the updater manifest.
3. Check analysis time on a 2017 to 2020 Intel Mac before promising it.

## What not to do

- Do not build a universal binary that doubles the macOS download for
  everyone.
