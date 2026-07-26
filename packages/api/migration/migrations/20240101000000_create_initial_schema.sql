-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create custom enum types
CREATE TYPE role_name AS ENUM ('SchoolManager', 'Teacher', 'Parent', 'Student');
CREATE TYPE assignment_status AS ENUM ('Draft', 'Published', 'InProgress', 'Submitted', 'Graded', 'Archived');
CREATE TYPE custom_status AS ENUM ('Assigned', 'InProgress', 'Submitted', 'Graded');
CREATE TYPE pcr_status AS ENUM ('PENDING', 'APPROVED', 'REJECTED');

-- Create roles table
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name role_name UNIQUE NOT NULL,
    permissions JSONB NOT NULL DEFAULT '{}'
);

-- Create users table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    role_id UUID NOT NULL REFERENCES roles(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create schools table
CREATE TABLE schools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create subjects table
CREATE TABLE subjects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL
);

-- Create class_sections table
CREATE TABLE class_sections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    school_id UUID NOT NULL REFERENCES schools(id),
    subject_id UUID NOT NULL REFERENCES subjects(id),
    name TEXT NOT NULL,
    term TEXT NOT NULL,
    UNIQUE(school_id, subject_id, name, term)
);

-- Create students table
CREATE TABLE students (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID UNIQUE NOT NULL REFERENCES users(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    parent_id UUID REFERENCES users(id),
    talent_profile_ref TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create teachers table
CREATE TABLE teachers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID UNIQUE NOT NULL REFERENCES users(id),
    school_id UUID NOT NULL REFERENCES schools(id),
    subject TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create lectures table
CREATE TABLE lectures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_section_id UUID NOT NULL REFERENCES class_sections(id),
    topic TEXT NOT NULL,
    sequence_no INTEGER NOT NULL,
    held_on DATE NOT NULL,
    UNIQUE(class_section_id, sequence_no)
);

-- Create enrollments table
CREATE TABLE enrollments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_section_id UUID NOT NULL REFERENCES class_sections(id),
    student_id UUID NOT NULL REFERENCES students(id),
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(class_section_id, student_id)
);

-- Create teaching_assignments table
CREATE TABLE teaching_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_section_id UUID NOT NULL REFERENCES class_sections(id),
    teacher_id UUID NOT NULL REFERENCES teachers(id),
    UNIQUE(class_section_id, teacher_id)
);

-- Create assignments table
CREATE TABLE assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    teacher_id UUID NOT NULL REFERENCES teachers(id),
    class_section_id UUID NOT NULL REFERENCES class_sections(id),
    subject_id UUID NOT NULL REFERENCES subjects(id),
    lecture_id UUID REFERENCES lectures(id),
    lecture_title TEXT,
    lecture_number INTEGER,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    due_at TIMESTAMPTZ NOT NULL,
    status assignment_status NOT NULL DEFAULT 'Draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

-- Create custom_assignments table
CREATE TABLE custom_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    assignment_id UUID NOT NULL REFERENCES assignments(id),
    student_id UUID NOT NULL REFERENCES students(id),
    prompt_ctx JSONB,
    rubric JSONB,
    due_at TIMESTAMPTZ NOT NULL,
    status custom_status NOT NULL DEFAULT 'Assigned',
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    submitted_at TIMESTAMPTZ,
    graded_at TIMESTAMPTZ,
    UNIQUE(assignment_id, student_id)
);

-- Create submissions table
CREATE TABLE submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    custom_assignment_id UUID NOT NULL REFERENCES custom_assignments(id),
    student_id UUID NOT NULL REFERENCES students(id),
    content JSONB NOT NULL DEFAULT '{}',
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    grade NUMERIC(5,2) CHECK (grade >= 0 AND grade <= 100),
    feedback TEXT,
    graded_by UUID REFERENCES teachers(id),
    grading_rubric JSONB
);

-- Create reports table
CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    student_id UUID NOT NULL REFERENCES students(id),
    teacher_id UUID REFERENCES teachers(id),
    ai_summary TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create profiles table
CREATE TABLE profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID UNIQUE NOT NULL REFERENCES users(id),
    fields JSONB NOT NULL DEFAULT '{}',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create profile_change_requests table
CREATE TABLE profile_change_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    payload_diff JSONB NOT NULL,
    requested_by UUID NOT NULL REFERENCES users(id),
    status pcr_status NOT NULL DEFAULT 'PENDING',
    decided_by UUID REFERENCES users(id),
    decided_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create audit_logs table
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id UUID NOT NULL REFERENCES users(id),
    action TEXT NOT NULL,
    entity TEXT NOT NULL,
    entity_id UUID,
    "before" JSONB,
    "after" JSONB,
    at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ip INET,
    user_agent TEXT
);

-- Create indexes for performance
-- Note: users_email_key is automatically created by UNIQUE constraint on email
CREATE INDEX users_role_id_idx ON users(role_id);
CREATE INDEX users_updated_at_idx ON users(updated_at);

-- Note: enrollments_class_section_id_student_id_key is automatically created by UNIQUE constraint
CREATE INDEX enrollments_student_id_idx ON enrollments(student_id);

-- Note: teaching_assignments_class_section_id_teacher_id_key is automatically created by UNIQUE constraint

CREATE INDEX assignments_teacher_id_idx ON assignments(teacher_id);
CREATE INDEX assignments_status_idx ON assignments(status);
CREATE INDEX assignments_class_section_id_idx ON assignments(class_section_id);
CREATE INDEX assignments_due_at_idx ON assignments(due_at);

-- Note: custom_assignments_assignment_id_student_id_key is automatically created by UNIQUE constraint
CREATE INDEX custom_assignments_status_idx ON custom_assignments(status);

CREATE INDEX submissions_custom_assignment_id_idx ON submissions(custom_assignment_id);
CREATE INDEX submissions_student_id_idx ON submissions(student_id);

CREATE INDEX reports_student_id_created_at_idx ON reports(student_id, created_at DESC);

-- Note: profiles_user_id_key is automatically created by UNIQUE constraint

CREATE INDEX pcr_user_id_status_idx ON profile_change_requests(user_id, status);

CREATE INDEX audit_logs_actor_id_at_idx ON audit_logs(actor_id, at);
CREATE INDEX audit_logs_entity_entity_id_idx ON audit_logs(entity, entity_id);

-- Create triggers for updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_profiles_updated_at BEFORE UPDATE ON profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default roles
INSERT INTO roles (name, permissions) VALUES
('SchoolManager', '{"can_manage_users": true, "can_manage_classes": true, "can_approve_profiles": true, "can_view_all_data": true}'),
('Teacher', '{"can_create_assignments": true, "can_grade_submissions": true, "can_view_own_students": true}'),
('Parent', '{"can_view_child_data": true, "can_request_profile_changes": true}'),
('Student', '{"can_submit_assignments": true, "can_view_own_data": true, "can_request_profile_changes": true}');