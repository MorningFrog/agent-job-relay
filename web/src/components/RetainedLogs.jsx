import React, { useEffect, useRef, useState } from "react";
import { api, exportLogText } from "../transport.js";
export function RetainedLogs({ job, stream, token }) {
  const [page, setPage] = useState(null), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const generation = useRef(0);
  useEffect(() => { generation.current++; setPage(null); setError(""); setBusy(false);
    return () => { generation.current++; };
  }, [job, stream, token]);
  async function read(cursor = 0) {
    const current = ++generation.current;setBusy(true);setError("");
    try { const value=await api({op:"logs",job_id:job,stream,cursor,limit:8192},token);
      if (current===generation.current) setPage(value);
    } catch(e) {if (current===generation.current) setError(e.message);}
    finally {if (current===generation.current) setBusy(false);}
  }
  async function download() {
    const current=++generation.current;setBusy(true);setError("");
    try {
      const parts=await exportLogText(job,stream,token,{isCurrent:()=>current===generation.current});
      if (current!==generation.current) return;
      const url=URL.createObjectURL(new Blob(parts,{type:"text/plain;charset=utf-8"}));
      const a=document.createElement("a");a.href=url;a.download=`${job}-${stream}.txt`;a.click();
      setTimeout(()=>URL.revokeObjectURL(url),1000);
    } catch(e) {if(current===generation.current)setError(e.message);}
    finally {if(current===generation.current)setBusy(false);}
  }
  return <details className="job-summary"><summary>查阅与导出保留日志</summary>
    <p className="muted small">按原始字节位置读取。原生任务保留前 16 MiB 与之后最近四个分段；不足一段的新输出先见上方实时尾部。导出为 UTF-8 文本，缺失区间有明确标记。</p>
    <button className="button" disabled={busy} onClick={()=>read(0)}>读取保留区间</button>{" "}
    <button className="button" disabled={busy} onClick={download}>导出保留日志</button>
    {error && <p role="alert">{error}</p>}
    {page && <>
      <p className="small">保留区间：{page.retained_ranges?.map((r,i)=><button className="button" disabled={busy} key={i} onClick={()=>read(r.from)}>{r.from}–{r.to}</button>)}</p>
      {page.skipped && <p className="notice">原始字节 {page.skipped.from}–{page.skipped.to} 已不在保留范围。</p>}
      <p className="small">当前 {page.cursor}–{page.next_cursor}</p><pre className="log">{page.text || "此区间暂无输出。"}</pre>
      <button className="button" disabled={busy || page.next_cursor>=page.retained_end} onClick={()=>read(page.next_cursor)}>下一段</button>
    </>}
  </details>;
}
