// @smoke @authorization — PR-12 negative journeys using direct URL and object-ID manipulation.
//
// Authorization evidence does not depend on hidden buttons: tests exercise the
// role route directly and tamper an actual server-function request with a known
// object ID from another seeded school.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const STUDENT = { email: 'e2e-student-a@example.test', password: 'e2e-password' };
const TEACHER = { email: 'e2e-teacher-a@example.test', password: 'e2e-password' };
const SCHOOL_A_ASSET = 'f3000000-0000-0000-0000-0000000000a1';
const SCHOOL_B_ASSET = 'f3000000-0000-0000-0000-0000000000b1';

async function signIn(page: Page, email: string, password: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(password);
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

test('student direct navigation to teacher-only area is denied @smoke @authorization', async ({ page }) => {
  await signIn(page, STUDENT.email, STUDENT.password);

  const response = await page.goto('/dashboard/teacher');
  const deniedByRedirect = /\/$|\/dashboard$/.test(new URL(page.url()).pathname);
  const deniedByStatus = response !== null && [401, 403, 404].includes(response.status());
  const deniedByBody = await page.locator('body').evaluate((body) =>
    /forbidden|unauthorized|access denied|not found|دسترسی/i.test(body.textContent ?? ''),
  );

  expect(
    deniedByRedirect || deniedByStatus || deniedByBody,
    `expected teacher-only route to deny a student (url=${page.url()}, status=${response?.status()})`,
  ).toBeTruthy();
});

test('teacher cannot mutate a School B knowledge asset by tampering its object ID @smoke @authorization', async ({ page }) => {
  await signIn(page, TEACHER.email, TEACHER.password);

  await page.getByRole('button', { name: /knowledge assets/i }).first().click();
  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E School B Asset', { exact: true })).toHaveCount(0);

  let tamperObserved = false;
  await page.route('**/api/teacher/scoped/knowledge-assets/toggle', async (route) => {
    const original = route.request().postData();
    expect(original, 'toggle request must contain an object identifier').toBeTruthy();
    expect(original).toContain(SCHOOL_A_ASSET);
    const tampered = original!.replace(SCHOOL_A_ASSET, SCHOOL_B_ASSET);
    tamperObserved = true;
    const response = await route.fetch({ postData: tampered });
    await route.fulfill({ response });
  });

  await page.getByRole('button', { name: /enable for generation/i }).click();
  await expect.poll(() => tamperObserved).toBeTruthy();
  await expect(page.locator('body')).toContainText(/update failed|forbidden|not found|unauthorized/i);
});
