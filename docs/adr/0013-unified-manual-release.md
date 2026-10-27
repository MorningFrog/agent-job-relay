# ADR 0013: immutable manual GitHub/npm release promotion

Accepted, 2026-10-27. This extends ADR 0010's candidate packaging with explicitly manual publication. The two main npm identities are `agent-job-relay-runtime` and `agent-job-relay-opencode-v1`; scoped duplicates are not part of this release.

## Decision

Use one `workflow_dispatch` workflow on main. Cargo.toml is the version authority; source package/dependency/plugin/launcher pins must agree before any build. CHANGELOG supplies a unique reviewed version section, with generated distribution boundaries appended to the Release Notes. The workflow never edits source, guesses a version bump or pushes a release commit.

Dry-run checks credentials with honest limits, runs both native gates and installed-package/actual-loader checks, then seals the repository, source revision, original run, channel and complete asset hashes. Promotion normally requires unchanged main and never rebuilds or repacks. Explicit candidate recovery additionally permits descendant commits that change only the exact release-tooling/operational-documentation allowlist; runtime/package/build/version/CHANGELOG changes remain forbidden. The same original source identity and artifact are the recovery boundary after partial publication.

First package creation uses a short-lived stage-only npm token. Actual staging is separate from dry-run, recognizes npm's `validating` lifecycle before attempting tarball downloads, verifies ready staged bytes and requires human npm 2FA approval. Both packages are considered even if the first awaits validation; Summary distinguishes automated validation from human approval. It can create npm's public placeholder for a new name. A subsequent promotion of the same candidate verifies the public versions and finishes GitHub publication. Later releases use package-bound OIDC Trusted Publishing; inherited npm tokens are excluded rather than acting as a fallback. The environment secret is never exposed to build jobs or artifacts.

Create/retain a GitHub draft with reviewed assets before npm uploads, publish runtime before OpenCode, and expose GitHub Release only after public byte/integrity and clean-install checks. The by-tag API omits drafts, so authenticated paginated listing discovers them and exact Release IDs select asset uploads. A matching empty duplicate from older retry logic is preserved while reusing the populated original; multiple populated drafts stop. An existing matching upload/stage is reused; changed/foreign assets, bodies, channels or tag targets stop without replacement. Never delete releases/assets, move tags, reject stages, unpublish versions or silently roll back partial success.

## Consequences and regression boundaries

There is no multi-service atomic transaction. A green staging run may still await npm approval; Summary must distinguish that from completed publication. Stage-only authentication, OIDC exchange, package previews and GitHub reads cannot prove every eventual upload/write rule, so dry-run must not claim those operations succeeded. Account-backed authentication/publication remains a manual remote acceptance step.

Behavioral tests cover malformed changelogs/version pins, source/run identity, Git-tree recovery fences, modified/missing/symlink inventories, no-upload preview, npm validation states, stage/public byte conflicts, token/OIDC separation, draft pagination/ID uploads and preservation of foreign release assets. Existing native/package/host checks stay in place; simulated model checks do not become real host/provider evidence. Keep main unchanged during normal promotion; tooling recovery retains the original candidate. Fourteen-day artifact expiry requires a fresh candidate before any publication has occurred.
