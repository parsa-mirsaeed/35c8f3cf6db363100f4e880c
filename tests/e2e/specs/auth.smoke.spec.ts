// @smoke @auth — PR-12 Tier-1 critical journey: login, role landing, logout.
//
// Runs against the production-like server build with the mock IdP and the
// synthetic seed fixture. Fails on console errors and unexpected origins.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import {
  allowHttpResponse,
  watchConsole,
  assertNoConsoleErrors,
} from '../fixtures/console-guard';

const MANAGER = { email: 'e2e-manager-a@example.test', password: 'e2e-password' };

async function signIn(page: Page, email: string, password: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(password);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('production response enforces the offline CSP boundary @smoke @auth', async ({ page }) => {
  const response = await page.goto('/');
  expect(response, 'initial document response must exist').not.toBeNull();
  const headers = response!.headers();
  const csp = headers['content-security-policy'];
  expect(csp, 'production response must enforce a CSP').toBeTruthy();
  expect(csp).toContain("default-src 'self'");
  expect(csp).toContain("connect-src 'self'");

  const scriptDirective = csp
    .split(';')
    .map((directive) => directive.trim())
    .find((directive) => directive.startsWith('script-src'));
  expect(scriptDirective, 'CSP must contain script-src').toBeTruthy();
  const scriptTokens = scriptDirective!.split(/\s+/);
  expect(scriptTokens).toContain("'wasm-unsafe-eval'");
  expect(scriptTokens).not.toContain("'unsafe-eval'");
  expect(scriptTokens).not.toContain("'unsafe-inline'");
  expect(headers['x-content-type-options']).toBe('nosniff');
  expect(headers['x-frame-options']).toBe('DENY');
});

test('unauthenticated dashboard access returns to the canonical login route @smoke @auth', async ({ page }) => {
  await page.goto('/dashboard');
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('input[type="email"]')).toBeVisible();
});

test('manager can sign in, lands on dashboard, and UI logout terminates the session @smoke @auth', async ({ page }) => {
  await signIn(page, MANAGER.email, MANAGER.password);

  await expect(page).toHaveURL(/\/dashboard$/);
  await expect(page.locator('body')).not.toContainText(/invalid email or password/i);

  await page.getByRole('button', { name: /sign out|خروج/i }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('input[type="email"]')).toBeVisible();

  await page.goto('/dashboard');
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('input[type="email"]')).toBeVisible();
});

test('inactive account cannot authenticate @smoke @auth', async ({ page }) => {
  allowHttpResponse('/api/auth/login', 401);
  await signIn(page, 'e2e-inactive@example.test', 'e2e-password');

  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('body')).toContainText(/invalid|inactive|disabled|نامعتبر|غیرفعال/i);
});
