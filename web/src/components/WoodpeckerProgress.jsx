import React from "react";
import { Icon } from "./Icons.jsx";

export function WoodpeckerProgress({ spec, pipeline, stale }) {
  const states = { blocked: "等待批准", declined: "已拒绝", pending: "排队中", waiting: "等待中",
    running: "运行中", success: "成功", failure: "失败", error: "错误", killed: "已取消", skipped: "已跳过" };
  const url = `${spec.server_url.replace(/\/$/, "")}/repos/${spec.repo_id}/pipeline/${spec.pipeline_number}`;
  return <div className="job-summary actions-progress">
    <strong>Woodpecker CI 进度</strong>
    <p className="execution-detail">{spec.repository} · pipeline #{spec.pipeline_number}</p>
    <p className="execution-detail mono">{spec.commit}</p>
    <p className="execution-detail">{states[pipeline?.status] || pipeline?.status || "等待首次查询"}</p>
    <a className="button actions-link" href={url} target="_blank" rel="noopener noreferrer">
      在 Woodpecker 查看此次运行 <Icon name="arrow" size={14} />
    </a>
    {stale && <p className="execution-detail">连接中断，以下为最近一次查询结果。</p>}
    {pipeline?.workflows?.items?.length > 0 && <ul className="actions-jobs">
      {pipeline.workflows.items.map(w => <li key={w.id}>
        <strong>{w.name}</strong><span>{states[w.status] || w.status}</span>
        <p>{w.total_steps == null ? "步骤尚未提供" : `已结束 ${w.completed_steps} / ${w.total_steps} 个步骤`}
          {w.current_step && ` · 当前：${w.current_step}`}</p>
      </li>)}
    </ul>}
    {pipeline?.workflows?.truncated && <p className="execution-detail">
      显示前 {pipeline.workflows.items.length} 个工作流，共 {pipeline.workflows.total_count} 个；完整列表请在 Woodpecker 查看。
    </p>}
    {pipeline?.workflows_error && <p className="execution-detail">暂时无法查询步骤详情，请在 Woodpecker 查看。</p>}
    <p className="execution-detail">步骤数量随运行变化，不代表耗时百分比。</p>
  </div>;
}
