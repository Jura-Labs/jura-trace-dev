// SPDX-License-Identifier: AGPL-3.0-or-later

import { describe, expect, it, vi } from 'vitest';
import {
  NEWSLETTER_BODY,
  NEWSLETTER_BUTTON_LABEL,
  NEWSLETTER_CARD_DISMISSED_KEY,
  NEWSLETTER_DISMISS_LABEL,
  NEWSLETTER_HEADING,
  NEWSLETTER_URL,
  dismissNewsletterCard,
  isNewsletterCardDismissed,
  openNewsletterSignup,
  type FlagStorage,
} from './newsletter';

function memoryStorage(): FlagStorage & { data: Map<string, string> } {
  const data = new Map<string, string>();
  return {
    data,
    getItem: (k) => data.get(k) ?? null,
    setItem: (k, v) => {
      data.set(k, v);
    },
  };
}

const throwingStorage: FlagStorage = {
  getItem: () => {
    throw new Error('denied');
  },
  setItem: () => {
    throw new Error('denied');
  },
};

describe('NEWSLETTER_URL', () => {
  it('points at the website signup page with only the source parameter', () => {
    const url = new URL(NEWSLETTER_URL);
    expect(url.protocol).toBe('https:');
    expect(url.host).toBe('juralabs.org');
    expect(url.pathname).toBe('/newsletter');
    expect([...url.searchParams.keys()]).toEqual(['source']);
    expect(url.searchParams.get('source')).toBe('trace-app');
  });
});

describe('newsletter copy', () => {
  const all = [NEWSLETTER_HEADING, NEWSLETTER_BODY, NEWSLETTER_BUTTON_LABEL, NEWSLETTER_DISMISS_LABEL].join(' ');

  it('follows the house style', () => {
    expect(all).not.toMatch(/—/);
    expect(all).not.toContain('!');
    expect(all).not.toMatch(/seamless|powerful|robust|empower|unlock|journey|excited/i);
    expect(all).not.toMatch(/\b(verify|prove|guarantee)/i);
  });

  it('is honest that this is the newsletter, not update notices only', () => {
    expect(NEWSLETTER_BODY).toContain('news from Jura Labs');
    expect(NEWSLETTER_BODY).toContain('never sees your email address');
  });
});

describe('Dashboard card dismissal', () => {
  it('is not dismissed on a fresh install', () => {
    expect(isNewsletterCardDismissed(memoryStorage())).toBe(false);
  });

  it('stays dismissed once dismissed', () => {
    const s = memoryStorage();
    dismissNewsletterCard(s);
    expect(s.data.get(NEWSLETTER_CARD_DISMISSED_KEY)).toBe('true');
    expect(isNewsletterCardDismissed(s)).toBe(true);
  });

  it('stores only a flag, nothing personal', () => {
    const s = memoryStorage();
    dismissNewsletterCard(s);
    expect([...s.data.entries()]).toEqual([[NEWSLETTER_CARD_DISMISSED_KEY, 'true']]);
  });

  it('hides the card when storage is unavailable, so it can never nag', () => {
    expect(isNewsletterCardDismissed(throwingStorage)).toBe(true);
    expect(isNewsletterCardDismissed(null)).toBe(true);
    expect(() => dismissNewsletterCard(throwingStorage)).not.toThrow();
    expect(() => dismissNewsletterCard(undefined)).not.toThrow();
  });
});

describe('openNewsletterSignup', () => {
  it('opens the URL through the shell in the desktop app', async () => {
    const shellOpen = vi.fn(async () => undefined);
    const windowOpen = vi.fn();
    await openNewsletterSignup({ isTauri: () => true, shellOpen, windowOpen });
    expect(shellOpen).toHaveBeenCalledWith(NEWSLETTER_URL);
    expect(windowOpen).not.toHaveBeenCalled();
  });

  it('uses window.open outside the desktop app', async () => {
    const shellOpen = vi.fn(async () => undefined);
    const windowOpen = vi.fn();
    await openNewsletterSignup({ isTauri: () => false, shellOpen, windowOpen });
    expect(shellOpen).not.toHaveBeenCalled();
    expect(windowOpen).toHaveBeenCalledWith(NEWSLETTER_URL);
  });

  it('falls back to window.open if the shell fails', async () => {
    const shellOpen = vi.fn(async () => {
      throw new Error('scope');
    });
    const windowOpen = vi.fn();
    await openNewsletterSignup({ isTauri: () => true, shellOpen, windowOpen });
    expect(windowOpen).toHaveBeenCalledWith(NEWSLETTER_URL);
  });
});
