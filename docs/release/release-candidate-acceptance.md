# Final release-candidate acceptance record

Plan item: **PR-15 — P0 final exact-head production acceptance**.

## Frozen candidate basis

- Preceding engineering baseline on `main`: `88c4131bc7e9e818d8b940de2afa419e339e39cc` (merged PR-14).
- Candidate branch: `agent/pr-15-final-release-acceptance`.
- Exact candidate head SHA: resolved from the pull-request event and written by the Final Release Acceptance workflow; do not substitute an older run or merge-preview SHA.
- Candidate scope: final release evidence/orchestration only; no product feature is being added by PR-15.

Any commit after a final proof starts invalidates that proof for release-candidate purposes. The workflow concurrency cancels superseded proof and every reused/dispatched run is required to report the exact current PR head SHA.

## Entry criteria

The automated release gate verifies at runtime that:

- the checked-out commit is the exact PR head;
- there is no open issue carrying a P0/release-blocker label in the repository issue set inspected by the gate;
- this PR has no unresolved review thread;
- the release documentation truth gate passes on the exact head;
- the manual/external acceptance dependency remains explicitly represented by PR #16 rather than being inferred from CI.

The merged PR-14 feature matrix is the contracted-scope source. Disabled/excluded product domains remain absent from the promised release scope unless a later engineering/release revision explicitly changes them.

## Exact-head automated sequence

`Final Release Acceptance` serializes the expensive stages so the single runner/cache is used efficiently and stale/skipped evidence is rejected:

1. AI Change Proof for the exact PR head.
2. Release Documentation truth/secret/PII/drift validation.
3. Complete Full Validation workflow dispatch:
   - database migrations/security;
   - full Rust compile/lint/tests;
   - final desktop/mobile browser/offline/accessibility automation;
   - full validation gate.
4. Complete Production Foundation workflow dispatch:
   - rendered topology/security invariants;
   - pinned Supabase PostgreSQL 17 migrations and constrained `NOBYPASSRLS` role;
   - complete production stack and runtime boundary proof.
5. Complete Production Operations workflow dispatch:
   - definitions/regressions and high/critical scans;
   - PostgreSQL PITR, Qdrant recovery and migration rollback;
   - encrypted backup/restore, alerts, sustained load/restart and fail-closed acceptance;
   - aggregate operations gate.
6. Complete Package workflow dispatch with actual image/release bundle proof.
7. Complete Air-gapped Appliance reusable proof:
   - exact image inventory/digests;
   - SBOM inventory;
   - signed immutable manifest/checksums;
   - manifest-owned `provenance/release-builder.json` tied to the exact Git SHA/platform;
   - packaged model revision/checksums;
   - registry-disabled/no-pull amd64 first startup;
   - native arm64 custom-image build;
   - aggregate appliance gate.
8. Final Release Acceptance aggregate artifact containing every exact-head run ID and the automated disposition.

For workflow-dispatch stages, an already successful complete run may be reused only when its `head_sha`, event, overall result, and every required job match the frozen candidate. A focused/skipped/failed/stale run is not accepted.

## Manual/external evidence intentionally separated

Per the repository owner's explicit sequencing instruction, all human/external production verifications are consolidated in **PR #16 — Manual/external production acceptance evidence**. They remain required before production/legal acceptance and include:

- independent authorization/security review;
- clean target-host replacement install/restore/PITR/Qdrant qualification;
- penetration test and remediation/retest disposition;
- keyboard and screen-reader acceptance;
- school-scale load/soak plus measured deployment-specific RPO/RTO;
- privacy/legal/contract approval;
- operator acceptance and incident rehearsal;
- residual-risk ownership/sign-off.

Moving those checks to PR #16 means **deferred, not passed**.

## Release decision boundary

The maximum classification that automated PR-15 evidence can produce while qualified human/external acceptance remains outside this PR is:

**ready for final validation**

The automated workflow must explicitly record `ready_for_contracted_production: false`. Only the completed human/external acceptance process may support a later `ready for limited pilot` or `ready for contracted production` decision as appropriate. Green CI alone is never sufficient for the contracted-production classification.
