# macOS CI Release Job — Design Draft

**Status**: DRAFT — post-launch review only (v1.0.1+)  
**Created**: 2026-06-18  
**Author**: DevOps agent  
**DO NOT MERGE** before Mon 22 June 2026 launch. Tracked in backlog as post-v1.0 item.

---

## Context

macOS release builds currently run locally on an Apple Silicon Mac, not in CI. The root cause is the existing CI "Phase 5 — per-file sign nested sidecar-bundle code" step in `.github/workflows/release.yml` redirects all codesign stderr to `/dev/null` inside a `set -euo pipefail` loop. When any of the ~610 nested Mach-O files fails to sign, the only surfaced error is `sort: stdout: Broken pipe` — the actual failing file and codesign error message are swallowed. This made the macOS CI job unreliable to debug.

The local script `scripts/build-local-mac.sh` does NOT silence codesign, and it includes the rc.29-era Phase 0.5 fix (signing the SOURCE staging directory before `cargo tauri bundle` copies from it). This draft brings those improvements into a self-contained CI job.

---

## Proposed Job YAML

Drop this into `.github/workflows/release.yml` as a replacement for the macOS matrix entry's steps, or as a standalone job. It is written as a standalone job for clarity. In production it would replace the `aarch64-apple-darwin` matrix path.

