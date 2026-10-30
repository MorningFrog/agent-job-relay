import React from "react";
import { Icon, Badge } from "./Icons.jsx";
import { ActionsProgress } from "./ActionsProgress.jsx";
import { RuleForm } from "./RuleForm.jsx";
import {
  terminal,
  names,
  phaseNames,
  date,
  age,
  elapsed,
  command,
  ruleSummary,
} from "../view.js";
export function TaskDetail({
  selected,
  detail,
  busy,
  tab,
  streamName,
  form,
  grace,
  mutate,
  setForm,
  setStreamName,
  setTab,
  setGrace,
  closeDetail,
}) {
  const progress =
    detail?.observation?.progress ||
    detail?.observation?.external?.observation?.progress;
  const job = detail?.job,
    active = job && !terminal.has(job.status),
    rules = (detail?.rules || []).filter((r) => !r.deleted),
    alerts = (detail?.events || []).filter((e) => e.kind === "monitor_alert");
  return (
    selected && (
      <section className="panel detail" id="task-detail" aria-label="任务详情">
        <div className="section-heading">
          <div>
            <span className="eyebrow">任务详情</span>
            <h2>{job?.job_id?.slice(0, 8) || "加载中…"}</h2>
          </div>
          <button
            className="icon-button"
            aria-label="关闭任务详情"
            onClick={closeDetail}
          >
            <Icon name="close" />
          </button>
        </div>
        {job && (
          <>
            <div className="job-summary">
              <Badge status={job.status} />
              <span className="muted">{elapsed(job)}</span>
              <pre>{command(job)}</pre>
              <div className="mono small muted">{job.cwd}</div>
              <div className="small muted">
                启动 {date(job.run?.started_at)}
                {job.exit_code != null && ` · 退出码 ${job.exit_code}`}
              </div>
            </div>
            {job.command?.kind === "github_actions" && (
              <ActionsProgress
                spec={job.command.workflow}
                workflow={detail.observation?.external?.workflow}
                stale={detail.observation?.external?.evidence_stale}
              />
            )}
            {detail.observation?.external && (
              <div className="job-summary">
                <strong>
                  {detail.observation.external.mode === "github_actions"
                    ? "工作流观察"
                    : "远程任务"}
                </strong>
                <p className="execution-detail">
                  连接：{detail.observation.external.connection} · 最近成功{" "}
                  {date(detail.observation.external.last_success_at)}
                </p>
                {detail.observation.external.run && (
                  <p className="execution-detail mono">
                    远端运行 {detail.observation.external.run.run_id} · attempt{" "}
                    {detail.observation.external.run.attempt} ·{" "}
                    {detail.observation.external.status || "待核实"}
                  </p>
                )}
                {!detail.observation.external.can_stop && (
                  <p className="execution-detail">
                    仅观察：停止观察不会取消远端运行。
                  </p>
                )}
                {detail.observation.external.last_error && (
                  <p className="execution-detail">
                    {detail.observation.external.last_error}；远端结果待核实。
                  </p>
                )}
              </div>
            )}
            {progress && (
              <div className="job-summary">
                <strong>进度与里程碑</strong>
                {Object.values(progress.sources || {}).map((s) => (
                  <div key={s.source}>
                    <p className="execution-detail">
                      {s.source} · {s.stage || "进行中"}
                      {s.step != null &&
                        ` · ${s.step}${s.total ? ` / ${s.total}` : ""} 步`}
                    </p>
                    {s.total && (
                      <progress
                        aria-label={`${s.source} 进度`}
                        max={s.total}
                        value={s.step}
                      />
                    )}
                    <p className="execution-detail">
                      {Object.entries(s.metrics || {})
                        .map(([k, v]) => `${k}: ${v}`)
                        .join(" · ")}
                    </p>
                    {s.observed_run && (
                      <p className="execution-detail mono">
                        观察对象：{s.observed_run}
                      </p>
                    )}
                  </div>
                ))}
                <p className="muted small">
                  已接收 {progress.accepted} 次 · 告警 {progress.alerts} 次 ·
                  受限反馈 {progress.suppressed} 次 · 丢弃 {progress.dropped} 次
                  · 无效 {progress.rejected} 次。输出作为数据展示。
                </p>
                {progress.milestones?.map((m, i) => (
                  <p className="execution-detail" key={i}>
                    {date(m.at)} · {m.sample.source} · {m.sample.stage || ""}
                    {m.sample.step != null && ` · ${m.sample.step} 步`}
                  </p>
                ))}
              </div>
            )}
            <div className="detail-meta">
              <span>
                最近输出
                <strong>
                  {age(
                    Math.max(
                      detail.observation?.stdout?.last_output_at || 0,
                      detail.observation?.stderr?.last_output_at || 0,
                    ) || null,
                  )}
                </strong>
              </span>
              <span>
                监控规则
                <strong>{rules.filter((r) => r.enabled).length} 条启用</strong>
              </span>
            </div>
            {active &&
              detail.observation?.sampled_at &&
              Date.now() - detail.observation.sampled_at > 5000 && (
                <div className="notice">
                  运行器采样已超过 5 秒未更新，当前监控状态待核实。
                </div>
              )}
            {(active || detail.stop) && (
              <div className="stop-controls">
                <span>
                  {detail.stop
                    ? active
                      ? phaseNames[detail.observation?.stop_phase] ||
                        "已请求停止"
                      : detail.observation?.stop_phase === "graceful"
                        ? "已温和结束"
                        : detail.observation?.stop_phase === "forced"
                          ? "已强制结束"
                          : "已结束"
                    : "停止宽限"}
                </span>
                <select
                  aria-label="停止宽限"
                  value={grace}
                  onChange={(e) => setGrace(Number(e.target.value))}
                >
                  <option value="5">5 秒</option>
                  <option value="30">30 秒</option>
                  <option value="60">60 秒</option>
                  <option value="0">立即强制结束</option>
                </select>
                <button
                  className="button danger"
                  disabled={!active || busy}
                  onClick={() =>
                    mutate({
                      op: "stop",
                      job_id: job.job_id,
                      grace_seconds: grace,
                      wait_seconds: 0,
                    })
                  }
                >
                  <Icon name="stop" size={15} />
                  {job.command?.kind === "github_actions" ||
                  (job.command?.kind === "remote" &&
                    job.command.remote.intent.kind === "attach" &&
                    !job.command.remote.intent.allow_stop)
                    ? "停止观察"
                    : "停止任务"}
                </button>
              </div>
            )}
            <div className="tabs">
              {[
                ["output", "输出"],
                ["rules", "监控规则"],
                [
                  "events",
                  `事件${alerts.length ? ` · ${alerts.length} 条告警` : ""}`,
                ],
              ].map(([id, label]) => (
                <button
                  key={id}
                  className={tab === id ? "active" : ""}
                  onClick={() => setTab(id)}
                >
                  {label}
                </button>
              ))}
            </div>
            {tab === "output" && (
              <div className="output-view">
                <div className="output-toolbar">
                  <div>
                    {["stdout", "stderr"].map((s) => (
                      <button
                        key={s}
                        className={streamName === s ? "selected" : ""}
                        onClick={() => setStreamName(s)}
                      >
                        {s}
                      </button>
                    ))}
                  </div>
                  <span className="muted small">
                    {detail.observation?.[streamName]?.total_bytes || 0} 字节
                  </span>
                </div>
                <pre className="log">
                  {detail.observation?.[streamName]?.text || "等待输出…"}
                </pre>
                {detail.observation?.[streamName]?.truncated && (
                  <p className="muted small">
                    显示最近 64 KiB；更早的输出可能已超出保留范围。
                  </p>
                )}
                {job.detail && <p className="execution-detail">{job.detail}</p>}
              </div>
            )}
            {tab === "rules" && (
              <div className="rules-view">
                {active && !form && (
                  <button
                    className="button primary"
                    onClick={() => setForm({})}
                  >
                    <Icon name="plus" size={16} />
                    添加规则
                  </button>
                )}
                {form && (
                  <RuleForm
                    externalKind={
                      job.command?.kind === "remote" ||
                      job.command?.kind === "github_actions"
                        ? job.command.kind
                        : null
                    }
                    key={form.id || "new"}
                    existing={form.id ? form : null}
                    close={() => setForm(null)}
                    busy={busy}
                    save={(spec) =>
                      mutate({
                        op: "monitor",
                        job_id: job.job_id,
                        mutation: form.id
                          ? {
                              kind: "update",
                              rule_id: form.id,
                              expected_revision: form.revision,
                              rule: spec,
                            }
                          : { kind: "add", rule: spec },
                      })
                    }
                  />
                )}
                {!rules.length && !form && (
                  <div className="empty compact">
                    <Icon name="shield" size={28} />
                    <h3>让运行器替你持续检查</h3>
                    <p>
                      按静默时长、输出文本或运行时长设置规则，无需 Agent
                      保持在线。
                    </p>
                  </div>
                )}
                {rules.map((r) => (
                  <article className="rule-card" key={r.id}>
                    <div className="rule-top">
                      <strong>{ruleSummary(r.spec)}</strong>
                      <span
                        className={`rule-state ${r.triggered_revision === r.revision ? "triggered" : ""}`}
                      >
                        {!r.enabled
                          ? "已禁用"
                          : r.applied_revision !== r.revision
                            ? "等待应用"
                            : r.triggered_revision === r.revision
                              ? r.spec.condition.kind === "progress"
                                ? "持续检测 · 已有反馈"
                                : "已触发"
                              : "已生效"}
                      </span>
                    </div>
                    <p>
                      {r.spec.action === "stop"
                        ? `停止任务 · 宽限 ${r.spec.grace_seconds} 秒`
                        : "记录本地告警"}
                      {r.spec.notify_agent ? " · 通知 Agent" : ""}
                    </p>
                    <small className="muted">
                      版本 {r.revision} ·{" "}
                      {r.triggered_at
                        ? `上次命中 ${date(r.triggered_at)}`
                        : `应用于 ${date(r.applied_at)}`}
                    </small>
                    {active && (
                      <div className="rule-actions">
                        <button disabled={busy} onClick={() => setForm(r)}>
                          编辑
                        </button>
                        <button
                          disabled={busy}
                          onClick={() =>
                            mutate({
                              op: "monitor",
                              job_id: job.job_id,
                              mutation: {
                                kind: "enable",
                                rule_id: r.id,
                                expected_revision: r.revision,
                                enabled:
                                  r.triggered_revision === r.revision &&
                                  r.spec.condition.kind !== "progress"
                                    ? true
                                    : !r.enabled,
                              },
                            })
                          }
                        >
                          {r.triggered_revision === r.revision &&
                          r.spec.condition.kind !== "progress"
                            ? "重新布防"
                            : r.enabled
                              ? "禁用"
                              : "启用"}
                        </button>
                        <button
                          disabled={busy}
                          onClick={() =>
                            mutate({
                              op: "monitor",
                              job_id: job.job_id,
                              mutation: {
                                kind: "remove",
                                rule_id: r.id,
                                expected_revision: r.revision,
                              },
                            })
                          }
                        >
                          移除
                        </button>
                      </div>
                    )}
                  </article>
                ))}
              </div>
            )}
            {tab === "events" && (
              <div className="events-view">
                {!detail.events?.length && <p className="muted">暂无事件。</p>}
                {detail.events?.map((e) => (
                  <article
                    className={`event ${e.kind === "monitor_alert" ? "alert-event" : ""}`}
                    key={e.event_id}
                  >
                    <span className="event-dot" />
                    <div>
                      <strong>
                        {e.kind === "monitor_alert"
                          ? "监控规则命中"
                          : names[e.kind] || e.kind}
                      </strong>
                      <small>{date(e.at)}</small>
                      <pre>
                        {e.kind === "monitor_alert"
                          ? `${ruleSummary(e.payload.rule)}\n${JSON.stringify(e.payload.evidence, null, 2)}`
                          : JSON.stringify(e.payload, null, 2)}
                      </pre>
                    </div>
                  </article>
                ))}
              </div>
            )}
          </>
        )}
      </section>
    )
  );
}
