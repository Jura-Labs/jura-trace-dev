# BL-REC-001: the Verification Record cannot show when it was made

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 review of legislation.
**Severity**: Medium. The record's time is the user's own clock, which
anyone can set.

## What is wrong

`docs/VERIFICATION_RECORD.md:72-73`: "It does not prove when the record was
made. `createdAtUtc` is that computer's own clock. There is no independent
timestamp."

Trace already obtains RFC 3161 timestamps when it signs Content
Credentials (`src-tauri/src/c2pa.rs:323`, `TSA_URL`;
`sign_*` takes `tsa_url` at `:474-487`), and v1.2.0 asks before doing so
in Standard network mode.

The EU Code of Practice on AI-generated content (June 2026, sub-measure
2.1.2) describes a downloadable detection result as signed and including a
hash of the content, an identifier of the tool and a timestamp. That binds
AI providers, not Trace, but it is the format readers will come to expect.
The record already has the first two.

## What would fix it

1. Optional, off by default: when exporting a record, offer "Add an
   independent timestamp". Explain that it sends a hash of the record's
   signature to a timestamp service, as signing does.
2. Ask in Standard mode, as signing does; offer by default in Enhanced
   mode. Never send the image or the record.
3. Store the timestamp token in the ZIP and document how to check it with
   OpenSSL (`openssl ts -verify`) in `VERIFICATION_RECORD.md`.
4. Update the record's "what it does not prove" text: with a timestamp, it
   shows the record existed by that time.

## What not to do

- Do not change the record's signing key or mode. The timestamp is added
  alongside; a record without one stays valid.
