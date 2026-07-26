-- Adding an enum value does not create the matching reference-data row used by
-- users.role_id. Provision a narrowly-scoped platform administration role.
INSERT INTO roles (name, permissions)
VALUES (
    'PlatformAdmin',
    jsonb_build_object(
        'review_knowledge_assets', true,
        'embed_knowledge_assets', true,
        'publish_knowledge_assets', true,
        'archive_knowledge_assets', true,
        'view_knowledge_audit', true
    )
)
ON CONFLICT (name) DO UPDATE
SET permissions = roles.permissions || EXCLUDED.permissions;
