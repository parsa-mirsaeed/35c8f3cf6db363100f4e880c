// Console / WASM / HTTP error guard for PR-12 item 9.
//
// Application console errors, unhandled promise/WASM errors, and unexpected
// HTTP failures fail the test. Chromium also emits a generic console error for
// failed resources; reconcile that duplicate only when the same status was
// observed on an explicitly allowlisted exact response path.
import { test, type Page } from '@playwright/test';

type AllowedResponse = Readonly<{ path: string; status: number }>;
type GenericResourceError = Readonly<{ status: number; text: string }>;

const DEFAULT_ALLOWED_RESPONSES: readonly AllowedResponse[] = [
  { path: '/api/auth/whoami', status: 401 },
];

const consoleErrors: string[] = [];
const unexpectedHttpResponses: string[] = [];
const allowedHttpStatusesSeen: number[] = [];
const genericResourceErrors: GenericResourceError[] = [];
let allowedResponses: AllowedResponse[] = [...DEFAULT_ALLOWED_RESPONSES];

export function allowHttpResponse(path: string, status: number): void {
  allowedResponses.push({ path, status });
}

export function watchConsole(page: Page): void {
  consoleErrors.length = 0;
  unexpectedHttpResponses.length = 0;
  allowedHttpStatusesSeen.length = 0;
  genericResourceErrors.length = 0;
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
    if (allowed) {
      allowedHttpStatusesSeen.push(status);
    } else {
      unexpectedHttpResponses.push(`${status} ${path}`);
    }
  });

  page.on('console', (message) => {
    if (message.type() !== 'error') {
      return;
    }

    const text = message.text();
    const genericResourceMatch =
      /^Failed to load resource: the server responded with a status of (\d+)\b/.exec(text);
    if (genericResourceMatch) {
      genericResourceErrors.push({
        status: Number(genericResourceMatch[1]),
        text,
      });
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

  // Chromium's generic resource console entry omits the URL. Treat it as the
  // duplicate of an expected failure only when an allowlisted exact-path
  // response with the same status was actually observed. Consume matches
  // one-for-one so an unrelated second failure cannot hide behind one allowed
  // authorization denial.
  const allowedStatusBudget = [...allowedHttpStatusesSeen];
  const unmatchedGenericResourceErrors = genericResourceErrors.filter((error) => {
    const matchIndex = allowedStatusBudget.indexOf(error.status);
    if (matchIndex < 0) {
      return true;
    }
    allowedStatusBudget.splice(matchIndex, 1);
    return false;
  });
  test.expect(
    unmatchedGenericResourceErrors.map((error) => error.text),
    `unmatched browser resource errors: ${unmatchedGenericResourceErrors
      .map((error) => error.text)
      .join(' | ')}`,
  ).toHaveLength(0);

  test.expect(
    consoleErrors,
    `browser console/WASM errors: ${consoleErrors.join(' | ')}`,
  ).toHaveLength(0);
}
