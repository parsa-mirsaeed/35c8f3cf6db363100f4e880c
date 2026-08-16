# PR-12 Manual Keyboard and Screen-Reader Acceptance

This record exists because PR-12 requires human keyboard and screen-reader acceptance in addition to automated WCAG checks. Do not pre-fill a pass result from CI; a human tester must complete the evidence below on the final release candidate.

## Test record

- PR / exact head SHA:
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

List every defect with severity, reproduction steps and issue/PR reference. WCAG 2.2 AA findings must be fixed before acceptance unless the production-readiness plan permits a documented risk acceptance with an explicit owner and date.

- Findings:
- Accepted risks (owner + date + rationale), if any:

## Sign-off

- Keyboard acceptance: PASS / FAIL
- Screen-reader acceptance: PASS / FAIL
- Tester name:
- Sign-off date:
- Final exact head SHA verified unchanged:
