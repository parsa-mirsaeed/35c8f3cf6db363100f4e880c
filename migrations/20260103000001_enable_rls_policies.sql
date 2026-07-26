-- =====================================================
-- RLS Policies for EduTalent
-- ISO 27001:2022 A.8.3 Access Restriction Compliance
-- OWASP ASVS 4.0.3 V4.2 Operation Level Access Control
-- =====================================================
-- CRITICAL: Helper functions must be created first!
-- Assumes set_app_context, get_user_id, get_role, get_school_id, is_school_manager exist

-- =====================================================
-- TIER 1: CORE USER TABLES
-- =====================================================

-- -------------------- USERS --------------------
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
-- Force RLS even for table owner (critical for service role)
ALTER TABLE users FORCE ROW LEVEL SECURITY;

-- SELECT: Own record OR SchoolManager sees same school
CREATE POLICY users_select_policy ON users FOR SELECT USING (
    id = get_user_id() 
    OR (is_school_manager() AND school_id = get_school_id())
);

-- INSERT: SchoolManager only for their school
CREATE POLICY users_insert_policy ON users FOR INSERT WITH CHECK (
    is_school_manager() AND school_id = get_school_id()
);

-- UPDATE: Own record (limited fields) OR SchoolManager for same school
CREATE POLICY users_update_policy ON users FOR UPDATE USING (
    id = get_user_id() 
    OR (is_school_manager() AND school_id = get_school_id())
);

-- DELETE: SchoolManager only
CREATE POLICY users_delete_policy ON users FOR DELETE USING (
    is_school_manager() AND school_id = get_school_id()
);

-- -------------------- STUDENTS --------------------
ALTER TABLE students ENABLE ROW LEVEL SECURITY;
ALTER TABLE students FORCE ROW LEVEL SECURITY;

-- SELECT: Own record, parent's child, teacher via enrollment, SchoolManager
CREATE POLICY students_select_policy ON students FOR SELECT USING (
    -- Student sees own record
    user_id = get_user_id()
    -- Parent sees their children
    OR parent_id = get_user_id()
    -- Teacher sees students in their classes
    OR EXISTS (
        SELECT 1 FROM enrollments e
        JOIN teaching_assignments ta ON ta.class_section_id = e.class_section_id
        JOIN teachers t ON t.id = ta.teacher_id
        WHERE e.student_id = students.id AND t.user_id = get_user_id()
    )
    -- SchoolManager sees all in their school
    OR (is_school_manager() AND school_id = get_school_id())
);

-- INSERT/UPDATE/DELETE: SchoolManager only
CREATE POLICY students_modify_policy ON students FOR ALL USING (
    is_school_manager() AND school_id = get_school_id()
) WITH CHECK (
    is_school_manager() AND school_id = get_school_id()
);

-- -------------------- TEACHERS --------------------
ALTER TABLE teachers ENABLE ROW LEVEL SECURITY;
ALTER TABLE teachers FORCE ROW LEVEL SECURITY;

-- SELECT: Own record or SchoolManager same school  
CREATE POLICY teachers_select_policy ON teachers FOR SELECT USING (
    user_id = get_user_id()
    OR (is_school_manager() AND school_id = get_school_id())
);

-- INSERT/UPDATE/DELETE: SchoolManager only
CREATE POLICY teachers_modify_policy ON teachers FOR ALL USING (
    is_school_manager() AND school_id = get_school_id()
) WITH CHECK (
    is_school_manager() AND school_id = get_school_id()
);

-- -------------------- PARENTS --------------------
ALTER TABLE parents ENABLE ROW LEVEL SECURITY;
ALTER TABLE parents FORCE ROW LEVEL SECURITY;

-- SELECT: Own record or SchoolManager same school
CREATE POLICY parents_select_policy ON parents FOR SELECT USING (
    user_id = get_user_id()
    OR (is_school_manager() AND school_id = get_school_id())
);

-- INSERT/UPDATE/DELETE: SchoolManager only
CREATE POLICY parents_modify_policy ON parents FOR ALL USING (
    is_school_manager() AND school_id = get_school_id()
) WITH CHECK (
    is_school_manager() AND school_id = get_school_id()
);