```yaml
# ── macOS release job (DRAFT — post-v1.0.1 review) ─────────────────────────
#
# This job mirrors scripts/build-local-mac.sh exactly, including:
#   - Phase 0.5: sign SOURCE sidecar-bundle/ before cargo tauri bundle copies it
#   - Phase 5a: belt-and-braces re-sign inside the .app
#   - Phase 6.5: full nested-signing verification gate (fail BEFORE notarisation)
#   - Phase 7: codesign the DMG
#   - Phase 8: notarytool --wait + stapler + spctl gate
#
# THE BUG FIXED vs the current release.yml Phase 5 step:
#   The existing step has `>/dev/null 2>&1` on the codesign call inside
#   `set -euo pipefail`. When any of the ~610 nested .so/.dylib fails to
#   sign, the loop exits on the next `sort` call whose stdout pipe breaks
#   — the operator sees only "sort: stdout: Broken pipe" and has no idea
#   which file failed. This job captures and prints the failing file +
#   codesign stderr before failing the step.
#
# NOTARISATION: prefers App Store Connect API key (APPLE_API_KEY /
# APPLE_API_ISSUER / APPLE_API_KEY_ID) over the Apple-ID trio because
# API-key auth does not require a per-account 2FA step and is more
# reliable in CI. Falls back to the Apple-ID trio when the API key
# secrets are not set (backwards-compatible with the current secret
# inventory).
#
# RUNNERS:
#   - GitHub-hosted: macos-latest (Sequoia, ARM64)
#   - Self-hosted:   ["self-hosted","macOS","ARM64","m4"]
#   Swap the `runner` key below to switch. Self-hosted notes in
#   PORTABILITY section at the end of this file.
#
# REQUIRED SECRETS (Jura-Labs/jura-archive):
#   APPLE_CERTIFICATE           — base64-encoded .p12 Developer ID cert
#   APPLE_CERTIFICATE_PASSWORD  — passphrase for the .p12
#   APPLE_SIGNING_IDENTITY      — "Developer ID Application: Jura Labs CIC (Y82C4P9L7F)"
#   RELEASE_PAT                 — GitHub PAT with repo scope on Jura-Labs/jura-trace
#   TAURI_SIGNING_PRIVATE_KEY   — minisign private key (updater payload signing)
#   TAURI_SIGNING_PRIVATE_KEY_PASSWORD — empty string for the 2026-05-22 keypair
#
#   Notarisation (preferred — App Store Connect API key):
#   APPLE_API_KEY_ID            — Key ID (10-char string, e.g. "ABCD1234EF")
#   APPLE_API_ISSUER            — Issuer UUID (from App Store Connect → Integrations)
#   APPLE_API_KEY               — base64-encoded .p8 private key content
#
#   Notarisation (fallback — Apple ID):
#   APPLE_ID                    — Apple ID email address
#   APPLE_PASSWORD              — app-specific password (NOT the Apple ID password)
#   APPLE_TEAM_ID               — "Y82C4P9L7F"

  build-macos-ci-draft:
    name: Build macOS (Apple Silicon) [DRAFT]
    needs: [create-release]
    runs-on: macos-latest
    # Cold build: PyInstaller ~10-15 min + cargo ~15-25 min + notarisation ~5-10 min.
    # 90 minutes covers the worst-case cold cache with notarisation.
    timeout-minutes: 90

    steps:
      - uses: actions/checkout@v4

      # ── Clean stray hdiutil mounts and running Jura Trace instances ────────
      # On a self-hosted runner (not ephemeral) leftover mounts from a
      # previous failed build cause "hdiutil: create failed" in the Tauri
      # DMG bundler. On cloud runners this is defensive but harmless.
      - name: Pre-flight cleanup (detach stray hdiutil mounts)
        shell: bash
        run: |
          for d in $(hdiutil info 2>/dev/null | awk '/^\/dev\/disk/ {print $1}'); do
            hdiutil detach "$d" -force 2>/dev/null || true
          done
          # Kill any lingering Jura Trace process — codesign cannot sign
          # a running .app on macOS (EPERM on the bundle root).
          pkill -x "Jura Trace" 2>/dev/null || true

      # ── Import Apple Developer ID cert into a fresh temporary keychain ─────
      # A temporary keychain (not login) is used so:
      #   a) the cert is not left in the user's login keychain after the run
      #   b) codesign can find it without GUI prompts (partition list open)
      # The keychain is locked and deleted in the cleanup step at job end.
      # security set-key-partition-list is required to allow codesign to use
      # the private key without a "codesign wants to access key" dialog,
      # which would hang CI indefinitely.
      - name: Import signing certificate into temporary keychain
        env:
          APPLE_CERTIFICATE:          ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
        shell: bash
        run: |
          set -euo pipefail
          KEYCHAIN_PATH="$RUNNER_TEMP/jura-signing.keychain-db"
          KEYCHAIN_PASS="jura-ci-$(uuidgen)"
          # Store for downstream steps (keychain path + temp password)
          echo "KEYCHAIN_PATH=$KEYCHAIN_PATH" >> "$GITHUB_ENV"
          echo "KEYCHAIN_PASS=$KEYCHAIN_PASS" >> "$GITHUB_ENV"

          # Create and unlock a fresh keychain
          security create-keychain -p "$KEYCHAIN_PASS" "$KEYCHAIN_PATH"
          security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
          security unlock-keychain -p "$KEYCHAIN_PASS" "$KEYCHAIN_PATH"

          # Decode the base64 cert and import it
          CERT_PATH="$RUNNER_TEMP/certificate.p12"
          echo "$APPLE_CERTIFICATE" | base64 --decode > "$CERT_PATH"
          security import "$CERT_PATH" \
            -k "$KEYCHAIN_PATH" \
            -P "$APPLE_CERTIFICATE_PASSWORD" \
            -T /usr/bin/codesign \
            -T /usr/bin/security \
            -T /usr/bin/productbuild \
            -f pkcs12
          rm -f "$CERT_PATH"

          # Make the temp keychain visible to codesign without prompting.
          # list-keychains -s REPLACES the search list; include both the
          # temp keychain and the login keychain so other tools still work.
          security list-keychains -d user -s "$KEYCHAIN_PATH" \
            "$(security list-keychains -d user | head -1 | tr -d '"')"

          # Allow codesign to access the private key non-interactively.
          # The partition list "apple-tool:,apple:,codesign:" covers the
          # codesign binary invocations and the Security framework callbacks.
          security set-key-partition-list \
            -S "apple-tool:,apple:,codesign:" \
            -s -k "$KEYCHAIN_PASS" \
            "$KEYCHAIN_PATH"

          echo "Temporary keychain created: $KEYCHAIN_PATH"
          security find-identity -v -p codesigning "$KEYCHAIN_PATH"

      # ── Rust toolchain ───────────────────────────────────────────────────────
      - name: Install Rust 1.88
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: "1.88"
          targets: aarch64-apple-darwin

      - name: Cache Cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            src-tauri/target/
          key: cargo-release-aarch64-apple-darwin-${{ hashFiles('src-tauri/Cargo.lock') }}
          restore-keys: cargo-release-aarch64-apple-darwin-

      # ── Python / PyInstaller ─────────────────────────────────────────────────
      - name: Setup Python 3.12
        uses: actions/setup-python@v5
        with:
          python-version: "3.12"

      - name: Cache pip
        uses: actions/cache@v4
        with:
          path: ~/.cache/pip
          key: pip-pyinstaller-aarch64-apple-darwin-${{ hashFiles('sidecar/requirements.lock') }}
          restore-keys: pip-pyinstaller-aarch64-apple-darwin-

      - name: Install sidecar dependencies
        shell: bash
        run: |
          if [ -f sidecar/requirements-ci.txt ]; then
            pip install -r sidecar/requirements-ci.txt
          elif [ -f sidecar/requirements.lock ]; then
            pip install -r sidecar/requirements.lock
          else
            pip install -r sidecar/requirements.txt
          fi
          pip install "pyinstaller>=6,<7"

      # ── Build frozen sidecar (--onedir, macOS only) ─────────────────────────
      - name: Build frozen sidecar (PyInstaller --onedir)
        shell: bash
        env:
          JURA_SIDECAR_ONEDIR: "1"
        run: python -m PyInstaller jura-sidecar.spec --noconfirm
        working-directory: sidecar

      - name: Validate sidecar --onedir output
        shell: bash
        run: |
          set -euo pipefail
          [[ -x sidecar/dist/jura-sidecar/jura-sidecar ]] \
            || { echo "::error::Sidecar bootloader missing after PyInstaller --onedir"; exit 1; }
          [[ -d sidecar/dist/jura-sidecar/_internal ]] \
            || { echo "::error::Sidecar _internal/ missing after PyInstaller --onedir"; exit 1; }
          echo "Sidecar --onedir output OK: $(du -sh sidecar/dist/jura-sidecar | cut -f1)"

      # ── Stage sidecar-bundle into src-tauri/ ────────────────────────────────
      - name: Stage sidecar-bundle into src-tauri/sidecar-bundle/
        shell: bash
        run: |
          mkdir -p src-tauri/sidecar-bundle
          cp -R sidecar/dist/jura-sidecar/. src-tauri/sidecar-bundle/
          chmod +x src-tauri/binaries/jura-sidecar-aarch64-apple-darwin 2>/dev/null || true
          echo "Staged: $(du -sh src-tauri/sidecar-bundle | cut -f1)"

      # ── Phase 0.5: sign SOURCE sidecar-bundle BEFORE cargo tauri bundle ─────
      #
      # CRITICAL: cargo tauri bundle RE-COPIES src-tauri/sidecar-bundle/ into
      # the .app, overwriting any signatures applied to files already inside
      # a previously-built .app. Signing the SOURCE here means both the
      # initial `--bundles app` call and the subsequent `--bundles dmg,updater`
      # call copy already-signed Mach-O files into the .app.
      #
      # The rc.29-class failure: Phase 5a signing inside the .app was correct
      # but `cargo tauri bundle --bundles dmg,updater` then re-copied UNSIGNED
      # source files from src-tauri/sidecar-bundle/ on top of the signed ones,
      # producing a DMG rejected by notarytool with "binary is not signed with
      # a valid Developer ID certificate" on 600+ nested .so/.dylib files.
      #
      # BUG FIXED vs release.yml Phase 5 step:
      #   Old:  codesign ... >/dev/null 2>&1   (silences errors; only "sort:
      #         stdout: Broken pipe" surfaces when codesign fails)
      #   New:  errors are captured and printed before the build fails, so the
      #         operator knows which of the ~610 files caused the failure.
      - name: Phase 0.5 — sign SOURCE sidecar-bundle/ (pre-bundle, fixes rc.29 class)
        env:
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
        shell: bash
        run: |
          set -euo pipefail
          STAGING="src-tauri/sidecar-bundle"
          sign_count=0
          sign_failed=0
          sign_skipped=0

          # Find every Mach-O by content (not extension) so we catch the
          # bootloader (no extension), .so, and .dylib. Sort by path length
          # descending: deepest files first so parent-bundle seals happen
          # after their children.
          while IFS= read -r f; do
            [[ -s "$f" ]] || { sign_skipped=$((sign_skipped + 1)); continue; }
            # Capture stderr explicitly — do NOT redirect to /dev/null.
            # This is the fix for the CI "sort: Broken pipe" symptom.
            codesign_err="$(codesign --force \
              --sign "$APPLE_SIGNING_IDENTITY" \
              --options runtime \
              --timestamp \
              "$f" 2>&1)" && {
              sign_count=$((sign_count + 1))
            } || {
              sign_failed=$((sign_failed + 1))
              echo "::error::codesign FAILED on: $f"
              echo "  stderr: $codesign_err"
              # Print extended attributes — a common root cause is a quarantine
              # bit set by pip / PyInstaller on downloaded dylibs.
              xattr -l "$f" 2>/dev/null | head -5 || true
            }
          done < <(
            find "$STAGING" -type f \
              \( -name "*.so" -o -name "*.dylib" -o ! -name "*.*" \) \
              -exec sh -c 'file -b "$1" 2>/dev/null | grep -q "Mach-O" && echo "$1"' _ {} \; \
              2>/dev/null \
            | awk '{print length($0), $0}' | sort -rn | cut -d' ' -f2-
          )

          if [[ $sign_failed -gt 0 ]]; then
            echo "::error::Phase 0.5 failed: $sign_failed source file(s) failed codesign."
            echo "  Signed OK: $sign_count  Skipped (empty): $sign_skipped"
            exit 1
          fi
          echo "Phase 0.5 complete: signed $sign_count Mach-O files in $STAGING (skipped $sign_skipped empty)."

      # ── Copy ML model files ─────────────────────────────────────────────────
      # Models ship with the app but are not in the source repo (too large).
      # In CI they should be pre-downloaded to src-tauri/models/ via a
      # private artifact store or secrets-injected download step. If missing,
      # the build succeeds but the app will fail to load the deepfake classifier.
      # TODO: wire up a private S3 or GitHub artifact restore step here.
      - name: Copy model files to src-tauri/models/
        shell: bash
        run: |
          mkdir -p src-tauri/models
          # Attempt to copy from models/ if present (e.g. self-hosted runner
          # with the USB drive attached or a pre-checkout restore step).
          cp models/deepfake_classifier.joblib src-tauri/models/ 2>/dev/null \
            || echo "::warning::deepfake_classifier.joblib not found — sidecar will degrade gracefully."
          cp models/univfd_probe.joblib src-tauri/models/ 2>/dev/null \
            || echo "::warning::univfd_probe.joblib not found — sidecar will degrade gracefully."
          ls -lh src-tauri/models/ || true

      # ── Node.js: build SvelteKit frontend ───────────────────────────────────
      - name: Setup Node 20
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: ui/package-lock.json

      - name: Install frontend dependencies
        run: cd ui && npm ci

      # ── Install Tauri CLI ────────────────────────────────────────────────────
      - name: Install Tauri CLI
        run: |
          cargo install cargo-binstall --locked --version "~1.10"
          cargo binstall tauri-cli --version "^2" --no-confirm
        shell: bash

      # ── Cargo macro-cache invalidation ──────────────────────────────────────
      # tauri::generate_context! embeds ui/build/ at compile time. Cargo does
      # not detect changes to files INSIDE ui/build/ — only Rust source
      # changes invalidate the macro cache. Without this step a warm cache
      # re-bundles a stale frontend into the .app silently.
      # See scripts/build-local-mac.sh for the full explanation.
      - name: Invalidate cargo macro cache and rebuild binary
        shell: bash
        run: |
          TARGET_BASE="src-tauri/target/aarch64-apple-darwin/release"
          rm -f "$TARGET_BASE/jura-trace"
          rm -rf "$TARGET_BASE/.fingerprint/jura-trace-"*
          cd src-tauri && cargo build --release --target aarch64-apple-darwin
          [[ -x "../$TARGET_BASE/jura-trace" ]] \
            || { echo "::error::cargo build --release did not produce the binary"; exit 1; }
          echo "Binary rebuilt: $(du -sh ../$TARGET_BASE/jura-trace | cut -f1)"

      # ── Phase 1: build .app only (no DMG, no notarisation yet) ─────────────
      # Passing APPLE_CERTIFICATE + APPLE_CERTIFICATE_PASSWORD here allows
      # tauri-action's built-in codesign pass on the .app top level. We do NOT
      # pass APPLE_ID / APPLE_PASSWORD — that would trigger tauri-action's
      # internal notarisation against the not-yet-fully-signed .app.
      - name: Phase 1 — build .app (tauri-action --bundles app)
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN:               ${{ secrets.GITHUB_TOKEN }}
          APPLE_CERTIFICATE:          ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY:     ${{ secrets.APPLE_SIGNING_IDENTITY }}
          TAURI_SIGNING_PRIVATE_KEY:          ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tauriScript: cargo tauri
          # --bundles app only — DMG and updater are produced AFTER per-file signing.
          args: --target aarch64-apple-darwin --bundles app

      # ── Phase 5a: per-file sign nested .app sidecar-bundle (belt-and-braces) ─
      #
      # Phase 0.5 signs the SOURCE before any `cargo tauri bundle` invocation.
      # This phase re-signs the same files INSIDE the .app as a defensive pass —
      # in case a future Tauri release changes its bundler in a way that strips
      # or modifies nested signatures. The two phases together survive any
      # bundler behaviour we haven't anticipated.
      #
      # BUG FIXED: stderr is NOT redirected. Each failure prints the file path
      # and the full codesign error message so CI logs are actionable.
      - name: Phase 5a — per-file sign nested sidecar-bundle in .app
        env:
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
        shell: bash
        run: |
          set -euo pipefail
          BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle"
          APP_PATH=$(find "$BUNDLE_DIR/macos" -maxdepth 1 -name "*.app" -type d | head -1)
          [[ -n "$APP_PATH" ]] || { echo "::error::No .app found under $BUNDLE_DIR/macos"; exit 1; }

          SIDECAR_ROOT="$APP_PATH/Contents/Resources/sidecar-bundle"
          [[ -d "$SIDECAR_ROOT" ]] \
            || { echo "::error::sidecar-bundle not in .app at $SIDECAR_ROOT"; exit 1; }

          count=0
          failed=0
          while IFS= read -r f; do
            codesign_err="$(codesign --force \
              --sign "$APPLE_SIGNING_IDENTITY" \
              --options runtime \
              --timestamp \
              "$f" 2>&1)" && {
              count=$((count + 1))
            } || {
              failed=$((failed + 1))
              echo "::error::codesign FAILED in .app: $f"
              echo "  stderr: $codesign_err"
              xattr -l "$f" 2>/dev/null | head -3 || true
            }
          done < <(find "$SIDECAR_ROOT" -type f \( -name "*.so" -o -name "*.dylib" \) \
                   | awk '{print length($0), $0}' | sort -rn | cut -d' ' -f2-)

          if [[ $failed -gt 0 ]]; then
            echo "::error::Phase 5a: $failed nested file(s) failed codesign. Signed OK: $count"
            exit 1
          fi
          echo "Per-file signed $count nested .so/.dylib files."

          # Sign the PyInstaller bootloader (no extension — found separately)
          codesign --force \
            --sign "$APPLE_SIGNING_IDENTITY" \
            --options runtime \
            --timestamp \
            "$SIDECAR_ROOT/jura-sidecar"
          echo "Signed sidecar bootloader."

          # Re-seal .app top level so nested hashes are embedded in the outer seal.
          codesign --force \
            --sign "$APPLE_SIGNING_IDENTITY" \
            --options runtime \
            --timestamp \
            "$APP_PATH"
          echo "Re-sealed .app top level."

          # Deep-verify the sealed .app
          codesign --verify --deep --strict --verbose=2 "$APP_PATH" 2>&1 | tail -5

      # ── Phase 5b: spot-check TeamIdentifier on nested files ─────────────────
      - name: Phase 5b — verify nested signing TeamIdentifier
        shell: bash
        run: |
          set -euo pipefail
          BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle"
          APP_PATH=$(find "$BUNDLE_DIR/macos" -maxdepth 1 -name "*.app" -type d | head -1)
          SIDECAR_ROOT="$APP_PATH/Contents/Resources/sidecar-bundle"
          TEAM_ID="Y82C4P9L7F"

          failed=0
          checked=0
          while IFS= read -r f; do
            checked=$((checked + 1))
            ident=$(codesign --display --verbose=2 "$f" 2>&1 \
                    | awk -F= '/^TeamIdentifier=/{print $2}' | head -1)
            if [[ "$ident" != "$TEAM_ID" ]]; then
              echo "::error::MIS-SIGNED: $f  Team='${ident:-NONE}' (expected $TEAM_ID)"
              failed=$((failed + 1))
            else
              echo "OK: $(basename "$f")  Team=$ident"
            fi
          done < <(find "$SIDECAR_ROOT" -type f \( -name "*.dylib" -o -name "*.so" \) | head -5)

          boot_ident=$(codesign --display --verbose=2 "$SIDECAR_ROOT/jura-sidecar" 2>&1 \
                       | awk -F= '/^TeamIdentifier=/{print $2}' | head -1)
          if [[ "$boot_ident" != "$TEAM_ID" ]]; then
            echo "::error::Bootloader mis-signed: Team='${boot_ident:-NONE}'"
            failed=$((failed + 1))
          else
            echo "OK: bootloader Team=$boot_ident"
          fi

          if [[ $failed -gt 0 ]]; then
            echo "::error::$failed of $((checked + 1)) sampled files mis-signed."
            exit 1
          fi
          echo "Phase 5b PASSED."

      # ── Phase 6: regenerate DMG + updater from the correctly-signed .app ────
      # The .app produced by Phase 1 is now fully signed. Regenerate the DMG
      # and the updater payload (.app.tar.gz + .sig) from this sealed .app.
      # Without this step the DMG would be built from the pre-Phase-5a .app,
      # and the updater archive would reference a .app with unsigned nested files.
      - name: Phase 6 — regenerate DMG + updater payload (cargo tauri bundle --bundles dmg,updater)
        shell: bash
        env:
          TAURI_SIGNING_PRIVATE_KEY:          ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        run: |
          set -euo pipefail
          cd src-tauri
          cargo tauri bundle --bundles dmg,updater --target aarch64-apple-darwin
          echo "--- Phase 6 bundle output ---"
          ls -la target/aarch64-apple-darwin/release/bundle/dmg/ || true
          ls -la target/aarch64-apple-darwin/release/bundle/macos/ || true

      # ── Phase 6.5: verify final .app nested signing (BEFORE notarisation) ───
      # Notarisation takes 5-10 minutes and rejects with an opaque error if any
      # nested binary is unsigned. This gate catches the rc.29 failure mode
      # at build time so the operator sees it immediately, not 10 minutes later.
      - name: Phase 6.5 — verify final .app nested signing (notarisation gate)
        shell: bash
        run: |
          set -euo pipefail
          BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle"
          APP_PATH=$(find "$BUNDLE_DIR/macos" -maxdepth 1 -name "*.app" -type d | head -1)
          SIDECAR_ROOT="$APP_PATH/Contents/Resources/sidecar-bundle"
          TEAM_ID="Y82C4P9L7F"

          mis_signed=0
          checked=0
          while IFS= read -r f; do
            checked=$((checked + 1))
            ident=$(codesign --display --verbose=2 "$f" 2>&1 \
                    | awk -F= '/^TeamIdentifier=/{print $2}' | head -1)
            if [[ "$ident" != "$TEAM_ID" ]]; then
              mis_signed=$((mis_signed + 1))
              echo "::error::POST-BUNDLE MIS-SIGNED: $f  Team='${ident:-NONE}'"
            fi
          done < <(
            find "$SIDECAR_ROOT" -type f \
              \( -name "*.so" -o -name "*.dylib" -o ! -name "*.*" \) \
              -exec sh -c 'file -b "$1" 2>/dev/null | grep -q "Mach-O" && echo "$1"' _ {} \; 2>/dev/null
          )

          if [[ $mis_signed -gt 0 ]]; then
            echo "::error::$mis_signed of $checked nested Mach-O binaries mis-signed in final .app."
            echo "  This is the rc.29-class failure. Notarisation would reject with 'binary is not"
            echo "  signed with a valid Developer ID certificate'. Phase 0.5 or 5a is not effective."
            exit 1
          fi
          echo "Phase 6.5 PASSED: all $checked nested Mach-O binaries signed by Team=$TEAM_ID."

      # ── Phase 7: codesign the DMG ────────────────────────────────────────────
      - name: Phase 7 — codesign DMG
        env:
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
        shell: bash
        run: |
          set -euo pipefail
          BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle"
          DMG=$(find "$BUNDLE_DIR/dmg" -maxdepth 1 -name "*.dmg" | head -1)
          [[ -n "$DMG" ]] || { echo "::error::No DMG found after Phase 6"; exit 1; }
          codesign --force --sign "$APPLE_SIGNING_IDENTITY" --timestamp "$DMG"
          codesign --verify --verbose=2 "$DMG" 2>&1 | tail -3
          echo "DMG_PATH=$DMG" >> "$GITHUB_ENV"

      # ── Phase 8: notarise + staple ───────────────────────────────────────────
      # Preferred: App Store Connect API key (APPLE_API_KEY_ID / APPLE_API_ISSUER /
      # APPLE_API_KEY). More reliable in CI — no 2FA dependency.
      # Fallback: Apple ID + app-specific password (APPLE_ID / APPLE_PASSWORD /
      # APPLE_TEAM_ID). Used when the API key secrets are not set.
      #
      # Both paths call `notarytool submit --wait` and `stapler staple` on both
      # the DMG and the .app (so the .app inside the DMG AND the .app.tar.gz
      # updater archive carry the notarisation ticket).
      #
      # Hard gate: spctl --assess must report "source=Notarized Developer ID".
      # If it reports only "source=Developer ID" the build fails — a non-
      # notarised DMG will trigger a Gatekeeper warning on every Mac that
      # isn't the building machine, which is not acceptable for a release.
      - name: Phase 8 — notarise + staple (App Store Connect API key preferred)
        shell: bash
        env:
          # App Store Connect API key (preferred)
          APPLE_API_KEY_ID:  ${{ secrets.APPLE_API_KEY_ID }}
          APPLE_API_ISSUER:  ${{ secrets.APPLE_API_ISSUER }}
          APPLE_API_KEY:     ${{ secrets.APPLE_API_KEY }}
          # Apple ID fallback
          APPLE_ID:       ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID:  ${{ secrets.APPLE_TEAM_ID }}
        run: |
          set -euo pipefail
          BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle"
          DMG=$(find "$BUNDLE_DIR/dmg" -maxdepth 1 -name "*.dmg" | head -1)
          APP_PATH=$(find "$BUNDLE_DIR/macos" -maxdepth 1 -name "*.app" -type d | head -1)

          # Determine authentication method
          if [[ -n "${APPLE_API_KEY_ID:-}" && -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY:-}" ]]; then
            echo "Notarising via App Store Connect API key (preferred method)."

            # Write the .p8 key to a temp file (notarytool needs a path)
            API_KEY_PATH="$RUNNER_TEMP/notary-api.p8"
            echo "$APPLE_API_KEY" | base64 --decode > "$API_KEY_PATH"
            chmod 600 "$API_KEY_PATH"

            xcrun notarytool submit "$DMG" \
              --key "$API_KEY_PATH" \
              --key-id "$APPLE_API_KEY_ID" \
              --issuer "$APPLE_API_ISSUER" \
              --wait
            rm -f "$API_KEY_PATH"

          elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
            echo "Notarising via Apple ID + app-specific password (fallback method)."
            xcrun notarytool submit "$DMG" \
              --apple-id "$APPLE_ID" \
              --password "$APPLE_PASSWORD" \
              --team-id "$APPLE_TEAM_ID" \
              --wait

          else
            echo "::error::No notarisation credentials found."
            echo "  Set either: APPLE_API_KEY_ID + APPLE_API_ISSUER + APPLE_API_KEY"
            echo "  or:         APPLE_ID + APPLE_PASSWORD + APPLE_TEAM_ID"
            exit 1
          fi

          echo "Notarisation accepted. Stapling..."
          xcrun stapler staple "$DMG"
          xcrun stapler staple "$APP_PATH"
          xcrun stapler validate "$DMG"
          xcrun stapler validate "$APP_PATH"

          # Hard Gatekeeper gate — fail the build if the DMG is not notarised.
          spctl_out=$(spctl --assess --verbose=2 --type install "$DMG" 2>&1)
          echo "$spctl_out"
          if echo "$spctl_out" | grep -q "source=Notarized Developer ID"; then
            echo "Gatekeeper gate PASSED: source=Notarized Developer ID"
          else
            echo "::error::Gatekeeper gate FAILED: DMG is signed but NOT notarised."
            echo "  spctl output: $spctl_out"
            echo "  A non-notarised DMG will show a Gatekeeper warning on every other Mac."
            echo "  This is a release-blocker. Check notarytool logs above."
            exit 1
          fi

      # ── Verify updater signature ─────────────────────────────────────────────
      - name: Verify updater signature (.app.tar.gz.sig)
        shell: bash
        run: |
          BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle"
          while IFS= read -r payload; do
            sig="${payload}.sig"
            if [[ ! -f "$sig" ]]; then
              echo "::error::Missing .sig for ${payload}"
              exit 1
            elif [[ ! -s "$sig" ]]; then
              echo "::error::Empty .sig for ${payload} (zero bytes) — check TAURI_SIGNING_PRIVATE_KEY secret"
              exit 1
            fi
            echo "OK: $(basename "$sig") ($(wc -c < "$sig") bytes)"
          done < <(find "$BUNDLE_DIR" -name "*.app.tar.gz" -maxdepth 4 2>/dev/null)

      # ── Friendly-rename DMG and upload all artefacts ────────────────────────
      # Follows the same pattern as the existing macOS upload step in
      # release.yml: DMG gets a friendly JuraTrace-<ver>-macOS-AppleSilicon.dmg
      # name; updater archives (.app.tar.gz + .app.tar.gz.sig) are uploaded
      # RAW so the manifest step can match them by their Tauri-produced suffix.
      - name: Upload macOS artefacts to jura-trace
        shell: bash
        env:
          GITHUB_TOKEN: ${{ secrets.RELEASE_PAT }}
        run: |
          set -euo pipefail
          tag="${{ github.event.inputs.tag || github.ref_name }}"
          ver="${tag#v}"
          BUNDLE_DIR="src-tauri/target/aarch64-apple-darwin/release/bundle"

          find "$BUNDLE_DIR" \
            \( -name "*.dmg" \
               -o -name "*.app.tar.gz" \
               -o -name "*.app.tar.gz.sig" \
            \) \
            -maxdepth 4 2>/dev/null | sort | while IFS= read -r f; do
            fname="$(basename "$f")"
            case "$fname" in
              *.app.tar.gz|*.app.tar.gz.sig)
                echo "Uploading (raw) $fname ..."
                gh release upload "$tag" "$f" --repo Jura-Labs/jura-trace --clobber
                ;;
              *.dmg)
                friendly="JuraTrace-${ver}-macOS-AppleSilicon.dmg"
                tmp_dir="$(mktemp -d)"
                cp "$f" "${tmp_dir}/${friendly}"
                echo "Uploading (friendly) ${friendly} ..."
                gh release upload "$tag" "${tmp_dir}/${friendly}" \
                  --repo Jura-Labs/jura-trace --clobber
                rm -rf "$tmp_dir"
                ;;
            esac
          done

          found=$(find "$BUNDLE_DIR" -name "*.dmg" -maxdepth 4 2>/dev/null | head -1)
          [[ -n "$found" ]] || { echo "::error::No DMG found after upload loop"; exit 1; }
          echo "macOS artefacts uploaded."

      # ── Clean up temporary keychain ──────────────────────────────────────────
      # Always runs, even if previous steps failed, to avoid leaving cert
      # material in the runner's keychain between runs.
      - name: Cleanup — delete temporary keychain
        if: always()
        shell: bash
        run: |
          security lock-keychain "${KEYCHAIN_PATH}" 2>/dev/null || true
          security delete-keychain "${KEYCHAIN_PATH}" 2>/dev/null || true
          echo "Temporary keychain deleted."
```

