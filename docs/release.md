# Plugin release candidates

The public source repository is [MorningFrog/agent-job-relay](https://github.com/MorningFrog/agent-job-relay). Release preparation and a self-hosted ChatGPT Desktop/Codex marketplace are authorized; publishing still needs the user's final confirmation. The npm account is awaiting verification and the two unscoped package names are provisional. No npm credentials, registry packages, tags, release or remote push are created by local preparation. Official OpenAI/Anthropic directory submissions remain separate future work.

## Build and verify locally

Use Linux x86_64/glibc, Rust stable/C toolchain, Node.js 20+/npm, Python 3.11+ and readelf. From the repository root:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
python3 scripts/prepare_release.py --output .tmp/release-candidate
npm install --prefix .tmp/opencode-toolchain --cache .tmp/npm-cache --ignore-scripts --no-audit --no-fund opencode-ai@1.19.4 @opencode-ai/plugin@1.19.4 @opencode-ai/sdk@1.19.4 @openai/codex@0.164.0
python3 scripts/check_release.py --release .tmp/release-candidate --opencode .tmp/opencode-toolchain/node_modules/opencode-linux-x64/bin/opencode --codex "$PWD/.tmp/opencode-toolchain/node_modules/.bin/codex"
```

Choose a new output path for another candidate; preparation refuses existing output. It stages only allowlisted files, checks source/native/plugin/dependency versions, runs `npm pack --ignore-scripts`, bundles the Linux executable with a SHA-256/minimum-glibc manifest, and produces the two npm tarballs, self-hosted marketplace ZIP, release.json and SHA256SUMS. It includes no selected state profile, credentials, databases or install scripts. Hashes establish integrity, not publisher identity. Local builds can require a newer glibc than release builds: inspect release.json rather than assuming distribution portability.

The checker installs the real tarballs into an isolated prefix, runs the packaged native program, verifies retained job history after package relocation and refuses a corrupt payload before execution. The actual OpenCode loader/MCP/session API and completed agent-loop response use a simulated model. The actual Codex marketplace loader discovers 20 tools using a loopback registry for the unpublished runtime; only the registry transport is replaced in the test copy. It performs no model turn, real Desktop contact or public registry write. Reports remain under ignored `.tmp/`.

## Manually trigger GitHub Actions

After reviewing and pushing the changes, open **Actions → Release candidate → Run workflow**. Select the source branch/tag, enter its exact numeric version and choose a destination:

| Destination | Result |
|---|---|
| `artifacts` (default) | Run all native gates and both actual-loader checks, then upload a downloadable candidate artifact for 14 days |
| `draft` | Do the same, then create a new prerelease draft with those exact files and the selected commit; publishing that draft remains a manual GitHub action |

The workflow has only `workflow_dispatch`: pushes, PRs and tag pushes never trigger a release. It builds on Ubuntu 22.04 for a glibc 2.35 baseline, checks the exact requested source version and retains existing releases/assets. An existing tag must point to the selected commit; no tag is moved. Default workflow permissions are read-only; only the requested draft job gets repository-content write access. No custom token or npm secret is needed. This definition must run on GitHub before its runner compatibility is established.

Ordinary **CI** continues on push/PR/manual dispatch. Linux runs the full behavior suite, packs candidates and checks the installed OpenCode package; macOS/Windows compile and run portable library tests only. These checks do not assert native macOS/Windows runtime support. The Windows `Artifact.executable` compile error is addressed by explicitly rejecting unsupported executable installation; only the debug-image-heavy installation test command gets a longer deadline, while IPC/workload deadlines remain bounded.

## Confirm names and publish npm later

Before npm publication, confirm ownership of `agent-job-relay-runtime` and `agent-job-relay-opencode-v1` (or select owned scoped names), update all manifest/MCP/launcher/skill pins together, and rerun the candidate checks. Review the tarball inventories, license, publisher identity, native build provenance, minimum glibc, supported hosts, data handling and release notes. Restore npm account verification and enable the account's required second factor; do not commit credentials. Trusted publishing can be configured separately after package ownership is established.

With explicit publication approval, publish the reviewed runtime tarball first, then the reviewed OpenCode tarball using the experimental `next` tag. The exact dependency/runtime version must exist before consumers use either online installation path. Do not silently select `latest`, reuse a published version, rebuild different bytes after review or claim official marketplace approval. After both packages are available, verify installation from the public registry in a clean profile; only then advertise the normal online commands below as available.

## Consumer install after publication

OpenCode V1 `opencode.json`:

```json
{
  "plugin": ["agent-job-relay-opencode-v1@0.1.6"]
}
```

ChatGPT Desktop/Codex self-hosted marketplace:

```bash
codex plugin marketplace add MorningFrog/agent-job-relay
```

Restart Desktop, select **AgentJobRelay** in Plugins and install/enable the plugin in a new local chat. CLI-only users can run `codex plugin add agent-job-relay@agent-job-relay` for tool/inbox use. This is the documented [self-hosted catalog format](https://developers.openai.com/plugins/build/plugins), not OpenAI's official directory. Node/npm must be available to the host and terminal; first use downloads the pinned runtime from npm with scripts disabled. The source marketplace is discoverable before npm publication but its MCP will fail until the exact runtime is available; do not call that a working online installation.

The npm runtime keeps stable `codex` and `opencode` state profiles outside caches. It does not inherit a chat identity, migrate a previous offline profile, edit personal configuration, stop daemons or purge data. Review an existing installation's recorded state before deliberately choosing it with AJR_STATE_DIR. See [installation](installation.md), [host plugins](host-plugins.md) and [manual acceptance](manual-testing.md). Source/native versions, supported host versions and honest evidence boundaries govern publication; backup/export/retention features are not prerequisites for this experimental release.
