# CI/CD Secrets Configuration

Secrets required in GitHub repository Settings → Secrets and variables → Actions.

## Currently configured

| Secret | Used by | Status |
|---|---|---|
| `GITHUB_TOKEN` | All workflows | Auto-provided by GitHub |

## Required before signed builds (Sprint 20)

### Apple Code Signing

| Secret | Value | How to obtain |
|---|---|---|
| `APPLE_CERTIFICATE` | Base64-encoded .p12 certificate | Export from Keychain Access after Developer ID enrolment |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the .p12 file | Set during export |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Juralabs CIC (TEAMID)` | From Keychain after cert install |
| `APPLE_ID` | Apple ID email used for notarisation | Your Apple Developer account email |
| `APPLE_PASSWORD` | App-specific password | Generate at appleid.apple.com → Security → App-Specific Passwords |
| `APPLE_TEAM_ID` | 10-character team identifier | From developer.apple.com → Membership |

### Windows Code Signing

| Secret | Value | How to obtain |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Base64-encoded signing key | Run `cargo tauri signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the key | Set during generation |

### Auto-Updater

| Secret | Value | How to obtain |
|---|---|---|
| (uses TAURI_SIGNING_PRIVATE_KEY above) | Same key signs update bundles | Generated once, used for both |

## Setup steps

1. Obtain Apple Developer ID certificate (CIC org account applied)
2. Export the certificate as .p12 from Keychain Access
3. Base64-encode: `base64 -i certificate.p12 | pbcopy`
4. Add all 6 Apple secrets to GitHub
5. Run `cargo tauri signer generate` for the Tauri update key pair
6. Add the private key as `TAURI_SIGNING_PRIVATE_KEY`
7. Set the public key in `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
8. Uncomment the signing env vars in `.github/workflows/release.yml` (lines 270-280)
9. Tag a new release to test signed builds
