#!/bin/bash
#
# test-linux-headless.sh — run sidecar pytest inside an OrbStack Ubuntu
# container against the current working tree.
#
# Much faster than spinning up a full UTM VM (~3 s boot vs ~30 s), but
# can't test the AppImage or GUI. Use for sidecar-only iteration.
#
# Prereq: OrbStack installed + 'jura-linux' machine created per README.

set -euo pipefail

MACHINE="jura-linux"
REPO="$HOME/Downloads/ecoadvisor/juralabs"

if ! command -v orb >/dev/null 2>&1; then
  echo "error: orb command not found. Install OrbStack:" >&2
  echo "  brew install --cask orbstack" >&2
  exit 1
fi

if ! orb list 2>/dev/null | grep -q "$MACHINE"; then
  echo "Creating $MACHINE (one-time, ~30 s)…"
  orb create --arch arm64 ubuntu:24.04 "$MACHINE"
  orb -m "$MACHINE" -- sudo apt update -qq
  orb -m "$MACHINE" -- sudo apt install -y -qq python3 python3-pip python3-venv libfuse2
fi

echo "Running sidecar pytest in $MACHINE…"
orb -m "$MACHINE" --cwd "$REPO/sidecar" -- bash -c '
  set -e
  python3 -m venv .venv-orb 2>/dev/null || true
  source .venv-orb/bin/activate
  pip install -q -r requirements-ci.txt
  python -m pytest tests/ -x --timeout=60 -q
'

echo ""
echo "OrbStack Ubuntu pytest complete."
