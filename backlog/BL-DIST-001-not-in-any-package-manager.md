# BL-DIST-001: Jura Trace is in no package manager

**Status**: Open. Target v1.3.0 (Paul, 22 October 2026): winget and a
Homebrew tap.
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: Medium for reach. Windows is 55 to 65% of downloads of every
release (GitHub release counts, 22 Oct), and IT teams install through
package managers.

## What is wrong

The only install route is the download page. There is no winget,
Homebrew, Snap or Flathub entry.

## What would fix it

1. **winget.** A manifest PR to `microsoft/winget-pkgs` per release,
   automated from the GitHub release (for example the `winget-releaser`
   action), using the signed `.exe` or `.msi` URL and its SHA-256. Check
   first how winget's upgrade and Trace's own updater interact; if they
   conflict, the winget manifest should point at an installer whose
   updater defers to winget.
2. **Homebrew.** A third-party tap, `jura-labs/homebrew-tap`, with a cask
   for the DMG. The main Homebrew cask repository needs 75 stars on the
   repository (225 to self-submit) per its acceptance policy, as reported
   by the review; the public repo has 1.
3. Add both to the release checklist and the download page.

## What not to do

- **Not Flathub.** Its requirements say "Flathub manifests must not
  contain AI-generated or AI-assisted content" and ask for disclosure of
  AI-generated code (docs.flathub.org, read 22 Oct 2026). Snap, built from
  the `.deb`, is the Linux route if one is wanted.
- Do not hand-edit manifests per release; automate or they will drift.
