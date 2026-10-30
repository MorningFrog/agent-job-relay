import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";

const server = await createServer({
  root: fileURLToPath(new URL("../", import.meta.url)),
  server: { middlewareMode: true, hmr: false, watch: null },
  appType: "custom",
});
test.after(() => server.close());
const { TaskDetail } = await server.ssrLoadModule(
  "/src/components/TaskDetail.jsx",
);
const { RuntimeStatus } = await server.ssrLoadModule(
  "/src/components/RuntimeStatus.jsx",
);

test("split detail components render output, editable rules and events from the runtime contract", () => {
  const job = {
    job_id: "a",
    status: "running",
    created_at: 1,
    cwd: "fixture",
    command: { kind: "exec", program: "fixture", args: [] },
    run: { started_at: 1 },
  };
  const detail = {
    job,
    rules: [],
    events: [
      {
        event_id: "event",
        kind: "started",
        at: 2,
        payload: { workload_pid: 123 },
      },
    ],
    observation: { stdout: { text: "retained output", total_bytes: 15 } },
  };
  const props = {
    selected: "a",
    detail,
    streamName: "stdout",
    grace: 5,
    busy: false,
    form: null,
  };
  const output = renderToStaticMarkup(
    React.createElement(TaskDetail, { ...props, tab: "output" }),
  );
  assert.match(output, /retained output/);
  assert.match(output, /停止任务/);
  const rules = renderToStaticMarkup(
    React.createElement(TaskDetail, { ...props, tab: "rules", form: {} }),
  );
  assert.match(rules, /class="rule-form"/);
  assert.match(rules, /value="no_output"/);
  const events = renderToStaticMarkup(
    React.createElement(TaskDetail, { ...props, tab: "events" }),
  );
  assert.match(events, /workload_pid/);
  assert.equal(
    renderToStaticMarkup(
      React.createElement(TaskDetail, { ...props, selected: null }),
    ),
    "",
  );
});

test("runtime component shows health sampling time separately from newer task samples", () => {
  const healthAt = 1000,
    taskAt = 1000000;
  const html = renderToStaticMarkup(
    React.createElement(RuntimeStatus, {
      connection: "connected",
      overview: {
        sampled_at: taskAt,
        active_jobs: [],
        health: { sampled_at: healthAt, version: "fixture", store: "ok" },
      },
    }),
  );
  assert.ok(html.includes(new Date(healthAt).toLocaleString()));
  assert.ok(!html.includes(new Date(taskAt).toLocaleString()));
});
