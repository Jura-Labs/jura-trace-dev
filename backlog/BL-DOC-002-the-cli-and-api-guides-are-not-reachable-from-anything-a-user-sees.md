# BL-DOC-002: the CLI and API guides are not reachable from anything a user sees

**Status**: Open. Target v1.3.0.
**Raised**: 21 October 2026, checking v1.2.0 was live before the
announcement.
**Severity**: Medium. Nothing is broken and nothing is unsafe. But the
command line and the headless API are the main reason to install v1.2.0,
and the people they are for (newsroom developers, researchers scripting a
batch) only get as far as the in-app help's short version.

## What is wrong

Two full guides exist, and both are current for v1.2.0:

- `docs/CLI_QUICKSTART.md` (189 lines): finding the programs, making a
  key, verifying a file, scripting, batches, signing, ephemeral sessions,
  running `jura-trace-api` as a service.
- `docs/API_WRAPPER.md` (899 lines): authentication, modes, endpoints,
  the result schema, errors, limits, the `jura` CLI, examples.

They live in `Jura-Labs/jura-trace-dev`. Nothing a user lands on links to
them:

1. **The release page cites bare paths.** `CHANGELOG.md:26`, `:39`, `:43`
   and `:161` say "See `docs/CLI_QUICKSTART.md`" and similar. The release
   page is on `Jura-Labs/jura-trace`, whose `docs/` holds only `branding/`
   and `methodology.md`, so every one of those paths is a dead end there.
   They are code spans, not links.
2. **`GETTING_STARTED.md` on the public repo is out of date.** Line 38
   describes the REST API as "used by the desktop app itself". It says
   nothing about `jura`, `jura-trace-api`, API keys or the Linux server
   tarball.
3. **The website says nothing.** On 21 October 2026,
   `https://juralabs.org/download` mentioned neither the API nor the
   command line. (It also still offered v1.1.0, which is a separate
   release-day step, not this item.)
4. **The in-app help is the only route, and it is the short version.**
   `ui/src/routes/help/settings/+page.svelte:564` onward ("API keys and the
   command line") covers keys, where the programs are installed and three
   commands. It does not link to either guide, so a user who needs
   `--format ndjson`, `--fail-on`, `--ephemeral` or the endpoint list has
   nowhere to go.

5. **`docs/API_WRAPPER.md` still tells the v1.1.0 story about keys.** Its
   "Obtaining a key: the honest position" section opens "In v1.1.0 there is
   no supported way for a user to obtain a key" and calls the Settings
   panel switched off. The Examples section says getting a key is "not yet
   straightforward". In v1.2.0 Settings, API Keys works in every edition and
   `jura-trace-api keys add` exists. Found 22 October 2026, drafting the
   announcement.

## Why it matters

The v1.2.0 announcement needs a walkthrough to point to. As it stands, it
would have to link into the development repository by hand.

## What would fix it

- Publish both guides somewhere public and stable: in `Jura-Labs/jura-trace`
  under `docs/`, or as pages on juralabs.org. Pick one canonical home and
  link to it from everywhere else.
- Make the `CHANGELOG.md` references real links to that home. The release
  page is built from the first `## ` block (`release.yml:130-135`), so this
  has to happen before a tag.
- Update `GETTING_STARTED.md` with a short "Command line and API" section
  that links to both guides.
- Add a "Full guide" link at the end of help section 7.
- Add a line and link to the download page, next to the Linux server
  tarball.
- Rewrite the key section of `docs/API_WRAPPER.md` for v1.2.0: Settings,
  API Keys first, then `jura-trace-api keys add`, and drop "not yet
  straightforward" from Examples.

## What not to do

- Do not copy the guides into a second place by hand. Two copies will drift
  apart, as the API description in `GETTING_STARTED.md` already has. Keep one
  source and either sync it in the release workflow or link to it.
- Do not point users at `jura-trace-dev` as the documentation home. It is
  the development repository and moves with every branch.
