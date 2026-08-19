# EduTalent UI Exploration and Consolidated Fix Plan

## Purpose

This document is the running engineering record for issues discovered while interactively exploring the EduTalent production-topology UI before the next implementation PR.

The workflow for this exploration phase is deliberately:

1. reproduce one UI/functional problem at a time;
2. identify the actual backend/database/frontend root cause rather than recording only the visible symptom;
3. prove the diagnosis with the narrowest safe local diagnostic change when necessary;
4. record the issue here with implementation and regression-test requirements;
5. continue exploring without implementing repository fixes yet;
6. after exploration is sufficiently complete, implement the accumulated findings together in one focused PR;
7. run the smallest workflows/tests that fully cover the changed surfaces, then make the PR merge-ready.

A local diagnostic database patch is evidence for diagnosis only. It must not be treated as repository-complete or formal production acceptance evidence. Every durable database correction must be delivered as a forward migration with regression coverage.

---

## Issue Registry

| # | Area | Severity | Status | Summary |
|---|---|---|---|---|
| 1 | Class Management / PostgreSQL RLS / UI feedback | High | Root-caused; local fix proven; repository fix pending | Class creation succeeds but class-list refresh returns HTTP 500 because `enrollments` and `students` SELECT policies recurse; UI also provides no success confirmation and discards refresh errors. |
| 2 | School Manager User Creation / endpoint capability contract / account provisioning | High | Root-caused; repository fix pending | Student creation UI calls `/api/user_management/create`, but production policy intentionally marks that server function `Disabled`, so authorization fails closed with HTTP 404 before the handler executes. Existing implementations are not safe to enable unchanged. |

---

# Issue 1 — Class creation succeeds, but class list refresh fails with HTTP 500

## User-visible behavior

From the School Manager Class Management UI:

- creating a class appears not to succeed;
- no positive `class created successfully` feedback is shown;
- the newly created class does not appear in the class list;
- browser console repeatedly reports:

```text
api/classes/get_school_classes: Failed to load resource: server responded with status 500
```

However, direct database inspection proves that the class row was inserted successfully.

Observed created row during exploration:

```text
id:         8bb60147-a83d-4a66-9e95-becb8468d8db
name:       ۵/۳
term:       1405/1
school_id:  00000000-0000-0000-0000-000000000001
subject_id: 491253de-1314-4c9d-87f6-58d4dc7e540e
subject:    Mathematics / MATH
```

The insert path itself therefore works.

## Exact application behavior

`create_class_section()` performs the class insert successfully.

The UI then closes the create-class modal and restarts the class-list resource. The class-list resource calls `get_school_classes()`.

`get_school_classes()` performs a query that includes:

- `class_sections`;
- `subjects`;
- a correlated `COUNT(*)` over `enrollments`;
- `teaching_assignments`;
- `teachers`;
- `users`.

The UI loader currently converts the class-list call with `.await.ok()`, so the detailed server error is discarded. This makes a successful insert followed by a failed refresh look like a failed create operation.

There is also no success-toast/success-message path after a successful class creation.

## Root cause

The production database uses forced PostgreSQL Row Level Security.

The relevant policies create a recursion cycle:

```text
enrollments_select_policy
    -> reads students
    -> students_select_policy
    -> reads enrollments
    -> enrollments_select_policy
    -> ...
```

Exact failure reproduced as the dedicated `edutalent_app` role under the same School Manager transaction-scoped RLS context used by the application:

```text
ERROR: infinite recursion detected in policy for relation "enrollments"
```

The problematic authorization relationships are conceptually:

```text
students_select_policy
    teacher visibility
    -> enrollments
    -> teaching_assignments
    -> teachers

enrollments_select_policy
    student / parent visibility
    -> students
```

Because both RLS-protected relations refer back to each other through policy expressions, PostgreSQL recursively expands the policies and aborts the query.

This is a policy-design defect, not corrupt class data, missing permissions, or a bad School Manager account.

## Security facts already verified

The runtime database identity remains correctly constrained:

```text
role:        edutalent_app
superuser:   false
bypassrls:   false
createrole:  false
createdb:    false
login:       true
```

`edutalent_app` has the expected SELECT privileges on the participating tables, so the failure is not a missing table grant.

