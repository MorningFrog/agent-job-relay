import React from "react";
import { Icon, Badge } from "./Icons.jsx";
import { NotificationStatus } from "./NotificationStatus.jsx";
import { FeedbackStatus } from "./FeedbackStatus.jsx";
import { RuleForm } from "./RuleForm.jsx";
import { TaskProgress } from "./TaskProgress.jsx";
import { TerminalOutput } from "./TerminalOutput.jsx";
import "./TaskDetail.css";
import { terminal, names, phaseNames, date, elapsed, command, taskTitle, ruleSummary } from "../view.js";

function feedbackHint(detail) {
  const notification = detail.notifications, feedback = detail.feedback;
  if (notification?.goal?.status === "attention" || notification?.goal?.continuation?.action === "host_control") return { text: "Goal 需要处理", attention: true };
  if (notification?.goal?.host_pause_required || ["native_pause", "confirm_pause", "report_then_pause"].includes(notification?.goal?.continuation?.action)) return { text: "Goal 等待交接", attention: true };
  if (notification?.status === "dead_letter" || notification?.latest_problem?.failure_kind === "rebind_required") return { text: "通知需要处理", attention: true };
  if (feedback?.visibility?.advisories?.length) return { text: "输出与反馈需检查", attention: true };
  const labels = { local_only: "仅本地告警", inbox_only: "反馈写入收件箱", configured: "已配置中途反馈",
    not_configured: "未配置中途反馈", goal_completion_only: "仅合并完成反馈", goal_feedback: "已配置 Goal 反馈",
    goal_feedback_exhausted: "Goal 反馈次数已用完", goal_wait_closed: "Goal 反馈已关闭", exhausted: "反馈次数已用完",
    unbound: "反馈尚未绑定", binding_unavailable: "反馈绑定需检查" };
  if (labels[feedback?.status]) return { text: labels[feedback.status], attention: ["unbound", "binding_unavailable"].includes(feedback.status) };
  if (notification?.status === "accepted_by_host") return { text: "宿主已接收通知", attention: false };
  if (notification?.status === "retry_wait") return { text: "通知自动重试中", attention: false };
  return { text: "反馈与通知", attention: false };
}

