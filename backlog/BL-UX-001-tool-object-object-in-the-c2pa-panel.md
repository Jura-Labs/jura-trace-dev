# BL-UX-001: the C2PA panel shows "Tool: [object Object]"

**Status**: Open. Found in the shipped v1.0.0 on 21 June 2026 (Plane
JTV-248); confirmed still present in the code on 3 September 2026.
**Raised**: 3 September 2026
**Severity**: Low to fix, high to be seen. It appears in the one panel that
is the product's proof of competence.

## What is wrong

`ui/src/routes/verify/+page.svelte:3583`:

```svelte
Tool: {action.softwareAgent}
```

`softwareAgent` is typed as a `string` at `ui/src/lib/types.ts:954` and
parsed as one at `verify/+page.svelte:628-634`:

```ts
const actions: { action: string; description?: string; digitalSourceType?: string; softwareAgent?: string }[] = ...
softwareAgent: a.softwareAgent ?? null,
```

In C2PA 2.x an action's `softwareAgent` is a `GeneratorInfo` object, with
`name` and `version` and optional further fields, not a bare string. It was
a string in earlier specification versions. When a manifest uses the object
form, Svelte interpolates it and the user reads:

```
Tool: [object Object]
```

## Why it matters more than a formatting slip

Jura Trace is on the C2PA conforming products list as a Validator, spec
2.2. The manifests most likely to trigger this are exactly the ones written
by other conformant 2.x tools, which is to say the manifests a
professional user is most likely to test it against first, and the ones a
C2PA implementer would use when looking at what Jura Labs built.

The panel is also the part of the product that says "we read provenance
carefully". A rendering bug there costs more credibility than the same bug
anywhere else in the application.

## What would fix it

1. Widen the type to `string | { name?: string; version?: string }` and
   render accordingly: `name` alone, or `name` and `version` joined, and
   fall back to the string form when that is what arrived. Handle the
   absent case as it is handled now.
2. Do the same audit on the sibling fields in that block. `description` and
   `digitalSourceType` are read from the same parsed action and the same
   assumption may be wrong for one of them.
3. Add a fixture manifest with an object-form `softwareAgent` to the
   frontend tests, so the string assumption cannot come back.

## What not to do

Do not fix it with `String(action.softwareAgent)` or a `typeof` check that
prints JSON. Showing `{"name":"Adobe Photoshop","version":"25.0"}` to a
user is the same failure with more characters.

## Update, 16 September 2026

Fixed on branch `fix/bl-ux-001-software-agent-object`, v1.2.0 item A5.
Two corrections to this file, found while fixing it:

- **Jura Trace triggers it on its own files.** `src-tauri/src/c2pa.rs:500`
  signs every action with `softwareAgent` as an object (`name`, `version`,
  `operating_system`), so every file sealed in Protect showed
  "Tool: [object Object]", not only files from other 2.x tools. The live
  getting-started page publishes a screenshot of it (`jt-signed-credentials.png`).
- **`types.ts:954` is the wrong type to widen.** It is `XmpHistoryEvent`,
  the XMP edit history, where `stEvt:softwareAgent` really is a string. The
  C2PA action type is inline in `verify/+page.svelte`, and that is what
  changed.

Sibling fields (step 2): `description` and `digitalSourceType` are strings
in the specification, but a manifest is external input. A non-string
`digitalSourceType` reached `humaniseDigitalSourceType`, whose `.replace`
threw, and the surrounding `catch` returned an empty list, hiding every
action in the panel without a word. Both are now read only when they are
strings. The fix lives in `formatSoftwareAgent` in `ui/src/lib/c2pa-labels.ts`,
pinned by `c2pa-labels.test.ts` using the exact shape Trace signs with.
