-- PR-03: make transaction-local authorization context the canonical RLS boundary.
--
-- The long-running application and worker roles are configured separately as
-- NOBYPASSRLS. This migration keeps policies fail-closed when context is absent,
-- forces RLS for every application table that already enables it, and removes
-- the obsolete pool-scoped context function from the runtime grant surface.

CREATE OR REPLACE FUNCTION public.get_user_id()
RETURNS UUID
LANGUAGE SQL
STABLE
PARALLEL SAFE
SET search_path = pg_catalog, public
AS $$
    SELECT NULLIF(current_setting('app.user_id', true), '')::UUID
$$;

CREATE OR REPLACE FUNCTION public.get_role()
RETURNS TEXT
LANGUAGE SQL
STABLE
PARALLEL SAFE
SET search_path = pg_catalog, public
AS $$
    SELECT NULLIF(current_setting('app.user_role', true), '')
$$;

CREATE OR REPLACE FUNCTION public.get_school_id()
RETURNS UUID
LANGUAGE SQL
STABLE
PARALLEL SAFE
SET search_path = pg_catalog, public
AS $$
    SELECT NULLIF(current_setting('app.school_id', true), '')::UUID
$$;

CREATE OR REPLACE FUNCTION public.get_elevated_operation()
RETURNS BOOLEAN
LANGUAGE SQL
STABLE
PARALLEL SAFE
SET search_path = pg_catalog, public
AS $$
    SELECT COALESCE(NULLIF(current_setting('app.elevated_operation', true), '')::BOOLEAN, FALSE)
$$;

CREATE OR REPLACE FUNCTION public.is_school_manager()
RETURNS BOOLEAN
LANGUAGE SQL
STABLE
PARALLEL SAFE
SET search_path = pg_catalog, public
AS $$
    SELECT public.get_role() = 'SchoolManager'
$$;

-- Preserve the historical function signature for migration compatibility, but
-- make it transaction-local, reset the elevated flag, and keep it outside the
-- long-running runtime grant surface. New Rust code uses AuthorizedTx directly.
CREATE OR REPLACE FUNCTION public.set_app_context(
    p_user_id UUID,
    p_role TEXT,
    p_school_id UUID
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY INVOKER
SET search_path = pg_catalog, public
AS $$
BEGIN
    PERFORM set_config('app.user_id', COALESCE(p_user_id::TEXT, ''), TRUE);
    PERFORM set_config('app.user_role', COALESCE(p_role, ''), TRUE);
    PERFORM set_config('app.school_id', COALESCE(p_school_id::TEXT, ''), TRUE);
    PERFORM set_config('app.elevated_operation', 'false', TRUE);
END;
$$;

REVOKE EXECUTE ON FUNCTION public.set_app_context(UUID, TEXT, UUID) FROM PUBLIC;

-- FORCE RLS for every repository-owned application table that has RLS enabled.
-- Migration registries are protected through grants/revokes rather than tenant
-- policies and are intentionally excluded.
DO $force_rls$
DECLARE
    target RECORD;
BEGIN
    FOR target IN
        SELECT namespace.nspname AS schema_name, relation.relname AS table_name
        FROM pg_class AS relation
        JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relkind IN ('r', 'p')
          AND relation.relrowsecurity
          AND relation.relname NOT IN ('_sqlx_migrations', 'edutalent_migration_files')
        ORDER BY relation.relname
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.%I FORCE ROW LEVEL SECURITY',
            target.schema_name,
            target.table_name
        );
    END LOOP;
END
$force_rls$;

COMMENT ON FUNCTION public.get_elevated_operation() IS
    'Returns the transaction-local bounded system-job flag; false when context is absent.';
COMMENT ON FUNCTION public.set_app_context(UUID, TEXT, UUID) IS
    'Legacy migration compatibility only. Runtime code must use a pinned AuthorizedTx.';
