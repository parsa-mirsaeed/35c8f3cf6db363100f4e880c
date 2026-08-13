// Console / WASM / HTTP error guard for PR-12 item 9.
//
// Application console errors, unhandled promise/WASM errors, and unexpected
// HTTP failures fail the test. Chromium emits a generic console error for any
// 401 response; the response listener remains authoritative for URL/status.
import { test, type Page } from '@playwright/test';

type AllowedResponse = Readonly<{ path: string; status: number }>;

const DEFAULT_ALLOWED_RESPONSES: readonly AllowedResponse[] = [
  { path: '/api/auth/whoami', status: 401 },
];

const consoleErrors: string[] = [];
const unexpectedHttpResponses: string[] = [];
let allowedResponses: AllowedResponse[] = [...DEFAULT_ALLOWED_RESPONSES];

export function allowHttpResponse(path: string, status: number): void {
  allowedResponses.push({ path, status });
}

export function watchConsole(page: Page): void {
  consoleErrors.length = 0;
  unexpectedHttpResponses.length = 0;
  allowedResponses = [...DEFAULT_ALLOWED_RESPONSES];

  page.on('response', (response) => {
    const status = response.status();
    if (status < 400) {
      return;
    }

    const path = new URL(response.url()).pathname;
    const allowed = allowedResponses.some(
      (entry) => entry.status === status && entry.path === path,
    );
    if (!allowed) {
      unexpectedHttpResponses.push(`${status} ${path}`);
    }
  });

  page.on('console', (message) => {
    if (message.type() !== 'error') {
      return;
    }

    const text = message.text();
    if (/^Failed to load resource: the server responded with a status of 401\b/.test(text)) {
      return;
    }
    consoleErrors.push(text);
  });

  page.on('pageerror', (error) => {
    consoleErrors.push(`pageerror: ${error.message}`);
  });
}

export function assertNoConsoleErrors(): void {
  test.expect(
    unexpectedHttpResponses,
    `unexpected browser HTTP failures: ${unexpectedHttpResponses.join(' | ')}`,
  ).toHaveLength(0);
  test.expect(
    consoleErrors,
    `browser console/WASM errors: ${consoleErrors.join(' | ')}`,
  ).toHaveLength(0);
}
