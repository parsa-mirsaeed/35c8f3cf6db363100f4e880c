# PR-12 Manual Keyboard and Screen-Reader Acceptance

## Follow-up ownership

This document is one evidence section of the dedicated **manual/external production acceptance** follow-up. Automated/browser PR-12 originally merged at main commit `52365ddfbb3196ee46261cbc771bc957c4467882`; the repository owner subsequently directed that **all human-needed production verifications** be consolidated into the same follow-up PR.

The final automated production-readiness sequence through plan PR-15 is now merged on `main` at `102c2ef56edf31dc5bff7982b234481a7fcbc43b`. PR-15's exact validated source head was `45daddb1fb9763388a9df64450c9f1aeab53225c`, and Final Release Acceptance run `31971376680` completed successfully while explicitly recording `ready_for_contracted_production: false`.

These automated facts are only the test baseline. This split does **not** mean keyboard or screen-reader acceptance has passed. A human tester must complete and sign the record below against the exact installed frozen release candidate. Automated axe/Playwright evidence must not be substituted for the human result.

The cross-cutting manual/external record is `docs/security/manual-external-production-acceptance.md`.

If testing finds a WCAG 2.2 AA defect, fix it in code with regression coverage before sign-off unless the production-readiness process explicitly permits a documented risk acceptance with owner, date, rationale, and review/expiry. If the candidate source or installed artifact changes in a way that could affect accessibility, rerun the impacted acceptance journeys.

## Frozen automated baseline

- Repository: `parsa-mirsaeed/35c8f3cf6db363100f4e880c`
- Final automated baseline on `main`: `102c2ef56edf31dc5bff7982b234481a7fcbc43b`
- PR-15 exact validated source head: `45daddb1fb9763388a9df64450c9f1aeab53225c`
- Final Release Acceptance workflow run: `31971376680`
- Final Release Acceptance artifact digest: `sha256:c4bb60cac2cd472019da1f2e06be6746e69b6667cf4253e5ffcf769f08902297`
- Automated classification: `ready for final validation`
- Automated contracted-production decision: `false`

## Human test record

Populate during the actual accessibility session. Do not pre-fill PASS from CI.

- Exact installed release/source SHA: `102c2ef56edf31dc5bff7982b234481a7fcbc43b` baseline; verify before testing
- Installed signed artifact/build digest:
- Tester:
- Date (UTC):
- Browser / version:
- Operating system:
- Screen reader / version:
- Keyboard-only pass completed: yes / no
- Screen-reader pass completed: yes / no

## Critical journeys

For every journey below, verify that the user can perceive the current context, reach and operate all controls without a pointer, understand status/error feedback, and recover/continue without focus loss.

### 1. Authentication and session termination

- Reach email, password and Sign In in visible/logical keyboard order.
- Sign in as an active role and confirm the role-appropriate dashboard is announced/understandable.
- Reach Sign Out by keyboard and activate it.
- Confirm the logged-out/login state is clear and protected content is no longer reachable.

Result / notes:

### 2. Role navigation

- Navigate a representative School Manager or Teacher dashboard using keyboard only.
- Confirm navigation labels, active context and page/section headings are understandable with the screen reader.
- Confirm no keyboard trap in desktop or responsive/mobile navigation controls.

Result / notes:

### 3. Student assignment submission

- As the seeded Student, reach an assigned item by keyboard.
- Open the assignment details and Start Assignment without a pointer.
- Enter representative text into the work editor.
- Submit and confirm the resulting status/transition is understandable.

Result / notes:

### 4. Teacher grading and feedback modal

- As the authorized Teacher, reach the submitted work by keyboard.
- Open Grade Submission.
- Confirm the dialog has an announced accessible name and focus enters the dialog.
- Tab and Shift+Tab through the dialog controls in a logical order.
- Enter a valid grade and feedback, save, and confirm the modal closes without leaving focus in an unusable state.
- Confirm the operation and any validation/error message are understandable to the screen reader.

Result / notes:

### 5. Student persisted grade view

- Return as the Student and navigate to Grades.
- Open class grade details.
- Confirm assignment title, grade/points and date are understandable in both English/LTR and Persian/RTL contexts.
- Confirm the grade-details dialog is keyboard operable and announced as a dialog.

Result / notes:

### 6. Persian / RTL pass

- Switch to Persian.
- Confirm document direction and navigation order remain usable.
- Confirm dates and numeric grades remain readable and are not visually or audibly reordered into misleading values.
- Re-check one modal flow for focus and announcement behavior in RTL.

Result / notes:

## Findings

List every defect with severity, reproduction steps and issue/PR reference. WCAG 2.2 AA findings must be fixed before acceptance unless the production-readiness process permits a documented risk acceptance with an explicit owner and date.

- Findings:
- Accepted risks (owner + date + rationale + review/expiry), if any:

## Sign-off

- Keyboard acceptance: PASS / FAIL
- Screen-reader acceptance: PASS / FAIL
- Tester name:
- Sign-off date:
- Final exact source SHA verified unchanged:
- Installed signed artifact/digest verified:
