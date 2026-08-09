# PR-11 Offline Browser Security Boundary

This document records the production contract for the browser offline boundary.

## Required invariants

1. No runtime browser script, module, font, stylesheet, image, or other executable dependency may be loaded from a public CDN.
2. Critical browser journeys must work with external browser networking blocked.
3. Production responses must enforce a restrictive Content Security Policy and must not rely on `unsafe-eval`.
4. Browser JavaScript/WASM assets must not contain provider credentials, session secrets, internal service credentials, or private service URLs.
5. Browser network destinations must be explicitly inventoryable and allowlisted; the offline appliance must have no external browser destinations.
6. Frontend dependencies must remain pinned and included in release SBOM/provenance evidence.
7. CSP/security-header changes must preserve Dioxus hydration, server functions, authentication cookies, and the local application/API boundary.

## Required validation

- static scan for public-CDN/runtime-origin references;
- static scan for credential and private-service material in browser assets;
- production web build/check/tests;
- offline browser smoke with external requests blocked;
- CSP/security-header assertion against the production-like server;
- exact-head AI Change Proof.

## Scope rule

This PR must remove the observed `esm.sh/ogl` runtime dependency in the login visual effect rather than adding a CSP exception for it. A visual effect is optional; the offline boundary is not.

## Exit condition

The release contains no runtime public-CDN dependency, the production CSP is enforced, and the critical browser journey succeeds with external browser access blocked.