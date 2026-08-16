# Tailored CIS Docker Benchmark v1.8.0 assessment map

**Scope:** EduTalent supported single-node production appliance host and Docker runtime.

**Benchmark:** CIS Docker Benchmark v1.8.0. The official CIS catalogue was rechecked in August 2026 and still lists Docker v1.8.0 as the current Docker benchmark. This document is a tailoring/evidence map only; it does not claim CIS certification or complete conformance.

The target host must retain the actual assessment output or equivalent evidence. Items below distinguish repository-automated evidence from host-specific manual evidence so a green CI run is never treated as a host benchmark pass.

| Assessment area | EduTalent treatment | Evidence / disposition |
|---|---|---|
| Host operating system | Supported baseline is Ubuntu Server 24.04 LTS on x86_64; kernel/filesystem/capacity are fail-closed automatic host-preflight checks. | `../host-baseline.json`, `../host_preflight.py`, retained host-preflight JSON. |
| Dedicated Docker/operator authority | Dedicated unprivileged `edutalent-operator`; rootless Docker preferred. Rootful Docker requires explicit daemon/socket-access review. | Automatic operator UID check; Docker rootless detection; manual disposition for rootful daemon. |
| Docker daemon remote access | EduTalent requires a local Docker context and does not require a remotely exposed daemon API. | Manual host daemon configuration/socket permission evidence. Any unauthenticated TCP Docker API is a blocker. |
| Host firewall and ingress | Only gateway ports 80/443 are public; administration remains CIDR-restricted at the gateway. | Rendered Compose validation plus target-host firewall ruleset evidence. Tool inability to read firewall state is `manual`, never PASS. |
| Host audit/log retention | Security-relevant host/Docker events must be available under the school's local logging policy without internet telemetry. | Manual host logging/audit configuration and retention evidence; document accepted exclusions. |
| Docker content trust / immutable release | Published EduTalent release images are verified by checksum/signature/provenance and the locked release inventory must use SHA-256 digests. | Air-gapped release evidence plus `../container_hardening_inventory.py --require-digests` against the locked release. |
| Build secrets | Provider credentials, TLS keys, backup passphrases and production env files are not baked into normal images/artifacts. | Existing secret scans, package/appliance gates, release-manifest evidence. |
| Privileged containers | Privileged containers are forbidden. | `validate-rendered-compose.py`, operations `security-check`, container inventory. |
| Docker socket mounts | Docker socket mounts into application containers are forbidden. | Rendered/live security checks and container inventory. Host operator CLI access remains a privileged operational boundary and must be restricted. |
| Host networking | Host networking is forbidden for production services. | Rendered/live security checks and container inventory. |
| Linux capabilities | Custom gateway and AI gateway drop all capabilities; no service may silently add capabilities outside reviewed topology. | Rendered/live checks and container inventory. Upstream services requiring a different posture must be explicitly reviewed rather than masked. |
| Container user | Custom long-running security-sensitive services use numeric non-root identities where the image supports it. | Rendered/live checks plus container inventory. Upstream Supabase image identities are recorded and reviewed individually. |
| Read-only root filesystem | Read-only root is required where the service supports it; writable state must be explicit. | Container inventory records root mode and every writable/read-only mount. Upstream services that require writable roots are assessed as tailored deviations, not silently marked compliant. |
| Resource limits | Custom gateway/app/AI/Qdrant services have explicit CPU/resource policy; school-scale capacity requires measured acceptance. | Rendered Compose checks, container inventory, target-host load/soak result. |
| PID limits | PID limits are recorded per service and should be set where compatible with the pinned upstream image. Missing limits require explicit review. | Container inventory + target-host assessment disposition. |
| Restart policy and health checks | Restart and health behavior is recorded; AI/Qdrant remain degradable rather than being misclassified as core-health dependencies. | Container inventory, Production Foundation live checks, operations fault tests. |
| Published ports | Only the gateway may publish 80/443; internal Supabase/PostgreSQL/Qdrant services publish no host ports. | Rendered Compose validator and live operations security check. |
| Networks | Application/data/admin networks are internal; only gateway ingress and AI Gateway egress are non-internal by design. | Rendered Compose validator and live network-membership checks. |
| `no-new-privileges`, seccomp, AppArmor | Record `security_opt`, seccomp/AppArmor state per container. Add host/profile evidence where defaults are relied on. | Container inventory plus target-host `docker inspect`/LSM evidence. Reliance on defaults must be explicit. |
| Sensitive host paths | Production services must not receive broad host filesystem mounts. TLS source is staged through a bounded one-shot service and backup paths stay external to application containers. | Rendered Compose validator, operations backup boundary tests, container inventory. |
| Logging driver / rotation | Configure bounded local Docker logs appropriate to the supported host and incident-retention policy. | Manual daemon/container log-policy evidence; absence of an approved rotation policy is an acceptance finding. |
| Daemon patch level | Docker Engine/Compose minimums are machine checked; security updates follow the documented upgrade/rollback procedure. | Host-preflight JSON and `DEPLOYMENT_UPGRADE.md`. |

## Required target-host disposition

For each benchmark item applicable to the installed Docker/Ubuntu host, the assessor must record exactly one of:

- `pass` — evidence demonstrates the tailored requirement;
- `fail` — remediation is required before acceptance;
- `not-applicable` — rationale explains why the benchmark recommendation does not apply to this architecture;
- `accepted-risk` — only when the production-readiness process permits it, with owner, date, rationale and review/expiry.

Attach the resulting assessment artifact/hash to `TARGET_HOST_ACCEPTANCE.md`. Repository checks validate architecture invariants but cannot inspect daemon settings, disk encryption, firewall policy, host audit configuration, physical backup separation, or operator access governance on a school's actual machine.