---

## Secrets Inventory

All of the following must be set in `Jura-Labs/jura-archive` → Settings → Secrets → Actions.

| Secret | Required? | Description |
|--------|-----------|-------------|
| `APPLE_CERTIFICATE` | Yes | base64-encoded Developer ID Application .p12 certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Yes | Passphrase for the .p12 |
| `APPLE_SIGNING_IDENTITY` | Yes | `"Developer ID Application: Jura Labs CIC (Y82C4P9L7F)"` |
| `RELEASE_PAT` | Yes | Already in place — GitHub PAT with `repo` scope on `Jura-Labs/jura-trace` |
| `TAURI_SIGNING_PRIVATE_KEY` | Yes | Already in place — minisign private key (`~/.tauri/jura-trace-v10.key`) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Yes | Already in place — empty string for the 2026-05-22 keypair |
| `APPLE_API_KEY_ID` | Preferred | App Store Connect API key ID (10-char string) |
| `APPLE_API_ISSUER` | Preferred | App Store Connect issuer UUID |
| `APPLE_API_KEY` | Preferred | base64-encoded `.p8` key content (generated at App Store Connect → Integrations) |
| `APPLE_ID` | Fallback | Apple ID email for notarytool fallback path |
| `APPLE_PASSWORD` | Fallback | App-specific password (not the Apple ID password) |
| `APPLE_TEAM_ID` | Fallback | `"Y82C4P9L7F"` |

