import { test, expect } from '@playwright/test';

const allRoutes = ['/', '/protect', '/verify', '/settings'];

test.describe('WCAG basics', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  for (const route of allRoutes) {
    test(`exactly one h1 on ${route}`, async ({ page }) => {
      await page.goto(route);
      const h1s = page.locator('h1');
      await expect(h1s).toHaveCount(1);
    });
  }

  for (const route of allRoutes) {
    test(`all images have alt text or aria-hidden on ${route}`, async ({ page }) => {
      await page.goto(route);

      const offenders = await page.evaluate(() => {
        const imgs = Array.from(document.querySelectorAll('img'));
        return imgs
          .filter((img) => {
            const hasAlt = img.hasAttribute('alt');
            const isHidden = img.getAttribute('aria-hidden') === 'true';
            return !hasAlt && !isHidden;
          })
          .map((img) => img.outerHTML.slice(0, 200));
      });

      expect(offenders, `Images without alt or aria-hidden on ${route}: ${offenders.join('\n')}`).toHaveLength(0);
    });
  }

  for (const route of allRoutes) {
    test(`all visible form inputs have associated labels on ${route}`, async ({ page }) => {
      await page.goto(route);

      const unlabelled = await page.evaluate(() => {
        const inputs = Array.from(
          document.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(
            'input:not([type="hidden"]), select, textarea'
          )
        );
        return inputs
          .filter((el) => {
            if (el.closest('[aria-hidden="true"]')) return false;
            if (el.getAttribute('aria-hidden') === 'true') return false;

            const id = el.id;
            const hasLabelFor = id
              ? document.querySelector(`label[for="${id}"]`) !== null
              : false;
            const hasAriaLabel = el.hasAttribute('aria-label') || el.hasAttribute('aria-labelledby');
            return !hasLabelFor && !hasAriaLabel;
          })
          .map((el) => el.outerHTML.slice(0, 200));
      });

      expect(
        unlabelled,
        `Unlabelled inputs on ${route}: ${unlabelled.join('\n')}`
      ).toHaveLength(0);
    });
  }

  test('skip link becomes visible when programmatically focused', async ({ page }) => {
    await page.goto('/');

    const skipLink = page.getByRole('link', { name: 'Skip to main content' });

    // Before focus: sr-only clips to ≤1×1 px
    const hiddenBox = await skipLink.boundingBox();
    expect(hiddenBox).not.toBeNull();
    expect(hiddenBox!.width).toBeLessThanOrEqual(1);

    // Focus via the API
    await skipLink.focus();

    const visibleBox = await skipLink.boundingBox();
    expect(visibleBox!.width).toBeGreaterThan(1);
    expect(visibleBox!.height).toBeGreaterThan(1);
  });

  test('skip link appears before all other interactive elements in the DOM', async ({ page }) => {
    await page.goto('/');

    // Wait for Svelte to hydrate — the brand link in the header is a reliable
    // sentinel because it renders in the first tick of onMount.
    await page.locator('header a[aria-label="Jura Trace — home"]').waitFor({ state: 'visible' });

    // Now inspect DOM order of anchors with href
    const skipLinkIsFirst = await page.evaluate(() => {
      const allLinks = Array.from(document.querySelectorAll('a[href]'));
      if (allLinks.length === 0) return false;
      return allLinks[0].textContent?.trim() === 'Skip to main content';
    });

    expect(skipLinkIsFirst).toBe(true);
  });

  test('dark mode renders without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await page.addInitScript(() => {
      localStorage.setItem('jura-dark-mode', 'true');
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/');

    await expect(page.locator('html')).toHaveClass(/dark/);
    expect(errors).toHaveLength(0);
  });

  test('light mode renders without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await page.addInitScript(() => {
      localStorage.setItem('jura-dark-mode', 'false');
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
    await page.goto('/');

    await expect(page.locator('html')).not.toHaveClass(/dark/);
    expect(errors).toHaveLength(0);
  });
});

// Focus-visible tests that require the desktop nav to be visible.
test.describe('WCAG basics — desktop focus-visible', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('jura-onboarded', 'true'); localStorage.setItem('jura-setup-complete', 'true');
    });
  });

  test('focus-visible ring class is present on desktop nav link', async ({ page }) => {
    await page.goto('/');

    const desktopNav = page.locator('nav[aria-label="Main navigation"]');
    const protectLink = desktopNav.getByRole('link', { name: 'Protect' });

    await protectLink.focus();

    const isFocused = await protectLink.evaluate((el) => el === document.activeElement);
    expect(isFocused).toBe(true);

    // The layout applies focus-visible:ring-2 — verify the class string
    const cls = await protectLink.getAttribute('class');
    expect(cls).toContain('focus-visible:ring-2');
  });
});
