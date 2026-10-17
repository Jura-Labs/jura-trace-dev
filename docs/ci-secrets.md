# CI/CD Secrets Configuration

Secrets required in GitHub repository Settings → Secrets and variables → Actions.

## Currently configured

| Secret | Used by | Status |
|---|---|---|
| `GITHUB_TOKEN` | All workflows | Auto-provided by GitHub |

## Annual costs

| Service | Cost | Purpose |
|---|---|---|
| Apple Developer ID (Organisation) | $99/yr (~£79) | macOS code signing + notarisation |
| Azure Trusted Signing (Basic) | $9.99/mo (~£95/yr) | Windows code signing (immediate SmartScreen trust) |
| **Total** | **~£174/yr** | |

---

## macOS — Apple Developer ID

**Status:** Application submitted (28 March 2026). Awaiting verification (2-4 weeks).

### Secrets required

| Secret | Value | How to obtain |
|---|---|---|
| `APPLE_CERTIFICATE` | Base64-encoded .p12 certificate | Export from Keychain Access after Developer ID enrolment |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the .p12 file | Set during export |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Jura Labs CIC (TEAMID)` | From Keychain after cert install |
| `APPLE_ID` | Apple ID email used for notarisation | Your Apple Developer account email |
| `APPLE_PASSWORD` | App-specific password | Generate at appleid.apple.com → Security → App-Specific Passwords |
| `APPLE_TEAM_ID` | 10-character team identifier | From developer.apple.com → Membership |

### Setup steps (when certificate arrives)

1. Sign in to developer.apple.com → Certificates, Identifiers & Profiles
2. Create a "Developer ID Application" certificate
3. Download and install the certificate in Keychain Access
4. Export as .p12: Keychain Access → My Certificates → right-click → Export
5. Base64 encode: `base64 -i certificate.p12 | pbcopy`
6. Add all 6 secrets to GitHub repository settings
7. Uncomment the Apple signing env vars in `.github/workflows/release.yml`

### Generate app-specific password for notarisation

1. Go to appleid.apple.com → Sign In & Security → App-Specific Passwords
2. Click "Generate" → name it "Jura Trace CI"
3. Copy the generated password → add as `APPLE_PASSWORD` secret

---

## Windows — Azure Trusted Signing

**Why Azure Trusted Signing?**
- $9.99/month (~£95/yr) — cheapest cloud-based option
- **Immediate SmartScreen trust** — Microsoft's own service, no reputation building
- Fully automated CI/CD — native GitHub Actions integration
- No hardware tokens, no phone approvals, no certificate reissues
- 5,000 signatures/month on Basic tier (more than enough)

### Setup steps

#### 1. Create Azure account and subscription

1. Go to [portal.azure.com](https://portal.azure.com)
2. Sign up with your Microsoft account (or create one)
3. Create a Pay-As-You-Go subscription

#### 2. Create a Trusted Signing account

```bash
# Install Azure CLI (if not installed)
brew install azure-cli   # macOS
# Or: winget install Microsoft.AzureCLI   # Windows

# Sign in
az login

# Create resource group
az group create --name jura-signing --location westeurope

# Create Trusted Signing account
az resource create \
  --resource-group jura-signing \
  --resource-type Microsoft.CodeSigning/codeSigningAccounts \
  --name juralabs-signing \
  --location westeurope \
  --properties '{}'
```

#### 3. Create identity validation

1. In Azure Portal → Trusted Signing → your account → Identity validation
2. Select "Public" identity validation type
3. Enter organisation details:
   - Organisation name: Jura Labs CIC
   - Street address, city, country
   - Companies House number
   - Website: juralabs.org
4. Submit for verification (1-3 business days)

#### 4. Create a certificate profile

Once identity validation is approved:
1. Trusted Signing → Certificate profiles → Create
2. Profile type: "Public Trust"
3. Name: `juralabs-public-trust`
4. CN value will be set automatically from your validated identity

#### 5. Create an Azure AD app registration for CI

```bash
# Create service principal for GitHub Actions
az ad sp create-for-rbac \
  --name "jura-trace-signing-ci" \
  --role "Trusted Signing Certificate Profile Signer" \
  --scopes "/subscriptions/<SUB_ID>/resourceGroups/jura-signing/providers/Microsoft.CodeSigning/codeSigningAccounts/juralabs-signing"
```

This outputs:
```json
{
  "appId": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "password": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
  "tenant": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
}
```

#### 6. Add GitHub Secrets

| Secret | Value | From |
|---|---|---|
| `AZURE_CLIENT_ID` | `appId` from step 5 | Service principal output |
| `AZURE_CLIENT_SECRET` | `password` from step 5 | Service principal output |
| `AZURE_TENANT_ID` | `tenant` from step 5 | Service principal output |
| `AZURE_SIGNING_ACCOUNT` | `juralabs-signing` | The name you chose in step 2 |
| `AZURE_CERT_PROFILE` | `juralabs-public-trust` | The name you chose in step 4 |
| `AZURE_SIGNING_ENDPOINT` | `https://weu.codesigning.azure.net` | Based on your region. **The secret is named `AZURE_SIGNING_ENDPOINT`, not `AZURE_ENDPOINT`** — `release.yml:741` reads the former, and a secret under the wrong name is invisible to the workflow rather than an error. |

