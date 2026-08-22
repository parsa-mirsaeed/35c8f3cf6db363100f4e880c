// @smoke @workflow-truth — PR-1 browser evidence for production-backed workflows.
//
// These journeys intentionally use the real Dioxus server functions and local
// Auth/Storage HTTP contracts. They do not stub application responses.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const FIXTURE_PASSWORD = 'e2e-password';
const MANAGER_EMAIL = 'e2e-manager-a@example.test';
const TEACHER_EMAIL = 'e2e-teacher-a@example.test';
const STUDENT_EMAIL = 'e2e-pr1-student@example.test';
const CREATED_TEACHER_EMAIL = 'e2e-pr1-teacher@example.test';
const PARENT_EMAIL = 'e2e-pr1-parent@example.test';
const EMPTY_CLASS = 'E2E Empty Class A';
const GUIDED_ASSIGNMENT = 'E2E Guided Publish Draft';

async function signIn(page: Page, email: string, password = FIXTURE_PASSWORD): Promise<void> {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(password);
  await page.getByRole('button', { name: /sign in/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

async function endSession(page: Page): Promise<void> {
  const response = await page.context().request.post('/api/auth/logout');
  expect(response.ok(), 'role-switch logout endpoint must succeed').toBeTruthy();
  await page.goto('/');
  await expect(page).toHaveURL(/\/$/);
}

function actionWithIcon(page: Page, icon: string) {
  return page.locator('button', {
    has: page.locator('span.material-icons-outlined', {
      hasText: new RegExp(`^${icon}$`),
    }),
  }).first();
}

async function openUserCreation(page: Page): Promise<void> {
  await page.goto('/dashboard');
  await actionWithIcon(page, 'groups').click();
  await actionWithIcon(page, 'person_add').click();
  await expect(page.getByRole('tab', { name: /student/i })).toBeVisible();
}

async function revealTemporaryPassword(page: Page): Promise<string> {
  await expect(page.getByText('Account created successfully.', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: /reveal once/i }).click();
  const password = (await page.locator('code').textContent())?.trim() ?? '';
  expect(password.length, 'server-generated temporary credential must be returned once').toBeGreaterThan(10);
  expect(password).not.toContain('••');
  return password;
}

async function createStudent(page: Page): Promise<string> {
  await page.getByRole('tab', { name: /student/i }).click();
  await page.getByLabel('First name').fill('E2E PR1');
  await page.getByLabel('Last name').fill('Student');
  await page.getByLabel('Email').fill(STUDENT_EMAIL);
  await page.getByLabel('Date of birth').fill('2012-05-10');
  await page.getByLabel('Student ID').fill('E2E-PR1-STUDENT');
  await page.getByLabel('Grade level').fill('8');
  await page.getByLabel('Enrollment date').fill('2026-08-22');
  await page.getByLabel('Academic year').fill('2026-2027');
  await page.getByLabel('Class enrollment').selectOption({ label: `${EMPTY_CLASS} · E2E Mathematics` });
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  return revealTemporaryPassword(page);
}

async function createTeacher(page: Page): Promise<string> {
  await page.getByRole('tab', { name: /teacher/i }).click();
  await page.getByLabel('First name').fill('E2E PR1');
  await page.getByLabel('Last name').fill('Teacher');
  await page.getByLabel('Email').fill(CREATED_TEACHER_EMAIL);
  await page.getByLabel('Phone').fill('+41000000001');
  await page.getByLabel('Employee ID').fill('E2E-PR1-TEACHER');
  await page.getByLabel('Department').fill('Mathematics');
  await page.getByLabel('Hire date').fill('2026-08-01');
  await page.getByRole('checkbox', { name: /E2E Mathematics \(E2EMATH\)/ }).check();
  await page.getByRole('checkbox', { name: /E2E Class A1 · E2E Mathematics/ }).check();
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  return revealTemporaryPassword(page);
}

async function createParent(page: Page): Promise<string> {
  await page.getByRole('tab', { name: /parent/i }).click();
  await page.getByLabel('First name').fill('E2E PR1');
  await page.getByLabel('Last name').fill('Parent');
  await page.getByLabel('Email').fill(PARENT_EMAIL);
  await page.getByLabel('Phone').fill('+41000000002');
  await page.getByLabel('Parent ID').fill('E2E-PR1-PARENT');
  await page.getByRole('checkbox', { name: new RegExp(`E2E PR1 Student .* ${STUDENT_EMAIL}`) }).check();
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  return revealTemporaryPassword(page);
}

async function openGuidedAssignment(page: Page): Promise<void> {
  await actionWithIcon(page, 'assignment').click();
  const card = page
    .getByText(GUIDED_ASSIGNMENT, { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(card).toBeVisible();
  await card.getByRole('button', { name: 'View details', exact: true }).click();
  await expect(page.getByRole('dialog')).toContainText(GUIDED_ASSIGNMENT);
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('manager provisions Student Teacher Parent and guided publish persists @smoke @workflow-truth', async ({ page }) => {
  // The teacher first proves an empty class cannot be falsely published.
  await signIn(page, TEACHER_EMAIL);
  await openGuidedAssignment(page);
  await page.getByRole('dialog').getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByRole('dialog')).toContainText(/no active enrolled students/i);
  await endSession(page);

  // The manager provisions all three supported roles through the browser. The
  // Student is enrolled into the previously-empty class in the same atomic
  // provisioning workflow; the Parent is linked to that newly-created Student.
  await signIn(page, MANAGER_EMAIL);
  await openUserCreation(page);
  const studentPassword = await createStudent(page);
  const teacherPassword = await createTeacher(page);
  const parentPassword = await createParent(page);
  await endSession(page);

  // After the real enrollment exists, the original teacher can publish and the
  // assignment becomes visible to the newly-provisioned Student.
  await signIn(page, TEACHER_EMAIL);
  await openGuidedAssignment(page);
  await page.getByRole('dialog').getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByText('Assignment published.', { exact: true })).toBeVisible();
  const publishedCard = page
    .getByText(GUIDED_ASSIGNMENT, { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(publishedCard).toContainText('Published');
  await endSession(page);

  await signIn(page, STUDENT_EMAIL, studentPassword);
  await actionWithIcon(page, 'assignment').click();
  await expect(page.getByText(GUIDED_ASSIGNMENT, { exact: true })).toBeVisible();
  await endSession(page);

  // A newly-created Teacher can authenticate with the one-time credential and
  // sees the persisted class assignment as an available assignment-form class.
  await signIn(page, CREATED_TEACHER_EMAIL, teacherPassword);
  await actionWithIcon(page, 'assignment').click();
  await page.getByRole('button', { name: 'Create assignment', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByRole('dialog').locator('select').first().locator('option', { hasText: 'E2E Class A1' })).toHaveCount(1);
  await endSession(page);

  // The newly-created Parent authenticates and sees only the child linked by
  // the manager's provisioning transaction.
  await signIn(page, PARENT_EMAIL, parentPassword);
  await expect(page.getByText('E2E PR1 Student', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Student B', { exact: true })).toHaveCount(0);
});

test('knowledge storage unavailable state recovers and real PDF upload persists @smoke @workflow-truth', async ({ page, request }) => {
  const unavailable = await request.post('http://127.0.0.1:9100/__e2e/storage-mode', {
    data: { mode: 'unavailable' },
  });
  expect(unavailable.ok()).toBeTruthy();

  await signIn(page, MANAGER_EMAIL);
  await actionWithIcon(page, 'upload_file').click();
  await expect(page.getByText(/Knowledge storage is temporarily unavailable/i)).toBeVisible();
  await expect(page.getByLabel('PDF file *')).toBeDisabled();

  const ready = await request.post('http://127.0.0.1:9100/__e2e/storage-mode', {
    data: { mode: 'ready' },
  });
  expect(ready.ok()).toBeTruthy();
  await page.getByRole('button', { name: 'Retry storage check', exact: true }).click();
  await expect(page.getByText(/Private knowledge storage is ready/i)).toBeVisible();
  await expect(page.getByLabel('PDF file *')).toBeEnabled();

  await page.getByLabel('Title *').fill('E2E PR1 Uploaded PDF');
  await page.getByLabel('PDF file *').setInputFiles({
    name: 'e2e-pr1.pdf',
    mimeType: 'application/pdf',
    buffer: Buffer.from('%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n'),
  });
  await page.getByRole('button', { name: 'Upload for review', exact: true }).click();

  await expect(page.getByRole('status')).toContainText(/uploaded and registered with status submitted/i);
  const submission = page
    .getByText('E2E PR1 Uploaded PDF', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"glass-card")][1]');
  await expect(submission).toContainText('submitted');
  await expect(submission).toContainText(/not OCRed, embedded, or published/i);
});
