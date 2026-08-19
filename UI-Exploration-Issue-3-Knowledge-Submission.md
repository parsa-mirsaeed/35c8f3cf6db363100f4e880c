# Issue 3 — Knowledge submission registers successfully, but intake is URL-only and lifecycle success is ambiguous

This finding extends `UI-Exploration-Fix-Plan.md` on the UI exploration branch. It is intentionally recorded before implementation so it can be handled in the consolidated exploration PR together with Issues 1 and 2.

## User-visible behavior

From School Manager → Knowledge Submissions:

- the manager can submit a governed PDF by entering a controlled source URL plus original filename and metadata;
- after a successful request, the page displays `Submission registered for internal OCR and platform review.`;
- the newly created item appears in the independently refreshed `School submissions` list with status `submitted`;
- there is no `Browse`, file-picker, drag/drop, or other local-PC upload path beside the controlled URL field.

## What the current success actually proves

The manager endpoint `manager/knowledge-submissions` is an active SchoolManager-only production capability. It derives school scope from the authenticated manager rather than accepting school authority from the browser.

On successful submission, the repository performs one database transaction that:

1. inserts a `knowledge_assets` row with initial status `submitted`;
2. inserts the corresponding `knowledge_source_files` metadata row;
3. appends a `knowledge_asset.submitted` audit event;
4. commits the transaction;
5. returns the persisted asset to the server function.

The UI only displays the success notice when that server function returns `Ok`, then restarts the School submissions resource. Seeing the new asset in that independently loaded list is therefore strong evidence that registration/persistence succeeded.

However, `submitted` is only the first lifecycle state. The current manager submission path does **not** itself perform OCR, embedding, or publication.

The UI source explicitly states: `This does not extract, embed, or publish the document automatically.`

The governed lifecycle is separately administered:

```text
submitted
  -> verified OCR / ocr_ready
  -> embedding_pending
  -> embedded
  -> published
```

The platform-admin UI currently attaches verified OCR explicitly, queues embedding explicitly, and publishes explicitly. A manager-facing message must therefore not imply that OCR or review has already completed merely because registration succeeded.

## Root cause / product gap

The current manager form models the document source only as metadata:

- `Controlled source URL` text input;
- `Original filename` text input;
- `source_type = "pdf"`;
- `mime_type = "application/pdf"`;
- file size, SHA-256, and page count are currently unset;
- no file bytes are transferred by this form.

The backend persistence model already supports source-file metadata (`original_file_url`, filename, MIME type, size, SHA-256, page count, scanned-PDF flag), but the active manager UI does not provide a trusted local-file ingestion path.

This is especially important for the offline/on-premise product because a school operator should not have to first place a document at an externally reachable URL merely to register it for the governed knowledge workflow.

## Required repository implementation

Add a first-class local file intake path **beside**, not instead of, the existing controlled-URL path.

The final design should provide an explicit source choice such as:

```text
Source
  ( ) Upload file from this computer
  ( ) Controlled source URL
```

or an equivalent accessible UI.

### Local file path

The implementation must:

1. provide a normal browser file picker and, where useful, drag/drop;
2. initially accept only the production-supported document types; the current UI contract is PDF and must not silently claim support for formats that the ingestion pipeline does not actually process;
3. transfer the file to an EduTalent-controlled internal storage boundary rather than requiring an external public URL;
4. work in the supported on-premise/offline topology without internet access;
5. create the governed asset only after the source file has been durably accepted, or otherwise use a clearly defined compensating/cleanup state so an asset cannot falsely claim a stored source that does not exist;
6. derive and persist trustworthy server-side metadata where possible, including actual filename, MIME/content type, byte size, and SHA-256;
7. never trust a browser-supplied MIME string or filename as sufficient proof of file type;
8. enforce a bounded maximum upload size at the trusted server/reverse-proxy boundary and return an explicit safe validation error when exceeded;
9. sanitize storage keys/paths and never use the client filename as an unchecked filesystem path;
10. keep stored source documents tenant-scoped and inaccessible to other schools;
11. avoid exposing source documents through unauthenticated/public object URLs;
12. audit the source registration consistently with the existing governed knowledge lifecycle.

