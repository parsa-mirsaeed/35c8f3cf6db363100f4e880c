# EduTalent UI Exploration and Consolidated Fix Plan

## Purpose

This document is the running engineering record for issues discovered while interactively exploring the EduTalent production-topology UI before the next implementation PR.

Exploration workflow:

1. reproduce one UI/functional problem at a time;
2. identify the backend/database/frontend root cause rather than recording only the visible symptom;
3. use only the narrowest safe local diagnostic change when needed to continue exploration;
4. record the finding here with implementation and regression-test requirements;
5. continue exploring without implementing repository fixes yet;
6. after exploration is sufficiently complete, implement the accumulated findings together in one focused PR;
7. run only the tests/workflows necessary to cover the changed surfaces and make the PR merge-ready.

Local diagnostic database changes are evidence only. They are not repository-complete fixes or formal production acceptance evidence. Durable database corrections must be delivered as forward migrations with regression coverage.

---

## Issue Registry

| # | Area | Severity | Status | Summary |
|---|---|---|---|---|
| 1 | Class Management / PostgreSQL RLS / UI feedback | High | Root-caused; local fix proven; repository fix pending | Class creation succeeds but class-list refresh returns HTTP 500 because `enrollments` and `students` SELECT policies recurse. UI also gives no success confirmation and discards refresh errors. |
| 2 | School Manager User Provisioning / endpoint contract / form validation | High | Root-caused; repository fix pending | Student, Teacher and Parent creation forms call `/api/user_management/create`, but production marks that endpoint `Disabled`, so requests fail closed with HTTP 404. Existing provisioning code is not safe to enable unchanged. Several form required-field indicators also disagree with actual browser validation. |
| 3 | Manager Knowledge Submission / source ingestion | Medium-High | Functional URL registration verified; capability gap recorded | Controlled-URL registration succeeds, but the UI has no direct local file picker/upload path. Lifecycle wording also needs to distinguish registration from OCR, embedding and publication. |
| 4 | Platform Admin Knowledge Workflow / lifecycle UX / action gating | High UX / Medium functional | Root-caused from UI + lifecycle code; repository fix pending | The admin workflow exposes raw lifecycle operations without guidance. Destructive Archive has no confirmation, audit is developer-oriented, and invalid actions such as Attach verified OCR remain visible for `archived`, `embedded` and `published` assets. |

---

# Issue 1 — Class creation succeeds, but class list refresh fails with HTTP 500

## User-visible behavior

From School Manager → Class Management:

- class creation appears unsuccessful;
- no positive success feedback is shown;
- the new class does not appear in the list;
- browser console reports HTTP 500 from `api/classes/get_school_classes`.

Direct database inspection proved the class row was successfully inserted.

## Root cause

The production database uses forced PostgreSQL RLS. The policies form a recursion cycle:

```text
enrollments_select_policy
    -> reads students
    -> students_select_policy
    -> reads enrollments
    -> enrollments_select_policy
    -> ...
```

The exact application-role reproduction returned:

```text
ERROR: infinite recursion detected in policy for relation "enrollments"
```

The runtime role was verified as non-superuser and `NOBYPASSRLS`; the failure is policy design, not missing grants or corrupt class data.

## Local diagnostic proof

A local-only bounded `SECURITY DEFINER` helper was introduced to perform the student/parent relationship check without recursively re-entering `students` RLS. `enrollments_select_policy` was locally adjusted to call that helper.

The exact previously failing class-list query then succeeded as `edutalent_app`, and the class became visible in the UI.

This proves the RLS recursion is the root cause.

## Required repository implementation

- Add a new forward migration. Do not edit the historical migration that originally created the policies.
- Introduce the narrowest relationship helper required to break the cycle.
- Use fixed `search_path`, schema-qualified objects, restricted EXECUTE privileges and no broad RLS bypass.
- Recreate the affected policy while preserving Student, Parent, Teacher and SchoolManager authorization semantics.
- Keep FORCE RLS enabled and keep `edutalent_app` `NOBYPASSRLS`.
- Add an explicit regression using the same class-list query shape under the dedicated runtime role.
- Fix class-create UX so successful persistence is acknowledged even if the following list refresh fails.
- Stop swallowing the class-list error with `.await.ok()`; provide safe user feedback and structured server-side logging.

## Required regression coverage

