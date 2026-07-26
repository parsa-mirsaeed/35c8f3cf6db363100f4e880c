# EduTalent Agent Engineering Guide

**Engineered by Parsa Mirsaeed**

This file is the operating contract for every human or automated coding agent
working in this repository. Its purpose is to make implementation claims
verifiable. An agent must not describe a change as successful merely because the
code looks correct or because a commit was created. Success requires a green,
exact-head proof appropriate to the change.

## 1. Core rule

For every change:

1. Identify the behavior being changed.
2. Identify the affected packages and infrastructure.
3. Add or update the smallest meaningful tests.
4. Commit the implementation and tests together.
5. Wait for the workflows on the exact commit SHA.
6. Inspect every required job and its logs.
7. Fix failures rather than bypassing checks.
8. Report success only after the required exact-head gates are green.

A passing workflow on an older SHA is not evidence for a newer SHA.

## 2. Repository boundaries

EduTalent is a Rust/Dioxus full-stack workspace with PostgreSQL, Supabase,
Qdrant, Docker packaging, and a self-hosted production topology.

Important areas:

- `packages/api/`: backend domain, repositories, services, middleware, and
  server functions.
- `packages/web/`: Dioxus Web application and role-based views.
- `packages/ui/`: shared UI components.
- `migrations/` and `packages/api/migration/`: canonical database migrations.
- `scripts/ci/`: migration and security verification scripts.
- `docker/`, `Dockerfile`, `compose*.yaml`, `edutalent`: build and packaging.
- `deploy/production/`: production Supabase, Caddy, Qdrant, TLS, role, and
  network topology.
- `.github/workflows/`: objective implementation proof.

Do not weaken authorization, migration integrity, secret handling, runner
isolation, Qdrant filtering, or production network boundaries to make a test
pass.

## 3. Dioxus 0.7 implementation rules

Use Dioxus 0.7 APIs and documentation only. Older examples are not valid for
this repository.

- Do not use removed `cx`, `Scope`, or `use_state` APIs.
- Components use `#[component]` and return `Element`.
- Use `use_signal` for local reactive state and `use_memo` for derived state.
- Component props must be owned values and implement `Clone` and `PartialEq`.
  Prefer `String`, `Vec<T>`, and `ReadOnlySignal<T>` over borrowed props.
- Use `#[get]` and `#[post]` server functions with stable explicit paths.
- Keep server-only imports, derives, validation, SQLx, and operating-system code
  behind the appropriate `server` feature gates.
- Keep the initial client render identical to the server render. Use
  `use_server_future` when data must be present during SSR and hydration.
- Run browser-only APIs after hydration, normally inside `use_effect`.
- Prefer direct `for` loops and conditional elements in `rsx!`; wrap iterator
  expressions in braces.
- Use `asset!` for repository assets and `document::Stylesheet` for stylesheet
  injection.
- Define routes through the repository's `Routable` enum and preserve
  role-based route authorization.

## 4. Validation levels

### Level 1: AI Change Proof

Workflow: `.github/workflows/ci.yml`

Runs on every pull-request update. It is the minimum proof required after an
agent commit.

It automatically detects:

- Rust workspace changes;
- API/backend changes;
- Web/UI changes;
- database and migration changes;
- packaging changes;
- production-topology changes;
- documentation-only changes.

It runs only the relevant Rust/database checks and creates an
`ai-change-evidence` artifact tied to the exact head SHA.

Required branch-protection check:

- `AI change gate`

### Level 2: Specialized focused checks

These workflows run only when their paths are affected:

- `Package / Validate package definitions`
- `Production Foundation / Render and verify production topology`

They are required evidence for packaging and production-topology changes even
though they are not globally required branch-protection checks.

### Level 3: Full Validation

Workflow: `.github/workflows/full-validation.yml`

Full validation runs:

- on pushes to `main`;
- on the weekly schedule;
- by manual dispatch;
- on a PR while it has the `full-validation` label.

Once a PR enters final review, apply the `full-validation` label and keep it
until merge. Every later commit then reruns complete database and Rust
validation.

