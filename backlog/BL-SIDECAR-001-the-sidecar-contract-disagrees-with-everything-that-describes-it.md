# BL-SIDECAR-001: the sidecar's contract disagrees with everything that describes it

**Status**: Open. Found 11 September 2026, reading the sidecar's HTTP and
configuration surface while planning the v1.2.0 headless API and CLI.
**Raised**: 11 September 2026
**Severity**: Medium. Four of the five findings are dead weight and stale
prose. The fifth is a security test that cannot fail, which is the one that
justifies the item having a severity at all.

## What is wrong

Five separate disagreements between the Python sidecar and the things that
describe it: its Rust client, its own configuration object, its own module
docstring, its test suite, and the scripts that call the REST API in front
of it. They are filed together because they are one shape. Nothing here is
a bug in the sidecar's behaviour. Everything here is something that says
the sidecar behaves differently from how it does.

### 1. Five Rust client methods post to routes that were deleted

`src-tauri/src/sidecar.rs` still carries:

| Method | Line | Posts to |
|---|---|---|
| `check_video_metadata` | 1507 | `/forensics/video/metadata` |
| `check_audio_metadata` | 1532 | `/forensics/audio/metadata` |
| `analyse_video_deepfake` | 1558 | `/forensics/video/deepfake` |
| `transcribe` | 1591 | `/forensics/transcribe` |
| `detect_audio_deepfake` | 1623 | `/forensics/audio/deepfake` |

None of those routes exists. `sidecar/app/api/forensics.py:555-562` is the
tombstone:

```python
# JTV-138 (2026-05-02): five video/audio routes lived here (video/metadata,
# audio/metadata, video/deepfake, video/frames, transcribe). Removed for
# v1.0 to close a reachable-but-ungated HTTP surface on the local sidecar
# port; the Rust pipeline gates ENABLE_VIDEO_DEEPFAKE_GROUP and
# ENABLE_AUDIO_GROUP to false so nothing in the v1.0 verify path calls
# them.
```

The gates are real: `src-tauri/src/verify/pipeline.rs:1103` and `:1217` are
both `const ... : bool = false`. So the methods are unreachable rather than
broken, and would return 404 the moment a gate were flipped. Twelve unit
tests in `src-tauri/src/sidecar.rs` (`:2645` through `:3046`) exercise the
deserialisation of their result types and pass, so `cargo test` reports
green coverage over a path that cannot execute.

This matters for v1.0.1, which `project_v1_video_audio_drop.md` says is
where video and audio return. Whoever flips those gates will find the
client pointing at nothing, and the tests will not have warned them.

### 2. Two settings nothing reads

`sidecar/app/config.py:13-14`:

```python
    sidecar_port: int = 8200
    sidecar_log_level: str = "INFO"
```

`model_config` at `:40` sets `env_prefix = "JURA_"`, so those advertise
`JURA_SIDECAR_PORT` and `JURA_SIDECAR_LOG_LEVEL`. `grep` over the whole
repository finds no reader of either name and no reader of either field.
What actually binds is argv, at `sidecar/main.py:212-216`, defaulting to
`127.0.0.1` and `8200`. Setting the environment variable changes nothing
and warns about nothing.

### 3. Two docstrings say the opposite of the code

`sidecar/app/config.py:35-37`:

```
    # If empty (default), the sidecar accepts all requests — suitable for
    # development but not recommended for shared workstations.
```

`sidecar/main.py:17`:

```
    If JURA_SIDECAR_KEY is empty (default), no authentication is enforced.
```

Since 21 May 2026 an empty key returns 503 to every `/forensics` request.
`sidecar/main.py:166-178` is the code, and its own comment at `:168-171`
describes the change the two lines above have not caught up with.

### 4. The only test of that path cannot fail

`sidecar/tests/test_api_auth.py:90-101`,
`test_no_auth_when_key_not_configured`, sets an empty key and asserts:

```python
        # 422 = reached the handler, auth middleware was a no-op.
        assert response.status_code != 401
```

Under pytest the request takes the `PYTEST_CURRENT_TEST` bypass at
`sidecar/main.py:172`, reaches the handler and returns 422. Outside pytest
the same request returns 503. Both satisfy `!= 401`.

