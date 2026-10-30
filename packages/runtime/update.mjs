// npm-specific lifecycle orchestration. Native code parses options and fences
// profiles; host configuration is changed only through the public Codex CLI.
import { spawn } from "node:child_process";
import { accessSync, closeSync, constants, copyFileSync, fsyncSync, lstatSync, mkdtempSync, openSync, readFileSync, realpathSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, delimiter, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { resolveExecutable, defaultStateDir } from "./index.mjs";

const packageRoot = dirname(fileURLToPath(import.meta.url));
const registry = "https://registry.npmjs.org";
const repository = "https://github.com/MorningFrog/agent-job-relay.git";
const selector = "agent-job-relay@agent-job-relay";
const packageName = "agent-job-relay-runtime";
const versionPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const running = new Set();
let interrupted = false;

export function compareVersions(a, b) {
  if (![a, b].every(v => typeof v === "string" && versionPattern.test(v))) throw new Error("Expected a published numeric x.y.z version");
  const x = a.split(".").map(BigInt), y = b.split(".").map(BigInt);
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] > y[i] ? 1 : -1;
  return 0;
}

function executable(name) {
  for (const directory of (process.env.PATH ?? "").split(delimiter)) {
    if (!directory) continue;
    for (const suffix of process.platform === "win32" ? [".exe", ".cmd", ".bat", ""] : [""]) {
      const path = resolve(directory, name + suffix);
      try { accessSync(path, process.platform === "win32" ? constants.F_OK : constants.X_OK); return path; } catch { /* Try the next PATH entry. */ }
    }
  }
  throw new Error(`${name} must be installed and available on PATH`);
}

