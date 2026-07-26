-- Legacy subject seed.
--
-- Some historical migration sequences place this file before the migration that
-- creates public.subjects. Keep the migration safe for fresh installations; the
-- idempotent late seed migration replays the same data after the full schema is
-- present.
DO $$
BEGIN
    IF to_regclass('public.subjects') IS NULL THEN
        RAISE NOTICE 'public.subjects does not exist yet; deferring subject seed';
        RETURN;
    END IF;

    INSERT INTO public.subjects (id, code, name) VALUES
        (gen_random_uuid(), 'MATH', 'Mathematics'),
        (gen_random_uuid(), 'ENG', 'English Language & Literature'),
        (gen_random_uuid(), 'SCI', 'Science'),
        (gen_random_uuid(), 'PHYS', 'Physics'),
        (gen_random_uuid(), 'CHEM', 'Chemistry'),
        (gen_random_uuid(), 'BIO', 'Biology'),
        (gen_random_uuid(), 'HIST', 'History'),
        (gen_random_uuid(), 'GEO', 'Geography'),
        (gen_random_uuid(), 'CS', 'Computer Science'),
        (gen_random_uuid(), 'ART', 'Art & Design'),
        (gen_random_uuid(), 'MUS', 'Music'),
        (gen_random_uuid(), 'PE', 'Physical Education'),
        (gen_random_uuid(), 'LANG', 'Foreign Languages'),
        (gen_random_uuid(), 'ECON', 'Economics'),
        (gen_random_uuid(), 'PSY', 'Psychology'),
        (gen_random_uuid(), 'SOC', 'Sociology'),
        (gen_random_uuid(), 'PHI', 'Philosophy'),
        (gen_random_uuid(), 'ENV', 'Environmental Science'),
        (gen_random_uuid(), 'STAT', 'Statistics'),
        (gen_random_uuid(), 'CALC', 'Calculus')
    ON CONFLICT DO NOTHING;
END
$$;
