# API/server-function inventory and security architecture

The authoritative endpoint inventory is `../../packages/api/endpoint_authorization_manifest.psv`. It is pipe-delimited and records endpoint kind, policy, allowed roles, tenant scope, object scope, access type, resource class, audit requirement, owner and exception expiry.

## Security model

- Browser requests authenticate through the supported session/auth boundary; public signup is disabled in production.
- Server functions re-authorize role, school/tenant and object ownership; browser routing is not an authorization boundary.
- Database access uses a constrained non-superuser `NOBYPASSRLS` application role plus transaction-scoped database authorization context. Migration/bootstrap authority is separate.
- PostgreSQL RLS complements application authorization for governed data boundaries.
- Qdrant retrieval is downstream of database authorization and exact authorized asset filters; unpublished/archived knowledge must not be widened by vector search.
- Provider credentials/destinations and the internal AI-Gateway token do not enter browser code.
- The AI Gateway requires internal authentication and authoritative school identity, fixes approved provider origins/models, disables redirects, and is the only approved AI-egress service.
- Host ingress is gateway-only on 80/443. Database, Qdrant, Supabase internals and AI internals do not publish direct host ports.

## Manifest interpretation

`Public` is limited to explicitly public health/auth routes. Role policies such as `TeacherOnly`, `StudentOnly`, `ParentOnly`, `SchoolManagerOnly` and `PlatformAdminOnly` require the matching authenticated authority and the manifest's tenant/object scope. `SessionOwner` is bound to the authenticated user/session. Rows whose policy is `Disabled` are unavailable and are excluded from the first contracted product scope.

Examples of enabled authorization contracts include:

- `assignments/my_assignments` — Student, session-student scope;
- `submissions/submit` — Student, student-assignment scope;
- `teacher/submissions/grade` — Teacher, teacher-submission scope;
- `classes/student/grades` — Student, student-class scope;
- `parent/child/grades` and parent-scoped equivalents — Parent, linked-child scope;
- governed knowledge administration/selection/search with PlatformAdmin/SchoolManager/Teacher-specific scopes.

The feature matrix is validated against both this manifest and `product-capabilities.json`. Adding a new endpoint is not enough to make a capability contract-ready: its UI/product status, negative authorization tests, documentation and release evidence must also be complete.

## Audit and exception handling

Manifest rows flag operations requiring security audit evidence. Any temporary authorization exception includes an expiry and remains an engineering/security debt item; commercial documentation must describe only the supported product behavior, not an expired exception as a permanent entitlement.

## Related architecture evidence

- `../adr/0001-offline-first-production-architecture.md`
- `../adr/0002-controlled-external-ai.md`
- `../adr/0003-air-gapped-appliance-and-ghcr.md`
- `../adr/0004-production-operations.md`
- `../adr/0005-transaction-scoped-rls.md`
- `../security/production-threat-model.md`
- `../security/controlled-external-ai-threat-model.md`
- `../security/production-operations-threat-model.md`