-- =====================================================
-- TIER 2: CLASS STRUCTURE
-- =====================================================

-- -------------------- SCHOOLS --------------------
ALTER TABLE schools ENABLE ROW LEVEL SECURITY;
ALTER TABLE schools FORCE ROW LEVEL SECURITY;

-- SELECT: Users see their own school
CREATE POLICY schools_select_policy ON schools FOR SELECT USING (
    id = get_school_id()
);

-- INSERT/UPDATE/DELETE: SchoolManager only (for their school)
CREATE POLICY schools_modify_policy ON schools FOR ALL USING (
    is_school_manager() AND id = get_school_id()
) WITH CHECK (
    is_school_manager() AND id = get_school_id()
);

-- -------------------- CLASS_SECTIONS --------------------
ALTER TABLE class_sections ENABLE ROW LEVEL SECURITY;
ALTER TABLE class_sections FORCE ROW LEVEL SECURITY;

-- SELECT: Same school (students, teachers, parents, managers)
CREATE POLICY class_sections_select_policy ON class_sections FOR SELECT USING (
    school_id = get_school_id()
);

-- INSERT/UPDATE/DELETE: SchoolManager only
CREATE POLICY class_sections_modify_policy ON class_sections FOR ALL USING (
    is_school_manager() AND school_id = get_school_id()
) WITH CHECK (
    is_school_manager() AND school_id = get_school_id()
);

-- -------------------- ENROLLMENTS --------------------
ALTER TABLE enrollments ENABLE ROW LEVEL SECURITY;
ALTER TABLE enrollments FORCE ROW LEVEL SECURITY;

-- SELECT: Student enrolled, teacher teaches class, parent's child, SchoolManager
CREATE POLICY enrollments_select_policy ON enrollments FOR SELECT USING (
    -- Student sees own enrollment
    EXISTS (SELECT 1 FROM students s WHERE s.id = enrollments.student_id AND s.user_id = get_user_id())
    -- Teacher sees enrollments in their classes
    OR EXISTS (
        SELECT 1 FROM teaching_assignments ta
        JOIN teachers t ON t.id = ta.teacher_id
        WHERE ta.class_section_id = enrollments.class_section_id AND t.user_id = get_user_id()
    )
    -- Parent sees child's enrollments
    OR EXISTS (
        SELECT 1 FROM students s 
        WHERE s.id = enrollments.student_id AND s.parent_id = get_user_id()
    )
    -- SchoolManager sees all
    OR (is_school_manager() AND EXISTS (
        SELECT 1 FROM class_sections cs WHERE cs.id = enrollments.class_section_id AND cs.school_id = get_school_id()
    ))
);

-- INSERT/UPDATE/DELETE: SchoolManager only
CREATE POLICY enrollments_modify_policy ON enrollments FOR ALL USING (
    is_school_manager() AND EXISTS (
        SELECT 1 FROM class_sections cs WHERE cs.id = enrollments.class_section_id AND cs.school_id = get_school_id()
    )
) WITH CHECK (
    is_school_manager() AND EXISTS (
        SELECT 1 FROM class_sections cs WHERE cs.id = enrollments.class_section_id AND cs.school_id = get_school_id()
    )
);

-- -------------------- TEACHING_ASSIGNMENTS --------------------
ALTER TABLE teaching_assignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE teaching_assignments FORCE ROW LEVEL SECURITY;

-- SELECT: Teacher sees own assignments, SchoolManager sees all
CREATE POLICY teaching_assignments_select_policy ON teaching_assignments FOR SELECT USING (
    EXISTS (SELECT 1 FROM teachers t WHERE t.id = teaching_assignments.teacher_id AND t.user_id = get_user_id())
    OR (is_school_manager() AND EXISTS (
        SELECT 1 FROM class_sections cs WHERE cs.id = teaching_assignments.class_section_id AND cs.school_id = get_school_id()
    ))
);

