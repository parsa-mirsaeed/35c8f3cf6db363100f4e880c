# PR-12 Browser E2E, Accessibility, and Product Acceptance

This document records the PR-12 contract and the evidence required to close it.

## Why

Compilation and unit tests do not prove a user can operate the product. PR-12 establishes deterministic browser evidence against the production-like server build, without making every commit run an expensive full matrix.

## Required invariants

1. A pinned Playwright harness; no floating browser or library tags.
2. Synthetic two-school tenant fixtures with active and inactive accounts, classes, assignments, submissions, grades and governed knowledge assets.
3. Two tiers: `@smoke` for targeted per-change evidence and `@final` for complete product acceptance.
4. Authorization is proven by direct URL / object-ID manipulation, not only by hidden buttons.
5. Automated accessibility checks target WCAG 2.2 AA; critical journeys also receive manual keyboard/screen-reader acceptance.
6. English and Persian/RTL layouts, date/number direction, and modal/focus behavior are exercised.
7. Browser console errors, unhandled page/WASM errors, and unexpected external network calls fail the test.
8. Tests run against the production-like server build, never a mock-only UI.
9. Failure traces/screenshots/video are retained by the harness policy, and final workflow evidence is retained for release review.
10. Test-only identity/fixture behavior remains confined to the explicitly enabled E2E environment and is not a production authentication fallback.

## Automated implementation coverage

The final suite exercises the enabled pilot surface, including:

- authentication, role landing/direct role aliases, logout and inactive-account denial;
- School Manager user, class and governed-knowledge views;
- Teacher assignments and governed knowledge;
- Student enrollment and assignment views;
- Student submission -> authorized Teacher grading/feedback -> persisted Student grade view;
- Parent child/enrollment visibility;
- Platform Admin governed-knowledge visibility;
- cross-role denial;
- cross-school direct object-ID denial for governed knowledge, Student submission and Teacher grading;
- English/LTR and Persian/RTL direction;
- explicit LTR isolation of persisted grade dates/numbers inside RTL content;
- production modal dialog semantics and keyboard focus entry/order;
- automated WCAG A/AA and keyboard-order checks;
- desktop and mobile Chromium final acceptance under the PR-11 offline/CSP boundary.

Features disabled by the production capability inventory—attendance, timetable, reports, messaging, grade trends, derived metrics and synthetic health—are excluded rather than represented by fake acceptance journeys.

## Evidence

Each browser run writes evidence for the exact workflow head and uses the strict outbound allowlist plus console/pageerror/WASM guards. The pull-request description records the final exact head and successful required workflow runs after validation.

`tests/e2e/EVIDENCE.md` is the merge-evidence contract. Manual assistive-technology results are recorded separately in `docs/security/pr-12-manual-accessibility-acceptance.md` so automated CI is never misrepresented as human screen-reader acceptance.

## Exit gate

Every contracted feature has at least one end-to-end positive journey and the relevant negative authorization journey; WCAG 2.2 AA issues are fixed or explicitly risk-accepted with an owner and date where the plan permits risk acceptance.

PR-12 remains draft until the unchanged final head has green AI Change Proof and Full Validation evidence, no unresolved review blockers, and the required human keyboard/screen-reader acceptance is recorded.
