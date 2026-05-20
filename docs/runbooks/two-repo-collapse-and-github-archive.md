# Two-repo collapse + GitHub mirror archive

**Tickets:** JTV-153 (two-repo collapse) + JTV-154 (GitHub mirror archive + read-only banner)
**Phase:** Pre-launch infrastructure migration
**Target:** Complete before rc.25 tag-cut (week of 15 June 2026)
**Author:** Generated 2026-05-20

## What this does

Today's release model uses a **two-repo split**:

- `Jura-Labs/jura-archive` (private, GitHub) holds source + CI workflows
- `juralabs/jura-trace` (public, GitHub) hosts only release installers

The agreed end-state per `project_hosting_migration_plan` memory is a
**single-repo Codeberg-primary model**:

- `codeberg.org/juralabs/jura-trace` is the canonical home for source +
  CI + release tags (AGPL public source, single repo)
- GitHub repos are set to **read-only mirrors with a banner pointing
  to Codeberg** for new development
- GitHub Releases continue to host the installer binaries (the Tauri
  auto-updater + the juralabs.org Cloudflare worker both read from
  there); this preserves the existing install link surface without
  forcing every contributor to create a Codeberg account

This runbook covers both halves of that migration. It is **destructive
to the existing GitHub setup** (the private `jura-archive` source repo
loses its active role) and should be executed in a single sitting with
verification at each step.

## Prerequisites

- Codeberg account with admin access to `codeberg.org/juralabs`
- GitHub admin access to both `Jura-Labs/jura-archive` and
  `juralabs/jura-trace`
- `git`, `git-filter-repo` (if history scrub needed), and
  `gh` CLI authenticated
- Pre-commit hook active locally (`.githooks/pre-commit` ran on
  every commit per memory; commits must stay clean)
- All open PRs on `Jura-Labs/jura-archive` merged or explicitly
  declared abandoned (no in-flight work to lose)
- Mac Mini M4 Forgejo runner registered + verified
  (see `infrastructure/forgejo-runner/SETUP.md`)
- Cloudflare Worker for the updater endpoint deployed
  (see `infrastructure/cloudflare-worker-updater/DEPLOYMENT.md`)
- juralabs.org `/downloads/` page deployed

## Phase 1: Codeberg as canonical home

### 1.1 Create the Codeberg organisation and repository

If the organisation does not yet exist:

```sh
# Create via the web UI: https://codeberg.org/orgs/new
# Name: juralabs
# Visibility: Public
# Description: Jura Labs CIC: open-source content verification tools
```

Create the source repository:

```sh
# Via the web UI: https://codeberg.org/repo/create
# Owner: juralabs
# Name: jura-trace
# Visibility: Public
# Initialise: NO (push from local instead)
# Licence: AGPL-3.0-or-later (matches the existing LICENSE file)
```

### 1.2 Push the full local repo to Codeberg

From a clean local clone of `Jura-Labs/jura-archive` (the current
source-of-truth):

```sh
cd ~/Downloads/ecoadvisor/juralabs

# Confirm clean working tree
git status

# Add Codeberg as a new remote alongside the existing GitHub remote
git remote add codeberg https://codeberg.org/juralabs/jura-trace.git
git remote -v

# Push main + all tags
git push codeberg main
git push codeberg --tags

# Optionally push other long-lived branches if they exist
git for-each-ref --format='%(refname:short)' refs/heads | while read b; do
  case "$b" in
    main) ;;  # already pushed
    *) git push codeberg "$b" ;;
  esac
done
```

Verify in the Codeberg web UI that `main` is the default branch and
all tags from `v0.9.0-rc.1` onward are present.

### 1.3 Configure Codeberg repository settings

Via Settings → Repository:

- **Default branch:** `main`
- **Description:** "Local-first content verification for cultural
  institutions and investigative communities. AGPL-3.0-or-later."
- **Website:** https://juralabs.org
- **Topics:** `content-verification`, `c2pa`, `provenance`, `forensics`,
  `tauri`, `svelte`, `rust`, `deepfake-detection`, `agpl`
