# Verification Record

A Verification Record is a signed, tamper-evident record of the checks Jura
Trace ran on one file. An organisation may find it useful as supporting
evidence. It does not determine, certify, or guarantee compliance with any
law or standard, and it is not a finding that the file is authentic or
inauthentic.

It exists so that the person who ran a check can hand the result to somebody
else, and that person can confirm it has not been altered, using a tool that
Jura Labs did not write.

## Making one

In the desktop app, verify a file, then press **Verification Record** in the
actions row under the result. The app saves a ZIP named
`jura-verification-record-<file>-<time>.zip`.

Each export is noted in the app's audit log with the record's SHA-256.

## What is in the ZIP

| File | What it is |
|---|---|
| `record.json` | The record. Its exact bytes are what is signed |
| `record.json.sig` | A detached signature over `record.json` |
| `signer-cert.pem` | The signing certificate, then the certificate of the per-install authority that issued it |
| `HOW-TO-CHECK.txt` | The steps below, with this record's own hash and fingerprint filled in. It is not signed |

## Checking one

You need [OpenSSL](https://www.openssl.org/). In the unzipped folder:

```sh
openssl x509 -in signer-cert.pem -pubkey -noout > signer-pubkey.pem
openssl dgst -sha256 -verify signer-pubkey.pem -signature record.json.sig record.json
```

The second command prints `Verified OK` when `record.json` is byte for byte
what was signed. Any other output means the record or its signature has been
changed, and the record should not be relied on. Opening `record.json` in an
editor and saving it can change its bytes, so check the copy you were sent.

Then check the record is about your file. `subject.sha256` in `record.json`
is the SHA-256 of the file that was examined:

```sh
openssl dgst -sha256 <the file you were given>
```

If you need to know which copy of Jura Trace signed it, compare the
certificate's fingerprint with the one the maker reads out of their own app
(Protect page, in the details shown before signing):

```sh
openssl x509 -in signer-cert.pem -noout -fingerprint -sha256
```

## What a passing check means, and what it does not

It means the record is unchanged since it was signed by that key.

It does not mean the file is authentic. The record reports what automated
checks found, and those checks can be wrong in both directions.

It does not prove who made the record. In this release every record is
signed in **Sovereign mode**: the key is generated on the computer that made
the record and no certificate authority vouches for it. Anyone with that
computer can sign a record. The record states its signing mode in
`signing.mode`.

It does not prove when the record was made. `createdAtUtc` is that
computer's own clock. There is no independent timestamp.

## The format

`record.json` is UTF-8 JSON, format version 1.

| Field | Content |
|---|---|
| `format`, `formatVersion` | `jura-trace-verification-record`, `1` |
| `notice` | The paragraph at the top of this page |
| `createdAtUtc`, `createdAtNote` | When it was made, by the maker's clock, and a sentence saying so |
| `generator` | App name, version and platform |
| `signing` | `mode`, a description of the mode, the algorithm, the two file names and the certificate's SHA-256 fingerprint |
| `subject` | File name, SHA-256, content type and source type of what was examined |
| `summary` | Analysis mode, trust score, verdict band, the list of detectors that ran, and the engine and model versions |
| `result` | The full result, with what each detector reported |

Two things are left out of `result` on purpose. Absolute paths on the
maker's computer are replaced with `[local path left out]`, because they name
a user account and say nothing about the file. Very long values, such as an
embedded image, are replaced with their length and SHA-256.

The signature is ECDSA P-256 with SHA-256, in ASN.1 DER, over the bytes of
`record.json` as they are in the ZIP. Nothing is canonicalised.

## Limits in this release

- Desktop app only. The REST API and the `jura` CLI do not produce records.
- Sovereign mode only.
- No analyst name, case reference or notes. Those are in the PDF report.
- Images only, as for the rest of the app.

Source: `src-tauri/src/verification_record.rs`. Its tests run the two
OpenSSL commands above against a real export, change one byte, and require
the check to fail.
