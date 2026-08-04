use super::{AuthorizedAssignmentRepository, RepositoryError};
use crate::models::{CreateAssignmentRequest, UpdateAssignmentRequest};
use chrono::{Duration, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;
use uuid::Uuid;

async fn role_id(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar("SELECT id FROM roles WHERE name::text = $1 LIMIT 1")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("missing {name} role fixture: {error}"))
}

async fn insert_user(
    pool: &PgPool,
    suffix: &str,
    label: &str,
    role_id: Uuid,
    school_id: Uuid,
    active: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, '{}'::jsonb)
        "#,
    )
    .bind(id)
    .bind(label)
    .bind(format!("{}-{suffix}@example.test", label.to_lowercase().replace(' ', "-")))
    .bind(role_id)
    .bind(school_id)
    .bind(active)
    .execute(pool)
    .await
    .unwrap_or_else(|error| panic!("insert {label}: {error}"));
    id
}

async fn insert_teacher(pool: &PgPool, user_id: Uuid, school_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teachers (id, user_id, school_id, subject, created_at) VALUES ($1, $2, $3, NULL, NOW())",
    )
    .bind(id)
    .bind(user_id)
    .bind(school_id)
    .execute(pool)
    .await
    .expect("insert teacher");
    id
}

async fn insert_class(
    pool: &PgPool,
    school_id: Uuid,
    subject_id: Uuid,
    suffix: &str,
    label: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO class_sections (id, school_id, subject_id, name, term) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(school_id)
    .bind(subject_id)
    .bind(format!("{label} {suffix}"))
    .bind("2026")
    .execute(pool)
    .await
    .expect("insert class");
    id
}

fn assignment_request(
    class_section_id: Uuid,
    subject_id: Uuid,
    title: &str,
) -> CreateAssignmentRequest {
    CreateAssignmentRequest {
        class_section_id: class_section_id.into(),
        subject_id: subject_id.into(),
        lecture_id: None,
        lecture_title: None,
        lecture_number: None,
        title: title.into(),
        body: "Authorization regression fixture".into(),
        due_at: Utc::now() + Duration::days(7),
        material_ids: None,
    }
}

