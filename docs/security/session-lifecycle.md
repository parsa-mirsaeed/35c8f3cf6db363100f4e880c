# Session lifecycle and production API policy

## Execution evidence

```text
Repository: parsa-mirsaeed/35c8f3cf6db363100f4e880c
Base branch: main
Base SHA: 2f0f0181dcebc7a31ffe56c5f71f3bcc55c73d7e
Feature branch: agent/pr-02-session-lifecycle-api-cleanup
Relevant plan PR: PR-02 — Session lifecycle, inactive-account enforcement and production API cleanup
Finding still reproducible at base: yes
Required targeted workflow: AI Change Proof with PostgreSQL, API tests, dependent Web compile, local mock-auth lifecycle proof, and server-function inventory
Heavy workflows intentionally deferred: Production Foundation, Production Operations, Package and Air-gapped Appliance because no production topology or packaging definition is changed
```

## Canonical authenticated session

A Supabase token proves only that the provider signed a token. Every authenticated request also resolves the token subject through PostgreSQL and requires:

- an existing user row;
- `users.is_active = true`;
- an existing canonical role relationship;
- an existing school relationship.

The middleware injects `UserInfo` only from this database result. Email, role and school authorization are not trusted from browser input or token metadata.

A valid token for a disabled or deleted account is rejected immediately and both session cookies are removed. An expired or otherwise invalid access token may use the refresh path, but the refreshed token is subjected to the same database checks before cookies are rotated. A refresh can therefore never reactivate a disabled or deleted account.

## Cookie policy

Access, refresh and removal cookies share these attributes:

- `Path=/`;
- `HttpOnly`;
- `Secure`;
- `SameSite=Strict`.

Access tokens use a 15-minute maximum age. Refresh tokens use seven days. Logout and rejected sessions issue both removal cookies with `Max-Age=0` and the same security attributes.

## Authentication throttling

Login failures are limited per normalized email in an unknown-address bucket. The application deliberately does not trust forwarded client-address headers until the gateway and trusted-proxy chain are explicitly configured and tested. Refresh failures are limited per available connection address and a SHA-256-derived token fingerprint; raw refresh tokens are never stored in limiter keys or logs. The current limiter is process-local, bounded to 10,000 keys, permits five failures in five minutes, and clears a key after success.

This control limits routine abuse on the supported single-node appliance. A future multi-instance deployment must replace it with a shared local rate-limit store or enforce an equivalent trusted-gateway policy. A future trusted-proxy implementation may add verified client-address scoping without accepting browser-supplied forwarding headers.

## Production API cleanup

- Browser-callable token verification and refresh server functions were removed; the browser uses HttpOnly cookies and `auth/whoami`.
- Notification list, summary and mutation operations derive the current user from middleware and accept no token or user ID argument.
- Legacy submission endpoints that returned empty results or fabricated `created`, `updated` or delete success were removed.
- The two real student submission endpoints require an active Student session, same-school enrollment, a published assignment and the exact custom assignment belonging to that student.
- The development `echo` server function was removed.
- A test discovers every production server-function module, requires an authorization classification, rejects duplicate/forbidden endpoints and prevents token arguments from returning.

## Error and logging policy

Authentication clients receive generic credential or service-availability errors. Upstream provider response bodies, passwords, access tokens and refresh tokens are not returned or logged. Internal failures are logged with status/error class only.
