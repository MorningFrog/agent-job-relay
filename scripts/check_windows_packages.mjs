// Actual Windows npm/native/MCP/host checks. Uses a local simulated OpenCode model.
import { readFileSync, writeFileSync, mkdirSync, existsSync, copyFileSync, cpSync, openSync, closeSync, mkdtempSync } from "node:fs";
import { dirname, join, resolve, basename } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { createServer as createNetServer } from "node:net";
import { createHash } from "node:crypto";
import { gunzipSync } from "node:zlib";
import { createInterface } from "node:readline";
import assert from "node:assert/strict";

if (process.platform !== "win32") throw new Error("Run this check on native Windows");
const args = new Map();
for (let i = 2; i < process.argv.length; i += 2) args.set(process.argv[i], process.argv[i + 1]);
const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const release = resolve(args.get("--release"));
const temporary = join(repo, ".tmp");mkdirSync(temporary, { recursive: true });
const root = mkdtempSync(join(temporary, "wp-"));
const meta = JSON.parse(readFileSync(join(release, "release.json")));
for (const line of readFileSync(join(release, "SHA256SUMS"), "utf8").trim().split(/\r?\n/)) {
  const [hash, name] = line.split("  ");assert.equal(name, basename(name));
  assert.equal(createHash("sha256").update(readFileSync(join(release, name))).digest("hex"), hash);
}
const env = { ...process.env, HOME: join(root, "home"), USERPROFILE: join(root, "home"),
  LOCALAPPDATA: join(root, "local"), APPDATA: join(root, "app"), XDG_DATA_HOME: join(root, "data"), XDG_STATE_HOME: join(root, "state"),
  XDG_CONFIG_HOME: join(root, "config"), XDG_CACHE_HOME: join(root, "cache"),
  npm_config_cache: join(root, "npm"), npm_config_userconfig: join(root, "empty-npmrc") };
