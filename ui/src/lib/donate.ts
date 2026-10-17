// Donation surface constants for Jura Trace.
//
// Decision document:
//   jura-labs-docs/jura-trace-strategy/donation-surface-design-2026-06-02.md
//
// Primary mechanism: Stripe Payment Link. Open Collective will be added as
// a secondary in-app link in v1.0.1 once JTV-166 fiscal-host setup is
// verified live. GitHub Sponsors is intentionally not surfaced in-app
// because it locks out the many Jura Trace users without a GitHub account.
//
// Copy uses the structural-framing variant per the agent recommendation
// (passive, mission-and-structure language, not an explicit "please donate"
// ask) because the journalist, fact-checker, and human-rights audience
// responds better to it. British spelling, no em dashes, no sentence-
// breaking hyphens per the saved feedback memo.

export const DONATE_URL =
  'https://buy.stripe.com/cNibJ19g473m3v1cj88og00';

export const DONATE_HEADING = 'Support development';

export const DONATE_BODY_SETTINGS =
  'Jura Labs CIC is an asset-locked non-profit: any surplus must be ' +
  'reinvested in the mission. Jura Trace will always be free. Donations ' +
  'fund corpus expansion, calibration work, and ongoing open-source ' +
  'maintenance.';

export const DONATE_BODY_HELP =
  'Jura Labs CIC is an asset-locked non-profit. Surplus cannot be paid ' +
  'to shareholders. Contributions fund model retraining, corpus ' +
  'expansion, and keeping Jura Trace free for public-interest users ' +
  'worldwide.';

export const DONATE_BUTTON_LABEL = 'Contribute via Stripe';

export const DONATE_FOOTER_LABEL = 'Support';