To encode the certificate: `base64 -i DeveloperIDApplication.p12 | pbcopy`

To encode the .p8 API key: `base64 -i AuthKey_XXXXXXXX.p8 | pbcopy`

### Security note on signing-cert-in-CI

Storing a Developer ID .p12 in GitHub Secrets is the industry-standard approach and is used by Tauri's own documentation and thousands of macOS CI pipelines. The cert is base64-encoded (not encrypted with a key other than the passphrase). The mitigations in this job are:

1. The cert is imported into a **temporary keychain** created fresh each run, not the login keychain, and deleted unconditionally in the `if: always()` cleanup step.
2. `security set-key-partition-list` restricts access to `codesign` only — no other process can use the private key.
3. The `RUNNER_TEMP` directory is used for all ephemeral files; it is scoped to the runner workspace and is cleaned after the job ends.
4. The job runs with `permissions: contents: write` (inherited from the workflow); no additional permissions are requested.

Risk surface: if the GitHub-hosted runner is compromised at the OS level (outside the GitHub Actions sandbox), the cert could be extracted during the run. This risk is present in any cloud macOS signing flow and is accepted by Apple and the macOS CI community. For a higher-security posture use a self-hosted Mac Mini with full-disk encryption and physical access controls (see PORTABILITY note below).

