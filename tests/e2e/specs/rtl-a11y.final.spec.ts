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
  const forgotIndex = visited.indexOf('href:/forgot-password');
  const submitIndex = visited.indexOf('button:submit');

  expect(emailIndex, `email not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThanOrEqual(0);
  expect(passwordIndex, `password not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThan(emailIndex);
  expect(forgotIndex, `forgot-password not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThan(passwordIndex);
  expect(submitIndex, `submit not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThan(forgotIndex);
});
