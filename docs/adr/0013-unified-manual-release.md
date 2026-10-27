# ADR 0013: immutable manual GitHub/npm release promotion

Accepted 2026-10-27; workflow separation revised 2026-10-28. This extends ADR 0010's packaging decision. The npm identities are `agent-job-relay-runtime` and `agent-job-relay-opencode-v1`; scoped duplicates are outside this release route.

## Decision

Use two maintainer-operated `workflow_dispatch` workflows on main. **Prepare Release** (`release.yml`) validates Cargo version/pins/CHANGELOG, runs Linux/Windows native and installed-package/actual-loader checks, then seals the source/run/channel and complete asset hashes. It receives no npm publishing credentials. Formal releases default to `latest`; `next` is an explicit prerelease decision.

**Publish Release** (`release-publish.yml`) accepts the original preparation Run ID and defaults to dry-run. It checks the actual publication workflow's OIDC identity, exchanges tokens for both packages and previews the sealed channel without uploads or release mutations. A separate manual run with dry-run unchecked authorizes publication. npm Trusted Publisher configuration must identify this publishing filename; using the preparation workflow would check a different identity. Publishing uses OIDC without a traditional-token fallback.

Cargo.toml remains the version authority, and CHANGELOG supplies a unique reviewed version section with generated distribution boundaries appended. Actions never guesses a version bump, edits source or pushes a release commit. Publication reuses the successful preparation run's unexpired artifact and never rebuilds or repacks. Normal promotion requires unchanged main; opt-in recovery admits only exact release-tooling/operational-documentation changes in a descendant commit. Runtime/package/build/version/CHANGELOG changes remain forbidden, and the original source/artifact identity is retained.

First-package bootstrap can be performed manually by a maintainer from downloaded reviewed tarballs with interactive npm authentication/2FA; it does not require another general workflow or weaken routine OIDC publication. Legacy stage-only helper behavior remains covered, including npm's `validating` state and verified ready bytes, but the two current Actions entry points do not receive a stage token.

Create/retain a GitHub draft before npm uploads, publish runtime before OpenCode, and expose GitHub Release only after matching public bytes and clean installation pass. The by-tag endpoint omits drafts, so authenticated pagination discovers them and exact Release IDs select uploads. Preserve matching empty duplicate drafts and reuse the populated original; multiple populated drafts or foreign content stop. No release/asset deletion, tag movement, stage rejection, unpublish or implicit rollback occurs.

## Consequences and regression boundaries

The two dry-run phases separate build qualification from credential qualification. A green preparation does not establish OIDC access, and a green publication dry-run does not prove every eventual upload/write/provenance rule. Real publication remains a maintainer-operated remote acceptance step. GitHub/npm do not provide a multi-service atomic transaction; matching partial uploads must remain reusable across retries and previews.

Successful npm writes can precede public metadata propagation. After an upload or channel write, allow a bounded two-minute read-only propagation window, retaining integrity/newer-channel fences and never repeating the write in that wait. A timeout preserves the existing upload/draft for same-candidate retry; it never permits a rebuild under the used version.

Behavioral tests cover version/changelog errors, source/run and Git-tree fences, immutable inventories, preparation without publishing credentials, the exact OIDC workflow identity, sealed-channel previews with no uploads, partial publication, staged/public byte conflicts, token separation and draft pagination/ID uploads. Native/host checks retain their evidence boundaries: a simulated model is not a real provider or new UI acceptance. Keep main unchanged during normal publication; tooling recovery retains original bytes, and fourteen-day artifact expiry still stops promotion.
