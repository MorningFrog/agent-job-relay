import React, { useEffect, useRef, useState } from "react";
import { api } from "../transport.js";
import { date } from "../view.js";
const labels = {
  none: "暂无通知记录", pending: "等待投递", leased: "正在投递",
  retry_wait: "等待重试", dead_letter: "通知需要处理", accepted_by_host: "宿主已接收",
  notification_recorded: "已写入收件箱", inbox_only: "仅收件箱", cancelled: "通知已关闭",
};
export function NotificationStatus({ detail, token }) {
  const [history, setHistory] = useState(null), [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const generation = useRef(0);
  useEffect(() => { generation.current++; setHistory(null); setError(""); setBusy(false);
    return () => { generation.current++; };
  }, [detail?.job?.job_id, token]);
  const n = detail?.notifications;
  if (!n) return null;
  const problem = n.latest_problem, goal = n.goal;
  async function attempts(id, after = 0) {
    const current = ++generation.current;
    setBusy(true); setError("");
    try {
      const value = await api({ op: "delivery_attempts", delivery_id: id, after, limit: 20 }, token);
      if (generation.current === current) setHistory(old => ({ id, ...value,
        attempts: after && old?.id === id ? [...old.attempts, ...value.attempts] : value.attempts }));
    } catch (e) { if (generation.current === current) setError(e.message); }
    finally { if (generation.current === current) setBusy(false); }
  }
  return <section className="job-summary notification-panel" aria-label="通知与恢复">
    <strong>通知与恢复 · {labels[n.status] || n.status}</strong>
    <p className="muted small">任务执行、通知投递和 Agent 实际继续分别判断。宿主接收后，Agent 是否继续仍待核实。</p>
    <p className="execution-detail">{Object.entries(n.counts || {}).map(([s, count]) => `${labels[s] || s} ${count}`).join(" · ")}</p>
    {problem?.delivery_id && <div className="notice">
      <p>{problem.failure_kind === "rebind_required" ? "需要从原聊天重新绑定。" : "存在未解决的通知。"} {problem.error}</p>
      <p className="small">有效期至 {date(problem.expires_at)} · 下次尝试 {date(problem.next_attempt_at)}</p>
      <button className="button" disabled={busy} onClick={() => attempts(problem.delivery_id)}>查看失败尝试</button>
    </div>}
    {goal?.status && <div className="notice">
      <p>Goal 等待：{goal.status} · 版本 {goal.revision}</p>
      {goal.host_pause_required && <p>等待原聊天原生暂停及版本确认。</p>}
      {goal.detail && <p>{goal.detail}</p>}
    </div>}
    <details><summary>查看恢复步骤</summary>
      <p>请回到发起任务的原聊天，让 Agent 使用该任务记录的启动器和数据目录执行 <code>connect --recover</code> 预览；核对后使用 <code>connect --recover --apply</code>。</p>
      <p>此面板不能证明聊天身份。恢复保留任务、投递 ID、有效期和尝试历史，不重新执行命令。已接收和已过期通知不自动补投；Goal 等待保留原绑定，需通过原聊天的 Goal 控制处理。</p>
    </details>
    <details><summary>投递记录与尝试历史</summary>
      {(detail.deliveries?.deliveries || []).map(d => <p key={d.id}>
        <button className="button" disabled={busy} onClick={() => attempts(d.id)}>{labels[d.status] || d.status} · {d.attempts} 次</button> <code>{d.id}</code>
      </p>)}
      {detail.deliveries?.next_cursor > 0 && <p className="muted small">此处显示一页记录；完整分页可通过 job_deliveries 查询。</p>}
    </details>
    {error && <p role="alert">{error}</p>}
    {history && <div><p className="mono small">{history.id}</p>
      {history.attempts.map(a => <p key={a.attempt}>{a.attempt} · {date(a.started)} · {a.outcome || "执行中"} {a.error}</p>)}
      {history.attempts.length > 0 && <button className="button" disabled={busy} onClick={() => attempts(history.id, history.next_cursor)}>读取后续尝试</button>}
    </div>}
  </section>;
}
