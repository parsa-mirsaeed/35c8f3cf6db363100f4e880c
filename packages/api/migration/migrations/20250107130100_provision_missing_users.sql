-- ========================================
-- Provision missing local users for Supabase Auth users
-- This creates local records for users that exist in Supabase Auth but not in local DB
-- ========================================

-- Create missing users with default roles and school assignment
INSERT INTO users (id, name, email, role_id, school_id, is_active, created_at, updated_at)
SELECT
    a.id,
    COALESCE(a.raw_user_meta_data->>'name', 'Unknown User') as name,
    a.email,
    r.id as role_id,
    s.id as school_id,
    true as is_active,
    a.created_at,
    a.updated_at
FROM auth.users a
CROSS JOIN roles r
CROSS JOIN schools s
LEFT JOIN users u ON a.id = u.id
WHERE u.id IS NULL
  AND r.name = 'Student'  -- Default role
  AND s.name = 'Demo School'  -- Default school
ON CONFLICT (id) DO NOTHING;

-- Verify the users were created
SELECT
    u.id,
    u.name,
    u.email,
    r.name as role_name,
    s.name as school_name,
    u.is_active
FROM users u
JOIN roles r ON u.role_id = r.id
JOIN schools s ON u.school_id = s.id
WHERE u.id IN (
    SELECT a.id
    FROM auth.users a
    LEFT JOIN users u ON a.id = u.id
    WHERE u.id IS NULL
    LIMIT 10
);

-- Final status check
SELECT
    'Final Status' as status,
    COUNT(DISTINCT u.id) as local_users,
    COUNT(DISTINCT a.id) as auth_users,
    COUNT(DISTINCT CASE WHEN u.id IS NOT NULL THEN a.id END) as synced_users,
    COUNT(DISTINCT CASE WHEN u.id IS NULL THEN a.id END) as unprovisioned_users
FROM auth.users a
LEFT JOIN users u ON a.id = u.id;