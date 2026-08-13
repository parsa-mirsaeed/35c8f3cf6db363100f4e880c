// @final @rtl @accessibility — PR-12 Tier-2 layout and accessibility evidence.
import AxeBuilder from '@axe-core/playwright';
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

test('Farsi and English document direction are coherent @final @rtl', async ({ page }) => {
  await page.goto('/');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^fa/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');

  await page.locator('.language-switcher-toggle').click();
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^en/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('ltr');
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
  const email = page.locator('input[type="email"]');
  const password = page.locator('input[type="password"]');
  const forgotPassword = page.locator('form button[type="button"]').first();
  const submit = page.locator('form button[type="submit"]');

  await email.focus();
  await expect(email).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(password).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(forgotPassword).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(submit).toBeFocused();
});
