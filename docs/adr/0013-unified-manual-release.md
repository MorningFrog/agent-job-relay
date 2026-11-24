# ADR 0013: immutable manual GitHub/npm release promotion

This decision extends ADR 0010. The published npm identities are `agent-job-relay-runtime` and `agent-job-relay-opencode-v1`. Every release starts with an explicitly selected maintainer workflow on `main`.

## Decision

Cargo.toml is the version authority. The local finalizer checks unused-version evidence, synchronizes package and plugin pins, runs the delivery gates and finalizes the reviewed CHANGELOG section. It preserves published sections, dates and tags. Maintainers review and commit before preparing the candidate.

Prepare Release validates the manual event, repository, source revision and version availability. It verifies Linux/Windows native and installed packages and the Linux glibc baseline, then seals source/run/channel and complete payload hashes. Later operations verify the actual successful original run before reading its immutable artifact. Development package runs cannot produce a formal candidate. Candidates and receipts remain bound to their original provider and run.

Preview and independent Check Release have read-only permissions. Stage uses a non-bypass npm token without GitHub write access. It checks for an existing matching stage before staging a tarball, verifies the staged bytes and relies on a maintainer's npm 2FA approval. GitHub finalization is separately selected and requires both complete public npm tarballs and their channels to match before publishing the original assets. There is no cross-registry atomic transaction.

Release All defaults to preparation or reuse and read-only preview. Explicit publication selection then dispatches staging, waits for both human-approved packages to appear on their sealed channel, and dispatches GitHub finalization followed by independent verification. Approval waiting uses read-only credentials outside the child concurrency group. Child dispatch pins the source and exact Run ID, with a stable correlation key to recover an uncertain response without repeating the dispatch. A main revision change fences subsequent dispatch.

Candidate recovery is opt-in and admits only allowlisted release-tooling and documentation changes in a descendant commit. Existing tags, drafts, legacy publication receipts and assets are verified against the original candidate. Conflicting public bytes, channels or ownership stop the operation. An independent check adds isolated clean installation and verifies the original GitHub notes/assets/Tag; bounded read-only retries account for npm propagation. A failed phase leaves original candidate bytes and prior external outcomes intact.
