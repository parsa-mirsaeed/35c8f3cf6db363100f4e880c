# PR-12 Browser E2E, Accessibility, and Product Acceptance

This document records the PR-12 contract and the evidence required to close it.

## Why

Compilation and unit tests do not prove a user can operate the product. PR-12
establishes deterministic browser evidence against the production-like server
build, without making every commit run an expensive full matrix.

## Required invariants

1. A pinned Playwright harness; no floating browser or library tags.
2. Synthetic two-school tenant fixtures with active and inactive accounts.
3. Two tiers: `@smoke` per commit and `@final` under the `full-validation`
   label.
4. Authorization is proven by direct URL / object-ID manipulation, not only by
   hidden buttons.
5. Automated accessibility checks target WCAG 2.2 AA; critical journeys also
   receive manual keyboard/screen-reader acceptance.
6. English and Persian/RTL layouts, date/number direction, and modal/focus
   behavior are exercised.
7. Browser console errors, unhandled promise/WASM errors, and unexpected
   external network calls fail the test.
8. Tests run against the production-like server build, never a mock-only UI.
9. Failure artifacts are captured on failure for routine PRs and preserved for
   final evidence.

## Evidence

Each run writes `evidence.json` with the exact head SHA, tier, tags, outbound
allowlist, unexpected-origin count, and console-error count. Final evidence is
retained per the release policy.

## Exit gate

Every contracted feature has at least one end-to-end positive journey and the
relevant negative authorization journey; WCAG 2.2 AA issues are fixed or
explicitly risk-accepted with an owner and date.
