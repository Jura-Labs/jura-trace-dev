# BL-CLAIM-003: the exported forensic report asserts checks that never ran

**Status**: Open. Found 4 September 2026.
**Severity**: High, and the highest legal exposure found this week. This is
a false forensic claim inside the artefact a user hands to somebody else.
**Recommend a legal-compliance-advisor pass before it is corrected**, so
the replacement wording is right first time.

## What is wrong

`ui/src/lib/pdf.ts:676-684` writes into the exported trust report:

```ts
m.verificationMode === 'enhanced'
  ? 'Enhanced (OCSP/CRL + remote manifest fetch)'
  : 'Standard (offline, local trust anchors only)'
```

**Neither OCSP nor CRL checking happens.** `src-tauri/src/c2pa.rs:1536-1568`
records that `enhanced` selects the recorded *label* only, and
`c2pa.rs:74` and `:1542` note that c2pa-rs 0.79 does not expose revocation
checking at all. `src-tauri/Cargo.toml:63` compiles the crate without the
remote-fetch feature. So the mode is a string.

The same false assertion appears in the UI: `ui/src/lib/api.ts:1373`, and a
settings toast at `ui/src/routes/settings/+page.svelte:1134` telling the
user "Online verification features are now active".

## Why the PDF is the serious one

The other two are screen text, correctable in the next release with no
lasting artefact. The PDF is different: it is generated, saved, attached to
emails, filed in case notes and handed to third parties. Every report
exported in Enhanced mode since April is a document asserting that
certificate revocation was checked when it was not, over Jura Labs'
name.

That is precisely the class of claim the brain's claims register exists to
prevent, and it is worse than the register's existing stale rows because it
is machine-generated at scale rather than written once on a web page.

For the intended users — a journalist supporting a story, an investigator
building a case file — a provenance report that overstates what was
verified is the most damaging possible error. It is also the kind that
surfaces at the worst moment, when somebody competent checks.

## A second fault in the same area: it fails open

`src-tauri/src/network_mode.rs:66-79` returns **Enhanced** when the config
cannot be read. So a user who deliberately selected offline operation, and
whose config is then unreadable, is silently switched to the mode that
makes network calls. The safe default for a local-first tool is the
offline one, and the failure direction here is backwards.

## What would fix it

1. **Correct the PDF wording first**, because it is the only output that
   persists. Say what actually happens: the signature chain is validated
   against local trust anchors, and revocation status is not checked.
2. **Correct the UI label and the settings toast** to match.
3. **Invert the `network_mode` fallback** so an unreadable config yields
   Standard, not Enhanced.
4. **Decide what "Enhanced" should mean**, if anything, while c2pa-rs 0.79
   cannot do revocation. Either rename it to what it does or hide it until
   the c2pa bump (SR-34) makes the claim true.

   **Correction, 5 September.** An earlier draft of this item said "the
   weather lookup and remote manifest fetch are real". Only the weather
   lookup is. `src-tauri/Cargo.toml:63` builds c2pa with
   `features = ["file_io"]`, so remote manifest fetching is not compiled
   in. Enhanced gates exactly two outbound calls, the historical weather
   lookup (`lib.rs:1175`) and the Watched Locations scheduler
   (`monitor_scheduler.rs:177`), and neither touches C2PA. Watched
   Locations is flag-hidden in v1.0, so for nearly every user the setting
   changes nothing observable.

   **The C2PA path is byte-identical in both modes**, confirmed at
   `c2pa.rs:1565`: `let mode_str = if enhanced { "enhanced" } else
   { "standard" };`. The flag reaches `extract_manifest_info` as a label
   and nothing else.

   Worth quoting the existing doc comment at `c2pa.rs:74`, because it
   states the defect precisely without meaning to: the field exists so the
   UI and PDF "can accurately report whether online checks were
   **attempted**". The PDF reports that they **happened**. The whole fault
   sits in the gap between those two words.

   Filed for decision as D7 and D8 in
   `~/jura-brain/inbox/2026-09-05-trace-four-decisions.md`, with proposed
   replacement wording drafted so a legal pass has something concrete to
   react to.
5. **Add a row to the brain's `claims.md`** covering the exported report's
   assertions, since it is a published claim surface that the register does
   not currently track at all.

## What not to do

Do not simply delete the Verification mode row from the report. A forensic
document should say what was and was not checked; removing the line leaves
the reader to assume. Replace it with an accurate statement.

Do not fix the label and leave the fallback. A correct label on a mode the
user did not choose is still wrong.