---

## Integration with latest.json

The macOS job uploads `.app.tar.gz` and `.app.tar.gz.sig` with their raw Tauri-produced names unchanged. The existing `publish-release` job's `Generate and upload update manifest` step already handles `darwin-aarch64` via:

```js
'darwin-aarch64': {
  urlSuffix: '.app.tar.gz',
  sigSuffix: '.app.tar.gz.sig',
},
```

No changes to the manifest step are needed. The macOS artefacts will flow into `latest.json` automatically once this job runs and uploads them before `publish-release` runs. The `publish-release` job already has `if: ${{ always() && ... }}` to tolerate a partial build matrix, so adding this job to the matrix does not require gating changes.

---

## PORTABILITY: Self-Hosted Mac Mini M4

The hosting migration plan targets a Mac Mini M4 as a self-hosted runner to eliminate the ~10x GitHub-hosted macOS minutes cost. Replace:

```yaml
runs-on: macos-latest
```

with:

```yaml
runs-on: ["self-hosted", "macOS", "ARM64", "m4"]
```

**Non-ephemeral runner considerations** — unlike a GitHub-hosted runner, the Mac Mini persists state between runs:

1. **Keychain cleanup**: the `if: always()` keychain cleanup step handles this, but verify that a build killed mid-step (e.g. by a timeout) does not leave `jura-signing.keychain-db` in the user's keychain search list. Add a post-run hook to the runner that runs `security delete-keychain "$HOME/Library/Keychains/jura-signing.keychain-db"` unconditionally.

