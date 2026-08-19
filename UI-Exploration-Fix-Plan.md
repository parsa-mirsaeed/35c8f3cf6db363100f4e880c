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
