# BL-UX-006: first run is four screens and no sample image

**Status**: Open. Target v1.3.0, to implement (Paul, 22 October 2026).
**Raised**: 22 October 2026, from the pre-v1.3.0 adoption review.
**Severity**: Medium. A new user has to get through several screens before
they can check anything, and has nothing to check unless they bring a file.

## What is wrong

On first launch (`ui/src/routes/+layout.svelte:160-185`):

1. `OnboardingOverlay` shows, because `jura-onboarded` is unset.
2. Then `SetupWizard`, until `jura-setup-complete` is set.
3. Then, on a later launch, a sidecar-offline reminder if the analysis
   engine is slow to answer.
4. The dashboard then shows its first-run card (`ui/src/routes/+page.svelte:44,130`,
   when there are no assets and no verifications) and the release-news
   card.

The four practice images, which show what each trust band looks like, are
only on the website (BL-DOC-004). Nothing in the app lets a user try a
check without finding their own file.

## What would fix it

1. One first-run screen: what Trace does in two sentences, the network
   mode choice the wizard asks today, and a single button, "Try a sample
   image".
2. Ship the four practice images in the app (they are small next to the
   installer). The button runs a real check on one of them and opens the
   result, with the band explained.
3. Fold the remaining wizard steps into Settings, with defaults that need
   no answer.
4. Keep the sidecar reminder, but only after a real check has failed to
   reach the engine, not on a timer.
5. Keep the release-news card, after the first check rather than before.

## What not to do

- Do not drop the network mode choice. It is the one first-run decision
  with a privacy consequence.
- Do not count the sample check in any statistics shown to the user.
