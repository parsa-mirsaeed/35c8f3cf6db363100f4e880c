# ADR 0005: Transaction-scoped PostgreSQL row-level security

## Status

Accepted and implemented by PR-03 of the EduTalent Production Readiness plan.

## Context

EduTalent uses PostgreSQL row-level security as a defense-in-depth tenant boundary for school data. The previous runtime configured actor values through a pooled connection and granted the long-running application role `BYPASSRLS`. That combination could not provide a dependable boundary:

- a context statement and the protected query could execute on different pool connections;
- session-scoped values could survive reuse of a pooled connection;
- `BYPASSRLS` made policy correctness irrelevant for the application and worker identities;
- repository-owned pools made it impossible to prove that every protected query used the same transaction as the actor context.

## Decision

EduTalent treats one authenticated request or one explicitly scoped background job as one PostgreSQL authorization transaction.

The runtime begins an `AuthorizedTx`, installs canonical transaction-local values with `set_config(..., true)`, and executes protected repository queries through the same pinned connection. The canonical settings are:

- `app.user_id`;
- `app.user_role`;
- `app.school_id`;
- `app.elevated_operation`, which defaults to `false` and is reserved for narrowly defined school-scoped system jobs.

The application state exposes a raw pool only to the authentication/bootstrap boundary and readiness/public operations. Protected repositories receive an `AuthorizedPool` executor facade. The facade fails closed when no authorized task-local transaction is active. Repository methods that historically began nested transactions use PostgreSQL savepoints inside the request transaction; dropping an uncommitted nested transaction marks the outer transaction rollback-only.

Authentication middleware owns the outer transaction. It commits only after a successful response and rolls back on authentication failure, authorization failure, server error, or an explicit rollback-only mark. Actor school and role are resolved canonically before protected work proceeds.

The long-running application and worker database role is configured `NOBYPASSRLS`, `NOINHERIT`, `NOCREATEDB`, `NOCREATEROLE`, `NOREPLICATION`, and non-superuser. It may not retain memberships that permit `SET ROLE`. The role cannot execute the retired `set_app_context` helper and cannot write migration registries. Every RLS-enabled application table is set to `FORCE ROW LEVEL SECURITY`.

CI proves:

- the runtime role attributes, including `NOBYPASSRLS`;
- absence of inherited or `SET ROLE` memberships;
- forced RLS on every RLS-enabled application table;
- canonical helper-function settings;
- denial of schema, role, and database creation;
- denial of migration-registry writes;
- concurrent school transactions cannot observe each other's rows;
- transaction context is absent after commit and rollback;
- pool-scoped RLS setup and repository-owned `PgPool` access cannot return unnoticed.

## Consequences

A protected query without an active `AuthorizedTx` fails instead of silently using the raw pool. This makes missing request or worker scoping visible during tests and startup rather than weakening tenant isolation.

Repository code keeps SQLx query shapes while the executor facade centralizes transaction pinning. The facade introduces synchronization around the active transaction, so repository operations within one request are deliberately serialized at the database connection boundary. This matches PostgreSQL transaction semantics and avoids unsupported concurrent use of one connection.

Background workers must create an explicit actor context for each school-scoped unit of work. A generic elevated bypass is not permitted; policies must explicitly recognize the bounded system-job role and operation flag where needed.

Readiness checks and public/bootstrap database operations remain on the raw pool because they do not depend on tenant context. Their allowed surface is intentionally small and source-inventoried.

## Rejected alternatives

- Keeping `BYPASSRLS` and relying only on application predicates: does not provide an independent database boundary.
- Setting context on `PgPool`: does not pin context and queries to one connection.
- Session-scoped `SET` values: can leak across pool reuse.
- Opening a new transaction inside every repository method: fragments one request across unrelated authorization contexts.
- A universal system-worker bypass: recreates the privileged long-running role that PR-03 removes.