function command(program, args, options = {}) {
  if (process.platform === "win32" && /\.(cmd|bat)$/i.test(program)) {
    // cmd expansion cannot safely represent these path characters. Fail before
    // mutation rather than letting a shell reinterpret an executable or argument.
    const quoted = [program, ...args].map(value => {
      if (/["%!\r\n]/.test(value)) throw new Error("Command path contains unsupported cmd.exe expansion characters");
      return `"${value}"`;
    });
    return spawn(process.env.ComSpec || "cmd.exe", ["/d", "/s", "/c", `"${quoted.join(" ")}"`], { ...options, windowsVerbatimArguments: true });
  }
  return spawn(program, args, options);
}

async function run(program, args) {
  if (interrupted) throw new Error("Update interrupted");
  const child = command(program, args, { stdio: ["ignore", "pipe", "pipe"] });
  running.add(child);
  let stdout = "", stderr = "";
  const timer = setTimeout(() => child.kill(), 300000);
  for (const [stream, append] of [[child.stdout, data => stdout += data], [child.stderr, data => stderr += data]]) {
    stream.on("data", data => { append(data); if (stdout.length + stderr.length > 4 * 1024 * 1024) child.kill(); });
  }
  try {
    const code = await new Promise((accept, reject) => { child.once("error", reject); child.once("close", accept); });
    if (code !== 0) throw new Error(`${args.slice(0, 3).join(" ")} failed (${code}): ${stderr.slice(-2000)}`);
    return stdout.trim();
  } finally { clearTimeout(timer); running.delete(child); }
}

function npmEntry() {
  const entry = realpathSync(executable("npm"));
  if (entry.endsWith("npm-cli.js")) return entry;
  // Official Windows Node/npm distribution.
  const cli = join(dirname(entry), "node_modules/npm/bin/npm-cli.js");
  accessSync(cli, constants.R_OK);
  return cli;
}

function canonical(path) {
  const value = realpathSync(path);
  return process.platform === "win32" ? value.toLowerCase() : value;
}

export async function inspectMarketplace(codex, runCommand = run) {
  const data = JSON.parse(await runCommand(codex, ["plugin", "marketplace", "list", "--json"]));
  if (!Array.isArray(data.marketplaces)) throw new Error("Codex returned an unsupported marketplace list; no changes applied");
  const matches = data.marketplaces.filter(item => item.name === "agent-job-relay");
  if (matches.length > 1) throw new Error("Multiple AgentJobRelay marketplace registrations; no changes applied");
  const market = matches[0];
  if (!market) return null;
  if (market.marketplaceSource?.sourceType !== "git" || market.marketplaceSource.source !== repository) {
    throw new Error("The existing agent-job-relay marketplace is local or foreign. Keep its native update route, or use ajr update --cli-only");
  }
  const git = executable("git");
  if (await runCommand(git, ["-C", market.root, "status", "--porcelain", "--untracked-files=all"])) throw new Error("Marketplace has local changes; preserve/review them before updating");
  const descriptor = JSON.parse(await runCommand(git, ["-C", market.root, "show", "HEAD:plugins/chatgpt/.codex-plugin/plugin.json"]));
  compareVersions(descriptor.version, descriptor.version);
  const tags = await runCommand(git, ["-C", market.root, "tag", "--points-at", "HEAD"]);
  if (!tags.split(/\r?\n/).includes(`v${descriptor.version}`)) throw new Error("Marketplace checkout is not at its matching release tag; preserve this development/custom source or use --cli-only");
  return { version: descriptor.version, root: market.root };
}

async function installedPlugin(codex, runCommand = run) {
  const data = JSON.parse(await runCommand(codex, ["plugin", "list", "--json"]));
  if (!Array.isArray(data.installed)) throw new Error("Codex returned an unsupported plugin list");
  return data.installed.find(item => item.pluginId === selector);
}

export async function updateMarketplace(codex, target, before, enabled, record, runCommand = run) {
  const initial = await installedPlugin(codex, runCommand);
  if (enabled === false && initial?.version !== target) throw new Error("This plugin is disabled; the public Codex CLI cannot reinstall it while preserving disabled state. Use --cli-only or manage it in the host");
  if (before && before.version !== target) {
    record("removing_marketplace");
    await runCommand(codex, ["plugin", "marketplace", "remove", "agent-job-relay"]);
    record("marketplace_removed");
  }
  if (!before || before.version !== target) {
    record("adding_marketplace");
    await runCommand(codex, ["plugin", "marketplace", "add", repository, "--ref", `v${target}`, "--json"]);
    record("marketplace_added");
  }
  const current = await installedPlugin(codex, runCommand);
  if (!current || current.version !== target) {
    record("installing_plugin");
    const installed = JSON.parse(await runCommand(codex, ["plugin", "add", selector, "--json"]));
    if (installed.pluginId !== selector || installed.version !== target) throw new Error("Codex did not confirm installation of the exact requested plugin version");
  }
  const after = await installedPlugin(codex, runCommand);
  if (!after || after.version !== target || enabled === false && after.enabled !== false) throw new Error("Installed plugin verification failed; inspect Codex before retrying");
  record("plugin_verified");
}

async function guard(executablePath, profiles, target, apply) {
  const args = ["update-guard", "--installation", packageRoot, "--target-version", target];
  for (const profile of profiles) args.push("--profile", profile);
  if (!apply) return { report: JSON.parse(await run(executablePath, args)) };
  args.push("--apply");
  const child = spawn(executablePath, args, { stdio: ["pipe", "pipe", "pipe"] });
  child.stdin.on("error", () => {});
  let errors = "";
  child.stderr.on("data", data => { errors = (errors + data).slice(-65536); });
  const closed = new Promise((accept, reject) => { child.once("error", reject); child.once("close", code => code === 0 ? accept() : reject(new Error(`Update preflight/restart failed: ${errors}`))); });
  // Observe early exits even while awaiting the first protocol line.
  closed.catch(() => {});
  const lines = createInterface({ input: child.stdout });
  const iterator = lines[Symbol.asyncIterator]();
  const first = await Promise.race([iterator.next(), closed.then(() => { throw new Error("Update guard exited before readiness"); })]);
  if (first.done) { await closed; throw new Error("Update guard returned no readiness"); }
  const report = JSON.parse(first.value);
  let finishing;
  return {
    report,
    finish(image, version) {
      // Repeated observers share the original acknowledgement without replay.
      if (!finishing) finishing = (async () => {
        child.stdin.write(JSON.stringify({ executable: image, version }) + "\n");
        const next = await Promise.race([iterator.next(), closed.then(() => { throw new Error("Update guard exited before restart acknowledgement"); })]);
        if (next.done || JSON.parse(next.value).finished !== true) throw new Error("Update guard did not confirm restart");
      })();
      return finishing;
    },
    async release() { child.stdin.end("release\n"); await closed; lines.close(); },
    async abort() { child.stdin.destroy(); await closed.catch(() => {}); lines.close(); },
  };
}

export async function update(request) {
  interrupted = false;
  const signals = ["SIGINT", "SIGTERM", "SIGHUP"].map(signal => {
    const cancel = () => { interrupted = true; for (const child of running) child.kill(signal); };
    process.on(signal, cancel);
    return [signal, cancel];
  });
  try { return await updateInner(request); }
  finally { for (const [signal, cancel] of signals) process.off(signal, cancel); }
}

async function updateInner(request) {
  const npm = npmEntry();
  const npmRun = args => run(process.execPath, [npm, ...args]);
  const globalRoot = await npmRun(["root", "--global"]);
  let selectedRoot;
  try { selectedRoot = canonical(join(globalRoot, packageName)); } catch { /* Report a missing global package as a setup error. */ }
  if (selectedRoot !== canonical(packageRoot)) throw new Error("This is not the selected global npm installation. Install the CLI globally once before running ajr update");
  const prefix = await npmRun(["prefix", "--global"]);
  const shortEntry = executable("ajr");
  const expectedEntry = process.platform === "win32" ? join(prefix, "ajr.cmd") : join(packageRoot, "ajr.mjs");
  if (canonical(shortEntry) !== canonical(expectedEntry)) throw new Error("PATH selects another ajr installation; put this npm prefix's command directory on PATH before updating");
  const originalVersion = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")).version;
  const codex = request.cli_only ? null : executable("codex");
  let hostTarget = null;
  if (codex) {
    executable("git");
    const home = process.env.CODEX_HOME || join(process.platform === "win32" ? process.env.USERPROFILE : process.env.HOME, ".codex");
    hostTarget = { executable: realpathSync(codex), home: resolve(home) };
  }
  const state = request.state_dir;
  const journal = join(state, "online-update.json");
  let previous;
  try {
    const stat = lstatSync(journal);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 65536) throw new Error("Update journal must be a bounded regular private file");
    previous = JSON.parse(readFileSync(journal, "utf8"));
    if (previous.format !== 1 || previous.packageRoot !== packageRoot || previous.prefix !== prefix) throw new Error("Update journal belongs to another npm installation; inspect it before retrying");
    if (!["runtime_pending", "runtime_verified", "removing_marketplace", "marketplace_removed", "adding_marketplace", "marketplace_added", "installing_plugin", "plugin_verified", "complete"].includes(previous.stage)) throw new Error("Unknown update journal stage; inspect it before retrying");
  } catch (error) { if (error.code !== "ENOENT") throw error; }
  const pending = previous && previous.stage !== "complete";
  if (pending && JSON.stringify(previous.hostTarget) !== JSON.stringify(hostTarget)) throw new Error("The unfinished update belongs to a different Codex executable or CODEX_HOME; restore the original host selection before retrying");
  if (pending && (previous.cliOnly !== request.cli_only || request.to && request.to !== previous.target)) throw new Error("An unfinished update exists. Repeat the original options to finish forward");
  const target = pending ? previous.target : request.to ?? JSON.parse(await npmRun(["view", `${packageName}@latest`, "version", "--json", `--registry=${registry}`]));
  if (compareVersions(target, originalVersion) < 0) throw new Error("The selected version is older than this CLI; downgrade refused. Use --to <newer-published-version> when testing a development build");
  // Resolve an explicit version too, before any host/runtime mutation.
  if (JSON.parse(await npmRun(["view", `${packageName}@${target}`, "version", "--json", `--registry=${registry}`])) !== target) throw new Error("Requested npm version could not be resolved exactly");
  const before = codex ? await inspectMarketplace(codex) : null;
  if (before && compareVersions(target, before.version) < 0) throw new Error("Marketplace is newer than the selected runtime; downgrade refused");
  const plugin = codex ? await installedPlugin(codex) : null;
  if (plugin && compareVersions(target, plugin.version) < 0) throw new Error("Installed plugin is newer than the selected runtime; downgrade refused");
  const enabled = pending ? previous.enabled : plugin?.enabled;
  if (enabled === false && plugin?.version !== target) throw new Error("This plugin is disabled; use ajr update --cli-only or enable/manage it in the host before updating");
  const profiles = [...new Set([state, defaultStateDir("codex", null), defaultStateDir("opencode", null)])];
  const originalImage = resolveExecutable();
  const unchanged = target === originalVersion && (!codex || before?.version === target && plugin?.version === target) && !pending;
  const plan = { version: target, cli: target !== originalVersion ? "update" : "current", plugin: codex ? "install/refresh matching ChatGPT Desktop/Codex plugin" : "unchanged", profiles, data: "preserved", resumed_update: Boolean(pending), changed: !unchanged };
  if (unchanged) { console.log(JSON.stringify({ mode: request.dry_run ? "dry_run" : "applied", ...plan }, null, 2)); return; }
  await guard(originalImage, profiles, target, false);
  if (request.dry_run) { console.log(JSON.stringify({ mode: "dry_run", ...plan }, null, 2)); return; }
  const temp = mkdtempSync(join(tmpdir(), "ajr-update-"));
  const helper = join(temp, process.platform === "win32" ? "agent-job-relay.exe" : "agent-job-relay");
  copyFileSync(originalImage, helper); accessSync(helper, constants.X_OK);
  let fencing, installedImage = originalImage, installedVersion = originalVersion;
  const record = stage => {
    const data = { format: 1, packageRoot, prefix, hostTarget, target, cliOnly: request.cli_only, enabled, beforeMarketplace: pending ? previous.beforeMarketplace : before, stage };
    const temporary = journal + ".new";
    writeFileSync(temporary, JSON.stringify(data) + "\n", { mode: 0o600, flag: "wx" });
    const file = openSync(temporary, "r+");
    try { fsyncSync(file); } finally { closeSync(file); }
    renameSync(temporary, journal);
    if (process.platform !== "win32") {
      const directory = openSync(state, "r");
      try { fsyncSync(directory); } finally { closeSync(directory); }
    }
  };
  try {
    // The native guard creates private profiles only for an applied operation.
    fencing = await guard(helper, profiles, target, true);
    if (interrupted) throw new Error("Update interrupted before replacement");
    if (codex && JSON.stringify(await inspectMarketplace(codex)) !== JSON.stringify(before)) throw new Error("Marketplace changed during preflight; inspect it before retrying");
    record("runtime_pending");
    if (target !== originalVersion) {
      console.error(`Updating AgentJobRelay to ${target}…`);
      await npmRun(["install", "--global", "--prefix", prefix, "--ignore-scripts", "--no-audit", "--no-fund", `--registry=${registry}`, `${packageName}@${target}`]);
      installedImage = resolveExecutable();
      installedVersion = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")).version;
      if (installedVersion !== target || await run(installedImage, ["--version"]) !== `agent-job-relay ${target}`) throw new Error("New runtime did not verify as the exact requested version");
    }
    record("runtime_verified");
    if (codex) {
      if (JSON.stringify(await inspectMarketplace(codex)) !== JSON.stringify(before)) throw new Error("Marketplace changed during npm installation; inspect the original host before retrying");
      await updateMarketplace(codex, target, before, enabled, record);
    }
    await fencing.finish(installedImage, installedVersion);
    record("complete");
    await fencing.release(); fencing = null;
    console.log(JSON.stringify({ mode: "applied", ...plan, next_step: codex ? "Restart/reconnect ChatGPT Desktop or Codex to load the updated plugin and MCP catalog." : "CLI updated. OpenCode plugin packages keep their host-managed update route." }, null, 2));
  } catch (error) {
    if (fencing) {
      // Resolve actual installed bytes after any uncertain npm result. Never
      // restore old package files or replay a host write on error.
      try {
        installedImage = resolveExecutable();
        installedVersion = JSON.parse(readFileSync(join(packageRoot, "package.json"), "utf8")).version;
        await fencing.finish(installedImage, installedVersion);
        await fencing.release(); fencing = null;
      } catch (restart) { error.message += `; daemon recovery: ${restart.message}`; }
    }
    throw new Error(`${error.message}. Completed steps remain recorded at ${journal}; inspect the same profile and repeat ajr update with the original options. No rollback was attempted`);
  } finally { if (fencing) await fencing.abort(); rmSync(temp, { recursive: true, force: true }); }
}
