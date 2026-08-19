#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL must be set}"

app_role="edutalent_app_ci"
app_password="RlsTransactionProbe.Password_2026_safe"

# Keep this verifier independently runnable. Configuration is idempotent and
# preserves the production NOBYPASSRLS runtime-role contract.
DATABASE_ADMIN_URL="${DATABASE_URL}" \
DATABASE_APP_USER="${app_role}" \
DATABASE_APP_PASSWORD="${app_password}" \
    bash scripts/ci/configure_database_role.sh >/dev/null

# Fixed UUIDs make assertions readable. The CI database is disposable and the
# fixture is removed at the end of the script.
school_a="a1100000-0000-0000-0000-000000000001"
school_b="b1100000-0000-0000-0000-000000000002"
manager_a="a1200000-0000-0000-0000-000000000001"
teacher_a="a1200000-0000-0000-0000-000000000002"
parent_a="a1200000-0000-0000-0000-000000000003"
student_a="a1200000-0000-0000-0000-000000000004"
student_b="b1200000-0000-0000-0000-000000000005"
parent_b="b1200000-0000-0000-0000-000000000006"
teacher_row_a="a1300000-0000-0000-0000-000000000002"
student_row_a="a1400000-0000-0000-0000-000000000004"
student_row_b="b1400000-0000-0000-0000-000000000005"
subject_id="a1500000-0000-0000-0000-000000000001"
class_a="a1600000-0000-0000-0000-000000000001"
class_b="b1600000-0000-0000-0000-000000000002"
enrollment_a="a1700000-0000-0000-0000-000000000001"
enrollment_b="b1700000-0000-0000-0000-000000000002"
assignment_a="a1800000-0000-0000-0000-000000000001"

psql "${DATABASE_URL}" --set=ON_ERROR_STOP=1 <<SQL
BEGIN;

INSERT INTO public.schools (id, name) VALUES
    ('${school_a}', 'RLS Class Probe School A'),
    ('${school_b}', 'RLS Class Probe School B');

INSERT INTO public.subjects (id, code, name)
VALUES ('${subject_id}', 'RLS-PROBE', 'RLS Probe Subject');

INSERT INTO public.users (id, name, email, role_id, school_id, is_active)
SELECT '${manager_a}', 'RLS Manager A', 'rls-manager-a@example.invalid', id, '${school_a}', TRUE
FROM public.roles WHERE name = 'SchoolManager'::role_name;
INSERT INTO public.users (id, name, email, role_id, school_id, is_active)
SELECT '${teacher_a}', 'RLS Teacher A', 'rls-teacher-a@example.invalid', id, '${school_a}', TRUE
FROM public.roles WHERE name = 'Teacher'::role_name;
INSERT INTO public.users (id, name, email, role_id, school_id, is_active)
SELECT '${parent_a}', 'RLS Parent A', 'rls-parent-a@example.invalid', id, '${school_a}', TRUE
FROM public.roles WHERE name = 'Parent'::role_name;
INSERT INTO public.users (id, name, email, role_id, school_id, is_active)
SELECT '${student_a}', 'RLS Student A', 'rls-student-a@example.invalid', id, '${school_a}', TRUE
FROM public.roles WHERE name = 'Student'::role_name;
INSERT INTO public.users (id, name, email, role_id, school_id, is_active)
SELECT '${student_b}', 'RLS Student B', 'rls-student-b@example.invalid', id, '${school_b}', TRUE
FROM public.roles WHERE name = 'Student'::role_name;
INSERT INTO public.users (id, name, email, role_id, school_id, is_active)
SELECT '${parent_b}', 'RLS Parent B', 'rls-parent-b@example.invalid', id, '${school_b}', TRUE
FROM public.roles WHERE name = 'Parent'::role_name;

INSERT INTO public.teachers (id, user_id, school_id, subject)
VALUES ('${teacher_row_a}', '${teacher_a}', '${school_a}', 'RLS Probe Subject');

INSERT INTO public.students (id, user_id, school_id, parent_id)
VALUES
    ('${student_row_a}', '${student_a}', '${school_a}', '${parent_a}'),
    ('${student_row_b}', '${student_b}', '${school_b}', '${parent_b}');

INSERT INTO public.class_sections (id, school_id, subject_id, name, term)
VALUES
    ('${class_a}', '${school_a}', '${subject_id}', 'RLS Class A', 'CI'),
    ('${class_b}', '${school_b}', '${subject_id}', 'RLS Class B', 'CI');

INSERT INTO public.teaching_assignments (id, class_section_id, teacher_id)
VALUES ('${assignment_a}', '${class_a}', '${teacher_row_a}');

INSERT INTO public.enrollments (id, class_section_id, student_id)
VALUES
    ('${enrollment_a}', '${class_a}', '${student_row_a}'),
    ('${enrollment_b}', '${class_b}', '${student_row_b}');

COMMIT;
SQL

# Security properties of the recursion-breaking helper.
helper_state="$(
    psql "${DATABASE_URL}" --tuples-only --no-align --set=ON_ERROR_STOP=1 <<'SQL'
SELECT concat_ws('|',
    p.prosecdef::text,
    CASE WHEN p.provolatile = 's' THEN 'stable' ELSE p.provolatile::text END,
    COALESCE(array_to_string(p.proconfig, ','), ''),
    EXISTS (
        SELECT 1
        FROM aclexplode(p.proacl) acl
        WHERE acl.grantee = 0
          AND acl.privilege_type = 'EXECUTE'
    )::text
)
FROM pg_proc p
WHERE p.oid = 'public.enrollment_student_actor_matches(uuid)'::regprocedure;
SQL
)"
helper_state="$(echo "${helper_state}" | xargs)"
if [[ "${helper_state}" != "true|stable|search_path=pg_catalog, public|false" ]]; then
    echo "Unexpected enrollment helper security state: ${helper_state:-<missing>}" >&2
    exit 1
