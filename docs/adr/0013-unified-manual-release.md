# ADR 0013: immutable manual GitHub/npm release promotion

Accepted 2026-10-27; open-loop publication and unified coordination revised 2026-10-28. This extends ADR 0010. The npm identities are agent-job-relay-runtime and agent-job-relay-opencode-v1; scoped duplicates are outside this release route.

## Decision

Use maintainer-operated workflow_dispatch entry points on main. Prepare Release (release.yml) derives Cargo versions, validates matching pins/CHANGELOG, runs Linux/Windows native and actual installed-package/loader gates, and seals source/run/channel plus complete payload hashes. It receives no npm publishing credentials. latest is formal; next is experimental.

Publish Release (release-publish.yml) defaults to dry-run and validates its actual OIDC identity and candidate. An explicit non-dry invocation authorizes writes: reuse/create the exact GitHub draft/assets, submit runtime then OpenCode, retain immutable acceptance receipts, and publish GitHub. It does not poll or depend on public registry metadata/installation after upload. Successful npm writes can precede public propagation; Publication submitted and Public verification passed are distinct outcomes.

Check Release (release-check.yml) has contents/actions read permissions and no npm publishing credential/OIDC permission. It separately checks exact public bytes, channel, clean installation and the original GitHub notes/assets/tag, with a bounded two-minute read-only propagation window. Integrity/newer-channel conflicts stop immediately. A failed check never republishes, moves channels or rolls back a Release.

Release All (release-all.yml) coordinates the same manual workflows through GitHub workflow_dispatch with the automatic Actions token. The actual npm publisher remains release-publish.yml, preserving existing Trusted Publisher configuration. The API returns exact child Run IDs; stable per-parent step keys recover lost responses and failed children without guessing the newest run. Successful candidates are reused; re-runs resume failed child Run IDs. Source fences detect main movement before dispatch and in children. Parent/child concurrency groups differ to avoid deadlock; cancellation never implies rollback or child cancellation.

Cargo.toml is the version authority. CHANGELOG provides one reviewed version section with distribution boundaries appended. Actions never bumps source versions or rebuilds during promotion. Reuse the original unexpired preparation artifact. Normal promotion requires unchanged main; explicit recovery admits only exact operational paths in a descendant commit. Runtime/package/build/version/CHANGELOG changes remain forbidden. Preparation refuses already public versions and matching GitHub drafts/releases to prevent accidental rebuilds after partial publication.

After npm accepts an upload, add an append-only receipt asset binding candidate/source/channel, tarball SHA-256 and the actual publisher Run ID/attempt. Receipt validation checks its exact fields/digest and GitHub workflow identity; failed publisher runs remain valid receipt sources because partial uploads survive failure. Matching receipts permit a retry to skip accepted uploads while metadata is unavailable. Receipt assets are supplemental to the sealed candidate inventory, never replacements for payload hashes or evidence of public availability. A lost upload/receipt acknowledgement may require later matching public metadata; ambiguous duplicate-version failures stop rather than force replacement.

Authenticated pagination discovers drafts and exact Release IDs select uploads. Preserve matching empty duplicate drafts and reuse the populated original. Multiple populated releases, foreign/modified notes/assets/receipts, integrity conflicts and tag movement stop. No deletion, unpublish, stage rejection or implicit rollback occurs. Legacy stage-only helpers remain covered for compatibility, but current workflows use OIDC and never receive NPM_STAGE_TOKEN. Initial package bootstrap may be performed by a maintainer using reviewed tarballs and interactive npm authentication/2FA.

## Consequences and regression boundaries

Public GitHub publication can briefly precede npm installability. Verification is therefore independent and repeatable without writes; unified execution exposes separate publish/check jobs. Both a dry-run and acceptance receipt are weaker than public-byte/install verification. GitHub/npm are independent systems with no atomic transaction.

Behavioral tests cover candidate/source fences, immutable payloads, OIDC identity, no-poll publication, partial upload and receipt recovery, read-only propagation checking, source-fenced exact child dispatch/reuse, lost responses, failed-child continuation and foreign content refusal. Native/host evidence retains its limits: simulated models do not establish real-provider or new UI acceptance. Real publication and unified-workflow qualification remain maintainer-operated; artifact expiry stops promotion.
