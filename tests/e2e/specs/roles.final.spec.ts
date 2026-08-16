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

async function signOut(page: Page): Promise<void> {
  await page.getByRole('button', { name: /sign out|خروج/i }).click();
  await expect(page).toHaveURL(/\/$/);
}

async function establishSession(page: Page, email: string): Promise<void> {
  // Direct-route acceptance must begin from a valid authenticated browser
  // context, not by tearing down a live hydrated Dioxus document. The browser
  // context request client shares its cookie jar with pages in the context, so
  // the role alias below is the first authenticated document navigation.
  const response = await page.context().request.post('/api/auth/login', {
    data: { email, password: PASSWORD },
  });
  expect(response.ok(), `session setup failed for ${email}`).toBeTruthy();
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
    await establishSession(page, role.email);
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
  await page.getByRole('button', { name: 'Knowledge Assets', exact: true }).click();
  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
});

test('student sees only persisted enrollment and assignment state @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-student-a@example.test');

  await expect(page.getByText('E2E Class A1', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('E2E Class B1', { exact: true })).toHaveCount(0);
});

test('student submission is graded by the authorized teacher and appears in persisted grades @final @workflows', async ({ page }, testInfo) => {
  const assignmentTitle = testInfo.project.name === 'mobile-chromium'
    ? 'E2E Submission Journey Mobile'
    : 'E2E Submission Journey Desktop';
  const submittedWork = `Persisted ${testInfo.project.name} submission`;
  const feedback = `Verified ${testInfo.project.name} feedback`;

  // Student performs the contracted submission workflow against the real server.
  await signInEnglish(page, 'e2e-student-a@example.test');
  await actionWithIcon(page, 'assignment').click();
  await expect(page.getByText(assignmentTitle, { exact: true })).toBeVisible();
  await page.getByText(assignmentTitle, { exact: true }).first().click();
  await page.getByRole('button', { name: 'Start Assignment', exact: true }).click();

  const workEditor = page.locator('textarea').first();
  await expect(workEditor).toBeVisible();
  await workEditor.fill(submittedWork);
  await page.getByRole('button', { name: 'Submit Assignment', exact: true }).click();
  await expect(workEditor).toHaveCount(0);
  await signOut(page);

  // The School A teacher sees that submission, records a grade, and persists feedback.
  await signInEnglish(page, 'e2e-teacher-a@example.test');
  await actionWithIcon(page, 'grading').click();
  await expect(page.getByText(assignmentTitle, { exact: true })).toBeVisible();
  await expect(page.getByText(submittedWork, { exact: true })).toBeVisible();

  const submissionCard = page
    .getByText(assignmentTitle, { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"rounded-xl")][.//button[contains(normalize-space(.),"Grade Submission")]][1]');
  await submissionCard.getByRole('button', { name: 'Grade Submission', exact: true }).click();

  const gradingDialog = page.getByRole('dialog');
  await expect(gradingDialog).toBeVisible();
  await gradingDialog.locator('input[type="number"]').fill('91');
  await gradingDialog.locator('textarea').fill(feedback);
  await gradingDialog.getByRole('button', { name: 'Save Grade', exact: true }).click();
  await expect(gradingDialog).toHaveCount(0);
  await expect(page.getByText(assignmentTitle, { exact: true })).toHaveCount(0);
  await signOut(page);

  // The student reads the persisted grade from the production grade view.
  await signInEnglish(page, 'e2e-student-a@example.test');
  await actionWithIcon(page, 'grade').click();
  const classCard = page
    .getByText('E2E Class A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"glass-card")][1]');
  await classCard.getByRole('button', { name: 'View Details', exact: true }).click();

  const gradesDialog = page.getByRole('dialog');
  await expect(gradesDialog).toContainText(assignmentTitle);
  await expect(gradesDialog).toContainText('91/100');
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