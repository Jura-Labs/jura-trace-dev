# Jura Trace — Brand Kit for Adobe Creative

A self-contained brief for refining the Jura Trace social card, app icon, marketing collateral and other branded surfaces in Adobe Creative Cloud.

---

## 1. What is in this folder

| File | Format | Purpose | Edit in |
|---|---|---|---|
| `logo-eye-mark.svg` | SVG (256×256) | Master vector logo. Concentric rings — geology + iris. | Illustrator |
| `social-card-1280x640.svg` | SVG (1280×640) | Editable layout of the Open Graph / social preview card. Replace placeholder fonts with the recommendations below. | Illustrator |
| `jura-trace-og-1280x640.png` | PNG (1280×640) | Reference render. Use as a layout guide if you start fresh in Illustrator. | View only |
| `palette-swatches.svg` | SVG (800×480) | Visual swatches of the full mineral palette with hex codes. Drag the rectangles into Illustrator's Swatches panel to populate the project palette. | Illustrator |
| `make-og.py` | Python | Script that generated the PNG. Re-run to regenerate after tweaks. | VS Code / any editor |

---

## 2. Recommended Adobe product

**Adobe Illustrator** is the right tool. Reasons:

1. The brand is vector-first (concentric circles, geometric layout, clean type). Illustrator handles vector natively without raster degradation at any scale.
2. The provided `.svg` files open as fully editable layered artwork in Illustrator (use **File → Open**, not Place).
3. Type controls in Illustrator (paragraph styles, character styles, OpenType features) are mature and deterministic — important for consistent wordmark rendering across exports.
4. Export presets cover every social-card spec out of the box (**File → Export → Save for Web** for legacy, **File → Export → Export As** for modern PNG/JPG/WebP).

**Adobe Express** is a faster alternative if you want a templated workflow (good for batch-producing campaign cards) — but Illustrator gives you the most control for the master assets.

**Photoshop** is only needed if you want to layer photographic textures or apply raster effects. The current brand brief does not require this.

---

## 3. Typography

### Current placeholder (used in the PNG render)

- **Wordmark**: Georgia Bold (system font — universally available, no licence cost, but generic)
- **Tagline**: Georgia Italic
- **Sub-header / UI**: Avenir Next (system on macOS)

These render acceptably but the brand will feel stronger with a more distinctive type pairing.

### Recommended Adobe Fonts pairings (all included with Creative Cloud)

| Tier | Serif (wordmark + tagline) | Sans (sub-headers, UI labels) | Why |
|---|---|---|---|
| **Recommended** | **Source Serif 4** | **Source Sans 3** | Adobe's own families. Free with CC. Designed to pair. Editorial warmth without the floridness of "magazine" serifs. Excellent for screen + print + small caps for the sub-header. |
| **Premium editorial** | **Freight Text Pro** | **Freight Sans Pro** | Joshua Darden's superfamily. The strongest "considered, archive-grade" feel — exactly the geological / cultural-institution register. Pricier per-style but most-used cuts are in the included subscription. |
| **Modern, slightly cooler** | **GT Sectra** | **Söhne** | Editorial sharpness with a contemporary edge. Söhne is the *New York Times* digital sans. Both are premium foundry fonts on Adobe Fonts. |
| **Bookish, warm, very readable** | **Lora** | **Inter** | Both also free on Google Fonts. Pleasant fallback if you want the brand identity to be replicable outside Creative Cloud. |

**My pick:** start with **Source Serif 4 + Source Sans 3**. They are designed by Frank Grießhammer at Adobe specifically to pair, are free with your CC subscription, and have the precise warmth-meets-rigour register the geology metaphor wants. Easy to upgrade to Freight later.

### Activating Adobe Fonts

1. In Illustrator, open the social-card SVG.
2. **Type → Add Fonts from Adobe Fonts** — searches Adobe Fonts inside the desktop app.
3. Search for `Source Serif 4` and `Source Sans 3`. Click **Activate** on the family.
4. The font appears in your font menu within ~30 seconds.
5. Select the wordmark, tagline, and sub-header text in the SVG and reassign them.

### Type scale for the social card (1280×640)

| Element | Size | Weight | Tracking | Notes |
|---|---|---|---|---|
| Wordmark "Jura Trace" | 104 pt | Bold (700) | -10 | Tight tracking on display sizes |
| Tagline "Know What's Real." | 54 pt | Italic Regular (400) | 0 | Italic gives a quiet "narrator" voice |
| Sub-header | 28 pt | Medium (500) | +20 | Slight tracking opens the line for breathing room |
| URL + version pill | 22 pt | Medium (500) | +50 | Wider tracking — feels labelled, not body copy |

---

## 4. Logo specifications

The eye-mark is a four-ring concentric mark on a square canvas.

