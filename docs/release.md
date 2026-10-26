# Plugin delivery and publication policy

Version 0.1.3 is an unreleased development milestone. The user's 2026-10-26 scope
revision cancels independent OS installers/package repositories in favor of
Agent plugin delivery. See TODO.md; the old system-package roadmap is superseded.

One native Rust/MCP runtime remains shared across host integrations. ChatGPT
Desktop/Codex, Claude Code and OpenCode need their own plugin formats, installation
checks and session/wake adapters. A plugin installation must hide tool paths from
normal prompts while preserving histories and other plugins. Linux remains the
only implemented runtime; plugin packaging does not establish other OS support.

The desired future channels are each Agent's official supported plugin platform
or installation mechanism. Official listing/review, uploading a package and
actually becoming discoverable are separate milestones. Check each host's current
rules and eligibility, including OpenAI's local-MCP approval requirements; do not
invent accepted listings. OpenCode's supported npm/plugin installation mechanism
does not itself imply an official marketplace endorsement.

**During current development, do not publish or submit.** Do not create remote
repositories, push, upload plugins, publish registry packages, configure release
credentials or start automatic publication. These require a later explicit request.
Local source builds, isolated plugin installation and real-loader checks are
permitted. The local install command is development tooling, not a public installer.

Before a future authorized plugin release: pass native runtime, migration and
owned-file/data-retention gates; qualify actual host tool discovery and the
advertised wake behavior; document supported host versions; test update/cache
and uninstall; then prepare platform-specific review materials and protected
artifact provenance. Installing a plugin alone never proves automatic resume.
