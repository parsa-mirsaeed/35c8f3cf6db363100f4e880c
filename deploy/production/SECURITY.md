# Production security operations

Before exposing a deployment:

- restrict SSH to a management VPN or explicit administrator addresses;
- apply a host firewall allowing inbound 80/443 and approved management traffic
  only;
- deny container outbound traffic at the host firewall except for future
  explicitly identified AI-gateway traffic;
- encrypt database, storage, Qdrant, secret, and backup volumes;
- store TLS and environment files outside the repository with restrictive
  ownership and permissions;
- disable password login for SSH where operationally possible;
- run `production-gateway-check` after startup to prove host bindings, non-root
  capability-free gateway state, public signup rejection, and denial of
  publishable-key administrator access;
- monitor certificate expiry, disk capacity, database health, queue depth, and
  authentication anomalies;
- establish encrypted off-host backups and prove restoration before launch;
- document school-specific privacy, retention, incident, and provider policies.

The Compose topology is one defense layer. A compromised Docker host or root
account can access local data and container secrets, so operating-system and
administrative controls remain mandatory.
