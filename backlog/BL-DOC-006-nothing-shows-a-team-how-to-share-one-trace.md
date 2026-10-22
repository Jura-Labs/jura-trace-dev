# BL-DOC-006: nothing shows a team how to share one Jura Trace

**Status**: Open. Target v1.3.0 (Paul, 22 October 2026). After BL-REC-002.
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: Medium. Newsrooms and human-rights teams work as desks, and
the pieces exist without a page that puts them together.

## What is wrong

v1.2.0 ships a Linux server download (`jura-trace-api`, `jura`, the engine
and models) and a CLI that takes `--api-url` or `JURA_API_URL`. The only
team guidance is a sentence about putting the SQLite database on a shared
network drive (`docs/user-guide/getting-started.md:97`), which is not a
safe pattern for SQLite.

Demand is inferred, not recorded: the newsroom personas in the help
(`ui/src/routes/help/personas`) and the HURIDOCS conversation.

## What would fix it

One page, "One server, many desks":

1. Run `jura-trace-api` on a Linux machine inside the team's network, as
   a service, with one key per person or per script.
2. Point each desk's `jura` at it with `JURA_API_URL`.
3. Write each check's JSON and Verification Record (BL-REC-002) to a
   shared folder or the team's case system, with `--analyst` and `--case`.
4. What the server keeps, how long, and how to clear it (`--ephemeral`
   for a session that leaves nothing).
5. Security: the API listens on `127.0.0.1` by default. Say plainly what
   changes if it is exposed to a network, and recommend a reverse proxy
   with TLS rather than binding it to all interfaces.

## What not to do

- Do not recommend the shared-drive database again; remove that sentence
  in BL-DOC-005.
- Do not add multi-user accounts to the API for this. Keys per person are
  enough until someone asks for more.
