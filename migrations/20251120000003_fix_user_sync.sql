-- ============================================
-- Diagnostic Queries for User Sync Issues
-- ============================================

-- 1. Check if user exists in application users table
SELECT 
    id, 
    name, 
    email, 
    role_id, 
    school_id, 
    is_active
FROM public.users 
WHERE id = '9c460ae9-4ff6-48e6-92e6-41058af93f02';

-- 2. List all users in your application
SELECT 
    u.id, 
    u.name, 
    u.email, 
    r.name as role_name,
    u.is_active
FROM public.users u
LEFT JOIN public.roles r ON u.role_id = r.id
ORDER BY u.created_at DESC;

-- 3. Check what roles exist
SELECT id, name FROM public.roles ORDER BY name;

-- 4. Check what schools exist
SELECT id, name FROM public.schools ORDER BY name;

-- ============================================
-- Fix: Create the missing user record
-- ============================================

-- IMPORTANT: You need to replace these values:
-- - Get the SchoolManager role_id from query #3
-- - Get or create a school_id from query #4

-- First, ensure you have a school (if not, create one):
INSERT INTO public.schools (id, name, created_at)
VALUES (gen_random_uuid(), 'Default School', NOW())
ON CONFLICT DO NOTHING
RETURNING id, name;

-- Then, create the user record that matches your Supabase Auth user:
-- Replace <school_id> and <role_id> with actual values from above queries

INSERT INTO public.users (
    id, 
    name, 
    email, 
    role_id, 
    school_id, 
    is_active, 
    created_at, 
    updated_at
)
VALUES (
    '9c460ae9-4ff6-48e6-92e6-41058af93f02',  -- Your Supabase user ID
    'Admin User',                              -- User's display name
    'admin@example.com',                       -- Email (must match Supabase)
    (SELECT id FROM public.roles WHERE name = 'SchoolManager' LIMIT 1),  -- Role ID
    (SELECT id FROM public.schools LIMIT 1),  -- School ID (first school)
    true,                                      -- Active user
    NOW(),
    NOW()
)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    email = EXCLUDED.email,
    is_active = EXCLUDED.is_active,
    updated_at = NOW();

-- 5. Verify the user was created correctly
SELECT 
    u.id, 
    u.name, 
    u.email, 
    r.name as role_name,
    s.name as school_name,
    u.is_active
FROM public.users u
LEFT JOIN public.roles r ON u.role_id = r.id
LEFT JOIN public.schools s ON u.school_id = s.id
WHERE u.id = '9c460ae9-4ff6-48e6-92e6-41058af93f02';
