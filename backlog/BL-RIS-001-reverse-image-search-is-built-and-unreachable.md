# BL-RIS-001: reverse image search is built and unreachable

**Status**: Open. Target v1.3.0, by Paul's decision of 22 October 2026.
**Raised**: 22 October 2026.
**Severity**: Medium. It is the feature users most often expect from a
tool like this, and the app tells them to go elsewhere.

## What is wrong

`src-tauri/src/ris.rs` (1,183 lines, 33 tests) holds the key store and
provider list for bring-your-own-key reverse image search (JTV-98). Its
header says it is not declared in `lib.rs`, not wired to any command and
not called from the frontend (`ris.rs:5-20`), so it does not compile into
the shipped app. End-to-end use against live Google Vision was checked on
4 May 2026.

Meanwhile the verdict summary tells the user that "Jura Trace does not
perform reverse image search itself in v1.0"
(`ui/src/lib/components/VerdictSummary.svelte:203`), a sentence that is
already out of date in its version number.

## The design already decided (May 2026)

- Keys in the operating system's key store via the `keyring` crate, never
  in a file; Linux without a secret service fails rather than storing in
  plain text (`ris.rs:27-33`).
- Provider list `google_vision`, `tineye`; Google Vision only at first,
  TinEye behind a flag until its smoke test is done.
- Hash-only by default, and a per-search confirmation before the image's
  bytes are sent to a provider.

## What would fix it

1. Wire `ris.rs` into `lib.rs`, add the commands, and build the Settings
   key entry and a "Search for this image" button on the verify page.
2. Enhanced network mode only, through the outbound gate
   (`network_mode.rs:240`). In Standard mode the button explains why it is
   off.
3. The confirmation says, in plain words, that the image will be sent to
   Google under the user's own account and key.
4. Run TinEye's smoke test before showing it.
5. Change `VerdictSummary.svelte:203` and the help, compliance and monitor
   pages that mention RIS (`ui/src/routes/help/verify`,
   `help/compliance`, `monitor`).
6. claims.md row 6 and the README's network disclosure list must name
   Google Vision before this ships (BL-CLAIM-001 history).

## What not to do

- Do not send image bytes without the per-search confirmation, ever.
- Do not ship a Jura Labs key. Searches run on the user's own key and
  account.
- Do not present search results as evidence of authenticity. They show
  where else an image appears, nothing more.
