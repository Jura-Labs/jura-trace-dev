# BL-OBS-001: nobody reads the adoption signals we already have

**Status**: Open. Target: later, not v1.3.0 (Paul, 22 October 2026).
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: Low. Nothing is broken; the data exists and goes unread.

## What is there

Trace collects nothing, by design. Three outside signals exist:

1. **Update checks.** The updater worker counts one per request, per day,
   per platform bucket, in KV namespace `UPDATE_COUNTS`, kept 100 days
   (`infrastructure/cloudflare-worker-updater/src/index.ts:111-128`,
   called before the cache at `:247-259`). Read 22 October 2026: 34 daily
   keys since 6 September, mostly 1 to 11 checks a day, 34 on release day.
   Test runs are mixed in; four on 22 October were a review's own
   requests.
2. **Download clicks.** juralabs.org/download already tags each button for
   Umami (`download-macos`, `download-windows-exe`, `download-linux`,
   `download-linux-appimage`, `download-linux-server`,
   `download-practice-pack` and others, read 22 October). The brain's
   `products/trace.md` says download clicks are not recorded; that is out
   of date.
3. **GitHub release downloads** per asset, which include CI and crawlers.

## What would fix it

1. A weekly read of all three into one line in the brain (a script, read
   only).
2. Separate test traffic: the live updater legs could send a header the
   worker counts in its own bucket.
3. Correct `products/trace.md`.

## What not to do

- Do not add telemetry to the app. Counting at the edge is the boundary.
