// SPDX-License-Identifier: AGPL-3.0-or-later

import { test, expect } from '@playwright/test';

test.describe('Navigation and layout', () => {
  test.beforeEach(async ({ page }) => {
    // Suppress onboarding overlay so it does not interfere with layout tests
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/');
  });

  test('page loads with title "Jura Trace"', async ({ page }) => {
    await expect(page).toHaveTitle("Jura Trace — Know What's Real");
  });

  test('logo mark SVG is visible in header', async ({ page }) => {
    const headerBrand = page.locator('header a[aria-label="Jura Trace — home"]');
    await expect(headerBrand).toBeVisible();

    const logoSvg = headerBrand.locator('svg');
    await expect(logoSvg).toBeVisible();
  });

  test('skip link exists and is visually hidden until focused', async ({ page }) => {
    const skipLink = page.getByRole('link', { name: 'Skip to main content' });
    await expect(skipLink).toBeAttached();

    // Before focus: sr-only clips to 1×1 px
    const box = await skipLink.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeLessThanOrEqual(1);

    // After focus it becomes visible
    await skipLink.focus();
    const focusedBox = await skipLink.boundingBox();
    expect(focusedBox!.width).toBeGreaterThan(1);
    expect(focusedBox!.height).toBeGreaterThan(1);
  });

  test('footer contains Jura Labs CIC link', async ({ page }) => {
    const footer = page.locator('footer');
    const juralabsLink = footer.getByRole('link', { name: 'Jura Labs CIC' });
    await expect(juralabsLink).toBeVisible();
    await expect(juralabsLink).toHaveAttribute('href', 'https://juralabs.org');
  });

  test('footer contains Licences link', async ({ page }) => {
    // The About link was dropped from the footer in 176eef7 (2026-05-26);
    // the surviving footer links are Jura Labs CIC (tested above) and
    // Licences.
    const footer = page.locator('footer');
    const licencesLink = footer.getByRole('link', { name: 'Licences' });
    await expect(licencesLink).toBeVisible();
    await expect(licencesLink).toHaveAttribute('href', '/help/open-source');
  });

  test('footer contains version string', async ({ page }) => {
    const footer = page.locator('footer');
    await expect(footer).toContainText(/v\d+\.\d+\.\d+/);
  });
});

// These tests target desktop-specific UI elements that are hidden on mobile.
test.describe('Navigation and layout — desktop only', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/');
  });

  test('all four nav links present in desktop nav', async ({ page }) => {
    const desktopNav = page.locator('nav[aria-label="Main navigation"]');
    await expect(desktopNav.getByRole('link', { name: 'Dashboard' })).toBeVisible();
    await expect(desktopNav.getByRole('link', { name: 'Protect' })).toBeVisible();
    await expect(desktopNav.getByRole('link', { name: 'Verify' })).toBeVisible();
    await expect(desktopNav.getByRole('link', { name: 'Settings' })).toBeVisible();
  });

  test('dark mode toggle changes class on html element', async ({ page }) => {
    // Desktop toggle lives inside nav[aria-label="Main navigation"]
    const toggle = page
      .locator('nav[aria-label="Main navigation"]')
      .getByRole('button', { name: /Switch to (light|dark) mode/ });

    const html = page.locator('html');

    // Default is dark
    await expect(html).toHaveClass(/dark/);

    // Toggle to light
    await toggle.click();
    await expect(html).not.toHaveClass(/dark/);

    // Toggle back to dark
    await toggle.click();
    await expect(html).toHaveClass(/dark/);
  });

  test('active nav link has aria-current="page"', async ({ page }) => {
    // Dashboard is active on '/'
    const dashLink = page
      .locator('nav[aria-label="Main navigation"]')
      .getByRole('link', { name: 'Dashboard' });
    await expect(dashLink).toHaveAttribute('aria-current', 'page');

    // Other links should NOT have aria-current
    const protectLink = page
      .locator('nav[aria-label="Main navigation"]')
      .getByRole('link', { name: 'Protect' });
    await expect(protectLink).not.toHaveAttribute('aria-current', 'page');
  });
});