export function TaskDetail({ selected, token, detail, busy, tab, streamName, form, grace,
  mutate, setForm, setStreamName, setTab, setGrace, closeDetail }) {
  const job = detail?.job;
  const active = job && !terminal.has(job.status);
  const rules = (detail?.rules || []).filter(rule => !rule.deleted);
  const alerts = (detail?.events || []).filter(event => event.kind === "monitor_alert");
  const external = detail?.observation?.external;
  const remote = job?.command?.kind === "remote" ? job.command.remote : null;
  const remoteSpec = remote?.intent?.kind === "launch" ? remote.intent.spec : null;
  const remoteRun = external?.run || (remote?.intent?.kind === "attach" ? remote.intent.run : null);
  const hint = detail ? feedbackHint(detail) : null;
  const stopObservation = job?.command?.kind === "github_actions" ||
    (job?.command?.kind === "remote" && job.command.remote?.intent?.kind === "attach" && !job.command.remote.intent.allow_stop);
  const stale = active && detail.observation?.sampled_at && Date.now() - detail.observation.sampled_at > 5000;
  return selected && <section className="panel detail" id="task-detail" aria-label="任务详情">
    <div className="section-heading detail-heading">
      <div><span className="eyebrow">任务详情</span><h2>{job ? taskTitle(job) : "加载中…"}</h2></div>
      <button className="icon-button" aria-label="关闭任务详情" onClick={closeDetail}><Icon name="close" /></button>
    </div>
    {job && <>
      <div className="detail-overview">
        <div className="detail-status"><Badge status={job.status} /><span>{elapsed(job)}</span>
          {job.status === "queued" && job.queue && <span>排队第 {job.queue.position} 位 · {job.run?.started_at ? "等待恢复观察" : `截止 ${date(job.queue.expires_at)}`}</span>}
          {job.exit_code != null && <span>退出码 {job.exit_code}</span>}
          {detail.stop && <span>{active ? phaseNames[detail.observation?.stop_phase] || "已请求停止" : detail.observation?.stop_phase === "forced" ? "已强制结束" : "已结束"}</span>}
        </div>
        {active && <div className="detail-stop">
          <select aria-label="停止宽限" title="停止前的宽限时间" value={grace} onChange={event => setGrace(Number(event.target.value))}>
            <option value="5">5 秒宽限</option><option value="30">30 秒宽限</option><option value="60">60 秒宽限</option><option value="0">立即强制结束</option>
          </select>
          <button className="button danger" disabled={busy} onClick={() => mutate({ op: "stop", job_id: job.job_id, grace_seconds: grace, wait_seconds: 0 })}>
            <Icon name="stop" size={13} />{stopObservation ? "停止观察" : "停止任务"}
          </button>
        </div>}
      </div>
      {job.queue && job.status === "timeout" && !job.run?.started_at && <p className="sample-warning">排队期限已到，任务未启动。</p>}
      {hint && <div className="detail-signals">
        <button className={`detail-feedback-link ${hint.attention ? "needs-attention" : ""}`} onClick={() => setTab("feedback")}>
          <Icon name={hint.attention ? "bell" : "shield"} size={13} />{hint.text}<Icon name="arrow" size={12} />
        </button>
        {(stale || external?.evidence_stale) && <span className="sample-warning">{external ? "远端证据待核实" : "运行器采样待核实"}</span>}
      </div>}
      <div className="tabs detail-tabs" role="tablist" aria-label="任务详情分类">
        {[["output", "输出"], ["progress", "进度"], ["feedback", "反馈"], ["rules", "规则"], ["events", "事件"]].map(([id, label]) =>
          <button key={id} role="tab" id={`task-tab-${id}`} aria-selected={tab === id} aria-controls="task-tab-content"
            className={tab === id ? "active" : ""} tabIndex={tab === id ? 0 : -1}
            onKeyDown={event => {
              const tabs = [...event.currentTarget.parentElement.querySelectorAll('[role="tab"]')];
              const index = tabs.indexOf(event.currentTarget);
              const next = event.key === "ArrowRight" ? (index + 1) % tabs.length : event.key === "ArrowLeft" ? (index + tabs.length - 1) % tabs.length : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : null;
              if (next != null) { event.preventDefault(); tabs[next].click(); tabs[next].focus(); }
            }} onClick={() => setTab(id)}>
            {label}{id === "rules" && rules.length > 0 && <span className="tab-count">{rules.filter(rule => rule.enabled).length}</span>}
            {((id === "feedback" && hint?.attention) || (id === "events" && alerts.length > 0)) && <span className="tab-dot" aria-label="需要查看" />}
          </button>)}
      </div>
      <div id="task-tab-content" role="tabpanel" aria-labelledby={`task-tab-${tab}`}>
        {tab === "output" && <TerminalOutput key={job.job_id} job={job} observation={detail.observation} active={active}
          streamName={streamName} setStreamName={setStreamName} token={token} />}
        {tab === "progress" && <TaskProgress detail={detail} />}
        {tab === "feedback" && <div className="feedback-view">
          <FeedbackStatus feedback={detail.feedback} active={active}
            external={job.command?.kind === "remote" || job.command?.kind === "github_actions" ? job.command.kind : null}
            configure={preset => { setTab("rules"); setForm(preset === "manage" ? null : { preset }); }} />
          <NotificationStatus detail={detail} token={token} />
        </div>}
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
                key={form.id || form.preset || "new"}
                existing={form.id ? form : null}
                preset={form.preset}
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
      </div>
      <details className="execution-info">
        <summary><Icon name="terminal" size={13} />命令与执行信息</summary>
        <dl><dt>任务 ID</dt><dd className="mono">{job.job_id}</dd>
          <dt>运行 ID</dt><dd className="mono">{job.run_id || job.run?.run_id || "—"}</dd>
          <dt>{remote ? "本地观察目录" : "工作目录"}</dt><dd className="mono">{job.cwd}</dd>
          {remote && <><dt>远端目标 ID</dt><dd className="mono">{remote.target_id}</dd>
            <dt>远端工作目录</dt><dd className="mono">{remoteSpec?.cwd || "未提供（附加观察）"}</dd>
            <dt>远端任务 ID</dt><dd className="mono">{remoteRun?.job_id || "尚未确认"}</dd>
            <dt>远端运行 ID</dt><dd className="mono">{remoteRun?.run_id || "尚未确认"}</dd>
            <dt>远端 attempt</dt><dd>{remoteRun?.attempt ?? "尚未确认"}</dd>
            {remoteSpec?.command?.kind === "shell" && <><dt>远端解释器</dt><dd className="mono">{remoteSpec.command.interpreter}</dd></>}
          </>}
          <dt>启动时间</dt><dd>{date(job.run?.started_at)}</dd>
          {job.run?.ended_at && <><dt>结束时间</dt><dd>{date(job.run.ended_at)}</dd></>}
        </dl>
        {remoteSpec && <p className="small">提交的远端命令</p>}
        <pre className="execution-command">{command(job)}</pre>
        {job.detail && <p className="execution-detail">{job.detail}</p>}
      </details>
    </>}
  </section>;
}
