import React, { useEffect, useRef, useState } from "react";
import { api, exportLogText } from "../transport.js";
export function RetainedLogs({ job, stream, token }) {
  const [page, setPage] = useState(null), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const [text, setText] = useState(""), [matches, setMatches] = useState(null);
  useEffect(() => { generation.current++; setPage(null); setMatches(null); setText(""); setError(""); setBusy(false);
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
  async function find(append = false) {
    const current = ++generation.current; setBusy(true); setError("");
    let cursor = append ? matches.next_cursor : 0, end = append ? matches.end : null;
    const found = [];
    try {
      let value;
      do {
        value = await api({op:"search_logs", job_id:job, stream, text, cursor, end, limit:20-found.length}, token);
        if (current !== generation.current) return;
        if (!value.complete && value.next_cursor <= cursor) throw new Error("日志检索游标未前进。");
        found.push(...value.matches); cursor=value.next_cursor; end=value.end;
      } while (!value.complete && found.length < 20);
      setMatches({...value, matches: append ? [...matches.matches, ...found] : found});
    } catch(e) { if(current===generation.current)setError(e.message); }
    finally { if(current===generation.current)setBusy(false); }
  }
  return <details className="job-summary"><summary>查阅与导出保留日志</summary>
    <p className="muted small">按原始字节位置读取。原生任务保留前 16 MiB 与之后最近四个分段；不足一段的新输出先见上方实时尾部。导出为 UTF-8 文本，缺失区间有明确标记。</p>
    <button className="button" disabled={busy} onClick={()=>read(0)}>读取保留区间</button>{" "}
    <button className="button" disabled={busy} onClick={download}>导出保留日志</button>
    <form className="log-search" onSubmit={e=>{e.preventDefault();void find();}}>
      <label>定位日志文本<input aria-label="定位日志文本" required maxLength={1024} value={text} onChange={e=>{generation.current++;setText(e.target.value);setMatches(null);setBusy(false);setError("");}} placeholder="按大小写匹配原始文本" /></label>
      <button className="button" disabled={busy || !text}>查找保留日志</button>
    </form>
    {matches && <div aria-label="日志检索结果">
      <p className="small">找到 {matches.matches.length} 处 · 已检索至字节 {matches.next_cursor} / {matches.end}</p>
      {matches.gaps?.length > 0 && <p className="notice small">保留缺口：{matches.gaps.map(r=>`${r.from}–${r.to}`).join("、")}。这些区间无法检索。</p>}
      {matches.matches.map(m=><div key={m.offset}><button className="button" disabled={busy} onClick={()=>read(m.context_from)}>定位字节 {m.offset}–{m.end}</button><pre className="log">{m.text}</pre></div>)}
      {!matches.complete && <button className="button" disabled={busy} onClick={()=>find(true)}>继续查找</button>}
    </div>}
    {error && <p role="alert">{error}</p>}
    {page && <>
      <p className="small">保留区间：{page.retained_ranges?.map((r,i)=><button className="button" disabled={busy} key={i} onClick={()=>read(r.from)}>{r.from}–{r.to}</button>)}</p>
      {page.skipped && <p className="notice">原始字节 {page.skipped.from}–{page.skipped.to} 已不在保留范围。</p>}
      <p className="small">当前 {page.cursor}–{page.next_cursor}</p><pre className="log">{page.text || "此区间暂无输出。"}</pre>
      <button className="button" disabled={busy || page.next_cursor>=page.retained_end} onClick={()=>read(page.next_cursor)}>下一段</button>
    </>}
  </details>;
}
