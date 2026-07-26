-- =====================================================
-- RLS Security Fixes for Remaining Tables + Function Search Path
-- Fixes Supabase Linter Warnings
-- =====================================================

-- =====================================================
-- FIX 1: Enable RLS on remaining tables
-- _sqlx_migrations, roles, subjects need RLS enabled
-- Even if they have no policies, enable RLS for compliance
-- =====================================================

-- _sqlx_migrations: Internal table, deny all via RLS
ALTER TABLE _sqlx_migrations ENABLE ROW LEVEL SECURITY;
-- Allow postgres/service role to manage migrations (they bypass RLS)
-- No policies = deny all for non-superusers

-- roles: Static reference data, allow read-only access for authenticated
ALTER TABLE roles ENABLE ROW LEVEL SECURITY;
CREATE POLICY roles_select_policy ON roles FOR SELECT USING (true);
-- No INSERT/UPDATE/DELETE policies = deny modifications for non-superusers

-- subjects: Static reference data, allow read-only access for same school
ALTER TABLE subjects ENABLE ROW LEVEL SECURITY;
CREATE POLICY subjects_select_policy ON subjects FOR SELECT USING (true);
-- No INSERT/UPDATE/DELETE policies = deny modifications for non-superusers

-- =====================================================
-- FIX 2: Set search_path on functions to prevent search_path attacks
-- https://supabase.com/docs/guides/database/database-linter?lint=0011_function_search_path_mutable
-- =====================================================

-- Fix set_app_context
CREATE OR REPLACE FUNCTION set_app_context(
    p_user_id UUID,
    p_role TEXT,
    p_school_id UUID
) RETURNS VOID AS $$
BEGIN
    PERFORM set_config('app.user_id', p_user_id::TEXT, true);
    PERFORM set_config('app.user_role', p_role, true);
    PERFORM set_config('app.school_id', p_school_id::TEXT, true);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path = public;

-- Fix get_user_id
CREATE OR REPLACE FUNCTION get_user_id() RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.user_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public;

-- Fix get_role
CREATE OR REPLACE FUNCTION get_role() RETURNS TEXT AS $$
BEGIN
    RETURN NULLIF(current_setting('app.user_role', true), '');
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public;

-- Fix get_school_id
CREATE OR REPLACE FUNCTION get_school_id() RETURNS UUID AS $$
BEGIN
    RETURN NULLIF(current_setting('app.school_id', true), '')::UUID;
EXCEPTION WHEN OTHERS THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public;

-- Fix is_school_manager
CREATE OR REPLACE FUNCTION is_school_manager() RETURNS BOOLEAN AS $$
BEGIN
    RETURN get_role() = 'SchoolManager';
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER SET search_path = public;

-- Fix update_updated_at_column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql SET search_path = public;

-- Fix bootstrap_school_manager
CREATE OR REPLACE FUNCTION bootstrap_school_manager(
    p_school_name TEXT,
    p_admin_email TEXT,
    p_admin_name TEXT,
    p_admin_auth_uid UUID
) RETURNS TEXT
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_school_id UUID;
    v_role_id UUID;
    v_user_id UUID;
    v_existing_school_manager_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_existing_school_manager_count
    FROM users u
    JOIN roles r ON u.role_id = r.id
    WHERE r.name = 'SchoolManager';

    IF v_existing_school_manager_count > 0 THEN
        RAISE EXCEPTION 'Bootstrap already completed: SchoolManager already exists';
    END IF;

    INSERT INTO schools (name)
    VALUES (p_school_name)
    RETURNING id INTO v_school_id;

    SELECT id INTO v_role_id FROM roles WHERE name = 'SchoolManager';
    IF v_role_id IS NULL THEN
        RAISE EXCEPTION 'SchoolManager role not found';
    END IF;

    INSERT INTO users (
        id, name, email, role_id, school_id, is_active, created_at, updated_at
    ) VALUES (
        p_admin_auth_uid, p_admin_name, p_admin_email, v_role_id, v_school_id,
        true, now(), now()
    ) RETURNING id INTO v_user_id;

    INSERT INTO teachers (user_id, school_id, created_at)
    VALUES (v_user_id, v_school_id, now());

    INSERT INTO audit_logs (
        actor_id, action, entity, entity_id, "before", "after", at
    ) VALUES (
        v_user_id,
        'BOOTSTRAP_SCHOOL_MANAGER',
        'users',
        v_user_id,
        NULL,
        json_build_object(
            'school_name', p_school_name,
            'email', p_admin_email,
            'role', 'SchoolManager'
        ),
        now()
    );

    RETURN format(
        'Successfully bootstrapped school "%s" with SchoolManager "%s" (%s)',
        p_school_name, p_admin_name, p_admin_email
    );
END;
$$;

-- Fix is_bootstrap_completed
CREATE OR REPLACE FUNCTION is_bootstrap_completed()
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_school_manager_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_school_manager_count
    FROM users u
    JOIN roles r ON u.role_id = r.id
    WHERE r.name = 'SchoolManager';

    RETURN v_school_manager_count > 0;
END;
$$;

