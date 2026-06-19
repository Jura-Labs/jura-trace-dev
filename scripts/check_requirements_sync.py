#!/usr/bin/env python3
"""
check_requirements_sync.py — Recurrence guard for requirements-ci.txt drift.

Fails with a non-zero exit code if any top-level package present in
requirements.txt is absent from requirements-ci.txt.

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
        --ci-requirements sidecar/requirements-ci.txt

Comparison rules:
  - Package names are compared case-insensitively (PEP 503).
  - Normalise: underscores and hyphens are equivalent (e.g. "scikit-learn"
    == "scikit_learn").
  - Comments (lines starting with #) and blank lines are ignored.
  - Only package NAMES are compared; version pins are not checked here
    (pip will error on version mismatches at install time).
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
    args = parser.parse_args()

    req_path = Path(args.requirements)
    ci_path = Path(args.ci_requirements)

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

    # Informational: packages in requirements-ci.txt not in requirements.txt.
    # These are transitive pins — allowed, not an error.
    extras = ci_pkgs - req_pkgs - excluded
    print(
        f"OK: all {len(req_pkgs) - len(req_pkgs & excluded)} non-test packages "
        f"from requirements.txt are present in requirements-ci.txt."
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