The exact School Manager transaction context was verified:

```text
app.user_id   = 19a958d1-94a2-4466-9d36-fc4d1172ee83
app.user_role = SchoolManager
app.school_id = 00000000-0000-0000-0000-000000000001
```

The user and school rows are visible correctly under that context.

## Local diagnostic fix that proved the diagnosis

A local-only database helper was introduced to remove the direct `enrollments -> students` policy dependency while retaining the same user/parent authorization semantics.

The diagnostic helper was shaped as a narrow `SECURITY DEFINER` function:

```sql
public.enrollment_student_actor_matches(p_student_id uuid)
```

Behavior:

- returns true only when the supplied student belongs to the current transaction user as either:
  - `students.user_id = get_user_id()`, or
  - `students.parent_id = get_user_id()`;
- uses a fixed `search_path`;
- is `STABLE`;
- is not executable by `PUBLIC`;
- is executable only by the dedicated application runtime role for the diagnostic test;
- does not grant `BYPASSRLS` to `edutalent_app`;
- does not disable or weaken RLS globally.

`enrollments_select_policy` was locally recreated so its student/parent branch calls that bounded helper instead of directly selecting from `students`.

After that diagnostic change, the exact previously failing class-list query succeeded under `edutalent_app`:

```text
id                                   name  term    subject_name  subject_code  student_count
8bb60147-a83d-4a66-9e95-becb8468d8db ۵/۳  1405/1  Mathematics   MATH          0
```

The Class Management UI then displayed the class correctly.

This proves that the RLS recursion is the root cause of the HTTP 500.

## Required repository implementation

### A. Add a new forward migration

Do **not** edit the historical migration that originally created the RLS policies. EduTalent's migration runner stores migration-file checksums and intentionally fails closed if an already-applied migration changes.

Add a new timestamped migration under `migrations/` that:

1. introduces the narrow relationship helper needed to break the recursion;
2. uses `SECURITY DEFINER` only for the minimum relationship check required;
3. uses a fixed `SET search_path = pg_catalog, public`;
4. is `STABLE` where appropriate;
5. schema-qualifies governed objects/functions;
6. revokes function execution from `PUBLIC`;
7. grants only the necessary runtime execution privilege;
8. drops and recreates `enrollments_select_policy` without a direct `students` lookup that recursively re-enters `students_select_policy`;
9. preserves all intended authorization behavior;
10. keeps FORCE ROW LEVEL SECURITY enabled;
11. does not grant the long-running application identity `BYPASSRLS`, elevated role membership, or broader schema privileges.

### B. Preserve intended authorization semantics

Regression behavior must prove all of the following:

#### Student

- can see the student's own enrollment;
- cannot see another student's unrelated enrollment.

#### Parent

- can see an enrollment belonging to their child;
- cannot see an unrelated student's enrollment.

#### Teacher

- can see enrollments for a class assigned to that teacher;
- cannot see enrollments for an unrelated class/school.

#### School Manager

- can see enrollments/classes within the manager's current school;
- cannot see cross-school enrollment/class data.

#### Runtime role

- remains non-superuser;
- remains `NOBYPASSRLS`;
- retains no privileged role memberships;
- cannot execute any helper that it does not need;
- cannot obtain global/unscoped student data from the helper.

### C. Add an explicit RLS recursion regression test

The regression suite must include the exact query shape that exposed the production defect, executed as the dedicated NOBYPASSRLS runtime identity inside a transaction carrying canonical application context.

At minimum:

```text
SchoolManager context
    -> SELECT class_sections
    -> JOIN subjects
    -> correlated COUNT(*) FROM enrollments
    -> must return successfully
    -> must not raise recursive-policy errors
```

The test should fail if either `students_select_policy` or `enrollments_select_policy` later reintroduces a recursion cycle.

Prefer extending the repository's existing transaction-scoped RLS verification workflow rather than creating an unrelated high-cost workflow.

### D. Fix class-management success/error UX

After a successful `create_class_section()` call:

- show a clear success confirmation, e.g. `Class created successfully`;
- restart/refetch the list;
- keep the newly created class visible when the refresh succeeds.

If the insert succeeds but the refresh fails:

- do not imply that class creation failed;
- show a differentiated message such as:

```text
Class was created, but the class list could not be refreshed.
```

The class-list loader must not silently discard server errors with `.await.ok()`.

Expose an actionable error state to the UI while keeping sensitive internal database details out of user-facing messages.

### E. Server-side observability

`get_school_classes()` currently maps database errors into a server-function error, but the production investigation produced no useful application log output for the failed request.

As part of the consolidated PR, review whether this server function should emit structured server-side error logging with safe contextual fields such as:

- operation: `get_school_classes`;
- authenticated role;
- school identifier where allowed by logging policy;
- error class/code;

without leaking secrets or authentication tokens.

## Acceptance criteria for Issue 1

Issue 1 is complete only when all of the following are true from clean repository state:

- [ ] new forward migration applies successfully;
- [ ] historical migration files remain unchanged;
- [ ] `edutalent_app` remains `NOBYPASSRLS`;
- [ ] exact class-list query no longer causes RLS recursion;
- [ ] School Manager class list loads successfully;
- [ ] creating a class persists the row;
- [ ] created class appears in the UI after refresh;
- [ ] successful creation shows a success confirmation;
- [ ] failed list refresh produces visible but safe error feedback;
- [ ] own/student/parent/teacher/manager authorization regression cases pass;
- [ ] cross-school/cross-user negative authorization cases pass;
- [ ] relevant migration/RLS tests are green;
- [ ] relevant API/web tests are green;
- [ ] no broad RLS bypass or permissive policy workaround is introduced.

## Local exploration state

During exploration, the current local database has the diagnostic RLS helper/policy adjustment applied manually. This allows continued UI exploration.

Therefore:

- it is useful for discovering additional UI problems;
- it is **not** pristine exact-head database state anymore;
- it must **not** be used as proof that the repository already contains the fix;
- final verification for the consolidated PR must be repeated from clean repository-controlled migrations.

---

# Issue 2 — School Manager student creation UI calls an intentionally disabled provisioning endpoint

## User-visible behavior

From School Manager → User Management → create student:

- submitting the student form fails;
- the UI displays a generic server-function error indicating HTTP 404;
- browser console reports a failed request to:

```text
/api/user_management/create
```

The student email used during exploration is real and is intentionally omitted from this document and must remain absent from repository artifacts, test fixtures, commands, screenshots added to issues/PRs, and logs copied into review material.

## Reproduction steps

1. Sign in as a valid School Manager.
2. Open User Management and the student-creation form.
3. Complete the required fields using test data.
4. Submit.
5. Observe HTTP 404 from `/api/user_management/create` and the generic error banner.

Do not reuse the real exploration email in automated tests; use a generated reserved-domain address instead.

## Expected behavior

School Manager should be able to provision a student through an explicitly supported production capability. The operation should either complete coherently across Auth and application persistence or fail without leaving partial identities/records.

The UI must not expose a creation control whose only backend mutation route is intentionally unavailable.

## Actual behavior and exact route contract

The School Manager creation UI imports:

```rust
api::server_functions::user_management::{create_user, CreateUserPayload, ...}
```

and calls `create_user(payload).await`.

That server function declares:

```rust
#[server(endpoint = "user_management/create")]
```

so the browser correctly generates a request for `/api/user_management/create`.

However, `endpoint_authorization_manifest.psv` deliberately classifies:

```text
server|user_management/create|Disabled|...
```

The deny-by-default endpoint authorization middleware maps every `Disabled` endpoint to `NotFound`, which is returned as HTTP 404 without calling the handler.

Therefore the observed 404 is not a missing file, malformed email, invalid student data, Supabase failure, PostgreSQL failure, or RLS failure. It is a frontend/production-capability contract mismatch: an active production UI invokes an endpoint that production policy intentionally makes undiscoverable.

Because authorization returns 404 before `next.run(request)`, this specific failed request does not execute `create_user()` and therefore does not create an Auth identity or local EduTalent user/student record.

## Why the existing endpoint must not simply be re-enabled

`user_management::create_user` currently performs a multi-system provisioning sequence:

1. resolve the School Manager and role;
2. create the account in Supabase Auth;
3. create the local `users` row;
4. create a role-specific `teachers` or `students` record, or parent links;
5. optionally perform teacher class assignments.

