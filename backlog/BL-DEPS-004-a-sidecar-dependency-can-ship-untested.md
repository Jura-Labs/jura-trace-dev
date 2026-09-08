# BL-DEPS-004: a sidecar dependency can ship without CI having tested it

**Status**: Open. Found 8 September 2026, reading the pip half of the
Dependabot queue against `ci.yml`.
**Severity**: High. Not because any bump is known to have moved a detector
score, but because the check that would tell us is wired so that it cannot.
A green Python job on a sidecar dependency pull request is a statement
about the old version of the package.
**Raised**: 8 September 2026

## What happened

Three files pin the sidecar's Python dependencies, and three consumers read
three different ones.

| File | Size | Read by |
|---|---|---|
| `sidecar/requirements.txt` | 3,903 bytes, 17 pins | the source of truth, and the input `requirements.lock` is compiled from |
| `sidecar/requirements-ci.txt` | 2,622 bytes, 22 pins | `release.yml:522-533`, the PyInstaller build that becomes the sidecar users download |
| `sidecar/requirements.lock` | 98,724 bytes, 51 hash-pinned entries | `ci.yml:250-256`, the job that runs the sidecar pytest suite |

`ci.yml:253` installs `sidecar/requirements.lock` and falls back to
`requirements.txt` only if the lock is absent. The lock is present, so the
fallback never fires. `release.yml:527` installs
`sidecar/requirements-ci.txt`, with the lock only as its second choice, so
that fallback never fires either.

Dependabot's pip pull requests edit `requirements.txt` and
`requirements-ci.txt`. Not one of them touches `requirements.lock`. The
whole open pip queue on 8 September, by files changed:

| PR | Package | Files changed |
|---|---|---|
| #1 | opencv-python-headless 4.11.0.86 to 5.0.0.93 | `requirements-ci.txt`, `requirements.txt` |
| #4 | scipy 1.17.1 to 1.18.1 | `requirements-ci.txt`, `requirements.txt` |
| #5 | scikit-learn 1.8.0 to 1.9.0 | `requirements-ci.txt`, `requirements.txt` |
| #8 | uvicorn 0.34.0 to 0.52.4 | `requirements-ci.txt`, `requirements.txt` |
| #9 | numpy 2.2.4 to 2.5.2 | `requirements-ci.txt`, `requirements.txt` |
| #10 | pillow-avif-plugin 1.5.5 to 1.6.0 | `requirements-ci.txt`, `requirements.txt` |
| #11 | certifi 2026.2.25 to 2026.7.22 | `requirements-ci.txt` only |
| #12 | pytest-asyncio >=0.24 to >=1.4.0 | `requirements.txt` only |

`requirements.lock` appears in none of them.

## What that produces

PR #9 proposes numpy 2.5.2. `sidecar/requirements.lock:104` still pins
`numpy==2.2.4`. The Python job on that pull request installed the lock and
its log says exactly what it installed:

```
Collecting numpy==2.2.4 (from -r sidecar/requirements.lock (line 104))
...
Successfully installed ... numpy-2.2.4 ... scipy-1.17.1 ...
354 passed, 71 skipped, 37 warnings in 25.60s
```

The Python check is green. It is green about numpy 2.2.4. Merging the pull
request puts numpy 2.5.2 into `requirements-ci.txt`, which is the file
`release.yml` installs before freezing the sidecar binary. On the one pull
request where the version changes, the version CI tests and the version
that ships are guaranteed to be different versions.

## The guard that should catch this compares names only

`scripts/check_requirements_sync.py` runs in the Repo hygiene job at
`ci.yml:366-367`. It is the recurrence guard written after onnxruntime and
regex went missing from CI sidecar builds (JTV-143, 3 May 2026). It passed
on PR #9, and it was always going to. Its own docstring says why, at
`scripts/check_requirements_sync.py:28`:

```
  - Only package NAMES are compared; version pins are not checked here
    (pip will error on version mismatches at install time).
```

The parenthetical is the load-bearing claim and it is false for this
layout. pip errors on a version conflict within a single resolution. It
never sees these three files in one resolution, because no consumer
installs more than one of them. Nothing anywhere compares `numpy==2.2.4` in
the lock against `numpy==2.5.2` in `requirements-ci.txt`.

