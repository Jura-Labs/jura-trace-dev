# Jura Archive — Brand Guidelines

## Brand Identity

**Name**: Jura Archive
**Tagline**: "Know What's Real"
**Parent Organisation**: Juralabs CIC

## Brand Metaphor

**Minerals and Geology** — permanence, layers, provenance, deep time.

Where ROOTED uses organic/growth imagery (earth tones, moss, roots), Jura Archive uses geological imagery (obsidian, lapis, malachite, quartz). Both evoke the natural world but from different perspectives:

- ROOTED = living systems, growth, connection to place
- Jura Archive = deep time, permanence, layers of evidence, immutable record

The name "Jura" references the Jura mountains and the Jurassic geological period — layers of rock that preserve an accurate record of what existed. This directly maps to the product's purpose: preserving an accurate, layered, verifiable record of digital content.

## Colour Palette

### Dark Mode (Primary)

| Name | Hex | Usage | WCAG vs #1C1E26 |
|------|-----|-------|-----------------|
| Deep Obsidian | #1C1E26 | Background | — |
| Graphite | #2A2D37 | Surface/cards | — |
| Quartz | #E4E2DE | Primary text | AAA (13.2:1) |
| Flint | #9B9890 | Secondary text | AA (5.1:1) |
| Lapis | #4A7CBA | Accent / links / verification | AA (4.5:1) |
| Malachite | #5B9A6F | Success / authentic / verified | AA (4.8:1) |
| Amber | #D4943A | Warning / review needed | AA (5.2:1) |
| Cinnabar | #C45B52 | Danger / manipulated / unverified | AA (4.6:1) |

### Light Mode

| Name | Hex | Usage |
|------|-----|-------|
| Bone White | #F5F3EE | Background |
| White | #FFFFFF | Surface/cards |
| Deep Obsidian | #1C1E26 | Primary text |
| Flint | #9B9890 | Secondary text |
| Lapis Dark | #3A6499 | Accent / links |
| Malachite Dark | #478558 | Success |
| Amber Dark | #B87D2E | Warning |
| Cinnabar Dark | #A84840 | Danger |

### Semantic Colour Mapping

| Semantic | Colour | Meaning |
|----------|--------|---------|
| **Verified / Authentic** | Malachite (green) | Content has valid provenance, C2PA chain intact |
| **Review Needed** | Amber (yellow) | Inconclusive forensics, partial provenance, medium confidence |
| **Manipulated / Unverified** | Cinnabar (red) | Failed forensics, no provenance, high manipulation likelihood |
| **Informational / Action** | Lapis (blue) | Links, buttons, interactive elements, verification in progress |

## Typography

### Font Stack (System Fonts Only)

- **Headings**: Georgia, Times New Roman, DejaVu Serif, serif
- **Body**: -apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Oxygen, Ubuntu, Cantarell, sans-serif
- **Monospace**: SF Mono, Menlo, Monaco, Courier New, monospace

### Spacing

| Context | Letter-spacing | Example |
|---------|---------------|---------|
| Brand name | 0.15em | J U R A  A R C H I V E |
| Navigation | 0.05em | P R O T E C T |
| Headings | -0.01em | Know What's Real |
| Buttons | 0.02em | Import Files |

### Brand Name Display

Always rendered in uppercase with wide letter-spacing:
```
JURA ARCHIVE
```

Never: "jura archive", "Jura archive", "JURAARCHIVE"

## Tone of Voice

### Principles

1. **Trustworthy**: Clear, factual, precise. No hype or marketing language.
2. **Calm authority**: Confident without being arrogant. Like a forensics expert explaining findings.
3. **Accessible**: Complex concepts in plain language. No jargon without explanation.
4. **Respectful**: Content creators and communities deserve control over their work.

### Language

- **British spelling** in all user-facing text: Organisation, Colour, Catalogue, Analyse
- **American spelling** in code: organization, color, catalog, analyze
- **No emojis** in the application interface
- **No exclamation marks** unless quoting user content

### Example Phrases

| Context | Good | Avoid |
|---------|------|-------|
| Verification result | "This image shows signs of manipulation in the lower-right region." | "WARNING: FAKE IMAGE DETECTED!" |
| C2PA status | "No Content Credentials found. This doesn't mean the content is false — most content doesn't have them yet." | "UNVERIFIED! No proof of origin!" |
| Welcome | "Jura Archive processes everything locally. Your files never leave this machine." | "We're SO excited to help you fight misinformation!" |
| Error | "Could not connect to Ollama. Auto-cataloguing requires a running Ollama instance." | "Oops! Something went wrong!" |

## Accessibility

- **WCAG 2.2 AA minimum** on all pages
- All interactive elements have visible focus indicators
- All images have descriptive alt text
- All form inputs have associated labels
- Minimum touch target: 44x44px
- Dark mode is the default (reduced eye strain for extended forensic analysis)

## Design Principles

1. **Text-only interface**: No icons, no emojis. Clean typography, spacing, and colour convey hierarchy.
2. **Evidence first**: Show data, not opinions. Trust scores backed by specific forensic findings.
3. **Progressive disclosure**: Summary first, details on demand. Don't overwhelm.
4. **Low carbon**: Target <500KB per page. System fonts only. No decorative images.
