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
- the pull-request validation jobs in Package, Production Foundation, and
  Air-gapped Appliance with their expensive jobs intentionally skipped;
- `.github/workflows/mirror-final-proof.yml` in the public validation mirror,
  which verifies the exact-head AI and Full Validation gates and then dispatches
  the complete workflows in this strict order:
  1. Production Foundation migrations, role verification, and full-stack smoke;
  2. Package image/archive build and repeated packaged migrations;
  3. Air-gapped Appliance amd64 bundle/offline startup and native arm64 build;
- `.github/workflows/air-gapped-appliance.yml` with definition validation, the
  complete amd64 image/model/SBOM/signature appliance build, first startup with
  pulls disabled, a native arm64 custom-image build, and its final gate successful.

The sequential dispatch prevents complete Production Foundation, Package, and
appliance builds from competing or duplicating heavy work at the same time. Package
runs immediately before the appliance so both use the shared
`edutalent-runtime` BuildKit cache. The final exact-head appliance proof remains
mandatory; only its timing is deferred until the cheaper gates have succeeded.

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