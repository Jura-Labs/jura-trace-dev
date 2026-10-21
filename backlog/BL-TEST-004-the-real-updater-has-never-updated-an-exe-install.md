# BL-TEST-004: the real updater has never updated a Windows install made with the .exe

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, reviewing the download page's choice of
installer.
**Severity**: Medium. Nothing is known to be broken. But juralabs.org/download
has offered the `.exe` (NSIS) as the main Windows download since 11 September
2026, and the path every one of those users takes on update has never run
end to end.

## What is wrong

The update manifest gives Windows one entry, `windows-x86_64`, and it points
at the `.msi` (checked 22 Oct: `JuraTrace-1.2.0-Windows-x64.msi`). A user who
installed with the `.exe` is therefore updated by the `.msi`.

Two tests come close, and neither covers that path:

- `.github/workflows/updater-live-leg.yml` installs the previous release
  from the `.msi` (`:158`, `:438-440`) and updates it. It never starts from
  the `.exe`.
- `.github/workflows/installer-test.yml` installs the `.exe`, then runs the
  `.msi` by hand with `msiexec /qn` to stand in for an update, and checks
  that no second copy appears (`:678-714`). It passed on 7 September and on
  21 October (run 39564088691): after the `.msi`, the only copy was the
  `.exe`'s, in `%LOCALAPPDATA%\Jura Trace`.

The second test is the evidence that the split does not happen. It is not
proof, because the real updater may run the `.msi` with different arguments
than `/qn`. And installed alone on a clean machine, the `.msi` goes to
`C:\Program Files\Jura Trace` (log, 7 Sep 2026), so it is able to make a
second copy.

## Why it matters

If the real updater does make a second copy, every `.exe` user gets an
updated copy they never open, while their shortcuts still start the old
version. Nothing tells them. The 3 September 2026 review predicted exactly
this, and the hand-run test has not seen it since.

## What would fix it

- Add an `.exe` leg to `updater-live-leg.yml`: install the previous
  release's `setup.exe` with `/S`, launch it, let the real updater install
  the new version, then check that there is only one copy, that it reports
  the new version, and that the Start menu shortcut starts that copy.
- Run it on the v1.3.0 release, starting from v1.2.0's `.exe`.
- Then settle the 7 September caveat with the `.msi` first on a clean
  machine, then the `.exe`, as `docs/release/v1.2.0-plan.md` asks.

## What not to do

- Do not soften the parallel-install check in `installer-test.yml`. Its
  comment explains why.
- Do not switch the download page back to the `.msi` on this item alone.
  The `.exe` leads for clarity (website commit `d3ade43`, 11 Sep 2026), and
  the evidence so far does not show the split. Decide after the new leg has
  run.
