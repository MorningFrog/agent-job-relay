import React, { useState } from "react";
import { Icon } from "./Icons.jsx";
export function RuleForm({ existing, close, save, busy, externalKind }) {
  const c = existing?.spec.condition;
  const [kind, setKind] = useState(
      c?.kind || (externalKind ? "elapsed" : "no_output"),
    ),
    [text, setText] = useState(c?.text || ""),
    [seconds, setSeconds] = useState(c?.seconds || 600),
    [which, setWhich] = useState(c?.stream || "both"),
    [action, setAction] = useState(existing?.spec.action || "alert"),
    [notify, setNotify] = useState(existing?.spec.notify_agent || false),
    [grace, setGrace] = useState(existing?.spec.grace_seconds ?? 5),
    [source, setSource] = useState(c?.source || "training"),
    [triggerKind, setTriggerKind] = useState(c?.trigger?.kind || "stage"),
    [stage, setStage] = useState(c?.trigger?.stage || ""),
    [every, setEvery] = useState(c?.trigger?.every || 100),
    [metric, setMetric] = useState(c?.trigger?.name || "loss"),
    [comparison, setComparison] = useState(c?.trigger?.comparison || "le"),
    [threshold, setThreshold] = useState(c?.trigger?.value ?? 0.1),
    [cooldown, setCooldown] = useState(c?.cooldown_seconds || 60);
  return (
    <form
      className="rule-form"
      onSubmit={(e) => {
        e.preventDefault();
        save({
          condition:
            kind === "progress"
              ? {
                  kind,
                  source,
                  trigger:
                    triggerKind === "stage"
                      ? { kind: "stage", stage: stage || null }
                      : triggerKind === "step"
                        ? { kind: "step", every: Number(every) }
                        : {
                            kind: "metric",
                            name: metric,
                            comparison,
                            value: Number(threshold),
                          },
                  cooldown_seconds: Number(cooldown),
                }
              : kind === "contains"
                ? { kind, text, stream: which }
                : { kind, seconds: Number(seconds) },
          action,
          notify_agent: notify,
          grace_seconds: Number(grace),
        });
      }}
    >
      <div className="section-heading">
        <strong>{existing ? "编辑规则" : "添加监控规则"}</strong>
        <button
          type="button"
          className="icon-button"
          aria-label="关闭规则表单"
          onClick={close}
        >
          <Icon name="close" />
        </button>
      </div>
      <div className="form-grid">
        <label>
          检测条件
          <select value={kind} onChange={(e) => setKind(e.target.value)}>
            {!externalKind && <option value="no_output">连续无输出</option>}
            {externalKind !== "github_actions" && (
              <option value="contains">输出包含文本</option>
            )}
            <option value="elapsed">总运行时间超过</option>
            {!externalKind && <option value="progress">结构化进度</option>}
          </select>
        </label>
        {kind === "progress" ? (
          <>
            <label>
              进度来源
              <input
                required
                maxLength={32}
                pattern="[A-Za-z0-9._-]+"
                value={source}
                onChange={(e) => setSource(e.target.value)}
              />
            </label>
            <label>
              反馈条件
              <select
                value={triggerKind}
                onChange={(e) => setTriggerKind(e.target.value)}
              >
                <option value="stage">阶段变化</option>
                <option value="step">步数里程碑</option>
                <option value="metric">指标阈值</option>
              </select>
            </label>
            {triggerKind === "stage" ? (
              <label>
                指定阶段（留空表示所有阶段）
                <input
                  maxLength={64}
                  value={stage}
                  onChange={(e) => setStage(e.target.value)}
                />
              </label>
            ) : triggerKind === "step" ? (
              <label>
                每隔步数
                <input
                  type="number"
                  required
                  min="1"
                  max="9007199254740991"
                  value={every}
                  onChange={(e) => setEvery(e.target.value)}
                />
              </label>
            ) : (
              <>
                <label>
                  指标名称
                  <input
                    required
                    maxLength={32}
                    pattern="[A-Za-z0-9._-]+"
                    value={metric}
                    onChange={(e) => setMetric(e.target.value)}
                  />
                </label>
                <label>
                  比较
                  <select
                    value={comparison}
                    onChange={(e) => setComparison(e.target.value)}
                  >
                    <option value="le">≤</option>
                    <option value="lt">&lt;</option>
                    <option value="ge">≥</option>
                    <option value="gt">&gt;</option>
                  </select>
                </label>
                <label>
                  阈值
                  <input
                    type="number"
                    required
                    step="any"
                    value={threshold}
                    onChange={(e) => setThreshold(e.target.value)}
                  />
                </label>
              </>
            )}
            <label>
              重复反馈最短间隔（秒）
              <input
                type="number"
                required
                min="1"
                max="604800"
                value={cooldown}
                onChange={(e) => setCooldown(e.target.value)}
              />
            </label>
          </>
        ) : kind === "contains" ? (
          <>
            <label>
              匹配文本
              <input
                required
                maxLength={1024}
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder="例如 CUDA out of memory"
              />
            </label>
            <label>
              输出来源
              <select value={which} onChange={(e) => setWhich(e.target.value)}>
                <option value="both">stdout 与 stderr</option>
                <option value="stdout">stdout</option>
                <option value="stderr">stderr</option>
              </select>
            </label>
          </>
        ) : (
          <label>
            时长（秒）
            <input
              type="number"
              required
              min="1"
              max="604800"
              value={seconds}
              onChange={(e) => setSeconds(e.target.value)}
            />
          </label>
        )}
        <label>
          命中后执行
          <select value={action} onChange={(e) => setAction(e.target.value)}>
            <option value="alert">记录本地告警</option>
            <option value="stop">停止任务并记录告警</option>
          </select>
        </label>
        {action === "stop" && (
          <label>
            停止宽限（秒）
            <input
              type="number"
              required
              min="0"
              max="300"
              value={grace}
              onChange={(e) => setGrace(e.target.value)}
            />
          </label>
        )}
      </div>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={notify}
          onChange={(e) => setNotify(e.target.checked)}
        />{" "}
        同时通知任务绑定的 Agent 会话
      </label>
      <p className="muted">
        保存后等待运行器应用。文本和时长规则每版本触发一次。进度规则按新序号重复触发，受间隔和配额限制；仅检查生效后的新输出。
      </p>
      <div className="form-actions">
        <button type="button" className="button" onClick={close}>
          取消
        </button>
        <button className="button primary" disabled={busy}>
          保存规则
        </button>
      </div>
    </form>
  );
}
