-- PR-01 assignment authorization: make publication fan-out idempotent.
--
-- The authorized publish path can be retried or executed concurrently. A
-- database uniqueness boundary is required so one student receives at most one
-- custom assignment for a given assignment. Fail closed if historical duplicate
-- rows exist; they require explicit operator review instead of silent deletion.

DO $assignment_publish_duplicates$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM custom_assignments
        GROUP BY assignment_id, student_id
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            'Cannot enforce assignment publication idempotency: duplicate custom assignments exist';
    END IF;
END
$assignment_publish_duplicates$;

CREATE UNIQUE INDEX IF NOT EXISTS custom_assignments_assignment_student_unique
    ON custom_assignments (assignment_id, student_id);
