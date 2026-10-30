// npm-specific lifecycle orchestration. Native code parses options and fences
// profiles; host configuration is changed only through the public Codex CLI.
import { spawn } from "node:child_process";
import { accessSync, closeSync, constants, copyFileSync, fsyncSync, lstatSync, mkdtempSync, openSync, readFileSync, realpathSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, delimiter, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { isDeepStrictEqual } from "node:util";
import { resolveExecutable, defaultStateDir } from "./index.mjs";

const packageRoot = dirname(fileURLToPath(import.meta.url));
const registry = "https://registry.npmjs.org";
const repository = "https://github.com/MorningFrog/agent-job-relay.git";
const marketplaceRecord = ".codex-marketplace-install.json";
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

async function run(program, args, { raw = false } = {}) {
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
    return raw ? stdout : stdout.trim();
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

function marketplaceChanges(output) {
  const entries = output.split("\0").filter(Boolean), changes = [];
  for (let i = 0; i < entries.length; i++) {
    const status = entries[i].slice(0, 2), path = entries[i].slice(3);
    const from = /[RC]/.test(status) ? entries[++i] : undefined;
    changes.push({ status, path, from });
  }
  return changes;
}

function describeMarketplaceChanges(changes) {
  const labels = { M: "modified", A: "added", D: "deleted", R: "renamed", C: "copied", T: "type changed", U: "unmerged" };
  return changes.slice(0, 10).map(({ status, path, from }) => {
    const label = status === "??" ? "untracked" : ["DD", "AU", "UD", "UA", "DU", "AA", "UU"].includes(status) ? "merge conflict" : [
      status[0] !== " " && `${labels[status[0]] ?? status[0]} in index`,
      status[1] !== " " && `${labels[status[1]] ?? status[1]} in working tree`,
    ].filter(Boolean).join(", ");
    return `  - ${label}: ${from === undefined ? "" : `${JSON.stringify(from)} -> `}${JSON.stringify(path)}`;
  }).join("\n") + (changes.length > 10 ? `\n  - … ${changes.length - 10} more changed paths` : "");
}

export async function inspectMarketplace(codex, runCommand = run) {
  const data = JSON.parse(await runCommand(codex, ["plugin", "marketplace", "list", "--json"]));
  if (!Array.isArray(data.marketplaces)) throw new Error("Codex returned an unsupported marketplace list; no changes applied");
  const matches = data.marketplaces.filter(item => item.name === "agent-job-relay");
  if (matches.length > 1) throw new Error("Multiple AgentJobRelay marketplace registrations; no changes applied");
  const market = matches[0];
  if (!market) return null;
  if (market.marketplaceSource?.sourceType !== "git" || market.marketplaceSource.source !== repository) {
    throw new Error(`The existing agent-job-relay marketplace at ${JSON.stringify(market.root)} is local or foreign; automatic maintenance requires Git source ${repository}. Keep its native maintenance route, or use --cli-only to leave the host plugin unchanged`);
  }
  const git = executable("git");
  const changes = marketplaceChanges(await runCommand(git, ["-C", market.root, "status", "--porcelain=v1", "-z", "--untracked-files=all"], { raw: true }));
  // Codex adds its own untracked provenance record after cloning a marketplace.
  // Accept only that exact entry, never a tracked edit or another untracked path.
  const isHostRecord = change => change.status === "??" && change.path === marketplaceRecord;
  const hostRecord = changes.some(isHostRecord), localChanges = changes.filter(change => !isHostRecord(change));
  if (localChanges.length) throw new Error(`Marketplace has local changes at ${JSON.stringify(market.root)}:\n${describeMarketplaceChanges(localChanges)}\nPreserve/review these paths before retrying, or use --cli-only to leave the host plugin unchanged.`);
  const descriptor = JSON.parse(await runCommand(git, ["-C", market.root, "show", "HEAD:plugins/chatgpt/.codex-plugin/plugin.json"]));
  compareVersions(descriptor.version, descriptor.version);
  const tags = await runCommand(git, ["-C", market.root, "tag", "--points-at", "HEAD"]);
  if (!tags.split(/\r?\n/).includes(`v${descriptor.version}`)) throw new Error(`Marketplace checkout at ${JSON.stringify(market.root)} is not at its matching release tag: expected v${descriptor.version} on HEAD; preserve this development/custom source or use --cli-only`);
  if (hostRecord) {
    try {
      const path = join(market.root, marketplaceRecord);
      const stat = lstatSync(path);
      if (stat.isSymbolicLink()) throw new Error("symbolic links are not accepted");
      if (!stat.isFile()) throw new Error("expected a regular file");
      if (stat.size > 65536) throw new Error("file exceeds the 65536-byte limit");
      let record;
      try { record = JSON.parse(readFileSync(path, "utf8")); }
      catch (error) { if (error instanceof SyntaxError) throw new Error("file is not valid JSON"); throw error; }
      const revision = await runCommand(git, ["-C", market.root, "rev-parse", "HEAD"]);
      const expected = { source_type: "git", source: repository, ref_name: `v${descriptor.version}`, sparse_paths: [], revision };
      if (!record || Array.isArray(record) || typeof record !== "object") throw new Error("expected a JSON object");
      const mismatched = Object.keys(expected).filter(key => !isDeepStrictEqual(record[key], expected[key]));
      if (mismatched.length) throw new Error(`fields do not match the official release checkout: ${mismatched.map(key => `${key} (expected ${JSON.stringify(expected[key])})`).join(", ")}`);
      const unrecognized = Object.keys(record).filter(key => !Object.hasOwn(expected, key));
      if (unrecognized.length) throw new Error(`unrecognized fields would be lost during replacement: ${unrecognized.slice(0, 10).map(key => JSON.stringify(key.slice(0, 128))).join(", ")}`);
    } catch (error) {
      throw new Error(`Marketplace installation record ${JSON.stringify(join(market.root, marketplaceRecord))} is invalid or mismatched: ${error.message}; preserve/review it before retrying, or use --cli-only to leave the host plugin unchanged.`, { cause: error });
    }
  }
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

async function guard(executablePath, profiles, target, apply, removal = false, force = false, allowActive = false, originalVersion = target) {
  const args = ["update-guard", "--installation", packageRoot, "--target-version", target];
  args.push("--original-version", originalVersion);
  if (removal) args.push("--remove");
  if (force) args.push("--force");
  if (allowActive) args.push("--allow-active");
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
  const finishRequest = request => {
    // Repeated observers share the original acknowledgement without replay.
    if (!finishing) finishing = (async () => {
      child.stdin.write(JSON.stringify(request) + "\n");
      const next = await Promise.race([iterator.next(), closed.then(() => { throw new Error("Maintenance guard exited before acknowledgement"); })]);
      if (next.done || JSON.parse(next.value).finished !== true) throw new Error("Maintenance guard did not confirm completion");
    })();
    return finishing;
  };
  return {
    report,
    finish(image, version) { return finishRequest({ executable: image, version }); },
    releaseOnly() { return finishRequest({ release_only: true }); },
    async release() { child.stdin.end("release\n"); await closed; lines.close(); },
    async abort() { child.stdin.destroy(); await closed.catch(() => {}); lines.close(); },
  };
}

async function withSignals(callback) {
  interrupted = false;
  const signals = ["SIGINT", "SIGTERM", "SIGHUP"].map(signal => {
    const cancel = () => { interrupted = true; for (const child of running) child.kill(signal); };
    process.on(signal, cancel);
    return [signal, cancel];
  });
  try { return await callback(); }
  finally { for (const [signal, cancel] of signals) process.off(signal, cancel); }
}

export function update(request) { return withSignals(() => updateInner(request)); }
export function uninstall(request) { return withSignals(() => uninstallInner(request)); }

async function globalInstallation() {
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
  return { npmRun, prefix, originalVersion };
}

function hostSelection(cliOnly) {
  const codex = cliOnly ? null : executable("codex");
  let hostTarget = null;
  if (codex) {
    executable("git");
    const home = process.env.CODEX_HOME || join(process.platform === "win32" ? process.env.USERPROFILE : process.env.HOME, ".codex");
    hostTarget = { executable: realpathSync(codex), home: resolve(home) };
  }
  return { codex, hostTarget };
}

function readJournal(path) {
  try {
    const stat = lstatSync(path);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 65536) throw new Error("Maintenance journal must be a bounded regular private file");
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) { if (error.code !== "ENOENT") throw error; }
}

function writeJournal(path, data) {
  const temporary = path + ".new";
  const encoded = JSON.stringify(data) + "\n";
  if (Buffer.byteLength(encoded) > 65536) throw new Error("Maintenance journal exceeds its size limit; nothing was recorded");
  writeFileSync(temporary, encoded, { mode: 0o600, flag: "wx" });
  const file = openSync(temporary, "r+");
  try { fsyncSync(file); } finally { closeSync(file); }
  renameSync(temporary, path);
  if (process.platform !== "win32") {
    const directory = openSync(dirname(path), "r");
    try { fsyncSync(directory); } finally { closeSync(directory); }
  }
}

function requireFinished(path, operation) {
  const journal = readJournal(path);
  if (journal && journal.stage !== "complete") throw new Error(`An unfinished ${operation} exists at ${path}; finish it with the original options before changing maintenance operations`);
}

async function updateInner(request) {
  const { npmRun, prefix, originalVersion } = await globalInstallation();
  const { codex, hostTarget } = hostSelection(request.cli_only);
  const state = request.state_dir;
  requireFinished(join(state, "online-uninstall.json"), "uninstall");
  const journal = join(state, "online-update.json");
  const previous = readJournal(journal);
  if (previous) {
    if (previous.format !== 1 || previous.packageRoot !== packageRoot || previous.prefix !== prefix) throw new Error("Update journal belongs to another npm installation; inspect it before retrying");
    if (!["runtime_pending", "runtime_verified", "removing_marketplace", "marketplace_removed", "adding_marketplace", "marketplace_added", "installing_plugin", "plugin_verified", "complete"].includes(previous.stage)) throw new Error("Unknown update journal stage; inspect it before retrying");
  }
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
  let daemonUpgrade = false;
  if (unchanged) {
    for (const profile of profiles.filter(exists)) {
      const health = JSON.parse(await run(originalImage, ["--state-dir", profile, "doctor"]));
      if (health.daemon === "stopped") continue;
      if (compareVersions(health.version, target) > 0) throw new Error("Running daemon is newer than the selected runtime; downgrade refused");
      daemonUpgrade ||= health.version !== target;
    }
  }
  const plan = { version: target, cli: target !== originalVersion ? "update" : "current", plugin: codex ? "install/refresh matching ChatGPT Desktop/Codex plugin" : "unchanged", profiles, data: "preserved", daemon: daemonUpgrade ? "upgrade older online daemon" : "restart online daemon when applying changes", resumed_update: Boolean(pending), changed: !unchanged || daemonUpgrade };
  if (unchanged && !daemonUpgrade) { console.log(JSON.stringify({ mode: request.dry_run ? "dry_run" : "applied", ...plan }, null, 2)); return; }
  const temp = mkdtempSync(join(tmpdir(), "ajr-update-"));
  const helper = join(temp, process.platform === "win32" ? "agent-job-relay.exe" : "agent-job-relay");
  let tarball;
  const allowActive = process.platform === "linux";
  let fencing, installedImage = originalImage, installedVersion = originalVersion;
  const record = stage => {
    const data = { format: 1, packageRoot, prefix, hostTarget, target, cliOnly: request.cli_only, enabled, beforeMarketplace: pending ? previous.beforeMarketplace : before, stage };
    writeJournal(journal, data);
  };
  try {
    let guardImage = originalImage;
    if (target !== originalVersion) {
      const packed = JSON.parse(await npmRun(["pack", `${packageName}@${target}`, "--ignore-scripts", "--json", "--pack-destination", temp, `--registry=${registry}`]));
      if (packed.length !== 1 || packed[0].filename !== `${packageName}-${target}.tgz`) throw new Error("npm returned an unexpected runtime tarball identity");
      tarball = join(temp, packed[0].filename);
      const staging = join(temp, "staged");
      await npmRun(["install", "--prefix", staging, "--ignore-scripts", "--no-audit", "--no-fund", `--registry=${registry}`, tarball]);
      guardImage = resolveExecutable(join(staging, "node_modules", packageName));
      if (await run(guardImage, ["--version"]) !== `agent-job-relay ${target}`) throw new Error("Staged native version did not match the selected package");
    }
    const preview = await guard(guardImage, profiles, target, false, false, false, allowActive, originalVersion);
    plan.active_jobs = preview.report.active_jobs;
    plan.runner_continuation = allowActive ? "compatible active runners continue without replay" : "finish active tasks before package replacement on this platform";
    if (request.dry_run) { console.log(JSON.stringify({ mode: "dry_run", ...plan }, null, 2)); return; }
    copyFileSync(guardImage, helper); accessSync(helper, constants.X_OK);
    // The native guard creates private profiles only for an applied operation.
    fencing = await guard(helper, profiles, target, true, false, false, allowActive, originalVersion);
    if (interrupted) throw new Error("Update interrupted before replacement");
    requireFinished(join(state, "online-uninstall.json"), "uninstall");
    if ((await globalInstallation()).originalVersion !== originalVersion) throw new Error("Installed runtime changed during preflight; retry from the observed installation");
    if (codex && JSON.stringify(await inspectMarketplace(codex)) !== JSON.stringify(before)) throw new Error("Marketplace changed during preflight; inspect it before retrying");
    record("runtime_pending");
    if (target !== originalVersion) {
      console.error(`Updating AgentJobRelay to ${target}…`);
      await npmRun(["install", "--global", "--prefix", prefix, "--ignore-scripts", "--no-audit", "--no-fund", `--registry=${registry}`, tarball]);
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

async function removalState(codex, runCommand = run) {
  const marketplace = await inspectMarketplace(codex, runCommand);
  const data = JSON.parse(await runCommand(codex, ["plugin", "list", "--json"]));
  if (!Array.isArray(data.installed)) throw new Error("Codex returned an unsupported plugin list");
  const plugin = data.installed.find(item => item.pluginId === selector);
  if (plugin && (plugin.marketplaceSource?.sourceType !== "git" || plugin.marketplaceSource.source !== repository)) throw new Error("Installed plugin has an unknown/local/foreign source; remove it through its original host route or use --cli-only");
  const shared = data.installed.some(item => item.pluginId !== selector && (item.marketplaceName === "agent-job-relay" || item.pluginId?.endsWith("@agent-job-relay")));
  return { marketplace, plugin, shared };
}

export async function removeOnlinePlugin(codex, expected, record, runCommand = run) {
  let current = await removalState(codex, runCommand);
  if (!isDeepStrictEqual(current, expected)) throw new Error("Plugin/marketplace changed during removal preflight; inspect the original host before retrying");
  if (current.plugin) {
    record("removing_plugin");
    await runCommand(codex, ["plugin", "remove", selector, "--json"]);
    current = await removalState(codex, runCommand);
    if (current.plugin) throw new Error("Codex still reports the installed plugin; CLI retained for recovery");
    record("plugin_removed");
  }
  if (!isDeepStrictEqual(current.marketplace, expected.marketplace)) throw new Error("Marketplace changed during plugin removal; CLI retained for recovery");
  if (current.marketplace && !current.shared) {
    record("removing_marketplace");
    await runCommand(codex, ["plugin", "marketplace", "remove", "agent-job-relay"]);
    current = await removalState(codex, runCommand);
    if (current.plugin || current.marketplace) throw new Error("Host removal did not verify; CLI retained for recovery");
  }
  record("host_removed");
  return { marketplace_retained: Boolean(current.marketplace), shared: current.shared };
}

function exists(path) {
  try { lstatSync(path); return true; } catch (error) { if (error.code !== "ENOENT") throw error; return false; }
}

function commandPaths(prefix) {
  return ["ajr", "agent-job-relay"].flatMap(name => process.platform === "win32" ? [name, name + ".cmd", name + ".ps1"].map(file => join(prefix, file)) : [join(prefix, "bin", name)]);
}

async function uninstallInner(request) {
  const { npmRun, prefix, originalVersion } = await globalInstallation();
  const { codex, hostTarget } = hostSelection(request.cli_only);
  const state = request.state_dir;
  requireFinished(join(state, "online-update.json"), "update");
  const journal = join(state, "online-uninstall.json");
  const previous = readJournal(journal);
  if (previous && (previous.format !== 1 || previous.packageRoot !== packageRoot || previous.prefix !== prefix || !["prepared", "removing_plugin", "plugin_removed", "removing_marketplace", "host_removed", "removing_cli", "complete"].includes(previous.stage))) throw new Error("Uninstall journal belongs to another installation or has an unknown stage; inspect it before retrying");
  const pending = previous && previous.stage !== "complete";
  if (pending && (previous.version !== originalVersion || previous.cliOnly !== request.cli_only || previous.force !== request.force || JSON.stringify(previous.hostTarget) !== JSON.stringify(hostTarget))) throw new Error("An unfinished uninstall exists; restore the original npm version, Codex selection and command options before retrying");
  const before = codex ? await removalState(codex) : null;
  const profiles = [...new Set([state, defaultStateDir("codex", null), defaultStateDir("opencode", null)])];
  const originalImage = resolveExecutable();
  const preview = await guard(originalImage, profiles, originalVersion, false, true, request.force);
  const plan = { mode: request.dry_run ? "dry_run" : "applied", cli: "remove global package and command registrations", plugin: codex ? "remove ChatGPT Desktop/Codex plugin" : "unchanged (requires CLI for future use)", marketplace: codex ? before.shared ? "retain for other installed plugins" : "remove official registration if present" : "unchanged", profiles, data: "history/logs/backups preserved", stop_daemon: true, force: request.force, active_jobs: preview.report.active_jobs, resumed_uninstall: Boolean(pending) };
  if (request.dry_run) { console.log(JSON.stringify(plan, null, 2)); return; }
  const temp = mkdtempSync(join(tmpdir(), "ajr-uninstall-"));
  const helper = join(temp, process.platform === "win32" ? "agent-job-relay.exe" : "agent-job-relay");
  copyFileSync(originalImage, helper); accessSync(helper, constants.X_OK);
  let fencing;
  const record = stage => writeJournal(journal, { format: 1, packageRoot, prefix, hostTarget, version: originalVersion, cliOnly: request.cli_only, force: request.force, before: pending ? previous.before : before, stage });
  try {
    fencing = await guard(helper, profiles, originalVersion, true, true, request.force);
    if (interrupted) throw new Error("Uninstall interrupted before removal");
    requireFinished(join(state, "online-update.json"), "update");
    if ((await globalInstallation()).originalVersion !== originalVersion) throw new Error("Installed runtime changed during preflight; inspect it before removing");
    record("prepared");
    if (codex) await removeOnlinePlugin(codex, before, record);
    record("removing_cli");
    let npmError;
    try { await npmRun(["uninstall", "--global", "--prefix", prefix, "--ignore-scripts", "--no-audit", "--no-fund", packageName]); } catch (error) { npmError = error; }
    if (exists(packageRoot) || commandPaths(prefix).some(exists)) throw npmError || new Error("npm removal left package files/commands; inspect them without deleting unrelated content");
    await fencing.releaseOnly();
    record("complete");
    await fencing.release(); fencing = null;
    console.log(JSON.stringify({ ...plan, next_step: "Restart/reconnect ChatGPT Desktop or Codex to discard loaded plugin connections. Task data and npm download caches were retained." }, null, 2));
  } catch (error) {
    if (fencing) {
      try { await fencing.releaseOnly(); await fencing.release(); fencing = null; } catch (release) { error.message += `; guard release: ${release.message}`; }
    }
    throw new Error(`${error.message}. Inspect the same profile and any progress written at ${journal}; repeat ajr uninstall with the original options. If npm already removed the CLI, reinstall ${packageName}@${originalVersion} in the same prefix first. No data was purged and stopped daemons were not restarted`);
  } finally { if (fencing) await fencing.abort(); rmSync(temp, { recursive: true, force: true }); }
}