Required final-review check:

- `Full validation gate`

### Level 4: Package and production proof

For a release, production/security change, or packaging change, run the
specialized full workflows on the exact final SHA:

- `Package / Docker image and release bundle`
- `Production Foundation / Apply migrations and roles on pinned Supabase PostgreSQL 17`
- `Production Foundation / Start complete self-hosted production stack`

The full package job runs automatically for a relevant PR carrying the
`full-validation` label. The full production workflow can be dispatched
manually on the PR branch and runs automatically on relevant `main` changes and
its schedule.

## 5. Change-to-test matrix

### Documentation only

Examples: `README.md`, `docs/**`, `AGENTS.md`, `SECURITY.md`, and `LICENSE`.

Required: `AI change gate`.

Do not start PostgreSQL or compile Rust unless documentation also changes a
generated or executable configuration.

### API/backend logic

Examples: services, repositories, server functions, and middleware under
`packages/api/src/`.

Required:

```bash
cargo check -p api --features server --all-targets --locked
cargo clippy -p api --features server --lib --tests --locked -- \
  -A warnings -D clippy::correctness -D clippy::suspicious
cargo test -p api --features server --lib --locked
cargo check -p web --features server --all-targets --locked
```

The Web compile is required because Web depends on API.

### Web or shared UI

Examples: `packages/web/**` and `packages/ui/**`.

Required:

```bash
cargo check -p web --features server --all-targets --locked
cargo clippy -p web --features server --all-targets --locked -- \
  -A warnings -D clippy::correctness -D clippy::suspicious
cargo test -p web --features server --locked
```

Compilation and unit tests do not prove browser behavior. For changes to login,
navigation, forms, permissions, or browser state, add a browser-level test when
the browser harness is available and describe the manual verification until it
is automated.

### Workspace or dependency configuration

Changes to `Cargo.toml`, `Cargo.lock`, workspace features, desktop, or mobile
require complete workspace compile plus API and Web compile, Clippy, and tests.

### Database migrations or SQL queries

Required:

1. Apply all migrations.
2. Replay all migrations.
3. Verify governed schema lifecycle.
4. Verify security invariants.
5. Export the verified schema.
6. Compile and test affected Rust packages against that schema.

Never edit an already-applied migration silently. The checksum protection is a
security and operational guarantee.

### Authentication, authorization, and governed knowledge

Required:

- affected API tests;
- database security invariants;
- dependent Web compile;
- focused integration proof for the changed boundary.

Preserve these invariants:

- database authorization precedes vector retrieval;
- Qdrant filters include school, publication state, and exact authorized asset
  IDs;
- unpublished or archived assets are not retrievable;
- teachers cannot bypass the governed ingestion boundary;
- duplicate active ingestion jobs remain prevented;
- migration/bootstrap credentials never reach long-running services.

### Packaging

Per commit: shell, Compose, and package-definition validation.

Final PR: image build, source-free release archive, checksum inspection, and
packaged migrations executed twice.

### Production topology and security

Per commit: executable syntax, pinned Supabase materialization, isolated test
secrets, rendered Compose, and fail-closed security invariants.

Final PR: pinned Supabase PostgreSQL 17 migrations and role checks, exact runtime
image build, full stack startup, and database, TLS, gateway, authentication, and
administrative-boundary smoke tests.

## 6. Test design rules

- Prefer behavior tests over implementation-detail tests.
- A production bug fix must include a regression test when practical.
- New backend behavior should have API, service, or repository tests in the same
  change.
- New authorization behavior requires both allowed and denied cases.
- Migration changes require first-run and replay/idempotence proof.
- Do not delete, ignore, or weaken a failing test without explaining the
  obsolete requirement.
- Do not use `#[ignore]`, broad Clippy allowances, or catch-all error handling as
  substitutes for a fix.
- Use deterministic fixtures. Do not commit real student, teacher, school,
  customer, credential, document, or production identifiers.
- Never make external AI or network availability a health requirement for the
  core offline school system.

## 7. Agent execution protocol

Before editing:

```bash
git status --short
git branch --show-current
git fetch --all --tags --prune
```

