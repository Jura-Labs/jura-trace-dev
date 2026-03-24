# Jura Trace — Feature Scoping Document
## Online Content Monitoring & In-App Help Documentation

**Prepared**: 24 March 2026
**Prepared by**: Project Manager
**Context**: Sprint 17 in progress; v1.0 target 27 June 2026 (Sprint 20); v1.1 planning horizon
**Status**: Scoping — not yet scheduled

---

## Executive Summary

This document provides a deep feasibility analysis and implementation scope for two proposed features:

1. **Online Content Monitoring** — detecting whether watermarked or C2PA-signed content is being used without authorisation across the web
2. **In-App Help Documentation System** — comprehensive, persona-specific help content built into the application

Both features have paid-tier dimensions. The document also recommends a paid tier structure for Jura Trace as a whole.

The headline conclusions are:

- **Online monitoring is architecturally complex and philosophically tensions-laden for a local-first product.** A carefully designed hybrid model — local orchestration, optional cloud fingerprint registry, and API integrations as add-ons — can thread the needle. The free tier delivers meaningful local monitoring; the paid tier delivers web-scale search.
- **In-app help is well-scoped and deliverable.** The MVP (static `/help` route, 7 pages) fits into Sprint 18 alongside existing documentation work. The full system (search, contextual links, video tutorials) is v1.1 scope.
- **Paid tiers are viable and appropriate for a CIC**, provided the core mission remains free. The recommended tier structure uses geological naming consistent with the Sanctuary brand.

---

## Part 1: Online Content Monitoring

### 1.1 The Problem

A museum digitises 10,000 images, runs them through Jura Trace PROTECT (C2PA signing, perceptual fingerprinting, invisible watermarking), and publishes them. Three months later, an AI company scrapes the archive and uses those images to train a model. The museum wants to know. Currently, Jura Trace provides no mechanism to detect this.

The user's question: "How can Monitor realistically detect online if my PDF content with a watermark or C2PA is being used?"

### 1.2 Technical Depth: What Signals Are Available

Jura Trace already embeds three types of detectable signal in protected content. Each has different detectability properties across the web:

#### Signal 1: C2PA Content Credentials (XMP/JUMBF metadata)

**What it is**: A cryptographically signed JSON manifest embedded in the file's metadata. Contains creator, timestamp, rights, and a hash of the original content.

**Survives**: File copying, format conversion (JPEG → PNG if metadata is preserved), archiving, hosting unchanged.

**Does NOT survive**: Platform recompression (Instagram, Twitter/X, Facebook strip or rewrite XMP), screenshot, crop, format conversion without metadata preservation, AI model training (the model learns visual features, not the manifest).

**Detection method**: Download the file; call `c2pa-rs` verify on it. The C2PA Trust List (contentauthenticity.org/verify) provides a web-accessible verification endpoint. Twitter/X announced C2PA support in 2024 for images published through their API. Adobe's Content Authenticity Initiative maintains a browser extension that surfaces C2PA badges on supported platforms.

**Realistic monitoring use**: Good for detecting verbatim republication on platforms that preserve metadata. Useless for detecting AI training extraction or screenshots.

**Legal weight**: Strong. A verified C2PA manifest is a timestamped, tamper-evident record of provenance. Useful as evidence in copyright claims.

#### Signal 2: Invisible DWT-DCT-SVD Watermark (128-bit UUID payload)

**What it is**: Frequency-domain modification encoding a 128-bit institution identifier. Survives JPEG Q70+, resize, and 30% crop (validated in Sprint 11).

**Survives**: JPEG compression, resizing, moderate cropping, colour adjustment (partial), screen capture (partial — depends on screen resolution).

**Does NOT survive**: Heavy editing (>50% crop, rotation >10°, heavy JPEG compression Q<50), AI image-to-image transformation, significant style transfer, watermark removal tools (e.g. generative inpainting).

**Detection method**: Jura Trace sidecar already has `/forensics/watermark/extract` — the extraction capability exists. The question is how to run it at scale on URLs found in the wild.

**Realistic monitoring use**: Strong for detecting verbatim hosting or minor edits. Fragile against sophisticated evasion or AI transformation.

**Legal weight**: Moderate to strong. Extractable watermark UUID matching the protected asset's record is meaningful evidence, but defence counsel will challenge robustness.

#### Signal 3: Perceptual Hashes (aHash, dHash, pHash)

**What it is**: Compact numeric representations of visual content that remain similar when the image is resized, compressed, or colour-adjusted. Hamming distance < 10 typically indicates the same image.

**Survives**: Resize, recompression, minor colour grading, moderate crop (hash degrades but remains matchable within threshold).

**Does NOT survive**: Significant crop (>40%), rotation, heavy editing, AI outpainting/inpainting, style transfer, or mirroring (flip creates different hash).

