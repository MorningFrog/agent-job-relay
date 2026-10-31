import assert from "node:assert/strict";
import test from "node:test";
import { api, mutateTask, observeTask, streamTask, exportLogText } from "./transport.js";

const settle = async () => {
  await Promise.resolve();
  await Promise.resolve();
};
function driver(job = "a", token = "token") {
  const document = new EventTarget();
  document.hidden = false;
  const calls = [],
    timers = new Map(),
    samples = [],
    connections = [],
    errors = [];
  let nextTimer = 0;
  const options = {
    job,
    token,
    document,
    onSample: (value) => samples.push(value),
    onConnection: (value) => connections.push(value),
    onError: (value) => errors.push(value),
    schedule: (run, delay) => {
      const id = ++nextTimer;
      timers.set(id, { run, delay });
      return id;
    },
    unschedule: (id) => timers.delete(id),
    open: (job, token, signal, receive) =>
      new Promise((resolve, reject) =>
        calls.push({ job, token, signal, receive, resolve, reject }),
      ),
  };
  const close = observeTask(options);
  return {
    document,
    calls,
    timers,
    samples,
    connections,
    errors,
    close,
    options,
    async retry() {
      const [id, timer] = timers.entries().next().value;
      timers.delete(id);
      timer.run();
      await settle();
    },
    visibility(hidden) {
      document.hidden = hidden;
      document.dispatchEvent(new Event("visibilitychange"));
    },
  };
}

test("SSE handles split UTF-8, CRLF, comments and multiline data with bearer authentication", async () => {
  const bytes = new TextEncoder().encode(
    ': keepalive\r\n\r\ndata: {"title":\r\ndata: "汉字"}\r\n\r\ndata: {"detail":null}\n\n',
  );
  const received = [];
  await assert.rejects(
    streamTask(
      "a/b",
      "secret",
      new AbortController().signal,
      (value) => received.push(value),
      async (url, options) => {
        assert.equal(url, "/api/events?job_id=a%2Fb");
        assert.equal(options.headers.Authorization, "Bearer secret");
        return new Response(
          new ReadableStream({
            start(controller) {
              for (let index = 0; index < bytes.length; index += 3)
                controller.enqueue(bytes.slice(index, index + 3));
              controller.close();
            },
          }),
        );
      },
    ),
    /连接已结束/,
  );
  assert.deepEqual(received, [{ title: "汉字" }, { detail: null }]);
});

test("oversized or invalid task frames fail and release the reader", async () => {
  for (const data of [
    "x".repeat(4 * 1024 * 1024 + 1),
    "data: invalid JSON\n\n",
  ]) {
    let cancelled = false;
    const response = new Response(
      new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(data));
        },
        cancel() {
          cancelled = true;
        },
      }),
    );
    await assert.rejects(
      streamTask(
        "a",
        "token",
        new AbortController().signal,
        () => assert.fail("invalid sample delivered"),
        async () => response,
      ),
    );
    assert.equal(cancelled, true);
  }
});

test("disconnect backoff is bounded, resets on a sample and pauses when hidden", async () => {
  const d = driver();
  for (const delay of [2500, 5000, 10000, 20000, 30000, 30000]) {
    d.calls.at(-1).reject(new Error("offline"));
    await settle();
    assert.equal([...d.timers.values()][0].delay, delay);
    await d.retry();
  }
  d.calls.at(-1).receive({ overview: {}, detail: { job: { job_id: "a" } } });
  assert.equal(d.connections.at(-1), "connected");
  assert.equal(d.errors.at(-1), "");
  d.calls.at(-1).reject(new Error("dropped"));
  await settle();
  assert.equal([...d.timers.values()][0].delay, 2500);
  d.visibility(true);
  assert.equal(d.timers.size, 0);
  assert.equal(d.calls.at(-1).signal.aborted, true);
  const count = d.calls.length;
  d.visibility(false);
  assert.equal(d.calls.length, count + 1);
  d.close();
  d.calls.at(-1).receive({ detail: "late" });
  d.calls.at(-1).reject(new Error("late failure"));
  await settle();
  d.visibility(false);
  assert.equal(d.samples.length, 1);
  assert.equal(d.timers.size, 0);
  assert.equal(d.calls.length, count + 1);
});

test("quick task changes fence late responses and expired credentials do not retry", async () => {
  const a = driver("a");
  a.close();
  const b = driver("b", "fresh");
  a.calls[0].receive({ detail: { job: { job_id: "a" } } });
  b.calls[0].receive({ detail: { job: { job_id: "b" } } });
  assert.deepEqual(a.samples, []);
  assert.equal(b.samples[0].detail.job.job_id, "b");
  const error = new Error("expired");
  error.name = "AuthError";
  b.calls[0].reject(error);
  await settle();
  assert.equal(b.timers.size, 0);
  assert.equal(b.connections.at(-1), "disconnected");
  b.close();
});

test("rule responses cannot overwrite another task or apply a stale revision", async () => {
  let current = "a",
    finish;
  const applied = [];
  const body = {
    op: "monitor",
    job_id: "a",
    mutation: {
      kind: "update",
      rule_id: "rule",
      expected_revision: 7,
      rule: {},
    },
  };
  const pending = mutateTask(body, "token", {
    send: () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
    isCurrent: (id) => id === current,
    onRules: (value) => applied.push(value),
  });
  current = "b";
  finish({ rules: ["a"] });
  await pending;
  assert.deepEqual(applied, []);
  await assert.rejects(
    mutateTask({ ...body, job_id: "b" }, "token", {
      send: (request, token) =>
        api(request, token, async (_url, options) => {
          assert.equal(JSON.parse(options.body).mutation.expected_revision, 7);
          return new Response(
            JSON.stringify({
              error: "rule revision changed; inspect before editing",
            }),
            { status: 400 },
          );
        }),
      onRules: (value) => applied.push(value),
    }),
    /revision changed/,
  );
  assert.deepEqual(applied, []);
  await mutateTask({ ...body, job_id: "b" }, "token", {
    send: async () => ({ rules: ["b"] }),
    isCurrent: (id) => id === current,
    onRules: (value) => applied.push(value),
  });
  assert.deepEqual(applied, [{ rules: ["b"] }]);
});


test("log export preserves byte gaps, bounds its snapshot and fences task switches", async () => {
  const calls=[];
  const send=async body=>{ calls.push(body); return body.cursor===0
    ? {cursor:0,next_cursor:3,retained_end:10,text:"abc"}
    : {cursor:8,next_cursor:10,retained_end:10,text:"YZ"}; };
  const parts=await exportLogText("job","stderr","token",{send});
  assert.equal(parts.join(""),"abc\n[AgentJobRelay missing bytes 3..8]\nYZ");
  assert.equal(calls.length,2);
  assert.equal(calls[1].cursor,3);
  await assert.rejects(exportLogText("job","stdout","token",{send,isCurrent:()=>false}),/任务已切换/);
  await assert.rejects(exportLogText("job","stdout","token",{send:async()=>({cursor:0,next_cursor:0,retained_end:10,text:""})}),/未前进/);
});