fi

runtime_helper_exec="$(
    PGPASSWORD="${app_password}" psql "${DATABASE_URL/postgresql:\/\//postgresql://${app_role}:${app_password}@}" \
        --tuples-only --no-align --set=ON_ERROR_STOP=1 \
        --command="SELECT has_function_privilege(current_user, 'public.enrollment_student_actor_matches(uuid)', 'EXECUTE')"
)"
runtime_helper_exec="$(echo "${runtime_helper_exec}" | xargs)"
if [[ "${runtime_helper_exec}" != "t" ]]; then
    echo "Runtime role cannot execute bounded enrollment helper" >&2
    exit 1
fi

runtime_url="$(python3 - "${DATABASE_URL}" "${app_role}" "${app_password}" <<'PY'
import sys
from urllib.parse import urlsplit, urlunsplit, quote

url, user, password = sys.argv[1:]
parts = urlsplit(url)
host = parts.hostname or 'localhost'
port = f':{parts.port}' if parts.port else ''
netloc = f'{quote(user)}:{quote(password)}@{host}{port}'
print(urlunsplit((parts.scheme, netloc, parts.path, parts.query, parts.fragment)))
PY
)"

query_enrollments() {
    local user_id="$1"
    local role="$2"
    local school_id="$3"
    PGPASSWORD="${app_password}" psql "${runtime_url}" \
        --quiet --tuples-only --no-align --set=ON_ERROR_STOP=1 <<SQL
BEGIN;
SET LOCAL app.user_id = '${user_id}';
SET LOCAL app.user_role = '${role}';
SET LOCAL app.school_id = '${school_id}';
SET LOCAL app.elevated_operation = 'false';
SELECT COALESCE(string_agg(cs.name, ',' ORDER BY cs.name), '<none>')
FROM public.enrollments e
JOIN public.class_sections cs ON cs.id = e.class_section_id;
ROLLBACK;
SQL
}

assert_visible_classes() {
    local label="$1"
    local expected="$2"
    local user_id="$3"
    local role="$4"
    local school_id="$5"
    local observed
    observed="$(query_enrollments "${user_id}" "${role}" "${school_id}" | grep -E '^(RLS Class|<none>)' | tail -n 1)"
    if [[ "${observed}" != "${expected}" ]]; then
        echo "${label} observed unexpected enrollment visibility: ${observed:-<none>} (expected ${expected})" >&2
        exit 1
    fi
}

assert_visible_classes "student own enrollment" "RLS Class A" "${student_a}" "Student" "${school_a}"
assert_visible_classes "parent child enrollment" "RLS Class A" "${parent_a}" "Parent" "${school_a}"
assert_visible_classes "assigned teacher enrollment" "RLS Class A" "${teacher_a}" "Teacher" "${school_a}"
assert_visible_classes "school manager same-school enrollment" "RLS Class A" "${manager_a}" "SchoolManager" "${school_a}"
assert_visible_classes "other-school student isolation" "RLS Class B" "${student_b}" "Student" "${school_b}"
assert_visible_classes "other-school parent isolation" "RLS Class B" "${parent_b}" "Parent" "${school_b}"

# Reproduce the exact query shape that failed in production UI exploration.
manager_class_result="$(
    PGPASSWORD="${app_password}" psql "${runtime_url}" \
        --quiet --tuples-only --no-align --field-separator='|' --set=ON_ERROR_STOP=1 <<SQL
BEGIN;
SET LOCAL app.user_id = '${manager_a}';
SET LOCAL app.user_role = 'SchoolManager';
SET LOCAL app.school_id = '${school_a}';
SET LOCAL app.elevated_operation = 'false';
SELECT
    cs.name,
    sub.code,
    COALESCE((SELECT COUNT(*) FROM public.enrollments e WHERE e.class_section_id = cs.id), 0)
FROM public.class_sections cs
JOIN public.subjects sub ON cs.subject_id = sub.id
WHERE cs.school_id = '${school_a}'::uuid
ORDER BY cs.name;
ROLLBACK;
SQL
)"
manager_class_result="$(echo "${manager_class_result}" | grep '^RLS Class' | tail -n 1)"
if [[ "${manager_class_result}" != "RLS Class A|RLS-PROBE|1" ]]; then
    echo "SchoolManager class-list regression query returned: ${manager_class_result:-<none>}" >&2
    exit 1
fi

# Remove probe data in FK-safe order.
psql "${DATABASE_URL}" --set=ON_ERROR_STOP=1 <<SQL
BEGIN;
DELETE FROM public.enrollments WHERE id IN ('${enrollment_a}', '${enrollment_b}');
DELETE FROM public.teaching_assignments WHERE id = '${assignment_a}';
DELETE FROM public.class_sections WHERE id IN ('${class_a}', '${class_b}');
DELETE FROM public.students WHERE id IN ('${student_row_a}', '${student_row_b}');
DELETE FROM public.teachers WHERE id = '${teacher_row_a}';
DELETE FROM public.users WHERE id IN (
    '${manager_a}', '${teacher_a}', '${parent_a}', '${student_a}', '${student_b}', '${parent_b}'
);
DELETE FROM public.subjects WHERE id = '${subject_id}';
DELETE FROM public.schools WHERE id IN ('${school_a}', '${school_b}');
COMMIT;
SQL

echo "class/enrollment RLS recursion and actor visibility matrix verified"
