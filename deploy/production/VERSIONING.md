# Versioning policy

- `SUPABASE_UPSTREAM` is an immutable Git commit, never a branch or tag.
- The official Supabase image versions are consumed as one tested coordinated
  set from that commit.
- EduTalent, Qdrant, Caddy, and optional TEI images use explicit version tags in
  this foundation; the air-gapped delivery phase will additionally pin and
  verify immutable OCI digests.
- Changing an embedding model or dimensions requires a new versioned vector
  collection and a controlled full re-index. Existing vectors are never mixed
  across embedding spaces.
- Production database migration files are append-only and remain protected by
  the repository migration checksum registry.