As currently written, the external Auth creation occurs before the local database work and there is no explicit compensation path that deletes/invalidates the newly created Auth identity if later local persistence fails. The subsequent application writes are also not presented as one explicit database transaction. Re-enabling this endpoint unchanged could therefore expose partial-provisioning states such as an Auth identity without a complete local account/role record.

The browser also currently generates a temporary password itself from the first eight characters of a random UUID and sends that password to the server. Production provisioning should not depend on client-generated temporary credentials. Credential generation/invitation semantics belong on the trusted server side with an explicit expiry/rotation/onboarding contract.

The student form also captures a selected class section, but `create_user` only places that value into generic metadata. Its `Student` branch creates the student profile but does not create an `enrollments` row for the selected class. Therefore merely enabling the current endpoint would still leave part of the form's advertised behavior unfulfilled.

## Existing alternative endpoints are not a complete replacement

`students/create` is active for School Managers, but it only creates a `students` domain record for an **already existing**, active, same-school local user whose role is already `Student`. It does not provision the Supabase Auth identity or base `users` row. The UI cannot simply switch to this endpoint as a one-call replacement.

The separate legacy `user_creation/create_student` endpoint is also explicitly `Disabled` in the production manifest. Its current implementation contains a TODO stating that it creates the Supabase Auth user but does not yet create the corresponding local database record. It is therefore not production-complete either.

The correct fix is to define one coherent supported provisioning workflow rather than activating one of several incomplete overlapping paths.

## Security and tenant-isolation requirements

The consolidated implementation must preserve all of the following:

- only an authenticated, active School Manager may provision users for the manager's current school;
- school identity must come from the authenticated transaction/session, never from a browser-trusted school identifier;
- requested role must be restricted to supported provisionable roles;
- parent association must reference an active Parent in the same school;
- any selected class must belong to the same school;
- a student may be enrolled only into an authorized same-school class;
- duplicate email handling must fail predictably and must not create duplicate/local-orphan identities;
- Auth success followed by database failure must be compensated or designed around a durable provisioning state so no login-capable orphan remains;
- database success must never be committed before the Auth identity contract is safely established unless the reverse failure is equally compensated;
- temporary credential generation/invitation must occur server-side and secrets must never be logged;
- user-facing errors must not reveal Supabase service credentials, tokens, internal SQL, or cross-tenant existence information;
- audit-relevant provisioning events should follow the endpoint manifest's required-audit contract.

## Required repository implementation

Before choosing which legacy function to retain, review the overlapping `user_management`, `user_creation`, and `students/create` paths and collapse the production workflow around one authoritative provisioning service/endpoint.

The implementation should:

1. define one active SchoolManager-only account-provisioning endpoint for Student/Teacher/Parent creation, or separate hardened role-specific endpoints if that produces clearer authorization boundaries;
2. derive school scope from the authenticated School Manager, not payload authority;
3. generate invitation/temporary credentials on the server with an explicit secure onboarding contract;
4. provision Supabase Auth and local application state with a documented compensation/saga strategy for cross-system failure;
5. group local `users` + role-specific record + optional enrollment/assignment writes in an explicit database transaction where feasible;
6. validate parent and class references against the authenticated school before mutation;
7. make the selected student class produce an actual `enrollments` relationship if class assignment is part of the UI contract;
8. return a typed result that distinguishes validation, duplicate-account, dependency, and safe internal failure classes;
9. update the production authorization manifest only after the hardened endpoint is complete;
10. remove or retire conflicting legacy browser calls so the active UI cannot target an endpoint marked `Disabled`;
11. preserve deny-by-default behavior for every endpoint that remains disabled;
12. avoid exposing the real exploration email in code, fixtures, tests, docs, PR descriptions, or committed screenshots.

## Required UI behavior

The creation form should:

- call only an active production capability;
- validate required fields locally for usability without treating client validation as authorization;
- prevent duplicate submits while provisioning is in progress;
- show a clear success state only after the full provisioning contract succeeds;
- show a safe actionable failure state when provisioning does not complete;
- not present a selected class as applied unless the enrollment was actually persisted;
- never surface raw `ServerFnError` internals directly as the main user-facing copy;
- use generated test addresses in automated/e2e coverage.

