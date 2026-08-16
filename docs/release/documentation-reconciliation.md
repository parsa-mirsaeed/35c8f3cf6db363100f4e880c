# Production documentation reconciliation record

PR-14 treats ADRs as historical decision records and avoids rewriting their original context merely because later decisions completed their follow-up work. Current operational/product claims, however, must not contradict the implemented architecture.

## Reconciled current documents

- Root `README.md`: retired statement that the application role intentionally used `BYPASSRLS`/issue #8 has been removed. Current text states constrained `NOBYPASSRLS` + transaction-scoped PostgreSQL authorization.
- `docs/security/production-threat-model.md`: updated from the old “future AI Gateway / intentional BYPASSRLS” state to the current AI Gateway, transaction-scoped RLS, signed appliance and operations controls.
- `docs/adr/0005-transaction-scoped-rls.md`: decision status updated from proposed to accepted/implemented.

## Historical ADR context retained intentionally

- ADR 0001 predates the controlled-AI implementation; its “follow-up AI gateway” language is historical and is resolved by ADR 0002.
- ADR 0002 predates the full appliance/operations implementation; later-work statements are historical and are resolved by ADR 0003 plus the production operations/host qualification implementation.
- ADR 0003 records that backup/rollback operations were deferred at the time of the appliance decision; those controls now exist under `deploy/production/operations/` and PR-13 host/maintenance controls.

These historical statements must not be copied into current customer/operator claims as if still unresolved. The current release package, production runbooks, accepted ADR 0005 and current threat models govern present-tense review.

## Current invariant references

- controlled AI: ADR 0002 + controlled external AI threat model;
- full offline appliance: ADR 0003 + air-gapped release threat model;
- transaction-scoped database authorization: ADR 0005;
- operations/recovery: ADR 0004 + production operations threat model;
- host qualification/maintenance: `deploy/production/HOST_BASELINE.md` and operations acceptance/runbooks;
- exact commercial product truth: `feature-matrix.md`, `product-capabilities.json` and the endpoint authorization manifest.
