# BL-MON-001: Watched Locations is built and hidden

**Status**: Open. Target v1.3.0, by Paul's decision of 22 October 2026.
One question for Paul below.
**Raised**: 22 October 2026.
**Severity**: Medium. The Monitor tab ships empty of its main feature.

## What is wrong

`V1_SHOW_WATCHED_LOCATIONS = false` (`ui/src/lib/featureFlags.ts:140`)
hides the Monitor tab's Watched Locations section. The backend ships:
`src-tauri/src/monitor_scheduler.rs` (703 lines; 23 tests per the flag's
comment), four IPC commands and the `monitor_urls` and `monitor_events`
tables. The flag's comment planned to re-enable it in v1.1 (JTV-206),
reframed as a "C2PA Manifest Integrity Monitor": the scheduler already
records `c2pa_stripped` and `c2pa_changed` events.

The scheduler runs only on the Professional and Enterprise tiers and idles
on Community (`monitor_scheduler.rs:10-11,113-124`). There is no Pro tier
in any shipped release, so turning on the flag alone would show a feature
that never runs.

Watched Locations fetches are already gated as Enhanced-mode traffic
(`src-tauri/src/network_mode.rs:7`).

## Question for Paul

The tier decision of 9 May 2026 put "unlimited Watched Locations" in Pro,
which implies a limited number in Community. Pro is not live. For v1.3.0:
a fixed number of watched URLs in Community (say five), or no limit until
Pro exists?

## What would fix it

1. Remove the Pro/Enterprise gate in `monitor_scheduler.rs`, replaced by
   the Community limit Paul chooses.
2. Turn on the flag, with the reframing above: it watches a page where
   your image is published and tells you if its Content Credentials are
   stripped or changed.
3. Enhanced mode only; in Standard mode the section explains why it is off.
4. The check interval and what is fetched are stated in Settings.
5. Check the 23 tests still pass and add one for the Community limit.

## What not to do

- Do not describe it as finding copies of an image elsewhere. That is
  reverse image search (BL-RIS-001). The name changed from "URL Watchlist"
  in April 2026 for exactly that reason.
