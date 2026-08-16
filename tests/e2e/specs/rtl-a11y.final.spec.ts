// @final @rtl @accessibility — PR-12 Tier-2 layout and accessibility evidence.
import AxeBuilder from '@axe-core/playwright';
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';

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

test('Farsi and English document direction are coherent @final @rtl', async ({ page }) => {
  await page.goto('/');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^fa/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');

  await page.locator('.language-switcher-toggle').click();
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^en/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('ltr');
});

test('Persian grade dates and numbers are isolated LTR inside the RTL document @final @rtl', async ({ page }) => {
  await signIn(page, 'e2e-student-a@example.test');
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');

  await actionWithIcon(page, 'grade').click();
  const classCard = page
    .getByText('E2E Class A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"glass-card")][1]');
  await classCard.getByRole('button').click();

  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const seededGradeRow = dialog
    .getByText('E2E Assignment A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"rounded-lg")][1]');
  const isolatedValues = seededGradeRow.locator('bdi[dir="ltr"]');
  await expect(isolatedValues).toHaveCount(3);
  for (let index = 0; index < 3; index += 1) {
    await expect(isolatedValues.nth(index)).toHaveCSS('direction', 'ltr');
  }
});

test('shared grading modal exposes dialog semantics and moves keyboard focus inside @final @accessibility', async ({ page }) => {
  await signInEnglish(page, 'e2e-teacher-a@example.test');
  await actionWithIcon(page, 'grading').click();

  const assignmentTitle = page.getByText('E2E Authorization Submission A', { exact: true });
  await expect(assignmentTitle).toBeVisible();
  const submissionCard = assignmentTitle.locator(
    'xpath=ancestor::div[contains(@class,"rounded-xl")][.//button[contains(normalize-space(.),"Grade Submission")]][1]',
  );
  await submissionCard.getByRole('button', { name: 'Grade Submission', exact: true }).click();

  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute('aria-modal', 'true');
  await expect(dialog).toHaveAttribute('aria-labelledby', 'edutalent-modal-title');
  await expect(dialog.locator('#edutalent-modal-title')).toContainText('Grade Submission');

  const closeButton = dialog.getByRole('button', { name: 'Close', exact: true });
  await expect(closeButton).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(dialog.locator('input[type="number"]')).toBeFocused();
  await page.keyboard.press('Shift+Tab');
  await expect(closeButton).toBeFocused();
});

test('login has no automated WCAG A/AA violations @final @accessibility', async ({ page }) => {
  await page.goto('/');
  const result = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
    .analyze();
  expect(
    result.violations,
    result.violations.map((violation) => `${violation.id}: ${violation.help}`).join('\n'),
  ).toEqual([]);
});

test('login form controls follow the visible keyboard order @final @accessibility', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });

  const visited: string[] = [];
  for (let step = 0; step < 16; step += 1) {
    await page.keyboard.press('Tab');
    visited.push(
      await page.evaluate(() => {
        const element = document.activeElement;
        if (!(element instanceof HTMLElement)) return '';
        if (element.id) return `id:${element.id}`;
        if (element instanceof HTMLAnchorElement) return `href:${element.getAttribute('href') ?? ''}`;
        if (element instanceof HTMLButtonElement) return `button:${element.type}`;
        return element.tagName.toLowerCase();
      }),
    );
  }

  const emailIndex = visited.indexOf('id:login-email');
  const passwordIndex = visited.indexOf('id:login-password');
  const submitIndex = visited.indexOf('button:submit');

  expect(emailIndex, `email not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThanOrEqual(0);
  expect(passwordIndex, `password not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThan(emailIndex);
  expect(submitIndex, `submit not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThan(passwordIndex);
});
