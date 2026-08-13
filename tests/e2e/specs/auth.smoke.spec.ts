// @smoke @auth — PR-12 Tier-1 critical journey: login, role landing, logout.
//
// Runs against the production-like server build with the mock IdP and the
// synthetic seed fixture. Fails on console errors and unexpected origins.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

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

  // A fresh protected navigation must still be unauthenticated; this proves the
  // visible logout path cleared the HttpOnly server session, not only UI state.
  await page.goto('/dashboard');
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('input[type="email"]')).toBeVisible();
});

test('inactive account cannot authenticate @smoke @auth', async ({ page }) => {
  await signIn(page, 'e2e-inactive@example.test', 'e2e-password');

  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('body')).toContainText(/invalid|inactive|disabled|نامعتبر|غیرفعال/i);
});