#[cfg(feature = "server")]
#[tokio::test]
async fn required_teacher_mutation_matrix_is_enforced() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is required for assignment authorization tests");
    let pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(8)
            .connect(&database_url)
            .await
            .expect("connect assignment authorization database"),
    );
    let repository = AuthorizedAssignmentRepository::new(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();

    let school_a = Uuid::new_v4();
    let school_b = Uuid::new_v4();
    sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2), ($3, $4)")
        .bind(school_a)
        .bind(format!("Assignment Matrix School A {suffix}"))
        .bind(school_b)
        .bind(format!("Assignment Matrix School B {suffix}"))
        .execute(&*pool)
        .await
        .expect("insert schools");

    let subject_id = Uuid::new_v4();
    sqlx::query("INSERT INTO subjects (id, code, name) VALUES ($1, $2, $3)")
        .bind(subject_id)
        .bind(format!("ASSIGN-{suffix}"))
        .bind(format!("Assignment Matrix Subject {suffix}"))
        .execute(&*pool)
        .await
        .expect("insert subject");

    let teacher_role = role_id(&pool, "Teacher").await;
    let student_role = role_id(&pool, "Student").await;
    let parent_role = role_id(&pool, "Parent").await;
    let manager_role = role_id(&pool, "SchoolManager").await;

    let teacher_a_user = insert_user(
        &pool,
        &suffix,
        "Teacher A",
        teacher_role,
        school_a,
        true,
    )
    .await;
    let teacher_a2_user = insert_user(
        &pool,
        &suffix,
        "Teacher A2",
        teacher_role,
        school_a,
        true,
    )
    .await;
    let teacher_b_user = insert_user(
        &pool,
        &suffix,
        "Teacher B",
        teacher_role,
        school_b,
        true,
    )
    .await;
    let inactive_teacher_user = insert_user(
        &pool,
        &suffix,
        "Inactive Teacher",
        teacher_role,
        school_a,
        false,
    )
    .await;
    let student_user = insert_user(
        &pool,
        &suffix,
        "Student Actor",
        student_role,
        school_a,
        true,
    )
    .await;
    let parent_user = insert_user(
        &pool,
        &suffix,
        "Parent Actor",
        parent_role,
        school_a,
        true,
    )
    .await;
    let manager_user = insert_user(
        &pool,
        &suffix,
        "Manager Actor",
        manager_role,
        school_a,
        true,
    )
    .await;

    let teacher_a_id = insert_teacher(&pool, teacher_a_user, school_a).await;
    let teacher_a2_id = insert_teacher(&pool, teacher_a2_user, school_a).await;
    let teacher_b_id = insert_teacher(&pool, teacher_b_user, school_b).await;
    insert_teacher(&pool, inactive_teacher_user, school_a).await;

    let student_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO students (id, user_id, school_id, parent_id, talent_profile_ref, created_at) VALUES ($1, $2, $3, NULL, NULL, NOW())",
    )
    .bind(student_id)
    .bind(student_user)
    .bind(school_a)
    .execute(&*pool)
    .await
    .expect("insert student");

    let class_a = insert_class(&pool, school_a, subject_id, &suffix, "Class A").await;
    let class_b = insert_class(&pool, school_b, subject_id, &suffix, "Class B").await;
    for (class_section_id, teacher_id) in
        [(class_a, teacher_a_id), (class_b, teacher_b_id)]
    {
        sqlx::query(
            "INSERT INTO teaching_assignments (id, class_section_id, teacher_id) VALUES ($1, $2, $3)",
        )
        .bind(Uuid::new_v4())
        .bind(class_section_id)
        .bind(teacher_id)
        .execute(&*pool)
        .await
        .expect("insert teaching assignment");
    }
    sqlx::query(
        "INSERT INTO enrollments (id, class_section_id, student_id, enrolled_at) VALUES ($1, $2, $3, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(class_a)
    .bind(student_id)
    .execute(&*pool)
    .await
    .expect("insert enrollment");

    let actor_a = repository
        .resolve_active_teacher(teacher_a_user, "Teacher")
        .await
        .expect("resolve Teacher A");
    let actor_a2 = repository
        .resolve_active_teacher(teacher_a2_user, "Teacher")
        .await
        .expect("resolve same-school Teacher A2");
    let actor_b = repository
        .resolve_active_teacher(teacher_b_user, "Teacher")
        .await
        .expect("resolve cross-school Teacher B");

    for (user_id, role) in [
        (student_user, "Student"),
        (parent_user, "Parent"),
        (manager_user, "SchoolManager"),
        (inactive_teacher_user, "Teacher"),
    ] {
        assert!(
            matches!(
                repository.resolve_active_teacher(user_id, role).await,
                Err(RepositoryError::Unauthorized)
            ),
            "{role} must not obtain a teacher mutation actor"
        );
    }

    let assignment = repository
        .create_for_teacher(
            actor_a,
            assignment_request(class_a, subject_id, "Owned assignment"),
        )
        .await
        .expect("Teacher A creates own assignment");

    let updated = repository
        .update_for_teacher(
            actor_a,
            assignment.id,
            UpdateAssignmentRequest {
                title: Some("Owned assignment updated".into()),
                body: None,
                due_at: None,
                lecture_title: None,
                lecture_number: None,
            },
        )
        .await
        .expect("Teacher A updates own assignment");
    assert_eq!(updated.title, "Owned assignment updated");

    for actor in [actor_a2, actor_b] {
        assert!(matches!(
            repository
                .update_for_teacher(
                    actor,
                    assignment.id,
                    UpdateAssignmentRequest {
                        title: Some("Unauthorized update".into()),
                        body: None,
                        due_at: None,
                        lecture_title: None,
                        lecture_number: None,
                    },
                )
                .await,
            Err(RepositoryError::NotFound { .. })
        ));
        assert!(matches!(
            repository.publish_for_teacher(actor, assignment.id).await,
            Err(RepositoryError::NotFound { .. })
        ));
    }

    repository
        .publish_for_teacher(actor_a, assignment.id)
        .await
        .expect("Teacher A publishes own assignment");

    let delete_target = repository
        .create_for_teacher(
            actor_a,
            assignment_request(class_a, subject_id, "Owned delete target"),
        )
        .await
        .expect("Teacher A creates delete target");
    repository
        .delete_for_teacher(actor_a, delete_target.id)
        .await
        .expect("Teacher A deletes own assignment");
    assert!(matches!(
        repository.find_for_teacher(actor_a, delete_target.id).await,
        Err(RepositoryError::NotFound { .. })
    ));

    assert_ne!(teacher_a2_id, teacher_a_id);
}