2. **hdiutil mounts**: the pre-flight cleanup step detaches stray mounts. On a persistent runner this is load-bearing, not defensive.

3. **sidecar-bundle accumulation**: `src-tauri/sidecar-bundle/` and `src-tauri/target/` grow across runs. Add `rm -rf src-tauri/sidecar-bundle src-tauri/target/aarch64-apple-darwin/release/bundle` to the pre-flight step or configure a periodic `cargo clean` cron job on the runner host.

4. **TCC (Transparency, Consent and Control)**: macOS Sequoia tightened TCC for launchd-spawned processes (the runner agent). The existing release.yml already documents this at the `Configure self-hosted runner paths` step. The CARGO_HOME isolation pattern and RUSTC_WRAPPER unset from that step apply equally here.

5. **Tool cache path**: the existing `Configure self-hosted runner paths` step in release.yml (which would still run for the matrix) sets `RUNNER_TOOL_CACHE` and `AGENT_TOOLSDIRECTORY` to `$HOME/.runner-tool-cache`. Copy that step here if this becomes a standalone job.

---

## Cost Comparison

| Runner | Cost | Notes |
|--------|------|-------|
| GitHub-hosted `macos-latest` | ~$0.24/min (10x Linux multiplier) | Cold build ~45-60 min = ~$11-14 per release. Cloud-hosted macOS bills at Apple Silicon rates as of 2026. |
| Self-hosted Mac Mini M4 | Electricity only (~0.5p/min) | ~$0.01-0.02 per build once hardware is amortised. JTV-145 plan: Mac Mini M4, 02:00-08:00 Copenhagen window. |
| Savings | ~$11-14 per release | At ~1 release/week = ~$600/year savings; hardware cost ~£700 breaks even in ~18 months. |

