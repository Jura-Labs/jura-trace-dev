# Format truth-grid rollout checks

Two scheduled verification passes for the JTV-105 format-support honesty
work that landed in commit `fa56562` on 2026-04-28. Pilot rollout window is
2026-04-28 → 2026-05-26 (four weeks). These checks ensure the truth-grid
still reflects reality at the mid-point and 4 days before launch.

Cron jobs were also scheduled (IDs `c596f106` and `25bd031f`) but are
session-only — if Claude Code restarts they will not fire. This file is
the durable record. To run a check on the target date, paste the prompt
below into a fresh Claude Code session.

## Mid-development drift check — fire 2026-05-12

Two weeks after landing, two weeks before pilot rollout. Quick sanity
sweep to catch any regressions before they pile up.

```
Mid-development drift check for the JTV-105 format-support truth-grid
(landed 2026-04-28 in commit fa56562, two weeks ago). Pilot rollout is
2 weeks away. Verify the seven items are still in place and nothing has
regressed:

1. Item 1 (HEIC) — grep -n "pillow_heif\|pillow-heif" sidecar/main.py
   sidecar/requirements.txt sidecar/requirements-ci.txt
   sidecar/tests/test_heic_codec.py — all four references should be
   present. Run `cd sidecar && python -m pytest tests/test_heic_codec.py -v`.

2. Item 2 (video trust wiring) — grep -n
   "effective_deepfake_score\|aggregate_score" src-tauri/src/lib.rs should
   show the substitution at the compute_trust call site. Run
   `cd src-tauri && cargo test --lib -- trust_video`.

3. Item 3 + 5 (picker filters) — `cd ui && npm test -- --run` should show
   45+ filter tests passing. Inspect ui/src/lib/api.ts PROTECT_FILE_FILTERS
   / VERIFY_FILE_FILTERS — confirm webm/mkv/avi/docx/gif/wav/mp3/flac/aac/
   m4a are absent.

4. Item 4 (codec gating) — grep -n "should_run_ela\|should_run_jpeg_ghost"
   src-tauri/src/format_router.rs src-tauri/src/lib.rs — helpers in
   format_router, gating call in lib.rs. Run
   `cd src-tauri && cargo test --lib -- codec_gate`.

5. Item 6 (PDF panel) — grep -n
   "Origin metadata only\|manipulation detection is not available"
   ui/src/routes/verify/+page.svelte — both copy strings present.

6. Item 7 (help page) — confirm ui/src/routes/help/format-support/
   +page.svelte exists; `cd ui && npx svelte-check` 0 errors.

Also: Plane JTV-105 + 106-112 should still be Done; if anyone reopened
them, flag it.

Report findings in under 250 words. If drift detected, propose a
remediation plan but do not implement without confirmation. If everything
is intact, state that and end.

Memory references: project_format_truth_grid_apr2026.md, reference_plane.md.
```

## Pre-rollout sweep — fire 2026-05-22

Four days before pilot launch. Final pass with broader scope: copy
hygiene, audio AUC discipline, full test suite, JTV-113 status.

```
Pre-pilot-rollout truth-grid sweep — 4 days before pilot launch. Final
pass on the JTV-105 format-support honesty work (landed 2026-04-28 in
commit fa56562). Goals:

1. Re-run the truth-grid: walk through each format family
   (JPEG/PNG/TIFF, WebP/AVIF/HEIC, MP4/MOV, PDF, audio, WebM/MKV/AVI,
   DOCX) and confirm the marketed support still matches actual pipeline
   behaviour. Use the project_format_truth_grid_apr2026.md memory as
   the baseline.

2. Pilot-tester surface check: spawn Explore over
   ui/src/routes/protect/+page.svelte and ui/src/routes/verify/+page.svelte
   looking for any user-facing copy that has crept back in mentioning
   unsupported formats (WebM, MKV, AVI, DOCX, WAV, MP3, FLAC, audio
   deepfake, etc.). Also scan README, CHANGELOG, and docs/ for the same.

3. Audio AUC 1.0 hygiene: per CLAUDE.md "Audio deepfake metrics MUST
   NOT be cited in user-facing copy". Grep ui/, docs/, CHANGELOG.md
   for "AUC 1.0" or "audio deepfake" mentions; flag any.

4. JTV-113 status: check Plane — is the v1.1 audio AASIST plan still
   in Backlog with the 10–12 week scope? Has anyone shortcut it back
   to "wire what we have"? If so, flag urgently.

5. Test suite green: `cd src-tauri && cargo test --lib`,
   `cd sidecar && python -m pytest tests/`,
   `cd ui && npm test -- --run` and `npx svelte-check`.

If issues found: prioritised fix list with rough effort. If clean:
state "ready for pilot rollout" and end.

Use /agents (Explore, persona-testing if any UX copy issues) where it
pays. Report under 400 words.
```

## Why this lives in a file rather than only in cron

Claude Code cron jobs are session-only — if the session restarts before
the fire date the job is gone. For a 4-week pilot window it is safer to
keep the prompts in the repo and run them manually from a fresh session
on the target date. The cron jobs (`c596f106` mid-check, `25bd031f`
pre-rollout) will fire automatically if the session is alive on those
dates, but this document is the source of truth.