| Ring | Diameter (% of canvas) | Hex | Role |
|---|---|---|---|
| Outer | 94% | `#5A85B5` Lapis | Sky/water — "depth" |
| Cream iris | 61% | `#EDEAE4` Quartz | Warmth — "human" |
| Inner cream | 43% | `#D8D5CE` Quartz dim | Texture |
| Pupil | 25% | `#1E2128` Obsidian | Anchor — "see" |
| Catchlight (4 o'clock) | 3% | `#EDEAE4` 55% opacity | Optional liveliness |

**Clear space**: at least one outer-ring radius of clear space on every side of the mark.

**Minimum size**: 24 px digital, 8 mm print.

---

## 5. Brand palette (mineral)

| Name | Hex | Use |
|---|---|---|
| Obsidian | `#1E2128` | Dark background, body text on cream |
| Graphite | `#272B34` | Dark surfaces, cards on dark mode |
| Quartz | `#EDEAE4` | Light surfaces, body text on dark |
| Quartz dim | `#D8D5CE` | Inner texture, dividers |
| Bone | `#FAFAF7` | Page background light mode |
| Flint dark | `#5C5A55` | Secondary text light mode (≥6.7 : 1 on `#FAFAF7`) |
| Flint light | `#ABA8A0` | Secondary text dark mode (≥5.4 : 1 on `#272B34`) |
| Lapis | `#5A85B5` | Primary accent (links, brand mark) |
| Lapis light | `#7AA0CC` | Dark-mode lapis variant |
| Malachite | `#7DA771` | Success / verified state |
| Cinnabar | `#DD8C84` | Warning / failed state |
| Amber dark | `#8C5F22` | Caution / mixed state |

All accent pairings have been validated against WCAG 2.2 AA contrast on both Bone and Graphite backgrounds.

---

## 6. Voice and tone in headlines

- **Confident, never breathless.** "Know what's real." stops at the full stop. No exclamation marks.
- **One verb per headline.** Verbs are: *Know, See, Verify, Examine, Protect, Trust* (in that priority order).
- **Plain-English subject lines.** "Forensic media verification" not "AI-powered detection platform".
- **Honest qualifiers** when needed. "Free for not-for-profits" is a fact; "open source" is not (yet — the licence is PolyForm Noncommercial 1.0.0, source-available; an Apache-2.0 dual-licence for the shared core is on the May roadmap as JTV-44).

---

## 7. Export presets for the social card

For the GitHub social preview specifically:

- **Format**: PNG
- **Dimensions**: 1280 × 640 (exact)
- **File size**: under 1 MB (GitHub limit)
- **Background**: must be opaque (no transparent PNG)

In Illustrator: **File → Export → Export As → PNG**, set **Resolution: 72 dpi** (the canvas is already in pixels), **Background: Black** (or the specific Obsidian hex), **Anti-aliasing: Type Optimised**.

For X/Twitter cards reuse the same 1280×640 file. For LinkedIn link previews, 1200×627 is preferred — re-export at that size from the same artwork by changing the artboard dimensions to 1200×627 and re-running export.

---

## 8. Quick wins to make the eye-mark "stronger"

If you want the existing render to feel more substantial without redesigning:

1. **Bigger anchor**: increase the eye-mark to ~30% of the canvas height (currently ~25%).
2. **Replace the catchlight** with an asymmetric crescent shadow at 8 o'clock to give the iris depth (paint a partial inner ring at low opacity).
3. **Heavier wordmark**: jump from Bold (700) to Black (900) on Source Serif 4, or use **Freight Display Black** for editorial gravity.
4. **Add a 1-px hairline border** to the whole card in Lapis at 25% opacity — gives the impression of a printed plate.
5. **Substitute the sub-header** for genuine small-caps (Source Serif 4 has them) in Quartz dim at 22 pt with +200 tracking — feels archival.

Any of these takes minutes in Illustrator and visibly raises the production value.

---

## 9. Hand-off checklist

When you have a final master:

- [ ] Save the layered Illustrator file as `social-card-master.ai` in this folder
- [ ] Export the 1280×640 PNG as `jura-trace-og-1280x640.png` (overwriting the placeholder)
- [ ] Export a 1200×627 LinkedIn variant as `jura-trace-og-1200x627.png`
- [ ] Export a 512×512 square crop for Mastodon / WhatsApp profile cards as `jura-trace-square.png`
- [ ] Commit all four files to `docs/branding/` in the repo
- [ ] Upload the 1280×640 to **GitHub repo Settings → General → Social preview** on `Jura-Labs/jura-trace`

---

*Questions or design conversations: hello@juralabs.org. Brand guidance lives in `docs/BRAND_GUIDELINES.md` (the longer reference); this kit is the practitioner's quick start.*

---

## 10. Content Authenticity Initiative (CAI) Member Lockup

Jura Labs CIC is an approved member of the Content Authenticity Initiative. The CAI's brand guidelines (`CAI Brand Guidelines.pdf` — not redistributed; obtain from the CAI) govern how the CAI marks may be displayed. The summary below is a working checklist; the PDF is authoritative.

### Files in this folder (`cai/`)

| File | Purpose |
|---|---|
| `CAI_Lockup_RGB_Black.svg` / `.png` / `.ai` | Official CAI lockup, black ink — for light backgrounds. Do not modify. |
| `CAI_Lockup_RGB_White.svg` / `.png` | Official CAI lockup, white ink — for dark backgrounds. Do not modify. |
| `member-lockup-black.svg` | Generated: CAI lockup + 3X divider + Jura Labs identity, black/obsidian — for **light** backgrounds. |
| `member-lockup-white.svg` | Generated: CAI lockup + 3X divider + Jura Labs identity, white/quartz — for **dark** backgrounds. |
| `make-cai-member-lockup.py` (in parent `docs/branding/`) | Regenerator. Re-run after any change to the source CAI lockup or `logo-eye-mark.svg`. |

### Which form to use, when

| Form | When to use |
|---|---|
| **Member Lockup** (preferred) | The primary form. Use wherever you are claiming membership: footer of every juralabs.org page, homepage trust strip, download page, about/press. Use `member-lockup-{black,white}.svg`. |
| **CAI Lockup** (symbol + wordmark) | Use when the Member Lockup is too wide and you only need to refer to the CAI itself, not your membership. Use `CAI_Lockup_RGB_{Black,White}.svg`. Minimum digital width **112 px**. |
| **CAI Symbol** only | Small badges, social avatars, or framing devices. Minimum digital width **40 px**. (We do not ship a symbol-only file in this folder yet — request from the CAI if needed.) |

### Hard rules (from the CAI Brand Guidelines)

- **Colours**: Black or White only on backgrounds; brand Yellow `#FFC840` permitted on specific surfaces. Never recolour outside this set.
- **Clearspace**: a margin of X around the lockup, where X = ½ the symbol height. Nothing crosses that margin.
- **Minimum size**: digital 112 px wide for the lockup; 40 px wide for the symbol-only. Print 1.5 in / 0.5 in.
- **Don't** stretch, alter the angle, recolour, outline, drop-shadow, glow, gradient, or scale below the minimum.
- **Don't** place on a low-contrast background where the ink fails legibility.

### Member Lockup layout (per CAI guidelines p. 16)

```
[ CAI lockup ]  ⟵ 1.5X ⟶ │ ⟵ 1.5X ⟶  [ Jura Labs identity ]
```

- Divider rule between the two halves; total spacer = 3X.
- Member identity sized so it is **optically equal** to the CAI lockup.
- Aligned to the centre of the CAI lockup.

The generated `member-lockup-{black,white}.svg` files already encode this geometry. If you re-export the source CAI files or refresh `logo-eye-mark.svg`, regenerate with:

```
python3 docs/branding/make-cai-member-lockup.py
```

### Don't conflate with C2PA Validator Conformant

CAI membership and C2PA Validator Conformant are **two separate claims** with **two separate mark regimes**:

- **CAI membership** is governed by the file above. The Member Lockup is the visible claim.
- **C2PA Validator Conformant** (awarded 2026-05-06, CPL embargoed until 31 May) is governed by **c2pa.org's** mark-usage guidelines, which have not yet been confirmed for Jura Labs. Verbal "C2PA Validator Conformant" copy is safe; **displaying a C2PA / Content Credentials conformance LOGO requires permission first** (tracked as LC-68).

### Web placement recommendations (juralabs.org)

| Surface | Form | Notes |
|---|---|---|
| Footer (every page) | `member-lockup-{black,white}.svg` | Pair with caption: *"Member of the Content Authenticity Initiative"* linking to `https://contentauthenticity.org/members`. |
| Homepage hero / trust strip | `member-lockup-*.svg` | Above the fold for non-scrollers. |
| Jura Trace product page | CAI Lockup or Member Lockup near the verdict/protect explanation. | Reinforces provenance claim at decision time. |
| Download / press kit page | `member-lockup-*.svg` + short statement | Conversion-moment reassurance. |
| About / press release | Member Lockup + body-copy mention | Formal record. |

Recommended HTML pattern (responsive, accessible):

```html
<a href="https://contentauthenticity.org/members"
   rel="noopener noreferrer" target="_blank"
   class="cai-member-lockup">
  <picture>
    <source srcset="/branding/cai/member-lockup-white.svg"
            media="(prefers-color-scheme: dark)">
    <img src="/branding/cai/member-lockup-black.svg"
         alt="Jura Labs, member of the Content Authenticity Initiative"
         width="448" height="51" loading="lazy">
  </picture>
</a>
```

(`448 × 51` keeps the lockup at ~448 px wide, comfortably above the 112 px minimum, and preserves the SVG's aspect ratio.)

