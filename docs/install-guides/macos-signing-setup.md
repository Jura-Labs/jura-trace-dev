# macOS Code Signing & Notarisation Setup

**Status**: Apple Developer Program purchased 2026-04-14. Developer ID
Application certificate issued and installed in login keychain.

## Identity

| Field | Value |
|-------|-------|
| Certificate type | Developer ID Application |
| Sub-CA | G2 (Xcode 11.4.1 or later) |
| Common Name | `Developer ID Application: Jura Labs CIC (Y82C4P9L7F)` |
| Team ID | `Y82C4P9L7F` |
| Organisation | `Jura Labs CIC` (note the space — must match Apple registration) |
| Issued | 14 April 2026 |
| Expires | 15 April 2031 |
| Key size | RSA 2048 |
| Hardened runtime | Required (configured in `tauri.conf.json`) |

## Prerequisites verified

```bash
# Should list at least 1 valid identity
security find-identity -v -p codesigning
# Expected output (excerpt):
#   1) <hash> "Developer ID Application: Jura Labs CIC (Y82C4P9L7F)"
#      1 valid identities found
```

If "0 valid identities found" — install the Apple intermediate certs from
https://www.apple.com/certificateauthority/:
- Developer ID - G2 (Expiring 09/17/2031)
- Apple Root CA - G2

## Tauri configuration

Already wired in `src-tauri/tauri.conf.json`:

```json
"macOS": {
  "entitlements": "Entitlements.plist",
  "minimumSystemVersion": "13.0",
  "signingIdentity": "Developer ID Application: Jura Labs CIC (Y82C4P9L7F)",
  "providerShortName": "Y82C4P9L7F",
  "hardenedRuntime": true
}
```

Local signed builds work out of the box — `cargo tauri build` will
automatically pick up the keychain identity.

## Notarisation setup (one-time)

Notarisation is required for distribution outside the App Store. Without it,
macOS Gatekeeper still warns end users even with a Developer ID-signed app.

### 1. Generate an app-specific password

1. Sign in at https://appleid.apple.com
2. Sign-In and Security → App-Specific Passwords → "+"
3. Label: `Jura Trace notarytool`
4. Save the generated password securely

### 2. Store credentials in the keychain

Replace `your-apple-id@email.com` with the Apple ID used for the Developer
Program enrolment, and paste the app-specific password when prompted:

```bash
xcrun notarytool store-credentials "jura-trace-notary" \
  --apple-id "your-apple-id@email.com" \
  --team-id "Y82C4P9L7F"
# Prompts for the app-specific password
```

This stores the credentials in the login keychain under the profile name
`jura-trace-notary` for use by `notarytool` invocations below.

## Local signed + notarised build

```bash
# Build the signed app + DMG
cd src-tauri && cargo tauri build

# Verify the signature
codesign --verify --deep --strict --verbose=2 \
  "target/release/bundle/macos/Jura Trace.app"

# Should print "accepted source=Developer ID"
spctl -a -vvv -t install \
  "target/release/bundle/macos/Jura Trace.app"

# Submit DMG for notarisation (replace version/arch as needed)
xcrun notarytool submit \
  "target/release/bundle/dmg/Jura Trace_0.9.0_aarch64.dmg" \
  --keychain-profile "jura-trace-notary" \
  --wait

# When status returns "Accepted" — staple the notarisation ticket
xcrun stapler staple \
  "target/release/bundle/dmg/Jura Trace_0.9.0_aarch64.dmg"

# Verify the staple
spctl -a -vvv -t open --context context:primary-signature \
  "target/release/bundle/dmg/Jura Trace_0.9.0_aarch64.dmg"
```

A stapled, notarised DMG installs without any Gatekeeper warning on macOS
13.0 and later, including air-gapped Macs (the staple bundles the
notarisation ticket into the file).

## CI integration (GitHub Actions)

Required secrets in the `juralabs/jura-archive` repository:

| Secret name | Contents |
|-------------|----------|
| `APPLE_CERTIFICATE_BASE64` | base64-encoded `.p12` export of the Developer ID Application cert + private key |
| `APPLE_CERTIFICATE_PASSWORD` | password used when exporting the `.p12` |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Jura Labs CIC (Y82C4P9L7F)` |
| `APPLE_ID` | Apple ID email used for the Developer Program |
| `APPLE_ID_PASSWORD` | the app-specific password from the notarisation setup above |
| `APPLE_TEAM_ID` | `Y82C4P9L7F` |

To export the `.p12` for upload to GitHub:

1. Keychain Access → Login → My Certificates
2. Right-click "Developer ID Application: Jura Labs CIC (Y82C4P9L7F)" → Export
3. Format: Personal Information Exchange (.p12)
4. Set a strong password (this becomes `APPLE_CERTIFICATE_PASSWORD`)
5. Convert to base64: `base64 -i developer_id.p12 | pbcopy`
6. Paste into the GitHub secret

The release workflow (`.github/workflows/release.yml`) imports these secrets
into a temporary keychain on the macOS runner, then runs `cargo tauri build`
which picks up the identity automatically. Notarisation runs after the
build via `xcrun notarytool submit ... --wait` followed by `stapler staple`.

(Workflow YAML to be added in a follow-up commit — for now, local signing is
sufficient for pilot distribution.)

## Pilot distribution checklist

For each pilot release:

- [ ] `cargo tauri build` produces signed `.app` and `.dmg`
- [ ] `codesign --verify --deep --strict` returns success
- [ ] `spctl -a -vvv -t install` returns "accepted source=Developer ID"
- [ ] `notarytool submit --wait` returns status "Accepted"
- [ ] `stapler staple` succeeds on the DMG
- [ ] DMG opens cleanly on a fresh Mac with no Gatekeeper warning
- [ ] SHA-256 checksum published alongside the DMG on the release page

## Troubleshooting

**"Developer ID Application … is not trusted"** — Apple intermediate cert
missing. Download from https://www.apple.com/certificateauthority/ and
double-click to install.

**"errSecInternalComponent" during `cargo tauri build`** — keychain locked
or signing identity not unlocked. Run `security unlock-keychain` first or
pass the password via env var `APPLE_KEYCHAIN_PASSWORD`.

**Notarisation fails with "The signature does not include a secure
timestamp"** — pass `--timestamp` to codesign (Tauri does this automatically
when `hardenedRuntime: true` is set).

**Notarisation fails with "The executable does not have the hardened
runtime enabled"** — confirm `hardenedRuntime: true` in `tauri.conf.json`
macOS section, then rebuild.

## Continuity (founder note)

The Developer ID cert and private key are tied to this Mac's keychain.
For business continuity, export the `.p12` to encrypted offsite storage
immediately after issuance so the cert can be reinstalled if the Mac is
lost. Apple does not allow re-issuing an identical Developer ID certificate
— a lost private key means signing under a new identity, which would
require all installed users to re-authorise the app at next update.

**Action**: export the `.p12` to a password-protected store (1Password,
encrypted USB drive, etc.) within 24 hours of cert issuance.