The guard is also one-directional. `main()` at
`check_requirements_sync.py:124-135` checks only that every name in
`requirements.txt` appears in `requirements-ci.txt`, and the file never
opens `requirements.lock` in its 176 lines. Small evidence that the reverse
direction drifts too: `requirements-ci.txt:31` pins `sniffio==1.3.1`, a
package that appears in neither `requirements.txt` nor the lock, and that
PR #9's install did not pull in.

BL-SILENT-001 lists "the requirements sync guard is sound" under **Checked
and clean**. That is true of the drift it was written for, a missing
package name, and false of version drift. Worth amending there.

## Why this matters more here than in most repositories

Jura Trace's output is a calibrated number. The sidecar is where the ML
forensic detectors compute it, and they are numerical all the way down.

- `sidecar/app/services/deepfake.py:27-28` imports `fft2`, `fftshift` and
  `dctn` from `scipy.fft`, and `laplace` from `scipy.ndimage`.
- `deepfake.py:1281`, `:1715` and `:1857` fit slopes with `np.polyfit`.
- `sidecar/app/services/gan_fingerprint.py:67` solves a least-squares fit
  with `np.linalg.lstsq`.
- `sidecar/app/services/npr.py:40-41` takes `kurtosis` and `skew` from
  `scipy.stats`.

Those feed one vector. `deepfake.py:1043-1052` builds an 84-value feature
vector and trims it to `clf.n_features_in_`, which is 80 for the shipped
GBM v4 model, before calling `predict_proba`. A change in how numpy
accumulates a sum, which BLAS backend a wheel is built against, or how
`scipy.fft` normalises, moves feature values without a line of our code
changing. The thresholds those values are compared against were calibrated
against the current libraries.

This is not hypothetical in this repository. The Pillow bump was held for
exactly this reason and taken only after a decode-drift test on 84 golden
images across five codecs, written up in
`docs/calibration/pillow-12-decode-drift-sep2026.md` and summarised in
commit `7700c054`. numpy and scipy sit closer to the arithmetic than Pillow
does.

## The tests would not catch the drift either

Even if CI installed the proposed version, the sidecar suite asserts bounds
rather than values, and in places asserts nothing at all.

`sidecar/tests/test_deepfake.py:243-247`:

```python
    def test_verdict_level_authentic_for_low_score(self):
        """An image scoring < 0.30 should get verdict_level 'authentic'."""
        result = perform_deepfake_detection(_make_noisy_photo())
        if result.score < 0.30:
            assert result.verdict_level == "authentic"
```

If a library change pushes that score from 0.29 to 0.31, the condition is
false, the body never runs, and the test passes having asserted nothing.
`:249-254` is the same shape at the synthetic end, and `:266-272` repeats
it across three fixtures. A test whose assertion is gated on the value
under test cannot detect a change in that value.

What is asserted unconditionally is bounds.
`sidecar/tests/test_clip_detector.py:147` is
`assert 0.0 <= result.score <= 1.0`, which holds for every score the
detector can emit, and the same line recurs at `:156`, `:164`, `:172`,
`:286` and `:294`. The nearest thing to a golden-value test is
`test_deepfake.py:587`, `assert result.score < 0.5` over real camera
photos, and its class autouse-skips whenever the photo paths are missing
(`:559-565`), which is always true on a CI runner. The CLIP model class
skips as well, on `pytest.importorskip("open_clip")` at
`test_clip_detector.py:134-135`, and open_clip is deliberately absent from
both the lock and `requirements-ci.txt`. That is a large part of why 71 of
425 tests skipped in PR #9's run.

So the failure is silent in both directions. CI reports green on a version
it did not install, and had it installed it, nothing in the suite pins a
number tightly enough to notice.

## The repository already knows the correct shape

`git show --stat 7700c054`, the Pillow 11.2.1 to 12.3.0 bump of
4 September:

```
 docs/calibration/pillow-12-decode-drift-sep2026.md | 117 ++++++++++++++
 sidecar/requirements-ci.txt                        |   2 +-
 sidecar/requirements.lock                          | 170 +++++++++++----------
 sidecar/requirements.txt                           |   2 +-
 4 files changed, 207 insertions(+), 84 deletions(-)
```

All three manifests plus the evidence, in one commit. That is what a
sidecar bump has to look like. Dependabot produces two thirds of it and the
gate does not notice the missing third. `AGENTS.md:199` names all three
files together, and `AGENTS.md:304` flags `requirements-ci.txt` as "the
file CI actually uses for PyInstaller builds", which is right about the
release workflow and wrong about `ci.yml`.

