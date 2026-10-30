@echo off
setlocal DisableDelayedExpansion
call ajr.cmd update --help >nul 2>nul
if errorlevel 1 (
  echo Install the CLI first: npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@0.1.14 (then run ajr update) 1>&2
  exit /b 1
)
npx --yes --ignore-scripts --registry=https://registry.npmjs.org --package=agent-job-relay-runtime@0.1.14 agent-job-relay codex %*
