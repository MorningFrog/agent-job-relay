import React from "react";

export function PageHeader({ page, overview, connection }) {
  const runtime = page === "runtime";
  const active = overview?.active_jobs;
  const running = active?.filter((job) =>
    ["launching", "running"].includes(job.status),
  ).length;
  const queued = active?.filter((job) => job.status === "queued").length;

  return (
    <header className={runtime ? undefined : "overview-header"}>
      <div className={runtime ? undefined : "overview-heading"}>
        {runtime && <div className="eyebrow">工作空间 / 运行时</div>}
        <h1>{runtime ? "运行时状态" : page === "alerts" ? "告警与事件" : "任务概览"}</h1>
        {runtime ? (
          <p>查看执行进度，设置规则，在需要时停止任务。</p>
        ) : (
          <dl className="task-counts" aria-label="当前任务状态">
            <div><dt>运行中</dt><dd>{running ?? "—"}</dd></div>
            <div><dt>等待启动</dt><dd>{queued ?? "—"}</dd></div>
          </dl>
        )}
      </div>
      <div className="live-label">
        <span className={`connection-dot ${connection}`} />
        {connection === "connected" ? "实时更新" : connection === "connecting" ? "正在连接…" : "连接已中断"}
      </div>
    </header>
  );
}
