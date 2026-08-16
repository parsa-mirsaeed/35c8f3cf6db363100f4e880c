# Privacy and governance package — draft for qualified review

**Draft only. Not legal advice and not approved customer terms.** Qualified privacy/legal review and final approval are tracked in PR #16 against the frozen release candidate and actual school deployment/provider choices.

## Role allocation working model

The intended commercial model is that the school/customer determines educational purposes, account population, lawful basis/notices, retention instructions and authorized users; EduTalent's contracting entity operates/provides the software and support according to the signed deployment model. Counsel must confirm controller/processor roles for the jurisdiction and whether any activity creates an independent-controller or joint-controller role.

## Personal-data / processing-activity inventory

| Data/activity | Typical subjects | Purpose | Location/path | Notes |
| --- | --- | --- | --- | --- |
| Account identity/profile/role/school linkage | staff, students, parents | authentication and authorization | local Supabase/PostgreSQL | Public signup disabled. |
| Class/enrollment/assignment records | students, teachers | education workflow | local PostgreSQL | Tenant/object scoped. |
| Student submissions | students | assignment completion | local PostgreSQL/storage path used by product | Access restricted to authorized student/teacher context. |
| Grades and feedback | students, teachers, parents where linked | assessment/feedback | local PostgreSQL | Parent only linked child; no grade-trend product. |
| Notifications/preferences/profile-change requests | authenticated users | service operation/preferences | local PostgreSQL | Session-owner/school-manager scoped. |
| Governed/class knowledge material and derived embeddings | staff/school context | retrieval/personalization | local data + private Qdrant | Publication/selection authorization precedes retrieval. |
| Operational logs/alerts/backup metadata | users may be indirectly referenced | security/availability/recovery | target host/off-appliance backup destination | Minimize identifiers and define evidence retention. |
| Optional AI request content | authorized school context | approved embedding/LLM purpose | AI Gateway then selected local/external provider | External processing only when connected profile is approved. |

The final data inventory must be reconciled with the deployed release, customer configuration and provider terms before signature.

## Data flow

1. Browser -> TLS gateway on 80/443.
2. Gateway -> internal application/auth APIs.
3. Application -> authoritative Supabase PostgreSQL under transaction-scoped authorization.
4. Authorized knowledge/material operations -> private Qdrant/ingestion worker.
5. Optional AI operation -> internal AI Gateway carrying authoritative school identity.
6. Connected profile only: AI Gateway -> fixed approved external provider origin; provider credentials stay in the gateway.
7. Offline/local profile: AI request stays within packaged/local services as defined by the approved profile.
8. Operations -> encrypted local backup/WAL -> verified off-appliance destination; passphrase escrow is separate.

No browser path is intended to connect directly to PostgreSQL, Qdrant or external provider credentials.

## DPA inputs / subprocessors and providers

The executed DPA must identify the contracting parties, role allocation, processing instructions, categories of data/subjects, confidentiality/security measures, assistance duties, audit/cooperation, incident notice, deletion/return, and subprocessor-change process.

The provider list is deployment-dependent. At minimum review the actual hosting/target-host operator, any selected external AI provider, source/release infrastructure used operationally, and any customer-selected support/communication service. Do not list an external AI provider as a mandatory subprocessor when the contracted deployment is fully offline and no data is sent to that provider.

Before contract signature, a qualified reviewer must re-verify each selected provider's then-current processing terms, retention/training controls, processing locations, transfer mechanism and privacy/security contact route. Repository documentation intentionally does not freeze those changing legal terms.

## Data location and international transfers

Core production data is intended to remain on the customer's/self-hosted target environment and its approved backup location. Connected external AI may create an additional processing location determined by the selected provider/region/terms. Counsel/privacy review must document lawful transfer grounds and supplementary measures where cross-border processing applies. Offline/local mode avoids the external-AI transfer path but does not by itself resolve other customer infrastructure transfers.

## Retention and deletion schedule — contract input

