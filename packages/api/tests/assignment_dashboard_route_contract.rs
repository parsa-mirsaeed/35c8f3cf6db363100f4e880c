fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source.find(start).expect("route start marker");
    let end = source[start..]
        .find(end)
        .map(|offset| start + offset)
        .expect("route end marker");
    &source[start..end]
}

#[test]
fn live_assignment_dashboard_routes_are_actor_scoped() {
    let source = include_str!("../src/server_functions/dashboard_functions.rs");
    let student = between(
        source,
        "pub async fn get_student_assignments()",
        "// ==================== Teacher Dashboard Functions",
    );
    let teacher = between(
        source,
        "pub async fn get_teacher_assignments()",
        "// ==================== Parent Dashboard Functions",
    );

    for required in [
        "resolve_active_student",
        "list_for_student",
        "student_user.is_active = TRUE",
        "student_role.name::text = 'Student'",
        "JOIN enrollments enrollment",
        "a.status = 'Published'::assignment_status",
    ] {
        assert!(student.contains(required), "missing student guard: {required}");
    }
    assert!(!student.contains("SELECT id FROM students WHERE user_id = $1"));

    for required in [
        "resolve_active_teacher",
        "list_for_teacher",
        "teacher_user.is_active = TRUE",
        "teacher_role.name::text = 'Teacher'",
        "JOIN teaching_assignments teaching_assignment",
        "cs.school_id = teacher_user.school_id",
    ] {
        assert!(teacher.contains(required), "missing teacher guard: {required}");
    }
    assert!(!teacher.contains("SELECT id FROM teachers WHERE user_id = $1"));
}
