-- Performance optimization indexes
-- These indexes target frequently used queries in dashboard_functions.rs

-- Index for looking up students by school (used in various dashboard queries)
CREATE INDEX IF NOT EXISTS idx_students_school_id ON students(school_id);

-- Index for looking up teachers by school
CREATE INDEX IF NOT EXISTS idx_teachers_school_id ON teachers(school_id);

-- Index for students.parent_id (used for parent dashboard lookups)
CREATE INDEX IF NOT EXISTS idx_students_parent_id ON students(parent_id) WHERE parent_id IS NOT NULL;

-- Index for class_sections by school_id (used frequently)
CREATE INDEX IF NOT EXISTS idx_class_sections_school_id ON class_sections(school_id);

-- Composite index for custom_assignments by student and status (used in dashboard stats)
CREATE INDEX IF NOT EXISTS idx_custom_assignments_student_status ON custom_assignments(student_id, status);

-- Index for custom_assignments by due_at (for pending assignment queries)
CREATE INDEX IF NOT EXISTS idx_custom_assignments_due_at ON custom_assignments(due_at);

-- Composite index for custom_assignments by student and due_at (for sorting)
CREATE INDEX IF NOT EXISTS idx_custom_assignments_student_due ON custom_assignments(student_id, due_at DESC);

-- Index for teaching_assignments by teacher_id (for teacher dashboard queries)
CREATE INDEX IF NOT EXISTS idx_teaching_assignments_teacher_id ON teaching_assignments(teacher_id);

-- Index for assignments by subject_id
CREATE INDEX IF NOT EXISTS idx_assignments_subject_id ON assignments(subject_id);

-- Composite index for submissions by student and grade (for GPA calculations)
CREATE INDEX IF NOT EXISTS idx_submissions_student_grade ON submissions(student_id, grade) WHERE grade IS NOT NULL;

-- Index for users by school_id (for user management queries)
-- First add the column if it doesn't exist (it should from schema)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'users' AND column_name = 'school_id'
    ) THEN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_users_school_id ON users(school_id)';
    END IF;
END $$;

-- Analyze tables to update statistics for query planner
ANALYZE students;
ANALYZE teachers;
ANALYZE enrollments;
ANALYZE custom_assignments;
ANALYZE submissions;
ANALYZE class_sections;
ANALYZE teaching_assignments;
ANALYZE assignments;
ANALYZE users;
