// @final @roles — PR-12 final role landing and guarded-alias evidence.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const roles = [
  { name: 'platform admin', email: 'e2e-admin@example.test', alias: '/dashboard/platform-admin' },
  { name: 'school manager', email: 'e2e-manager-a@example.test', alias: '/dashboard/school-manager' },
  { name: 'teacher', email: 'e2e-teacher-a@example.test', alias: '/dashboard/teacher' },
  { name: 'student', email: 'e2e-student-a@example.test', alias: '/dashboard/student' },
  { name: 'parent', email: 'e2e-parent-a@example.test', alias: '/dashboard/parent' },
] as const;

async function signIn(page: Page, email: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(PASSWORD);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

for (const role of roles) {
  test(`${role.name} reaches the canonical dashboard and role alias @final @roles`, async ({ page }) => {
    await signIn(page, role.email);
    const response = await page.goto(role.alias);
    expect(response === null || response.status() < 400).toBeTruthy();
    await expect(page).toHaveURL(new RegExp(`${role.alias.replaceAll('/', '\\/')}$`));
  });
}

test('authenticated dashboard supports English/LTR @final @roles', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
  await signIn(page, 'e2e-manager-a@example.test');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^en/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('ltr');
});
