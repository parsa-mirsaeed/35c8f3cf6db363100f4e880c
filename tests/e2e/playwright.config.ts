import { defineConfig, devices } from '@playwright/test';

// PR-12: evidence runs against the production-like server build, never a
// mock-only UI. The base URL is the real Dioxus server started by CI.
const baseURL = process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8080';

// Offline boundary: only the application origin and the local mock IdP may be
// contacted. Everything else is an unexpected external request (PR-11/PR-12).
const allowedOrigins = (process.env.E2E_ALLOWED_ORIGINS ?? `${baseURL},http://127.0.0.1:9100`)
  .split(',')
  .map((origin) => origin.trim())
  .filter(Boolean);

export default defineConfig({
  testDir: './specs',
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1, // single self-hosted runner; serial evidence (plan §5)
  retries: 0, // deterministic acceptance; no flaky-retry masking
  reporter: [
    ['list'],
    ['json', { outputFile: 'evidence.json' }],
    ['html', { open: 'never', outputFolder: 'playwright-report' }],
  ],
  use: {
    baseURL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    extraHTTPHeaders: {
      'x-e2e-allowed-origins': allowedOrigins.join(','),
    },
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'mobile-chromium',
      use: { ...devices['Pixel 7'] },
      grep: /@final|@mobile/,
    },
  ],
  metadata: {
    allowedOrigins,
    headSha: process.env.E2E_HEAD_SHA ?? 'unknown',
  },
});
