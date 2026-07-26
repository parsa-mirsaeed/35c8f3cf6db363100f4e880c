-- Governed knowledge asset ingestion for enterprise-style RAG.
--
-- Design goals:
--   * school managers submit source documents without triggering embedding;
--   * platform administrators verify OCR, embed, and publish;
--   * teachers retrieve only published, school-scoped, explicitly enabled assets;
--   * every lifecycle action remains auditable and traceable.

-- Platform administrators are intentionally distinct from school managers.
ALTER TYPE role_name ADD VALUE IF NOT EXISTS 'PlatformAdmin';

DO $$
BEGIN
    CREATE TYPE knowledge_asset_status AS ENUM (
        'submitted',
        'ocr_pending',
        'ocr_ready',
        'embedding_pending',
        'embedded',
        'published',
        'archived',
        'failed'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DO $$
BEGIN
    CREATE TYPE ingestion_job_status AS ENUM (
        'queued',
        'running',
        'succeeded',
        'failed',
        'cancelled'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS knowledge_assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK (char_length(btrim(title)) BETWEEN 1 AND 255),
    description TEXT,
    source_type TEXT NOT NULL DEFAULT 'pdf'
        CHECK (source_type IN ('pdf', 'docx', 'text', 'url', 'other')),
    status knowledge_asset_status NOT NULL DEFAULT 'submitted',
    language TEXT NOT NULL DEFAULT 'fa',
    subject TEXT,
    grade TEXT,
    template_type TEXT,
    tags JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by UUID NOT NULL REFERENCES users(id),
    reviewed_by UUID REFERENCES users(id),
    published_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT knowledge_asset_publish_consistency CHECK (
        (status = 'published' AND published_at IS NOT NULL)
        OR (status <> 'published')
    )
);

CREATE TABLE IF NOT EXISTS knowledge_source_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL REFERENCES knowledge_assets(id) ON DELETE CASCADE,
    original_file_url TEXT,
    original_filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size_bytes BIGINT CHECK (file_size_bytes IS NULL OR file_size_bytes >= 0),
    sha256 TEXT CHECK (sha256 IS NULL OR sha256 ~ '^[0-9a-fA-F]{64}$'),
    page_count INTEGER CHECK (page_count IS NULL OR page_count >= 0),
    is_scanned_pdf BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS knowledge_ocr_texts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL UNIQUE REFERENCES knowledge_assets(id) ON DELETE CASCADE,
    raw_text TEXT NOT NULL,
    clean_text TEXT NOT NULL,
    ocr_provider TEXT NOT NULL,
    ocr_verified_by UUID NOT NULL REFERENCES users(id),
    ocr_verified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    text_sha256 TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (char_length(btrim(clean_text)) > 0)
);

CREATE TABLE IF NOT EXISTS knowledge_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL REFERENCES knowledge_assets(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL CHECK (chunk_index >= 0),
    text TEXT NOT NULL CHECK (char_length(btrim(text)) > 0),
    token_count INTEGER NOT NULL CHECK (token_count >= 0),
    embedding_provider TEXT NOT NULL,
    embedding_model TEXT NOT NULL,
    vector_id TEXT NOT NULL,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_id, chunk_index),
    UNIQUE (vector_id)
);

