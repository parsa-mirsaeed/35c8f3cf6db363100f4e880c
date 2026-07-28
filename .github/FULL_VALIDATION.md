# Full-validation control

The `full-validation` label marks a pull request as being in final review.

## Iterative development

Keep a pull request in draft and remove `full-validation` while implementation is
still changing. Every commit still runs `AI change gate`, package-definition
validation, production-topology validation, and appliance-definition validation
when their paths are affected. The complete image, database, production-stack,
and appliance builds are deliberately not repeated for every draft commit.

## Final exact-head review

When the implementation is stable:

1. apply `full-validation`;
2. mark the pull request ready for review;
3. do not push another commit unless the pull request is returned to draft first;
4. inspect every exact-head result and artifact before merge.

While the PR is ready and the label is present, the final candidate must complete:

- `.github/workflows/full-validation.yml` with complete database, Rust, and gate
  jobs successful;
- `.github/workflows/package.yml` with package definitions, image/archive build,
  and repeated packaged migrations successful;
- `.github/workflows/production-foundation.yml` with topology, pinned Supabase
  PostgreSQL migrations/role verification, and complete production-stack smoke
  successful;
- `.github/workflows/mirror-final-proof.yml` in the public validation mirror,
  which waits for those exact-head PR runs and only then dispatches the complete
  Air-gapped Appliance proof;
- `.github/workflows/air-gapped-appliance.yml` with definition validation, the
  complete amd64 image/model/SBOM/signature appliance build, first startup with
  pulls disabled, a native arm64 custom-image build, and its final gate successful.

This ordering prevents Package, Production Foundation, the amd64 appliance build,
and architecture proof from compiling the same Rust runtime concurrently. The
final exact-head appliance proof remains mandatory; it is merely deferred until
the cheaper gates have succeeded.

GHCR publication is intentionally skipped on pull requests. It becomes mandatory
only for a protected `v*` release tag or an explicitly approved workflow dispatch
from `main`. Publication uses native amd64 and arm64 runners, creates immutable
multi-architecture indexes, signs their digests, attaches SBOM/provenance
attestations, and writes exact publication evidence. No `latest` tag is permitted.

Keep `full-validation` on the ready pull request until the exact merge head is
green. If a fix is required, convert the PR back to draft and remove the label
before committing. Reapply the label and mark ready only when the next final
candidate is stable.

Do not count an older SHA, canceled workflow, or skipped required job as passing.
Inspect diagnostic and release evidence artifacts before merge. Merge only with an
expected-head guard after all required jobs and review threads are resolved.