const MAX_FRAME = 4 * 1024 * 1024;

export async function api(body, token, request = fetch) {
  const response = await request("/api/request", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${token}`,
    },
    body: JSON.stringify(body),
  });
  const value = await response.json();
  if (!response.ok) {
    const error = new Error(value.error || `请求失败 (${response.status})`);
    error.status = response.status;
    throw error;
  }
  return value;
}

export async function mutateTask(
  body,
  token,
  { send = api, isCurrent = () => true, onRules = () => {} } = {},
) {
  const value = await send(body, token);
  // A response to task A must not replace task B's rules or close B's form.
  if (body.op === "monitor" && isCurrent(body.job_id)) onRules(value);
  return value;
}

export async function streamTask(job, token, signal, receive, request = fetch) {
  const query = job ? `?job_id=${encodeURIComponent(job)}` : "";
  const response = await request(`/api/events${query}`, {
    headers: { Authorization: `Bearer ${token}` },
    signal,
  });
  if (!response.ok) {
    const error = new Error(
      response.status === 401
        ? "访问凭据已失效，请在终端重新运行 ui 打开面板。"
        : `连接失败 (${response.status})`,
    );
    if (response.status === 401) error.name = "AuthError";
    throw error;
  }
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    while (!signal.aborted) {
      const { done, value } = await reader.read();
      if (done) throw new Error("连接已结束，正在重连。");
      buffer += decoder.decode(value, { stream: true });
      let boundary;
      while ((boundary = /\r?\n\r?\n/.exec(buffer))) {
        const frame = buffer.slice(0, boundary.index);
        buffer = buffer.slice(boundary.index + boundary[0].length);
        if (frame.length > MAX_FRAME) throw new Error("任务更新超过大小限制。");
        const data = frame
          .split(/\r?\n/)
          .filter((line) => line.startsWith("data:"))
          .map((line) => line.slice(5).replace(/^ /, ""))
          .join("\n");
        if (data && !signal.aborted) receive(JSON.parse(data));
      }
      if (buffer.length > MAX_FRAME) throw new Error("任务更新超过大小限制。");
    }
  } finally {
    await reader.cancel().catch(() => {});
  }
}

export function observeTask({
  job,
  token,
  onSample,
  onConnection,
  onError,
  document = globalThis.document,
  open = streamTask,
  schedule = setTimeout,
  unschedule = clearTimeout,
}) {
  let controller;
  let timer;
  let disposed = false;
  let retry = 2500;
  async function connect() {
    if (disposed || document?.hidden || !token) return;
    const active = new AbortController();
    controller = active;
    onConnection("connecting");
    const current = () =>
      !disposed && !active.signal.aborted && controller === active;
    try {
      await open(job, token, active.signal, (value) => {
        if (!current()) return;
        if (value.error) {
          onError(value.error);
          return;
        }
        retry = 2500;
        onError("");
        onConnection("connected");
        onSample(value);
      });
    } catch (error) {
      if (!current() || error.name === "AbortError") return;
      onConnection("disconnected");
      onError(error.message);
      if (error.name !== "AuthError") {
        timer = schedule(connect, retry);
        retry = Math.min(30000, retry * 2);
      }
    }
  }
  function visibility() {
    unschedule(timer);
    controller?.abort();
    if (!document.hidden) void connect();
  }
  document?.addEventListener("visibilitychange", visibility);
  void connect();
  return () => {
    disposed = true;
    unschedule(timer);
    controller?.abort();
    document?.removeEventListener("visibilitychange", visibility);
  };
}

// Explicit export reads one bounded snapshot; it never waits for future output.
export async function exportLogText(job, stream, token, { send = api, isCurrent = () => true } = {}) {
  let page = await send({ op: "logs", job_id: job, stream, cursor: 0, limit: 65536 }, token);
  const end = page.retained_end;
  if (!Number.isSafeInteger(end) || end < 0) throw new Error("运行时不支持保留日志导出，请先更新。");
  let cursor = 0;
  const parts = [];
  for (let count = 0; count < 512; count++) {
    if (!isCurrent()) throw new Error("任务已切换，导出已停止。");
    const actual = page.cursor, next = page.next_cursor;
    if (!Number.isSafeInteger(actual) || actual < cursor || !Number.isSafeInteger(next)) throw new Error("日志游标无效。");
    if (actual > cursor) parts.push(`\n[AgentJobRelay missing bytes ${cursor}..${Math.min(actual, end)}]\n`);
    if (actual >= end) return parts;
    cursor = actual;
    if (next <= actual) throw new Error("日志读取未前进，请重新查看保留区间。");
    if (next <= end) { parts.push(page.text); cursor = next; }
    const from = next > end ? actual : cursor;
    if (cursor >= end) return parts;
    page = await send({ op: "logs", job_id: job, stream, cursor: from, limit: Math.min(65536, end - from) }, token);
  }
  throw new Error("导出超出有界读取额度。");
}
