import React from "react";
import { Badge } from "./Icons.jsx";
import { date } from "../view.js";
const space = bytes => bytes == null ? "待核实" : `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
export function StorageNotice({ capacity }) {
  if (!capacity) return null;
  const pressure = capacity.disk_pressure;
  return <>
    {pressure && pressure.status !== "ok" && <div className="notice storage-notice" role="status">
      <strong>{pressure.status === "critical" ? "磁盘空间不足：新任务准入受限" : pressure.status === "warning" ? "磁盘空间偏低" : "磁盘容量采样不可用"}</strong>
      <p>可用 {space(pressure.available_bytes)} · 提醒阈值 {space(pressure.warn_below_bytes)} · 新任务拒绝阈值 {space(pressure.stop_new_jobs_below_bytes)}</p>
      <p className="small">采样于 {date(pressure.sampled_at)}。每次新任务准入重新核对容量；已有任务、停止操作及原键重试继续保留。</p>
      {pressure.error && <p className="small">{pressure.error}</p>}
    </div>}
    {capacity.backup_health === "overdue" && <p className="notice">备份已超过配置间隔；最近记录 {date(capacity.last_backup_at)}。请检查数据维护状态。</p>}
    {capacity.last_error && <p className="notice" role="status">数据维护需要处理：{capacity.last_error}</p>}
    {capacity.warning && <p className="notice">{capacity.warning}</p>}
  </>;
}
export function RuntimeStatus({ connection, overview }) {
  const capacity = overview?.health?.capacity, pressure = capacity?.disk_pressure;
  return (
    <section className="panel runtime">
      <div className="section-heading">
        <h2>本地运行时</h2>
        <Badge status={connection === "connected" ? "running" : "orphaned"} />
      </div>
      <dl>
        <dt>版本</dt>
        <dd>{overview?.health?.version || "—"}</dd>
        <dt>状态配置</dt>
        <dd className="mono">{overview?.profile || "—"}</dd>
        <dt>存储健康</dt>
        <dd>{overview?.health?.store || "—"}</dd>
        <dt>磁盘可用空间</dt>
        <dd>{space(pressure?.available_bytes)}</dd>
        <dt>容量采样</dt><dd>{date(pressure?.sampled_at)}</dd>
        <dt>最近记录备份</dt><dd>{date(capacity?.last_backup_at)}</dd>
        <dt>备份状态</dt><dd>{{ recorded: "已记录备份", overdue: "超过配置间隔", not_recorded: "尚无备份记录" }[capacity?.backup_health] || "待核实"}</dd>
        <dt>暂缓归档任务</dt><dd>{capacity?.last_scan?.deferred ?? "—"}</dd>
        <dt>运行中任务</dt>
        <dd>{(overview?.active_jobs || []).filter(job => ["launching", "running"].includes(job.status)).length} / {overview?.health?.bounded_queue?.execution_limit || 8}</dd>
        <dt>等待启动的任务</dt>
        <dd>{(overview?.active_jobs || []).filter(job => job.status === "queued").length}</dd>
        <dt>等待重试的通知</dt>
        <dd>{overview?.health?.retrying_deliveries || 0}</dd>
        <dt>停止重试的通知</dt>
        <dd>{overview?.health?.dead_letters || 0}</dd>
        <dt>需要处理的 Goal 等待</dt>
        <dd>{overview?.health?.goal_waits?.attention || 0}</dd>
        <dt>需要恢复绑定</dt>
        <dd>{overview?.health?.rebind_required || 0}</dd>
        <dt>健康采样</dt>
        <dd>{date(overview?.health?.sampled_at || overview?.sampled_at)}</dd>
      </dl>
      <p className="muted">
        宿主接收通知与 Agent 实际继续执行分别记录。任务输出均作为数据展示。
      </p>
    </section>
  );
}
