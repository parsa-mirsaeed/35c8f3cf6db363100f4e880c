-- The users table stores role_id, not a denormalized role column. Resolve the
-- role through the canonical roles table so the persistence-boundary guard also
-- works at runtime, not only at function-creation time.
CREATE OR REPLACE FUNCTION prevent_teacher_document_ingestion()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = public
AS $$
BEGIN
    IF NEW.material_type = 'document'
       AND EXISTS (
            SELECT 1
            FROM users app_user
            JOIN roles role ON role.id = app_user.role_id
            WHERE app_user.id = NEW.created_by
              AND role.name = 'Teacher'::role_name
       ) THEN
        RAISE EXCEPTION 'Teacher document upload is disabled; use a published knowledge asset'
            USING ERRCODE = '42501';
    END IF;
    RETURN NEW;
END;
$$;