For the pre-launch phase (1-2 releases/month) the cost difference is modest (~$25-30/month). The self-hosted path becomes clearly preferable when release cadence increases after v1.0 launch.

---

## Open Questions / Decisions Needed

1. **App Store Connect API key**: does one exist for `Y82C4P9L7F`? If not, create one at App Store Connect → Integrations → API Keys (Developer role sufficient for notarisation). This is a one-time setup that makes notarisation significantly more reliable in CI than the Apple-ID path.

2. **Model files in CI**: the build degrades gracefully without models but ships a broken deepfake-detection experience. Options: (a) pre-download to a private GitHub artifact on a schedule, (b) store on the Mac Mini for self-hosted only, (c) accept degraded CI builds and rely on the local build for production. Decision needed before this job ships.

3. **Runner choice**: GitHub-hosted vs self-hosted Mac Mini M4 for v1.0.1. If GitHub-hosted, cost per release is ~$11-14 and the job is ready to run immediately. If self-hosted, TCC and CARGO_HOME isolation patterns from the existing workflow need to be ported into this standalone job.

4. **`build-local-mac.sh` smoke test in CI**: the local script runs `scripts/smoke_test_frozen_sidecar.py` against the frozen sidecar binary before staging it. This job does not include that step because the script is not confirmed available in CI. If it is, add it after the `Validate sidecar --onedir output` step.

