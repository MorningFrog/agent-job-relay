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
const { StorageNotice } = await server.ssrLoadModule("/src/components/RuntimeStatus.jsx");
const { FeedbackStatus } = await server.ssrLoadModule("/src/components/FeedbackStatus.jsx");
const { recoveryCommand } = await server.ssrLoadModule("/src/components/NotificationStatus.jsx");
const { RuleForm } = await server.ssrLoadModule("/src/components/RuleForm.jsx");
const { TaskProgress } = await server.ssrLoadModule("/src/components/TaskProgress.jsx");
test("remote feedback subscriptions expose alert-only controls and escape original remote evidence", () => {
  const form = renderToStaticMarkup(React.createElement(RuleForm, { externalKind: "remote", preset: "remote" }));
  assert.ok(form.includes('value="remote_alert"'));
  assert.ok(form.includes("远端规则 ID"));
  assert.ok(!form.includes("停止任务并记录告警"));
  const card = renderToStaticMarkup(React.createElement(FeedbackStatus, { active: true, external: "remote", feedback: { status: "configured", rules: [] } }));
  assert.ok(card.includes("配置远端告警反馈"));
  const progress = renderToStaticMarkup(React.createElement(TaskProgress, { detail: {
    job: { command: { kind: "remote" } }, observation: { remote_feedback: { alerts: 1, coalesced: 2, suppressed: 0,
      milestones: [{ rule_id: "rule", at: 1000, evidence: { remote: { event_id: "original-event" }, source_evidence: { stage: "<script>checkpoint</script>" } } }] } },
  } }));
  assert.ok(progress.includes("original-event") && progress.includes("&lt;script&gt;checkpoint"));
  assert.ok(!progress.includes("<script>"));
});
test("storage warnings retain sample time, admission scope and escaped maintenance errors", () => {
  const html=renderToStaticMarkup(React.createElement(StorageNotice,{capacity:{disk_pressure:{status:"critical",available_bytes:1,sampled_at:1000,warn_below_bytes:1024,stop_new_jobs_below_bytes:64},backup_health:"overdue",last_error:"<script>backup</script>"}}));
  for(const text of ["新任务准入受限","已有任务","备份已超过配置间隔",new Date(1000).toLocaleString(),"&lt;script&gt;backup&lt;/script&gt;"])assert.ok(html.includes(text));
  assert.ok(!html.includes("<script>"));
});

test("feedback card distinguishes configuration, applied revisions, sources and notification limits", () => {
  const html = renderToStaticMarkup(React.createElement(FeedbackStatus, { active: true, feedback: {
    status: "inbox_only", configured_rules: 1, notifying_rules: 1,
    rules: [{ rule_id: "rule", enabled: true, application: "pending", sample_observed: false, can_trigger_again: true,
      condition: { kind: "progress", source: "<script>source</script>", trigger: { kind: "stage" }, cooldown_seconds: 60 } }],
    targets: [{ target_id: "target", status: "configured", can_resume: false }],
    visibility: { status: "quiet", advisories: [{ code: "quiet_output" }] },
  } }));
  for (const text of ["等待运行器应用", "尚无该来源数据", "仅收件箱", "静默不能证明任务卡住", "配置阶段反馈", "配置时长检查点"]) assert.ok(html.includes(text));
  assert.ok(!html.includes("<script>"));
  assert.ok(html.includes("&lt;script&gt;source"));
});

test("recovery command preserves argv quoting for the original profile", () => {
  const command = { program: "/tools/relay", args: ["--state-dir", "/state/user's $(data)", "connect", "--recover"] };
  assert.ok(recoveryCommand(command, "posix").includes("'/state/user'\\''s $(data)'"));
  assert.ok(recoveryCommand(command, "powershell").startsWith("& '/tools/relay'"));
  assert.ok(recoveryCommand(command, "powershell").includes("'/state/user''s $(data)'"));
});

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
    selected:"job", tab:"feedback", streamName:"stdout", detail:{
      job:{job_id:"job", status:"completed"},
      notifications:{status:"dead_letter", counts:{dead_letter:1},
        latest_problem:{delivery_id:"delivery", failure_kind:"rebind_required", error:"<script>host changed</script>"},
        goal:{status:"attention",revision:4,host_pause_required:true,detail:"native pause needed"}},
      deliveries:{deliveries:[{id:"delivery",status:"dead_letter",attempts:2}]}
    }
  }));
  assert.ok(html.includes("通知需要处理"));
  assert.ok(html.includes("connect --recover"));
  assert.ok(html.includes("OpenCode"));
  assert.ok(html.includes("relay_recover"));
  assert.ok(html.includes("native pause needed"));
  assert.ok(html.includes("Agent 是否继续仍待核实"));
  assert.ok(html.includes("&lt;script&gt;host changed&lt;/script&gt;"));
  assert.ok(!html.includes("<script>"));
});