- **Issues:** Enabled
- **Pull requests:** Enabled
- **Actions:** Enabled (required for the Forgejo runner)

Via Settings → Secrets and variables → Actions, mirror the secrets
from GitHub `Jura-Labs/jura-archive`:

| Secret | Used for |
|---|---|
| `RELEASE_PAT` | gh CLI uploads to `juralabs/jura-trace` GitHub Releases |
| `APPLE_CERTIFICATE` | macOS Developer ID signing |
| `APPLE_CERTIFICATE_PASSWORD` | macOS Developer ID signing |
| `APPLE_SIGNING_IDENTITY` | macOS Developer ID signing |
| `APPLE_ID` | macOS notarisation |
| `APPLE_PASSWORD` | macOS notarisation |
| `APPLE_TEAM_ID` | macOS notarisation |
| `AZURE_TENANT_ID` | Windows Azure Trusted Signing |
| `AZURE_CLIENT_ID` | Windows Azure Trusted Signing |
| `AZURE_CLIENT_SECRET` | Windows Azure Trusted Signing |
| `AZURE_SIGNING_ENDPOINT` | Windows Azure Trusted Signing |
| `AZURE_SIGNING_ACCOUNT` | Windows Azure Trusted Signing |
| `AZURE_CERT_PROFILE` | Windows Azure Trusted Signing |
| `TAURI_SIGNING_PRIVATE_KEY` | Updater payload minisign |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Updater payload minisign |

### 1.4 Confirm the Forgejo CI workflow exists

The `.forgejo/workflows/release.yml` file should already be in the
repo (it has been maintained alongside `.github/workflows/release.yml`
since 2026-05 per CHANGELOG). Verify both files are present:

```sh
ls .forgejo/workflows/
ls .github/workflows/
```

If `.forgejo/workflows/release.yml` is missing, copy the GitHub one as
a starting point and adapt for Forgejo's `gitea.token` instead of
`GITHUB_TOKEN`.

### 1.5 Trigger a test workflow from Codeberg

Push a non-tag commit to a feature branch on Codeberg:

```sh
git checkout -b test/codeberg-runner-pickup
echo "test: codeberg runner pickup $(date)" >> .test-codeberg
git add .test-codeberg
git commit -m "test: verify codeberg runner picks up workflows"
git push codeberg test/codeberg-runner-pickup
```

The Mac Mini M4 runner should pick up the workflow within seconds of
the 02:00 to 08:00 active window opening. Verify in the Codeberg
Actions UI that the runner picked up the job, ran successfully, and
the workflow's status reports green.

Once verified, delete the test branch + the `.test-codeberg` file.

## Phase 2: Two-repo collapse

The `juralabs/jura-trace` public GitHub repo currently exists solely
to host release installers. After Phase 1, the Codeberg repo holds
the source. The two-repo split on GitHub is no longer required.

Two paths forward:

### Option A: Keep `juralabs/jura-trace` as installer host (recommended)

The Tauri auto-updater + the juralabs.org Cloudflare worker both
reference `juralabs/jura-trace` GitHub Releases. The simplest path is:

- Keep the repo as-is but treat it as **installer-only** (no source,
  no PRs, no issues)
- Set the repository description to make the role explicit
- Disable issues + pull requests in repo settings
- Add a `README.md` that explains the repo's purpose and points
  contributors at Codeberg

This is the lowest-risk option and preserves every install URL
already in the wild. Recommended.

```sh
# Update the repo description via gh CLI
gh repo edit juralabs/jura-trace \
  --description "Installer binaries for Jura Trace. Source code lives at codeberg.org/juralabs/jura-trace." \
  --homepage "https://codeberg.org/juralabs/jura-trace"

# Disable issues + PRs (already enabled by default on most repos)
gh repo edit juralabs/jura-trace --enable-issues=false
gh repo edit juralabs/jura-trace --enable-projects=false
gh repo edit juralabs/jura-trace --enable-wiki=false
```

