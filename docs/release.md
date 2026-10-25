# Release policy (future distribution)

0.1.0 is an unreleased source development milestone, not a public release.
Cargo.lock is committed. CI compiles/tests only; no release workflow, online
credentials, tags, package repositories or store submissions were created.

Before a future release: pass all native lifecycle gates, qualify platform
adapters, document known wake/containment limitations, verify migrations and
upgrade rollback, then implement the independently tracked Distribution &
Packaging plan in TODO.md. Build/upload and package-manager listing are separate
operations. Signing, notarization and repository publication require explicit
authorization and protected credentials/OIDC. Never infer permission to publish
from permission to build or make local Conventional Commits.
