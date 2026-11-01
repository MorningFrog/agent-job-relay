import React, { useEffect, useRef, useState } from "react";
import { RetainedLogs } from "./RetainedLogs.jsx";
import { Icon } from "./Icons.jsx";
import { age } from "../view.js";

export function TerminalOutput({ job, observation, streamName, setStreamName, token, active }) {
  const [follow, setFollow] = useState(true);
  const output = useRef(null);
  const stream = observation?.[streamName];
  const text = stream?.text || (job.command?.kind === "github_actions"
    ? "此观察任务不提供终端日志。请在“进度”中查看工作流和步骤。"
    : active ? "等待输出…" : `此视图暂无 ${streamName} 输出。`);
  useEffect(() => {
    if (follow && output.current) output.current.scrollTop = output.current.scrollHeight;
  }, [text, follow, streamName]);
  return <div className="output-view">
    <div className="terminal-frame">
      <div className="output-toolbar">
        <div className="stream-picker" role="group" aria-label="输出来源">
          <Icon name="terminal" size={14} />
          {["stdout", "stderr"].map(name => <button type="button" key={name}
            aria-pressed={streamName === name} className={streamName === name ? "selected" : ""}
            onClick={() => setStreamName(name)}>{name}</button>)}
        </div>
        <button type="button" className={`follow-output ${follow ? "following" : ""}`}
          aria-pressed={follow} onClick={() => setFollow(!follow)}>{follow ? "跟随输出" : "继续跟随"}</button>
      </div>
      <pre ref={output} className="log terminal-log" tabIndex={0} aria-label={`${streamName} 终端输出`}
        onScroll={event => { const el = event.currentTarget;
          if (el.scrollHeight - el.scrollTop - el.clientHeight > 24) setFollow(false);
        }}>{text}</pre>
      <div className="terminal-footer"><span>{stream?.total_bytes || 0} 字节</span>
        <span>最近输出 {age(stream?.last_output_at)}</span></div>
    </div>
    {stream?.truncated && <p className="output-note">显示最近 64 KiB；更早的输出可按保留区间查阅。</p>}
    <RetainedLogs job={job.job_id} stream={streamName} token={token} />
  </div>;
}
