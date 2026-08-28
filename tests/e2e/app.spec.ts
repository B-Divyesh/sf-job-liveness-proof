import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('empty ledger, keyboard path, and legal pages are usable', async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
  await page.goto('/');
  await expect(page).toHaveTitle(/Run Proof/);
  await expect(page.locator('main')).toBeVisible();
  await expect(page.locator('h1')).toHaveCount(1);
  await expect(page.getByRole('heading', { name: 'The ledger is ready' })).toBeVisible();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to ledger' })).toBeFocused();
  await page.goto('/privacy');
  await expect(page.locator('h1')).toHaveText('Privacy');
  await expect(page.locator('main')).toBeVisible();
  expect(consoleErrors).toEqual([]);
});

test('home and legal page have no serious accessibility violations', async ({ page }) => {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    for (const path of ['/', '/terms']) {
      await page.goto(path);
      const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
      expect(results.violations.filter(item => item.impact === 'serious' || item.impact === 'critical')).toEqual([]);
    }
  }
});

test('390px layout has no horizontal overflow', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== 'mobile');
  await page.goto('/');
  await expect(page.getByRole('heading', { name: /Know the job ran/ })).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
  expect(overflow).toBe(false);
});
