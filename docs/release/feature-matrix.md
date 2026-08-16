# Product feature matrix

Status vocabulary:

- **Implemented** — present and supported in the contracted application scope, subject to role/tenant authorization.
- **Optional** — implemented but requires an explicitly selected deployment/profile or external dependency.
- **Disabled** — deliberately unavailable and must not be promised or exposed as implemented.
- **Excluded** — outside the first contracted scope even if lower-level primitives may exist.

Machine sources: `product-capabilities.json` and `../../packages/api/endpoint_authorization_manifest.psv`.

## Implemented product scope

| Capability | Status | Primary roles | Contract note |
| --- | --- | --- | --- |
| Password authentication, session checks and logout | Implemented | All authenticated roles | Public signup is disabled; accounts are administratively provisioned. |
| Platform school administration | Implemented | PlatformAdmin | School create/list/read and governed platform knowledge administration are authorization-scoped. |
| School/class administration | Implemented | SchoolManager | School-scoped class, enrollment, profile-request and supported user-management views/actions only. |
| Teacher class/roster views | Implemented | Teacher | Teacher-owned/assigned classes and students only. |
| Assignments | Implemented | Teacher, Student | Teacher creation/publish/update/delete; student authorized assignment views. |
| Student assignment submission | Implemented | Student | Student can submit only to an authorized assigned object. |
| Teacher submission review, grading and feedback | Implemented | Teacher | Teacher-owned assignment/submission scope only. |
| Student grades | Implemented | Student | Student sees own authorized class grades. |
| Parent child overview, assignments and grades | Implemented | Parent | Restricted to linked child records. |
| Notifications and user preferences | Implemented | Authenticated user | Session-owner scoped. |
| Profile-change request workflow | Implemented | Authenticated user, SchoolManager | User request + school-manager decision. |
| Governed knowledge assets | Implemented | PlatformAdmin, SchoolManager, Teacher | Publish/selection/search lifecycle remains authorization-scoped. |
| Teacher class materials and vectorization status | Implemented | Teacher | Teacher/class scope; durable vectorization lifecycle. |
| Assignment personalization | Optional | Teacher, Student | Requires an approved AI profile; durable jobs degrade safely if AI is unavailable. |
| Connected external AI | Optional | Authorized AI-backed flows | Only through AI Gateway to fixed approved provider origins/models with school identity. |
| Local/offline AI | Optional | Authorized AI-backed flows | Local embedding profile/appliance artifacts; no external-provider dependency for the local profile. |
| Air-gapped appliance deployment | Optional | Operator | Complete no-pull appliance, signed/verified per release process. |

## Deliberately disabled capabilities

The following keys are required to remain **Disabled** while `product-capabilities.json` marks them unavailable:

| Capability key | Product label | Status | Reason for contract scope |
| --- | --- | --- | --- |
| `attendance` | Attendance workflow | Disabled | No production attendance workflow is contracted. |
| `timetable` | Timetable/schedule management | Disabled | No production timetable management is contracted. |
| `grade_trends` | Grade trend analytics | Disabled | Derived trend analytics are not contracted. |
| `parent_reports` | Parent report generation | Disabled | Parent receives supported child views, not report generation. |
| `parent_teacher_communication` | Parent/teacher messaging | Disabled | Messaging is not a contracted product capability. |
| `school_manager_reports` | School-manager reports | Disabled | Report/dashboard placeholders are not contracted reporting. |
| `derived_academic_metrics` | Derived academic metrics | Disabled | No inferred academic KPI/analytics claim. |
| `synthetic_system_health` | Synthetic in-product system-health data | Disabled | Operational health belongs to operator monitoring, not synthetic product UI. |

## Explicit exclusions for first contract

- high availability/multi-node failover;
- public self-service signup;
- arbitrary provider/model selection by schools or browser clients;
- direct browser access to database, Qdrant, AI provider credentials or internal gateway credentials;
- automatic vector-space fallback between embedding models/dimensions;
- unsupported endpoint families marked `Disabled` in the endpoint authorization manifest;
- legal/compliance certification claims not backed by independent qualification.

Any commercial schedule must copy this matrix or reference an exact revision and must not convert a Disabled/Excluded item to Implemented without a subsequent engineering PR and exact-head acceptance evidence.
