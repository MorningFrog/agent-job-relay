import React from "react";
import { ruleSummary } from "../view.js";

const statuses = {
  not_configured: "未配置中途反馈", local_only: "仅记录本地告警", inbox_only: "反馈写入收件箱",
  configured: "已配置同聊天反馈", unbound: "尚未绑定通知目标", binding_unavailable: "通知绑定需要检查",
  exhausted: "当前反馈规则已耗尽", goal_completion_only: "Goal 仅合并完成反馈",
  goal_feedback: "Goal 中途反馈已配置", goal_feedback_exhausted: "Goal 中途反馈次数已用完",
  goal_wait_closed: "Goal 自动反馈已关闭", terminal: "任务已结束", unknown: "反馈状态待核实",
};
const visibility = { awaiting_sample: "等待首次采样", unavailable: "采样信息不足", stale: "采样已过期",
  observed: "已有输出时间采样", quiet: "长时间没有捕获到输出", external: "使用远端观察证据", terminal: "已保留最终证据" };
const advisories = {
  quiet_output: "至少十分钟没有捕获到输出。检查日志去向、缓冲或进度来源；静默不能证明任务卡住。",
  no_mid_run_notification: "长任务尚无启用的中途通知规则。如已选择仅完成通知，可保持当前配置。",
};
export function FeedbackStatus({ feedback, active, external, configure }) {
  if (!feedback) return null;
  return <section className="job-summary" aria-label="中途反馈状态">
    <strong>中途反馈 · {statuses[feedback.status] || "状态待核实"}</strong>
    <p className="small">启用规则 {feedback.configured_rules ?? 0} 条 · 通知规则 {feedback.notifying_rules ?? 0} 条</p>
    <p className="small">输出证据：{visibility[feedback.visibility?.status] || "等待采样"}
      {feedback.visibility?.output_idle_seconds != null && ` · 距最近输出 ${feedback.visibility.output_idle_seconds} 秒`}</p>
    {(feedback.rules || []).map(rule => <p className="small" key={rule.rule_id}>
      {ruleSummary({ condition: rule.condition })} · {!rule.enabled ? "已禁用" : rule.application === "applied" ? "运行器已应用" : rule.application === "terminal" ? "任务已结束" : "等待运行器应用"}
      {rule.sample_observed != null && ` · ${rule.sample_observed ? "已观察到进度来源" : "尚无该来源数据"}`}
      {rule.enabled && !rule.can_trigger_again && " · 此规则不能继续触发"}
    </p>)}
    {(feedback.targets || []).map(target => <p className="small" key={target.target_id}>
      通知目标：{target.status === "configured" ? target.can_resume ? "已登记同聊天续接能力" : "仅收件箱" : target.status === "lease_expired" ? "租期已到，投递时需验证续租" : "绑定已失效"}
    </p>)}
    {feedback.visibility?.advisories?.map((item, i) => <p className="notice small" key={i}>{advisories[item.code] || item.message}</p>)}
    {feedback.reason && <details><summary>查看反馈诊断</summary><p className="small">{feedback.reason}</p></details>}
    <p className="muted small">规则保存、运行器应用、来源数据、通知送达和 Agent 实际继续分别判断。仅完成通知也是有效选择；Goal 中途反馈须沿用原聊天授权。</p>
    {active && <div className="rule-actions">
      {!external && <button className="button" onClick={() => configure("stage")}>配置阶段反馈</button>}
      <button className="button" onClick={() => configure("elapsed")}>配置时长检查点</button>
      <button className="button" onClick={() => configure("manage")}>管理规则／仅完成通知</button>
    </div>}
  </section>;
}