- Student sees own enrollment, not unrelated enrollment.
- Parent sees child's enrollment, not unrelated enrollment.
- Assigned Teacher sees class enrollments, unrelated Teacher does not.
- SchoolManager sees same-school classes/enrollments, not cross-school data.
- Runtime application identity remains non-superuser and `NOBYPASSRLS`.
- Exact `class_sections + subjects + COUNT(enrollments)` query executes without recursive-policy failure.

## Acceptance criteria

- [ ] forward migration applies from clean repository state;
- [ ] historical migration files remain unchanged;
- [ ] class list loads without HTTP 500;
- [ ] created class appears after refresh;
- [ ] success confirmation is shown;
- [ ] refresh failure is differentiated from create failure;
- [ ] positive and negative RLS tests pass;
- [ ] no broad RLS bypass is introduced.

---

# Issue 2 — School Manager Student / Teacher / Parent provisioning calls an intentionally disabled endpoint

## User-visible behavior

All three School Manager creation forms fail with HTTP 404 from:

```text
/api/user_management/create
```

The real email used during exploration is intentionally omitted from this document and must remain absent from committed artifacts and automated test data.

Teacher/Parent exploration also exposed required-field contract drift. Example: Teacher phone is browser-required but the label does not display a required marker. Some other fields are visually marked required while not carrying equivalent browser constraints.

## Root cause

The active UI imports and calls `user_management::create_user`, whose endpoint is `user_management/create`.

The production endpoint authorization manifest explicitly classifies that capability as `Disabled`. The deny-by-default middleware intentionally returns HTTP 404 for disabled endpoints before the handler executes.

Therefore these failed requests do not reach account creation. The failure is a frontend/production-capability contract mismatch.

## Why the current handler must not simply be enabled

The existing provisioning path spans Supabase Auth and local application persistence. Auth creation happens before local user/role-specific writes and there is no complete compensation contract if later persistence fails. Re-enabling it unchanged could leave partial login-capable identities.

The current UI also generates temporary credentials client-side. Student class selection is captured by the form but the current student branch does not fully guarantee the advertised enrollment behavior.

The existing active `students/create` endpoint is not a complete replacement because it expects an already-existing local Student user. Another legacy student creation endpoint is also disabled/incomplete.

## Required repository implementation

Choose one authoritative production provisioning workflow for Student / Teacher / Parent creation and retire conflicting browser calls.

The final service must:

- derive school scope from the authenticated SchoolManager;
- enforce provisionable roles server-side;
- validate Parent and Class references within the same school;
- use one identity UUID consistently across Supabase Auth and local `users`;
- use an explicit compensation/saga strategy for Auth-vs-DB partial failures;
- group local user + role-specific + enrollment/assignment writes transactionally where possible;
- generate onboarding/temporary credentials on the trusted server side;
- avoid logging credentials or sensitive dependency errors;
- persist selected Student enrollment and Teacher assignments when the UI advertises them;
- expose typed, safe result classes to the UI;
- update the endpoint authorization manifest only after the hardened capability is complete.

## Form validation contract

For Student / Teacher / Parent forms, visible required markers, HTML/browser constraints, typed payload requirements and server validation must agree from one authoritative field contract.

Do not patch individual `*` labels independently.

## Required regression coverage

- active creation UI references only active production endpoint(s);
- disabled legacy endpoints remain 404;
- SchoolManager-only authorization is enforced;
- same-school boundaries are enforced for parent/class references;
- Auth and local UUIDs remain identical;
- selected Student class creates a real enrollment;
- Teacher assignments persist as advertised;
- duplicate email and forced mid-provisioning failures do not leave partial login-capable identities;
- required-field UI indicators and actual validation rules agree for all three role forms;
- no real exploration email appears in committed tests/docs.

## Acceptance criteria

- [ ] Student creation works coherently;
- [ ] Teacher creation works coherently;
- [ ] Parent creation works coherently;
- [ ] no active form calls a disabled endpoint;
- [ ] partial failures are compensated safely;
- [ ] visual and actual required-field rules match;
- [ ] cross-school negative tests pass;
- [ ] endpoint authorization remains deny-by-default.

---

# Issue 3 — Manager knowledge submission needs direct local file upload and clearer lifecycle semantics

## What was verified

School Manager controlled-URL registration succeeded.

Evidence from the UI and database:

```text
status = submitted
source URL metadata persisted = true
knowledge_asset.submitted audit event = 1
OCR records = 0
ingestion jobs = 0
embedded chunks = 0
```

