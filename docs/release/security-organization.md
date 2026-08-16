# Security organization and incident-response package

This package aligns engineering process with NIST SSDF-style secure-development practices and ISO/IEC 27001-style governance concepts. It is a control mapping/process definition, **not a certification claim**.

## Vulnerability disclosure

Until a production security mailbox/domain is approved, do not publish a personal address from this repository. The executed customer/support package must provide a monitored security contact route and backup escalation path. Incoming reports are acknowledged, triaged, access-limited, tracked to remediation/retest, and protected from unnecessary disclosure.

## Supported-version policy

Production support applies only to release versions explicitly listed in the signed service schedule. Security fixes are developed on supported branches/releases and customers are given an upgrade/mitigation path. End-of-support dates require advance commercial communication and cannot silently leave a contracted production release unmaintained.

## Severity and remediation SLA schedule

Severity is defined now; contractual time targets are explicit approval fields rather than hidden assumptions. Complete the final SLA in PR #16 / the signed support schedule. Until then, the incident process still starts immediately for suspected Critical impact.

| Severity | Example | Required handling | Contract remediation target |
| --- | --- | --- | --- |
| Critical | active compromise, cross-tenant data exposure, remote privilege boundary failure | declare incident, contain/preserve evidence, executive/security escalation, emergency mitigation/release path | **TO APPROVE:** ___ hours/days |
| High | exploitable auth/authorization/security-control failure with material impact | prioritized security owner, mitigation/release plan, retest | **TO APPROVE:** ___ days |
| Medium | bounded weakness requiring conditions | tracked remediation with owner/review date and regression evidence | **TO APPROVE:** ___ days |
| Low | hardening/defense-in-depth | backlog/risk-register disposition with owner/expiry | **TO APPROVE:** ___ days or supported release |

The executed agreement must also define when the clock starts/stops, customer-dependent pauses, accepted compensating mitigations, retest requirements and treatment of unsupported releases. No blank target is a customer promise.

## Incident roles and escalation

- Incident Lead: coordinates scope, timeline, decisions and recovery.
- Security Lead: containment, evidence, vulnerability/credential response.
- Operations Lead: host/service/backup/recovery actions.
- Product/Data owner: tenant/data-impact analysis.
- Customer/Privacy/Legal liaison: approved external notifications.

Critical incidents use the documented customer/security contact path immediately; preserve UTC timeline/evidence and avoid destructive cleanup before capture unless required to stop active harm.

## Response lifecycle

Detect -> triage/severity -> contain -> preserve evidence -> investigate scope/root cause -> eradicate/rotate -> recover from verified state -> validate authorization/data integrity -> notify under approved contract/privacy procedure -> retrospective/actions -> evidence retention.

## Security update signing and release

Protected releases use the repository's signed/attested release processes. Air-gapped artifacts include immutable manifest/checksums, SBOMs, signatures/provenance and verification policy. Do not publish `latest` as production identity. Security releases must identify exact source SHA/artifact digest and pass applicable exact-head gates before distribution.

## Supplier/dependency review

Review pinned production dependencies/providers for support state, security advisories, license/terms impact and processing role before adoption/upgrades. High/critical dependency/configuration scanning is CI evidence, not a substitute for provider/legal review. AI provider terms and data handling are re-verified before contracting when connected mode is selected.

## Access review and operator onboarding/offboarding

Use named least-privilege operators, dedicated production accounts, restricted admin networks, separate bootstrap/migration authority and secret escrow. Review privileged access on onboarding, role change and offboarding; remove stale operator credentials and rotate shared/affected secrets. Routine timers use the dedicated unprivileged operator identity, not personal admin accounts.

## Evidence retention

Retain release manifests/signatures/provenance, relevant CI gate artifacts, host-qualification evidence, backup/recovery evidence, incident timelines, security findings/retests, access reviews and accepted risks according to the signed retention schedule. Evidence repositories must not become an uncontrolled store of passwords, provider keys or unnecessary personal data.

## Risk register

Every residual production risk records: description, affected release/school, severity/likelihood, treatment/compensating control, owner, acceptance authority, review/expiry date and linked issue/evidence. Expired risk acceptance blocks release/continued acceptance until reviewed.

## Penetration-test remediation

Independent penetration testing and human authorization review are tracked in PR #16. Each finding receives severity, owner and remediation reference. Remediated material findings require retest. Any residual finding requires explicit owner/date/rationale before final production classification.