Replace the README at `juralabs/jura-trace` (push from a clean local
clone of `juralabs/jura-trace`, not the source repo):

```sh
mkdir -p /tmp/jura-trace-installer
cd /tmp/jura-trace-installer
git clone https://github.com/juralabs/jura-trace.git
cd jura-trace
```

Write a new README that names the repo's purpose. Template:

```markdown
# Jura Trace: installer binaries

This repository hosts release installers for Jura Trace (macOS DMG,
Windows MSI, Linux AppImage / DEB). It is the destination URL for the
auto-updater and the juralabs.org/downloads/ page.

**Source code lives at https://codeberg.org/juralabs/jura-trace.**
Open issues, send pull requests, and read the source there. This
repository accepts no pull requests and no issues. Both surfaces are
disabled.

## Download

The latest installers are always available at:

- macOS (Apple Silicon): https://github.com/juralabs/jura-trace/releases/latest/download/Jura.Trace_aarch64.dmg
- Windows: https://github.com/juralabs/jura-trace/releases/latest/download/Jura.Trace_x64_en-US.msi
- Linux AppImage: https://github.com/juralabs/jura-trace/releases/latest/download/jura-trace_amd64.AppImage

Or visit https://juralabs.org/downloads/ for the platform picker.

## Verify your download

Each release publishes SHA-256 sums and minisign signatures.
See https://codeberg.org/juralabs/jura-trace/src/branch/main/docs/install-guides/verify-downloads.md
for the verification commands.

## Licence

Jura Trace is licensed under the GNU Affero General Public License
v3.0 or later. Commercial licence enquiries: commercial@juralabs.org.
```

Commit + push:

```sh
git add README.md
git commit -m "docs: README points contributors at Codeberg; installer-only role"
git push origin main
```

### Option B: Delete `juralabs/jura-trace` (not recommended pre-launch)

Deleting the repo breaks every install link in the wild (the Tauri
auto-updater config in `tauri.conf.json:40`, the existing
`docs/install-guides/`, every CHANGELOG entry, every external
reference). Strongly discouraged before launch.

Defensible only if:

1. juralabs.org takes over as the canonical download host
2. All install URLs in shipping installers + docs are updated FIRST
3. A redirect from `github.com/juralabs/jura-trace/releases/...` is
   set up (GitHub does not support this; would need to delete the
   repo and accept the link breakage)

For v1.0 launch, **stick with Option A**.

## Phase 3: GitHub mirror archive + read-only banner

The `Jura-Labs/jura-archive` repo (private source) becomes a
**read-only archive of pre-Codeberg-migration history**. New
development happens on Codeberg.

### 3.1 Push final state from local to GitHub source

Before archiving, ensure GitHub `Jura-Labs/jura-archive` has every
commit that Codeberg `juralabs/jura-trace` has. They should already
be in sync if Phase 1 was done from a clean clone, but verify:

```sh
git fetch origin     # the GitHub source remote
git fetch codeberg

# Should report 0 commits ahead/behind
git rev-list --left-right --count origin/main...codeberg/main
```

If non-zero, push from local to whichever side is behind.

### 3.2 Add a read-only banner to the GitHub source README

Edit the GitHub source repo's README to add a prominent banner at
the top. From a clean clone of `Jura-Labs/jura-archive`:

```markdown
> # ⚠️ This repository is archived
>
> Active development of Jura Trace has moved to **Codeberg**:
> **https://codeberg.org/juralabs/jura-trace**
>
> This GitHub repository is a read-only mirror preserved for history.
> Pull requests and issues are not monitored here. To contribute,
> open an issue or pull request on the Codeberg repository.
>
> Installers are still published to https://github.com/juralabs/jura-trace
> (a separate installer-host repo) so the existing download URLs and
> auto-updater continue to work.
```

Commit + push to GitHub `main`.

### 3.3 Set the GitHub repo to archived

Archiving the repo locks all pull requests, issues, and pushes.
Existing clones still work for read-only access; nobody can write
new commits via GitHub.

