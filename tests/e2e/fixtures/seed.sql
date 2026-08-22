-- PR-12 synthetic tenant fixture (plan §8). Deterministic IDs; no real data.
-- Loaded into the local CI PostgreSQL before the production-like server starts.
--
-- Schools A and B, a PlatformAdmin, managers/teachers/students/parents for both
-- schools, active and inactive accounts, classes, published assignments,
-- submissions/grades, and published knowledge assets. Separate desktop/mobile
-- assignment IDs keep stateful acceptance deterministic across Playwright projects.

BEGIN;

INSERT INTO schools (id, name) VALUES
  ('a0000000-0000-0000-0000-0000000000a1', 'E2E School A'),
  ('a0000000-0000-0000-0000-0000000000b1', 'E2E School B')
ON CONFLICT (id) DO NOTHING;

INSERT INTO users (id, name, email, role_id, school_id, is_active) VALUES
  ('b0000000-0000-0000-0000-0000000000a0', 'E2E Platform Admin',  'e2e-admin@example.test',    (SELECT id FROM roles WHERE name = 'PlatformAdmin'),  'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000a1', 'E2E Manager A',      'e2e-manager-a@example.test',(SELECT id FROM roles WHERE name = 'SchoolManager'), 'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b1', 'E2E Manager B',      'e2e-manager-b@example.test',(SELECT id FROM roles WHERE name = 'SchoolManager'), 'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a2', 'E2E Teacher A',      'e2e-teacher-a@example.test',(SELECT id FROM roles WHERE name = 'Teacher'),       'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b2', 'E2E Teacher B',      'e2e-teacher-b@example.test',(SELECT id FROM roles WHERE name = 'Teacher'),       'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a3', 'E2E Student A',      'e2e-student-a@example.test',(SELECT id FROM roles WHERE name = 'Student'),       'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b3', 'E2E Student B',      'e2e-student-b@example.test',(SELECT id FROM roles WHERE name = 'Student'),       'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a4', 'E2E Parent A',       'e2e-parent-a@example.test', (SELECT id FROM roles WHERE name = 'Parent'),        'a0000000-0000-0000-0000-0000000000a1', true),
  ('b0000000-0000-0000-0000-0000000000b4', 'E2E Parent B',       'e2e-parent-b@example.test', (SELECT id FROM roles WHERE name = 'Parent'),        'a0000000-0000-0000-0000-0000000000b1', true),
  ('b0000000-0000-0000-0000-0000000000a9', 'E2E Inactive Teacher','e2e-inactive@example.test',(SELECT id FROM roles WHERE name = 'Teacher'),       'a0000000-0000-0000-0000-0000000000a1', false)
ON CONFLICT (id) DO NOTHING;

INSERT INTO teachers (id, user_id, school_id) VALUES
  ('c0000000-0000-0000-0000-0000000000a2', 'b0000000-0000-0000-0000-0000000000a2', 'a0000000-0000-0000-0000-0000000000a1'),
  ('c0000000-0000-0000-0000-0000000000b2', 'b0000000-0000-0000-0000-0000000000b2', 'a0000000-0000-0000-0000-0000000000b1')
ON CONFLICT (id) DO NOTHING;

INSERT INTO students (id, user_id, school_id, parent_id) VALUES
  ('c0000000-0000-0000-0000-0000000000a3', 'b0000000-0000-0000-0000-0000000000a3', 'a0000000-0000-0000-0000-0000000000a1', 'b0000000-0000-0000-0000-0000000000a4'),
  ('c0000000-0000-0000-0000-0000000000b3', 'b0000000-0000-0000-0000-0000000000b3', 'a0000000-0000-0000-0000-0000000000b1', 'b0000000-0000-0000-0000-0000000000b4')
ON CONFLICT (id) DO NOTHING;

INSERT INTO subjects (id, code, name) VALUES
  ('d0000000-0000-0000-0000-0000000000a1', 'E2EMATH', 'E2E Mathematics')
ON CONFLICT (id) DO NOTHING;

INSERT INTO class_sections (id, school_id, subject_id, name, term) VALUES
  ('e0000000-0000-0000-0000-0000000000a1', 'a0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Class A1', 'E2E-Term'),
  ('e0000000-0000-0000-0000-0000000000a2', 'a0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Empty Class A', 'E2E-Term'),
  ('e0000000-0000-0000-0000-0000000000b1', 'a0000000-0000-0000-0000-0000000000b1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Class B1', 'E2E-Term')
