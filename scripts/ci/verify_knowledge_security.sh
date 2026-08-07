#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
    missing_rls_tables text[];
    missing_policies text[];
    insecure_policy_count integer;
    teacher_policy text;
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM roles
        WHERE name = 'PlatformAdmin'::role_name
          AND permissions @> '{"publish_knowledge_assets": true}'::jsonb
    ) THEN
        RAISE EXCEPTION 'PlatformAdmin role row or least-privilege permissions are missing';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM roles
        WHERE name = 'PlatformAdmin'::role_name
          AND permissions ?| ARRAY['can_manage_users', 'can_manage_classes', 'manage_system_settings']
    ) THEN
        RAISE EXCEPTION 'PlatformAdmin role unexpectedly contains school-management permissions';
    END IF;

    SELECT array_agg(required_name ORDER BY required_name)
    INTO missing_rls_tables
    FROM (
        VALUES
            ('knowledge_assets'),
            ('knowledge_source_files'),
            ('knowledge_ocr_texts'),
            ('knowledge_chunks'),
            ('teacher_asset_selections'),
            ('ingestion_jobs'),
            ('knowledge_audit_logs')
    ) AS required(required_name)
    WHERE NOT EXISTS (
        SELECT 1
        FROM pg_class relation
        JOIN pg_namespace namespace ON namespace.oid = relation.relnamespace
        WHERE namespace.nspname = 'public'
          AND relation.relname = required_name
          AND relation.relrowsecurity
    );

    IF missing_rls_tables IS NOT NULL THEN
        RAISE EXCEPTION 'RLS is not enabled for governed knowledge tables: %', missing_rls_tables;
    END IF;

    SELECT array_agg(required_policy ORDER BY required_policy)
    INTO missing_policies
    FROM (
        VALUES
            ('knowledge_assets_scoped_select'),
            ('knowledge_assets_scoped_insert'),
            ('knowledge_assets_admin_update'),
            ('knowledge_source_files_scoped_select'),
            ('knowledge_source_files_scoped_insert'),
            ('knowledge_source_files_admin_write'),
            ('knowledge_ocr_texts_admin_all'),
            ('knowledge_chunks_admin_all'),
            ('teacher_asset_selection_owner'),
            ('ingestion_jobs_admin_all'),
            ('knowledge_audit_logs_admin_select'),
            ('knowledge_audit_logs_actor_insert')
    ) AS required(required_policy)
    WHERE NOT EXISTS (
        SELECT 1
        FROM pg_policies
        WHERE schemaname = 'public'
          AND policyname = required_policy
    );

    IF missing_policies IS NOT NULL THEN
        RAISE EXCEPTION 'Missing governed knowledge policies: %', missing_policies;
    END IF;

    SELECT COUNT(*)
    INTO insecure_policy_count
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename LIKE 'knowledge%'
      AND (COALESCE(qual, '') || COALESCE(with_check, '')) LIKE '%app.current_%';

    IF insecure_policy_count <> 0 THEN
        RAISE EXCEPTION 'Governed knowledge policies still reference obsolete app.current_* settings';
    END IF;

    SELECT qual
    INTO teacher_policy
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename = 'knowledge_assets'
      AND policyname = 'knowledge_assets_scoped_select';

    IF teacher_policy NOT LIKE '%get_role()%'
       OR teacher_policy NOT LIKE '%get_school_id()%'
       OR teacher_policy NOT LIKE '%published%' THEN
        RAISE EXCEPTION 'Teacher asset visibility policy is not school- and publication-scoped: %', teacher_policy;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'material_embeddings'
          AND column_name = 'current_batch'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'material_embeddings'
          AND column_name = 'total_batches'
    ) OR NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'material_embeddings'
          AND column_name = 'cancelled'
    ) THEN
        RAISE EXCEPTION 'Material embedding progress/cancellation columns are incomplete';
    END IF;

    IF pg_get_functiondef('prevent_teacher_document_ingestion()'::regprocedure)
       NOT LIKE '%JOIN roles%' THEN
        RAISE EXCEPTION 'Teacher document-ingestion guard does not resolve the canonical roles table';
    END IF;
END
$$;

-- Exercise the teacher document-ingestion boundary inside a rolled-back fixture.
BEGIN;
DO $$
DECLARE
    school_uuid uuid;
    subject_uuid uuid;
    class_uuid uuid;
    teacher_user_uuid uuid;
    teacher_role_uuid uuid;
BEGIN
    INSERT INTO schools (name)
    VALUES ('Knowledge security probe ' || gen_random_uuid()::text)
    RETURNING id INTO school_uuid;

    INSERT INTO subjects (code, name)
    VALUES ('KSP-' || substr(gen_random_uuid()::text, 1, 8), 'Knowledge Security Probe')
    RETURNING id INTO subject_uuid;

    INSERT INTO class_sections (school_id, subject_id, name, term)
    VALUES (school_uuid, subject_uuid, 'Security Probe', 'CI')
    RETURNING id INTO class_uuid;

    SELECT id INTO teacher_role_uuid
    FROM roles
    WHERE name = 'Teacher'::role_name;

    INSERT INTO users (name, email, role_id, school_id)
    VALUES (
        'Knowledge Security Probe Teacher',
        'knowledge-security-' || gen_random_uuid()::text || '@example.invalid',
        teacher_role_uuid,
        school_uuid
    )
    RETURNING id INTO teacher_user_uuid;

    BEGIN
        INSERT INTO class_materials (
            class_section_id, title, material_type, created_by
        ) VALUES (
            class_uuid, 'Blocked teacher PDF', 'document', teacher_user_uuid
        );
        RAISE EXCEPTION 'Teacher document ingestion guard accepted a document';
    EXCEPTION
        WHEN insufficient_privilege THEN NULL;
    END;

    INSERT INTO class_materials (
        class_section_id, title, material_type, external_link, created_by
    ) VALUES (
        class_uuid, 'Allowed teacher link', 'link', 'https://example.invalid', teacher_user_uuid
    );
END
$$;
ROLLBACK;

SELECT 'governed knowledge security invariants verified' AS result;
SQL

bash scripts/ci/verify_transaction_scoped_rls.sh
