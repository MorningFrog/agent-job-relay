import React from "react";
import { Badge } from "./Icons.jsx";
import { date } from "../view.js";
export function RuntimeStatus({ connection, overview }) {
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
        <dt>运行中任务</dt>
        <dd>{overview?.active_jobs?.length || 0}</dd>
        <dt>等待重试的通知</dt>
        <dd>{overview?.health?.retrying_deliveries || 0}</dd>
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