So the assertion is true in both worlds, the comment above it describes the
world that only exists inside the test runner, and nothing anywhere asserts
the 503 that every real caller receives. The bypass is deliberate and
documented; what is wrong is that the only test covering the behaviour it
bypasses is written so that it cannot notice either behaviour.

### 5. Two environment variable names for the API base URL

`scripts/agents/config.py:14` reads `JURA_API_URL`.
`scripts/build-sample-pack.sh:33` reads `JURA_API`. Both default to
`http://127.0.0.1:8300` and mean the same thing. Neither name appears in
`docs/API_WRAPPER.md`.

(`docs/design/v1.2.0-headless-api-and-cli.md:1112-1116` records this as
three names by counting `JURA_API_KEY` from the evidence bundle README.
That one is the key, not the URL, and it is used consistently. The defect
is two names for the base URL, not three for one thing. Corrected here
rather than in the design document, per the constraint on the deliverable
that found it.)

## Verified, not inferred

Read on `main` at `d9eb2da1` on 11 September 2026.

- The five Rust methods and their URL literals at
  `src-tauri/src/sidecar.rs:1512`, `:1537`, `:1568`, `:1596`, `:1628`.
- `grep -n "@router.post" sidecar/app/api/forensics.py` lists 24 routes and
  none of the five.
- `src-tauri/src/verify/pipeline.rs:1103`, `:1217`, both `false`.
- `grep -rn "JURA_SIDECAR_PORT\|JURA_SIDECAR_LOG_LEVEL"` over the tree
  matches only the v1.2.0 design document. `grep -rn "sidecar_port"`
  matches only `sidecar/app/config.py:13`.
- `sidecar/main.py:174-178`, the 503, and `:172`, the bypass.
- Finding 4 was proved by running it, not by reading it. The suite passes:

```
$ python3 -m pytest tests/test_api_auth.py -q
5 passed in 2.25s
```

  and the same request with `PYTEST_CURRENT_TEST` removed from the
  environment, against the same ASGI app with the same empty key:

```
status without the bypass: 503 | assertion (!= 401) holds: True
```

- `scripts/agents/config.py:14` and `scripts/build-sample-pack.sh:33`.

## Who it affects, and how badly

No user of v1.1.0 is affected by any of the five. Everything here is
latent.

The people affected are the next two pieces of work. v1.0.1 restores video
and audio and will find finding 1. v1.2.0 builds a CLI over the REST API
and will find findings 2 and 5 while writing the configuration precedence
section that `docs/design/v1.2.0-headless-api-and-cli.md` section 5.3
already sketches. Finding 4 affects whoever next changes the sidecar's
authentication, which is JTV-205, because they will be relying on a test
that agrees with them regardless of what they do.

## What to do, in order

1. **Fix finding 4 first.** It is the only one with a security surface and
   it is the smallest: assert `503` explicitly with the bypass suppressed,
   and assert `422` with it present, so the two paths are distinguished.
   Two tests where there is now one. This is also the change that makes
   JTV-205 safe to attempt.
2. **Correct the two docstrings**, finding 3, in the same commit. They are
   two lines and they currently describe a behaviour that was removed
   on 21 May 2026 because it was a vulnerability.
3. **Delete or gate the five client methods**, finding 1. Deleting is
   cleaner and JTV-139's stated plan is to restore the routes by reverting
   a cleanup commit, which would restore the server side; if the client is
   kept, it needs the same `#[cfg]` or a comment naming JTV-139, and the
   twelve deserialisation tests need to say what they are for.
4. **Make the two settings real or remove them**, finding 2. Reading
   `settings.sidecar_port` as the argparse default is three lines and makes
   the documented name true. Deleting the fields is one line and makes the
   documentation honest. Either is better than the present state.
5. **Pick one environment variable name for the base URL**, finding 5, and
   put it in `docs/API_WRAPPER.md` alongside the exit-code contract, so the
   v1.2.0 CLI inherits one name rather than choosing a third.

## What not to do

- **Do not remove the `PYTEST_CURRENT_TEST` bypass as part of this.** It is
  a deliberate decision with its own ticket. This item is about the test
  that cannot see it, not about the bypass.
- **Do not fold finding 1 into the v1.0.1 video and audio work and forget
  it.** That work will be looking at the sidecar side. The stale client is
  on the Rust side and is what the sidecar work will trip over.