That is a valid successful registration. The current design deliberately does not run OCR, embedding or publication automatically at manager submission time.

## Capability gap

The form currently supports only:

- Controlled source URL;
- Original filename;
- subject/grade/description metadata.

There is no browser `<input type="file">` path and no direct upload of PDF bytes from the School Manager's computer.

For the intended on-premise/offline deployment model, requiring a separately hosted controlled URL is unnecessarily difficult for normal school staff.

## Required repository implementation

Provide two clearly separated source options:

```text
Source document

○ Upload file from this computer
   [ Browse... ] curriculum-guide.pdf

○ Controlled source URL
   [ https://... ]
```

The local-upload path must:

- stream/upload bytes to an EduTalent-controlled internal storage boundary;
- enforce server-side size limits;
- validate MIME/content expectations rather than trust the browser filename alone;
- sanitize filenames and prevent path traversal;
- calculate server-side SHA-256 and trustworthy file size;
- associate the stored object with the authenticated school and created knowledge asset;
- avoid exposing server filesystem paths to clients;
- work in the air-gapped/on-premise deployment profile;
- preserve the existing governed lifecycle and audit requirements.

Controlled URL should remain available as an alternative for approved internal source stores.

## Lifecycle UX requirement

The manager success message must explicitly mean **registered/submitted**, not OCR-complete or published.

Prefer a visible lifecycle such as:

```text
Submitted → OCR review → Embedding → Published
                         ↘ Failed
Submitted/Published → Archived
```

The UI should explain who performs the next action and whether the document is currently available to teachers/generation.

## Required regression coverage

- URL registration continues to work;
- local PDF upload works;
- unsupported/oversize/malformed upload is rejected safely;
- server-calculated metadata is persisted;
- cross-school access to uploaded sources is denied;
- source bytes are not publicly exposed;
- registration creates one audit event and starts at `submitted`;
- registration does not falsely claim OCR/embedding/publication completion.

## Acceptance criteria

- [ ] SchoolManager can select a PDF directly from the computer;
- [ ] controlled URL remains supported;
- [ ] server-side validation/storage/hash behavior is covered;
- [ ] submission status wording is truthful;
- [ ] upload works in the target offline/on-premise topology.

---

# Issue 4 — Platform Admin knowledge lifecycle is operationally confusing and exposes invalid/destructive actions

## User-visible behavior observed

The Platform Admin successfully saw the SchoolManager submission.

The UI then exposed buttons such as:

- `Attach verified OCR`;
- `Queue embedding` depending on state;
- `Publish` depending on state;
- `Archive`.

During exploration the asset moved through:

```text
submitted
  -> OCR text attached
ocr_ready
  -> Archive clicked
archived
```

The audit trail recorded the submission, OCR verification and archive events, proving that the operations were real.

However, the page did not explain what each state meant, what the next correct step was, or what Archive would do. After the asset became `archived`, the card still displayed `Attach verified OCR`.

## Exact UI defect

`Attach verified OCR` is currently rendered unconditionally for every asset status.

That is incompatible with the backend lifecycle state machine. Attaching OCR updates the asset to `ocr_ready`; that transition is not valid from every state. In particular, an archived asset is not meant to be silently returned to OCR-ready by this action, and the database transition guard will reject invalid transitions.

The same misleading action can also appear for states such as `embedded` and `published`, where direct transition to `ocr_ready` is not an ordinary allowed lifecycle action.

Therefore the current UI advertises operations the backend state machine may reject.

## Archive is too easy to trigger

`Archive` currently executes immediately on click with no confirmation dialog or consequence explanation.

Archiving is not cosmetic. The backend marks the asset archived and disables teacher asset selections; the service also attempts to mark associated vector content unpublished before persisting the archive state.

A destructive lifecycle action with those consequences needs explicit confirmation and clear result feedback.

## Audit trail usability problem

The audit page currently shows raw developer/operator fields:

- machine-oriented action names such as `knowledge_asset.ocr_verified`;
- raw target UUIDs;
- raw JSON details;
- timestamps in technical formatting;
- internal actor role strings.

This is useful as low-level evidence but not sufficient as the primary workflow explanation for a human Platform Administrator.

## Required UX redesign

The asset card should become a guided workflow rather than a bag of operations.

For example:

