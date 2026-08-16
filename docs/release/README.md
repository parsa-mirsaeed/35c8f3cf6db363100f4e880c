# EduTalent production-readiness release package

Documentation package revision: **1.0 / PR-14**.

This directory is the review pack for a school pilot or contract discussion. It describes only repository-supported behavior and separates implemented product scope from optional modes, disabled features, operational responsibilities, and draft legal/privacy terms.

## Source-of-truth hierarchy

When documents disagree, stop the release review and reconcile them. Use this order for technical truth:

1. `product-capabilities.json` for deliberately disabled product capabilities;
2. `packages/api/endpoint_authorization_manifest.psv` for route/server-function authorization and disabled endpoint inventory;
3. production Compose/configuration and `deploy/production/` validation tooling for deployment behavior;
4. ADRs and threat models under `docs/adr/` and `docs/security/`;
5. this release package and the root README.

No document in this package expands product authority beyond the code/manifests above.

## Package index

- [Feature matrix](feature-matrix.md)
- [Operator manual v1.0](operator-manual-v1.0.md)
- [Administrator guide](guide-administrators.md)
- [Teacher guide](guide-teacher.md)
- [Parent guide](guide-parent.md)
- [Student guide](guide-student.md)
- [API and security inventory](api-security-inventory.md)
- [Privacy and governance draft](privacy-governance-draft.md)
- [Security organization and response](security-organization.md)
- [Procurement/security questionnaire](procurement-security-questionnaire.md)
- [Support/service definition](support-service-definition.md)
- [Contract feature/exclusion schedule](contract-feature-schedule.md)
- [Customer terms inputs — draft](customer-terms-draft.md)
- [Documentation reconciliation](documentation-reconciliation.md)

## Review status

Repository CI verifies internal consistency, relative links, disabled-feature truthfulness, endpoint evidence references, and common secret/PII patterns. CI does **not** provide legal advice, privacy approval, an independent penetration test, contractual acceptance, target-host qualification, or human accessibility acceptance.

All human/external approvals are consolidated in PR #16 and `docs/security/manual-external-production-acceptance.md`. This package must not be signed as customer terms until those qualified reviews are complete against the frozen release candidate.

## Product/architecture position

The first production architecture is self-hosted, single-node and **not highly available**. Only the gateway publishes host ports 80/443. PostgreSQL/Supabase internals, Qdrant and internal AI services remain private. The application uses a constrained `NOBYPASSRLS` database role with transaction-scoped authorization context. External AI is optional and mediated only through the AI Gateway; fully offline/local AI operation is separately supported by the appliance profile. Provider or AI-Gateway unavailability is not a core-login health dependency.

Availability, RPO and RTO are not inferred from CI thresholds. They become contractual only when measured on the accepted school host/workload and written into the signed commercial schedule.
