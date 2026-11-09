# Active workflows

Only active workflow YAML belongs directly in this directory:

- `linux.yaml`, `windows.yaml`, `package-windows-test.yaml`, `windows-packages.yaml`: the four required automatic verification workflows.
- `release-preflight.yaml`, `release-linux.yaml`, `release-windows.yaml`, `release-package.yaml`, `release-windows-packages.yaml`, `release-seal.yaml`: the six dependent workflows for explicit manual preparation.

Publication uses the deployment-side Release entry point and the local project adapter, not separate temporary `publish-*.yaml` workflows. It supports dry-run, original-candidate reuse and explicit human npm approval. Shared workflow templates belong in deployment tooling; project commands/helpers stay in `scripts/`.

Reusable support files use an `_` or `.` prefix and a non-workflow extension, or a separate directory outside the configured workflow path. Woodpecker 3.20.1 filters YAML by extension, so an underscore or dot alone does **not** make a YAML file inactive. Do not hide retired executable workflows by merely renaming them.
