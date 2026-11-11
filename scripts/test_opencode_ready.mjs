import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { waitForOpenCodeProject } from "./opencode_ready.mjs";

test("global health does not authorize a session until project bootstrap completes", async () => {
  const requests = [];
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const server = createServer(async (request, response) => {
    requests.push(`${request.method} ${request.url}`);
    if (request.url === "/config") await gate;
    response.setHeader("Content-Type", "application/json");
    response.end(JSON.stringify(request.url === "/config" ? { model: "relayfixture/relay-fixture" } : {}));
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  try {
    await (await fetch(base + "/global/health")).json();
    let ready = false;
    const bootstrap = waitForOpenCodeProject(base).then(result => { ready = true;return result; });
    while (requests.length < 2) await new Promise(resolve => setImmediate(resolve));
    assert.equal(ready, false);
    release();
    assert.equal((await bootstrap).opencode_project, "ready");
    await (await fetch(base + "/session", { method: "POST" })).json();
    assert.deepEqual(requests, ["GET /global/health", "GET /config", "POST /session"]);
  } finally { release();server.closeAllConnections();await new Promise(resolve => server.close(resolve)); }
});

test("failed or stalled bootstrap never creates or retries a session", async () => {
  for (const failure of ["stalled", "http", "wrong-project"]) {
    const requests = [];
    const server = createServer((request, response) => {
      requests.push(`${request.method} ${request.url}`);
      if (failure === "stalled") return;
      response.writeHead(failure === "http" ? 503 : 200, { "Content-Type": "application/json" });
      response.end(JSON.stringify({ model: "unexpected" }));
    });
    await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
    try {
      await assert.rejects(waitForOpenCodeProject(`http://127.0.0.1:${server.address().port}`, { timeoutMs: failure === "stalled" ? 200 : 5000 }), /before session creation/);
      assert.deepEqual(requests, ["GET /config"]);
    } finally { server.closeAllConnections();await new Promise(resolve => server.close(resolve)); }
  }
});