for (const key of Object.keys(env)) if (/^(OPENAI_|CODEX_|ANTHROPIC_|OPENCODE_|AJR_)/i.test(key)) delete env[key];
mkdirSync(env.HOME);mkdirSync(env.LOCALAPPDATA);writeFileSync(env.npm_config_userconfig, "");
const active = new Set();const states = new Set();
async function run(argv, name, expected = 0, extraEnv = {}) {
  const child = spawn(argv[0], argv.slice(1), { cwd: env.HOME, env: { ...env, ...extraEnv }, stdio: ["ignore", "pipe", "pipe"] });
  active.add(child);let out = "", err = "";
  child.stdout.on("data", b => { out += b; if (out.length > 4 * 1024 * 1024) child.kill(); });
  child.stderr.on("data", b => { err += b; if (err.length > 4 * 1024 * 1024) child.kill(); });
  const timer = setTimeout(() => child.kill(), 120000);
  try {
  const code = await new Promise((accept, reject) => { child.once("error", reject);child.once("exit", accept); });
  // Exit and capture EOF are separate Windows boundaries.
  await Promise.race([Promise.all([new Promise(r => child.stdout.closed ? r() : child.stdout.once("close", r)), new Promise(r => child.stderr.closed ? r() : child.stderr.once("close", r))]), new Promise((_, reject) => setTimeout(() => reject(new Error(`${name}: capture EOF timed out`)), 5000).unref())]);
  writeFileSync(join(root, name + ".stdout"), out);writeFileSync(join(root, name + ".stderr"), err);
  assert.equal(code, expected, `${name}: ${err.slice(-3000)}`);return out;
  } finally { clearTimeout(timer);if (child.exitCode === null) child.kill();active.delete(child); }
}
function tarPackage(path) {
  const data = gunzipSync(readFileSync(path));
  for (let cursor = 0; cursor + 512 < data.length;) {
    const header = data.subarray(cursor, cursor + 512), name = header.subarray(0, 100).toString().split("\0")[0];
    const size = parseInt(header.subarray(124, 136).toString().replace(/\0/g, "").trim() || "0", 8);
    if (name === "package/package.json") return JSON.parse(data.subarray(cursor + 512, cursor + 512 + size));
    cursor += 512 + Math.ceil(size / 512) * 512;
  }
  throw new Error("package.json missing from npm tarball");
}
const candidates = new Map();const tarballs = new Map();
for (const key of ["runtime", "opencode"]) {
  const entry = meta[key];const path = join(release, entry.tarball);
  candidates.set(entry.name, { entry, package: tarPackage(path) });tarballs.set(entry.tarball, readFileSync(path));
}
const registry = createServer((request, response) => {
  const path = decodeURIComponent(request.url.split("?", 1)[0]).replace(/^\//, "");
  if (candidates.has(path)) {
    const { entry, package: pkg } = candidates.get(path);
    const version = { ...pkg, dist: { tarball: `http://127.0.0.1:${registry.address().port}/${entry.tarball}`, integrity: entry.integrity } };
    response.setHeader("Content-Type", "application/json");response.end(JSON.stringify({ name: entry.name, "dist-tags": { latest: entry.version }, versions: { [entry.version]: version } }));
  } else if (tarballs.has(path)) { response.end(tarballs.get(path)); }
  else { response.writeHead(302, { Location: "https://registry.npmjs.org/" + path });response.end(); }
});
await new Promise(r => registry.listen(0, "127.0.0.1", r));
const registryUrl = `http://127.0.0.1:${registry.address().port}`;
const npm = join(dirname(process.execPath), "node_modules/npm/bin/npm-cli.js");assert(existsSync(npm));
let runtimeCli;let core;let codex;let model;let opencode;let hostLog;let failureContext;
const report = { platform: "native Windows", model: "OpenCode uses a local simulated model; Codex discovery starts no model", root };
async function requestCore(state, op, name = "request") {
  return JSON.parse(await run([core, "--state-dir", state, "request", JSON.stringify(op)], name));
}
async function waitUntil(fn, milliseconds, description) {
  const deadline = Date.now() + milliseconds;
  do { const value = await fn();if (value) return value;await new Promise(r => setTimeout(r, 50)); } while (Date.now() < deadline);
  throw new Error(description);
}
async function rpcChild(entry) {
  const child = spawn(process.execPath, [entry, "app-server", "--stdio"], { env, cwd: env.HOME, stdio: ["pipe", "pipe", "pipe"] });active.add(child);
  let counter = 0;const pending = new Map();let errors = "";child.stderr.on("data", b => { errors += b; });
  createInterface({ input: child.stdout }).on("line", line => {
    const value = JSON.parse(line);const call = pending.get(value.id);
    if (call) { pending.delete(value.id);value.error ? call.reject(new Error(JSON.stringify(value.error))) : call.resolve(value.result); }
  });
  child.once("exit", () => { for (const p of pending.values()) p.reject(new Error("app-server exited"));writeFileSync(join(root, "codex-app-server.stderr"), errors); });
  return { child, async rpc(method, params) {
    const id = ++counter;
    return Promise.race([new Promise((resolve, reject) => { pending.set(id, { resolve, reject });child.stdin.write(JSON.stringify({ id, method, params }) + "\n"); }), new Promise((_, reject) => setTimeout(() => reject(new Error(method + " timed out")), 30000).unref())]);
  } };
}
try {
  const installed = join(root, "installed");
  await run([process.execPath, npm, "install", "--prefix", installed, "--ignore-scripts", "--no-audit", "--no-fund", "--registry=https://registry.npmjs.org", join(release, meta.runtime.tarball), join(release, meta.opencode.tarball)], "npm-install");
  runtimeCli = join(installed, "node_modules/agent-job-relay-runtime/cli.mjs");
  core = join(installed, "node_modules/agent-job-relay-runtime/native/win32-x64/agent-job-relay.exe");
  assert.equal((await run([process.execPath, runtimeCli, "--version"], "version")).trim(), `agent-job-relay ${meta.version}`);
  const state = join(root, "runtime-state");states.add(state);
  const spec = { command: { kind: "exec", program: process.execPath, args: ["-e", "process.stdout.write('packaged-runtime')"] }, cwd: env.HOME };
  const started = await requestCore(state, { op: "start", spec }, "native-start");
  failureContext = { state, job_id: started.job.job_id };
  const job = await waitUntil(async () => { const j = await requestCore(state, { op: "status", job_id: started.job.job_id }, "native-status");return ["completed", "failed", "orphaned"].includes(j.status) && j; }, 10000, "native job timeout");
  assert.equal(job.status, "completed", JSON.stringify(job));assert.equal(job.exit_code, 0, JSON.stringify(job));
  const moved = join(root, "moved-runtime");cpSync(dirname(runtimeCli), moved, { recursive: true });
  const movedStatus = JSON.parse(await run([process.execPath, join(moved, "cli.mjs"), "--state-dir", state, "status", job.job_id], "moved-status"));assert.equal(movedStatus.job_id, job.job_id);
  let bundledUi = false;
  const [major, minor, patch] = meta.version.split(".").map(Number);
  if (major > 0 || minor > 1 || (minor === 1 && patch >= 10)) {
    const ui = JSON.parse(await run([process.execPath, join(moved, "cli.mjs"), "--state-dir", state, "ui", "--no-open"], "moved-ui"));
    const access = new URL(ui.url);assert.equal(access.hostname, "127.0.0.1");assert.equal(access.protocol, "http:");
    const page = await fetch(access.origin + "/", { signal: AbortSignal.timeout(5000) });assert.equal(page.status, 200);
    const html = await page.text();const assets = [...html.matchAll(/(?:src|href)="(\/assets\/[^"<>]+)"/g)].map(m => m[1]);assert(assets.length >= 2, "embedded JS/CSS references missing");
    for (const path of assets) {
      const asset = await fetch(access.origin + path, { signal: AbortSignal.timeout(5000) });assert.equal(asset.status, 200);assert((await asset.arrayBuffer()).byteLength > 0);
    }
    const token = new URLSearchParams(access.hash.slice(1)).get("token");assert(token);
    const body = JSON.stringify({ op: "inspect", job_id: job.job_id });
    const unauthenticated = await fetch(access.origin + "/api/request", { method: "POST", body, headers: { "Content-Type": "application/json" }, signal: AbortSignal.timeout(5000) });assert.equal(unauthenticated.status, 401);await unauthenticated.text();
    const inspected = await fetch(access.origin + "/api/request", { method: "POST", body, headers: { "Content-Type": "application/json", Authorization: "Bearer " + token }, signal: AbortSignal.timeout(5000) });assert.equal(inspected.status, 200);
    const snapshot = await inspected.json();assert.equal(snapshot.job.job_id, job.job_id);assert.equal(snapshot.job.status, "completed");
    const launcher = [process.execPath, join(moved, "cli.mjs"), "--state-dir", state];
    const monitored = JSON.parse(await run([...launcher, "start", "--cwd", env.HOME, "--timeout", "30", "--no-output", "1s", "--action", "alert", "--", process.execPath, "-e", "setInterval(()=>{},1000)"], "monitored-start"));
    failureContext = { state, job_id: monitored.job.job_id };
    const rules = JSON.parse(await run([...launcher, "monitor", "add", monitored.job.job_id, "--elapsed", "20s", "--action", "stop", "--grace", "0s"], "monitor-add"));assert.equal(rules.rules.length, 2);
    const observed = await waitUntil(async () => {
      const v = JSON.parse(await run([...launcher, "inspect", monitored.job.job_id, "--json"], "monitor-inspect"));
      return v.events.some(e => e.kind === "monitor_alert") && v;
    }, 10000, "packaged silence rule did not trigger");
    assert.equal(observed.job.status, "running");assert.equal(observed.rules[0].triggered_revision, 1);
    const stopped = JSON.parse(await run([...launcher, "stop", monitored.job.job_id, "--grace", "0s", "--wait", "5s"], "monitor-stop"));assert.equal(stopped.stop_outcome, "terminal_observed");assert.equal(stopped.status, "cancelled");
    bundledUi = true;
  }
  await run([core, "--state-dir", state, "daemon", "stop"], "native-stop");
  if (bundledUi) {
    const offline = JSON.parse(await run([process.execPath, join(moved, "cli.mjs"), "--state-dir", state, "inspect", job.job_id, "--json"], "offline-inspect"));
    assert.equal(offline.runtime, "offline_snapshot");assert.equal(offline.job.job_id, job.job_id);
  }
  states.delete(state);
  failureContext = undefined;
  writeFileSync(join(moved, "native/win32-x64/agent-job-relay.exe"), "invalid", { flag: "a" });
  await run([process.execPath, join(moved, "cli.mjs"), "--version"], "corrupt-refused", 1);
  report.installed_runtime = { version: meta.version, history_preserved: true, corruption_refused: true, bundled_ui: bundledUi, cli_monitoring: bundledUi, offline_inspection: bundledUi, job_id: job.job_id };

  const codexEntry = args.get("--codex-entry");
  if (codexEntry) {
    env.CODEX_HOME = join(root, "codex");mkdirSync(env.CODEX_HOME);
    env.AJR_STATE_DIR = join(root, "codex-state");
    writeFileSync(join(env.CODEX_HOME, "config.toml"), 'cli_auth_credentials_store = "file"\n[analytics]\nenabled = false\n[feedback]\nenabled = false\n');
    const market = join(root, "marketplace");
    await run([join(process.env.SystemRoot, "System32/WindowsPowerShell/v1.0/powershell.exe"), "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", `Expand-Archive -LiteralPath '${join(release, meta.marketplace).replaceAll("'", "''")}' -DestinationPath '${market.replaceAll("'", "''")}'`], "marketplace-extract");
    for (const file of [".mcp.json", "mcp.json", "relay", "relay.cmd"]) {
      const path = join(market, "plugins/chatgpt", file);writeFileSync(path, readFileSync(path, "utf8").replaceAll("https://registry.npmjs.org", registryUrl));
    }
    await run([process.execPath, codexEntry, "plugin", "marketplace", "add", market], "codex-marketplace");
    const added = JSON.parse(await run([process.execPath, codexEntry, "plugin", "add", "agent-job-relay@agent-job-relay", "--json"], "codex-install"));
    codex = await rpcChild(codexEntry);
    await codex.rpc("initialize", { clientInfo: { name: "windows-packaged-check", version: "1" }, capabilities: { experimentalApi: true } });codex.child.stdin.write('{"method":"initialized"}\n');
    const account = await codex.rpc("account/read", { refreshToken: false });assert.equal(account.account, null);
    const details = await codex.rpc("plugin/read", { pluginName: "agent-job-relay", marketplacePath: join(market, ".agents/plugins/marketplace.json") });assert(details.plugin.mcpServers.includes("agent-job-relay"));
    const inventory = await codex.rpc("mcpServerStatus/list", { detail: "toolsAndAuthOnly" });const server = inventory.data.find(s => s.name === "agent-job-relay");assert.equal(Object.keys(server?.tools ?? {}).length, 22);
    assert(!existsSync(join(env.CODEX_HOME, "auth.json")));
    report.codex = { actual_catalog: true, tools: 22, account: null, distribution: "original dual-platform marketplace with loopback registry", cache: added.installedPath };
    codex.child.stdin.end();
  }

  const ocExe = args.get("--opencode");
  if (ocExe) {
    const project = join(root, "project");mkdirSync(project);
    const ocState = join(root, "opencode-state");states.add(ocState);
    const gate = join(root, "gate");mkdirSync(gate);
    let resumed = false;const modelRequests = [];
    model = createServer(async (request, response) => {
      try {
        const chunks = [];for await (const b of request) chunks.push(b);
        const body = JSON.parse(Buffer.concat(chunks));modelRequests.push(body);writeFileSync(join(root, "model-requests.json"), JSON.stringify(modelRequests, null, 2));
        const users = body.messages.filter(m => m.role === "user"), last = users.at(-1);let content = last?.content ?? "";
        if (Array.isArray(content)) content = content.map(p => p.text ?? "").join("\n");
        const tools = body.tools?.map(t => t.function?.name) ?? [];let delta, finish;
        if (content.includes("AgentJobRelay event (delivery ")) { delta = { role: "assistant", content: "RELAY_RESUMED_MATCHING_SESSION" };finish = "stop";resumed = true; }
        else if (tools.includes("agent_job_relay_job_start") && !body.messages.slice(body.messages.indexOf(last) + 1).some(m => m.role === "tool")) {
          const script = "const fs=require('node:fs'),p=require('node:path');const d=process.argv[1];fs.writeFileSync(p.join(d,'ready'),'ready');const t=setInterval(()=>{if(fs.existsSync(p.join(d,'release'))){clearInterval(t);console.log('released')}},25)";
          const spec = { command: { kind: "exec", program: process.execPath, args: ["-e", script, gate] }, cwd: project, timeout_seconds: 70 };
          delta = { role: "assistant", tool_calls: [{ index: 0, id: "fixture_relay_start", type: "function", function: { name: "agent_job_relay_job_start", arguments: JSON.stringify({ spec }) } }] };finish = "tool_calls";
        } else { delta = { role: "assistant", content: "Submission turn ended." };finish = "stop"; }
        const common = { id: "fixture", object: "chat.completion.chunk", created: 0, model: "relay-fixture" };
        const data = [{ ...common, choices: [{ index: 0, delta, finish_reason: null }] }, { ...common, choices: [{ index: 0, delta: {}, finish_reason: finish }], usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 } }].map(c => "data: " + JSON.stringify(c) + "\n\n").join("") + "data: [DONE]\n\n";
        if (body.stream) { response.writeHead(200, { "Content-Type": "text/event-stream" });response.end(data); }
        else { response.writeHead(200, { "Content-Type": "application/json" });response.end(JSON.stringify({ ...common, object: "chat.completion", choices: [{ index: 0, message: delta, finish_reason: finish }], usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 } })); }
      } catch (error) { response.writeHead(500);response.end(String(error)); }
    });
    await new Promise(r => model.listen(0, "127.0.0.1", r));
    writeFileSync(join(project, "opencode.json"), JSON.stringify({ $schema: "https://opencode.ai/config.json", plugin: [[`${meta.opencode.name}@${meta.version}`, { stateDir: ocState }]], model: "relayfixture/relay-fixture", small_model: "relayfixture/relay-fixture", enabled_providers: ["relayfixture"], provider: { relayfixture: { npm: "@ai-sdk/openai-compatible", name: "Isolated test fixture", options: { baseURL: `http://127.0.0.1:${model.address().port}/v1`, apiKey: "isolated-fixture" }, models: { "relay-fixture": { name: "Relay fixture", limit: { context: 32768, output: 1024 } } } } }, permission: { "*": "deny", "agent_job_relay_*": "allow" } }));
    const portSocket = createNetServer();await new Promise(r => portSocket.listen(0, "127.0.0.1", r));const port = portSocket.address().port;await new Promise(r => portSocket.close(r));
    hostLog = openSync(join(root, "opencode.log"), "w");
    const ocEnv = { ...env, OPENCODE_DISABLE_MODELS_FETCH: "1", OPENCODE_DISABLE_DEFAULT_PLUGINS: "1", OPENCODE_DISABLE_SHARE: "1", npm_config_registry: registryUrl, NPM_CONFIG_REGISTRY: registryUrl };
    opencode = spawn(ocExe, ["serve", "--print-logs", "--log-level", "DEBUG", "--hostname", "127.0.0.1", "--port", String(port)], { cwd: project, env: ocEnv, stdio: ["ignore", hostLog, hostLog] });active.add(opencode);
    const api = async (path, body) => { const res = await fetch(`http://127.0.0.1:${port}${path}`, { method: body === undefined ? "GET" : "POST", headers: { "Content-Type": "application/json" }, body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(path === "/global/health" ? 1500 : 40000) });assert(res.ok, `${path}: HTTP ${res.status}`);const text = await res.text();return text ? JSON.parse(text) : null; };
    let startupError;
    try {
      await waitUntil(async () => {
        assert.equal(opencode.exitCode, null, "OpenCode exited during startup");
        try { return await api("/global/health"); } catch (error) { startupError = error;return false; }
      }, 60000, "OpenCode startup failed");
    } catch (error) { throw new Error(`${error.message}: ${startupError?.message}; inspect opencode.log`, { cause: startupError }); }
    const first = await api("/session", { title: "Relay owner" }), second = await api("/session", { title: "Control session" });
    const initial = await api(`/session/${first.id}/message`, { parts: [{ type: "text", text: "Use AgentJobRelay to start the explicit fixture job and end the turn." }] });writeFileSync(join(root, "initial-turn.json"), JSON.stringify(initial, null, 2));
    const jobs = (await requestCore(ocState, { op: "list", after: 0, limit: 20 }, "oc-jobs")).jobs;assert.equal(jobs.length, 1);assert(["queued", "launching", "running"].includes(jobs[0].status));
    failureContext = { state: ocState, job_id: jobs[0].job_id };
    await waitUntil(() => existsSync(join(gate, "ready")), 5000, "workload readiness failed");writeFileSync(join(gate, "release"), "release");
    await waitUntil(() => resumed, 15000, "OpenCode callback did not invoke its model");
    const transcript = await waitUntil(async () => { const t = await api(`/session/${first.id}/message`);return t.some(m => m.info.role === "assistant" && m.info.finish === "stop" && m.info.time?.completed && m.parts.some(p => p.text === "RELAY_RESUMED_MATCHING_SESSION")) && t; }, 5000, "resumed response was not persisted");
    assert.equal((await api(`/session/${second.id}/message`)).length, 0);writeFileSync(join(root, "resumed-turn.json"), JSON.stringify(transcript, null, 2));
    const status = await requestCore(ocState, { op: "status", job_id: jobs[0].job_id }, "oc-status");assert.equal(status.status, "completed");assert.equal(status.exit_code, 0);
    const deliveries = await requestCore(ocState, { op: "deliveries", job_id: status.job_id, after: 0, limit: 20 }, "oc-deliveries");assert.equal(deliveries.deliveries.length, 1);assert.equal(deliveries.deliveries[0].status, "accepted_by_host");
    report.opencode = { loader: "actual Bun name/version", submission: "actual MCP with caller hook", continuation: "actual public session API and agent loop", model: "simulated", other_session_untouched: true, job_id: status.job_id };
  }
  report.result = "passed";
} catch (error) {
  report.result = "failed";report.error = error.stack;process.exitCode = 1;
  if (failureContext) {
    const { state, job_id } = failureContext;
    report.diagnostics = {};
    for (const [name, request] of [
      ["status", { op: "status", job_id }], ["doctor", { op: "doctor" }],
      ["events", { op: "events", job_id, after: 0, limit: 20 }],
      ["stdout", { op: "logs", job_id, stream: "stdout", cursor: 0, limit: 8192 }],
      ["stderr", { op: "logs", job_id, stream: "stderr", cursor: 0, limit: 8192 }],
    ]) {
      try { report.diagnostics[name] = await requestCore(state, request, `failure-${name}`); }
      catch (diagnosticError) { report.diagnostics[name] = { error: String(diagnosticError) }; }
    }
  }
}
finally {
  if (codex) codex.child.stdin.end();
  for (const child of active) { child.kill(); }
  for (const state of states) {
    if (!existsSync(join(state, "relay.sqlite3"))) continue;
    try {
      const jobs = await requestCore(state, { op: "list", after: 0, limit: 100 }, "cleanup-list");
      for (const job of jobs.jobs) if (["queued", "launching", "running"].includes(job.status)) await requestCore(state, { op: "stop", job_id: job.job_id }, "cleanup-stop-job");
      await run([core, "--state-dir", state, "daemon", "stop"], "cleanup-stop-daemon");
    } catch (error) { (report.cleanup_errors ??= []).push(String(error));report.result = "failed";process.exitCode = 1; }
  }
  registry.closeAllConnections();registry.close();if (model) { model.closeAllConnections();model.close(); }if (hostLog !== undefined) closeSync(hostLog);
  writeFileSync(join(root, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
}
