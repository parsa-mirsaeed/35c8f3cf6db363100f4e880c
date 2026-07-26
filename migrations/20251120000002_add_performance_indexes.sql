-- Performance optimization indexes for auth queries
-- Run this to speed up user/role lookups

-- Index on users.id (primary lookups)
CREATE INDEX IF NOT EXISTS idx_users_id ON public.users(id);

-- Index on users.email (login lookups)
CREATE INDEX IF NOT EXISTS idx_users_email ON public.users(email);

-- Index on roles.id (join optimization)
CREATE INDEX IF NOT EXISTS idx_roles_id ON public.roles(id);

-- Index on users.role_id (join optimization)
CREATE INDEX IF NOT EXISTS idx_users_role_id ON public.users(role_id);

-- Composite index for the most common query pattern
CREATE INDEX IF NOT EXISTS idx_users_role_lookup ON public.users(id, role_id) WHERE is_active = true;

-- Analyze tables to update query planner statistics
ANALYZE public.users;
ANALYZE public.roles;
