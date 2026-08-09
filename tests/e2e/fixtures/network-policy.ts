// Outbound network policy for PR-12 (offline-first boundary).
//
// Every spec installs this guard. Any request to a non-allowlisted origin is
// aborted and recorded; a non-zero count fails the test. This is the browser
// enforcement of the PR-11 offline/CSP boundary.
import { test, type Page } from '@playwright/test';

const allowed = (process.env.E2E_ALLOWED_ORIGINS ?? 'http://127.0.0.1:8080,http://127.0.0.1:9100')
  .split(',')
  .map((origin) => origin.trim())
  .filter(Boolean);

export const unexpectedOrigins: string[] = [];

export async function enforceOfflineAllowlist(page: Page): Promise<void> {
  await page.route('**/*', async (route) => {
    const url = route.request().url();
    let origin: string;
    try {
      origin = new URL(url).origin;
    } catch {
      unexpectedOrigins.push(url);
      await route.abort();
      return;
    }
    if (url.startsWith('data:') || url.startsWith('blob:') || allowed.includes(origin)) {
      await route.continue();
      return;
    }
    unexpectedOrigins.push(origin);
    await route.abort();
  });
}

export function assertNoUnexpectedOrigins(): void {
  test.expect(
    unexpectedOrigins,
    `unexpected external browser origins: ${unexpectedOrigins.join(', ')}`,
  ).toHaveLength(0);
}
