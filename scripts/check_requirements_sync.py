#!/usr/bin/env python3
"""
check_requirements_sync.py — Recurrence guard for sidecar requirements drift.

Fails with a non-zero exit code if any top-level package present in
requirements.txt is absent from requirements-ci.txt, or if any package is
pinned to different versions across requirements.txt, requirements-ci.txt
and requirements.lock.

Purpose: requirements-ci.txt drifted from requirements.txt when onnxruntime
and regex were added to requirements.txt (JTV-143, 3 May 2026) without
being mirrored into requirements-ci.txt. The resulting CI sidecar binary
shipped without onnxruntime, so clip_detect was silently false on all
Windows and Linux installs. This guard prevents that class of drift
from regressing silently.

Usage (CI — wired into .github/workflows/ci.yml):
    python3 scripts/check_requirements_sync.py

Usage (local — from repo root):
    python3 scripts/check_requirements_sync.py
    python3 scripts/check_requirements_sync.py --requirements sidecar/requirements.txt \
        --ci-requirements sidecar/requirements-ci.txt --lock sidecar/requirements.lock

Comparison rules:
  - Package names are compared case-insensitively (PEP 503).
  - Normalise: underscores and hyphens are equivalent (e.g. "scikit-learn"
    == "scikit_learn").
  - Comments (lines starting with #) and blank lines are ignored.
  - Version pins ARE compared, across all three manifests including
    requirements.lock. Any package pinned with `==` in two or more of the
    files must carry the same version in each. This check was added on
    8 September 2026 (BL-DEPS-004). The previous version of this file said
    pins were not checked "because pip will error on version mismatches at
    install time". That is true within a single resolution and false for
    this layout: ci.yml installs requirements.lock, release.yml installs
    requirements-ci.txt, and Dependabot edits requirements.txt and
    requirements-ci.txt without touching the lock, so no single pip run
    ever sees the conflict. The result was a green Python job that had
    tested the version being replaced. See backlog/BL-DEPS-004.
  - Specifiers other than `==` (for example `>=1.4.0`) in requirements.txt
    or requirements-ci.txt are checked against the version the lock pins,
    and fail when the lock does not satisfy them. Added 15 October 2026
    (v1.2.0 C2). Before that they were name-checked only, so a floor above
    the locked version passed silently: Dependabot's #12 proposed
    `pytest-asyncio>=1.4.0` while the lock pinned 1.3.0, and ci.yml
    installed 1.3.0. Supported operators: ==, !=, >=, >, <=, <, ~=.
  - Every package in requirements-ci.txt must also be pinned in the lock,
    added 15 October 2026 (v1.2.0 C2). release.yml freezes
    requirements-ci.txt and ci.yml installs the lock, so a package in the
    first and not the second ships without CI ever installing it. The
    version check above cannot see this, because it needs a package in two
    files. Known exceptions are listed in CI_ONLY_ALLOWED with a reason.
  - Testing-only markers such as `pytest` that appear in requirements.txt
    under a comment block "# Testing" are excluded from the check via
    an explicit allowlist — they are not needed in the frozen binary.
  - The `httpx` bare pin in requirements.txt (used for async test client)
    is treated as testing-only and excluded.
"""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Packages present in requirements.txt that are intentionally absent from
# requirements-ci.txt because they are test-only, not runtime deps.
# Keep this list minimal and well-commented.
TESTING_ONLY = {
    "pytest",          # testing framework
    "pytest-asyncio",  # async test support
}

# httpx appears twice in requirements.txt: once as a runtime dep (pinned
# version, already in requirements-ci.txt) and once bare under "# Testing".
# The bare entry is also testing-only; but since httpx is already in CI
# requirements, this set is purely documentary — the presence check passes
# regardless.
TESTING_ONLY_BARE = {
    "httpx",  # bare re-pin under # Testing block; runtime version already pinned
}

# Packages in requirements-ci.txt that the lock does not pin, and why that
# is accepted. Each entry ships in the Windows and Linux sidecar without CI
# installing it. Keep this list empty if possible.
CI_ONLY_ALLOWED = {
    # Pinned in requirements-ci.txt as a transitive of anyio, but the lock is
    # a full pip-compile resolution of requirements.txt and does not contain
    # it, so nothing in the resolved graph needs it any more (anyio 4.14.2
    # does not depend on sniffio). An orphan pin, not a missing test.
    # Removing it changes what release.yml freezes, so it goes with the
    # C1 step 4 sidecar bump rather than with this guard (15 October 2026).
    "sniffio",
}

