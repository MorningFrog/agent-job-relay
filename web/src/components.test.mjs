import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";
import { taskTitle } from "./view.js";

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

test("named task details retain the purpose, original ID and complete escaped command", () => {
  const script = "printf '<script>untrusted</script>'; " + "echo detail; ".repeat(100);
  const job = {
    job_id: "original-job-id",
    title: "检查训练进度 <实验 A>",
    status: "completed",
    command: { kind: "shell", script },
    cwd: "project",
    created_at: 1,
  };
  const html = renderToStaticMarkup(React.createElement(TaskDetail, {
    selected: job.job_id, detail: { job }, tab: "output", streamName: "stdout",
  }));
  assert.ok(html.includes("<h2>检查训练进度 &lt;实验 A&gt;</h2>"));
  assert.ok(html.includes("original-job-id"));
  assert.ok(html.includes(script.replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll("'", "&#x27;")));
  assert.ok(!html.includes("<script>"));
  assert.equal(taskTitle(job), job.title);
});

test("unnamed legacy jobs have bounded project hints without inline script bodies", () => {
  for (const preview of [
    "/usr/bin/python3 -c " + "large_payload".repeat(50),
    "python3 - <<'PY'\nprint('large payload')\nPY",
  ]) {
    assert.equal(taskTitle({ command_preview: preview, cwd: "/work/project" }), "python3 脚本 · project");
  }
  const exec = { command: { kind: "exec", program: "C:\\tools\\cargo.exe", args: ["test", "--all-targets"] }, cwd: "C:\\work\\项目\\" };
  assert.equal(taskTitle(exec), "cargo.exe test --all-targets · 项目");
  assert.equal(taskTitle({ command_preview: "set -eu\necho payload", cwd: "/work/project" }), "Shell 脚本 · project");
  const long = taskTitle({ command_preview: "测".repeat(1000), cwd: "/" + "项".repeat(100) });
  assert.ok(Array.from(long).length <= 75);
  assert.ok(long.includes("…"));
  assert.equal(taskTitle({}), "后台任务");
});

test("notification diagnostics show recovery and goal attention without claiming Agent execution", () => {
  const html = renderToStaticMarkup(React.createElement(TaskDetail, {
    selected:"job", tab:"output", streamName:"stdout", detail:{
      job:{job_id:"job", status:"completed"},
      notifications:{status:"dead_letter", counts:{dead_letter:1},
        latest_problem:{delivery_id:"delivery", failure_kind:"rebind_required", error:"<script>host changed</script>"},
        goal:{status:"attention",revision:4,host_pause_required:true,detail:"native pause needed"}},
      deliveries:{deliveries:[{id:"delivery",status:"dead_letter",attempts:2}]}
    }
  }));
  assert.ok(html.includes("通知需要处理"));
  assert.ok(html.includes("connect --recover"));
  assert.ok(html.includes("native pause needed"));
  assert.ok(html.includes("Agent 是否继续仍待核实"));
  assert.ok(html.includes("&lt;script&gt;host changed&lt;/script&gt;"));
  assert.ok(!html.includes("<script>"));
});