```text
DRL
School: Test School
Current state: OCR reviewed

1. Submitted        ✓
2. OCR verified     ✓
3. Embedding        Next step
4. Published        Not yet available

[ Queue embedding ]
[ More actions ▾ ]
```

Each state must show:

- plain-language meaning;
- whether teachers/generation can use the asset;
- the recommended next action;
- blocking prerequisite if the next action is unavailable;
- failure reason and retry guidance where relevant.

## Required action gating

Actions must be derived from the lifecycle state machine, not rendered independently.

At minimum:

- `submitted`: allow OCR verification and archive;
- `ocr_pending`: allow relevant OCR completion/review and archive;
- `ocr_ready`: allow OCR correction/review if intended, queue embedding, archive;
- `embedding_pending`: show processing status; avoid conflicting operations unless explicitly supported;
- `embedded`: allow publish, re-embed if deliberately supported, archive; do not casually offer OCR reset;
- `published`: show that the asset is live; allow archive or an explicit reviewed rollback workflow only if intentionally supported;
- `failed`: show failure reason and only valid retry/recovery actions;
- `archived`: treat as inactive/terminal unless a deliberate restore workflow is implemented; do not show actions that the backend will reject.

The frontend state-to-action mapping should be tested against the backend/database transition rules so the two cannot drift.

## Destructive action confirmation

Before archive, show a confirmation that explains consequences, for example:

```text
Archive “DRL”?

This will make the asset unavailable for governed generation and disable teacher selections.

[ Cancel ] [ Archive asset ]
```

If active ingestion jobs or publication state will be affected, include that consequence in the confirmation/result.

## Better audit presentation

Keep the raw audit data available for diagnostics, but render a human-readable interpretation such as:

```text
18:43 — OCR verified by Platform Administrator
18:43 — Asset archived by Platform Administrator
18:07 — Submitted by School Manager
```

Advanced/raw details may be expandable rather than occupying the primary table.

## Required regression coverage

- every lifecycle state renders only valid actions;
- archived assets do not expose invalid OCR/embed/publish actions;
- published/embedded assets do not expose invalid OCR reset unless an explicit rollback workflow exists;
- Archive requires confirmation;
- cancelled confirmation produces no mutation;
- successful archive updates UI and audit consistently;
- failed lifecycle mutation shows a safe actionable error;
- state labels and recommended next step match backend transition rules;
- audit view produces understandable user-facing text while preserving raw audit evidence for diagnostics.

## Acceptance criteria

- [ ] Platform Admin can understand the current state without reading raw audit JSON;
- [ ] one clear recommended next step is visible for each active lifecycle state;
- [ ] invalid state transitions are not presented as clickable actions;
- [ ] Archive requires explicit confirmation and explains consequences;
- [ ] archived assets no longer show misleading `Attach verified OCR`;
- [ ] audit trail is human-readable;
- [ ] frontend state/action tests and backend lifecycle tests agree.

---

# Exploration-only Platform Admin account note

The local UI exploration environment originally contained no PlatformAdmin user. A synthetic exploration account was therefore created with one shared UUID across Supabase Auth and the local `users` table and was scoped so the current active-session resolver could authenticate it.

This account is local exploration support only. It is not evidence of a production operator bootstrap workflow and must not become a default credential in repository code or deployment artifacts.

A separate production-readiness review should ensure a documented secure first-operator/bootstrap procedure exists before school deployment.

---

# Template for the next exploration finding

```markdown
# Issue N — concise title

## User-visible behavior
## Reproduction steps
## Expected behavior
## Actual behavior
## Root cause / current evidence
## Security or tenant-isolation implications
## Local diagnostic proof, if any
## Required repository implementation
## Required regression tests
## Acceptance criteria
## Interaction/dependency with earlier issues
```

Do not implement merely because an issue has been recorded unless continuing exploration is blocked. If a temporary diagnostic change is required to continue, document it as local-only and preserve the durable repository implementation for the consolidated PR.

---

# Consolidated PR rule

After UI exploration is complete enough to define the batch, create one focused PR containing the recorded fixes that belong to this exploration cycle.

Before implementation:

1. review all findings for shared root causes;
2. group related database/frontend/API changes coherently;
3. avoid duplicative patches;
4. define the minimum sufficient tests/workflows for the complete batch;
5. preserve production security boundaries and migration immutability.

The PR is not merge-ready until every recorded issue included in its scope has its acceptance criteria satisfied and the relevant workflows are green.
