# PR-12 revalidation evidence note (plan §1.2)

```text
Repository:            parsa-mirsaeed/35c8f3cf6db363100f4e880c (sanitized CI mirror)
Base branch:           main
Base SHA:              8f21aa2a80be3353d4e3eeab5c83a630f3dc477d
Feature branch:        agent/pr-12-browser-e2e-acceptance
Current head SHA:      8f21aa2a80be3353d4e3eeab5c83a630f3dc477d (at branch creation)
PR number:             (opened as draft after scaffold)
Relevant plan PR:      PR-12 — Browser E2E, WCAG 2.2 AA, RTL and product acceptance
Finding still reproducible: yes — no browser E2E harness exists on main
Affected files:        tests/e2e/**, scripts/ci/run_browser_smoke.sh,
                       scripts/ci/run_browser_final.sh,
                       scripts/ci/verify_browser_harness.sh,
                       docs/security/pr-12-browser-e2e-acceptance.md
Required targeted workflow: AI Change Proof (AI change gate); browser smoke is
                       harness-scoped and does not require PostgreSQL/Docker on
                       docs/harness-only commits
Heavy workflows intentionally deferred: Production Foundation complete,
                       Production Operations, Package, Air-gapped Appliance
                       (no packaging/production-topology paths changed)
```

## Scope of this commit

Foundation only: pinned harness contract, configuration, synthetic fixtures,
mock IdP, offline/console guards, CI runner scripts, harness self-check, and
this evidence note. Spec implementations and the CI job wiring land in
subsequent commits on this branch, each validated on its exact head.
