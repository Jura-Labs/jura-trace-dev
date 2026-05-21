#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Local macOS build for Jura Trace (aarch64-apple-darwin).
#
# Mirrors the .github/workflows/release.yml Phase-5 sequence so a developer
# can produce the same signed .app / DMG / updater payload locally without
# pushing a tag. Useful for:
#   1. Diagnosing CI failures (this script does NOT silence codesign stderr;
#      the CI per-file-sign loop redirects stderr to /dev/null and the broken
#      pipe symptom is what surfaces. Locally you see the real cause.)
#   2. Press sneak-preview builds when CI is broken.
#   3. Developer test installs.
#
# Output: src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/Jura Trace_VERSION_aarch64.dmg
#
# Notarisation is OPTIONAL. By default the DMG is signed but not notarised.
# Gatekeeper will permit install on the building machine (your Developer ID
# cert is trusted in your own keychain) but will warn on other Macs unless
# notarised. Set APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID to enable.
#
# Updater payload (.app.tar.gz.sig) is OPTIONAL. Set TAURI_SIGNING_PRIVATE_KEY
# + TAURI_SIGNING_PRIVATE_KEY_PASSWORD to produce a signed updater payload.
# Without these, the .sig will be empty (fine for local install, breaks the
# auto-updater install path; the DMG itself still works).

set -euo pipefail

# ── Paths ─────────────────────────────────────────────────────────────
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

TARGET="aarch64-apple-darwin"

# Honour CARGO_TARGET_DIR if set (the user may point Cargo at an external
# disk to keep target/ off the system drive). Falls back to the in-tree
# src-tauri/target/ if unset.
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  CARGO_TARGET_BASE="$CARGO_TARGET_DIR"
else
  CARGO_TARGET_BASE="$REPO_ROOT/src-tauri/target"
fi
BUNDLE_DIR="$CARGO_TARGET_BASE/${TARGET}/release/bundle"

SIDECAR_DIST="sidecar/dist/jura-sidecar"
SIDECAR_STAGING="src-tauri/sidecar-bundle"
SIGNING_IDENTITY="Developer ID Application: Jura Labs CIC (Y82C4P9L7F)"
TEAM_ID="Y82C4P9L7F"

# ── Logging helpers ───────────────────────────────────────────────────
log() { printf "\n\033[1;34m==>\033[0m \033[1m%s\033[0m\n" "$*"; }
ok()  { printf "\033[1;32m✓\033[0m %s\n" "$*"; }
warn(){ printf "\033[1;33m!\033[0m %s\n" "$*"; }
die() { printf "\033[1;31m✗\033[0m %s\n" "$*" >&2; exit 1; }

# ── Pre-flight ────────────────────────────────────────────────────────
log "Pre-flight checks"

[[ "$(uname -s)" == "Darwin" ]] || die "This script is macOS-only."
[[ "$(uname -m)" == "arm64" ]] || die "This script is Apple Silicon (arm64) only."

command -v cargo >/dev/null || die "cargo not found. Install Rust via https://rustup.rs"
command -v npm >/dev/null   || die "npm not found. Install Node.js 20+."
command -v python3 >/dev/null || die "python3 not found."
command -v codesign >/dev/null || die "codesign not found (Xcode Command Line Tools)."

if ! security find-identity -v -p codesigning | grep -q "${TEAM_ID}"; then
  die "Apple Developer ID cert for ${TEAM_ID} not in login keychain. See docs/install-guides/macos-signing-setup.md."
fi
ok "Developer ID cert present in keychain: ${SIGNING_IDENTITY}"

[[ -f sidecar/jura-sidecar.spec ]] || die "sidecar/jura-sidecar.spec not found."
ok "Sidecar spec present."

# Optional env vars: report status without failing
if [[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  ok "TAURI_SIGNING_PRIVATE_KEY set — updater payload will be minisign-signed."
else
  warn "TAURI_SIGNING_PRIVATE_KEY not set — updater payload .sig will be empty."
fi

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
  ok "Apple notarisation creds set — DMG will be notarised + stapled."
  NOTARISE=1
else
  warn "Apple notarisation creds not set — DMG signed but NOT notarised."
  warn "Set APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID to notarise for off-machine install."
  NOTARISE=0
fi

# ── Clean previous artefacts (idempotent re-runs) ─────────────────────
log "Cleaning previous build artefacts"
log "(Cargo target base: $CARGO_TARGET_BASE)"
rm -rf "$BUNDLE_DIR"
rm -rf "$SIDECAR_STAGING"
rm -rf "$SIDECAR_DIST"
rm -rf sidecar/build
ok "Cleaned."

# ── Frontend build ────────────────────────────────────────────────────
log "Building SvelteKit frontend"
(cd ui && npm install --silent && npm run build --silent)
ok "Frontend built to ui/build/"

# ── Sidecar build (PyInstaller --onedir) ──────────────────────────────
log "Building Python sidecar (PyInstaller --onedir, may take 5-10 min)"
(cd sidecar && JURA_SIDECAR_ONEDIR=1 python3 -m PyInstaller jura-sidecar.spec --noconfirm)
[[ -x "$SIDECAR_DIST/jura-sidecar" ]] || die "Sidecar bootloader missing after PyInstaller."
[[ -d "$SIDECAR_DIST/_internal" ]] || die "Sidecar _internal/ missing after PyInstaller."
ok "Sidecar built ($(du -sh "$SIDECAR_DIST" | cut -f1))"

# ── Frozen-sidecar smoke test ─────────────────────────────────────────
# Launches the bundled binary and probes each /forensics/* endpoint to
# catch the class of bug where a feature works in dev (full Python env)
# but fails in the frozen bundle (missing transitive deps). Added after
# the 2026-05-21 imwatermark/torch incident.
log "Smoke-testing the frozen sidecar (probes each /forensics endpoint)"
if ! python3 scripts/smoke_test_frozen_sidecar.py "$SIDECAR_DIST/jura-sidecar"; then
  die "Frozen-sidecar smoke test failed. See output above."
fi
ok "All bundled /forensics endpoints respond without bundle-import failures."

# ── Stage sidecar-bundle into src-tauri/ ──────────────────────────────
log "Staging sidecar-bundle for Tauri resources"
mkdir -p "$SIDECAR_STAGING"
cp -R "$SIDECAR_DIST"/. "$SIDECAR_STAGING"/
chmod +x src-tauri/binaries/jura-sidecar-aarch64-apple-darwin 2>/dev/null || true
ok "Staged to $SIDECAR_STAGING ($(du -sh "$SIDECAR_STAGING" | cut -f1))"

# ── Copy model files ──────────────────────────────────────────────────
log "Copying model files into src-tauri/models/"
mkdir -p src-tauri/models
cp models/deepfake_classifier.joblib src-tauri/models/ 2>/dev/null || warn "deepfake_classifier.joblib not found in models/ — verify external USB mount."
cp models/univfd_probe.joblib src-tauri/models/ 2>/dev/null || warn "univfd_probe.joblib not found in models/ — verify external USB mount."
ok "Model files: $(ls src-tauri/models/ | tr '\n' ' ')"

# ── Phase 1: build + sign .app only (NO DMG, NO notarisation yet) ─────
log "Phase 1: cargo tauri bundle --bundles app (signs .app top level)"
(cd src-tauri && cargo tauri bundle --bundles app --target "$TARGET")

APP_PATH=$(find "$BUNDLE_DIR/macos" -maxdepth 1 -name "*.app" -type d | head -1)
[[ -n "$APP_PATH" ]] || die "No .app found under $BUNDLE_DIR/macos after Phase 1."
ok "Built .app: $APP_PATH"

# ── Phase 5a: per-file sign nested sidecar-bundle (with visible stderr) ─
log "Phase 5a: per-file sign nested .so/.dylib (deepest-first)"
log "(stderr is NOT redirected here — CI's silenced version is the bug we're diagnosing)"

SIDECAR_ROOT="$APP_PATH/Contents/Resources/sidecar-bundle"
[[ -d "$SIDECAR_ROOT" ]] || die "sidecar-bundle not in .app at $SIDECAR_ROOT. tauri.conf.json resources glob may be wrong."

count=0
failed=0
while IFS= read -r f; do
  if codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$f"; then
    count=$((count + 1))
  else
    failed=$((failed + 1))
    warn "codesign FAILED on: $f"
    # Show extended attributes that might be blocking
    xattr -l "$f" 2>&1 | head -3
  fi
done < <(find "$SIDECAR_ROOT" -type f \( -name "*.so" -o -name "*.dylib" \) | sort -r)

if [[ $failed -gt 0 ]]; then
  die "$failed nested file(s) failed codesign. See errors above. Total OK: $count."
fi
ok "Per-file signed $count nested .so/.dylib files."

# Sign bootloader
codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$SIDECAR_ROOT/jura-sidecar"
ok "Signed sidecar bootloader."

# Re-seal .app top level so nested hashes are embedded
codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$APP_PATH"
ok "Re-sealed .app top level."

# ── Phase 5b: verify deep ─────────────────────────────────────────────
log "Phase 5b: verify deep signing"
codesign --verify --deep --strict --verbose=2 "$APP_PATH" 2>&1 | tail -5
ok "Deep signing verified."

# Spot-check nested files are signed by Y82C4P9L7F
log "Phase 5c: spot-check 5 nested files for TeamIdentifier=${TEAM_ID}"
checked=0
mis_signed=0
while IFS= read -r f; do
  checked=$((checked + 1))
  ident=$(codesign --display --verbose=2 "$f" 2>&1 | awk -F= '/^TeamIdentifier=/{print $2}' | head -1)
  if [[ "$ident" != "$TEAM_ID" ]]; then
    warn "MIS-SIGNED: $f Team='$ident'"
    mis_signed=$((mis_signed + 1))
  else
    ok "$(basename "$f") Team=$ident"
  fi
done < <(find "$SIDECAR_ROOT" -type f \( -name "*.dylib" -o -name "*.so" \) | head -5)
[[ $mis_signed -eq 0 ]] || die "$mis_signed of $checked sampled files mis-signed."

# ── Phase 6: regenerate DMG + updater payload from re-signed .app ─────
log "Phase 6: cargo tauri bundle --bundles dmg,updater (uses re-signed .app)"
(cd src-tauri && cargo tauri bundle --bundles dmg,updater --target "$TARGET")

DMG_PATH=$(find "$BUNDLE_DIR/dmg" -maxdepth 1 -name "*.dmg" | head -1)
[[ -n "$DMG_PATH" ]] || die "No DMG found after Phase 6 regenerate."
ok "DMG built: $DMG_PATH ($(du -sh "$DMG_PATH" | cut -f1))"

# ── Phase 7: codesign the DMG itself ──────────────────────────────────
log "Phase 7: codesign the DMG"
codesign --force --sign "$SIGNING_IDENTITY" --timestamp "$DMG_PATH"
codesign --verify --verbose=2 "$DMG_PATH" 2>&1 | tail -3
ok "DMG signed."

# ── Phase 8 (optional): notarise + staple ─────────────────────────────
if [[ "$NOTARISE" == "1" ]]; then
  log "Phase 8: notarise via xcrun notarytool (may take 1-10 min)"
  xcrun notarytool submit "$DMG_PATH" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  ok "Notarisation accepted."

  log "Stapling notarisation ticket to DMG and .app"
  xcrun stapler staple "$DMG_PATH"
  xcrun stapler staple "$APP_PATH"
  xcrun stapler validate "$DMG_PATH"
  xcrun stapler validate "$APP_PATH"
  spctl --assess --verbose=2 --type install "$DMG_PATH" 2>&1 | tail -3
  ok "Stapled + Gatekeeper-validated."
fi

# ── Final summary ─────────────────────────────────────────────────────
log "Build complete"
echo
echo "DMG:        $DMG_PATH"
echo ".app:       $APP_PATH"
echo "Updater:    $(find "$BUNDLE_DIR/macos" -name "*.app.tar.gz" | head -1)"
echo "Updater .sig: $(find "$BUNDLE_DIR/macos" -name "*.app.tar.gz.sig" | head -1)"
echo
if [[ "$NOTARISE" == "1" ]]; then
  echo "Status: Signed + Notarised + Stapled. Installable on any Mac."
else
  echo "Status: Signed (not notarised). Installable on YOUR Mac. Other Macs will see Gatekeeper warning."
fi
echo
echo "To install:"
echo "  open \"$DMG_PATH\""
