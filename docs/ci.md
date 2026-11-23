# GitHub Actions CI

[Documentation index](README.md) · [Testing](testing.md) · [Manual release](release.md)

`.github/workflows/ci.yml` runs on pull requests targeting `main`, pushes to `main` and manual dispatch. PRs validate GitHub's merge revision. Push, PR and manual runs have separate concurrency groups, and a new run for the same event and ref supersedes an active one. The workflow uses GitHub-hosted runners and needs no external CI service or publication credentials.

| Check | Coverage |
|---|---|
| Minimum Rust | Rust 1.89.0 compiles all targets with the lockfile on Linux |
| Linux native | Rust 1.99.1 in the pinned Debian Bookworm image. Frontend build/tests, fmt, Clippy, every Cargo native test suite and the release build |
| Windows native | Rust 1.99.1 and MSVC on a fresh Windows runner. The same delivery gates, then PE version, revision and SHA-256 metadata |
| macOS ARM64 | All-target compilation on macOS 15. Runtime, installation and real-host acceptance remain deferred |
| Installed packages | Pack the exact Linux and Windows executables from the current run. Linux and Windows install those same development packages in parallel, check actual Codex and OpenCode loaders with isolated state, and use a simulated OpenCode model. Linux also runs the release fixtures and the Claude scaffold check |

The frontend bundle builds before native checks. Compiler caches are scoped by platform and CI/release purpose. Only pushes to `main` save CI caches. PRs, manual runs and release preparation rerun every applicable check. Same-run Actions artifacts wrap the repository ID, Run ID, commit, producer and complete SHA-256 inventory; consumers verify the inventory before extracting payloads. Artifact uploads use immutable names within each run. A conflicting upload fails.

## Native test scheduling

`node scripts/ci_tests.mjs` discovers every Cargo all-target test suite, compiles the locked test targets once and runs the suites through Cargo with bounded concurrency. It records per-suite results and timings under ignored `.tmp/` and fails if any suite fails. Local delivery uses the standard `cargo test --all-targets` gate as well.

## Main branch protection

Use the `CI required` check from the GitHub Actions app. It succeeds only if minimum Rust, Linux, Windows, macOS compilation and both installed-package checks succeed. After the first successful GitHub Actions PR run, a maintainer can replace the obsolete Woodpecker checks in the main ruleset with this check. Preserve the existing PR and bypass policies. Repository workflow files do not update rulesets or disable an external CI integration.

## Release boundary

Formal preparation uses the separately selected `Prepare Release` workflow on `main`. It checks a finalized unused version, reruns the Linux/Windows gates and Rust 1.89 compilation, tests the downloaded Linux executable on AlmaLinux 9/glibc 2.34, and seals the exact Linux/Windows packages after the two isolated installed-package jobs pass. Release operations require their original successful Prepare Release Run ID. The [release guide](release.md) describes the separate preview, staging, approval, GitHub finalization and public verification steps.
