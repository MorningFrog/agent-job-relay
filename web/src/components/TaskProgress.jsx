import React from "react";
import { ActionsProgress } from "./ActionsProgress.jsx";
import { WoodpeckerProgress } from "./WoodpeckerProgress.jsx";
import { date, isWorkflow } from "../view.js";

export function TaskProgress({ detail }) {
  const job = detail.job, external = detail.observation?.external;
  const progress = detail.observation?.progress || external?.observation?.progress;
  return <div className="progress-view">
    {job.command?.kind === "github_actions" && <ActionsProgress
      spec={job.command.workflow} workflow={external?.workflow} stale={external?.evidence_stale} />}
    {job.command?.kind === "woodpecker" && <WoodpeckerProgress
      spec={job.command.pipeline} pipeline={external?.pipeline} stale={external?.evidence_stale} />}
    {external && <section className="job-summary">
      <strong>{isWorkflow(external.mode) ? "工作流观察" : "远程任务"}</strong>
      <p className="execution-detail">连接：{external.connection} · 最近成功 {date(external.last_success_at)}</p>
      {external.run && <p className="execution-detail mono">远端运行 {external.run.run_id} · attempt {external.run.attempt} · {external.status || "待核实"}</p>}
      {!external.can_stop && <p className="execution-detail">仅观察：停止观察不会取消远端运行。</p>}
      {external.last_error && <p className="execution-detail">{external.last_error}；远端结果待核实。</p>}
    </section>}
    {detail.observation?.remote_feedback && <section className="job-summary" aria-label="远端反馈证据">
      <strong>远端告警反馈</strong>
      <p className="small">已记录 {detail.observation.remote_feedback.alerts} 次 · 合并 {detail.observation.remote_feedback.coalesced} 次 · 受限 {detail.observation.remote_feedback.suppressed} 次</p>
      {(detail.observation.remote_feedback.milestones || []).map(m => <div key={`${m.rule_id}:${m.at}`}>
        <p className="small">{date(m.at)} · 远端事件 {m.evidence.remote.event_id}</p>
        <pre>{JSON.stringify(m.evidence.source_evidence, null, 2)}</pre>
      </div>)}
    </section>}
    {progress ? <section className="job-summary">
      <strong>进度与里程碑</strong>
      {Object.values(progress.sources || {}).map(source => <div className="progress-source" key={source.source}>
        <p className="execution-detail">{source.source} · {source.stage || "进行中"}
          {source.step != null && ` · ${source.step}${source.total ? ` / ${source.total}` : ""} 步`}</p>
        {source.total && <progress aria-label={`${source.source} 进度`} max={source.total} value={source.step} />}
        <p className="execution-detail">{Object.entries(source.metrics || {}).map(([key, value]) => `${key}: ${value}`).join(" · ")}</p>
        {source.observed_run && <p className="execution-detail mono">观察对象：{source.observed_run}</p>}
      </div>)}
      {progress.milestones?.map((milestone, i) => <p className="execution-detail" key={i}>
        {date(milestone.at)} · {milestone.sample.source} · {milestone.sample.stage || ""}
        {milestone.sample.step != null && ` · ${milestone.sample.step} 步`}
        {milestone.notification_superseded && " · 未单独通知，已保留至最终结果"}
      </p>)}
      <details className="progress-counters"><summary>采样统计</summary><p className="muted small">
        已接收 {progress.accepted} 次 · 告警 {progress.alerts} 次 · 受限反馈 {progress.suppressed} 次 · 丢弃 {progress.dropped} 次 · 无效 {progress.rejected} 次。输出作为数据展示。
      </p></details>
    </section> : !external && <div className="empty compact"><h3>暂无结构化进度</h3><p>终端输出仍可在“输出”中查看。</p></div>}
  </div>;
}
