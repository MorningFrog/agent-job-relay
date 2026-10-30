@echo off
setlocal DisableDelayedExpansion
npx --yes --ignore-scripts --registry=https://registry.npmjs.org --package=agent-job-relay-runtime@0.1.13 agent-job-relay codex %*