SPEC_OPS = ("===", "~=", "==", "!=", ">=", "<=", ">", "<")


def normalise(name: str) -> str:
    """Normalise a package name per PEP 503 (lowercase, hyphens = underscores)."""
    return re.sub(r"[-_]+", "-", name.strip().lower())


def extract_package_name(line: str) -> str | None:
    """Extract the package name from a requirements.txt line.

    Handles:
      package==1.0.0
      package>=1.0
      package[extra]==1.0.0
      package                  (bare, no version spec)
    Returns None for comments, blank lines, and options flags (-r, -c, etc.)
    """
    line = line.strip()
    if not line or line.startswith("#") or line.startswith("-"):
        return None
    # Strip inline comments
    line = line.split("#")[0].strip()
    if not line:
        return None
    # Extract name before any version specifier or extras
    match = re.match(r"^([A-Za-z0-9._-]+)", line)
    return match.group(1) if match else None


def extract_pin(line: str) -> tuple[str, str] | None:
    """Return (normalised name, version) for a `name==version` line, else None.

    Handles extras (`name[extra]==1.0`), environment markers
    (`name==1.0; python_version >= "3.9"`), inline comments, and the
    pip-compile lock format, where the pin line ends in a backslash and is
    followed by `--hash=...` continuation lines (those start with `-` and are
    skipped by the caller). Lines with any other specifier return None: there
    is no single version to compare.
    """
    line = line.strip()
    if not line or line.startswith("#") or line.startswith("-"):
        return None
    line = line.split("#")[0].split(";")[0].strip().rstrip("\\").strip()
    match = re.match(r"^([A-Za-z0-9._-]+)(?:\[[^\]]*\])?==([^\s]+)$", line)
    if not match:
        return None
    return normalise(match.group(1)), match.group(2)


def extract_specifiers(line: str) -> tuple[str, list[tuple[str, str]]] | None:
    """Return (normalised name, [(operator, version), ...]) for a requirement line.

    `pytest>=9.1.1` gives ("pytest", [(">=", "9.1.1")]); a bare name gives an
    empty list. Returns None for comments, blank lines and option lines.
    """
    line = line.strip()
    if not line or line.startswith("#") or line.startswith("-"):
        return None
    line = line.split("#")[0].split(";")[0].strip().rstrip("\\").strip()
    match = re.match(r"^([A-Za-z0-9._-]+)(?:\[[^\]]*\])?\s*(.*)$", line)
    if not match:
        return None
    specs: list[tuple[str, str]] = []
    for part in filter(None, (p.strip() for p in match.group(2).split(","))):
        op = next((o for o in SPEC_OPS if part.startswith(o)), None)
        if op is None:
            raise ValueError(f"unparseable specifier {part!r} in line {line!r}")
        specs.append((op, part[len(op):].strip()))
    return normalise(match.group(1)), specs


def version_key(version: str) -> tuple:
    """A sortable key for a release version such as 2.5.2 or 2026.10.23.

    Stdlib only, so this covers release segments and treats any pre-release,
    post or local suffix on a segment as sorting before the bare number. That
    is enough for the plain release pins these manifests use; the guard's job
    is to notice a floor above the lock, not to reimplement PEP 440.
    """
    key: list[tuple[int, str]] = []
    for segment in version.split("+")[0].split("."):
        match = re.match(r"^(\d+)(.*)$", segment)
        if match:
            # A suffix ("0rc1") sorts before the bare number ("0").
            key.append((int(match.group(1)), match.group(2) or "~"))
        else:
            key.append((-1, segment))
    # Pad so 1.4 and 1.4.0 compare equal and 1.0rc1 sorts before 1.0.
    key.extend([(0, "~")] * (8 - len(key)))
    return tuple(key)


def satisfies(version: str, op: str, wanted: str) -> bool:
    """Whether `version` meets the single specifier `op wanted`."""
    if op == "===":
        return version == wanted
    if op in ("==", "!=") and wanted.endswith(".*"):
        prefix_len = len(wanted[:-2].split("."))
        matched = version_key(version)[:prefix_len] == version_key(wanted[:-2])[:prefix_len]
        return matched if op == "==" else not matched
    have, want = version_key(version), version_key(wanted)
    if op == "==":
        return have == want
    if op == "!=":
        return have != want
    if op == ">=":
        return have >= want
    if op == ">":
        return have > want
    if op == "<=":
        return have <= want
    if op == "<":
        return have < want
    if op == "~=":
        # ~=1.4.2 means >=1.4.2 and ==1.4.*
        prefix_len = len(wanted.split(".")) - 1
        return have >= want and have[:prefix_len] == want[:prefix_len]
    raise ValueError(f"unsupported operator {op!r}")


