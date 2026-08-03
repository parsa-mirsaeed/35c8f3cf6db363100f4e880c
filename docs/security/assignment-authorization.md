# Assignment authorization boundary

Assignment APIs are teacher-owned, school-scoped operations. Authentication by
itself is not authorization, and UUID knowledge is never sufficient access.

The production `assignment_functions` module resolves an active canonical
`Teacher` or `Student` from the authenticated request and delegates to
`AuthorizedAssignmentRepository`. Its SQL predicates bind all applicable:

- authenticated user ID;
- canonical role and active-account state;
- teacher or student record;
- school ID;
- teaching assignment or enrollment;
- assignment/custom-assignment ID;
- published state for student reads.

Teacher creation verifies that the class belongs to the teacher's school, the
teacher is assigned to the class, the subject matches the class, any lecture
belongs to that class, and every attached `class_materials` record belongs to
that exact class and teaching assignment. Read, update, delete, publish,
list-custom, and personalization paths do not call the legacy identifier-only
repository mutations.

Publication locks the authorized assignment row and uses the database unique
index on `(assignment_id, student_id)` so retries and concurrent publication do
not create duplicate student work. Only active same-school enrolled students
are included in fan-out.

Error responses deliberately avoid revealing whether a cross-school identifier
exists. Database details are logged server-side without returning SQL/provider
content to the browser.

## Verification

PR validation must include:

- migration first-run and replay;
- API and dependent Web server compilation;
- API unit tests;
- the database-backed multi-school assignment authorization matrix;
- concurrent publication idempotency;
- exact-head `AI change gate` evidence.

The process-local AI task remains a documented residual risk and is replaced by
the durable queue in plan PR-05. It is not an authorization bypass: only an
already authorized teacher publication can enqueue that work.
