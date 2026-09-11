# BL-REL-005: the macOS build ships whatever is installed on one laptop, and 258 MiB of it reached users

**Status**: Open. Found 11 September 2026, measuring the installed v1.1.0
bundle while planning the v1.2.0 size work.
**Raised**: 11 September 2026
**Severity**: High. Not for the megabytes, which are merely embarrassing,
but because the macOS payload is not a function of anything in this
repository. It is a function of what is installed on one developer's
machine on the day of the build, and no gate anywhere can see the
difference.

## What is wrong

The macOS release is built locally, by `scripts/build-local-mac.sh`, and it
freezes the Python sidecar like this, at `scripts/build-local-mac.sh:190`:

```sh
(cd sidecar && JURA_SIDECAR_ONEDIR=1 python3 -m PyInstaller jura-sidecar.spec --noconfirm \
```

The script contains no `pip install`, no virtual environment and no
reference to any requirements file. `python3` is whatever is first on the
path. On the machine that builds macOS releases that is
`/opt/homebrew/Caskroom/miniconda/base/bin/python3`.

`sidecar/jura-sidecar.spec:112-119` then asks that interpreter what it
happens to have:

```python
for _optional_pkg in ["chromadb", "sentence_transformers"]:
    try:
        d, b, h = collect_all(_optional_pkg)
        datas += d; binaries += b; hiddenimports += h
    except Exception:
        pass
```

On a clean runner the import fails and the `except` swallows it, which is
what the comment above it intends. On the build machine the import
succeeds, `collect_all` walks the dependency graph, and `datasets` pulls
`pyarrow`, which pulls the Arrow shared libraries, `pandas`, `grpc`,
`tokenizers` and `hf_xet`.

**258.4 MiB of that shipped to macOS users in v1.1.0.** The measurements
are in `docs/design/v1.2.0-windows-onedir-and-size.md:663-693`: the Arrow
family alone is 204.7 MiB once PyInstaller's hoisted dylibs are counted,
and the four passengers add the rest.

Windows and Linux do not carry any of it. `release.yml:541-544` installs
`sidecar/requirements-ci.txt` on a clean GitHub runner, and none of
`pyarrow`, `pandas`, `chromadb`, `sentence_transformers`, `datasets`,
`grpcio` or `tokenizers` appears in that file, in `sidecar/requirements.txt`
or in `sidecar/requirements.lock`.

So the three platforms ship different Python environments, and the one that
differs is the one no continuous integration job has ever built.

## Verified, not inferred

Checked by hand on 11 September 2026, on `main` at `d9eb2da1`, against the
installed application rather than only against the design document.

The shipped v1.1.0 macOS bundle really does contain it:

```
$ /usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" \
    "/Applications/Jura Trace.app/Contents/Info.plist"
1.1.0
$ du -sm "/Applications/Jura Trace.app/Contents/Resources/sidecar-bundle/_internal/pyarrow"
114     /Applications/Jura Trace.app/.../sidecar-bundle/_internal/pyarrow
$ ls ".../sidecar-bundle/_internal/" | grep -i arrow
libarrow_acero.2300.dylib
libarrow_compute.2300.dylib
libarrow_dataset.2300.dylib
libarrow_flight.2300.dylib
libarrow_python_flight.2300.dylib
libarrow_python_parquet_encryption.2300.dylib
libarrow_python.2300.dylib
libarrow_substrait.2300.dylib
libarrow.2300.dylib
pyarrow
```

The build machine really is the source:

```
$ which -a python3
/opt/homebrew/Caskroom/miniconda/base/bin/python3
/opt/homebrew/bin/python3
/usr/bin/python3
$ python3 -c "import pyarrow, chromadb, sentence_transformers; print(...)"
23.0.1 0.5.23 3.4.1
```

The dylib version, 2300, matches the installed `pyarrow 23.0.1`.

And nothing in the repository asked for it:

- `grep -in "pyarrow\|chromadb\|sentence" sidecar/requirements-ci.txt
  sidecar/requirements.lock sidecar/requirements.txt` returns nothing.
- `grep -rn "import pyarrow\|import chromadb\|import sentence_transformers\|import pandas" sidecar/`
  returns nothing. No code in the product imports any of them.
- `grep -n "pip install\|venv\|virtualenv" scripts/build-local-mac.sh`
  returns nothing.

## Who it affects, and how badly

