# Static pages for juralabs.org

Static HTML for pages hosted on `juralabs.org` outside the WordPress
or static-site CMS that the marketing site uses. These files are
deployed to Cloudflare Pages (or copied to the chosen static host)
and the route map at the juralabs.org zone level points the relevant
URLs at them.

## Pages

### `/downloads/`

`downloads/index.html`. Direct-installer page listing the macOS DMG,
Windows MSI, Linux AppImage / DEB. Auto-detects platform from
User-Agent and surfaces a recommended download card. All links resolve
to GitHub Releases on `juralabs/jura-trace` so the table tracks the
latest release automatically without re-deploying this page.

Verification callout above the install guides reminds users to check
the SHA-256 sum against the release notes before installing.

## Deployment

Two options depending on the juralabs.org hosting setup:

**Option A: Cloudflare Pages.** Push this directory as a project,
configure the build output as `infrastructure/static-pages`, set the
custom domain to `juralabs.org/*` with route restricted to
`/downloads/*`. Free tier, instant cache propagation.

**Option B: wrangler upload.** If the juralabs.org site is already a
Cloudflare Pages project, drop the `downloads/` directory into the
existing project's content tree and re-deploy.

**Option C: static-server upload.** If juralabs.org is hosted
elsewhere (Hetzner, DigitalOcean, etc.), copy the `downloads/`
directory into the document root.

## Updating

The download links resolve to GitHub Releases on
`juralabs/jura-trace`. As long as the release-asset naming convention
holds (`Jura.Trace_aarch64.dmg`, `Jura.Trace_x64_en-US.msi`,
`jura-trace_amd64.AppImage`, `jura-trace_amd64.deb`), no edits to this
page are needed for new releases.

If the asset naming convention changes, update the table rows + the
`data-platform` attributes in `downloads/index.html`. The detection
script reads from the `data-platform` attribute to pick the right
asset URL.
