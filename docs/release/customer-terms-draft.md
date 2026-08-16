# Customer terms inputs — draft for counsel/business approval

**This is not legal advice and is not an executable contract.** Counsel/business approval is required in PR #16.

The final agreement/order should address at least the following without contradicting the exact release/feature schedule.

## Proprietary deployment/use grant

Define a limited, non-exclusive, non-transferable (except as expressly agreed) right for the named customer to install/use the identified proprietary EduTalent release for the contracted school/users/environment. Define copying, modification, reverse-engineering, redistribution, third-party access, affiliate/use-by-contractor and backup-copy rules as counsel approves. Nothing in this draft grants ownership of the software/source/IP.

## Contracted features and exclusions

Incorporate `contract-feature-schedule.md` by exact revision. Disabled/Excluded features are not commitments. Optional AI/appliance modes require explicit selection.

## Support and escalation

State support hours/timezone, contact methods, severity, acknowledgement/response targets, emergency security route, supported versions and customer cooperation. Avoid implying 24x7 support unless purchased/operationally staffed.

## Maintenance and updates

Define planned/emergency maintenance, notice, supported upgrade cadence, security-update obligations, customer approval/cooperation and consequences of remaining on unsupported releases. Single-node maintenance can cause downtime.

## Availability

Define measurement source/window, planned maintenance/exclusions, customer/upstream/provider exclusions and remedies only after business approval. Do not describe the first topology as HA.

## Backup, RPO and RTO responsibilities

Identify who operates/monitors backup/WAL, supplies off-appliance destination and escrow, retains/restores data, and approves drills. Insert only RPO/RTO values measured/accepted for the target deployment; distinguish objectives/commitments from observed test results.

## Customer hardware/network responsibilities

Specify supported host/hypervisor/storage, capacity, encryption, DNS, TLS/domain control, firewall/upstream protection, time sync, physical/security controls and connectivity/provider dependencies.

## Security incidents

Define security contacts, cooperation, containment/evidence duties, customer access, notification timing/contents subject to applicable law/DPA, remediation and post-incident obligations. Do not use a universal statutory deadline unless counsel confirms it.

## Privacy/data processing

Execute appropriate DPA/privacy terms based on `privacy-governance-draft.md`, actual controller/processor roles, data inventory, provider/subprocessor list, locations/transfers, retention, DSR, incident and deletion/return obligations.

## AI provider responsibility and outage behavior

Identify selected AI mode/provider. Clarify that connected AI relies on an external provider/network and that AI-backed functions can degrade while core login/school operations remain available. Allocate provider-term changes, processing approval, outage/support and prohibited-content responsibilities. Do not promise provider training/retention behavior without current verified provider terms.

## Offboarding / export / deletion

Define export format/timing/scope, access revocation, return/deletion, backup expiry/holds, evidence of completion and customer cooperation. Avoid promising immediate per-row deletion from retained immutable recovery generations unless technically/contractually supported.

## Acceptance procedure

Define installation/qualification evidence, acceptance tests/timeframe, defect process and deemed/formal acceptance rules. The release gate must refer to exact artifact/SHA and applicable PR #16 manual/external evidence.

## Warranty / limitation inputs

Counsel must define warranties, disclaimers, exclusions, liability caps/carve-outs, indemnities, IP/security/privacy allocation and governing law. Engineering documentation must not invent those positions.
