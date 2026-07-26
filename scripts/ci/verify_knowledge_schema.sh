#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
    missing_tables text[];
    actual_statuses text[];
    expected_statuses text[] := ARRAY[
        'submitted',
        'ocr_pending',
        'ocr_ready',
        'embedding_pending',
        'embedded',
        'published',
        'archived',
        'failed'
    ];
    missing_indexes text[];
    missing_job_columns text[];
BEGIN
    SELECT array_agg(required_name ORDER BY required_name)
    INTO missing_tables
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
    WHERE to_regclass('public.' || required_name) IS NULL;

    IF missing_tables IS NOT NULL THEN
        RAISE EXCEPTION 'Missing governed knowledge tables: %', missing_tables;
    END IF;

    SELECT array_agg(enumlabel ORDER BY enumsortorder)
    INTO actual_statuses
    FROM pg_enum
    WHERE enumtypid = 'knowledge_asset_status'::regtype;

    IF actual_statuses IS DISTINCT FROM expected_statuses THEN
        RAISE EXCEPTION 'knowledge_asset_status mismatch. expected %, got %',
            expected_statuses, actual_statuses;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_enum
        WHERE enumtypid = 'role_name'::regtype
          AND enumlabel = 'PlatformAdmin'
    ) THEN
        RAISE EXCEPTION 'role_name enum does not contain PlatformAdmin';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_trigger
        WHERE tgname = 'trg_validate_knowledge_asset_status'
          AND NOT tgisinternal
    ) THEN
        RAISE EXCEPTION 'Missing knowledge lifecycle transition trigger';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_trigger
        WHERE tgname = 'trg_prevent_teacher_document_ingestion'
          AND NOT tgisinternal
    ) THEN
        RAISE EXCEPTION 'Missing teacher document-ingestion guard trigger';
    END IF;

    SELECT array_agg(required_name ORDER BY required_name)
    INTO missing_indexes
    FROM (
        VALUES
            ('idx_knowledge_assets_school_status'),
            ('idx_knowledge_assets_metadata'),
            ('idx_knowledge_assets_tags_gin'),
            ('idx_knowledge_source_files_asset'),
            ('idx_knowledge_chunks_asset'),
            ('idx_teacher_asset_selections_lookup'),
            ('idx_ingestion_jobs_asset_created'),
            ('idx_ingestion_jobs_claimable'),
            ('idx_ingestion_jobs_one_active_embed'),
            ('idx_knowledge_audit_target'),
            ('idx_knowledge_audit_school')
    ) AS required(required_name)
    WHERE to_regclass('public.' || required_name) IS NULL;

    IF missing_indexes IS NOT NULL THEN
        RAISE EXCEPTION 'Missing governed knowledge indexes: %', missing_indexes;
    END IF;

    SELECT array_agg(required_name ORDER BY required_name)
    INTO missing_job_columns
    FROM (
        VALUES
            ('requested_by'),
            ('available_at'),
            ('locked_at'),
            ('heartbeat_at')
    ) AS required(required_name)
    WHERE NOT EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'ingestion_jobs'
          AND column_name = required_name
    );

    IF missing_job_columns IS NOT NULL THEN
        RAISE EXCEPTION 'Missing durable ingestion job columns: %', missing_job_columns;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_index index_definition
        JOIN pg_class index_relation ON index_relation.oid = index_definition.indexrelid
        WHERE index_relation.relname = 'idx_ingestion_jobs_one_active_embed'
          AND index_definition.indisunique
    ) THEN
        RAISE EXCEPTION 'Active embedding job index is not unique';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conrelid = 'knowledge_assets'::regclass
          AND conname = 'knowledge_asset_publish_consistency'
    ) THEN
        RAISE EXCEPTION 'Missing publication consistency constraint';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = 'knowledge_assets'
          AND policyname = 'knowledge_assets_scoped_select'
    ) THEN
        RAISE EXCEPTION 'Missing school/publication-scoped knowledge-assets RLS policy';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = 'teacher_asset_selections'
          AND policyname = 'teacher_asset_selection_owner'
    ) THEN
        RAISE EXCEPTION 'Missing teacher selection ownership RLS policy';
    END IF;
END
$$;

CREATE TEMP TABLE knowledge_status_probe (
    id integer PRIMARY KEY,
    status knowledge_asset_status NOT NULL,
    published_at timestamptz,
    archived_at timestamptz,
    failure_reason text
);

CREATE TRIGGER trg_probe_knowledge_status
BEFORE UPDATE OF status ON knowledge_status_probe
FOR EACH ROW EXECUTE FUNCTION validate_knowledge_asset_status_transition();

INSERT INTO knowledge_status_probe (id, status) VALUES (1, 'submitted');
UPDATE knowledge_status_probe SET status = 'ocr_ready' WHERE id = 1;
UPDATE knowledge_status_probe SET status = 'embedding_pending' WHERE id = 1;
UPDATE knowledge_status_probe SET status = 'embedded' WHERE id = 1;
UPDATE knowledge_status_probe SET status = 'published' WHERE id = 1;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM knowledge_status_probe
        WHERE id = 1 AND status = 'published' AND published_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'Publishing did not set published_at';
    END IF;

    BEGIN
        UPDATE knowledge_status_probe SET status = 'ocr_ready' WHERE id = 1;
        RAISE EXCEPTION 'Invalid transition published -> ocr_ready was accepted';
    EXCEPTION
        WHEN check_violation THEN NULL;
    END;
END
$$;

UPDATE knowledge_status_probe SET status = 'archived' WHERE id = 1;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM knowledge_status_probe
        WHERE id = 1 AND status = 'archived' AND archived_at IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'Archiving did not set archived_at';
    END IF;
END
$$;

-- Exercise the single-active-job invariant against real referenced rows.
BEGIN;
DO $$
DECLARE
    school_uuid uuid;
    admin_role_uuid uuid;
    admin_uuid uuid;
    asset_uuid uuid;
BEGIN
    INSERT INTO schools (name)
    VALUES ('Knowledge queue probe ' || gen_random_uuid()::text)
    RETURNING id INTO school_uuid;

    SELECT id INTO admin_role_uuid
    FROM roles
    WHERE name = 'PlatformAdmin'::role_name;

    INSERT INTO users (name, email, role_id, school_id)
    VALUES (
        'Knowledge Queue Probe Admin',
        'knowledge-queue-' || gen_random_uuid()::text || '@example.invalid',
        admin_role_uuid,
        school_uuid
    )
    RETURNING id INTO admin_uuid;

    INSERT INTO knowledge_assets (
        school_id, title, source_type, status, language, created_by
    ) VALUES (
        school_uuid, 'Durable queue probe', 'pdf', 'ocr_ready', 'fa', admin_uuid
    )
    RETURNING id INTO asset_uuid;

    INSERT INTO ingestion_jobs (
        asset_id, stage, status, requested_by, available_at
    ) VALUES (
        asset_uuid, 'embed', 'queued', admin_uuid, NOW()
    );

    BEGIN
        INSERT INTO ingestion_jobs (
            asset_id, stage, status, requested_by, available_at
        ) VALUES (
            asset_uuid, 'embed', 'running', admin_uuid, NOW()
        );
        RAISE EXCEPTION 'Duplicate active embedding job was accepted';
    EXCEPTION
        WHEN unique_violation THEN NULL;
    END;
END
$$;
ROLLBACK;

SELECT 'governed knowledge schema, lifecycle, and durable queue verified' AS result;
SQL