-- INSERT/UPDATE/DELETE: SchoolManager only
CREATE POLICY teaching_assignments_modify_policy ON teaching_assignments FOR ALL USING (
    is_school_manager() AND EXISTS (
        SELECT 1 FROM class_sections cs WHERE cs.id = teaching_assignments.class_section_id AND cs.school_id = get_school_id()
    )
) WITH CHECK (
    is_school_manager() AND EXISTS (
        SELECT 1 FROM class_sections cs WHERE cs.id = teaching_assignments.class_section_id AND cs.school_id = get_school_id()
    )
);

-- =====================================================
-- TIER 3: EDUCATIONAL CONTENT
-- =====================================================

-- -------------------- ASSIGNMENTS --------------------
ALTER TABLE assignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE assignments FORCE ROW LEVEL SECURITY;

-- SELECT: Teacher who created, students enrolled in class, parents of enrolled students
CREATE POLICY assignments_select_policy ON assignments FOR SELECT USING (
    -- Teacher who created
    EXISTS (SELECT 1 FROM teachers t WHERE t.id = assignments.teacher_id AND t.user_id = get_user_id())
    -- Students enrolled in the class
    OR EXISTS (
        SELECT 1 FROM enrollments e
        JOIN students s ON s.id = e.student_id
        WHERE e.class_section_id = assignments.class_section_id AND s.user_id = get_user_id()
    )
    -- Parents of enrolled students
    OR EXISTS (
        SELECT 1 FROM enrollments e
        JOIN students s ON s.id = e.student_id
        WHERE e.class_section_id = assignments.class_section_id AND s.parent_id = get_user_id()
    )
    -- SchoolManager
    OR (is_school_manager() AND EXISTS (
        SELECT 1 FROM class_sections cs WHERE cs.id = assignments.class_section_id AND cs.school_id = get_school_id()
    ))
);

-- INSERT: Teacher only (for classes they teach)
CREATE POLICY assignments_insert_policy ON assignments FOR INSERT WITH CHECK (
    EXISTS (
        SELECT 1 FROM teachers t
        JOIN teaching_assignments ta ON ta.teacher_id = t.id
        WHERE t.user_id = get_user_id() AND ta.class_section_id = assignments.class_section_id
    )
);

-- UPDATE/DELETE: Teacher who owns
CREATE POLICY assignments_update_policy ON assignments FOR UPDATE USING (
    EXISTS (SELECT 1 FROM teachers t WHERE t.id = assignments.teacher_id AND t.user_id = get_user_id())
);

CREATE POLICY assignments_delete_policy ON assignments FOR DELETE USING (
    EXISTS (SELECT 1 FROM teachers t WHERE t.id = assignments.teacher_id AND t.user_id = get_user_id())
);

-- -------------------- CUSTOM_ASSIGNMENTS --------------------
ALTER TABLE custom_assignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE custom_assignments FORCE ROW LEVEL SECURITY;

-- SELECT: Student assigned, teacher who created parent assignment, parent of student
CREATE POLICY custom_assignments_select_policy ON custom_assignments FOR SELECT USING (
    -- Student sees own
    EXISTS (SELECT 1 FROM students s WHERE s.id = custom_assignments.student_id AND s.user_id = get_user_id())
    -- Teacher sees for assignments they created
    OR EXISTS (
        SELECT 1 FROM assignments a
        JOIN teachers t ON t.id = a.teacher_id
        WHERE a.id = custom_assignments.assignment_id AND t.user_id = get_user_id()
    )
    -- Parent sees child's
    OR EXISTS (
        SELECT 1 FROM students s WHERE s.id = custom_assignments.student_id AND s.parent_id = get_user_id()
    )
    -- SchoolManager
    OR is_school_manager()
);

-- INSERT/UPDATE: Teacher who created parent assignment
CREATE POLICY custom_assignments_modify_policy ON custom_assignments FOR ALL USING (
    EXISTS (
        SELECT 1 FROM assignments a
        JOIN teachers t ON t.id = a.teacher_id
        WHERE a.id = custom_assignments.assignment_id AND t.user_id = get_user_id()
    )
) WITH CHECK (
    EXISTS (
        SELECT 1 FROM assignments a
        JOIN teachers t ON t.id = a.teacher_id
        WHERE a.id = custom_assignments.assignment_id AND t.user_id = get_user_id()
    )
);