5. **Matrix vs standalone job**: this draft is written as a standalone job for clarity. Before merging, decide whether to fold it into the existing matrix (replacing the `aarch64-apple-darwin` path) or keep it as a parallel standalone job. The matrix approach shares the `create-release` dependency cleanly; the standalone approach is easier to iterate on without risking the Windows/Linux paths.

6. **Publish gate**: the existing `publish-release` job only auto-publishes when `needs.build.result == 'success'` (all matrix jobs succeeded). If this macOS job is added to the matrix, a macOS failure will leave the release as a draft. That is the current behaviour (intentional — manual publish after macOS upload). No change needed unless full-automation is desired.

---

## Ticket Summary (Plane / JTV)

Post-v1.0 macOS CI release job draft. Fixes the root cause of CI macOS abandonment: the existing Phase 5 codesign loop silences stderr (`>/dev/null 2>&1`), hiding which of the ~610 nested PyInstaller dylibs failed and surfacing only "sort: stdout: Broken pipe". The draft captures codesign stderr, prints the failing file, and includes the rc.29-era Phase 0.5 source-signing fix (sign SOURCE sidecar-bundle before `cargo tauri bundle` copies from it). Adds a pre-notarisation gate (Phase 6.5) that catches unsigned nested binaries before the 5-10 minute notarytool round-trip. Prefers App Store Connect API key auth over Apple-ID for more reliable CI notarisation. Follows the existing two-repo upload pattern (assets to Jura-Labs/jura-trace via RELEASE_PAT) and the existing latest.json manifest pipeline unchanged. Includes a self-hosted Mac Mini M4 portability guide and secrets inventory. File: `docs/decisions/macos-ci-release-draft.md`. Review target: v1.0.1 sprint planning.
