-- PostgreSQL requires a newly-added enum value to be committed before it is
-- referenced by later migrations, functions, policies, or prepared statements.
ALTER TYPE role_name ADD VALUE IF NOT EXISTS 'PlatformAdmin';