During implementation:

- keep the change scoped;
- inspect existing abstractions before adding new ones;
- preserve feature-gated client/server compilation;
- update tests and documentation together;
- do not commit generated build output, `.env`, private keys, database dumps,
  PDFs, or runtime secrets.

Before committing, run the narrowest local checks available. CI remains the
source of truth because it provides a clean, recorded environment.

After committing:

1. Record the exact head SHA.
2. Inspect workflow runs for that SHA.
3. Verify `AI change gate`.
4. Verify Package or Production focused checks when the evidence artifact marks
   them required.
5. For final review, apply `full-validation` and verify all final checks.
6. Never claim that skipped work passed.

A job that failed before checkout is infrastructure failure, not code success.
A job that was skipped is not proof unless the classifier explicitly determined
that it was irrelevant.

## 8. Evidence and reporting

Every implementation report must state:

- exact commit SHA;
- files or systems changed;
- tests added or changed;
- workflows and jobs that ran;
- pass, fail, or skip status;
- any check not run and why;
- remaining risks or manual verification.

Preferred completion statement:

> Exact head `<sha>` passed `AI change gate`; API unit tests and dependent Web
> compile passed; no package or production workflow was required.

Never write “all tests passed” unless every test implied by the sentence
actually ran on the exact SHA.

## 9. Self-hosted runner configuration

Repository variables select runner pools. Every value is a JSON array accepted
by `fromJSON`.

Backward-compatible default:

```text
EDUTALENT_RUNNER_LABELS=["self-hosted","Linux","X64","edutalent-ci"]
```

Optional specialized pools:

```text
EDUTALENT_FAST_RUNNER_LABELS=["self-hosted","Linux","X64","edutalent-fast"]
EDUTALENT_RUST_RUNNER_LABELS=["self-hosted","Linux","X64","edutalent-rust"]
EDUTALENT_DOCKER_RUNNER_LABELS=["self-hosted","Linux","X64","edutalent-docker"]
EDUTALENT_PRODUCTION_RUNNER_LABELS=["self-hosted","Linux","X64","edutalent-production"]
```

When a specialized variable is absent, workflows fall back to
`EDUTALENT_RUNNER_LABELS`, then to `ubuntu-latest`.

Multiple runners with the same label form a pool. GitHub assigns one queued job
to one available runner. Use separate machines or appropriately isolated VMs
for parallel heavy jobs. Do not start several heavy runners on the current
7.4-GiB laptop; competing Rust, PostgreSQL, and Docker workloads would reduce
reliability.

Runner requirements:

- repository-scoped;
- dedicated Linux user;
- no `sudo`;
- no rootful Docker group;
- no personal SSH or GPG credentials;
- rootless Docker for Docker jobs;
- separate work and cache directories;
- one job per runner;
- no untrusted public-fork code.

## 10. Branch and merge policy

For ordinary development:

1. create or update a feature branch;
2. open a PR;
3. require `AI change gate`;
4. require specialized path checks when triggered;
5. apply `full-validation` when implementation is complete;
6. verify `Full validation gate`;
7. run full Package or Production proof when affected;
8. merge only the exact validated head.

A force-push, rebase, conflict resolution, or new commit invalidates previous
evidence and must be revalidated.

## 11. Security and publication

This repository is proprietary and all rights are reserved. Follow `LICENSE`.

Never expose repository or runner secrets, generated production environment
files, Supabase secret keys, JWT signing material, database passwords, Qdrant
keys, TLS private keys, personal data, private documents, vector payloads, or
host-specific credentials.

Run secret scanning before public visibility or source releases:

```bash
gitleaks detect --source . --redact --no-banner --log-opts="--all"
```

## 12. Definition of done

A change is done only when:

- the implementation satisfies the requested behavior;
- relevant tests exist;
- exact-head required workflows are green;
- evidence is inspectable;
- diagnostics do not expose secrets or personal data;
- documentation is updated;
- no known regression is hidden by skipped or weakened validation.

When these conditions are not met, report the implementation as incomplete or
partially validated.
