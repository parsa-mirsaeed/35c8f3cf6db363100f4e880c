-- Align governed-knowledge RLS with the canonical application context helpers.
-- set_app_context writes app.user_id, app.user_role, and app.school_id; the
-- get_* helpers are the stable interface used throughout the existing schema.

DROP POLICY IF EXISTS knowledge_assets_school_read ON knowledge_assets;
DROP POLICY IF EXISTS knowledge_assets_manager_submit ON knowledge_assets;
DROP POLICY IF EXISTS knowledge_assets_admin_write ON knowledge_assets;
DROP POLICY IF EXISTS teacher_asset_selection_owner ON teacher_asset_selections;

CREATE POLICY knowledge_assets_scoped_select ON knowledge_assets
FOR SELECT USING (
    get_role() = 'PlatformAdmin'
    OR (get_role() = 'SchoolManager' AND school_id = get_school_id())
    OR (
        get_role() = 'Teacher'
        AND school_id = get_school_id()
        AND status = 'published'
    )
);

CREATE POLICY knowledge_assets_scoped_insert ON knowledge_assets
FOR INSERT WITH CHECK (
    get_role() = 'PlatformAdmin'
    OR (get_role() = 'SchoolManager' AND school_id = get_school_id())
);

CREATE POLICY knowledge_assets_admin_update ON knowledge_assets
FOR UPDATE
USING (get_role() = 'PlatformAdmin')
WITH CHECK (get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS knowledge_source_files_scoped_select ON knowledge_source_files;
DROP POLICY IF EXISTS knowledge_source_files_scoped_insert ON knowledge_source_files;
DROP POLICY IF EXISTS knowledge_source_files_admin_write ON knowledge_source_files;

CREATE POLICY knowledge_source_files_scoped_select ON knowledge_source_files
FOR SELECT USING (
    get_role() = 'PlatformAdmin'
    OR (
        get_role() = 'SchoolManager'
        AND EXISTS (
            SELECT 1
            FROM knowledge_assets asset
            WHERE asset.id = knowledge_source_files.asset_id
              AND asset.school_id = get_school_id()
        )
    )
);

CREATE POLICY knowledge_source_files_scoped_insert ON knowledge_source_files
FOR INSERT WITH CHECK (
    get_role() = 'PlatformAdmin'
    OR (
        get_role() = 'SchoolManager'
        AND EXISTS (
            SELECT 1
            FROM knowledge_assets asset
            WHERE asset.id = knowledge_source_files.asset_id
              AND asset.school_id = get_school_id()
        )
    )
);

CREATE POLICY knowledge_source_files_admin_write ON knowledge_source_files
FOR UPDATE
USING (get_role() = 'PlatformAdmin')
WITH CHECK (get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS knowledge_ocr_texts_admin_all ON knowledge_ocr_texts;
CREATE POLICY knowledge_ocr_texts_admin_all ON knowledge_ocr_texts
FOR ALL
USING (get_role() = 'PlatformAdmin')
WITH CHECK (get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS knowledge_chunks_admin_all ON knowledge_chunks;
CREATE POLICY knowledge_chunks_admin_all ON knowledge_chunks
FOR ALL
USING (get_role() = 'PlatformAdmin')
WITH CHECK (get_role() = 'PlatformAdmin');

DROP POLICY IF EXISTS ingestion_jobs_admin_all ON ingestion_jobs;
CREATE POLICY ingestion_jobs_admin_all ON ingestion_jobs
FOR ALL
USING (get_role() = 'PlatformAdmin')
WITH CHECK (get_role() = 'PlatformAdmin');

CREATE POLICY teacher_asset_selection_owner ON teacher_asset_selections
FOR ALL
USING (
    get_role() = 'PlatformAdmin'
    OR (
        get_role() = 'Teacher'
        AND teacher_id IN (
            SELECT teacher.id
            FROM teachers teacher
            WHERE teacher.user_id = get_user_id()
              AND teacher.school_id = get_school_id()
        )
    )
)
WITH CHECK (
    get_role() = 'PlatformAdmin'
    OR (
        get_role() = 'Teacher'
        AND teacher_id IN (
            SELECT teacher.id
            FROM teachers teacher
            WHERE teacher.user_id = get_user_id()
              AND teacher.school_id = get_school_id()
        )
        AND EXISTS (
            SELECT 1
            FROM knowledge_assets asset
            WHERE asset.id = teacher_asset_selections.asset_id
              AND asset.school_id = get_school_id()
              AND asset.status = 'published'
        )
    )
);

DROP POLICY IF EXISTS knowledge_audit_logs_admin_select ON knowledge_audit_logs;
DROP POLICY IF EXISTS knowledge_audit_logs_actor_insert ON knowledge_audit_logs;

CREATE POLICY knowledge_audit_logs_admin_select ON knowledge_audit_logs
FOR SELECT USING (get_role() = 'PlatformAdmin');

CREATE POLICY knowledge_audit_logs_actor_insert ON knowledge_audit_logs
FOR INSERT WITH CHECK (
    get_role() = 'PlatformAdmin'
    OR (
        get_role() IN ('SchoolManager', 'Teacher')
        AND actor_id = get_user_id()
        AND school_id = get_school_id()
    )
);

COMMENT ON POLICY knowledge_assets_scoped_select ON knowledge_assets IS
    'Platform administrators see all assets; managers see their school; teachers see only published assets in their school.';