-- -------------------- SUBMISSIONS --------------------
ALTER TABLE submissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE submissions FORCE ROW LEVEL SECURITY;

-- SELECT: Student who submitted, teacher who grades, parent
CREATE POLICY submissions_select_policy ON submissions FOR SELECT USING (
    -- Student sees own
    EXISTS (SELECT 1 FROM students s WHERE s.id = submissions.student_id AND s.user_id = get_user_id())
    -- Teacher who can grade
    OR EXISTS (SELECT 1 FROM teachers t WHERE t.id = submissions.graded_by AND t.user_id = get_user_id())
    -- Teacher for the class
    OR EXISTS (
        SELECT 1 FROM custom_assignments ca
        JOIN assignments a ON a.id = ca.assignment_id
        JOIN teachers t ON t.id = a.teacher_id
        WHERE ca.id = submissions.custom_assignment_id AND t.user_id = get_user_id()
    )
    -- Parent
    OR EXISTS (
        SELECT 1 FROM students s WHERE s.id = submissions.student_id AND s.parent_id = get_user_id()
    )
    -- SchoolManager
    OR is_school_manager()
);

-- INSERT: Student submits own work
CREATE POLICY submissions_insert_policy ON submissions FOR INSERT WITH CHECK (
    EXISTS (SELECT 1 FROM students s WHERE s.id = submissions.student_id AND s.user_id = get_user_id())
);

-- UPDATE: Teacher grades, student updates own before grading
CREATE POLICY submissions_update_policy ON submissions FOR UPDATE USING (
    -- Student can update own ungraded submission
    (EXISTS (SELECT 1 FROM students s WHERE s.id = submissions.student_id AND s.user_id = get_user_id()) AND submissions.grade IS NULL)
    -- Teacher can grade
    OR EXISTS (
        SELECT 1 FROM custom_assignments ca
        JOIN assignments a ON a.id = ca.assignment_id
        JOIN teachers t ON t.id = a.teacher_id
        WHERE ca.id = submissions.custom_assignment_id AND t.user_id = get_user_id()
    )
);

-- -------------------- LECTURES --------------------
ALTER TABLE lectures ENABLE ROW LEVEL SECURITY;
ALTER TABLE lectures FORCE ROW LEVEL SECURITY;

-- SELECT: Via class_section access (same school)
CREATE POLICY lectures_select_policy ON lectures FOR SELECT USING (
    EXISTS (SELECT 1 FROM class_sections cs WHERE cs.id = lectures.class_section_id AND cs.school_id = get_school_id())
);

-- INSERT/UPDATE/DELETE: Teacher assigned to class
CREATE POLICY lectures_modify_policy ON lectures FOR ALL USING (
    EXISTS (
        SELECT 1 FROM teaching_assignments ta
        JOIN teachers t ON t.id = ta.teacher_id
        WHERE ta.class_section_id = lectures.class_section_id AND t.user_id = get_user_id()
    )
    OR is_school_manager()
) WITH CHECK (
    EXISTS (
        SELECT 1 FROM teaching_assignments ta
        JOIN teachers t ON t.id = ta.teacher_id
        WHERE ta.class_section_id = lectures.class_section_id AND t.user_id = get_user_id()
    )
    OR is_school_manager()
);

-- -------------------- CLASS_MATERIALS --------------------
ALTER TABLE class_materials ENABLE ROW LEVEL SECURITY;
ALTER TABLE class_materials FORCE ROW LEVEL SECURITY;

-- SELECT: Via class_section access
CREATE POLICY class_materials_select_policy ON class_materials FOR SELECT USING (
    EXISTS (SELECT 1 FROM class_sections cs WHERE cs.id = class_materials.class_section_id AND cs.school_id = get_school_id())
);

-- INSERT: Teacher assigned to class
CREATE POLICY class_materials_insert_policy ON class_materials FOR INSERT WITH CHECK (
    EXISTS (
        SELECT 1 FROM teaching_assignments ta
        JOIN teachers t ON t.id = ta.teacher_id
        WHERE ta.class_section_id = class_materials.class_section_id AND t.user_id = get_user_id()
    )
);

