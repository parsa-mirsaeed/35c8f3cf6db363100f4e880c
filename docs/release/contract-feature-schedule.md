# Contract feature and exclusion schedule — template

**Draft commercial schedule. Requires business/legal approval in PR #16.**

## Release identity

- Release/tag:
- Exact commit SHA:
- Artifact/image/appliance digest(s):
- Deployment mode: connected AI / degraded-capable connected / fully offline-local
- Accepted target host record:

## Included features

The default included feature scope is the rows marked **Implemented** in `feature-matrix.md`, plus only the **Optional** rows explicitly selected below.

Selected optional capabilities:

- [ ] assignment personalization / approved AI profile
- [ ] connected external AI profile
- [ ] local/offline AI profile
- [ ] air-gapped appliance deployment

Other agreed included features/constraints:

- _None recorded until completed._

## Excluded/disabled features

Unless a later signed engineering/release revision states otherwise, the contract excludes attendance, timetable, grade trends, parent reports, parent/teacher messaging, school-manager reports, derived academic metrics and synthetic in-product health, plus endpoint families marked `Disabled` in the authorization manifest.

## Operational responsibility schedule

Complete responsibility owner for:

- supported host/hypervisor:
- DNS/TLS/network/firewall:
- production operator account/access:
- off-appliance backup storage:
- backup-passphrase escrow:
- backup monitoring/restore drills:
- patch/upgrade approval:
- incident contacts:
- lawful user provisioning/notices:
- AI profile/provider approval:

## Service/support

Reference the completed `support-service-definition.md` schedule for support hours/escalation, maintenance, availability measurement, RPO/RTO and customer dependencies.

## Acceptance

Acceptance requires the exact release automated gates plus the applicable manual/external evidence in PR #16. Any unresolved material finding/risk must identify owner, rationale and review/expiry date.
