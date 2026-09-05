import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { readFile } from 'node:fs/promises';

test('cold landing names the job, audience, and first action', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveTitle('Run Proof — track scheduled jobs that ran');
  await expect(page.getByRole('heading', { name: 'Track scheduled jobs that ran' })).toBeVisible();
  await expect(page.getByText('For small app teams running cron jobs and queue workers.')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Try it with sample data' })).toBeVisible();
  await expect(page.getByText('Opens a populated ledger')).toBeVisible();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: 'Skip to content' })).toBeFocused();
});

test('sample ledger is populated, labelled, resettable, and isolated @claim:demo-ledger', async ({ page }) => {
  const requests: { method: string; pathname: string }[] = [];
  page.on('request', request => requests.push({ method: request.method(), pathname: new URL(request.url()).pathname }));
  await page.goto('/');
  const realLedgerBeforeDemo = await page.evaluate(() => localStorage.getItem('run-proof:last-ledger'));
  requests.length = 0;
  await page.getByRole('link', { name: 'Try it with sample data' }).click();
  await expect(page).toHaveURL(/\/demo$/);
  await expect(page).toHaveTitle('Demo — Run Proof');
  await expect(page.getByText('Demo — sample data, nothing is saved')).toBeVisible();
  await expect(page.getByText('Nightly billing sweep')).toBeVisible();
  await expect(page.getByText('Warehouse inventory sync')).toBeVisible();
  await expect(page.getByText('Customer digest mailer')).toBeVisible();
  await expect(page.getByText('Invoice export')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Reset demo' })).toBeVisible();
  await page.getByRole('button', { name: 'Reset demo' }).click();
  await expect(page.getByText('The sample ledger has been reset. Your real ledger was not changed.')).toBeVisible();
  expect(requests.some(request => request.pathname === '/api/v1/ledger')).toBe(false);
  expect(requests.some(request => request.pathname === '/api/v1/demo/ledger')).toBe(true);
  expect(requests.every(request => request.method === 'GET')).toBe(true);
  await expect.poll(() => page.evaluate(() => localStorage.getItem('run-proof:last-ledger'))).toBe(realLedgerBeforeDemo);
  await page.getByRole('link', { name: 'Start for real' }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByText('Demo — sample data, nothing is saved')).toHaveCount(0);
});

test('sample ledger exports one CSV row per sample run @claim:csv-export', async ({ page }) => {
  await page.goto('/demo');
  await expect(page.getByText('Invoice export')).toBeVisible();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export ledger CSV' }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe('run-proof-sample-ledger.csv');
  const contents = await readFile((await file.path())!, 'utf8');
  expect(contents).toContain('job_key,display_name,run_id,state,completion_count');
  expect(contents.trim().split('\n')).toHaveLength(5);
});

test('sample receipt download contains the selected sample run @claim:sample-receipt', async ({ page }) => {
  await page.goto('/demo');
  await expect(page.getByText('Nightly billing sweep')).toBeVisible();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export receipt for Nightly billing sweep' }).click();
  const file = await download;
  const contents = await readFile((await file.path())!, 'utf8');
  expect(file.suggestedFilename()).toContain('run-proof-sample-billing-sweep');
  expect(JSON.parse(contents)).toMatchObject({ format: 'run-proof-demo-receipt/v1', sample: true, run: { job_key: 'billing-sweep' } });
});

test('offline reload works after the first visit @claim:offline-reload', async ({ browser }) => {
  const context = await browser.newContext();
  try {
    const page = await context.newPage();
    await page.goto('/demo');
    await expect(page.locator('.connection')).toContainText('Sample ledger loaded');
    await page.evaluate(async () => { await navigator.serviceWorker.ready; });
    await page.reload();
    await expect(page.getByText('Demo — sample data, nothing is saved')).toBeVisible();
    await context.setOffline(true);
    await page.reload();
    await expect(page.locator('.connection')).toContainText('Offline · showing last copy');
    await expect(page.getByRole('main')).toBeVisible();
  } finally {
    await context.close();
  }
});

test('demo flow makes only same-origin requests @claim:no-tracking', async ({ page }) => {
  const external: string[] = [];
  page.on('request', request => { if (new URL(request.url()).origin !== 'http://127.0.0.1:4179') external.push(request.url()); });
  await page.goto('/demo');
  await expect(page.getByText('Nightly billing sweep')).toBeVisible();
  await page.getByRole('button', { name: 'Reset demo' }).click();
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Export ledger CSV' }).click();
  await download;
  expect(external).toEqual([]);
});

test('home, demo, and legal pages have no serious accessibility violations', async ({ page }) => {
  for (const colorScheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme });
    for (const path of ['/', '/demo', '/terms']) {
      await page.goto(path);
      await expect(page.getByRole('main')).toBeVisible();
      const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
      expect(results.violations.filter(item => item.impact === 'serious' || item.impact === 'critical')).toEqual([]);
    }
  }
});