## A third channel, pinned by nothing

`scripts/build-local-mac.sh` is the real release channel for macOS, as
BL-SILENT-001 item 7 records. It freezes the sidecar at line 169 with
`JURA_SIDECAR_ONEDIR=1 python3 -m PyInstaller jura-sidecar.spec`. The
script contains no `pip install`, no virtualenv and no reference to any
requirements file. Every macOS sidecar is frozen against whatever happens
to be installed in the developer's ambient Python 3.13 at build time. The
divergence described above is between two pinned files. On macOS there is
no pin in the loop at all.

## The fix

Four options, and they are less interchangeable than they look.

**(a) Point the CI Python job at `requirements-ci.txt`.** One line at
`ci.yml:253`. CI would then test the package set the release actually
freezes. The cost is the property the lock was introduced for: hash-pinned,
fully resolved transitive dependencies. CI would stop proving the locked
set installs at all, and `pip-audit` at `ci.yml:280-281` would audit a
looser environment than the lock describes. It also does nothing for
`requirements.txt`-only pull requests such as #12.

**(b) Extend `check_requirements_sync.py` to compare versions across all
three files.** Roughly thirty lines on top of the parsing it already does:
read `requirements.lock` as well, and fail when a package named in two
files carries two versions. This does not make CI test the shipping
version. It does convert today's silent divergence into a red build, on the
pull request that creates it, in about five seconds of stdlib-only work
with no extra CI cost. Every Dependabot sidecar pull request goes red until
its lock is regenerated, which is the correct signal and not an
inconvenience.

**(c) Make Dependabot regenerate the lock.** The clean fix and the least
certain to be available. `requirements.lock:1-5` records that it is
generated by
`pip-compile --generate-hashes --no-annotate --output-file=requirements.lock requirements.txt`.
Dependabot's pip ecosystem does handle pip-compile output, but it
identifies those files by extension, and this one is `.lock`. Renaming it
to `requirements-lock.txt` may be the whole fix. That has to be confirmed
against current Dependabot behaviour before anything is built on it, and
the rename touches the `hashFiles` cache keys at `ci.yml:227` and
`release.yml:515` as well as both install steps.

**(d) Accept that a sidecar bump is a manual four-file operation**, done
the way `7700c054` did it: regenerate the lock, update both text manifests,
record the drift evidence in `docs/calibration/`. This is correct
regardless, and it is what will actually happen for numpy and scipy whether
or not (c) works.

**Do (b) first, then investigate (c), and keep (d) as the standing rule.**
(b) is the only one of the four that is cheap, certain and fails closed,
and it is worth doing even if (c) lands, because it is then the thing that
proves (c) is still working. If Dependabot's behaviour changes later, (b)
says so on the next pull request instead of at the next release.

Do not take (a) on its own. It makes the Dependabot pull requests honest
and makes the lock file untested, which swaps one blind spot for another.

## What not to do

- **Do not merge #4, #5 or #9 on the version number.** scipy,
  scikit-learn and numpy are the three packages in the queue sitting
  directly under the detector arithmetic. Each needs what Pillow got: a
  drift run against a fixed corpus, before and after, recorded in
  `docs/calibration/`. #5 carries a second risk on top of that, because it
  is the library that unpickles `deepfake_classifier.joblib`, and a
  scikit-learn minor can change what a v4 estimator loads as.
- **Do not fix this by deleting `requirements.lock`.** The fallback at
  `ci.yml:255` would silently take over and install `requirements.txt`.
  That is option (a) with no record that a decision was made, and with hash
  pinning lost as well.
- **Do not fold golden-value detector tests into this item.** They are the
  right answer to the second half of the problem, they depend on a corpus,
  and they are separate work. What belongs here is the version guard. The
  conditional assertions at `test_deepfake.py:243-254` and `:266-272`
  belong on BL-TEST-002's surface list instead, as tests that cannot fail.

## Relationship to the other items

This is BL-SILENT-001's Cause B applied to a dependency: success defined as
"the step exited 0" rather than "the step exercised what ships". The green
Python check is the mechanism reporting success while testing something
other than the change under review.

It is also where BL-DEPS-001 stopped one step short. That item triaged the
queue by risk and said Pillow needs a calibration re-run before acceptance,
which is right, but it assumed the gate would at least build and test the
proposed version. For the sidecar, it does not.