-- UPDATE/DELETE: Creator or SchoolManager
CREATE POLICY class_materials_update_policy ON class_materials FOR UPDATE USING (
    created_by = get_user_id() OR is_school_manager()
);

CREATE POLICY class_materials_delete_policy ON class_materials FOR DELETE USING (
    created_by = get_user_id() OR is_school_manager()
);

-- -------------------- MATERIAL_EMBEDDINGS --------------------
ALTER TABLE material_embeddings ENABLE ROW LEVEL SECURITY;
ALTER TABLE material_embeddings FORCE ROW LEVEL SECURITY;

-- SELECT: Via material -> class_section
CREATE POLICY material_embeddings_select_policy ON material_embeddings FOR SELECT USING (
    EXISTS (
        SELECT 1 FROM class_materials cm
        JOIN class_sections cs ON cs.id = cm.class_section_id
        WHERE cm.id = material_embeddings.material_id AND cs.school_id = get_school_id()
    )
);

-- INSERT/UPDATE/DELETE: System only (service role bypasses, or teacher who owns material)
CREATE POLICY material_embeddings_modify_policy ON material_embeddings FOR ALL USING (
    EXISTS (
        SELECT 1 FROM class_materials cm
        WHERE cm.id = material_embeddings.material_id AND cm.created_by = get_user_id()
    )
) WITH CHECK (
    EXISTS (
        SELECT 1 FROM class_materials cm
        WHERE cm.id = material_embeddings.material_id AND cm.created_by = get_user_id()
    )
);

-- =====================================================
-- TIER 4: SYSTEM TABLES
-- =====================================================

-- -------------------- NOTIFICATIONS --------------------
ALTER TABLE notifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE notifications FORCE ROW LEVEL SECURITY;

-- SELECT: Own notifications only
CREATE POLICY notifications_select_policy ON notifications FOR SELECT USING (
    user_id = get_user_id()
);

-- INSERT: System only (no direct INSERT by users, use functions)
-- This policy allows the system (service role bypasses) to insert
CREATE POLICY notifications_insert_policy ON notifications FOR INSERT WITH CHECK (
    is_school_manager() -- Only managers or system can create
);

-- UPDATE: User can mark as read
CREATE POLICY notifications_update_policy ON notifications FOR UPDATE USING (
    user_id = get_user_id()
);

-- DELETE: Not allowed via RLS
CREATE POLICY notifications_delete_policy ON notifications FOR DELETE USING (false);

-- -------------------- PROFILES --------------------
ALTER TABLE profiles ENABLE ROW LEVEL SECURITY;
ALTER TABLE profiles FORCE ROW LEVEL SECURITY;

-- SELECT: Own profile or SchoolManager
CREATE POLICY profiles_select_policy ON profiles FOR SELECT USING (
    user_id = get_user_id()
    OR is_school_manager()
);

-- INSERT/UPDATE: Own profile
CREATE POLICY profiles_modify_policy ON profiles FOR ALL USING (
    user_id = get_user_id()
) WITH CHECK (
    user_id = get_user_id()
);

-- -------------------- PROFILE_CHANGE_REQUESTS --------------------
ALTER TABLE profile_change_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE profile_change_requests FORCE ROW LEVEL SECURITY;

-- SELECT: Own requests or SchoolManager
CREATE POLICY pcr_select_policy ON profile_change_requests FOR SELECT USING (
    user_id = get_user_id()
    OR requested_by = get_user_id()
    OR is_school_manager()
);

-- INSERT: Own requests
CREATE POLICY pcr_insert_policy ON profile_change_requests FOR INSERT WITH CHECK (
    requested_by = get_user_id()
);

-- UPDATE: SchoolManager can approve/reject
CREATE POLICY pcr_update_policy ON profile_change_requests FOR UPDATE USING (
    is_school_manager()
);

-- -------------------- REPORTS --------------------
ALTER TABLE reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE reports FORCE ROW LEVEL SECURITY;

