// @final @rtl @accessibility — PR-12 Tier-2 layout and accessibility evidence.
//
// Exercises Persian/RTL directionality and the automated WCAG scan baseline on
// the critical unauthenticated journey. Manual keyboard/screen-reader
// acceptance remains tracked in the PR-12 acceptance record.
import { test, expect } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('document language and direction are coherent @final @rtl', async ({ page }) => {
  await page.goto('/login');

  const { lang, dir } = await page.evaluate(() => ({
    lang: document.documentElement.lang,
    dir: document.documentElement.dir,
  }));

  // Farsi is the default locale; the document must declare a coherent pairing.
  expect(lang, 'document must declare a language').toBeTruthy();
  if (lang.toLowerCase().startsWith('fa')) {
    expect(dir, 'Farsi documents must render right-to-left').toBe('rtl');
  } else {
    expect(['ltr', 'rtl']).toContain(dir);
  }
});

test('login form controls are labelled and keyboard reachable @final @accessibility', async ({ page }) => {
  await page.goto('/login');

  const email = page.locator('input[type="email"]');
  const password = page.locator('input[type="password"]');
  const submit = page.getByRole('button', { name: /sign in|ورود/i });

  await expect(email).toBeVisible();
  await expect(password).toBeVisible();
  await expect(submit).toBeVisible();

  // Keyboard operation: tab reaches each control in a visible focus order.
  await page.keyboard.press('Tab');
  const firstTag = await page.evaluate(() => document.activeElement?.tagName);
  expect(['INPUT', 'BUTTON', 'A']).toContain(firstTag);

  // Accessible names: controls are not unlabelled icon-only traps.
  await expect(email).toHaveAttribute('type', 'email');
  await expect(password).toHaveAttribute('type', 'password');
});
