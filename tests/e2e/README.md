# PR-12 Browser E2E, Accessibility, and Product Acceptance Harness

This directory contains the pinned browser-acceptance harness required by plan
PR-12. It is intentionally isolated from the Rust workspace so browser evidence
runs against the production-like server build, never a mock-only UI.

## Contract

- Pinned runtime: Playwright `1.49.1` and `@playwright/test` `1.49.1`. No
  floating `latest` tags (plan §7.4).
- The runner image must preinstall the pinned browser; CI must not download
  browsers per run (plan §5.2).
- The harness starts the real production-like stack: PostgreSQL migrations are
  applied, the Dioxus server is built and run, and a mock identity provider
  (`fixtures/mock-idp.mjs`) stands in for Supabase Auth/JWKS. No live or paid
  external service is ever contacted (plan §5.2, §1.1).
- Every spec runs with an explicit outbound-network allowlist. Any request to a
  non-allowlisted origin fails the test (plan PR-11/PR-12 offline boundary).
- Browser console errors, unhandled promise rejections, and WASM errors fail
  the test (plan PR-12 item 9).
- Authorization is proven by direct URL / object-ID manipulation, not only by
  hidden buttons (plan PR-12 item 4).

## Tiers

- `@smoke`: login, role landing, one critical changed-feature path,
  authorization denial, logout. Runs on ordinary PR commits (one engine).
- `@final`: all contracted workflows, roles, desktop + mobile viewports,
  English and Persian/RTL, accessibility scan, offline network policy. Runs
  under the `full-validation` label.

## Layout

- `playwright.config.ts` — pinned projects, offline allowlist, fail-on-console.
- `fixtures/seed.sql` — synthetic two-school tenant fixture (plan §8).
- `fixtures/mock-idp.mjs` — local Supabase Auth/JWKS stand-in.
- `fixtures/network-policy.ts` — outbound allowlist + unexpected-origin failure.
- `fixtures/console-guard.ts` — console/WASM error guard.
- `specs/` — `@smoke` and `@final` journeys (added with the harness runbooks).

## Evidence

Each run writes a compact `evidence.json` (exact head SHA, tier, tags,
allowlist, unexpected-origin count, console-error count). Failure artifacts
(screenshots, traces, console/network logs) are captured only on failure for
routine PRs and preserved for final evidence (plan §16).
