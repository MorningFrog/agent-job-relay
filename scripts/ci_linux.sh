#!/bin/sh
# Account-free Linux CI. Run inside the image in .woodpecker/linux.yaml.
set -eu
cd "$(dirname "$0")/.."

# Keep toolchains and all generated state in the workflow's shared workspace.
export CARGO_HOME="$PWD/.tmp/ci/cargo"
export RUSTUP_HOME="$PWD/.tmp/ci/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_BUILD_JOBS=4
export PYTHONDONTWRITEBYTECODE=1

case "${1:-}" in
  toolchain)
    for tool in node npm python3 git curl cc readelf; do
      command -v "$tool"
    done
    node --version
    npm --version
    python3 --version
    mkdir -p .tmp/ci
    curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs -o .tmp/ci/rustup-init.sh
    sh .tmp/ci/rustup-init.sh -y --no-modify-path --profile minimal --default-toolchain 1.99.1 --component rustfmt --component clippy
    rustup toolchain install 1.89.0 --profile minimal
    rustc +1.99.1 --version
    rustc +1.89.0 --version
    ;;
  frontend)
    npm ci --prefix web --ignore-scripts --no-audit --no-fund
    npm test --prefix web
    npm run build --prefix web
    node --test scripts/test_online_update.mjs scripts/test_online_uninstall.mjs
    ;;
  msrv)
    cargo +1.89.0 check --all-targets --locked --target-dir target/msrv
    ;;
  native)
    cargo +1.99.1 fmt --check
    cargo +1.99.1 clippy --all-targets --locked -- -D warnings
    cargo +1.99.1 test --all-targets --locked
    cargo +1.99.1 build --release --locked
    cargo +1.99.1 run --locked -- --help
    ;;
  packages)
    python3 scripts/release_actions.py source --allow-unreleased
    python3 -m unittest discover -s scripts -p test_release.py -v
    npm install --prefix .tmp/opencode-toolchain --cache .tmp/npm-cache --ignore-scripts --no-audit --no-fund opencode-ai@1.19.4 @opencode-ai/plugin@1.19.4 @opencode-ai/sdk@1.19.4
    python3 scripts/prepare_release.py --allow-unreleased --output .tmp/release-candidate
    python3 scripts/check_release.py --release .tmp/release-candidate --opencode .tmp/opencode-toolchain/node_modules/opencode-linux-x64/bin/opencode
    python3 scripts/check_claude_scaffold.py --binary "$PWD/target/release/agent-job-relay"
    ;;
  *)
    echo "Usage: sh scripts/ci_linux.sh {toolchain|frontend|msrv|native|packages}" >&2
    exit 2
    ;;
esac
