# BL-I18N-001: the app can only speak English

**Status**: Open. Target v1.3.0 for the groundwork and the process; the
translations themselves follow (Paul, 22 October 2026: "start process for
translations").
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: Medium. Much of the intended audience works in other
languages.

## What is wrong

- `ui/src/app.html:2` is `lang="en"`; `ui/package.json` has no
  internationalisation library. Every string is written inline in Svelte
  components, including long help pages.
- The review reports (IFCN state of fact-checkers 2025, published 2026)
  that 60.6% of fact-checkers publish in more than one language. Not
  re-read.
- Denmark is a home market, and its likeness bill (L 49) is a reason
  Danish publishers may need a tool like this.

## What would fix it

The process, in order:

1. Choose a library that suits Svelte 5 and SvelteKit's static build (for
   example Paraglide), and write the decision down.
2. Extract the strings of the core path first: first run (BL-UX-006),
   verify, the result and its bands, Content Credentials, the Verification
   Record export, Settings. Leave the long help pages in English for now.
3. Set up a translation workflow volunteers can use without Git (a hosted
   Weblate instance or similar, with the licence and data terms checked).
4. Write a glossary first: trust band names, "Content Credentials",
   "Local Signing", "Verification Record", so every language says the same
   thing. The band names and their meaning are the hardest part.
5. Pilot one language end to end. Suggested: Danish (home market, native
   reviewers to hand), then Spanish, French and Arabic for fact-checkers,
   which also tests right-to-left layout.
6. Rule: no machine translation shipped without a native speaker's review
   (voice.md, Denmark).

## What not to do

- Do not translate the API band names or JSON fields. They are a contract.
- Do not ship a partly translated result screen. A language is shown only
  when the core path is complete in it.
