// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Newsletter opt-in for Jura Trace (W38-3, v1.2.0).
 *
 * Someone who has just installed the app is the most qualified person to
 * hear about new releases, and until now the only route to the newsletter
 * was a footer link on the website.
 *
 * What this module deliberately does NOT do:
 *
 *   - It never collects, stores or transmits an email address or any other
 *     personal data. The only action is opening the website signup page in
 *     the user's default browser. The signup itself happens on juralabs.org,
 *     outside the app.
 *   - It adds no tracking. `source=trace-app` tells the website where the
 *     visit came from and nothing else. Do not add UTM or other parameters.
 *   - It does not consult the network mode. Standard and Enhanced modes
 *     govern outbound traffic that the app makes itself; opening a page in
 *     the user's browser, after the user pressed a button, is a user action
 *     outside the app, so it behaves identically in both modes.
 *
 * The only local state is a single dismissal flag for the one-time Dashboard
 * card, stored in localStorage alongside the other one-time UI flags such as
 * `jura-verify-intro-dismissed`. It holds the string "true" and nothing else.
 */

export const NEWSLETTER_URL = 'https://juralabs.org/newsletter?source=trace-app';

export const NEWSLETTER_HEADING = 'Hear when a new version is out';

export const NEWSLETTER_BODY =
  'Jura Trace checks for updates itself. If you would also like an email ' +
  'when a release ships, with occasional news from Jura Labs, you can sign ' +
  'up on our website. The app never sees your email address.';

export const NEWSLETTER_BUTTON_LABEL = 'Sign up on juralabs.org';

export const NEWSLETTER_DISMISS_LABEL = 'No thanks';

/** localStorage key for the one-time Dashboard card. */
export const NEWSLETTER_CARD_DISMISSED_KEY = 'jura-newsletter-card-dismissed';

/** The subset of Storage this module needs, so tests can pass a fake. */
export type FlagStorage = Pick<Storage, 'getItem' | 'setItem'>;

/**
 * Whether the Dashboard card has been dismissed.
 *
 * If storage cannot be read (it throws in some privacy configurations) the
 * card is treated as dismissed. Without working storage the dismissal could
 * never be remembered, and a card that reappears on every launch is exactly
 * the nagging this feature promises not to do. Settings still carries the
 * permanent link.
 */
export function isNewsletterCardDismissed(storage: FlagStorage | null | undefined): boolean {
  if (!storage) return true;
  try {
    return storage.getItem(NEWSLETTER_CARD_DISMISSED_KEY) === 'true';
  } catch {
    return true;
  }
}

/** Remember that the Dashboard card was dismissed. Never throws. */
export function dismissNewsletterCard(storage: FlagStorage | null | undefined): void {
  if (!storage) return;
  try {
    storage.setItem(NEWSLETTER_CARD_DISMISSED_KEY, 'true');
  } catch {
    // Storage unavailable; isNewsletterCardDismissed already hides the card
    // in that case, so there is nothing further to do.
  }
}

export interface OpenNewsletterDeps {
  isTauri: () => boolean;
  /** Opens a URL in the default browser (tauri-plugin-shell `open`). */
  shellOpen: (url: string) => Promise<void>;
  /** Browser fallback, used outside the desktop app or if the shell fails. */
  windowOpen: (url: string) => void;
}

/**
 * Open the newsletter signup page in the user's default browser.
 *
 * Mirrors how Settings opens ollama.com: tauri-plugin-shell `open` inside
 * the desktop app, `window.open` otherwise or if the shell call fails.
 */
export async function openNewsletterSignup(deps?: OpenNewsletterDeps): Promise<void> {
  const d = deps ?? defaultOpenDeps();
  try {
    if (d.isTauri()) {
      await d.shellOpen(NEWSLETTER_URL);
      return;
    }
  } catch {
    // Fall through to the browser fallback.
  }
  d.windowOpen(NEWSLETTER_URL);
}

function defaultOpenDeps(): OpenNewsletterDeps {
  return {
    isTauri: () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window,
    shellOpen: async (url) => {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
    },
    windowOpen: (url) => {
      window.open(url, '_blank', 'noopener,noreferrer');
    },
  };
}
