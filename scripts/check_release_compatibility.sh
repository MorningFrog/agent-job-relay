#!/bin/sh
# Run only in the pinned AlmaLinux 9/glibc 2.34 workflow step.
set -eu
cd "$(dirname "$0")/.."
[ "$(getconf GNU_LIBC_VERSION)" = 'glibc 2.34' ]
binary="$PWD/target/release/agent-job-relay"
"$binary" --version
"$binary" --help >/dev/null
# Exercise actual SQLite/config initialization without starting a daemon or job.
"$binary" --state-dir "$PWD/.tmp/release-glibc-profile" data check
mkdir -p .tmp
digest=$(sha256sum "$binary")
digest=${digest%% *}
printf '{"glibc":"2.34","sha256":"%s","result":"passed"}\n' "$digest" > .tmp/release-compatibility.json
