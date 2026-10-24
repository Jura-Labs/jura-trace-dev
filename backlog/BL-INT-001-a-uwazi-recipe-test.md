# BL-INT-001: test a Uwazi recipe before building anything further

**Status**: Open. Target v1.3.0, after BL-REC-002 (Paul, 22 October 2026:
"proceed with 1 and then recommend a recipe test initially before
proceeding further"). HURIDOCS is open to joint funding.
**Raised**: 22 October 2026, from the Uwazi integration review (brain
`leads/2026-10-22-huridocs-uwazi-integration.md`).
**Severity**: Medium. A partnership lead with a live conversation.

## Why a recipe, not code in Trace

Uwazi (HURIDOCS's open-source database for human-rights collections) has
an API, but it is not a published contract (review, read in the source at
huridocs/uwazi d3d7437):

- Login is a session cookie from `POST /api/login`, and every write needs
  an `X-Requested-With: XMLHttpRequest` header
  (`app/api/auth/CSRFMiddleware.ts:7`, confirmed). No API keys; an account
  with two-factor login cannot be scripted.
- Attachments upload to `POST /api/files/upload/attachment`
  (`app/api/files/routes.ts:83`, confirmed).
- The file record has no hash field and drops unknown fields
  (`app/shared/types/fileSchema.ts`, `additionalProperties: false`,
  confirmed), so a hash must go in a template property.
- Uwazi 2.0 (announced September 2026, no launch date) is rebuilding
  entities and attached files. Code written now may break.

## What the test is

1. A script in `docs/recipes/uwazi/` (Python, standard library plus
   `requests`): log in, find an entity, download its image, run
   `jura verify --record --format json` (BL-REC-002), upload the record ZIP
   as a supporting file, and write verification fields.
2. A Uwazi template and thesaurus as JSON: "Jura Trace SHA-256", "Jura
   Trace band" (trusted, uncertain, untrusted, inconclusive), "Jura Trace
   Content Credentials" (valid, invalid, none), engine version, check date.
   Prefix every name "Jura Trace"; Uwazi already uses `provenance` for
   something else.
3. Run it against a test Uwazi instance (self-hosted in Docker, or one
   HURIDOCS provides), with sample images only.
4. Write up what worked and what did not, and share it with HURIDOCS with
   the questions in the lead file.

Only after that, decide with HURIDOCS whether to go further (batch checks
of a collection, a supported API in 2.0, a pilot with a partner
organisation, a joint funding application).

## What not to do

- Do not use real case material. Sample images only, until a partner
  organisation agrees a pilot.
- Do not add a `jura uwazi` command to Trace until Uwazi's 2.0 API is
  known.
- Do not commit to anything with HURIDOCS in this item; that is Paul's.
