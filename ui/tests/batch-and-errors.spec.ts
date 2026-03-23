import { test, expect } from '@playwright/test';

test.describe('Verify error states', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
    });
    await page.goto('/verify');
    await page.waitForSelector('h1');
    // Wait for reactivity to settle
    await page.waitForTimeout(300);
  });

  test('shows sidecar error with amber styling', async ({ page }) => {
    await page.evaluate(() => {
      (window as any).__juraSetVerifyError('sidecar connection refused');
    });
    const alert = page.locator('[role="alert"]');
    await expect(alert).toBeVisible();
    await expect(alert).toContainText('Sidecar offline');
    await expect(alert).toContainText('Analysis services are not running');
  });

  test('shows format error with lapis styling', async ({ page }) => {
    await page.evaluate(() => {
      (window as any).__juraSetVerifyError('unsupported format');
    });
    const alert = page.locator('[role="alert"]');
    await expect(alert).toBeVisible();
    await expect(alert).toContainText('Unsupported format');
    await expect(alert).toContainText('JPEG, PNG, TIFF');
  });

  test('shows network error for URL failures', async ({ page }) => {
    await page.evaluate(() => {
      (window as any).__juraSetVerifyError('fetch failed');
    });
    const alert = page.locator('[role="alert"]');
    await expect(alert).toBeVisible();
    await expect(alert).toContainText('Network error');
    await expect(alert).toContainText('Check the address');
  });

  test('shows general error for unknown failures', async ({ page }) => {
    await page.evaluate(() => {
      (window as any).__juraSetVerifyError('Tauri not available');
    });
    const alert = page.locator('[role="alert"]');
    await expect(alert).toBeVisible();
    await expect(alert).toContainText('Error');
    await expect(alert).toContainText('application bridge unavailable');
  });
});

test.describe('Verify batch tab', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
    });
    await page.goto('/verify');
    await page.waitForSelector('h1');
  });

  test('batch tab is present and switchable', async ({ page }) => {
    const batchTab = page.getByRole('tab', { name: 'Batch' });
    await expect(batchTab).toBeVisible();
    await batchTab.click();
    await expect(batchTab).toHaveAttribute('aria-selected', 'true');
  });

  test('batch tab shows drop zone', async ({ page }) => {
    await page.getByRole('tab', { name: 'Batch' }).click();
    const dropZone = page.locator('button[aria-label*="Drop files here"]');
    await expect(dropZone).toBeVisible();
  });

  test('batch controls hidden when no files queued', async ({ page }) => {
    await page.getByRole('tab', { name: 'Batch' }).click();
    await expect(page.getByText('Run Batch')).not.toBeVisible();
    await expect(page.getByText('Clear all')).not.toBeVisible();
  });

  test('can switch between file, batch, and URL tabs', async ({ page }) => {
    const fileTab = page.getByRole('tab', { name: 'File' });
    const batchTab = page.getByRole('tab', { name: 'Batch' });
    const urlTab = page.getByRole('tab', { name: 'URL' });

    await batchTab.click();
    await expect(batchTab).toHaveAttribute('aria-selected', 'true');
    await expect(fileTab).toHaveAttribute('aria-selected', 'false');

    await urlTab.click();
    await expect(urlTab).toHaveAttribute('aria-selected', 'true');
    await expect(batchTab).toHaveAttribute('aria-selected', 'false');

    await fileTab.click();
    await expect(fileTab).toHaveAttribute('aria-selected', 'true');
  });
});

test.describe('Protect page structure', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true');
    });
    await page.goto('/protect');
    await page.waitForSelector('h1');
  });

  test('drop zone is accessible', async ({ page }) => {
    const dropZone = page.getByRole('button', { name: /drop files/i });
    await expect(dropZone).toBeVisible();
  });

  test('filter bar has content type and status selects', async ({ page }) => {
    const filterBar = page.locator('[role="search"][aria-label="Filter assets"]');
    await expect(filterBar).toBeVisible();
    await expect(filterBar.locator('#filter-content-type')).toBeVisible();
    await expect(filterBar.locator('#filter-status')).toBeVisible();
  });

  test('asset count shows zero when no assets imported', async ({ page }) => {
    await expect(page.locator('text=0 assets')).toBeVisible();
  });

  test('Watermark All Images button hidden when no assets', async ({ page }) => {
    // No unwatermarked images, so the batch button should not be visible
    await expect(
      page.locator('button', { hasText: 'Watermark All Images' })
    ).not.toBeVisible();
  });
});
