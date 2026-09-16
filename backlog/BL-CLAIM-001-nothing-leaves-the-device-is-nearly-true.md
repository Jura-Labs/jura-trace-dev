# BL-CLAIM-001: "no data is uploaded to external servers" is nearly true, and nearly is not good enough

**Status**: Open, and narrowed. Found 3 September 2026. On the same day,
on this evidence, Paul set claims row 5 to `true`, moved row 7 to
`conflict`, and closed SR-25. **Row 6 is what remains**, and it is closed
by the README rewrite and SR-24 below.
**Raised**: 3 September 2026
**Severity**: High, and the correction is now advised as immediate rather
than scheduled. This is the claim the product is bought on, it appears on
the front page of the README, and it is carried in three published
articles.

## What is wrong

`README.md:7`:

> All processing happens locally on your machine. No data is uploaded to
> external servers.

The first sentence is true. The second is true of user content and is not
true as written, because five outbound calls exist. Read from the code on 3
September 2026:

| What | Where | When it fires |
|---|---|---|
| Update check to `juralabs.org/api/updates/latest.json` and the GitHub releases fallback | `src-tauri/tauri.conf.json:37-45`; called from `ui/src/lib/updater.ts:56` | Only when the user clicks Check for Updates in Settings or the menu. **Not on startup** |
| RFC3161 timestamp fetch to `timestamp.digicert.com` | `src-tauri/src/c2pa.rs:323`, used at `:607` | Every time the user signs a file |
| Download of a URL the user pasted | `src-tauri/src/lib.rs:1651-1750` | Verify-by-URL, user-initiated |
| Weather cross-reference to `archive-api.open-meteo.com` | `fetch_weather_context`, `src-tauri/src/lib.rs:~1163` | Enhanced network mode only, gated server-side |
| Swagger UI stylesheet and bundle from `unpkg.com` | `src-tauri/src/api/mod.rs:278,282` | When the local REST API's docs page is opened |

None of these uploads a user's file. Four of the five are things the user
asked for. The claim is defensible in substance and wrong in wording, which
is the worst combination, because it cannot be defended by pointing at the
sentence.

**One of them is closer to the line than the others.** Legal review on 3
September made the point that the RFC3161 timestamp request sends a hash
*derived from the user's content*. No file leaves the machine and the hash
does not disclose the content, but "no data is uploaded" is then not merely
imprecise, it is literally false on the product's core claim. The other
four calls leak IP address and activity timing rather than content. That
assessment reads the current sentence as a misleading action under the
DMCC Act 2024 s.226, and advises the one-line correction as immediate
rather than scheduled.

Two things sharpen it.

**The Swagger call is unpinned.** The URLs carry no version:
`https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js`. Whatever unpkg
serves as latest is executed inside a page served by the local authenticated
API. This is SR-24 in the security register, and it is the one item on the
list that is not user-initiated in any meaningful sense.

**Telemetry is in better shape than the register thinks.** SR-25 records
that opt-in false-positive reports "fail silently" because
`telemetry.juralabs.org` is not served. Reading the code, they do not fail,
because they are never attempted. `src-tauri/src/telemetry/mod.rs:222`
`upload_report` is a Phase A stub that logs `[TELEMETRY STUB] Would
upload...` and returns; it has **no callers outside its own module**;
`telemetry_enabled()` at `:184` defaults to false. The shipped false-positive
route is `ui/src/lib/fp-report.ts`, which builds a `mailto:` link and a
clipboard copy, and whose header comment states "Nothing is transmitted by
the app". Claims register row 5, "Trace: no telemetry, no account, no
server storing files", is **true of the shipped product** and should be
re-verified and moved off `stale`.

**Watermarking is not in the product at all.** `V1_SHOW_WATERMARK = false`
(`ui/src/lib/featureFlags.ts:81`). The Rust embed and the Python extract are
bit-incompatible. Claims row 7, "Trace applies invisible watermarks", is
false as a present-tense statement, and it appears on the services page and
in the data-sovereignty article.