CREATE TABLE IF NOT EXISTS teacher_asset_selections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    teacher_id UUID NOT NULL REFERENCES teachers(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES knowledge_assets(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    context_scope TEXT NOT NULL DEFAULT 'global'
        CHECK (context_scope IN ('global', 'workflow', 'class', 'generation_session')),
    context_key TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (teacher_id, asset_id, context_scope, context_key)
);

CREATE TABLE IF NOT EXISTS ingestion_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL REFERENCES knowledge_assets(id) ON DELETE CASCADE,
    stage TEXT NOT NULL CHECK (stage IN ('ocr', 'normalize', 'chunk', 'embed', 'publish', 'archive')),
    status ingestion_job_status NOT NULL DEFAULT 'queued',
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    error_message TEXT,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS knowledge_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id UUID REFERENCES users(id),
    actor_role TEXT NOT NULL,
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id UUID NOT NULL,
    school_id UUID REFERENCES schools(id) ON DELETE SET NULL,
    details_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    request_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_knowledge_assets_school_status
    ON knowledge_assets (school_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_knowledge_assets_metadata
    ON knowledge_assets (school_id, subject, grade, template_type)
    WHERE status = 'published';
CREATE INDEX IF NOT EXISTS idx_knowledge_assets_tags_gin
    ON knowledge_assets USING GIN (tags);
CREATE INDEX IF NOT EXISTS idx_knowledge_source_files_asset
    ON knowledge_source_files (asset_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_chunks_asset
    ON knowledge_chunks (asset_id, chunk_index);
CREATE INDEX IF NOT EXISTS idx_teacher_asset_selections_lookup
    ON teacher_asset_selections (teacher_id, enabled, context_scope, context_key);
CREATE INDEX IF NOT EXISTS idx_ingestion_jobs_asset_created
    ON ingestion_jobs (asset_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_knowledge_audit_target
    ON knowledge_audit_logs (target_type, target_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_knowledge_audit_school
    ON knowledge_audit_logs (school_id, created_at DESC);

CREATE OR REPLACE FUNCTION set_knowledge_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_knowledge_assets_updated_at ON knowledge_assets;
CREATE TRIGGER trg_knowledge_assets_updated_at
BEFORE UPDATE ON knowledge_assets
FOR EACH ROW EXECUTE FUNCTION set_knowledge_updated_at();

DROP TRIGGER IF EXISTS trg_knowledge_ocr_updated_at ON knowledge_ocr_texts;
CREATE TRIGGER trg_knowledge_ocr_updated_at
BEFORE UPDATE ON knowledge_ocr_texts
FOR EACH ROW EXECUTE FUNCTION set_knowledge_updated_at();

DROP TRIGGER IF EXISTS trg_teacher_asset_selections_updated_at ON teacher_asset_selections;
CREATE TRIGGER trg_teacher_asset_selections_updated_at
BEFORE UPDATE ON teacher_asset_selections
FOR EACH ROW EXECUTE FUNCTION set_knowledge_updated_at();

DROP TRIGGER IF EXISTS trg_ingestion_jobs_updated_at ON ingestion_jobs;
CREATE TRIGGER trg_ingestion_jobs_updated_at
BEFORE UPDATE ON ingestion_jobs
FOR EACH ROW EXECUTE FUNCTION set_knowledge_updated_at();

-- Enforce the lifecycle as a state machine. This prevents accidental publication
-- before verified OCR and embedding have completed.
CREATE OR REPLACE FUNCTION validate_knowledge_asset_status_transition()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;

    IF NOT (
        (OLD.status = 'submitted' AND NEW.status IN ('ocr_pending', 'ocr_ready', 'archived', 'failed')) OR
        (OLD.status = 'ocr_pending' AND NEW.status IN ('ocr_ready', 'archived', 'failed')) OR
        (OLD.status = 'ocr_ready' AND NEW.status IN ('embedding_pending', 'archived', 'failed')) OR
        (OLD.status = 'embedding_pending' AND NEW.status IN ('embedded', 'ocr_ready', 'archived', 'failed')) OR
        (OLD.status = 'embedded' AND NEW.status IN ('embedding_pending', 'published', 'archived', 'failed')) OR
        (OLD.status = 'published' AND NEW.status IN ('embedded', 'archived')) OR
        (OLD.status = 'failed' AND NEW.status IN ('ocr_ready', 'embedding_pending', 'archived'))
    ) THEN
        RAISE EXCEPTION 'Invalid knowledge asset status transition: % -> %', OLD.status, NEW.status
            USING ERRCODE = '23514';
    END IF;

    IF NEW.status = 'published' THEN
        NEW.published_at = COALESCE(NEW.published_at, NOW());
        NEW.archived_at = NULL;
        NEW.failure_reason = NULL;
    ELSIF NEW.status = 'archived' THEN
        NEW.archived_at = COALESCE(NEW.archived_at, NOW());
    ELSIF NEW.status <> 'published' THEN
        NEW.published_at = NULL;
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_validate_knowledge_asset_status ON knowledge_assets;
CREATE TRIGGER trg_validate_knowledge_asset_status
BEFORE UPDATE OF status ON knowledge_assets
FOR EACH ROW EXECUTE FUNCTION validate_knowledge_asset_status_transition();

-- The legacy teacher PDF path is disabled at the persistence boundary. Teachers
-- may still create non-document class resources, but documents must enter through
-- the manager submission / admin publication workflow.
CREATE OR REPLACE FUNCTION prevent_teacher_document_ingestion()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.material_type = 'document'
       AND EXISTS (
            SELECT 1
            FROM users u
            WHERE u.id = NEW.created_by
              AND u.role = 'Teacher'::role_name
       ) THEN
        RAISE EXCEPTION 'Teacher document upload is disabled; use a published knowledge asset'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_prevent_teacher_document_ingestion ON class_materials;
CREATE TRIGGER trg_prevent_teacher_document_ingestion
BEFORE INSERT OR UPDATE OF material_type, created_by ON class_materials
FOR EACH ROW EXECUTE FUNCTION prevent_teacher_document_ingestion();

-- Row-level policies provide defense in depth for direct database access. The
-- application still performs explicit role/school authorization in every endpoint.
ALTER TABLE knowledge_assets ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_source_files ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_ocr_texts ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_chunks ENABLE ROW LEVEL SECURITY;
ALTER TABLE teacher_asset_selections ENABLE ROW LEVEL SECURITY;
ALTER TABLE ingestion_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_audit_logs ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS knowledge_assets_school_read ON knowledge_assets;
CREATE POLICY knowledge_assets_school_read ON knowledge_assets
FOR SELECT USING (
    current_setting('app.current_role', true) = 'PlatformAdmin'
    OR school_id::text = NULLIF(current_setting('app.current_school_id', true), '')
);

DROP POLICY IF EXISTS knowledge_assets_manager_submit ON knowledge_assets;
CREATE POLICY knowledge_assets_manager_submit ON knowledge_assets
FOR INSERT WITH CHECK (
    current_setting('app.current_role', true) IN ('SchoolManager', 'PlatformAdmin')
    AND (
        current_setting('app.current_role', true) = 'PlatformAdmin'
        OR school_id::text = NULLIF(current_setting('app.current_school_id', true), '')
    )
);

DROP POLICY IF EXISTS knowledge_assets_admin_write ON knowledge_assets;
CREATE POLICY knowledge_assets_admin_write ON knowledge_assets
FOR UPDATE USING (current_setting('app.current_role', true) = 'PlatformAdmin')
WITH CHECK (current_setting('app.current_role', true) = 'PlatformAdmin');

DROP POLICY IF EXISTS teacher_asset_selection_owner ON teacher_asset_selections;
CREATE POLICY teacher_asset_selection_owner ON teacher_asset_selections
FOR ALL USING (
    current_setting('app.current_role', true) = 'PlatformAdmin'
    OR teacher_id IN (
        SELECT t.id FROM teachers t
        WHERE t.user_id::text = NULLIF(current_setting('app.current_user_id', true), '')
    )
)
WITH CHECK (
    current_setting('app.current_role', true) = 'PlatformAdmin'
    OR teacher_id IN (
        SELECT t.id FROM teachers t
        WHERE t.user_id::text = NULLIF(current_setting('app.current_user_id', true), '')
    )
);

COMMENT ON TABLE knowledge_assets IS
    'Reviewed, school-scoped RAG assets governed by manager submission and platform-admin publication.';
COMMENT ON TABLE knowledge_chunks IS
    'Provenance records for chunks stored in the vector database; vectors themselves remain in Qdrant.';
COMMENT ON TABLE knowledge_audit_logs IS
    'Append-only security and provenance events for knowledge ingestion, selection, and retrieval.';
