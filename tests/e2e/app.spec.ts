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

test('offline reload uses the cached shell and last ledger', async ({ page, context }) => {
  await page.goto('/');
  await expect(page.locator('.connection')).toContainText('Receiver connected');
  await page.evaluate(async () => { await navigator.serviceWorker.ready; });
  await page.reload();
  await expect(page.locator('.connection')).toContainText('Receiver connected');
  await context.setOffline(true);
  await page.reload();
  await expect(page.locator('.connection')).toContainText('Offline · showing last copy');
  await expect(page.getByRole('main')).toBeVisible();
  await context.setOffline(false);
});

test('privacy page makes no third-party requests', async ({ page }) => {
  const external: string[] = [];
  page.on('request', request => { if (new URL(request.url()).origin !== 'http://127.0.0.1:4179') external.push(request.url()); });
  await page.goto('/privacy');
  await expect(page.getByRole('heading', { name: 'Privacy' })).toBeVisible();
  expect(external).toEqual([]);
});
