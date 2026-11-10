export const terminal = new Set([
  "completed",
  "failed",
  "cancelled",
  "timeout",
  "orphaned",
]);
export const names = {
  queued: "等待启动",
  launching: "启动中",
  running: "运行中",
  completed: "已完成",
  failed: "失败",
  cancelled: "已停止",
  timeout: "超时",
  orphaned: "结果未知",
};
export const phaseNames = {
  running: "运行中",
  graceful: "等待温和退出",
  forced: "已强制结束",
  finished: "已结束",
};
export const date = (n) => (n ? new Date(n).toLocaleString() : "—");
export const age = (n) =>
  n == null
    ? "尚无输出"
    : `${Math.max(0, Math.floor((Date.now() - n) / 1000))} 秒前`;
export const elapsed = (job) => {
  const seconds = Math.max(
    0,
    Math.floor(
      ((job.run?.ended_at || Date.now()) -
        (job.run?.started_at || job.created_at)) /
        1000,
    ),
  );
  return seconds < 60
    ? `${seconds} 秒`
    : `${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒`;
};
const commandText = (spec) => spec?.kind === "shell" ? spec.script :
  [spec?.program, ...(spec?.args || [])].filter(Boolean).join(" ");
export const isWorkflow = (kind) => kind === "github_actions" || kind === "woodpecker";
export const command = (job) => {
  const spec = job?.command;
  if (spec?.kind === "remote") {
    return spec.remote?.intent?.kind === "launch"
      ? commandText(spec.remote.intent.spec.command)
      : "未提供远端命令（附加观察）";
  }
  if (spec?.kind === "github_actions") {
    return `GitHub Actions ${spec.workflow.repository} · ${spec.workflow.run_id} / ${spec.workflow.attempt}`;
  }
  if (spec?.kind === "woodpecker") {
    return `Woodpecker ${spec.pipeline.repository} · pipeline #${spec.pipeline.pipeline_number} · ${spec.pipeline.server_url}`;
  }
  return spec ? commandText(spec) : job?.command_preview || "";
};

const basename = (path) => path?.replace(/[\\/]+$/, "").split(/[\\/]/).pop();
const shorten = (text, limit) => {
  const chars = Array.from(text);
  return chars.length > limit ? `${chars.slice(0, limit - 1).join("")}…` : text;
};
export const taskTitle = (job) => {
  if (job?.title?.trim()) return job.title.trim();
  // Legacy jobs have no purpose label. This is only a command hint for display.
  const preview = command(job)?.trim() || "";
  const first = preview.split(/\r?\n/, 1)[0];
  const [program = "", ...args] = first.split(/\s+/);
  const name = basename(program);
  let hint = [name, ...args].join(" ");
  if (
    /^(?:python[\d.]*|node|ruby|perl|[a-z]*sh|pwsh|powershell)(?:\.exe)?$/i.test(name) &&
    (args.some((a) => /^-(?:[a-z]*c|e|Command|EncodedCommand)$/i.test(a)) ||
      args[0] === "-" || first.includes("<<"))
  ) {
    hint = `${name} 脚本`;
  } else if (
    /^(?:set|export|cd|printf|echo|for|while|if|trap)\b/.test(first) ||
    first.startsWith("#!")
  ) {
    hint = "Shell 脚本";
  } else {
    hint = hint.split(/\s*(?:&&|\|\||[;|]|<<)\s*/, 1)[0];
  }
  const project = basename(job?.cwd);
  return `${shorten(hint || "后台任务", 48)}${project ? ` · ${shorten(project, 24)}` : ""}`;
};

export const ruleSummary = (r) =>
  r.condition.kind === "remote_alert" ? `远端告警 ${r.condition.rule_id || "全部规则"} · 间隔 ${r.condition.cooldown_seconds} 秒` : r.condition.kind === "progress"
    ? `${r.condition.source} · ${r.condition.trigger.kind === "stage" ? `阶段变化 ${r.condition.trigger.stage || ""}` : r.condition.trigger.kind === "step" ? `每 ${r.condition.trigger.every} 步` : `${r.condition.trigger.name} ${r.condition.trigger.comparison} ${r.condition.trigger.value}`} · 间隔 ${r.condition.cooldown_seconds} 秒`
    : r.condition.kind === "contains"
      ? `输出包含 “${r.condition.text}”`
      : r.condition.kind === "no_output"
        ? `连续 ${r.condition.seconds} 秒无输出`
        : `总运行超过 ${r.condition.seconds} 秒`;