#### 6b. Rotating the client secret, or wiring up a second repository

The client secret cannot be read back from Azure any more than from GitHub.
To give another repository access, or to replace an expiring secret, mint an
additional one against the **existing** service principal rather than making
a new principal — the existing one already holds the *Trusted Signing
Certificate Profile Signer* role, which is the part that is tedious to
recreate.

```bash
az ad sp list --display-name jura-trace-signing-ci --query "[].appId" -o tsv
az account show --query tenantId -o tsv
az ad app credential reset --id <appId> --append --end-date 2028-09-08
```

`--append` is load-bearing. **Without it, `credential reset` deletes every
existing credential on the app**, which breaks signing in any repository
still using the old secret. There is no `--years` flag; expiry is set with
`--end-date`.

Leave the previous secret in place until the new repository has completed a
signed release, then delete the old entry under Certificates & secrets.

#### 7. Update release workflow

Add this step in `.github/workflows/release.yml` for the Windows build, after the Tauri build step:

```yaml
# Windows code signing via Azure Trusted Signing
- name: Sign Windows installer
  if: matrix.target == 'x86_64-pc-windows-msvc'
  uses: azure/trusted-signing-action@v0.4
  with:
    azure-tenant-id: ${{ secrets.AZURE_TENANT_ID }}
    azure-client-id: ${{ secrets.AZURE_CLIENT_ID }}
    azure-client-secret: ${{ secrets.AZURE_CLIENT_SECRET }}
    endpoint: ${{ secrets.AZURE_SIGNING_ENDPOINT }}
    trusted-signing-account-name: ${{ secrets.AZURE_SIGNING_ACCOUNT }}
    certificate-profile-name: ${{ secrets.AZURE_CERT_PROFILE }}
    files-folder: src-tauri/target/x86_64-pc-windows-msvc/release/bundle/
    files-folder-filter: exe,msi
    file-digest: SHA256
    timestamp-rfc3161: http://timestamp.acs.microsoft.com
    timestamp-digest: SHA256
```

---

## Auto-Updater Signing

The Tauri auto-updater uses a separate minisign key pair for verifying update integrity.

### Setup

```bash
# Generate key pair
cargo tauri signer generate

# This produces:
#   Private key: store as TAURI_SIGNING_PRIVATE_KEY secret
#   Public key: set in tauri.conf.json → plugins.updater.pubkey
```

| Secret | Value | How to obtain |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | Base64-encoded minisign private key | `cargo tauri signer generate` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the key | Set during generation |

Add the **public key** to `src-tauri/tauri.conf.json`:
```json
"plugins": {
  "updater": {
    "pubkey": "YOUR_PUBLIC_KEY_HERE"
  }
}
```

---

## Complete setup checklist

When both certificates arrive:

- [ ] **Apple**: Download Developer ID cert → export .p12 → base64 → add 6 secrets
- [ ] **Azure**: Create Trusted Signing account → validate identity → create cert profile → create service principal → add 7 secrets
- [ ] **Updater**: Run `cargo tauri signer generate` → add 2 secrets → set pubkey in tauri.conf.json
- [ ] **Workflow**: Uncomment Apple signing env vars + add Azure signing step in release.yml
- [ ] **Test**: Tag a test release (e.g., `v1.0.0-beta.1`) and verify both platforms sign correctly
- [ ] **Verify**: macOS — no Gatekeeper warning. Windows — no SmartScreen warning.

---

## Summary of all secrets

| Secret | Service | Count |
|---|---|---|
| `APPLE_CERTIFICATE` | macOS signing | |
| `APPLE_CERTIFICATE_PASSWORD` | macOS signing | |
| `APPLE_SIGNING_IDENTITY` | macOS signing | |
| `APPLE_ID` | macOS notarisation | |
| `APPLE_PASSWORD` | macOS notarisation | |
| `APPLE_TEAM_ID` | macOS notarisation | |
| `AZURE_CLIENT_ID` | Windows signing | |
| `AZURE_CLIENT_SECRET` | Windows signing | |
| `AZURE_TENANT_ID` | Windows signing | |
| `AZURE_SIGNING_ACCOUNT` | Windows signing | |
| `AZURE_CERT_PROFILE` | Windows signing | |
| `AZURE_SIGNING_ENDPOINT` | Windows signing | |
| `TAURI_SIGNING_PRIVATE_KEY` | Auto-updater | |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Auto-updater | |
| **Total** | | **15 secrets** |

*Last updated: 28 March 2026*
