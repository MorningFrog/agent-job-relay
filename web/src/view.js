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
export const command = (job) =>
  job?.command_preview ||
  (job?.command?.kind === "remote"
    ? `远程 ${job.command.remote.target_id} · ${job.command.remote.intent.kind}`
    : job?.command?.kind === "github_actions"
      ? `GitHub Actions ${job.command.workflow.repository} · ${job.command.workflow.run_id} / ${job.command.workflow.attempt}`
      : job?.command?.kind === "shell"
        ? job.command.script
        : [job?.command?.program, ...(job?.command?.args || [])]
            .filter(Boolean)
            .join(" "));

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
  r.condition.kind === "progress"
    ? `${r.condition.source} · ${r.condition.trigger.kind === "stage" ? `阶段变化 ${r.condition.trigger.stage || ""}` : r.condition.trigger.kind === "step" ? `每 ${r.condition.trigger.every} 步` : `${r.condition.trigger.name} ${r.condition.trigger.comparison} ${r.condition.trigger.value}`} · 间隔 ${r.condition.cooldown_seconds} 秒`
    : r.condition.kind === "contains"
      ? `输出包含 “${r.condition.text}”`
      : r.condition.kind === "no_output"
        ? `连续 ${r.condition.seconds} 秒无输出`
        : `总运行超过 ${r.condition.seconds} 秒`;