## Required regression tests

Add focused coverage for:

### Endpoint/UI contract

- active School Manager user-creation UI must not reference a manifest endpoint classified `Disabled`;
- anonymous and non-SchoolManager callers are denied;
- the chosen production provisioning endpoint is present in the endpoint inventory/manifest with the intended policy;
- disabled legacy paths remain HTTP 404.

### Successful student provisioning

- one Auth user is created;
- one local `users` row is created with the same identity UUID;
- one `students` row is created for that user;
- school IDs are consistent and come from the manager's school;
- selected same-school class creates the expected enrollment when requested;
- success is returned only after required state is coherent.

### Tenant/object boundaries

- cross-school parent ID is rejected without mutation;
- cross-school class ID is rejected without mutation;
- non-Parent association is rejected;
- non-SchoolManager provisioning is rejected;
- browser-supplied school scope cannot override authenticated scope.

### Failure/compensation

- duplicate Auth email produces no duplicate local account;
- forced local-user insert failure after Auth creation leaves no login-capable orphan, according to the chosen compensation design;
- forced student-profile/enrollment failure does not leave an incorrectly reported success;
- retry behavior is deterministic/idempotent enough to avoid duplicate identities;
- sensitive generated credentials are absent from logs/error telemetry.

### UI

- form displays success only on coherent provisioning success;
- HTTP/validation/dependency failures render safe localized feedback;
- class selection is reflected in persisted enrollment or the UI no longer claims that behavior;
- submit button cannot create duplicate concurrent provisioning requests.

## Acceptance criteria for Issue 2

- [ ] active student-creation UI no longer calls `/api/user_management/create` while that endpoint is disabled;
- [ ] one authoritative production provisioning workflow is selected and implemented;
- [ ] endpoint authorization remains deny-by-default;
- [ ] SchoolManager-only and same-school boundaries are enforced server-side;
- [ ] Supabase Auth UUID and local `users` UUID are consistent;
- [ ] student role-specific row is created coherently;
- [ ] selected class is actually enrolled when the UI presents class assignment;
- [ ] parent/class cross-school negative tests pass;
- [ ] partial failure cannot leave an untracked login-capable Auth identity;
- [ ] temporary credential/invitation handling is server-side and not logged;
- [ ] disabled legacy endpoints remain unreachable unless deliberately replaced and reviewed;
- [ ] UI success/error states accurately represent the durable result;
- [ ] no real exploration email appears in committed artifacts or test data;
- [ ] endpoint inventory/authorization tests and relevant API/web tests are green.

## Interaction with Issue 1

Issue 1's local RLS diagnostic patch allows continued class-list exploration and is independent of the HTTP 404 described here.

If Issue 2's final student-provisioning workflow creates an enrollment for the selected class, its regression tests will also exercise the enrollment RLS policies corrected by Issue 1. The consolidated PR should therefore implement the RLS correction first at migration/test level, then exercise provisioning/enrollment behavior against the corrected policy graph.

---

# Template for the next exploration finding

Each newly discovered issue should be appended before implementation using this structure:

```markdown
# Issue N — concise title

## User-visible behavior
## Reproduction steps
## Expected behavior
## Actual behavior
## Root cause / current evidence
## Security or tenant-isolation implications
## Local diagnostic proof, if any
## Required repository implementation
## Required regression tests
## Acceptance criteria
## Interaction/dependency with earlier issues
```

Do not implement merely because an issue has been recorded unless continuing exploration is blocked. If a temporary diagnostic change is required to continue, document it explicitly as local-only and preserve the final repository implementation for the consolidated PR.

---

# Consolidated PR rule

After UI exploration is complete enough to define the batch, create one focused PR containing the recorded fixes that belong to this exploration cycle.

Before implementation:

1. review all findings for shared root causes;
2. group related database/frontend/API changes coherently;
3. avoid duplicative patches;
4. define the minimum sufficient tests/workflows for the complete batch;
5. preserve production security boundaries and migration immutability.

The PR should not be considered merge-ready until every recorded issue included in its scope has its acceptance criteria satisfied and the relevant workflows are green.
