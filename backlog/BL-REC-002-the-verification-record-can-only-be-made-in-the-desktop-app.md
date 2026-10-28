# BL-REC-002: the Verification Record can only be made in the desktop app

**Status**: Open. Target v1.3.0 (Paul, 22 October 2026). First step of
the HURIDOCS/Uwazi work (BL-INT-001) and of the newsroom setup
(BL-DOC-006).
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption and Uwazi
reviews.
**Severity**: Medium. The CLI and API, new in v1.2.0, cannot produce the
one output meant to be handed to someone else.

## What is wrong

`docs/VERIFICATION_RECORD.md`, "Limits in this release" (`:98-102`):

- "Desktop app only. The REST API and the `jura` CLI do not produce
  records."
- "No analyst name, case reference or notes. Those are in the PDF report."

So a script, a newsroom server or an integration with a case-management
system such as Uwazi cannot attach a record to what it checked.

## What would fix it

1. `jura verify <file> --record <dir>` writes the signed record ZIP next
   to the JSON result, one per file, named by the file's SHA-256.
2. The API: an optional `record=true` on `POST /api/v1/verify` returns the
   record (or a link to fetch it). Mark it `[schema]`, additive.
3. Optional `analyst`, `caseRef` and `note` fields, set from the app, the
   CLI (`--analyst`, `--case`) or the API, written into `record.json` and
   covered by the signature.
4. Same signing key and format as the desktop record; a record made by
   the CLI checks with the same OpenSSL steps.
5. Update `VERIFICATION_RECORD.md`, `CLI_QUICKSTART.md` and
   `API_WRAPPER.md`.

## What not to do

- Do not let the API write records to arbitrary server paths. Return the
  ZIP; the client decides where it goes.
- Analyst names are personal data. They stay in the record the user
  exports and are not logged.
