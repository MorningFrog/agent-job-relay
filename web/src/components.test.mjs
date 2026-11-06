import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";
import { command, taskTitle } from "./view.js";

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
const { PageHeader } = await server.ssrLoadModule("/src/components/PageHeader.jsx");

const header = (props = {}) => renderToStaticMarkup(React.createElement(PageHeader, {
  page: "tasks", connection: "connected", ...props,
}));

test("overview counts current execution and queued work independently of historical records", () => {
  const overview = { active_jobs: [
    { job_id: "starting", status: "launching" },
    { job_id: "executing", status: "running" },
    { job_id: "waiting", status: "queued" },
  ], jobs: [{ job_id: "old", status: "failed" }] };
  const html = header({ overview });
  assert.match(html, /<dt>运行中<\/dt><dd>2<\/dd>/);
  assert.match(html, /<dt>等待启动<\/dt><dd>1<\/dd>/);
  const history = ["failed", "timeout", "orphaned", "completed"].map((status) => ({ status }));
  assert.equal(header({ overview: { ...overview, jobs: [...overview.jobs, ...history] } }), html);
  assert.doesNotMatch(html, /需要检查|最近已完成|通知待处理/);
  const alerts = header({ page: "alerts", overview });
  assert.match(alerts, /<h1>告警与事件<\/h1>/);
  assert.match(alerts, /<dt>运行中<\/dt><dd>2<\/dd>/);
});

test("overview distinguishes unsampled counts from an empty active snapshot", () => {
  assert.match(header({ overview: null, connection: "connecting" }), /<dt>运行中<\/dt><dd>—<\/dd>/);
  assert.match(header({ overview: null }), /<dt>等待启动<\/dt><dd>—<\/dd>/);
  const empty = header({ overview: { active_jobs: [] } });
  assert.match(empty, /<dt>运行中<\/dt><dd>0<\/dd>/);
  assert.match(empty, /<dt>等待启动<\/dt><dd>0<\/dd>/);
});

test("overview retains last-sampled counts while displaying connection loss or reconnection", () => {
  const overview = { active_jobs: [{ status: "running" }] };
  for (const [connection, label] of [["disconnected", "连接已中断"], ["connecting", "正在连接…"], ["connected", "实时更新"]]) {
    const html = header({ overview, connection });
    assert.match(html, /<dt>运行中<\/dt><dd>1<\/dd>/);
    assert.ok(html.includes(label));
  }
  const runtime = header({ page: "runtime", overview });
  assert.match(runtime, /<h1>运行时状态<\/h1>/);
  assert.doesNotMatch(runtime, /当前任务状态/);
});

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
      pending_stage: "<script>checkpoint</script>",
      condition: { kind: "progress", source: "<script>source</script>", trigger: { kind: "stage" }, cooldown_seconds: 60 } }],
    targets: [{ target_id: "target", status: "configured", can_resume: false }],
    visibility: { status: "quiet", advisories: [{ code: "quiet_output" }] },
    activity: { pending_stages: 1 },
  } }));
  for (const text of ["等待运行器应用", "尚无该来源数据", "仅收件箱", "静默不能证明任务卡住", "配置阶段反馈", "配置时长检查点"]) assert.ok(html.includes(text));
  assert.ok(!html.includes("<script>"));
  assert.ok(html.includes("&lt;script&gt;source"));
  assert.ok(html.includes("1 条阶段反馈等待冷却结束") && html.includes("待报阶段：&lt;script&gt;checkpoint"));
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

test("remote launch details show the full submitted workload and distinct execution identities", () => {
  for (const workload of [
    { kind: "exec", program: "python", args: ["train.py", "<unsafe>", "尾".repeat(300)] },
    { kind: "shell", interpreter: "pwsh", script: "Write-Output '<unsafe>'; " + "尾".repeat(300) },
  ]) {
    const remote = { target_id: "pinned-target", intent: { kind: "launch", spec: {
      command: workload, cwd: "C:\\remote\\训练", env: { TOKEN: "hidden-environment" },
    } }, transport: { program: "ssh", args: ["hidden-transport"] } };
    const job = { job_id: "local-job", run_id: "local-run", title: "训练模型", status: "running",
      cwd: "/local/observer", command: { kind: "remote", remote }, command_preview: "truncated preview" };
    const html = renderToStaticMarkup(React.createElement(TaskDetail, {
      selected: job.job_id, detail: { job, observation: { external: {
        run: { job_id: "remote-job", run_id: "remote-run", attempt: 1 },
      } } }, tab: "output", streamName: "stdout",
    }));
    for (const expected of ["提交的远端命令", "本地观察目录", "/local/observer", "远端工作目录",
      "C:\\remote\\训练", "pinned-target", "local-job", "local-run", "remote-job", "remote-run", "尾".repeat(300), "&lt;unsafe&gt;"]) {
      assert.ok(html.includes(expected), expected);
    }
    for (const absent of ["<unsafe>", "hidden-environment", "hidden-transport", "truncated preview"]) {
      assert.ok(!html.includes(absent), absent);
    }
    if (workload.kind === "shell") assert.ok(html.includes("远端解释器") && html.includes("pwsh"));
    assert.equal(command(job), workload.kind === "shell" ? workload.script : [workload.program, ...workload.args].join(" "));
    const pending = renderToStaticMarkup(React.createElement(TaskDetail, {
      selected: job.job_id, detail: { job }, tab: "output", streamName: "stdout",
    }));
    assert.ok(pending.includes("尚未确认"));
    assert.ok(!pending.includes("未提供远端命令"));
  }
});

test("remote attach details disclose missing command metadata and retain the pinned run before sampling", () => {
  const job = { job_id: "observer", status: "running", cwd: "/local/observer",
    command: { kind: "remote", remote: { target_id: "pinned-target", intent: {
      kind: "attach", run: { job_id: "attached-job", run_id: "attached-run", attempt: 1 }, allow_stop: false,
    } } } };
  const html = renderToStaticMarkup(React.createElement(TaskDetail, {
    selected: job.job_id, detail: { job }, tab: "output", streamName: "stdout",
  }));
  for (const expected of ["未提供远端命令（附加观察）", "未提供（附加观察）", "attached-job", "attached-run", "pinned-target", "停止观察"]) {
    assert.ok(html.includes(expected), expected);
  }
  assert.ok(!html.includes("提交的远端命令"));
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

test("goal diagnostics correlate the feedback delivery and escape recorded failures", () => {
  const html = renderToStaticMarkup(React.createElement(TaskDetail, { selected: "job", tab: "feedback", detail: {
    job: { job_id: "job", status: "running" }, notifications: { status: "retry_wait", goal: {
      goal_wait_id: "original-wait", request_id: "original-request", status: "resumed", revision: 7,
      diagnostics: { phase: "feedback", feedback_cycles_remaining: 31,
        delivery: { delivery_id: "original-delivery", job_id: "original-job", run_id: "original-run", status: "retry_wait", attempts: 2, error: "<script>delivery failed</script>" },
        last_attempt: { error: "<script>attempt failed</script>" },
        last_control: { at: 1000, detail: "<script>control failed</script>" },
      },
    } },
  } }));
  for (const text of ["本次中途反馈", "剩余反馈次数 31", "original-wait", "original-request", "original-delivery", "original-job", "original-run", "&lt;script&gt;control failed", "&lt;script&gt;delivery failed", "&lt;script&gt;attempt failed"]) assert.ok(html.includes(text), text);
  assert.ok(!html.includes("<script>"));
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
