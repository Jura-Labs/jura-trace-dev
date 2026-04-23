#!/bin/bash
#
# pre-tag-check.sh — stage a Jura Trace installer into the SSD artefact
# tree and print manual-test instructions for the appropriate VM.
#
# Usage:
#   scripts/test-installer/pre-tag-check.sh <path-to-installer>
#
# Detects platform from file extension and hands off to the right VM
# workflow. Does NOT automatically run the VM — AppleScript UTM control
# is brittle and UTM's CLI is minimal. This script just prepares the
# artefact and reminds you what to do next.

set -euo pipefail

if [[ $# -ne 1 ]]; then
  cat <<EOF
Usage: $0 <installer-path>

Supported installers:
  *.msi    Windows installer → Windows 11 ARM64 VM
  *.dmg    macOS installer   → local macOS (no VM needed)
  *.AppImage / *.deb Linux installer → Ubuntu 24.04 ARM64 VM
EOF
  exit 64
fi

INSTALLER="$1"
if [[ ! -f "$INSTALLER" ]]; then
  echo "error: not a file: $INSTALLER" >&2
  exit 1
fi

RELEASE_DIR="/Volumes/MAC SSD/artefacts/releases"
if [[ ! -d "$RELEASE_DIR" ]]; then
  echo "error: SSD artefacts dir missing: $RELEASE_DIR" >&2
  exit 2
fi

BASE="$(basename "$INSTALLER")"
STAGED="$RELEASE_DIR/$BASE"

cp -f "$INSTALLER" "$STAGED"
HASH=$(shasum -a 256 "$STAGED" | awk '{print $1}')
SIZE=$(du -h "$STAGED" | awk '{print $1}')

echo "Staged: $STAGED"
echo "Size:   $SIZE"
echo "SHA256: $HASH"
echo ""

case "${BASE,,}" in
  *.msi)
    PLATFORM="Windows 11 ARM64"
    VM_NAME="jura-win11-arm"
    FILE_PATH_IN_VM="E:\\releases\\$BASE (UTM shared folder default)"
    ;;
  *.appimage|*.deb)
    PLATFORM="Ubuntu 24.04 ARM64"
    VM_NAME="jura-ubuntu-2404"
    FILE_PATH_IN_VM="/media/utm-shared/releases/$BASE"
    ;;
  *.dmg)
    echo "macOS installer — test locally, no VM needed:"
    echo "  open \"$STAGED\""
    exit 0
    ;;
  *)
    echo "Unknown installer type: $BASE"
    exit 3
    ;;
esac

cat <<EOF
Manual test checklist ($PLATFORM):

  1. Open UTM → select '$VM_NAME' → click 'Revert to snapshot' (baseline)
  2. Start the VM
  3. In the guest, navigate to: $FILE_PATH_IN_VM
  4. Run the installer, click through the wizard
  5. Launch Jura Trace from the application menu
  6. Golden-path test:
     a. Drop a sample image onto Verify
     b. Confirm the sidecar spins up (status pill green)
     c. Confirm C2PA manifest read + forensics run without error
     d. Check Settings → service status — no warnings
  7. Edge cases:
     - Disconnect from internet, relaunch → should still work
     - On Windows: test SmartScreen behaviour (should accept signed MSI)
     - On Linux: test AppImage desktop integration (menu entry, icon)
  8. On failure, capture screenshot + sidecar log, file in:
     /Volumes/MAC SSD/artefacts/test-reports/${BASE%.*}/

Once the VM test passes, it's safe to tag the release.
EOF
