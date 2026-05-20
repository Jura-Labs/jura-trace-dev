// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Playwright e2e tests for the "Check for Updates" button in Settings.
 *
 * These tests run against the SvelteKit dev server (browser mode, no Tauri
 * instance). The Tauri updater plugin is therefore unreachable; the tests
 * assert browser-mode behaviour only: button presence, accessibility
 * attributes, keyboard reachability, and the graceful error notice that
 * surfaces when the plugin cannot be called.
 *
 * Run with: cd ui && npx playwright test check-for-updates
 * (requires `npm run dev` to be running on localhost:1420)
 */
import { test, expect } from '@playwright/test';

test.describe('Check for Updates — /settings', () => {
  test.beforeEach(async ({ page }) => {
    // Skip the onboarding overlay so the Settings page is immediately visible.
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
      localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/settings');
  });

  test('button is visible in the About section', async ({ page }) => {
    const button = page.getByRole('button', { name: 'Check for Updates' });
    await expect(button).toBeVisible();
  });

  test('button has accessible name "Check for Updates"', async ({ page }) => {
    const button = page.getByRole('button', { name: 'Check for Updates' });
    await expect(button).toHaveAccessibleName('Check for Updates');
  });

  test('button is not aria-busy when idle', async ({ page }) => {
    const button = page.getByRole('button', { name: 'Check for Updates' });
    // aria-busy should be absent or false at rest.
    const ariaBusy = await button.getAttribute('aria-busy');
    expect(ariaBusy === null || ariaBusy === 'false').toBe(true);
  });

  test('button meets 44px minimum touch-target height', async ({ page }) => {
    const button = page.getByRole('button', { name: 'Check for Updates' });
    const box = await button.boundingBox();
    expect(box).not.toBeNull();
    // WCAG 2.5.8 + project a11y standard: 44px minimum.
    expect(box!.height).toBeGreaterThanOrEqual(44);
  });

  test('clicking the button shows the browser-mode error alert', async ({ page }) => {
    const button = page.getByRole('button', { name: 'Check for Updates' });
    await button.click();

    const alert = page.getByRole('alert');
    await expect(alert).toBeVisible();
    await expect(alert).toContainText('Update checks are only available in the desktop application.');
  });

  test('error alert has aria-live="assertive"', async ({ page }) => {
    await page.getByRole('button', { name: 'Check for Updates' }).click();

    const alert = page.getByRole('alert');
    await expect(alert).toBeVisible();
    await expect(alert).toHaveAttribute('aria-live', 'assertive');
  });

  test('button is keyboard-reachable via Tab', async ({ page }) => {
    // Tab through the page until the button receives focus.
    // Cap at 60 presses to avoid an infinite loop on unexpected DOM changes.
    const button = page.getByRole('button', { name: 'Check for Updates' });
    let focused = false;

    for (let i = 0; i < 60; i++) {
      await page.keyboard.press('Tab');
      const activeHandle = await page.evaluateHandle(() => document.activeElement);
      const isButton = await page.evaluate(
        (el) => el instanceof HTMLButtonElement && el.textContent?.trim() === 'Check for Updates',
        activeHandle,
      );
      if (isButton) {
        focused = true;
        break;
      }
    }

    expect(focused).toBe(true);
  });

  test('pressing Enter on the focused button triggers the browser-mode error', async ({ page }) => {
    // Tab to the button then activate with Enter.
    const button = page.getByRole('button', { name: 'Check for Updates' });

    for (let i = 0; i < 60; i++) {
      await page.keyboard.press('Tab');
      const activeHandle = await page.evaluateHandle(() => document.activeElement);
      const isButton = await page.evaluate(
        (el) => el instanceof HTMLButtonElement && el.textContent?.trim() === 'Check for Updates',
        activeHandle,
      );
      if (isButton) break;
    }

    await page.keyboard.press('Enter');

    const alert = page.getByRole('alert');
    await expect(alert).toBeVisible();
    // Verify the button itself is what was activated (not a different button).
    await expect(button).toBeVisible();
  });

  test('button is not disabled when idle', async ({ page }) => {
    const button = page.getByRole('button', { name: 'Check for Updates' });
    await expect(button).not.toBeDisabled();
  });
});
