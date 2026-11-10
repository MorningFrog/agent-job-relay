# Active workflows

Only active workflow YAML belongs directly in this directory. `release` is the human-facing entry; `_ci-*` / `_release-*` are internal workflows, normally hidden by the deployment UI customization and available through “Show internal workflows”. The underscore does not disable execution:

- `_ci-linux.yaml`, `_ci-windows.yaml`, `_ci-package-windows-test.yaml`, `_ci-windows-packages.yaml`: the four required automatic verification workflows.
- `_release-preflight.yaml`, `_release-linux.yaml`, `_release-windows.yaml`, `_release-package.yaml`, `_release-windows-packages.yaml`, `_release-seal.yaml`: the six dependent workflows for explicit manual preparation.

- `release.yaml`: the single explicit main/manual Release All entry selected from Woodpecker Web UI, with default dry-run, original-candidate reuse and recovery.

Publication is coordinated deployment-side through the local project adapter; no separate temporary `publish-*.yaml` workflows are needed. It supports dry-run, original-candidate reuse and explicit human npm approval. Shared workflow templates belong in deployment tooling; project commands/helpers stay in `scripts/`.

Reusable support files use an `_` or `.` prefix and a non-workflow extension, or a separate directory outside the configured workflow path. Woodpecker 3.20.1 filters YAML by extension, so an underscore or dot alone does **not** make a YAML file inactive. Do not hide retired executable workflows by merely renaming them.
