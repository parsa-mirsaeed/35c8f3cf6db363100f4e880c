// @smoke @authorization — PR-12 negative journey by direct URL manipulation.
//
// Backend denial is proven by navigating directly to role-scoped routes with an
// authenticated but unauthorized actor, not by the absence of buttons.
import { test, expect } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const STUDENT = { email: 'e2e-student-a@example.test', password: 'e2e-password' };

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('student direct navigation to teacher-only area is denied @smoke @authorization', async ({ page }) => {
  await page.goto('/login');
  await page.locator('input[type="email"]').fill(STUDENT.email);
  await page.locator('input[type="password"]').fill(STUDENT.password);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).not.toHaveURL(/login/);

  // Direct URL manipulation against a teacher-only route.
  const response = await page.goto('/dashboard/teacher');

  const deniedByRedirect = /login|dashboard(?!\/teacher)/.test(page.url());
  const deniedByStatus = response !== null && [401, 403, 404].includes(response.status());
  const deniedByBody = await page.locator('body').evaluate((body) =>
    /forbidden|unauthorized|access denied|not found|دسترسی/i.test(body.textContent ?? ''),
  );

  expect(
    deniedByRedirect || deniedByStatus || deniedByBody,
    `expected teacher-only route to deny a student (url=${page.url()}, status=${response?.status()})`,
  ).toBeTruthy();
});
