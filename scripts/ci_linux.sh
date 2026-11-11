#!/bin/sh
# Account-free Linux CI. Run inside the image in .woodpecker/_ci-linux.yaml.
set -eu
cd "$(dirname "$0")/.."

# A dedicated, single-workflow Agent supplies this project-only Docker volume.
# Keep the ordinary checkout disposable; never persist test profiles. Result reuse uses independent Server evidence.
ci_cache_root="$PWD/.tmp/ci"
if [ -d /ci-cache ]; then
  ci_cache_root=/ci-cache/node24-3d27e5c1-rust1.99.1-msrv1.89-v1
  mkdir -p "$ci_cache_root/target"
  echo "Persistent compiler and download cache: $ci_cache_root"
else
  echo 'No persistent volume: using disposable workspace cache.'
fi
export CARGO_HOME="$ci_cache_root/cargo"
export RUSTUP_HOME="$ci_cache_root/rustup"
export npm_config_cache="$ci_cache_root/npm"
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_BUILD_JOBS="${AJR_CI_BUILD_JOBS:-$(nproc)}"
export CARGO_INCREMENTAL=1
export PYTHONDONTWRITEBYTECODE=1

case "${1:-}" in
  toolchain)
    if [ -d /ci-cache ]; then
      # Fixtures hard-link native images into .tmp: keep target on the same
      # filesystem as the checkout, rather than symlinking across mounts.
      if [ -e target ] || [ -L target ]; then
        echo 'Persistent CI restore requires a fresh checkout without target/.' >&2
        exit 1
      fi
      mkdir target
      cp -a --reflink=auto "$ci_cache_root/target/." target/
    fi
    for tool in node npm python3 git curl cc readelf; do
      command -v "$tool"
    done
    node --version
    npm --version
    python3 --version
    echo "Cargo build jobs: $CARGO_BUILD_JOBS"
    mkdir -p .tmp/ci
    if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
      curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs -o .tmp/ci/rustup-init.sh
      sh .tmp/ci/rustup-init.sh -y --no-modify-path --profile minimal --default-toolchain none
    fi
    if ! rustup run 1.99.1 cargo --version >/dev/null 2>&1 || ! rustup run 1.99.1 cargo clippy --version >/dev/null 2>&1 || ! rustup run 1.99.1 cargo fmt --version >/dev/null 2>&1; then
      rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy --no-self-update
    fi
    if ! rustup run 1.89.0 cargo --version >/dev/null 2>&1; then
      rustup toolchain install 1.89.0 --profile minimal --no-self-update
    fi
    rustup default 1.99.1
    rustc +1.99.1 --version
    rustc +1.89.0 --version
    ;;
  frontend)
    npm ci --prefix web --ignore-scripts --no-audit --no-fund --prefer-offline
    node .ci/ci_checks.mjs frontend-linux
    npm run build --prefix web
    ;;
  frontend-tests)
    npm test --prefix web
    node --test scripts/test_online_update.mjs scripts/test_online_uninstall.mjs scripts/test_ci_artifacts.mjs scripts/test_release_artifacts.mjs scripts/test_ci_inputs.mjs scripts/test_opencode_ready.mjs
    ;;
  msrv)
    python3 -m unittest discover -s scripts -p test_ci_cache.py -v
    if [ -d /ci-cache ]; then
      python3 scripts/ci_cache.py restore "$ci_cache_root/inputs.json"
    fi
    cargo +1.89.0 check --all-targets --locked --target-dir target/msrv
    ;;
  native)
    cargo +1.99.1 fmt --check
    cargo +1.99.1 clippy --all-targets --locked -- -D warnings
    node .ci/ci_checks.mjs native-linux
    cargo +1.99.1 build --release --locked
    target/release/agent-job-relay --help
    ;;
  packages)
    python3 scripts/release_actions.py source --allow-unreleased
    python3 -m unittest discover -s scripts -p 'test_release*.py' -v
    npm install --prefix .tmp/opencode-toolchain --ignore-scripts --no-audit --no-fund --prefer-offline opencode-ai@1.19.4 @opencode-ai/plugin@1.19.4 @opencode-ai/sdk@1.19.4
    python3 scripts/prepare_release.py --allow-unreleased --output .tmp/release-candidate
    python3 scripts/check_release.py --release .tmp/release-candidate --opencode .tmp/opencode-toolchain/node_modules/opencode-linux-x64/bin/opencode
    python3 scripts/check_claude_scaffold.py --binary "$PWD/target/release/agent-job-relay"
    ;;
  cache)
    if [ -d /ci-cache ] && [ -d target ]; then
      # Store compiler artifacts only, including useful work from failed gates.
      # Package/profile state is recreated; checks use committed Server evidence.
      cp -au --reflink=auto target/. "$ci_cache_root/target/"
      python3 scripts/ci_cache.py record "$ci_cache_root/inputs.json"
    fi
    ;;
  *)
    echo "Usage: sh scripts/ci_linux.sh {toolchain|frontend|frontend-tests|msrv|native|packages|cache}" >&2
    exit 2
    ;;
esac