test('client routes move focus to the new page heading and announce it', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('link', { name: 'Privacy', exact: true }).first().click();
  await expect(page).toHaveURL(/\/privacy$/);
  await expect(page.getByRole('heading', { name: 'Privacy' })).toBeFocused();
  await expect(page.locator('#route-announcement')).toHaveText('Privacy.');
  await expect(page).toHaveTitle('Privacy — Run Proof');
});

test('390px layout has no horizontal overflow at 200% text', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== 'mobile');
  await page.goto('/demo');
  await expect(page.getByText('Nightly billing sweep')).toBeVisible();
  await page.evaluate(() => { document.documentElement.style.fontSize = '32px'; });
  const dimensions = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth }));
  expect(dimensions.scroll).toBe(dimensions.client);
});

test('finish-before-start evidence renders without a page error', async ({ page }) => {
  const pageErrors: string[] = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  await page.route('**/api/v1/ledger', route => route.fulfill({ json: { generated_at: new Date().toISOString(), summary: { completed: 1 }, rows: [{ job_key: 'mailer', display_name: 'Mailer', run_id: 'finish-first', scheduled_at: null, started_at: null, finished_at: new Date().toISOString(), completion_count: 0, state: 'completed', source: null, observed_status: null, source_url: null, observed_at: null, receipt_hash: 'sha256:test', is_virtual: false }] } }));
  await page.goto('/');
  await expect(page.getByText('Schedule not received')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Export receipt for Mailer' })).toHaveAttribute('data-job', 'mailer');
  expect(pageErrors).toEqual([]);
});

test('static policy, font delivery, install metadata, and touch targets are release safe', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== 'mobile');
  const response = await page.goto('/');
  expect(response?.headers()['strict-transport-security']).toContain('max-age=');
  expect(response?.headers()['permissions-policy']).toContain('camera=()');
  const asset = await page.request.get('/assets/app.js');
  expect(asset.headers()['cache-control']).toBe('no-cache');
  const manifest = await (await page.request.get('/manifest.webmanifest')).json() as { icons: unknown[] };
  expect(manifest.icons).toHaveLength(2);
  const font = await page.evaluate(() => document.fonts.check('16px "Atkinson Hyperlegible Next Variable"') && document.fonts.check('700 32px "Bitter Variable"'));
  expect(font).toBe(true);
  const fontPath = await page.evaluate(() => new URL(performance.getEntriesByType('resource').map(entry => entry.name).find(name => name.endsWith('.woff2')) ?? '', location.href).pathname);
  const fontResponse = await page.request.get(fontPath);
  expect(fontResponse.headers()['cache-control']).toContain('immutable');
  const undersized = await page.locator('a:visible, button:visible').evaluateAll(nodes => nodes.filter(node => { const box = node.getBoundingClientRect(); return box.width < 44 || box.height < 44; }).map(node => ({ text: node.textContent?.trim(), box: node.getBoundingClientRect().toJSON() })));
  expect(undersized).toEqual([]);
});

test('unknown browser routes return a designed 404', async ({ page }) => {
  const response = await page.goto('/not-a-route');
  expect(response?.status()).toBe(404);
  await expect(page).toHaveTitle('Page not found — Run Proof');
  await expect(page.getByRole('heading', { name: 'Page not found' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Go to the ledger' })).toBeVisible();
});

test('returned licenses use the rate-limited same-origin verification proxy', async ({ page }) => {
  const verificationRequests: string[] = [];
  const external: string[] = [];
  await page.route('**/api/v1/products/job-liveness-proof/verify', async route => {
    verificationRequests.push(route.request().url());
    expect(route.request().method()).toBe('POST');
    expect(route.request().postDataJSON()).toEqual({ license: 'invalid-token' });
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify({ valid: false, reason: 'invalid', expires_at: null }) });
  });
  page.on('request', request => { if (new URL(request.url()).origin !== 'http://127.0.0.1:4179') external.push(request.url()); });
  await page.goto('/?license=invalid-token');
  await expect.poll(() => verificationRequests.length).toBe(1);
  expect(new URL(verificationRequests[0]).origin).toBe('http://127.0.0.1:4179');
  expect(verificationRequests[0]).not.toContain('invalid-token');
  expect(external).toEqual([]);
  await expect.poll(() => page.evaluate(() => localStorage.getItem('sb_license:job-liveness-proof'))).toBe('invalid-token');
  expect(page.url()).not.toContain('license=');
});