```sh
gh repo archive Jura-Labs/jura-archive --confirm
```

This is **reversible** via `gh repo unarchive Jura-Labs/jura-archive`,
but the intent is one-way. Verify the archived flag is set:

```sh
gh repo view Jura-Labs/jura-archive --json isArchived
# Expect: {"isArchived":true}
```

### 3.4 Disable secrets on the archived GitHub source repo

Archived repositories cannot run new workflow runs, but the secrets
remain. Rotate or delete to reduce blast radius if the repo is ever
compromised:

```sh
# List existing secrets
gh secret list -R Jura-Labs/jura-archive

# Delete each (replace SECRET_NAME with each entry from the list above)
gh secret delete SECRET_NAME -R Jura-Labs/jura-archive
```

Codeberg now holds the active copies of these secrets (set in Phase 1.3).
The GitHub `juralabs/jura-trace` installer-host repo keeps its
`RELEASE_PAT` since CI still uploads installers there.

## Phase 4: Update the desktop app and documentation

### 4.1 Update tauri.conf.json updater endpoints (optional)

The current `tauri.conf.json` lists two updater endpoints:

```json
"endpoints": [
  "https://juralabs.org/api/updates/latest.json",
  "https://github.com/juralabs/jura-trace/releases/latest/download/latest.json"
]
```

Both remain correct after the migration. No edit required unless you
choose to remove the GitHub Releases fallback (not recommended;
keep it for resilience).

### 4.2 Update CONTRIBUTING.md to point at Codeberg

In the source repo, edit `CONTRIBUTING.md` to direct PRs and issues
to Codeberg rather than GitHub. The current file may still mention
the GitHub flow.

### 4.3 Update README.md badges + clone instructions

Replace any `git clone https://github.com/Jura-Labs/jura-archive.git`
with `git clone https://codeberg.org/juralabs/jura-trace.git`.
Replace any GitHub Actions badge with a Codeberg Actions badge.

### 4.4 Communicate the move

Once Phase 3 archives the GitHub source repo, anyone who has the URL
bookmarked will see the archived banner and the README pointer.

For pilot testers + funders + active contacts: send a short note
saying the source has moved to Codeberg, the install path is
unchanged.

## Phase 5: Verify the end-to-end pipeline

Run the full smoke test from `docs/runbooks/release-pipeline-smoke-test.md`
to confirm:

- A tag pushed to Codeberg triggers the Forgejo runner on the Mac Mini
- macOS artefacts build, sign, and notarise
- Windows artefacts build on GitHub Actions (parallel)
- All artefacts upload to `juralabs/jura-trace` GitHub Releases
- The Cloudflare Worker serves the updated manifest at
  juralabs.org/api/updates/latest.json
- An installed desktop app detects the update and offers to install

If any step fails, troubleshoot before declaring the migration done.

## Rollback

If the migration breaks the pipeline before launch:

1. **Unarchive** the GitHub source: `gh repo unarchive Jura-Labs/jura-archive`
2. **Restore secrets** from a password manager backup (rotate after)
3. **Push to GitHub** to resume the old pipeline
4. **Disable the Codeberg workflow** so it does not race the GitHub one

The Codeberg repo can be left as a passive mirror until the migration
is re-attempted with the failure resolved. No data is lost.

## What this runbook does NOT do

- **Filter-repo history scrub.** Internal docs that were committed to
  the GitHub source repo before the 2026-05-18 cleanup (commit
  `f8fac6e`) remain in git history. JTV-150 covers the optional
  `git filter-repo` scrub if a fully-clean public history is required.
  By default, the Codeberg push includes the full history.
- **Existing GitHub release re-upload.** Past releases on
  `juralabs/jura-trace` are not migrated to Codeberg. They stay on
  GitHub. New releases go to GitHub via the workflow.
- **External link updates.** Funders, press, partners that have
  cached `github.com/Jura-Labs/jura-archive` URLs will see the
  archived banner. Outreach to update those links is a separate
  marketing task.