ON CONFLICT (id) DO NOTHING;

INSERT INTO teaching_assignments (class_section_id, teacher_id) VALUES
  ('e0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a2'),
  ('e0000000-0000-0000-0000-0000000000a2', 'c0000000-0000-0000-0000-0000000000a2'),
  ('e0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b2')
ON CONFLICT DO NOTHING;

INSERT INTO enrollments (class_section_id, student_id) VALUES
  ('e0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a3'),
  ('e0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b3')
ON CONFLICT DO NOTHING;

INSERT INTO assignments (id, teacher_id, class_section_id, subject_id, title, body, due_at, status, published_at) VALUES
  ('f0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Assignment A1', 'Synthetic body', NOW() + INTERVAL '7 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a2', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Submission Journey Desktop', 'Desktop stateful acceptance body', NOW() + INTERVAL '8 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a3', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Submission Journey Mobile', 'Mobile stateful acceptance body', NOW() + INTERVAL '9 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a4', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Authorization Submission A', 'School A authorization probe', NOW() + INTERVAL '10 days', 'Published', NOW()),
  ('f0000000-0000-0000-0000-0000000000a5', 'c0000000-0000-0000-0000-0000000000a2', 'e0000000-0000-0000-0000-0000000000a2', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Guided Publish Draft', 'Must remain draft until an active student is enrolled.', NOW() + INTERVAL '11 days', 'Draft', NULL),
  ('f0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b2', 'e0000000-0000-0000-0000-0000000000b1', 'd0000000-0000-0000-0000-0000000000a1', 'E2E Authorization Submission B', 'School B authorization probe', NOW() + INTERVAL '10 days', 'Published', NOW())
ON CONFLICT (id) DO NOTHING;

INSERT INTO custom_assignments (id, assignment_id, student_id, due_at, status) VALUES
  ('f1000000-0000-0000-0000-0000000000a1', 'f0000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '7 days', 'Graded'),
  ('f1000000-0000-0000-0000-0000000000a2', 'f0000000-0000-0000-0000-0000000000a2', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '8 days', 'Assigned'),
  ('f1000000-0000-0000-0000-0000000000a3', 'f0000000-0000-0000-0000-0000000000a3', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '9 days', 'Assigned'),
  ('f1000000-0000-0000-0000-0000000000a4', 'f0000000-0000-0000-0000-0000000000a4', 'c0000000-0000-0000-0000-0000000000a3', NOW() + INTERVAL '10 days', 'Submitted'),
  ('f1000000-0000-0000-0000-0000000000b1', 'f0000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b3', NOW() + INTERVAL '10 days', 'Submitted')
ON CONFLICT (id) DO NOTHING;

INSERT INTO submissions (id, custom_assignment_id, student_id, content, grade, graded_by) VALUES
  ('f2000000-0000-0000-0000-0000000000a1', 'f1000000-0000-0000-0000-0000000000a1', 'c0000000-0000-0000-0000-0000000000a3', '{"text":"synthetic"}'::jsonb, 18.50, 'c0000000-0000-0000-0000-0000000000a2'),
  ('f2000000-0000-0000-0000-0000000000a4', 'f1000000-0000-0000-0000-0000000000a4', 'c0000000-0000-0000-0000-0000000000a3', '{"text":"school-a authorization submission"}'::jsonb, NULL, NULL),
  ('f2000000-0000-0000-0000-0000000000b1', 'f1000000-0000-0000-0000-0000000000b1', 'c0000000-0000-0000-0000-0000000000b3', '{"text":"school-b authorization submission"}'::jsonb, NULL, NULL)
ON CONFLICT (id) DO NOTHING;

INSERT INTO knowledge_assets (id, school_id, title, status, created_by, published_at) VALUES
  ('f3000000-0000-0000-0000-0000000000a1', 'a0000000-0000-0000-0000-0000000000a1', 'E2E Published Asset', 'published', 'b0000000-0000-0000-0000-0000000000a1', NOW()),
  ('f3000000-0000-0000-0000-0000000000b1', 'a0000000-0000-0000-0000-0000000000b1', 'E2E School B Asset', 'published', 'b0000000-0000-0000-0000-0000000000b1', NOW())
ON CONFLICT (id) DO NOTHING;

COMMIT;