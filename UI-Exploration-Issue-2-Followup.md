# Issue 2 follow-up — Teacher/Parent provisioning and required-field contract

This follow-up extends Issue 2 in `UI-Exploration-Fix-Plan.md`. It is kept on the same exploration branch so it can be folded back into the consolidated implementation plan before the fix PR.

No real user email from interactive exploration is included here. Do not add real exploration addresses to repository artifacts, fixtures, commands, logs, screenshots attached to PRs/issues, or automated tests.

## Additional user-visible behavior

The same production failure reproduced for all three School Manager account-creation tabs:

- Student creation -> HTTP 404;
- Teacher creation -> HTTP 404;
- Parent creation -> HTTP 404.

All three active forms call the same `user_management::create_user` server function, which maps to `/api/user_management/create`. Production authorization intentionally classifies that endpoint as `Disabled`, so the deny-by-default middleware returns HTTP 404 before the handler executes.

Therefore Issue 2 is not student-specific. It is a production capability-contract defect affecting the whole School Manager Student / Teacher / Parent provisioning surface.

## Required-field indicator and validation mismatch

Interactive exploration also exposed a separate but closely related form-contract defect: visible required markers and actual browser validation do not consistently agree.

### Teacher form

Observed/verified examples:

- phone input has HTML `required: true`, but the visible label does not indicate that it is required;
- department is visually presented as required in the current UI, but the `<select>` has no HTML `required` constraint;
- subjects are visually presented as required in the current UI, but the multi-select has no HTML `required` constraint;
- employee ID and hire date are both browser-required and visually presented as required.

The practical result is confusing behavior: a School Manager can be blocked by the browser on a field that appears optional, while other fields that appear mandatory can pass client validation empty.

### Parent form

Observed/verified examples:

- phone input has HTML `required: true`, but the visible label does not indicate that it is required;
- associated-students is visually presented as required in the current UI, while the multi-select has no HTML `required` constraint;
- full name, email, and parent ID are browser-required and visually presented as required.

### Student form

The consolidated fix must audit the Student form too rather than assuming it is internally consistent. Required markers, HTML constraints, typed payload validation, and server-side validation must describe the same product contract.

## Root cause classification

There are two related but distinct defects within Issue 2:

1. **Provisioning endpoint mismatch** — active UI calls a production endpoint deliberately marked `Disabled`, causing deterministic HTTP 404 before any account mutation.
2. **Validation/UX contract drift** — translation/label required markers, HTML `required` attributes, payload semantics, and eventual server-side validation are not maintained from one authoritative field contract.

Fixing only the 404 would leave confusing and potentially incomplete form validation. Fixing only the labels would leave account creation completely unavailable.

## Required repository implementation additions

The consolidated Issue 2 implementation must additionally:

1. migrate Student, Teacher, and Parent creation UIs together to the chosen hardened production provisioning capability;
2. define the authoritative required/optional field contract for each role;
3. make visible required indicators derive from that same contract in every supported locale;
4. make HTML/browser constraints match the visible contract;
5. enforce the same required/optional semantics again on the server, since browser validation is never an authorization or integrity boundary;
6. provide explicit validation errors for missing/invalid required values rather than relying only on browser-native blocking;
7. ensure optional fields truly remain optional through serialization and database/service layers;
8. ensure fields advertised as required cannot silently arrive empty because a `<select>` or multi-select omitted validation;
9. add focused UI tests for required-marker/constraint parity across Student, Teacher, and Parent forms;
10. avoid duplicating validation rules independently across translations, UI controls, and server handlers where a shared typed contract can prevent drift.

## Teacher-specific provisioning requirements

When Teacher provisioning is implemented, the PR must also verify that:

- the created Auth identity and local `users` UUID are identical;
- the local Teacher role/profile is created coherently;
- selected subject(s) have a defined persisted representation rather than being only display metadata;
- selected class assignments reference only classes from the authenticated manager's school;
- class assignments are persisted only after the teacher record exists;
- partial failure cannot leave an Auth-only or partially assigned teacher account;
- duplicate employee identifiers, if they are intended to be unique, are enforced by a server/database invariant rather than browser behavior alone.

## Parent-specific provisioning requirements

When Parent provisioning is implemented, the PR must also verify that:

- the created Auth identity and local `users` UUID are identical;
- the local Parent role/account representation is created coherently;
- every selected associated student belongs to the authenticated manager's school;
- cross-school and non-Student associations are rejected without revealing unrelated tenant data;
- parent/student relationships are persisted atomically with the local provisioning step where feasible;
- a failure while linking one or more students cannot be reported as complete success;
- the product decision on whether at least one associated student is mandatory is explicit and reflected identically by label, browser validation, server validation, and tests.

## Additional regression tests

Add focused tests covering:

- Student, Teacher, and Parent creation UI calls only active production endpoints;
- the disabled legacy `user_management/create` endpoint remains 404 unless intentionally replaced through reviewed policy change;
- Teacher phone required indicator matches its validation requirement;
- Teacher department/subjects required indicators match actual validation semantics;
- Parent phone required indicator matches its validation requirement;
- Parent associated-student required indicator matches actual product semantics;
- submitting each form with each required field missing produces a clear local/server validation result and no mutation;
- optional fields can be omitted without false browser rejection;
- required fields cannot be bypassed by direct API calls;
- all role-specific cross-school references are rejected;
- no real exploration email appears in fixtures or snapshots.

## Updated acceptance additions for Issue 2

Issue 2 is not complete until:

- [ ] Student creation works through a supported hardened production endpoint;
- [ ] Teacher creation works through a supported hardened production endpoint;
- [ ] Parent creation works through a supported hardened production endpoint;
- [ ] all three forms no longer target a deliberately disabled mutation route;
- [ ] visual required markers match actual required fields for all three forms and supported locales;
- [ ] HTML constraints match the same contract;
- [ ] server-side validation independently enforces the contract;
- [ ] Teacher class/subject persistence is truthful and same-school scoped;
- [ ] Parent/student associations are truthful and same-school scoped;
- [ ] partial provisioning failure is compensated or represented by a safe durable state;
- [ ] focused UI/API authorization and validation tests are green;
- [ ] no real exploration email is committed.
