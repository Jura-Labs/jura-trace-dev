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

# Auto-load TAURI_SIGNING_PRIVATE_KEY from ~/.tauri/jura-trace-v10.key if
# not already set. The key was generated fresh (2026-05-22) with an empty
# password, so TAURI_SIGNING_PRIVATE_KEY_PASSWORD is not needed. Without
# this auto-load, local builds skip the updater payload (.app.tar.gz +
# .sig) and only produce the .dmg, which means rc.x testers can never
# auto-update to the next rc. File mode is 600; gitignored.
TAURI_KEY_FILE="${HOME}/.tauri/jura-trace-v10.key"
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -r "$TAURI_KEY_FILE" ]]; then
  TAURI_SIGNING_PRIVATE_KEY="$(cat "$TAURI_KEY_FILE")"
  export TAURI_SIGNING_PRIVATE_KEY
  # Empty-password key generated 2026-05-22; the env var still needs to
  # be set (Tauri reads it even when empty) to disable password prompting.
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
  ok "TAURI_SIGNING_PRIVATE_KEY loaded from $TAURI_KEY_FILE (empty password)."
elif [[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  ok "TAURI_SIGNING_PRIVATE_KEY already set in environment."
else
  warn "TAURI_SIGNING_PRIVATE_KEY not set and $TAURI_KEY_FILE not readable."
  warn "Updater payload .sig will be empty. Auto-updater will refuse to apply this release."
fi

# ── Apple notarisation creds: env > .env.local > keychain profile > skip ─
#
# Three local-setup options, tried in order. Pick whichever fits your
# discipline. All three avoid committing secrets.
#
#   1. Shell env (already-set in this shell, CI, or a one-off `source`
#      step). Honoured first so CI semantics match local behaviour.
#
#   2. .env.local at the repo root (gitignored). Convenient for laptop dev:
#      drop the three vars into a single file and forget. Sourced before
#      the env-var check below so it feeds into the same path.
#
#   3. macOS keychain profile named 'jura-trace-notary'. Most secure of
#      the three — credentials never sit in plaintext on disk. Set up once
#      with:
#          xcrun notarytool store-credentials "jura-trace-notary" \
#              --apple-id <email> --team-id Y82C4P9L7F --password <app-pw>
#      Phase 8 below uses `--keychain-profile` when this is found, so no
#      env-var plumbing is required.
#
# If none of the three resolves, the build still completes — the DMG is
# signed but not notarised, suitable for testing on this Mac only.

NOTARY_PROFILE="jura-trace-notary"
USE_KEYCHAIN_PROFILE=0

# Path 2: source .env.local (gitignored) if it exists.
if [[ -f "$REPO_ROOT/.env.local" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env.local"
  set +a
  ok ".env.local sourced."
fi

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
  ok "Apple notarisation creds set in environment — DMG will be notarised + stapled."
  NOTARISE=1
elif security find-generic-password -s "com.apple.gke.notary.tool" \
        -a "$NOTARY_PROFILE" >/dev/null 2>&1; then
  ok "Apple notarisation creds in keychain profile '$NOTARY_PROFILE' — DMG will be notarised + stapled."
  NOTARISE=1
  USE_KEYCHAIN_PROFILE=1
else
  warn "Apple notarisation creds not set — DMG signed but NOT notarised."
  warn "To enable: set APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID in env, drop them in"
  warn "  $REPO_ROOT/.env.local (gitignored), or store them once via:"
  warn "  xcrun notarytool store-credentials \"$NOTARY_PROFILE\" \\"
  warn "    --apple-id <email> --team-id Y82C4P9L7F --password <app-specific>"
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

# ── Phase 0.5: per-file sign nested Mach-O binaries in SOURCE ─────────
# CRITICAL FIX (2026-06-07, post-rc.29 notarisation failure): cargo tauri
# bundle's Phase 6 (--bundles dmg,updater) re-copies from src-tauri/
# sidecar-bundle/ into the .app, OVERWRITING any signatures Phase 5a
# applied to .app/Contents/Resources/sidecar-bundle/. The .app's
# top-level re-seal then bakes in hashes of unsigned nested files,
# producing a DMG that fails notarisation with "binary is not signed
# with a valid Developer ID certificate" on every nested .so/.dylib.
#
# Proper fix: sign the SOURCE at $SIDECAR_STAGING/ before any cargo
# tauri bundle invocation. Both Phase 1 and Phase 6 then copy already-
# signed files into the .app. Phase 5a below remains as belt-and-braces
# (no-op idempotent re-sign).
#
# rc.29 was rescued by a 30-min manual recovery (extract .app from DMG,
# per-file sign 611 binaries, rebuild DMG, re-notarise). This phase
# eliminates that recovery step from every future build.
log "Phase 0.5: per-file sign nested Mach-O binaries in SOURCE"
log "(signs $SIDECAR_STAGING/ BEFORE Tauri bundlers copy from it)"

sign_count=0
sign_failed=0
sign_skipped=0
while IFS= read -r f; do
  # Skip if file is empty (defensive)
  [[ -s "$f" ]] || { sign_skipped=$((sign_skipped + 1)); continue; }
  if codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$f" 2>/dev/null; then
    sign_count=$((sign_count + 1))
  else
    sign_failed=$((sign_failed + 1))
    warn "codesign FAILED on: $f"
    codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$f" 2>&1 | head -3 || true
  fi
done < <(
  # Find every Mach-O in $SIDECAR_STAGING/ by INSPECTING content
  # (file -b), not relying on file extension. Catches the PyInstaller
  # bootloader at $SIDECAR_STAGING/jura-sidecar (no extension), .so,
  # .dylib, and any other Mach-O object that PyInstaller may include
  # in future dependency updates. Sort by path length descending so
  # deepest files sign first (parents seal after children).
  find "$SIDECAR_STAGING" -type f \
    \( -name "*.so" -o -name "*.dylib" -o ! -name "*.*" \) \
    -exec sh -c 'file -b "$1" 2>/dev/null | grep -q "Mach-O" && echo "$1"' _ {} \; 2>/dev/null \
  | awk '{print length($0), $0}' | sort -rn | cut -d' ' -f2-
)

if [[ $sign_failed -gt 0 ]]; then
  die "$sign_failed source file(s) failed codesign. Total OK: $sign_count, skipped: $sign_skipped."
fi
ok "Per-file signed $sign_count Mach-O binaries in $SIDECAR_STAGING/ (skipped $sign_skipped empty)."

# ── Copy model files ──────────────────────────────────────────────────
log "Copying model files into src-tauri/models/"
mkdir -p src-tauri/models
cp models/deepfake_classifier.joblib src-tauri/models/ 2>/dev/null || warn "deepfake_classifier.joblib not found in models/ — verify external USB mount."
cp models/univfd_probe.joblib src-tauri/models/ 2>/dev/null || warn "univfd_probe.joblib not found in models/ — verify external USB mount."
ok "Model files: $(ls src-tauri/models/ | tr '\n' ' ')"

# ── Cargo macro-cache invalidation ────────────────────────────────────
# tauri::generate_context! embeds the contents of ui/build/ at compile
# time. Cargo's incremental cache does NOT detect when files INSIDE
# ui/build/ change, it only watches Rust source. Result: rebuilding the
# frontend then running `cargo tauri bundle` re-signs the app with a
# stale embedded frontend (chunks with old content hashes). The .app
# passes codesign and launches, but users see the previous build's UI.
#
# Workaround (2026-05-21 incident): nuke the cargo fingerprint dir for
# jura-trace AND the release binary, then run `cargo build --release`
# explicitly so cargo re-runs the macro and re-embeds the fresh dist
# BEFORE the bundle step (which only bundles, never compiles).
log "Invalidating cargo macro cache + rebuilding (forces fresh frontend embed)"
rm -f "$CARGO_TARGET_BASE/${TARGET}/release/jura-trace"
rm -rf "$CARGO_TARGET_BASE/${TARGET}/release/.fingerprint/jura-trace-"*
(cd src-tauri && cargo build --release --target "$TARGET")
[[ -x "$CARGO_TARGET_BASE/${TARGET}/release/jura-trace" ]] || die "cargo build --release did not produce the binary."
ok "Macro cache invalidated and binary rebuilt."

# ── Phase 1: build + sign .app only (NO DMG, NO notarisation yet) ─────
log "Phase 1: cargo tauri bundle --bundles app (signs .app top level)"
(cd src-tauri && cargo tauri bundle --bundles app --target "$TARGET")

APP_PATH=$(find "$BUNDLE_DIR/macos" -maxdepth 1 -name "*.app" -type d | head -1)
[[ -n "$APP_PATH" ]] || die "No .app found under $BUNDLE_DIR/macos after Phase 1."
ok "Built .app: $APP_PATH"

# ── Phase 5a: per-file sign nested sidecar-bundle (belt-and-braces) ───
# Phase 0.5 above signs SOURCE at $SIDECAR_STAGING, which is the proper
# fix for the rc.29-class notarisation failure. Phase 5a here re-signs
# the SAME files inside the .app as a defensive idempotent pass — covers
# the case where cargo tauri bundle's Phase-1 signing strips or modifies
# nested sigs in any way we haven't anticipated. Cost: ~10-30 seconds of
# build time; risk of removal: a future Tauri release changes its
# bundler behaviour silently and we don't catch it until notarisation.
log "Phase 5a: per-file sign nested .so/.dylib in .app (belt-and-braces)"
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

# ── Phase 6.5: verify final .app has properly-signed nested binaries ──
# Build-time gate that catches yesterday's failure mode (rc.29: Phase 6
# regenerated unsigned nested files from source, .app passed top-level
# verify but notarisation rejected with "binary is not signed with a
# valid Developer ID certificate" on 600+ nested .so/.dylib).
#
# Sample the .app's nested Mach-O binaries and confirm every signature
# bears TeamIdentifier=Y82C4P9L7F. If any nested binary is unsigned or
# mis-signed, fail the build BEFORE Phase 7 / Phase 8 so the operator
# sees the problem at build time, not as a 25-minute notarisation
# rejection later.
log "Phase 6.5: verify final .app nested signing"

POST6_SIDECAR_ROOT="$APP_PATH/Contents/Resources/sidecar-bundle"
mis_signed=0
checked=0
# Verify every Mach-O in the final .app. Same content-based detection
# as Phase 0.5.
while IFS= read -r f; do
  checked=$((checked + 1))
  ident=$(codesign --display --verbose=2 "$f" 2>&1 | awk -F= '/^TeamIdentifier=/{print $2}' | head -1)
  if [[ "$ident" != "$TEAM_ID" ]]; then
    mis_signed=$((mis_signed + 1))
    warn "POST-BUNDLE MIS-SIGNED: $f Team='${ident:-NONE}'"
  fi
done < <(
  find "$POST6_SIDECAR_ROOT" -type f \
    \( -name "*.so" -o -name "*.dylib" -o ! -name "*.*" \) \
    -exec sh -c 'file -b "$1" 2>/dev/null | grep -q "Mach-O" && echo "$1"' _ {} \; 2>/dev/null
)

if [[ $mis_signed -gt 0 ]]; then
  die "$mis_signed of $checked nested Mach-O binaries in the final .app are mis-signed. \
This is the rc.29-class notarisation failure mode. Phase 0.5 (source signing) or Phase 5a \
(in-.app signing) is not effective; investigate cargo tauri bundle behaviour."
fi
ok "All $checked nested Mach-O binaries in final .app signed by Team=$TEAM_ID."

# ── Phase 7: codesign the DMG itself ──────────────────────────────────
log "Phase 7: codesign the DMG"
codesign --force --sign "$SIGNING_IDENTITY" --timestamp "$DMG_PATH"
codesign --verify --verbose=2 "$DMG_PATH" 2>&1 | tail -3
ok "DMG signed."

# ── Phase 8 (optional): notarise + staple ─────────────────────────────
if [[ "$NOTARISE" == "1" ]]; then
  log "Phase 8: notarise via xcrun notarytool (may take 1-10 min)"
  if [[ "$USE_KEYCHAIN_PROFILE" == "1" ]]; then
    # Read creds from the macOS keychain profile — no env-var plumbing.
    xcrun notarytool submit "$DMG_PATH" \
      --keychain-profile "$NOTARY_PROFILE" \
      --wait
  else
    xcrun notarytool submit "$DMG_PATH" \
      --apple-id "$APPLE_ID" \
      --password "$APPLE_PASSWORD" \
      --team-id "$APPLE_TEAM_ID" \
      --wait
  fi
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
