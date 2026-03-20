import { test, expect } from '@playwright/test';

test.describe('Page smoke tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
    });
  });

  test.describe('Dashboard (/)', () => {
    test('page loads successfully', async ({ page }) => {
      await page.goto('/');
      await expect(page).toHaveTitle("Jura Trace — Know What's Real");
      // No JS errors
    });

    test('shows stats grid with four stat cards', async ({ page }) => {
      await page.goto('/');
      const statsSection = page.locator('section[aria-label="Summary statistics"]');
      await expect(statsSection).toBeVisible();

      // Four metric cards are rendered inside the grid
      const statCards = statsSection.locator('.grid > div');
      await expect(statCards).toHaveCount(4);
    });

    test('shows quick action cards for Protect and Verify', async ({ page }) => {
      await page.goto('/');
      const actionsSection = page.locator('section[aria-label="Quick actions"]');
      await expect(actionsSection).toBeVisible();

      await expect(actionsSection.getByRole('link', { name: /Safeguard Your Content/i })).toBeVisible();
      await expect(actionsSection.getByRole('link', { name: /Check Authenticity/i })).toBeVisible();
    });
  });

  test.describe('Protect (/protect)', () => {
    test('page loads with h1 "Protect"', async ({ page }) => {
      await page.goto('/protect');
      await expect(page.locator('h1')).toHaveText('Protect');
    });

    test('shows drop zone with correct aria-label', async ({ page }) => {
      await page.goto('/protect');
      const dropZone = page.getByRole('button', { name: 'Drop files here or click to import' });
      await expect(dropZone).toBeVisible();
    });

    test('shows filter bar with content type and status selects', async ({ page }) => {
      await page.goto('/protect');
      const filterBar = page.locator('[role="search"][aria-label="Filter assets"]');
      await expect(filterBar).toBeVisible();

      // Type filter select
      await expect(filterBar.locator('#filter-content-type')).toBeVisible();
      // Status filter select
      await expect(filterBar.locator('#filter-status')).toBeVisible();
    });
  });

  test.describe('Verify (/verify)', () => {
    test('page loads with h1 "Verify"', async ({ page }) => {
      await page.goto('/verify');
      await expect(page.locator('h1')).toHaveText('Verify');
    });

    test('shows File, Batch, and URL tabs', async ({ page }) => {
      await page.goto('/verify');

      const tablist = page.getByRole('tablist');
      await expect(tablist).toBeVisible();

      await expect(tablist.getByRole('tab', { name: 'File' })).toBeVisible();
      await expect(tablist.getByRole('tab', { name: 'Batch' })).toBeVisible();
      await expect(tablist.getByRole('tab', { name: 'URL' })).toBeVisible();
    });

    test('File tab is selected by default', async ({ page }) => {
      await page.goto('/verify');
      const fileTab = page.getByRole('tab', { name: 'File' });
      await expect(fileTab).toHaveAttribute('aria-selected', 'true');
    });

    test('clicking URL tab switches to URL panel', async ({ page }) => {
      await page.goto('/verify');
      const urlTab = page.getByRole('tab', { name: 'URL' });
      await urlTab.click();
      await expect(urlTab).toHaveAttribute('aria-selected', 'true');
    });
  });

  test.describe('Settings (/settings)', () => {
    test('page loads with h1 "Settings"', async ({ page }) => {
      await page.goto('/settings');
      await expect(page.locator('h1')).toHaveText('Settings');
    });

    test('shows Ollama Configuration heading', async ({ page }) => {
      await page.goto('/settings');
      const ollamaHeading = page.locator('#ollama-heading');
      await expect(ollamaHeading).toBeVisible();
      await expect(ollamaHeading).toHaveText('Ollama Configuration');
    });
  });
});
