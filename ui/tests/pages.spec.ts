import { test, expect } from '@playwright/test';

test.describe('Page smoke tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test.describe('Dashboard (/)', () => {
    test('page loads successfully', async ({ page }) => {
      await page.goto('/');
      await expect(page).toHaveTitle("Jura Trace — Know What's Real");
      // No JS errors
    });

    test('shows four accomplishment statistics', async ({ page }) => {
      await page.goto('/');
      const statsSection = page.locator('section[aria-label="Summary statistics"]');
      await expect(statsSection).toBeVisible();

      // Four quiet accomplishments rendered as text-center divs
      const statItems = statsSection.locator('div.text-center');
      await expect(statItems).toHaveCount(4);
    });

    test('shows narrative chapters for Protect and Verify', async ({ page }) => {
      await page.goto('/');
      const chaptersSection = page.locator('section[aria-label="What you can do"]');
      await expect(chaptersSection).toBeVisible();

      await expect(chaptersSection.getByRole('link', { name: /Safeguard your content/i })).toBeVisible();
      await expect(chaptersSection.getByRole('link', { name: /Check what you're looking at/i })).toBeVisible();
    });
  });

  test.describe('Protect (/protect)', () => {
    test('page loads with h1 "Protect"', async ({ page }) => {
      await page.goto('/protect');
      await expect(page.locator('h1')).toHaveText('Protect');
    });

    test('shows drop zone for file import', async ({ page }) => {
      await page.goto('/protect');
      const dropZone = page.getByRole('button', { name: /drop files/i });
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

  test.describe('Monitor (/monitor)', () => {
    test('page loads with hero heading', async ({ page }) => {
      await page.goto('/monitor');
      const h1 = page.locator('h1');
      await expect(h1).toContainText('What has happened');
    });

    test('shows Protection Chronicle section', async ({ page }) => {
      await page.goto('/monitor');
      const chronicle = page.locator('#chronicle-heading');
      await expect(chronicle).toBeVisible();
      await expect(chronicle).toContainText('protected collection');
    });

    test('shows Trust Landscape section', async ({ page }) => {
      await page.goto('/monitor');
      const trustHeading = page.locator('#trust-heading');
      await expect(trustHeading).toBeVisible();
      await expect(trustHeading).toContainText('verified content');
    });

    test('shows Activity Record section', async ({ page }) => {
      await page.goto('/monitor');
      const activityHeading = page.locator('#activity-heading');
      await expect(activityHeading).toBeVisible();
      await expect(activityHeading).toContainText('happened here');
    });

    test('activity filter buttons are present and operable', async ({ page }) => {
      await page.goto('/monitor');
      const filterGroup = page.locator('[role="group"][aria-label="Filter activity by action type"]');
      await expect(filterGroup).toBeVisible();

      // All four filter tabs present
      await expect(filterGroup.getByRole('button', { name: 'All' })).toBeVisible();
      await expect(filterGroup.getByRole('button', { name: 'Import' })).toBeVisible();
      await expect(filterGroup.getByRole('button', { name: 'Verify' })).toBeVisible();
      await expect(filterGroup.getByRole('button', { name: 'Sign' })).toBeVisible();

      // All is pressed by default
      const allBtn = filterGroup.getByRole('button', { name: 'All' });
      await expect(allBtn).toHaveAttribute('aria-pressed', 'true');
    });

    test('Monitor link appears in dashboard narrative chapters', async ({ page }) => {
      await page.goto('/');
      const chaptersSection = page.locator('section[aria-label="What you can do"]');
      await expect(chaptersSection.getByRole('link', { name: /See what has happened/i })).toBeVisible();
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

  test.describe('Help: Bedrock Signing (/help/bedrock-signing)', () => {
    test('bedrock-signing help page loads', async ({ page }) => {
      await page.goto('/help/bedrock-signing');
      await expect(page.locator('h1')).toContainText('Bedrock');
    });
  });
});