**Detection method**: Compute hash of candidate image; query against local SQLite fingerprint database. For web-scale matching, would require either a central hash registry or integration with TinEye/Google Vision (which maintain proprietary hash databases at scale).

**Realistic monitoring use**: Best signal for identical or near-identical republication. Useless for AI training extraction.

### 1.3 What "Online Monitoring" Could Actually Mean

There are five distinct capabilities that users might mean by "online monitoring." They have very different technical scope and cost profiles:

| Capability | Description | Feasibility | Cloud Required? | Cost Tier |
|---|---|---|---|---|
| **A. Verbatim URL monitoring** | Check specific URLs for continued presence of protected content | High — automated HEAD/GET requests | No (local HTTP client) | Free |
| **B. Reverse image search** | Submit asset thumbnails to TinEye/Google Vision/Bing; receive matching URLs | High — API integration | No (API calls from desktop) | Paid (API costs) |
| **C. C2PA online verification** | Check if a URL's content carries a valid C2PA manifest matching your asset | High for supported platforms | No | Free |
| **D. Watermark extraction from URLs** | Download images from URLs; run watermark extraction; match UUID | High technically but legally complex | No (local processing) | Free / Paid (at scale) |
| **E. Crawl-based fingerprint matching** | Proactively crawl the web or index platforms for fingerprint matches | Very low — requires web-scale infrastructure | Yes — significant | Paid (institutional) |

The Monitor tab in PROJECT_SPEC.md §5.3 specifies "Periodic check of known URLs for institution's fingerprinted content" — this is Capability A, which is the most feasible and the least controversial.

### 1.4 Deep Analysis: Each Capability

#### Capability A: Verbatim URL Monitoring (Local — No Cloud Required)

**How it works**: The user registers a list of URLs where they have published protected content. Jura Trace periodically checks (HEAD request first, then GET if content is needed) whether each URL still returns the expected asset. If the content at a URL has changed (different content hash from the protected original), an alert is raised.

**Scope of detection**: This detects takedowns (the URL now 404s), substitution (a different image now lives at the URL), and modified content. It does NOT detect content appearing at NEW URLs.

**Implementation**: Rust `reqwest` crate (already in `Cargo.toml`) handles HTTP. SQLite already stores fingerprints. A background Tauri worker checks URLs on a configurable schedule (default: daily). No cloud infrastructure required.

**Legal and privacy considerations**: HEAD/GET requests to URLs the institution itself published is legally unproblematic. Respecting `robots.txt` is good practice but not required for monitoring your own content.

**Effort**: Medium (S — 1 sprint)

**Free tier**: Yes. This is core MONITOR functionality and should be free.

#### Capability B: Reverse Image Search API Integration

**How it works**: For each protected image, Jura Trace generates a thumbnail (or uses the stored perceptual hash) and submits it to reverse image search APIs that maintain their own web-scale indices.

**Candidate services**:

| Service | API Availability | Cost Model | Accuracy | Notes |
|---|---|---|---|---|
| TinEye | Yes — `api.tineye.com` | $200 for 5,000 searches; $0.04/search at scale | High for verbatim copies | Best for exact/near-exact matches; less good for AI derivatives |
| Google Vision API | Yes — `vision.googleapis.com/v1/images:annotate` | $1.50 per 1,000 images for Web Detection | Moderate | Returns matching URLs + similar images; no batch mode |
| Bing Visual Search | Yes — `api.bing.microsoft.com/v7.0/images/visualsearch` | $4.00 per 1,000 transactions | Moderate | Less coverage than Google for non-English content |
| IMATAG | Yes — watermark-based detection service | Enterprise pricing (est. EUR 500+/month) | High for watermarked content | Specifically designed for rights management |

**Cost modelling** for a museum with 10,000 assets, monthly scans:
- TinEye: 10,000 × $0.04 = $400/month
- Google Vision: 10,000 × $0.0015 = $15/month (cheap but less robust)
- IMATAG: ~EUR 500/month flat

**Privacy implications**: Submitting image thumbnails to third-party commercial services is a meaningful departure from the local-first philosophy. The user must explicitly consent, understand what is shared (thumbnail, not full image; IP address; institution identity implied by account).

**UK legal considerations**: Automated submission of your own published content to search services is not prohibited by the Computer Misuse Act 1990. GDPR considerations arise only if the images contain personal data (photographs of people). Terms of Service must be checked per API — Google Vision and TinEye both permit this use.

**Implementation**: API client in Rust or Python sidecar; API keys stored encrypted in SQLite (not in config files); rate limiting built in; results stored locally. No third-party service sees the full-resolution originals — thumbnails only, generated locally.

**Effort**: Medium-Large (M — 2 sprints for 2+ integrations with proper key management and result deduplication)

**Tier recommendation**: PAID (Stratum tier). API costs mean this cannot be offered free at scale.

#### Capability C: C2PA Online Verification

**How it works**: When the user provides a URL to an image or video, Jura Trace downloads it and runs `c2pa-rs` verification, checking whether a valid Content Credential is present and whether it matches an asset in the local database.

**Platform support landscape**:
- **Twitter/X**: Announced C2PA support; implemented in API v2 for images posted via the API. Web UI display in beta.
- **LinkedIn**: Active CAI member; C2PA badge in rollout.
- **Adobe Stock/Behance**: Full C2PA support.
- **Getty Images**: C2PA in progress.
- **Facebook/Instagram**: No announced C2PA support as of August 2025.
- **TikTok**: Pilot programme, not generally available.
- **General web**: C2PA only survives if the host preserves XMP/EXIF metadata — many CDNs and CMS platforms strip it.

**What this adds to the existing VERIFY pipeline**: Very little for one-off checks — the current VERIFY tab already reads C2PA manifests. The addition for MONITOR is **automated periodic checking** of registered URLs: "Has my content at this URL still got a valid C2PA credential?"

**Effort**: Small (XS — can be wired into Capability A's URL monitoring loop with 2–3 points extra)

**Tier recommendation**: FREE. C2PA reading is a core local capability. Automated periodic checking is a natural MONITOR extension.

#### Capability D: Watermark Extraction from URLs at Scale

**How it works**: The MONITOR tab maintains a list of URLs (user-provided, or sourced from reverse image search results). For each URL, Jura Trace downloads the image and submits it to the local sidecar `/forensics/watermark/extract` endpoint. The extracted UUID is matched against the local database of protected assets.

**Why this is powerful**: Unlike C2PA (which survives only when metadata is preserved) or perceptual hashing (which degrades with editing), the DWT-DCT-SVD watermark survives moderate manipulation. If a scraped image is slightly cropped or recompressed and hosted elsewhere, the watermark extraction can still identify it as belonging to a specific institution.

**Latency consideration**: Watermark extraction takes approximately 1–3 seconds per image. Monitoring 1,000 URLs takes 30–90 minutes of sidecar processing time. This is fine for overnight batch monitoring but not for real-time alerting.

**Legal considerations in UK**: Downloading a publicly accessible URL for the sole purpose of checking whether your own watermark is present is legally defensible under fair dealing provisions and because the content is yours in the first place. However, doing this at scale to URLs you do not own raises questions under the Computer Misuse Act 1990 if it imposes significant load on third-party servers. Rate limiting (max 1 request/second to any domain) is essential.

**Privacy/Terms of Service**: Most hosting platforms permit automated access to public URLs by bots that identify themselves properly (User-Agent). Institutional contexts (monitoring their own published content) have a clear legitimate interest.

**Effort**: Small-Medium (1–2 sprints to wire URL list management into MONITOR and automate watermark extraction on discovered URLs)

**Tier recommendation**: FREE for local URL list monitoring (user provides URLs); PAID for integration with discovery sources (reverse image search feeds results into watermark extraction pipeline).

#### Capability E: Web-Scale Crawl-Based Fingerprint Matching

**What it would require**: A service that indexes a significant portion of the web's image content as perceptual hashes, and allows institutions to query their own hashes against that index.

**Why this is not realistic for v1.1 or even v2.0 as a self-hosted service**:
- Common Crawl (the largest open web crawl) is 100+ TB. Processing it for perceptual hashes requires hundreds of GPU-hours.
- Google's reverse image search, TinEye, and Yandex Images are the only organisations with this infrastructure at scale.
- Running even a partial index for a specific content domain (e.g. European cultural heritage) would cost tens of thousands of pounds per month in compute.

**Realistic alternative — Federation model**: Multiple Jura Trace instances could share a lightweight hash registry without centralised cloud infrastructure. Institutions opt-in to publishing their perceptual hashes (NOT their content) to a shared registry. When one institution queries the registry, it checks all other institutions' hashes for matches. This is a peer-to-peer model — no central server needs to hold content.

**Technical implementation of federation**:
- Registry entries: `{ institution_id: UUID, asset_hash: [pHash, aHash, dHash], c2pa_credential_id: string, rights_statement: URL }` — no content, no thumbnails
- Protocol: Could use ActivityPub (the Mastodon/Fediverse protocol) for distributed publishing, or a simpler custom REST protocol
- Trust: Each institution self-certifies. A trust list (similar to C2PA's trust list) could vouch for registered institutions.
- Privacy: Perceptual hashes are not reversible to the original image. The institution is only sharing "I own something that hashes to X" — not the image itself.

**Development cost**: L size (3–4 sprints) for a basic federation protocol. XL (6+ sprints) for a production-ready federated network with trust management.

**Tier recommendation**: Federation registry participation = PAID (Stratum or Geode tier). Operating a registry node = institutional/enterprise.

### 1.5 The Local-First Tension

The project's core architectural principle is: "All processing on-device. No cloud dependency. No data exfiltration." Online monitoring necessarily involves external services. How to reconcile this?

**Resolution framework**:

1. **All processing of the actual content remains local.** Watermark extraction, fingerprint computation, C2PA verification — these always happen on the user's machine. The sidecar processes originals locally.

2. **What leaves the device is minimal and explicit**: For reverse image search, a thumbnail (not the full-resolution file) is submitted. The user must explicitly enable each external service and accept a clear privacy disclosure.

3. **Nothing is automatic or silent.** Every external API call is user-initiated or explicitly scheduled, with a log entry in the audit trail.

4. **The local-only tier is fully functional.** URL monitoring (Capability A), C2PA verification of known URLs (Capability C), and watermark extraction from user-provided URLs (Capability D) all work entirely locally. External APIs are an optional enhancement.

This framing means Jura Trace can credibly say: "Local-first. External services optional and explicit." That is different from "no cloud" but is honest and defensible for a CIC serving institutional users who need real monitoring capability.

### 1.6 MONITOR Architecture Recommendation

Based on the analysis above, the recommended MONITOR architecture for v1.1 is a **three-layer model**:

```
Layer 1 — Local (free, always on):
  ┌─────────────────────────────────────────────────────┐
  │  URL Watchlist                                      │
  │  • User registers URLs where content is published   │
  │  • Periodic HEAD check: still there? Content same?  │
  │  • C2PA verification on re-download                  │
  │  • Watermark extraction on re-download               │
  │  • Alert Dashboard: shows changes since last check  │
  │  • Stored entirely in local SQLite                  │
  └─────────────────────────────────────────────────────┘

Layer 2 — Assisted Discovery (paid, API costs passed through):
  ┌─────────────────────────────────────────────────────┐
  │  Reverse Image Search Integration                   │
  │  • User configures TinEye/Google Vision API key     │
  │  • Scheduled submissions of protected asset hashes  │
  │  • Matching URLs fed back into Layer 1 watchlist    │
  │  • Full provenance chain: found via search → checked│
  │    locally → watermark confirmed → alert generated  │
  └─────────────────────────────────────────────────────┘

Layer 3 — Federated Registry (paid, institutional):
  ┌─────────────────────────────────────────────────────┐
  │  Juralabs Hash Registry (v2.0 horizon)              │
  │  • Opt-in hash publication for protected assets     │
  │  • Peer queries against all participating           │
  │    institutions' hash sets                          │
  │  • No content leaves the institution — hashes only  │
  │  • Trust list for verified cultural institutions    │
  └─────────────────────────────────────────────────────┘
```

### 1.7 MONITOR Sprint Sizing

**v1.1 MONITOR MVP (Layer 1 + partial Layer 2):**

| Component | Size | Sprints | Notes |
|---|---|---|---|
| URL watchlist data model + SQLite schema | XS | 0.5 | New tables: `monitor_urls`, `monitor_events` |
| Watchlist management UI (MONITOR tab) | S | 1 | Add/remove URLs, check frequency, last-checked status |
| Periodic HTTP check engine (Rust, `reqwest`) | S | 1 | Background worker, respects rate limits, generates audit events |
| C2PA re-verification on content change | XS | 0.5 | Wire existing C2PA verify into the check loop |
| Watermark extraction on content change | XS | 0.5 | Wire existing sidecar watermark extract into check loop |
| Alert dashboard UI | S | 1 | New alerts view, filter by type, link to full analysis |
| TinEye API integration (Layer 2) | S | 1 | API key management, rate limiting, result deduplication |
| Privacy disclosure for Layer 2 | XS | 0.5 | Must-have for user consent |
| **Total MONITOR MVP** | **M** | **~3 sprints** | Confidence: Medium (±25%) |

**Full Layer 2 (all APIs + management UI):** Add 2 sprints.
**Layer 3 Federation:** Add 4–6 sprints. Post-v2.0 horizon.

### 1.8 Cost Model for Juralabs

If Juralabs runs reverse image search as a managed service (buying API credits and reselling as part of a subscription):

| Scenario | Assets Monitored | Monthly API Cost | Viable Price Point |
|---|---|---|---|
| Small institution (100 assets, weekly scan) | 400 calls/month | $0.60 (Google) — $16 (TinEye) | £5–10/month |
| Medium institution (1,000 assets, weekly scan) | 4,000 calls/month | $6 (Google) — $160 (TinEye) | £25–50/month |
| Large institution (10,000 assets, daily scan) | 300,000 calls/month | $450 (Google) — $12,000 (TinEye) | £500+/month |

**Conclusion**: Google Vision is viable for a managed service at small/medium scale. TinEye is viable only if pass-through (user provides their own API key). For large institutions, IMATAG or a negotiated enterprise deal would be required.

**CIC sustainability note**: At £25/month for medium institutions, Juralabs would need approximately 40 paying institutions to cover £1,000/month in API costs at break-even. This is realistic for a post-v1.0 sustainability model.

---

## Part 2: In-App Help Documentation System

### 2.1 What Was Proposed

A planning agent produced a detailed implementation plan for a `/help` route with:
- 7 content pages (4 section guides + methodology + glossary + personas)
- Sidebar navigation
- Contextual help links from existing pages
- GlossaryEntry, HelpTopicCard, ContextualHelpLink Svelte components

This document assesses fit, sizing, and sprint allocation for that plan.

### 2.2 Why In-App Help Matters for v1.0

The v1.0 KPI is 50+ institutional users. The target users — museum archivists, journalists, fact-checkers — are not software engineers. They need to understand:

- What trust scores mean and how to act on them
- Why a verdict says "inconclusive" and what to do next
- How to interpret ELA heatmaps without a forensics background
- What C2PA Content Credentials prove (and what they don't)
- How the watermark works, what attacks it resists, and what the limitations are

Without in-app help, every pilot institution requires a human support touchpoint. At 50 institutions, that is not scalable. Good in-app help reduces support overhead and increases user confidence — both are essential for the NPS > 40 target.

Sprint 18 already includes `Doc1` through `Doc5` (external documentation in `docs/user-guide/`). The question is whether to also build in-app help, and when.

### 2.3 MVP vs Full Scope Definition

#### Minimum Viable Help (v1.0)

The minimum that ships with v1.0 to hit the NPS target and support institutional adoption:

| Component | Description | Effort |
|---|---|---|
| `/help` route with sidebar navigation | Static SvelteKit page; no search; no dynamic content | 2 pts |
| Section guides: Protect, Verify | One page per section; plain prose, annotated screenshots | 4 pts (2 per page) |
| Methodology transparency page | How detectors work, what trust scores mean, limitations | 3 pts |
| Glossary (30–40 terms) | `GlossaryEntry` component; terms: C2PA, ELA, pHash, deepfake, watermark, etc. | 3 pts |
| `?` contextual help links | Small `ContextualHelpLink` component on verdict, trust score, and C2PA sections | 2 pts |
| **Total** | | **14 pts (~1 sprint)** |

This is deliberately conservative. The content writing is the bottleneck, not the engineering. The `/help` route is a static SvelteKit page with no search infrastructure required.

#### Full Help System (v1.1)

| Component | Description | Effort |
|---|---|---|
| Monitor + Settings guides | Completing the 4-section guide set | 4 pts |
| Persona-specific paths | Archivist / Journalist / Creator routes through the help content | 3 pts |
| Help search (client-side) | Lunr.js or Fuse.js search across help content | 3 pts |
| Progressive disclosure | Beginner / Advanced toggle on methodology page | 2 pts |
| Video tutorials | Embedded or linked screencasts (5 × 2–3 min) | 8 pts (production + integration) |
| **Total incremental** | | **~20 pts (~2 sprints)** |

### 2.4 Sprint Allocation Recommendation

**Sprint 18 is the right place for the MVP help system**, alongside the existing documentation work.

Rationale:
- Sprint 18's goal is "Trusted & Documented." The 22-point capacity is already allocated to external docs (Doc1–Doc5, Dev1–Dev3, PR1–PR3, CL1–CL2).
- The external docs in Sprint 18 (user-guide/protect.md, user-guide/verify.md) will be written anyway. The in-app help content can be written once and rendered in both places — same Markdown source, different rendering context.

**Revised Sprint 18 integration approach**: Write the user guide content as Markdown files in `ui/src/lib/help/`. These are:
1. Rendered at `/help` in-app (via SvelteKit's dynamic import or static bundling)
2. Copied/linked to `docs/user-guide/` for the external documentation site

This avoids writing the content twice. The engineering overhead is the `/help` route and navigation components (~6 points), which must come from somewhere in Sprint 18.

**Sprint 18 capacity re-evaluation**:

Current Sprint 18 allocated: 22 points
Adding help MVP: 14 points gross, but 6 points are saved if help content replaces separate external doc writing:

Net additional: ~8 points

Sprint 18 cannot absorb 8 additional points without trade-offs. Options:

**Option A**: Defer press kit (PR1–PR3, 5 points) to Sprint 19 buffer. Sprint 19 has 22 points with 0 buffer currently — this would make it tight but feasible if accessibility remediation (A2) is less than 3 points.

**Option B**: Keep Sprint 18 as-is, add help MVP as a parallel workstream in Sprint 18 if the documentation writing proves faster than estimated. Treat it as a stretch goal.

**Option C**: Split help MVP — ship the `/help` route and methodology + glossary pages in Sprint 18 (7 points); ship the Protect and Verify section guides in Sprint 19 alongside accessibility work (7 points).

**Recommendation**: Option C. The methodology page and glossary are pure content creation that parallelises well with engineering work (different skills). The section guides require screenshots from a complete UI, which will be more stable in Sprint 19.

**Revised allocation**:
- Sprint 18: `/help` route + navigation (2 pts), methodology page (3 pts), glossary (3 pts) = 8 pts additional
  - Offset by: consolidating Doc1 + Doc2 external guides with help content (save 4 pts)
  - Net: +4 pts. Achievable by trimming PR1 press kit to essentials (reduce from 2 to 1 pt)
- Sprint 19: Protect section guide (2 pts), Verify section guide (2 pts), `?` contextual links (2 pts) = 6 pts additional
  - Sprint 19 currently has 22 pts allocated and flagged AMBER with 0 buffer. These 6 pts must come from somewhere.
  - Recommended trade: scope the cross-platform edge case fixes (EF1–EF4, 6 pts) more tightly — EF1 and EF3 to 1.5 pts each; EF4 deferred to v1.0.1.

### 2.5 Content Parallelisation

The help content writing can be done in parallel with engineering work. Key points:

- A non-technical writer can produce draft content for the methodology page and glossary without needing to be in the codebase.
- Section guides require screenshots and specific UI knowledge — needs someone familiar with the application.
- Persona-specific paths (v1.1) benefit from direct input from pilot institutions — schedule a working session with pilot contacts during Sprint 19's RC feedback collection.

If Juralabs has any community volunteers (CIC community benefit angle), the help content writing is an excellent contribution task that does not require coding skills. This aligns with CIC community benefit obligations.

### 2.6 Paid Tier Angle for Help Content

**What must remain free**: All in-app help, methodology transparency, glossary, and section guides. Users of the free tier need to understand what the tool is doing — making methodology opaque behind a paywall would undermine the trust mission and conflict with CIC community benefit objectives.

**What could be paid**:

| Content | Rationale | Tier |
|---|---|---|
| Video tutorials (screencasts, narrated) | Production cost is real; niche audience; streaming costs | Stratum+ |
| Advanced methodology deep-dives (e.g. "Understanding JPEG ghost analysis for forensic investigations") | High value for specialists; low volume demand | Stratum+ |
| Sector-specific user guides (museums, journalists, NGOs — with institution-specific workflows) | Customisation work; institutional value | Geode (institutional) |
| API documentation for integrations | Relevant only to paying institutional/commercial users | Geode+ |

**Community benefit note**: Premium content must not gate the understanding needed for basic informed use. The free methodology page must explain what trust scores mean. Premium content can go deeper — "how to build a forensic case using Jura Trace outputs" is an advanced guide appropriate for paid access.

---

## Part 3: Paid Tier Structure

### 3.1 Design Principles for a CIC Tier Structure

Jura Trace is built by a Community Interest Company. CIC annual returns require demonstrating community benefit. The tier structure must be designed so that:

1. The communities that need the tool most — journalists, community groups, educators, small cultural organisations — can use it for free, indefinitely.
2. Organisations with greater resources (large institutions, commercial media companies) contribute to sustainability.
3. The paid tiers offer genuine additional value, not artificial restrictions on free features.
4. The structure is explainable in a funding application: "We charge commercial users a sustainability fee to ensure the tool remains free for the communities it was designed to serve."

### 3.2 Recommended Tier Structure

The geological brand naming already suggests a natural progression: surface → deep. The tiers below use mineral names from the existing Sanctuary palette for consistency.

---

#### Tier 1: Flint — Free, forever

**Who**: Non-commercial users — individual journalists, researchers, educators, community groups, small cultural organisations (under 50,000 annual visitors or equivalent)

**Licence**: PolyForm Noncommercial 1.0.0 (current licence — no change required)

**Features**:
- Full PROTECT pipeline: C2PA signing, perceptual fingerprinting, invisible watermarking, batch processing, CSV export
- Full VERIFY pipeline: all forensic detectors, deepfake analysis, C2PA verification, EXIF analysis, claim checker
- Full multi-format support: images, documents, video, audio (4 types at v1.0; 3D at v1.1)
- MONITOR: URL watchlist (Capability A), C2PA re-verification, watermark extraction on known URLs
- In-app help: all pages, methodology transparency, glossary
- Community support (GitHub Issues, documentation)

**Rationale**: This is the full product. The free tier must not be crippled — it must be genuinely useful for the communities the tool was built for.

---

#### Tier 2: Stratum — Sustainability contribution

**Who**: Commercial users, media organisations, for-profit companies using Jura Trace as part of their workflow

**Price**: GBP 49/month per organisation (or GBP 490/year)

**Licence**: Commercial licence (new licence required — straightforward extension of PolyForm)

**Additional features over Flint**:
- MONITOR: Reverse image search integration (user brings own API keys — Jura Trace handles the orchestration, rate limiting, result management)
- MONITOR: Scheduled monitoring with email/desktop alerts (SMTP configuration in Settings)
- Trust report PDF: branded output with organisation logo and custom header
- Advanced VERIFY: batch verification (process a folder of files through VERIFY, not just one at a time)
- Video tutorials and advanced methodology guides
- Priority support (dedicated email address; 48-hour response)

**Rationale**: The key differentiators are monitoring automation and workflow features that serve professional/commercial use cases. None of these features are "core" to the community benefit mission — they are productivity enhancements for organisational users.

---

#### Tier 3: Geode — Institutional

**Who**: Large cultural institutions, national archives, media companies, public authorities (EU AI Act compliance use cases)

**Price**: GBP 299/month per institution (or GBP 2,990/year)

**Licence**: Commercial institutional licence; includes deployment rights for up to 10 workstations

**Additional features over Stratum**:
- MONITOR: Managed reverse image search (Juralabs provides pre-configured API access; no need for institution to hold API keys)
- Federation registry participation: publish asset hashes to Juralabs' shared hash registry; receive alerts when matches are found by other participating institutions
- Custom RAG knowledge base package: sector-specific fact-check databases (e.g. cultural heritage domain, news verification domain)
- Sector-specific user guides and onboarding documentation
- Dedicated onboarding session (2 hours via video call)
- API documentation for workflow integration (e.g. embed Jura Trace verification into a CMS or DAM)
- Multi-seat deployment: up to 10 installed seats under one licence
- Annual compliance report template: pre-formatted for EU AI Act transparency reporting

**Rationale**: Institutional features justify the price point. The managed API access and federation registry are genuinely high-value for large collections. The EU AI Act compliance angle is commercially compelling — institutions facing legal obligations from August 2026 will pay for a tool that makes compliance tractable.

---

#### Tier 4: Bedrock — Enterprise / Commercial licence

**Who**: Commercial archives, media companies, licensing platforms, AI companies seeking to demonstrate compliance

**Price**: From GBP 1,000/month (custom contract)

**Licence**: Commercial enterprise licence; unlimited seats; right to embed in commercial products

**Additional features over Geode**:
- White-label rights (embed Jura Trace technology in commercial products with custom branding)
- Custom forensic model training (GBM classifier retrained on customer's specific content domain)
- SLA-backed support (99.5% uptime for sidecar; 4-hour response for critical issues)
- Custom RAG knowledge base build (Juralabs staff curate and maintain)
- Federated registry operator status (run your own registry node)
- Source code access under commercial licence (for security audit by enterprise IT)

**Rationale**: This tier targets commercial exploitation of the technology. The CIC community benefit obligation is fulfilled by ensuring this tier's revenues subsidise the Flint (free) tier. A contract with one Bedrock customer could fund the Flint tier for a year.

---

### 3.3 Free/Paid Feature Split Summary

| Feature | Flint (Free) | Stratum | Geode | Bedrock |
|---|---|---|---|---|
| Full PROTECT pipeline | Yes | Yes | Yes | Yes |
| Full VERIFY pipeline | Yes | Yes | Yes | Yes |
| MONITOR: URL watchlist | Yes | Yes | Yes | Yes |
| MONITOR: C2PA re-verification | Yes | Yes | Yes | Yes |
| MONITOR: Watermark extraction on URLs | Yes | Yes | Yes | Yes |
| MONITOR: Reverse image search (own keys) | No | Yes | Yes | Yes |
| MONITOR: Managed API (Juralabs keys) | No | No | Yes | Yes |
| MONITOR: Federation registry | No | No | Yes | Yes |
| Batch VERIFY | No | Yes | Yes | Yes |
| Branded trust reports | No | Yes | Yes | Yes |
| Video tutorials | No | Yes | Yes | Yes |
| In-app help (all text) | Yes | Yes | Yes | Yes |
| Sector-specific user guides | No | No | Yes | Yes |
| Custom RAG knowledge base | No | No | Yes | Yes |
| EU AI Act compliance template | No | No | Yes | Yes |
| Multi-seat deployment | 1 seat | 3 seats | 10 seats | Unlimited |
| White-label rights | No | No | No | Yes |
| Support | Community | Priority email | Dedicated + onboarding | SLA + custom |

### 3.4 CIC Community Benefit Framing

For the CIC annual return and funding applications:

"Jura Trace is free for all non-commercial use under PolyForm Noncommercial 1.0.0. This includes individual journalists, educators, community media organisations, and cultural institutions of any size. Commercial licensing fees from Stratum, Geode, and Bedrock tiers directly fund development of the free tier and ensure the tool remains maintained and improved for community users. In the reporting period, [N] organisations used the free tier, processing [N] assets and conducting [N] verification checks."

---

## Part 4: Dependencies and Risks

### 4.1 MONITOR Dependencies

| Dependency | Blocks | Status | Mitigation |
|---|---|---|---|
| MONITOR SQLite schema (new tables) | All MONITOR work | Not started | Design in v1.1 planning sprint |
| C2PA verify function exposed as Rust utility | URL re-verification | Already built (c2pa.rs) | Wire into monitor loop — 1 pt |
| Watermark extract function exposed as Rust utility | URL watermark check | Already built (sidecar) | Wire into monitor loop — 1 pt |
| Reverse image search API keys | Layer 2 monitoring | User-provided | Key storage (encrypted SQLite) must be built first |
| Federation protocol design | Layer 3 registry | Not started | Requires architectural spike in v1.1 planning |
| CIC commercial licence (Stratum/Geode/Bedrock) | Paid tier launch | Pending legal | Draft licence required before first paid customer |

### 4.2 Help System Dependencies

| Dependency | Blocks | Status |
|---|---|---|
| Stable UI (all sections complete) | Screenshot production for guides | Sprint 19 onwards — take screenshots at RC1 |
| Sprint 16 P1 timing data on VerificationResult | S17-D3 / methodology page | Conditional — check in Sprint 17 |
| Pilot institution review | Guide quality | RC1 distribution Sprint 19 |

### 4.3 Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| C2PA adoption on social platforms remains limited | High | Medium | Focus MONITOR on URL watchlist and watermark extraction, not C2PA reliance |
| API costs exceed projections for managed monitoring service | Medium | High | Hard per-institution monthly limits; circuit breaker in API client; transparent cost reporting in UI |
| Web scraping terms of service changes by Google/TinEye | Medium | Medium | Architecture supports multiple API backends; add/remove providers without structural change |
| Help content writing under-estimated | High | Medium | Timebox per page; ship prose over polish; screenshots can be added in v1.0.1 |
| CIC commercial licence drafting delayed | Low | Medium | Stratum/Geode tier blocked without licence; engage a CIC/charity solicitor in Sprint 18 |
| UK Computer Misuse Act concerns re: automated URL checking | Low | High | Limit to URLs the institution itself published; legal opinion in Geode tier documentation |

---

## Part 5: Recommended Next Steps

### Immediate (Sprints 17–18)

1. **Commercial licence drafting**: Engage a solicitor experienced in CIC and open-source licensing to draft the commercial licence covering Stratum, Geode, and Bedrock tiers. Budget: ~GBP 500–1,000 for a simple commercial addendum to PolyForm NC. Needed before Sprint 20 (v1.0 launch).

2. **MONITOR data model spike** (1 point, Sprint 18): Design the SQLite schema for `monitor_urls` and `monitor_events` tables. Does not need to be implemented in Sprint 18 — design only, so v1.1 can start building immediately.

3. **Help MVP in Sprint 18**: Implement the `/help` route, methodology page, and glossary as detailed in Part 2. Use Markdown source files in `ui/src/lib/help/` as the single source of truth for both in-app and external documentation.

4. **Apple Developer ID**: Already flagged as critical path. Start enrolment immediately if not done.

### v1.1 Planning (Post Sprint 20)

5. **MONITOR Layer 1 sprint** (2 sprints): URL watchlist + alert dashboard. This directly delivers the PROJECT_SPEC.md Phase 4 MONITOR capability.

6. **Reverse image search integration sprint** (1 sprint): TinEye + Google Vision API clients, key management, result deduplication. Gate behind Stratum tier.

7. **Help section guides sprint** (1 sprint): Protect, Verify, Monitor, Settings guides with screenshots. Persona-specific paths and search in the sprint after.

8. **Tier infrastructure**: Basic licence check (Stratum/Geode feature gating) — the simplest approach is a locally stored licence key validated against a public Juralabs endpoint. Deliberately simple: no telemetry, no phone-home for Flint tier users.

### v1.2+ (Federation)

9. **Federation protocol design spike** (1 sprint): Architecture document and protocol proposal for the hash registry. Engage with Europeana and Content Authenticity Initiative as potential early partners — they have the institutional network.

10. **Federation MVP** (3–4 sprints): Opt-in hash publication, basic peer discovery, match alerts.

---

## Appendix: Effort Summary

| Feature | Sprint | Size | Points | Confidence |
|---|---|---|---|---|
| MONITOR Layer 1 (URL watchlist, C2PA, watermark) | v1.1, S21–22 | M | ~30 pts | Medium |
| MONITOR Layer 2 (reverse image search integration) | v1.1, S23 | S | ~15 pts | Medium |
| MONITOR Layer 3 (federation) | v1.2+ | XL | 60–80 pts | Low |
| Help MVP (route, methodology, glossary, contextual links) | Sprint 18–19 | S | 14 pts | High |
| Help full system (search, persona paths, video) | v1.1, S21 | M | ~20 pts | Medium |
| Tier infrastructure (licence gating) | Sprint 20 / v1.1 | S | ~8 pts | High |
| Commercial licence drafting | Sprint 18 | External | Legal budget | N/A |

---

*Scoping document prepared 24 March 2026. Review at Sprint 20 retrospective (after v1.0 ships) to set v1.1 priorities.*
