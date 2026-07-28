# Full-validation control

The `full-validation` label marks a pull request as being in final review.

While the label is present, every new exact head must complete:

- `.github/workflows/full-validation.yml` with complete database, Rust, and gate
  jobs successful;
- `.github/workflows/package.yml` with package definitions, image/archive build,
  and repeated packaged migrations successful;
- `.github/workflows/production-foundation.yml` with topology, pinned Supabase
  PostgreSQL migrations/role verification, and complete production-stack smoke
  successful;
- `.github/workflows/air-gapped-appliance.yml` with definition validation, the
  complete image/model/SBOM/signature appliance build, first startup with pulls
  disabled, both custom-image architecture builds, and its final gate successful;
- `.github/workflows/mirror-final-proof.yml` in the public validation mirror,
  proving the separately dispatched Package, Production Foundation, and
  Air-gapped Appliance runs all used the same exact head SHA.

GHCR publication is intentionally skipped on pull requests. It becomes mandatory
only for a protected `v*` release tag or an explicitly approved workflow dispatch
from `main`. The publication must produce versioned and commit-addressed amd64/arm64
custom images, SBOM and provenance attestations, digest signatures, and exact
publication evidence. No `latest` tag is permitted.

Create the label once in the repository with the exact name `full-validation`.
Keep it on the pull request until the exact merge head is green. Remove it only
when returning the pull request to ordinary iterative work.

Do not count an older SHA, canceled workflow, or skipped required job as passing.
Inspect diagnostic and release evidence artifacts before merge. Merge only with an
expected-head guard after all required jobs and review threads are resolved.