Define school-specific periods before production for active accounts, assignments/submissions/grades, knowledge assets, operational logs/evidence, backups/WAL and support records. Product/business retention must not exceed the signed instruction without a legal/contractual basis. Backups may require expiry-based deletion rather than immediate row-level deletion; disclose that accurately.

## Data-subject request procedure

1. Authenticate/route the request through the school/controller's approved channel.
2. Verify subject/scope and legal basis/deadline.
3. Search supported authoritative stores plus relevant support/evidence records.
4. Export/correct/delete/restrict as instructed and technically supported.
5. Record backup-retention implications and later expiry/cryptographic-erasure behavior.
6. Keep an auditable request/disposition record without unnecessary personal data.

## Breach / security incident notification

Security operations escalate suspected confidentiality/integrity/availability incidents immediately under `security-organization.md`. Preserve evidence and establish known/affected scope. Contract/privacy counsel determines notification duties/timelines and customer/regulator communications. The signed DPA/contract must specify contact channels and notice timing; this draft does not invent a universal statutory deadline.

## DPIA trigger/template

Trigger qualified DPIA review when applicable law/customer policy identifies high-risk processing, including materially expanded student profiling, large-scale sensitive-data use, systematic monitoring, high-impact automated decisions, new cross-border/provider processing, or a material change in AI purpose/data categories.

Template sections: processing description; necessity/proportionality; data/subjects; lawful basis; recipients/locations/transfers; automated/AI logic and human oversight; risks to individuals; security/privacy controls; residual risk; consultation/approval; owner/review date.

## AI notice, purpose limitation and human oversight

AI is optional and purpose-limited to approved education/knowledge/personalization flows. Do not represent AI output as an unreviewable authoritative academic decision. Contract/customer notices should identify when external AI is enabled, categories of content sent, intended purpose, outage/degraded behavior and required teacher/staff oversight. Provider training/retention claims must come from current approved provider terms, not assumptions in this repository.

## School provisioning and parent/student notices

The school is responsible for lawful account provisioning instructions, appropriate role/linkage data, and jurisdiction/age-appropriate parent/student/staff notices and consents where required. Public self-signup is disabled. Privileged credentials must not be distributed as ordinary user accounts.

## End-of-contract export, return and deletion

Define authorized export format/scope and verification, then revoke access, complete agreed return/export, delete active-system school data and derived/vector copies after holds expire, and age out backups/WAL under the signed retention schedule. Record completion and any legal hold/technical exception. Do not promise immediate destruction of immutable backup generations unless the backup design and contract explicitly support it.

## Required approval

PR #16 must record the qualified privacy/legal reviewer, exact document/release revision, required corrections, provider-term re-verification and final disposition before this material is used as signed compliance/contract language.

## Connected-AI provider review register

The current controlled-AI architecture permits these optional external providers when the corresponding connected profile is selected. Inclusion here is a technical inventory, **not approval of the provider's current legal/privacy terms**.

| Provider | Technical purpose | Mandatory for offline mode? | Contract/privacy action before use |
| --- | --- | --- | --- |
| OpenAI | `openai-v1` embeddings (`text-embedding-3-small`) | No | Verify current DPA/processing role, retention/training controls, regions, subprocessors, transfers and security/privacy contact route. |
| DeepSeek | approved connected LLM origin/profile | No | Verify current DPA/processing role, retention/training controls, regions, subprocessors, transfers and security/privacy contact route. |
| Customer-selected target-host/backup operator | hosting/storage/operations where supplied by a third party | Deployment-dependent | Record legal entity, location, access, security terms, subprocessor role and transfer basis. |
| Customer-selected support/communications provider | support/incident communications if used | No | Record only when production support actually uses one; minimize school/personal data. |

A fully offline/local AI deployment must not be described as sending AI request content to OpenAI or DeepSeek. A connected deployment must not rely on this repository as evidence of then-current provider retention, training or international-transfer terms; PR #16 records qualified re-verification before signature.