-- SELECT: Student's own, parent's child, teacher who created
CREATE POLICY reports_select_policy ON reports FOR SELECT USING (
    -- Student sees own
    EXISTS (SELECT 1 FROM students s WHERE s.id = reports.student_id AND s.user_id = get_user_id())
    -- Parent sees child's
    OR EXISTS (SELECT 1 FROM students s WHERE s.id = reports.student_id AND s.parent_id = get_user_id())
    -- Teacher who created
    OR EXISTS (SELECT 1 FROM teachers t WHERE t.id = reports.teacher_id AND t.user_id = get_user_id())
    -- SchoolManager
    OR is_school_manager()
);

-- INSERT: Teacher creates
CREATE POLICY reports_insert_policy ON reports FOR INSERT WITH CHECK (
    EXISTS (SELECT 1 FROM teachers t WHERE t.id = reports.teacher_id AND t.user_id = get_user_id())
);

-- UPDATE/DELETE: Teacher who created
CREATE POLICY reports_update_policy ON reports FOR UPDATE USING (
    EXISTS (SELECT 1 FROM teachers t WHERE t.id = reports.teacher_id AND t.user_id = get_user_id())
);

-- -------------------- INVITES --------------------
ALTER TABLE invites ENABLE ROW LEVEL SECURITY;
ALTER TABLE invites FORCE ROW LEVEL SECURITY;

-- SELECT: SchoolManager for their school
CREATE POLICY invites_select_policy ON invites FOR SELECT USING (
    is_school_manager() AND school_id = get_school_id()
);

-- INSERT: SchoolManager creates
CREATE POLICY invites_insert_policy ON invites FOR INSERT WITH CHECK (
    is_school_manager() AND school_id = get_school_id()
);

-- UPDATE/DELETE: SchoolManager
CREATE POLICY invites_modify_policy ON invites FOR UPDATE USING (
    is_school_manager() AND school_id = get_school_id()
);

CREATE POLICY invites_delete_policy ON invites FOR DELETE USING (
    is_school_manager() AND school_id = get_school_id()
);

-- -------------------- AUDIT_LOGS --------------------
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;
-- NOTE: Not using FORCE - service role bypasses to allow system logging
-- ALTER TABLE audit_logs FORCE ROW LEVEL SECURITY;

-- SELECT: SchoolManager only (for their school's users)
CREATE POLICY audit_logs_select_policy ON audit_logs FOR SELECT USING (
    is_school_manager() AND EXISTS (
        SELECT 1 FROM users u WHERE u.id = audit_logs.actor_id AND u.school_id = get_school_id()
    )
);

-- INSERT: Denied via RLS (service role inserts directly)
CREATE POLICY audit_logs_insert_policy ON audit_logs FOR INSERT WITH CHECK (false);

-- UPDATE/DELETE: Never
CREATE POLICY audit_logs_update_policy ON audit_logs FOR UPDATE USING (false);
CREATE POLICY audit_logs_delete_policy ON audit_logs FOR DELETE USING (false);

-- -------------------- USER_PREFERENCES --------------------
-- Check if table exists first (it may have been created in a later migration)
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'user_preferences') THEN
        ALTER TABLE user_preferences ENABLE ROW LEVEL SECURITY;
        ALTER TABLE user_preferences FORCE ROW LEVEL SECURITY;
        
        -- Drop policies if they exist (PostgreSQL doesn't support IF NOT EXISTS for CREATE POLICY)
        DROP POLICY IF EXISTS user_preferences_select_policy ON user_preferences;
        DROP POLICY IF EXISTS user_preferences_modify_policy ON user_preferences;
        
        -- SELECT: Own preferences
        CREATE POLICY user_preferences_select_policy ON user_preferences FOR SELECT USING (
            user_id = get_user_id()
        );
        
        -- INSERT/UPDATE: Own preferences
        CREATE POLICY user_preferences_modify_policy ON user_preferences FOR ALL USING (
            user_id = get_user_id()
        ) WITH CHECK (
            user_id = get_user_id()
        );
    END IF;
END $$;

-- =====================================================
-- VERIFICATION QUERY
-- Run this after migration to verify all tables have RLS
-- =====================================================
-- SELECT schemaname, tablename, rowsecurity 
-- FROM pg_tables 
-- WHERE schemaname = 'public' 
--   AND tablename NOT IN ('_sqlx_migrations', 'roles', 'subjects')
-- ORDER BY tablename;
