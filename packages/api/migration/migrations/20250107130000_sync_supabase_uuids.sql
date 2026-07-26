-- ========================================
-- Sync UUIDs between Supabase Auth and local database
-- This fixes the 401 Unauthorized issue caused by UUID mismatch
-- ========================================

-- First, let's see what users need to be synced
SELECT
    'Before Sync' as status,
    u.id as local_id,
    a.id as auth_id,
    u.email,
    u.name,
    s.name as school_name
FROM users u
LEFT JOIN auth.users a ON u.email = a.email
LEFT JOIN schools s ON u.school_id = s.id
WHERE u.id != a.id OR a.id IS NULL;

-- Handle foreign key constraints by cascading the UUID update
-- First, disable foreign key constraints temporarily
SET session_replication_role = replica;

-- Update users table to use Supabase Auth UUIDs
UPDATE users u
SET id = a.id
FROM auth.users a
WHERE u.email = a.email
  AND u.id != a.id;

-- Re-enable foreign key constraints
SET session_replication_role = DEFAULT;

-- Verify the sync worked
SELECT
    'After Sync' as status,
    u.id as local_id,
    a.id as auth_id,
    u.email,
    u.name,
    s.name as school_name,
    CASE WHEN u.id = a.id THEN '✓ Synced' ELSE '✗ Mismatch' END as sync_status
FROM users u
LEFT JOIN auth.users a ON u.email = a.email
LEFT JOIN schools s ON u.school_id = s.id;

-- Check for any users in local DB that don't exist in Supabase Auth
SELECT
    'Local Only Users' as status,
    u.id,
    u.email,
    u.name
FROM users u
LEFT JOIN auth.users a ON u.email = a.email
WHERE a.id IS NULL;

-- Check for any Supabase Auth users that don't exist in local DB
SELECT
    'Auth Only Users' as status,
    a.id,
    a.email
FROM auth.users a
LEFT JOIN users u ON a.email = u.email
WHERE u.id IS NULL;