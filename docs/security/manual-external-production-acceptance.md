# Manual and External Production Acceptance

## Purpose

This document is the single owner-directed follow-up for **human-only and externally qualified production-readiness evidence** that cannot be legitimately produced by repository CI.

The production-readiness plan originally places several of these checks inside PR-12, PR-13, PR-14 and the final release gate. Per the repository owner's explicit sequencing decision on 2026-08-16, those human/external checks are consolidated into this dedicated follow-up PR so machine-verifiable engineering PRs can merge once their exact-head automated gates are green.

This is a sequencing deviation only. **Nothing in this document is considered PASS until the named qualified person or external reviewer completes and signs the corresponding record against the frozen release candidate.** Automated axe/Playwright, GitHub Actions, synthetic load, CI recovery drills, documentation generation, or AI review cannot substitute for the required human/external evidence.

## Release-candidate identity and automated baseline

The automated engineering/release sequence through plan PR-15 is merged on `main`. The values below are objective repository/CI evidence only; they do **not** pre-approve any human, legal, security, operator or target-host result in this PR.

- Repository: `parsa-mirsaeed/35c8f3cf6db363100f4e880c`
- Final automated baseline on `main`: `102c2ef56edf31dc5bff7982b234481a7fcbc43b`
- PR-15 exact validated source head: `45daddb1fb9763388a9df64450c9f1aeab53225c`
- Final Release Acceptance workflow run: `31971376680`
- Final Release Acceptance artifact: `final-release-acceptance-evidence`
- Final evidence artifact digest: `sha256:c4bb60cac2cd472019da1f2e06be6746e69b6667cf4253e5ffcf769f08902297`
- Appliance manifest SHA-256: `0d8b67f2d754b5b3ca08b4e70a66e6a3ab5a095977d92898f48ee95026edfa5c`
- Packaged model revision: `5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`
- Automated appliance evidence: 16 images; `linux/amd64`; registry-disabled/no-pull startup success; native arm64 custom-image proof success.
- Automated release classification: `ready for final validation`
- Automated `ready_for_contracted_production`: `false`
- Release/tag: **not created by this manual-acceptance PR; protected production release/tag remains subject to the applicable acceptance/release process**.
- Qualification start date (UTC):
- Qualification owner:

Human/external testing should identify the exact installed artifact/build digest in addition to the source baseline above. If the candidate source SHA, signed release artifact, deployment configuration, or other material input changes after a check whose result could be affected, rerun that check or explicitly record why the evidence remains applicable.

---

## A. Human keyboard and screen-reader acceptance

Detailed procedure: `docs/security/pr-12-manual-accessibility-acceptance.md`.

Required evidence:

- tester identity;
- exact installed/tested release artifact and source SHA;
- browser/version and operating system;
- screen reader/version;
- keyboard-only results for authentication/logout, role navigation, student submission, teacher grading/feedback, persisted grade view and Persian/RTL behavior;
- screen-reader results for the same critical journeys;
- findings with severity/reproduction;
- permitted accepted risks with owner/date/rationale/expiry;
- final exact tested SHA.

Status: **PENDING HUMAN TEST**

---

## B. Clean target-host qualification and operator acceptance

Primary engineering procedure: `deploy/production/operations/TARGET_HOST_ACCEPTANCE.md`.

Required real-host evidence:

- clean supported replacement-host identity and supported-host preflight output;
- exact signed release/artifact installed and its digests;
- OS/kernel/CPU/RAM/storage/inode/filesystem evidence;
- actual at-rest encryption evidence;
- firewall/network/DNS/time-sync evidence;
- rootless Docker evidence or a reviewed rootful-Docker exception;
- tailored CIS/container-hardening assessment;
- immutable locked-release image/digest inventory;
- genuinely off-appliance encrypted backup and WAL destination;
- separate backup-passphrase escrow;
- fresh installation of the frozen release;
- encrypted backup restore, PostgreSQL PITR and Qdrant restore/reindex decision on the replacement host;
- disk-low/full, TLS, corrupt-config, failed-migration and AI/Qdrant/provider outage acceptance;
- measured school-specific RPO and RTO;
- school-scale load/soak and capacity-headroom results;
- patch/certificate/key/Supabase/Qdrant/model upgrade/rollback rehearsal as applicable;
- explicit single-node/not-HA acknowledgement;
- residual-risk owner/date and operator/security sign-off.

