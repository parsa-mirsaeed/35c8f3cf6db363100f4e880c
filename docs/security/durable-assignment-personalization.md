# Durable assignment personalization

## PR-05 evidence note

```text
Repository: parsa-mirsaeed/35c8f3cf6db363100f4e880c
Base branch: main
Base SHA: f0908bd3fcf04d4ec20d9fdd37cf1954f4fd679b
Feature branch: agent/pr-05-durable-assignment-personalization
Current head SHA: recorded in the pull-request evidence before final review
PR number: 12
Relevant plan PR: PR-05 — P1 Durable assignment personalization queue
Finding still reproducible: yes on the base SHA; assignment publication launched personalization through process-local tokio::spawn after commit
Affected files: assignment publication/server functions, durable queue migration/repository/worker, endpoint manifest, teacher status UI
Required targeted workflow: AI Change Proof with migration replay, API worker/database tests, local mock AI Gateway fault tests, dependent Web server compile; Full Validation once on the stable exact head
Heavy workflows intentionally deferred: Production Foundation, Production Operations, Package, Air-gapped Appliance, Mirror Final Proof
```

## Security and durability model

Assignment publication remains authorized by the actor-scoped assignment repository. The existing publication transaction marks the assignment published and fans out one `custom_assignments` row per active enrolled student. An `AFTER INSERT` database trigger now creates the corresponding `assignment_personalization_jobs` row inside that same transaction. If any required database operation fails, publication, fan-out, and enqueue roll back together.

Jobs are per assignment/student/profile and use a stable unique identity. Repeated publication therefore cannot create duplicate personalization work. Explicit teacher retry reuses the same job identity instead of creating another row.

Global queue discovery is limited to narrow `SECURITY DEFINER` claim and stale-recovery functions. They require the transaction-scoped `system_job` queue actor with no school context. The functions may claim or recover queue metadata only; they do not read student context or assignment content.

Actual personalization runs under the original authenticated Teacher identity and school in a normal `NOBYPASSRLS` transaction. The worker verifies the exact teacher, teaching assignment, published assignment, active student, current enrollment, and custom-assignment relationship before calling the local AI Gateway. It verifies that relationship again after the provider call and before commit. If authorization changes while AI is working, the generated content and job-success transition are rolled back.

## Retry and restart behavior

Jobs carry bounded attempt counts, `available_at`, a worker lease, heartbeat, safe error code, safe summary, and lifecycle timestamps. A terminated worker leaves a running job with a stale heartbeat. Recovery returns it to the queue only while its attempt count remains below the configured bounded maximum; a repeatedly interrupted job that reaches that ceiling transitions to `failed` with the controlled `worker_restart_limit` code and requires explicit retry. Provider outages, rate limits, network failures, and invalid gateway responses use bounded retry with exponential/provider-directed delay. Content rejected by prompt-size or secret-shape safety rules is terminal until an operator/teacher changes the underlying input and explicitly retries.

Partial class completion is naturally resumable because each student has an independent job. A committed personalized payload is reconciled to `succeeded`; remaining queued students continue independently.

## Cancellation and authorization changes

Before every claim, the queue reconciles queued/running jobs against current authorization. A job is cancelled when the assignment is no longer published, the teacher is inactive or no longer assigned to the class, the student is inactive, enrollment is gone, or the custom assignment no longer exists. Deleted assignments remove their queue rows through the foreign-key lifecycle. A future assignment archive operation automatically becomes non-claimable because only `Published` assignments are eligible.

## Data minimization and observability

The queue stores identifiers required for authorization/idempotency, lifecycle metadata, the fixed model/profile/version, and controlled error codes/summaries. It does **not** store prompts, provider responses, provider credentials, generated content, or raw exception bodies.

Worker logs use job/assignment/school identifiers and controlled error codes. Provider bodies and prompts are never logged by the worker. Generated content stays in the existing authorized `custom_assignments` record.

The teacher dashboard exposes only aggregate queue counters for the authenticated active teacher. It does not expose student identifiers, prompts, generated content, or raw error details.

## Validation contract

Required PR-05 proof is intentionally narrow:

- migration first-run and replay;
- API server compile and correctness/suspicious Clippy;
- API unit and database integration tests;
- durable queue tests for atomic enqueue, duplicate publication, stale recovery, partial resume, bounded retry, cross-school isolation, revoked enrollment, and safe persisted errors;
- local mock AI Gateway faults covering timeout, rate limiting with retry delay, malformed success payloads, and temporary outage responses without relaxing the production fixed-origin gateway rule;
- dependent Web server compile because a teacher status component consumes the new server DTO;
- endpoint authorization inventory parity;
- final exact-head Full Validation once the implementation is stable.

Production Operations, Package, Air-gapped Appliance, and Mirror Final Proof are not PR-05 evidence because this PR does not change production topology, recovery scripts, package definitions, image inventory, installer, signing, or release orchestration.