## Why it matters

The claims register exists because a claim that is 95 per cent true is the
one that gets corrected in public by somebody else. Trace's whole
proposition is that it is honest about uncertainty. A front-page sentence
that overstates by a little is the specific failure that proposition cannot
survive.

`AGENTS.md:286` in this repository carries the same overstatement in a
stronger form: "No cloud calls. No telemetry." The telemetry half is right.
The first half is not.

## What would fix it

1. **Rewrite `README.md:7`** to say what is true, in Paul's register. A
   draft, for editing, not for publishing:

   > Your files are analysed on your own machine. Nothing you open, sign or
   > verify is uploaded anywhere. Jura Trace makes a small number of network
   > calls, all of them either something you asked for or something you can
   > turn off: an update check when you press the button, a timestamp
   > request when you sign a file, and a fetch of any URL you paste in.
   > Enhanced network mode, which you can disable, adds a weather lookup
   > used by one detector.

   The exact words are Paul's call. The list is the part that has to be
   there.

2. **Bundle the Swagger assets locally.** This is JTV-209 in Plane and
   SR-24 in the register. It removes the one call that is not user-initiated
   and the one unpinned remote script, and it shortens the sentence above.
   Legal review flags the unpinned `unpkg` load specifically: pin or vendor
   it, do not leave a third party able to change what executes inside a
   local authenticated surface.

3. **Correct `AGENTS.md:286`** to match, so the next session does not
   inherit the overstatement and repeat it in something published.

4. ~~**Propose the claims register updates to the brain inbox**~~ **Done,
   3 September 2026.** On the evidence above, Paul set claims row 5 to
   `true`, moved row 7 to `conflict`, and closed SR-25 with the corrected
   premise. The watermark overclaim is item 6 of a site-update bundle
   awaiting him, so the two published pages are not corrected yet.

   **Row 6 is the one still open**, and it is the one this file is mainly
   about. It becomes `true` when the README says what is above and SR-24
   is closed. Steps 1 to 3 are what close it.

## What not to do

Do not delete the sentence and say nothing. "No data is uploaded" is a
real and unusual property of this product and it is worth stating; it just
has to be stated accurately.

Do not fix the README and leave the articles and services page. Rows 6 and 7
name three published pages. The correction is not done until they match.

## Update, 11 September 2026: row 5 is cheaper to close than the plan assumes

Row 5 of the table above is the Swagger page's unpinned fetch of
`https://unpkg.com/swagger-ui-dist/...` from
`src-tauri/src/api/mod.rs:278` and `:282`. "What would fix it" step 2 says
to bundle the assets locally, and `docs/release/v1.2.0-plan.md:105` prices
that at half a day as item A2.

Reading the API surface on 11 September found something that changes the
shape of the fix rather than the priority. **The crate whose entire purpose
is to serve those assets from inside the binary is already compiled into
every build and is never referenced.** `src-tauri/Cargo.toml:104` declares
`utoipa-swagger-ui`, `:116` puts it in the `api` feature and `:114` puts
`api` in `default`, and `grep -rn "utoipa_swagger_ui\|SwaggerUi" src-tauri/src/`
returns nothing. The page at `src-tauri/src/api/mod.rs:271-302` is a hand
written string literal that reaches the CDN instead.

So there are three options where the plan assumed one, and the third is the
smallest. Delete `swagger_ui_html`, `swagger_ui_redirect` and their two
routes, and the outbound call goes away entirely rather than being pinned,
an unauthenticated route disappears from a port that is always listening,
and a dependency that pulls `zip`, `rust-embed` and a second major version
of axum (`Cargo.lock:385`, `axum 0.7.9`, whose only dependent is that
crate) comes out of the build with it. `GET /openapi.json` is unaffected,
because it is produced by the other utoipa crate, which is genuinely used.

Written up as **BL-API-004**, which holds the dependency evidence. The
decision belongs to this item, because it is this item's row 5 that the
fix closes. Whoever takes A2 should read both.
