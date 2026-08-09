// Console / WASM error guard for PR-12 item 9.
//
// Browser console errors, unhandled promise rejections, and WASM failures fail
// the test. This prevents silent hydration or client crashes from passing.
import { test, type Page } from '@playwright/test';

const consoleErrors: string[] = [];

export function watchConsole(page: Page): void {
  page.on('console', (message) => {
    if (message.type() === 'error') {
      consoleErrors.push(message.text());
    }
  });
  page.on('pageerror', (error) => {
    consoleErrors.push(`pageerror: ${error.message}`);
  });
}

export function assertNoConsoleErrors(): void {
  test.expect(
    consoleErrors,
    `browser console/WASM errors: ${consoleErrors.join(' | ')}`,
  ).toHaveLength(0);
}
