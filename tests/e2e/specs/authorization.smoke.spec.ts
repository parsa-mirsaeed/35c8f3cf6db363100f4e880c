// @smoke @authorization — PR-12 negative journeys using direct URL and object-ID manipulation.
//
// Authorization evidence does not depend on hidden buttons: tests exercise the
// role route directly and tamper actual server-function requests with known
// object IDs from another seeded school.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import {
  allowHttpResponse,
  watchConsole,
  assertNoConsoleErrors,
} from '../fixtures/console-guard';

const STUDENT = { email: 'e2e-student-a@example.test', password: 'e2e-password' };
const TEACHER = { email: 'e2e-teacher-a@example.test', password: 'e2e-password' };
const SCHOOL_A_ASSET = 'f3000000-0000-0000-0000-0000000000a1';
const SCHOOL_B_ASSET = 'f3000000-0000-0000-0000-0000000000b1';
const SCHOOL_A_CUSTOM_ASSIGNMENT = 'f1000000-0000-0000-0000-0000000000a2';
const SCHOOL_B_CUSTOM_ASSIGNMENT = 'f1000000-0000-0000-0000-0000000000b1';
const SCHOOL_A_SUBMISSION = 'f2000000-0000-0000-0000-0000000000a4';
const SCHOOL_B_SUBMISSION = 'f2000000-0000-0000-0000-0000000000b1';

async function signIn(page: Page, email: string, password: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(password);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

function actionWithIcon(page: Page, icon: string) {
  return page.locator('button', {
    has: page.locator('span.material-icons-outlined', {
      hasText: new RegExp(`^${icon}$`),
    }),
  }).first();
}

function allowExpectedAuthorizationDenial(path: string): void {
  // Dioxus server-function authorization errors currently serialize as HTTP
  // 500. Keep the browser guard strict everywhere else while also accepting
  // 403/404 so the test remains correct if the transport mapping improves.
  for (const status of [403, 404, 500]) {
    allowHttpResponse(path, status);
  }
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

test('student cannot submit a School B assignment by tampering its object ID @smoke @authorization', async ({ page }) => {
  await signIn(page, STUDENT.email, STUDENT.password);
  await actionWithIcon(page, 'assignment').click();

  await page.getByText('E2E Submission Journey Desktop', { exact: true }).first().click();
  await actionWithIcon(page, 'edit').click();
  await page.locator('textarea').first().fill('School A authorization probe');

  let tamperObserved = false;
  let denialStatus: number | undefined;
  let denialBody = '';
  allowExpectedAuthorizationDenial('/api/submissions/submit');
  await page.route('**/api/submissions/submit', async (route) => {
    const original = route.request().postData();
    expect(original, 'submission request must contain a custom-assignment identifier').toBeTruthy();
    expect(original).toContain(SCHOOL_A_CUSTOM_ASSIGNMENT);
    const tampered = original!.replace(SCHOOL_A_CUSTOM_ASSIGNMENT, SCHOOL_B_CUSTOM_ASSIGNMENT);
    const response = await route.fetch({ postData: tampered });
    denialStatus = response.status();
    denialBody = await response.text();
    await route.fulfill({ response, body: denialBody });
    tamperObserved = true;
  });

  await actionWithIcon(page, 'send').click();
  await expect.poll(() => tamperObserved).toBeTruthy();
  expect([403, 404, 500]).toContain(denialStatus);
  expect(denialBody).toMatch(/assignment not found|forbidden|unauthorized/i);
  await expect(page.locator('body')).toContainText(/not found|unauthorized|forbidden|failed|error|دسترسی|خطا/i);
});

test('teacher cannot grade a School B submission by tampering its object ID @smoke @authorization', async ({ page }) => {
  await signIn(page, TEACHER.email, TEACHER.password);
  await actionWithIcon(page, 'grading').click();

  const assignmentTitle = page.getByText('E2E Authorization Submission A', { exact: true });
  await expect(assignmentTitle).toBeVisible();
  const submissionCard = assignmentTitle.locator(
    'xpath=ancestor::div[contains(@class,"rounded-xl")][.//button[.//span[normalize-space()="grading"]]][1]',
  );
  await submissionCard.locator('xpath=.//button[.//span[normalize-space()="grading"]]').click();

  let tamperObserved = false;
  let denialStatus: number | undefined;
  let denialBody = '';
  allowExpectedAuthorizationDenial('/api/teacher/submissions/grade');
  await page.route('**/api/teacher/submissions/grade', async (route) => {
    const original = route.request().postData();
    expect(original, 'grade request must contain a submission identifier').toBeTruthy();
    expect(original).toContain(SCHOOL_A_SUBMISSION);
    const tampered = original!.replace(SCHOOL_A_SUBMISSION, SCHOOL_B_SUBMISSION);
    const response = await route.fetch({ postData: tampered });
    denialStatus = response.status();
    denialBody = await response.text();
    await route.fulfill({ response, body: denialBody });
    tamperObserved = true;
  });

  // 18 is valid on both the Persian 0–20 and English 0–100 scales, ensuring
  // client validation cannot mask the server authorization boundary.
  await page.locator('input[type="number"]').fill('18');
  await page.locator('textarea').fill('Cross-school grade must be rejected');
  await page.locator('xpath=//button[.//span[normalize-space()="check"]]').click();

  await expect.poll(() => tamperObserved).toBeTruthy();
  expect([403, 404, 500]).toContain(denialStatus);
  expect(denialBody).toMatch(/not owned|not found|forbidden|unauthorized/i);
  await expect(page.locator('body')).toContainText(/not owned|not found|unauthorized|forbidden|failed|error|دسترسی|خطا/i);
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
