# Jura Trace — Brand Guidelines

## Brand Identity

**Name**: Jura Trace
**Tagline**: "Know What's Real"
**Positioning**: "In a world of synthetic media, verification matters."
**Philosophy**: "Keep people at the heart of every decision. Use technology to support and guide, not to take over."
**Parent Organisation**: Juralabs CIC

## Logo

The Jura Trace logo is an **eye mark** — concentric circles representing observation, clarity, and the human at the centre of verification.

- **Outer circle**: Lapis blue (#5A85B5) — the act of looking, technology as a lens
- **Iris**: Warm cream (#EDEAE4) — clarity, truth, natural light
- **Inner iris**: Bone (#D8D5CE) — depth, a second layer of seeing
- **Pupil**: Deep obsidian (#1E2128) — focus, the human decision at the centre

The mark works at 16px (favicon), 24px (header), and 128px (hero). Always rendered with `aria-hidden="true"` alongside the text brand name.

## Brand Metaphor

**Minerals and Geology** — permanence, layers, provenance, deep time — expressed with **warmth and humanity**.

Where ROOTED uses organic/growth imagery (earth tones, moss, roots), Jura Trace uses geological imagery (obsidian, lapis, malachite, quartz). Both evoke the natural world but from different perspectives:

- ROOTED = living systems, growth, connection to place
- Jura Trace = deep time, permanence, layers of evidence, immutable record

The name "Jura" references the Jura mountains and the Jurassic geological period — layers of rock that preserve an accurate record of what existed. "Trace" speaks to both evidence (a trace of origin) and the act of following (tracing back to the source).

## Theme: Sanctuary

The visual system is called **Sanctuary** — warm, editorial, sheltering. Not a technical dashboard but a trusted space where people can examine evidence with care.

### Design Principles

1. **Generous whitespace** — the page must breathe. Max content width 900px.
2. **Georgia serif for headings** — editorial, not technical. Calm authority.
3. **Narrative over metrics** — stats are "quiet accomplishments", not KPI dashboards.
4. **Earth-line dividers** — subtle gradient stripes (brown, green, blue) between sections, evoking geological strata.
5. **Warm borders** — use rgba with opacity, not hard hex lines.
6. **Lowercase labels** — unhurried, confident, no shouting.
7. **Evidence first** — show data, not opinions. Trust scores backed by specific findings.
8. **Progressive disclosure** — summary first, details on demand.
9. **Low carbon** — target <500KB per page. System fonts only. No decorative images.

## Colour Palette

### Dark Mode (Default — Sanctuary)

| Name | Hex | Usage | WCAG vs #1E2128 |
|------|-----|-------|-----------------|
| Deep Obsidian | #1E2128 | Background | — |
| Graphite | #272B34 | Surface/cards | — |
| Quartz | #EDEAE4 | Primary text | AAA (12.8:1) |
| Flint | #9B9890 | Secondary text (dark mode) | AA (5.1:1) |
| Flint Dark | #78756D | Secondary text (light mode) | — |
| Lapis | #376399 | Accent / links (light mode) | — |
| Lapis Light | #5A85B5 | Accent / links (dark mode) | AA (4.6:1) |
| Malachite | #5B8A5F | Success / authentic / verified | AA (4.5:1) |
| Amber | #D4943A | Warning / review needed | AA (5.2:1) |
| Cinnabar | #C45B52 | Danger / manipulated / unverified | AA (4.6:1) |

### Light Mode

| Name | Hex | Usage |
|------|-----|-------|
| Warm White | #FAFAF7 | Background |
| White | #FFFFFF | Surface/cards |
| Warm Dark | #3A3832 | Primary text |
| Flint Dark | #78756D | Secondary text |
| Border | #E8E6E0 | Card borders |
| Lapis | #376399 | Accent / links (5.0:1 on white) |
| Malachite | #5B8A5F | Success |
| Amber Dark | #B87D2E | Warning |
| Cinnabar Dark | #A84840 | Danger |

### Semantic Colour Mapping

| Semantic | Colour | Meaning |
|----------|--------|---------|
| **Verified / Authentic** | Malachite (green) | Content has valid provenance, C2PA chain intact |
| **Review Needed** | Amber (yellow) | Inconclusive forensics, partial provenance, medium confidence |
| **Manipulated / Unverified** | Cinnabar (red) | Failed forensics, no provenance, high manipulation likelihood |
| **Informational / Action** | Lapis (blue) | Links, buttons, interactive elements, verification in progress |

### Earth Line

A thin gradient stripe used as a section divider, evoking geological strata:

```css
background: linear-gradient(90deg,
  transparent 0%, #5B4A3A 20%, #7A6B5A 40%,
  #5B7A5F 60%, #4A6A8A 80%, transparent 100%
);
opacity: 0.3;
```

## Typography

### Font Stack (System Fonts Only)

- **Headings**: Georgia, Times New Roman, DejaVu Serif, serif
- **Body**: -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Oxygen, Ubuntu, Cantarell, sans-serif
- **Monospace**: SF Mono, Menlo, Monaco, Courier New, monospace

### Spacing

| Context | Letter-spacing | Example |
|---------|---------------|---------|
| Brand name | 0.2em | J U R A  T R A C E |
| Navigation | 0.06em | P R O T E C T |
| Headings | -0.01em | Know What's Real |
| Buttons | 0.02em | Import Files |

### Brand Name Display

Always rendered in uppercase with wide letter-spacing:
```
JURA TRACE
```

Never: "jura trace", "Jura trace", "JURATRACE"

## Tone of Voice

### Principles

1. **Trustworthy**: Clear, factual, precise. No hype or marketing language.
2. **Calm authority**: Confident without being arrogant. Like a careful observer explaining what they see.
3. **Accessible**: Complex concepts in plain language. No jargon without explanation.
4. **Respectful**: Content creators and communities deserve control over their work.

### Language

- **British spelling** in all user-facing text: Organisation, Colour, Catalogue, Analyse
- **American spelling** in code: organization, color, catalog, analyze
- **No emojis** in the application interface
- **No exclamation marks** unless quoting user content
- **Human benefit first** — describe what people gain, not what the technology does

### Example Phrases

| Context | Good | Avoid |
|---------|------|-------|
| Verification result | "This image shows signs of manipulation in the lower-right region." | "WARNING: FAKE IMAGE DETECTED!" |
| C2PA status | "No C2PA provenance found. This doesn't mean the content is false — most content doesn't carry provenance yet." | "UNVERIFIED! No proof of origin!" |
| Welcome | "Jura Trace processes everything locally. Your files never leave this machine." | "We're SO excited to help you fight misinformation!" |
| Error | "Could not connect to the analysis services." | "Oops! Something went wrong!" |
| Sidecar status | "Analysis services connected" | "ML Sidecar: Online" |

## Accessibility

- **WCAG 2.2 AA minimum** on all pages
- **Lighthouse**: 97% accessibility, 100% best practices
- All interactive elements have visible `focus-visible` indicators
- All images have descriptive alt text or `aria-hidden="true"` (decorative)
- All form inputs have associated labels
- Minimum touch target: 44x44px
- Skip navigation link as first focusable element
- `aria-current="page"` on active navigation links
- `prefers-reduced-motion` respected globally
- Dark mode is the default (reduced eye strain for extended analysis)
- Light mode fully supported with WCAG AA contrast ratios
