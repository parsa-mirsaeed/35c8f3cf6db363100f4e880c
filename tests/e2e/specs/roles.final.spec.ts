// @final @roles @workflows — PR-12 final role and persisted core-workflow evidence.
//
// The production release deliberately disables unfinished attendance, timetable,
// reports, messaging, derived metrics, and synthetic-health domains. These
// journeys cover only enabled core school workflows against the deterministic
// two-school fixture and the real server-backed UI.
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

async function signInEnglish(page: Page, email: string): Promise<void> {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
  await signIn(page, email);
}

function actionWithIcon(page: Page, icon: string) {
  return page.locator('button', {
    has: page.locator('span.material-icons-outlined', {
      hasText: new RegExp(`^${icon}$`),
    }),
  }).first();
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
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^en/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('ltr');
});

test('school manager reads only the authorized school user directory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await actionWithIcon(page, 'groups').click();

  await expect(page.getByText('E2E Teacher A', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Student A', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Teacher B', { exact: true })).toHaveCount(0);
  await expect(page.getByText('E2E Student B', { exact: true })).toHaveCount(0);
});

test('school manager reads only the authorized class inventory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await actionWithIcon(page, 'class').click();

  await expect(page.getByText('E2E Class A1', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Class B1', { exact: true })).toHaveCount(0);
});

test('school manager sees the governed school knowledge inventory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await page.getByRole('button', { name: /register governed school sources for platform review/i }).click();

  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
});

test('teacher sees the persisted published assignment and governed knowledge asset @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-teacher-a@example.test');

  // The overview intentionally renders both persisted assignment and class context.
  await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('E2E Class A1', { exact: true }).first()).toBeVisible();

  await actionWithIcon(page, 'assignment').click();
  await expect(page.getByText('E2E Assignment A1', { exact: true })).toBeVisible();

  // Re-enter the canonical overview so this action is independent of the
  // responsive shell's desktop/mobile navigation rendering.
  await page.goto('/dashboard');
  await page.getByRole('button', { name: /knowledge assets/i }).click();
  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
});

test('student sees only persisted enrollment and assignment state @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-student-a@example.test');

  await expect(page.getByText('E2E Class A1', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('E2E Class B1', { exact: true })).toHaveCount(0);
});

test('parent sees only the authorized child enrollment @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-parent-a@example.test');

  await expect(page.getByText('E2E Student A', { exact: true })).toBeVisible();
  await expect(page.getByText('1 enrolled classes', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Student B', { exact: true })).toHaveCount(0);
});

test('platform admin sees the governed published asset inventory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-admin@example.test');

  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
});