def load_specifiers(path: Path) -> dict[str, list[tuple[str, str]]]:
    """Return {normalised name: specifiers} for every line in a requirements file.

    A package listed twice (httpx, pinned and then bare under # Testing) keeps
    the union of its specifiers.
    """
    specs: dict[str, list[tuple[str, str]]] = {}
    with open(path) as f:
        for line in f:
            parsed = extract_specifiers(line)
            if parsed:
                specs.setdefault(parsed[0], []).extend(parsed[1])
    return specs


def load_pins(path: Path) -> dict[str, str]:
    """Return {normalised name: version} for every `==` pin in a requirements file."""
    pins: dict[str, str] = {}
    with open(path) as f:
        for line in f:
            pin = extract_pin(line)
            if pin:
                pins[pin[0]] = pin[1]
    return pins


def load_packages(path: Path) -> set[str]:
    """Return the set of normalised package names from a requirements file."""
    packages: set[str] = set()
    with open(path) as f:
        for line in f:
            name = extract_package_name(line)
            if name:
                packages.add(normalise(name))
    return packages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--requirements",
        default=str(REPO_ROOT / "sidecar" / "requirements.txt"),
        help="Path to requirements.txt (source of truth)",
    )
    parser.add_argument(
        "--ci-requirements",
        default=str(REPO_ROOT / "sidecar" / "requirements-ci.txt"),
        help="Path to requirements-ci.txt (must mirror requirements.txt)",
    )
    parser.add_argument(
        "--lock",
        default=str(REPO_ROOT / "sidecar" / "requirements.lock"),
        help="Path to requirements.lock (what ci.yml installs); version pins "
        "must agree with the other two files",
    )
    args = parser.parse_args()

    req_path = Path(args.requirements)
    ci_path = Path(args.ci_requirements)
    lock_path = Path(args.lock)

    if not req_path.exists():
        print(f"ERROR: {req_path} not found", file=sys.stderr)
        return 1
    if not ci_path.exists():
        print(f"ERROR: {ci_path} not found", file=sys.stderr)
        return 1

    req_pkgs = load_packages(req_path)
    ci_pkgs = load_packages(ci_path)

    # Packages that are allowed to be absent from requirements-ci.txt
    excluded = {normalise(p) for p in (TESTING_ONLY | TESTING_ONLY_BARE)}

    # Check: every package in requirements.txt must appear in requirements-ci.txt
    # unless it is in the excluded set.
    missing: list[str] = sorted(
        pkg for pkg in req_pkgs
        if pkg not in ci_pkgs and pkg not in excluded
    )

    if missing:
        print(
            "FAIL: The following package(s) are in requirements.txt but ABSENT "
            "from requirements-ci.txt:",
            file=sys.stderr,
        )
        for pkg in missing:
            print(f"  {pkg}", file=sys.stderr)
        print(
            "\nAdd them to requirements-ci.txt with an explicit version pin "
            "matching requirements.txt.",
            file=sys.stderr,
        )
        print(
            "This guard exists because requirements-ci.txt drift caused "
            "onnxruntime to be omitted from the CI sidecar binary (JTV-143), "
            "resulting in clip_detect=false on all Windows and Linux installers.",
            file=sys.stderr,
        )
        return 1

    # Check: any package pinned with `==` in two or more manifests must carry
    # the same version in each. This is the BL-DEPS-004 guard. ci.yml
    # installs the lock and release.yml installs requirements-ci.txt, so a
    # version that differs between them is a version CI never tested but the
    # release would ship. No single pip run can see that, so this has to.
    manifests: list[tuple[str, dict[str, str]]] = [
        (req_path.name, load_pins(req_path)),
        (ci_path.name, load_pins(ci_path)),
    ]
    if lock_path.exists():
        manifests.append((lock_path.name, load_pins(lock_path)))
    else:
        print(
            f"WARN: {lock_path} not found; version agreement checked across "
            "two files only",
            file=sys.stderr,
        )

    all_pinned = set().union(*(pins.keys() for _, pins in manifests))
    divergent: list[tuple[str, list[tuple[str, str]]]] = []
    for pkg in sorted(all_pinned):
        found = [(name, pins[pkg]) for name, pins in manifests if pkg in pins]
        if len(found) >= 2 and len({v for _, v in found}) > 1:
            divergent.append((pkg, found))

    if divergent:
        print(
            "FAIL: The following package(s) are pinned to DIFFERENT versions "
            "across the sidecar manifests:",
            file=sys.stderr,
        )
        for pkg, found in divergent:
            where = "  ".join(f"{name}={ver}" for name, ver in found)
            print(f"  {pkg}: {where}", file=sys.stderr)
        print(
            "\nEvery manifest must agree. If this is a dependency bump, update "
            "all three: requirements.txt, requirements-ci.txt, and regenerate "
            "requirements.lock with pip-compile (see the header of that file).",
            file=sys.stderr,
        )
        print(
            "This guard exists because ci.yml installs requirements.lock while "
            "release.yml installs requirements-ci.txt. A version that differs "
            "between them is one CI never tested and the release would ship "
            "(BL-DEPS-004).",
            file=sys.stderr,
        )
        return 1

    if lock_path.exists():
        lock_pins = load_pins(lock_path)

        # Check: every specifier in requirements.txt and requirements-ci.txt
        # must be met by the version the lock pins. The `==` check above
        # covers equal pins; this covers floors and ranges, where a `>=`
        # above the locked version is a version CI never installs.
        unmet: list[str] = []
        for path in (req_path, ci_path):
            for pkg, specs in sorted(load_specifiers(path).items()):
                locked = lock_pins.get(pkg)
                if locked is None:
                    continue
                for op, wanted in specs:
                    if op != "==" and not satisfies(locked, op, wanted):
                        unmet.append(
                            f"  {pkg}: {path.name} wants {op}{wanted}, "
                            f"{lock_path.name} pins {locked}"
                        )
        if unmet:
            print(
                "FAIL: The following specifier(s) are not met by the version "
                f"{lock_path.name} pins:",
                file=sys.stderr,
            )
            print("\n".join(unmet), file=sys.stderr)
            print(
                "\nci.yml installs the lock, so CI is testing a version the "
                "manifest has ruled out. Regenerate requirements.lock with "
                "pip-compile (see the header of that file).",
                file=sys.stderr,
            )
            return 1

        # Check: every package release.yml freezes must be one ci.yml
        # installs. A package in requirements-ci.txt and not in the lock ships
        # in the Windows and Linux sidecar without CI ever installing it.
        allowed = {normalise(p) for p in CI_ONLY_ALLOWED}
        untested = sorted(
            pkg for pkg in ci_pkgs if pkg not in lock_pins and pkg not in allowed
        )
        if untested:
            print(
                f"FAIL: The following package(s) are in {ci_path.name}, which "
                f"release.yml freezes, but not in {lock_path.name}, which "
                "ci.yml installs:",
                file=sys.stderr,
            )
            for pkg in untested:
                print(f"  {pkg}", file=sys.stderr)
            print(
                "\nAdd them to requirements.txt and regenerate the lock, or "
                "remove them from requirements-ci.txt. If neither is right, "
                "add them to CI_ONLY_ALLOWED in this script with the reason.",
                file=sys.stderr,
            )
            return 1

    # Informational: packages in requirements-ci.txt not in requirements.txt.
    # These are transitive pins — allowed, not an error.
    extras = ci_pkgs - req_pkgs - excluded
    print(
        f"OK: all {len(req_pkgs) - len(req_pkgs & excluded)} non-test packages "
        f"from requirements.txt are present in requirements-ci.txt."
    )
    print(
        f"OK: {len(all_pinned)} pinned package(s) agree on version across "
        f"{len(manifests)} manifest(s)."
    )
    if lock_path.exists():
        print(
            f"OK: every specifier is met by {lock_path.name}, and every package "
            f"in {ci_path.name} is in it"
            + (f" except {', '.join(sorted(CI_ONLY_ALLOWED))} (CI_ONLY_ALLOWED)."
               if CI_ONLY_ALLOWED else ".")
        )
    if extras:
        print(
            f"INFO: {len(extras)} package(s) in requirements-ci.txt are "
            "transitive pins not in requirements.txt (this is expected):"
        )
        for pkg in sorted(extras):
            print(f"  {pkg}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
