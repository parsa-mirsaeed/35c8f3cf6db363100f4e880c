// @smoke @auth — PR-12 Tier-1 critical journey: login, role landing, logout.
//
// Runs against the production-like server build with the mock IdP and the
// synthetic seed fixture. Fails on console errors and unexpected origins.
import { test, expect } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const MANAGER = { email: 'e2e-manager-a@example.test', password: 'e2e-password' };

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('unauthenticated dashboard access is redirected to login @smoke @auth', async ({ page }) => {
  await page.goto('/dashboard');
  await expect(page).toHaveURL(/login/);
  await expect(page.locator('input[type="email"]')).toBeVisible();
});

test('manager can sign in, lands off the login page, and can sign out @smoke @auth', async ({ page, request }) => {
  await page.goto('/login');

  await page.locator('input[type="email"]').fill(MANAGER.email);
  await page.locator('input[type="password"]').fill(MANAGER.password);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();

  // Role landing: navigated away from login with no client errors.
  await expect(page).not.toHaveURL(/login/);
  await expect(page.locator('body')).not.toContainText(/invalid email or password/i);

  // Logout clears the session; the next protected navigation returns to login.
  await request.post('/api/auth/logout');
  await page.goto('/dashboard');
  await expect(page).toHaveURL(/login/);
});

test('inactive account cannot authenticate @smoke @auth', async ({ page }) => {
  await page.goto('/login');

  await page.locator('input[type="email"]').fill('e2e-inactive@example.test');
  await page.locator('input[type="password"]').fill('e2e-password');
  await page.getByRole('button', { name: /sign in|ورود/i }).click();

  // PR-02: a disabled account must not obtain a session. The journey stays on
  // login and surfaces an authentication failure without internals leakage.
  await expect(page).toHaveURL(/login/);
  await expect(page.locator('body')).toContainText(/invalid|inactive|disabled|نامعتبر|غیرفعال/i);
});