test("goal continuation diagnostics distinguish re-pause, notification and host recovery", () => {
  for (const [action, expected] of [
    ["report_then_pause", "原聊天汇报中途结果后需再次暂停"],
    ["confirm_pause", "原目标已暂停，核对已有暂停回执"],
    ["await_notification", "目标已恢复，等待最终通知确认"],
    ["host_control", "请回到原聊天检查目标状态并处理恢复"],
  ]) {
    const html = renderToStaticMarkup(React.createElement(TaskDetail, {
      selected: "job", tab: "feedback", streamName: "stdout", detail: {
        job: { job_id: "job", status: "running" },
        notifications: { status: "pending", counts: {}, goal: {
          status: "resumed", revision: 7, continuation: { action },
        } },
      },
    }));
    assert.ok(html.includes(expected));
    assert.ok(html.includes("Agent 是否继续仍待核实"));
  }
});

test("task detail opens on output with concise attention and deferred execution metadata", () => {
  const detail = {
    job: {job_id:"original-job", run_id:"original-run", title:"运行训练", status:"running", created_at:1,
      command:{kind:"shell",script:"large command ".repeat(1000)}, cwd:"/project", run:{started_at:1}},
    observation:{stdout:{text:"live terminal output",total_bytes:20},progress:{sources:{training:{source:"training",stage:"training",step:2,total:10}}}},
    notifications:{status:"dead_letter",counts:{dead_letter:1},latest_problem:{delivery_id:"original-delivery",error:"failure diagnostics"}},
    feedback:{status:"configured",configured_rules:1,notifying_rules:1},
  };
  const props={selected:"original-job",detail,tab:"output",streamName:"stdout",grace:5};
  const output=renderToStaticMarkup(React.createElement(TaskDetail,props));
  assert.ok(output.includes("live terminal output"));
  assert.ok(output.includes("通知需要处理"));
  assert.ok(!output.includes("failure diagnostics"));
  assert.ok(!output.includes('aria-label="中途反馈状态"'));
  assert.ok(output.indexOf("live terminal output") < output.indexOf("large command"));
  assert.match(output,/<details class="execution-info">/);
  assert.match(output,/role="tab" id="task-tab-output" aria-selected="true"/);
  const pending = renderToStaticMarkup(React.createElement(TaskDetail,{...props,detail:{...detail,notifications:{goal:{host_pause_required:true}}}}));
  assert.match(pending,/detail-feedback-link needs-attention/);
  assert.ok(pending.includes("Goal 等待交接"));
  const progress=renderToStaticMarkup(React.createElement(TaskDetail,{...props,tab:"progress"}));
  assert.ok(progress.includes("training 进度"));assert.ok(progress.includes("进度与里程碑"));
  const feedback=renderToStaticMarkup(React.createElement(TaskDetail,{...props,tab:"feedback"}));
  assert.ok(feedback.includes("failure diagnostics"));assert.ok(feedback.includes('aria-label="中途反馈状态"'));
});

test("queued tasks show their position and deadline separately from execution", () => {
  const job = {job_id:"queued-job",status:"queued",created_at:Date.now(),
    queue:{position:3,expires_at:Date.now()+60000},command:{kind:"exec",program:"build"},run:{started_at:null}};
  const html=renderToStaticMarkup(React.createElement(TaskDetail,{selected:job.job_id,detail:{job},tab:"output",streamName:"stdout",grace:5}));
  assert.ok(html.includes("排队第 3 位"));
  assert.ok(html.includes("截止"));
  assert.ok(html.includes("尚未启动运行器"));
  assert.ok(html.includes("排队时间不计入运行超时"));
  const status=renderToStaticMarkup(React.createElement(RuntimeStatus,{connection:"connected",overview:{active_jobs:[job,{status:"running"},{status:"launching"}],health:{bounded_queue:{execution_limit:8}}}}));
  assert.match(status,/运行中任务<\/dt><dd>2 \/ 8<\/dd>/);
  assert.match(status,/等待启动的任务<\/dt><dd>1<\/dd>/);
});
