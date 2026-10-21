# BL-REL-009: the release page and the download page recommend different Windows installers

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, reviewing the download page after v1.2.0.
**Severity**: Low. Both installers work. A reader comparing the two pages
gets opposite advice.

## What is wrong

- The GitHub release page marks the `.msi` "recommended" and calls the `.exe`
  the "NSIS installer". That text comes from `.github/workflows/release.yml:168-169`,
  so every release repeats it.
- juralabs.org/download leads with the `.exe` and presents the `.msi` as the
  option for managed rollouts (Intune, Group Policy). The website changed
  this on 11 September 2026 (website commit `d3ade43`) because the `.msi`
  "was confusing people".

## What would fix it

Make `release.yml` match the download page: the `.exe` for a normal
install, the `.msi` for managed deployment. The page is built at tag time,
so this has to land before the v1.3.0 tag.

## What not to do

Do not change it before BL-TEST-004 has run. If the `.exe` path turns out
to make a second copy on update, the download page changes instead, and
this item flips direction.
