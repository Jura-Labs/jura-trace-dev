import { test, expect } from '@playwright/test';

// All responsive tests use explicit viewports regardless of the project default,
// so that each case is unambiguous when running under either project configuration.

test.describe('Responsive behaviour — desktop (1280px)', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/');
  });

  test('hamburger button is hidden at desktop width', async ({ page }) => {
    const hamburger = page.getByRole('button', { name: /Open navigation menu|Close navigation menu/ });
    // The button exists in the DOM but is hidden via md:hidden / hidden classes
    await expect(hamburger).toBeHidden();
  });

  test('desktop nav is visible at desktop width', async ({ page }) => {
    const desktopNav = page.locator('nav[aria-label="Main navigation"]');
    await expect(desktopNav).toBeVisible();
  });
});

test.describe('Responsive behaviour — mobile (375px)', () => {
  test.use({ viewport: { width: 375, height: 812 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/');
  });

  test('desktop nav is hidden at mobile width', async ({ page }) => {
    const desktopNav = page.locator('nav[aria-label="Main navigation"]');
    await expect(desktopNav).toBeHidden();
  });

  test('hamburger button is visible at mobile width', async ({ page }) => {
    const hamburger = page.getByRole('button', { name: 'Open navigation menu' });
    await expect(hamburger).toBeVisible();
  });

  test('hamburger click opens mobile menu', async ({ page }) => {
    const hamburger = page.getByRole('button', { name: 'Open navigation menu' });
    await hamburger.click();

    const mobileMenu = page.locator('#mobile-menu');
    await expect(mobileMenu).toBeVisible();
  });

  test('mobile menu contains all nav items', async ({ page }) => {
    const hamburger = page.getByRole('button', { name: 'Open navigation menu' });
    await hamburger.click();

    const mobileNav = page.locator('#mobile-menu nav[aria-label="Mobile navigation"]');
    await expect(mobileNav.getByRole('link', { name: 'Dashboard' })).toBeVisible();
    await expect(mobileNav.getByRole('link', { name: 'Protect' })).toBeVisible();
    await expect(mobileNav.getByRole('link', { name: 'Verify' })).toBeVisible();
    await expect(mobileNav.getByRole('link', { name: 'Settings' })).toBeVisible();
  });

  test('mobile menu nav items have minimum 44px height', async ({ page }) => {
    const hamburger = page.getByRole('button', { name: 'Open navigation menu' });
    await hamburger.click();

    const mobileNav = page.locator('#mobile-menu nav[aria-label="Mobile navigation"]');
    const links = await mobileNav.getByRole('link').all();

    for (const link of links) {
      const box = await link.boundingBox();
      expect(box).not.toBeNull();
      expect(box!.height).toBeGreaterThanOrEqual(44);
    }
  });

  test('clicking a nav item closes the mobile menu', async ({ page }) => {
    const hamburger = page.getByRole('button', { name: 'Open navigation menu' });
    await hamburger.click();

    const mobileMenu = page.locator('#mobile-menu');
    await expect(mobileMenu).toBeVisible();

    // Click a nav item — this calls handleNavClick which closes the menu
    const mobileNav = page.locator('#mobile-menu nav[aria-label="Mobile navigation"]');
    await mobileNav.getByRole('link', { name: 'Protect' }).click();

    await expect(mobileMenu).toBeHidden();
  });
});