The exact internal storage mechanism should be chosen after reviewing the production appliance/storage architecture. The fix must not introduce a cloud-only dependency that breaks air-gapped operation.

### Controlled URL path

Retain the current URL workflow for cases where the school has an approved controlled internal document location.

The two source modes should have a clear one-of contract:

- local file upload requires a selected file and does not require manual URL/filename entry;
- controlled URL requires a valid allowed URL and an original filename or derives it safely when appropriate;
- the server rejects ambiguous requests that attempt incompatible source modes simultaneously.

Any URL retrieval performed by EduTalent must follow the repository's SSRF/egress security model; adding a local upload path must not weaken those boundaries.

## Success and lifecycle UX requirements

The manager UI should clearly separate **registration success** from **processing/review status**.

After the source is durably registered, show a message such as:

```text
Submission registered successfully. Status: Submitted.
OCR/review has not completed yet.
```

The list should render the real persisted lifecycle state with understandable labels, for example:

- Submitted;
- OCR pending;
- OCR ready / verified;
- Embedding pending;
- Embedded;
- Published;
- Failed;
- Archived.

Do not display a message that can reasonably be read as `OCR succeeded` or `publication succeeded` while the database status is merely `submitted`.

When a downstream state is `failed`, the manager-facing UI should provide safe actionable information without exposing sensitive provider/internal details.

If appropriate for the product workflow, the manager view should refresh/poll status or provide a deliberate refresh control so the operator can see progress without relying on the platform-admin screen.

## Verification and observability

For registration, success should be verifiable at three layers:

1. server function returned success with an asset ID;
2. a same-school manager list call returns that asset with persisted status;
3. database/audit state contains the asset, source-file row, and `knowledge_asset.submitted` audit event.

For downstream processing, success must be judged by lifecycle state rather than the submission toast:

- OCR success requires verified OCR state/data;
- embedding success requires the embedding job/state to complete;
- publication success requires `knowledge_assets.status = 'published'` with publication consistency satisfied.

## Required regression tests

Add focused tests for both source modes and lifecycle truthfulness:

- controlled-URL submission persists an asset, source metadata, and audit event in one coherent transaction;
- local PDF upload persists durable source bytes plus matching metadata/hash and creates exactly one governed asset;
- same-school manager can list the submitted asset;
- cross-school manager cannot observe or retrieve the source/asset;
- unsupported file type is rejected before creating a misleading successful asset;
- oversized files are rejected cleanly;
- spoofed MIME/extension does not bypass trusted validation;
- malicious/path-like filenames cannot escape the storage namespace;
- failed file persistence does not leave a successful-looking orphan asset;
- duplicate/retried upload behavior is deterministic enough to avoid unintended duplicate source objects/assets;
- URL mode and file mode enforce the intended one-of source contract;
- UI success appears only after durable registration;
- UI does not imply OCR/embedding/publication completion while status remains `submitted`;
- lifecycle status rendering maps every current `knowledge_asset_status` value accurately;
- offline/appliance validation proves local upload does not require internet egress.

## Acceptance criteria

- [ ] School Manager can choose a PDF from the local computer through the production UI.
- [ ] Controlled source URL submission remains available.
- [ ] Local upload works in the supported offline/on-premise topology.
- [ ] File bytes are durably stored in a controlled tenant-safe location.
- [ ] Trusted metadata including size and SHA-256 are recorded for uploaded files.
- [ ] No public/external URL is required for local upload.
- [ ] Upload validation is bounded and safe.
- [ ] Registration success returns/retains the durable asset ID.
- [ ] School submission list shows the newly registered asset and true lifecycle status.
- [ ] Registration messaging does not imply completed OCR/review/publication.
- [ ] OCR, embedding, failure, publication, and archive states are distinguishable in the UI.
- [ ] Source and asset remain school-scoped under RLS/application authorization.
- [ ] Audit evidence exists for registration and downstream governed actions.
- [ ] Relevant API/web/security/offline tests are green.

## Current exploration conclusion

The observed submission is considered **successfully registered** when it appears with status `submitted`; that is the correct initial lifecycle state. It should not yet be considered OCR-complete, embedded, or published.

No temporary local workaround is required to continue UI exploration. The missing local-file intake and lifecycle-status UX should be implemented in the consolidated PR after exploration is complete.