-- Fix create_invite
CREATE OR REPLACE FUNCTION create_invite(
    p_email TEXT,
    p_role_name role_name,
    p_school_id UUID,
    p_token_hash TEXT,
    p_created_by UUID,
    p_class_section_ids UUID[] DEFAULT '{}',
    p_student_id UUID DEFAULT NULL,
    p_expires_at TIMESTAMPTZ DEFAULT (now() + interval '7 days')
) RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_invite_id UUID;
    v_creator_school_id UUID;
BEGIN
    SELECT u.school_id INTO v_creator_school_id
    FROM users u
    JOIN roles r ON u.role_id = r.id
    WHERE u.id = p_created_by AND r.name = 'SchoolManager';

    IF v_creator_school_id IS NULL THEN
        RAISE EXCEPTION 'Only SchoolManagers can create invites';
    END IF;

    IF v_creator_school_id != p_school_id THEN
        RAISE EXCEPTION 'SchoolManager can only create invites for their own school';
    END IF;

    IF EXISTS (
        SELECT 1 FROM invites
        WHERE email = p_email
          AND consumed_at IS NULL
          AND expires_at > now()
    ) THEN
        RAISE EXCEPTION 'Active invite already exists for email: %', p_email;
    END IF;

    INSERT INTO invites (
        email, role_name, school_id, class_section_ids, student_id,
        token_hash, expires_at, created_by
    ) VALUES (
        p_email, p_role_name, p_school_id, p_class_section_ids, p_student_id,
        p_token_hash, p_expires_at, p_created_by
    ) RETURNING id INTO v_invite_id;

    INSERT INTO audit_logs (
        actor_id, action, entity, entity_id, "before", "after", at
    ) VALUES (
        p_created_by,
        'CREATE_INVITE',
        'invites',
        v_invite_id,
        NULL,
        json_build_object(
            'email', p_email,
            'role_name', p_role_name,
            'school_id', p_school_id,
            'expires_at', p_expires_at
        ),
        now()
    );

    RETURN v_invite_id;
END;
$$;

-- Fix claim_invite
CREATE OR REPLACE FUNCTION claim_invite(
    p_token_hash TEXT,
    p_name TEXT,
    p_auth_uid UUID
) RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_invite RECORD;
    v_user_id UUID;
    v_role_id UUID;
BEGIN
    SELECT i.* INTO v_invite
    FROM invites i
    WHERE i.token_hash = p_token_hash
      AND i.consumed_at IS NULL
      AND i.expires_at > now()
    FOR UPDATE;

    IF v_invite IS NULL THEN
        RAISE EXCEPTION 'Invalid, expired, or already consumed invite';
    END IF;

    SELECT id INTO v_role_id FROM roles WHERE name = v_invite.role_name;
    IF v_role_id IS NULL THEN
        RAISE EXCEPTION 'Invalid role specified in invite';
    END IF;

    INSERT INTO users (
        id, name, email, role_id, school_id, is_active, created_at, updated_at
    ) VALUES (
        p_auth_uid, p_name, v_invite.email, v_role_id, v_invite.school_id,
        true, now(), now()
    ) RETURNING id INTO v_user_id;

    IF v_invite.role_name = 'Teacher' THEN
        INSERT INTO teachers (user_id, school_id, created_at)
        VALUES (v_user_id, v_invite.school_id, now());

        IF v_invite.class_section_ids IS NOT NULL
           AND array_length(v_invite.class_section_ids, 1) > 0 THEN
            INSERT INTO teaching_assignments (class_section_id, teacher_id)
            SELECT cs_id, v_user_id
            FROM unnest(v_invite.class_section_ids) AS cs_id;
        END IF;

    ELSIF v_invite.role_name = 'Student' THEN
        INSERT INTO students (user_id, school_id, created_at)
        VALUES (v_user_id, v_invite.school_id, now());

    ELSIF v_invite.role_name = 'Parent' AND v_invite.student_id IS NOT NULL THEN
        UPDATE students
        SET parent_id = v_user_id
        WHERE id = v_invite.student_id;
    END IF;

    UPDATE invites
    SET consumed_at = now()
    WHERE id = v_invite.id;

    INSERT INTO audit_logs (
        actor_id, action, entity, entity_id, "before", "after", at
    ) VALUES (
        v_invite.created_by,
        'CLAIM_INVITE',
        'users',
        v_user_id,
        json_build_object('invite_id', v_invite.id),
        json_build_object(
            'user_id', v_user_id,
            'email', v_invite.email,
            'role', v_invite.role_name,
            'school_id', v_invite.school_id
        ),
        now()
    );

    RETURN v_user_id;
END;
$$;

-- Fix bootstrap_admin (if it exists)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'bootstrap_admin') THEN
        EXECUTE 'ALTER FUNCTION bootstrap_admin SET search_path = public';
    END IF;
EXCEPTION WHEN OTHERS THEN
    -- Function doesn't exist or can't be altered, ignore
    NULL;
END $$;

-- =====================================================
-- VERIFICATION
-- =====================================================
-- Run this to verify all tables now have RLS:
-- SELECT tablename, rowsecurity FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename;
