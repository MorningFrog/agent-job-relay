// Project bootstrap is separate from global health and non-idempotent sessions.
import assert from "node:assert/strict";
import { pathToFileURL } from "node:url";

export async function waitForOpenCodeProject(base, { timeoutMs = 90000 } = {}) {
  const started = performance.now();
  try {
    const response = await fetch(new URL("/config", base), { signal: AbortSignal.timeout(timeoutMs) });
    assert(response.ok, `HTTP ${response.status}`);
    const config = await response.json();
    assert.equal(config.model, "relayfixture/relay-fixture", "unexpected isolated project configuration");
    return { opencode_project: "ready", seconds: (performance.now() - started) / 1000 };
  } catch (error) {
    throw new Error(`OpenCode project bootstrap failed before session creation: ${error.message}`, { cause: error });
  }
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  console.log(JSON.stringify(await waitForOpenCodeProject(process.argv[2])));
}
