import React from "react";
import { Icon } from "./Icons.jsx";
export function ActionsProgress({ spec, workflow, stale }) {
  const states = {
    queued: "排队中",
    pending: "等待中",
    waiting: "等待中",
    requested: "已请求",
    in_progress: "运行中",
    completed: "已完成",
    success: "成功",
    failure: "失败",
    cancelled: "已取消",
    timed_out: "超时",
    skipped: "已跳过",
    neutral: "中立",
    action_required: "需要操作",
    stale: "已过期",
    startup_failure: "启动失败",
  };
  const url = `https://${spec.hostname || "github.com"}/${spec.repository.split("/").map(encodeURIComponent).join("/")}/actions/runs/${spec.run_id}/attempts/${spec.attempt}`;
  return (
    <div className="job-summary actions-progress">
      <strong>GitHub Actions 进度</strong>
      <p className="execution-detail">
        {spec.repository} · run {spec.run_id} · attempt {spec.attempt}
      </p>
      <p className="execution-detail">
        {states[workflow?.status] || workflow?.status || "等待首次查询"}
        {workflow?.conclusion &&
          ` · ${states[workflow.conclusion] || workflow.conclusion}`}
      </p>
      <a
        className="button actions-link"
        href={url}
        target="_blank"
        rel="noopener noreferrer"
      >
        在 GitHub 查看此次运行 <Icon name="arrow" size={14} />
      </a>
      {stale && (
        <p className="execution-detail">连接中断，以下为最近一次查询结果。</p>
      )}
      {workflow?.jobs?.items?.length > 0 && (
        <ul className="actions-jobs">
          {workflow.jobs.items.map((j) => (
            <li key={j.id}>
              <strong>{j.name}</strong>
              <span>
                {states[j.status] || j.status}
                {j.conclusion && ` · ${states[j.conclusion] || j.conclusion}`}
              </span>
              <p>
                已结束 {j.completed_steps} / {j.total_steps} 个步骤
                {j.current_step && ` · 当前：${j.current_step}`}
              </p>
            </li>
          ))}
        </ul>
      )}
      {workflow?.jobs?.truncated && (
        <p className="execution-detail">
          显示前 {workflow.jobs.items.length} 个作业，共{" "}
          {workflow.jobs.total_count} 个；完整列表请在 GitHub 查看。
        </p>
      )}
      {workflow?.jobs_error && (
        <p className="execution-detail">
          暂时无法查询步骤详情，请在 GitHub 查看。
        </p>
      )}
      <p className="execution-detail">步骤数量随运行变化，不代表耗时百分比。</p>
    </div>
  );
}
