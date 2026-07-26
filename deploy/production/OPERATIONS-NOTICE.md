# Operations notice

This pull request establishes the secure production foundation. It is not the
final high-availability or fully air-gapped release.

Do not treat the deployment as production-ready until the application has passed
full-stack startup, tenant-isolation, restore, load, and provider-outage tests in
the target infrastructure. Do not enable external AI directly from application
containers; wait for the controlled AI-gateway phase.
