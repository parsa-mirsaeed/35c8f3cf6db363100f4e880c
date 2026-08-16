# Support and service definition — commercial input

This document defines fields that must be completed in the signed customer schedule; it does not invent business commitments.

## Service classification

- **Pilot:** controlled school trial with explicit participant/scope limits, enhanced feedback/observation, no implied HA, and acceptance criteria defined in the pilot order.
- **Contracted production:** only after final release/manual-external gates are complete and the signed schedule identifies release, host, features, responsibilities, support and accepted risk.

## Signable service schedule fields

Complete every field before classifying the customer as contracted production; a blank is an unresolved business/operational decision, not a default promise.

| Field | Approved value |
| --- | --- |
| Support timezone | ___ |
| Standard support days/hours | ___ |
| Critical security/availability contact route and coverage | ___ |
| Severity acknowledgement/response targets | ___ |
| Regular maintenance window | ___ |
| Planned-maintenance notice | ___ |
| Emergency-maintenance authority/notice | ___ |
| Availability target (if purchased) | ___ |
| Availability measurement point/reporting period | ___ |
| Accepted RPO | ___ |
| Accepted RTO + recovery scenario | ___ |
| Backup/WAL retention and operator | ___ |
| Customer upstream/WAF/DoS responsibility | ___ |
| Supported release/version window | ___ |

## Support scope

Included technical scope should be selected from: supported release installation/configuration, application defects, security updates, production operations/recovery guidance, supported AI profile behavior, and contracted feature support. Customer-specific integrations or Disabled/Excluded features are not included unless separately engineered/ordered.

## Support hours and escalation

The signed schedule must state timezone, business hours, emergency security/availability route, severity definitions, acknowledgement/response targets, named customer contacts and vendor escalation contacts. Until those fields are executed, this repository makes no 24x7 support promise.

## Maintenance windows

Define regular maintenance window, advance notice, emergency maintenance authority and customer blackout periods. The first architecture is single-node; host/runtime/app maintenance may interrupt service.

## Availability measurement

If an availability commitment is purchased, define numerator/denominator, measurement point, excluded planned maintenance, customer/network/provider exclusions, incident attribution and reporting period. Do not equate `/healthz`, CI uptime or AI-provider availability with a contractual availability percentage.

## Backup / RPO / RTO

The accepted target-host record provides measured school-specific backup/recovery results. The signed schedule must state:

- backup/WAL frequency and retention responsibility;
- off-appliance destination/escrow responsibility;
- accepted RPO;
- accepted RTO and what recovery event it covers;
- customer dependencies and exclusions.

No numeric RPO/RTO is promised by this draft.

## Rate limiting, abuse and denial-of-service boundary

The product uses bounded payload/resource classes, restricted ingress/admin networks, service resource limits and operational monitoring. Internet-facing DoS protection beyond the single-host/gateway boundary depends on the customer's network/upstream environment. Before contracted exposure, target-host acceptance must document upstream/firewall controls, capacity/headroom and incident escalation. If customer scale/risk requires dedicated WAF/rate-limiting infrastructure not present in the accepted topology, that is a prerequisite/change rather than an undocumented promise.

## Customer responsibilities

Customer responsibilities normally include supported hardware/virtualization and network/DNS, approved TLS/domain control, physical/storage security, lawful user provisioning/notices, authorized admin contacts, upstream firewall/connectivity, backup destination/escrow as allocated, and timely upgrade/incident cooperation.

## AI provider responsibility/outage

Connected AI depends on the selected approved external provider and network path. Core service is designed to remain healthy when AI is unavailable; AI-backed functions may be temporarily unavailable/queued. Provider legal/data terms and any provider-specific service commitments are not silently inherited as EduTalent commitments.