Status: **PENDING CONTROLLED HOST QUALIFICATION**

---

## C. Independent authorization/security review

Required reviewer evidence:

- reviewer/organization and independence statement;
- exact release candidate reviewed;
- authorization/tenant-isolation review scope;
- authentication/session review scope;
- privileged/operator surface review scope;
- AI Gateway/provider egress and tenant-boundary review scope;
- backup/recovery and secret-handling review scope;
- findings with severity and reproduction/evidence;
- remediation PR/issue references;
- residual findings with explicit owner/date/acceptance rationale;
- reviewer disposition.

Status: **PENDING INDEPENDENT REVIEW**

---

## D. Penetration test and remediation disposition

Required evidence:

- qualified tester/organization;
- dates and exact release candidate/environment;
- agreed scope and exclusions;
- test methodology and authenticated roles used;
- findings and severity;
- remediation references;
- retest results for remediated findings;
- explicit disposition for every unresolved finding;
- final tester/report sign-off.

Status: **PENDING EXTERNAL PENETRATION TEST**

---

## E. Privacy, legal and contract sign-off

The documentation package in PR-14 is an engineering/business input, not legal advice or legal approval.

Qualified legal/privacy/business reviewers must sign off, as applicable, on:

- controller/processor role allocation;
- processing/data inventory and data-flow accuracy;
- subprocessors/providers, data location and international-transfer assessment;
- retention/deletion and data-subject procedures;
- breach/incident notification procedure;
- DPIA trigger/template and AI-use/human-oversight notice;
- school provisioning and parent/student notice responsibilities;
- end-of-contract export/return/deletion process;
- proprietary deployment/use grant and contracted feature/exclusion schedule;
- support/escalation/maintenance-window terms;
- availability definition and accepted RPO/RTO responsibilities;
- customer hardware/network responsibilities;
- security-incident cooperation;
- AI-provider responsibility/outage behavior;
- acceptance procedure and warranty/limitation language.

Record:

- legal/privacy reviewer(s):
- business/contract owner(s):
- review date(s):
- exact document/release revision reviewed:
- required changes and references:
- final approval/disposition:

Status: **PENDING QUALIFIED LEGAL/PRIVACY/BUSINESS REVIEW**

---

## F. Operator acceptance and incident rehearsal

Required human operational evidence:

- named primary and backup operator(s);
- exact release/deployment configuration used;
- installation/upgrade/rollback walkthrough completed;
- alert handling and escalation walkthrough completed;
- backup/restore/PITR procedure walkthrough completed;
- certificate/key rotation walkthrough completed;
- simulated incident with declared incident lead, communications/escalation and recovery actions;
- recovery/communication timestamps;
- gaps/actions with owners and due dates;
- operator acceptance sign-off.

Status: **PENDING OPERATOR REHEARSAL**

---

## G. Final manual/external release disposition

The final human/external acceptance decision must confirm all applicable sections above are complete and reconcile any residual risks with the exact frozen release candidate. The green PR-15 automated proof is an input to this decision, not a replacement for it.

Required sign-offs:

- accessibility tester:
- target-host/operator owner:
- security reviewer:
- penetration-test owner:
- privacy/legal approver:
- contract/business approver:
- release owner:

Open residual risks (owner + expiry/review date):

- 

Final manual/external classification — select exactly one only after the required qualified evidence exists:

- [ ] not accepted
- [ ] safe to continue developing
- [ ] ready for final validation
- [ ] ready for limited pilot
- [ ] ready for contracted production

Decision rationale:

Decision date:

Frozen release source SHA verified unchanged:

Installed signed artifact/digest verified:
