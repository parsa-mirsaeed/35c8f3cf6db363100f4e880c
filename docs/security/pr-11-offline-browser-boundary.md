# PR-11 Offline Browser Security Boundary

This document records the production contract for the browser offline boundary and the implementation evidence for PR-11.

## Required invariants

1. No runtime browser script, module, font, stylesheet, image, or other executable dependency may be loaded from a public CDN.
2. Critical browser journeys must work with external browser networking blocked.
3. Production responses must enforce a restrictive Content Security Policy and must not rely on the legacy `unsafe-eval` token.
4. Browser JavaScript/WASM assets must not contain provider credentials, session secrets, internal service credentials, or private service URLs.
5. Browser network destinations must be explicitly inventoryable and allowlisted; the offline appliance must have no external browser destinations.
6. Frontend dependencies must remain pinned and included in release SBOM/provenance evidence.
7. CSP/security-header changes must preserve Dioxus hydration, server functions, authentication cookies, and the local application/API boundary.

## PR-11 implementation

- The login iridescence effect was removed because its only implementation dynamically imported `ogl` from `https://esm.sh/ogl`. The login page now uses bundled CSS-only decoration and has no runtime module import.
- Every production response passes through the existing outer middleware and receives CSP, `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, and `Permissions-Policy` headers.
- HSTS is emitted only when `EDUTALENT_ENFORCE_HSTS=true`, because the application can run behind an external TLS terminator and must not force HTTP appliance endpoints into an invalid HSTS state.
- The CSP permits only same-origin application resources and explicitly denies frames, objects, arbitrary connections and remote scripts. It uses `wasm-unsafe-eval` only for WebAssembly execution; the legacy `unsafe-eval` directive is not present.
- `scripts/ci/verify_browser_asset_origins.py` scans browser source/assets for external origins, private service origins and credential-like material and fails closed.

## Required validation

- static scan for public-CDN/runtime-origin references;
- static scan for credential and private-service material in browser assets;
- production web build/check/tests;
- offline browser smoke with external requests blocked;
- CSP/security-header assertion against the production-like server;
- exact-head AI Change Proof.

## Scope rule

This PR removes the observed `esm.sh/ogl` runtime dependency rather than adding a CSP exception for it. A visual effect is optional; the offline boundary is not.

## Exit condition

The release contains no runtime public-CDN dependency, the production CSP is enforced, and the critical browser journey succeeds with external browser access blocked. Browser journey automation remains part of the dedicated PR-12 E2E acceptance suite where the repository's browser harness is established; PR-11's source/build boundary is independently enforced here.
