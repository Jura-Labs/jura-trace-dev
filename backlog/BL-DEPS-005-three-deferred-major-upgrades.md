# BL-DEPS-005: three major upgrades were deferred and are still waiting

**Status**: Open. Target v1.3.0.
**Raised**: 22 October 2026, from the pre-v1.3.0 backlog review.
**Severity**: Medium. Each is easier now than when an advisory forces it.

## What is wrong

Three upgrades were set aside during v1.2.0 because each changes
behaviour:

| Upgrade | Where | Why deferred |
|---|---|---|
| reqwest 0.12.28 to 0.13.2 | Dependabot PR #25, `src-tauri` | Changes the TLS backend; the updater, URL checks and timestamping all use it |
| onnxruntime 1.24.4 to 1.30.0 | Dependabot PR #91, `sidecar` | Can shift AI-detection scores; needs a calibration run |
| tailwindcss 3 to 4 | `ui`; BL-DEPS-002 | Configuration format changes; affects every page's styles |

## What would fix it

One branch each, in this order:

1. **onnxruntime**: run the 217-image golden set and the 84-image drift
   set before and after. Accept only within the tolerance used for v1.2.0
   (`docs/calibration/numerical-stack-drift-oct2026.md`). If BL-DET-001
   changes models, do this first so the two changes are measured apart.
2. **reqwest**: check which TLS backend and root store 0.13 uses on each
   platform, then run the installer test and the live updater legs.
3. **tailwindcss**: the visual end-to-end tests (all 318 run in CI) are
   the check.

## What not to do

- Do not merge #25 or #91 as Dependabot opened them. Both need the checks
  above.