Every macOS user carries about 258 MiB they cannot use, inside a DMG that
is already 848 MB. `BL-SIZE-001` records download size as the barrier
persona review named as a hard gate for field users on metered
connections, and this is that barrier made worse by accident on one
platform only.

That is the visible harm and it is the smaller one.

The real harm is that the macOS artefact is unreproducible. Two builds of
the same commit on two machines produce different bundles, and no assertion
in the release path compares them. The consequence follows directly:
**macOS can ship code that CI never tested**, because CI cannot install
what it does not know about. The four passengers include `grpc` and
`tokenizers`, which carry their own native code and their own advisories,
and `cargo audit`, `pip-audit` and OSV all scan the manifests rather than
the bundle, so none of them would ever see a vulnerability in any of it.
BL-DEPS-003 and BL-CI-002 are both about closing the channel through which
a disclosed vulnerability reaches somebody. That channel does not reach
these packages at all.

## Why this is its own item and not an update to BL-DEPS-004

BL-DEPS-004 already names this channel. Its section "A third channel,
pinned by nothing" says that `scripts/build-local-mac.sh` freezes the
sidecar against the developer's ambient Python and that "on macOS there is
no pin in the loop at all", and its update of 8 September lists the macOS
channel as remaining work.

It is nonetheless a separate item, for three reasons.

1. **They are different failures with different fixes.** BL-DEPS-004 is
   about three manifests disagreeing with each other, and its fix, option
   (b), is a guard that compares them. That guard has landed and is green,
   and it would stay green on every build described here, because the
   packages involved are in none of the three manifests. Nothing in
   BL-DEPS-004's plan touches the spec file or the build script.
2. **The evidence is different in kind.** BL-DEPS-004 is a hazard argued
   from the wiring. This is a measured quantity in a shipped artefact, with
   a version number, on a release users have installed. It closes rather
   than predicts.
3. **They will be scheduled differently.** BL-DEPS-004's remainder is
   Dependabot behaviour and a standing rule for sidecar bumps. This is half
   a day of spec-file work sitting inside the v1.2.0 size effort, alongside
   the CLIP text precompute, and it wants to be visible in that plan
   rather than nested inside a dependency item.

A cross-reference is added to BL-DEPS-004 under "Update, 11 September
2026" rather than the whole write-up, so the two do not drift apart.

## What to do, in order

1. **Delete the opportunistic block** at
   `sidecar/jura-sidecar.spec:112-119`. Nothing in `sidecar/app/` imports
   `chromadb` or `sentence_transformers`, so the collection is buying
   nothing on any platform. Deleting it makes the macOS bundle a function
   of the spec file rather than of the machine.
2. **Add the five names to `excludes`** in the same spec file, so that a
   future opportunistic import cannot reintroduce them:
   `pyarrow`, `pandas`, `chromadb`, `sentence_transformers`, `datasets`.
   This is the fail-closed half, and it is what makes step 1 durable.
3. **Verify by building, not by reading.** Build the sidecar on the build
   machine after the change and assert `_internal` contains no `arrow`,
   `pandas`, `grpc`, `tokenizers` or `hf_xet` entry, and that the sidecar
   still starts and answers `/health`. The risk named in
   `docs/design/v1.2.0-windows-onedir-and-size.md:790-795` is that the
   `chromadb` collection was supplying a hidden import something else
   needs on macOS only, and only a real run can rule that out.
4. **Give the macOS script a pinned environment.** A virtual environment
   built from `sidecar/requirements-ci.txt`, which is the file the release
   workflow uses, and a refusal to build if `python3` is not that
   environment. This is the step that stops the class rather than this
   instance, and it is the step BL-DEPS-004 has been waiting for.
5. **Assert it in the build script.** After the freeze, compare the set of
   top-level `_internal` package directories against the previous release's
   set and fail on an unexpected addition. A diff of a sorted list is
   cheap, and it is the only thing that would have caught this on the day
   rather than four months later.

## What not to do

- **Do not treat this as a size item and stop at step 1.** Removing 258 MiB
  without step 4 leaves the next unpinned package free to arrive the same
  way. The size is the symptom.
- **Do not move the macOS build to CI as the fix.** That is a much larger
  change, it is blocked by the known Phase 5 signing failure recorded in
  `project_mac_local_release_build.md`, and it has its own risks, which
  BL-REL-006 sets out. Pinning the local environment gets the property that
  matters here at a fraction of the cost.
- **Do not add the packages to the requirements files to make the bundle
  legitimate.** They are unused. The manifests are right and the bundle is
  wrong.
