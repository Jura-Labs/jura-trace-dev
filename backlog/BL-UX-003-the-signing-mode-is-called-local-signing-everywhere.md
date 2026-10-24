# BL-UX-003: the signing mode is called "Local Signing" everywhere

**Status**: Open. Target v1.3.0. Decided by Paul, 21 October 2026: "keep
Local Signing" (brain `inbox/2026-10-21-vps-php2shell-patch.md:44`).
**Raised**: 22 October 2026, from the pre-v1.3.0 backlog review.
**Severity**: Medium. The website's FAQ, getting-started and protect pages
say "Local Signing"; the app says "Sovereign" and "Bedrock".

## What is wrong

The mode that signs with a key made on the user's own computer has three
names. The website already says Local Signing. The app does not:

- `ui/src/lib/components/HelpSidebar.svelte:19`: "Sovereign vs Conformant
  signing".
- `ui/src/routes/settings/+page.svelte:505`: `codename: 'Bedrock'`, shown
  in the signing-mode card.
- The help page route is `/help/bedrock-signing`
  (`HelpSidebar.svelte:19`, `help/+layout.svelte:44`), and the FAQ sends
  readers to "Help, Bedrock Signing".
- `CHANGELOG.md` v1.2.0 says "Sovereign mode" for the Verification Record.

A grep for "sovereign" or "bedrock" across `ui/src`, `src-tauri/src` and
`CHANGELOG.md`, leaving out the internal `signing_mode` value, finds 88
lines. Most are comments; the user-facing ones are the work.

## What would fix it

1. Every string a person reads says "Local Signing", in the app, the help,
   reports, PDF output and the Verification Record's human-readable text.
2. Rename the help route to `/help/local-signing` and keep
   `/help/bedrock-signing` as a redirect, so the FAQ link keeps working.
   Then tell the website session to update the FAQ wording.
3. New changelog entries use Local Signing. Old entries stay as written;
   they are a dated record.

## What not to do

- Do not change the stored value `'bedrock'` or the `signing.mode` field in
  the Verification Record format. Existing installs and records carry it.
  Change the label, not the data.
