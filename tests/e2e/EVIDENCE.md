# PR-12 exact-head evidence contract

This note records what PR-12 must prove before merge. It intentionally does not hard-code a commit SHA: the authoritative exact head is the pull-request head, and CI writes that SHA into the retained browser evidence for each run.

## Scope

PR-12 validates the enabled, contracted browser surface against the production-like Dioxus server, PostgreSQL authorization boundary, deterministic two-school fixture, and local test identity provider.

The automated suite includes:

- login, canonical role landing, direct role aliases, logout and session termination;
- active/inactive account behavior;
- School Manager user/class/knowledge views;
- Teacher assignment and governed-knowledge views;
- Student enrollment/assignment views;
- the stateful Student submission -> authorized Teacher grading/feedback -> persisted Student grade journey;
- Parent child/enrollment view;
- Platform Admin governed knowledge view;
- direct role denial and direct cross-school object-ID tampering for governed knowledge, Student submission and Teacher grading;
- English/LTR and Persian/RTL document behavior;
- explicit bidirectional isolation for grade dates/numbers in RTL;
- shared production modal dialog semantics and keyboard focus entry/order;
- automated WCAG A/AA scanning and login keyboard-order checks;
- fatal unexpected external origins, browser console errors, page errors and WASM errors;
- desktop Chromium and mobile Chromium final acceptance.

Unfinished attendance, timetable, reports, messaging, grade trends, derived metrics and synthetic health are outside the production capability inventory and are not falsely exercised as contracted workflows.

## Required exact-head automated evidence

Before PR-12 can leave draft status, the unchanged PR head must have:

1. **AI Change Proof** green, including the targeted Browser smoke critical journeys and required changed-Rust formatting/checks.
2. **Full Validation** green, including full database validation, complete Rust validation and Final browser product acceptance.
3. No relaxation of the PR-11 CSP/offline network boundary, console/pageerror/WASM guards, retry policy or deterministic fixture contract.
4. No unresolved review blockers.

Package, Production Foundation and Air-gapped Appliance may run because of repository policy. They are useful incidental evidence but are not substitutes for the PR-12 gates above.

## Human accessibility evidence still required

The plan separately requires manual keyboard and screen-reader acceptance for critical journeys. Automated axe/focus tests do not replace that human acceptance.

Use `docs/security/pr-12-manual-accessibility-acceptance.md` to record the tester, date, environment, assistive technology, journeys and findings. Do not mark PR-12 fully satisfied until that record is completed, or until an explicit owner/date risk acceptance is recorded where the plan permits one.

## Merge rule

After the final implementation/docs commit, record the exact PR head and successful workflow run numbers in the PR description. Merge only if that exact head remains unchanged and all PR definition-of-done conditions are satisfied.
