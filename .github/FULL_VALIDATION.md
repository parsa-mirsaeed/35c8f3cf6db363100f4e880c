# Full-validation control

The `full-validation` label marks a pull request as being in final review.

## Iterative development

Keep a pull request in draft and remove `full-validation` while implementation is
still changing. Every commit still runs `AI change gate`, package-definition
validation, production-topology validation, appliance-definition validation, and
production-operations definition/security scanning when their paths are affected.
The complete image, database, production-stack, appliance, recovery, backup, and
load builds are deliberately not repeated for every draft commit.

## Final exact-head review

When the implementation is stable:

1. apply `full-validation`;
2. mark the pull request ready for review;
3. do not push another commit unless the pull request is returned to draft first;
4. inspect every exact-head result and artifact before merge.

While the PR is ready and the label is present, the final candidate must complete:

- `.github/workflows/full-validation.yml` with complete database, Rust, and gate
  jobs successful;
- the pull-request validation jobs in Package, Production Foundation, Air-gapped
  Appliance, and Production Operations with their expensive jobs successful when
  selected or intentionally skipped while focused validation is selected;
- `.github/workflows/production-operations.yml` with definition regressions and
  high/critical security scans successful, plus PostgreSQL PITR, Qdrant recovery,
  failed-migration rollback, encrypted full backup/restore, configuration-failure,
  local alert, sustained-load, database-restart, and controlled-app-recreation
  evidence successful on the exact final head;
- `.github/workflows/mirror-final-proof.yml` in the public validation mirror,
  which verifies the exact-head AI and Full Validation gates and then enforces
  this strict order:
  1. reuse an exact-head complete Production Foundation pull-request run when all
     three required jobs succeeded; otherwise dispatch exactly one complete
     Production Foundation fallback when the PR run is focused-only or absent;
  2. dispatch Package image/archive build and repeated packaged migrations;
  3. dispatch Air-gapped Appliance amd64 bundle/offline startup and native arm64
     build;
- `.github/workflows/air-gapped-appliance.yml` with definition validation, the
  complete amd64 image/model/SBOM/signature appliance build, first startup with
  pulls disabled, a native arm64 custom-image build, and its final gate successful.

A failed Production Foundation or Production Operations required job is never
hidden by fallback. Mirror uses fallback only for an absent exact-head Production
Foundation PR run or the expected focused result where topology succeeded and the
two complete jobs were skipped. Fresh dispatch IDs prevent older canceled runs on
the same SHA from being reused.

The sequential mirror proof prevents complete Production Foundation, Package, and
appliance builds from competing or duplicating heavy work. Package runs immediately
before the appliance so both use the shared `edutalent-runtime` BuildKit cache.
The final exact-head appliance and operations recovery proof remain mandatory;
only their timing is separated.

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
Inspect diagnostic, scan, backup, recovery, load, and release evidence artifacts
before merge. Merge only with an expected-head guard after all required jobs and
review threads are resolved.
