-- PR-03 finalizer: run only after the historical RLS policy migration has
-- completed successfully. The canonical migration runner may defer historical
-- files until a later dependency pass, so the earlier PR-03 migration cannot
-- assume every table has enabled RLS when it first executes.

DO $prerequisites$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM public.edutalent_migration_files
        WHERE path = 'migrations/20260103000001_enable_rls_policies.sql'
    ) THEN
        RAISE EXCEPTION
            'transaction-scoped RLS finalizer requires the legacy RLS policy migration';
    END IF;
END
$prerequisites$;

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

-- The final state must be fail-closed even when future historical migrations
-- are replayed through the dependency-pass runner.
DO $verify_force_rls$
DECLARE
    missing_force text[];
BEGIN
    SELECT array_agg(relation.relname ORDER BY relation.relname)
    INTO missing_force
    FROM pg_class AS relation
    JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
    WHERE namespace.nspname = 'public'
      AND relation.relkind IN ('r', 'p')
      AND relation.relrowsecurity
      AND NOT relation.relforcerowsecurity
      AND relation.relname NOT IN ('_sqlx_migrations', 'edutalent_migration_files');

    IF missing_force IS NOT NULL THEN
        RAISE EXCEPTION 'RLS-enabled application tables are not forced: %', missing_force;
    END IF;
END
$verify_force_rls$;
