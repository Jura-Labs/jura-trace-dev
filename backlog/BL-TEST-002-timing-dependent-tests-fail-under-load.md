# BL-TEST-002: timing-dependent tests fail when nothing is wrong

**Status**: Open. Found 5 September 2026, on the first day CI had run in
four months.
**Severity**: Medium as a defect, High as a habit. A gate that fails at
random teaches people to re-run it without reading it, which costs exactly
what BL-SILENT-001 costs, from the opposite direction.
**Raised**: 5 September 2026

## What happened

`lock_free_during_verify` (`src-tauri/tests/api_integration.rs:982-1010`)
failed on the CI run for PR #29 with:

```
AppState mutex must be free during the verify detector pipeline
(snapshot-and-drop fix regressed or not applied)
```

That message names a real and serious regression, so it reads as a genuine
finding. It was not one.

## Why we can be certain it is flaky rather than intermittent

Not inferred from re-running until it passed. PR #29 changes four files,
all of them Python requirements and one Markdown document, and **zero Rust
files**. The Rust source under test was therefore byte-identical to main's.

In the same half hour, on that identical source:

| Run | `cargo test` result |
|---|---|
| main, push-triggered | 568 lib passed, **20 of 20** integration passed |
| PR #29 | 568 lib passed, **19 of 20** integration, `lock_free_during_verify` failed |

Same code, opposite outcomes. That is proof rather than an argument.

## The mechanism

The test spawns `verify_content_inner` on a worker thread, sleeps 50 ms on
the main thread, then asserts `state.try_lock().is_ok()`. It is asserting
that the worker has reached the snapshot-and-drop point within 50 ms.

The comment says "50 ms is generous — the snapshot itself completes in
<5 ms", and on a developer machine that is true. CI runs on a 2-vCPU
shared cloud VM where scheduling jitter is normal and a thread can simply
not be scheduled promptly. Nothing is wrong with the code the test is
guarding; the test is measuring the runner.

**A correction worth recording, since I told Paul the wrong cause first.**
I attributed it to three concurrent Rust jobs competing for CPU. That is
wrong: each GitHub-hosted job gets its own VM, and `ci.yml`'s
`concurrency` block groups on `github.ref`, so it only cancels redundant
runs on the same branch. Our jobs never contended with each other. The
sensitivity is to ordinary cloud scheduling jitter, which matters because
it means the failure can recur on a quiet day and cannot be avoided by
scheduling.

## The fix

Replace the duration with a signal. The test wants "the worker has passed
the snapshot", which is a happens-before relationship, not an elapsed time.

The cheap correct version is to have the worker publish its progress and
have the assertion wait on that, or, if the production code should not grow
test hooks, to poll `try_lock()` with a deadline: attempt for up to a few
seconds and fail only if it never succeeds. Polling keeps the invariant
being tested ("the lock becomes free while verify is still running") while
removing the assumption about *when*. It stays fast in the normal case,
because it succeeds on the first poll.

**Do not fix this by lengthening the sleep.** A longer sleep is the same
bug with a lower failure rate, and it slows every run to buy that.

## The rest of the surface

Ten sleep-based points exist. They are not equally dangerous and the
distinction matters, so they are listed rather than lumped together.

**Test-side, load-sensitive, worth changing:**

| Location | What it assumes |
|---|---|
| `tests/api_integration.rs:999` | 50 ms is enough for a worker to pass a checkpoint. The failure above |
| `tests/api_integration.rs:99` | 20 ms is enough for an axum server to accept connections. Same shape, tighter margin, currently silent. Replace with a connect-retry loop |

**Test-side but sound:** `tests/api_integration.rs:1077` sleeps inside a
stub HTTP server to *simulate* a slow response. The delay is the fixture,
not an assumption about the runner.

**Production, and correct:** `lib.rs:884`, `startup.rs:220`, `:241` and
`:456` are retry backoffs; `monitor_scheduler.rs:136`, `:152` and `:248`
are poll cadence and inter-request delays. Sleeping is the intended
behaviour in all seven.

So the work is two test changes, not ten.

## Why this is worth a backlog item rather than a quiet patch

BL-SILENT-001 catalogues mechanisms that report success while doing
nothing. This is the mirror: a mechanism that reports failure while nothing
is wrong. Both end in the same place, a signal nobody reads.

The specific risk here is sharper than usual because of the assertion
message. It names the snapshot-and-drop regression explicitly, so a
spurious failure looks like the return of a known performance bug. The
first instinct on seeing it is to go looking for a regression that is not
there, and the second, after that happens twice, is to stop looking at all.

That matters more than normal right now: BL-TEST-001 is about to add
installer and updater workflows whose assertions are deliberately red at
the start. Distinguishing "red because it is telling the truth" from "red
because the runner was busy" needs to be easy, and it is easiest if the
second category does not exist.
