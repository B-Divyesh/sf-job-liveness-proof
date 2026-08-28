import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  timeout: 30_000,
  fullyParallel: false,
  reporter: 'line',
  use: { baseURL: 'http://127.0.0.1:4179', trace: 'retain-on-failure' },
  webServer: {
    command: "RUN_PROOF_SECRET='playwright-secret-with-more-than-thirty-two-characters' DATABASE_URL='sqlite://target/playwright.db?mode=rwc' PORT=4179 cargo run --quiet --bin run-proof-server",
    url: 'http://127.0.0.1:4179/health',
    reuseExistingServer: false,
    timeout: 120_000
  },
  projects: [
    { name: 'desktop', use: { ...devices['Desktop Chrome'] } },
    { name: 'mobile', use: { ...devices['iPhone 13'], browserName: 'chromium', viewport: { width: 390, height: 844 } } }
  ]
});
