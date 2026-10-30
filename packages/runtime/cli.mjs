#!/usr/bin/env node
import { spawn } from "node:child_process";
import { constants } from "node:os";
import { resolveExecutable, defaultStateDir } from "./index.mjs";

try {
  const args = process.argv.slice(2);
  const env = { ...process.env };
  env.AJR_NPM_LAUNCHER = "1";
  if (["codex", "opencode"].includes(args[0])) {
    const host = args.shift();
    // Native --state-dir remains an explicit override after the host selector.
    env.AJR_STATE_DIR = defaultStateDir(host);
  }
  // Rust remains the argument parser. Only route this command's validated
  // request back to npm after the native child releases its executable image.
  let first = 0;
  while (args[first]?.startsWith("--state-dir=") || args[first] === "--state-dir") first += args[first] === "--state-dir" ? 2 : 1;
  const updating = args[first] === "update";
  const child = spawn(resolveExecutable(), args, { stdio: ["inherit", updating ? "pipe" : "inherit", "inherit"], env });
  let output = "";
  if (updating) child.stdout.on("data", data => { output += data; if (output.length > 65536) child.kill(); });
  const signals = ["SIGINT", "SIGTERM", "SIGHUP"].map(signal => {
    const forward = () => child.kill(signal);
    process.on(signal, forward);
    return [signal, forward];
  });
  child.once("error", (error) => { console.error(`AgentJobRelay: ${error.message}`); process.exitCode = 1; });
  child.once("close", async (code, signal) => {
    for (const [name, forward] of signals) process.off(name, forward);
    process.exitCode = code ?? 128 + (constants.signals[signal] ?? 1);
    if (!updating) return;
    try {
      let request;
      try { request = JSON.parse(output); } catch { /* Native help remains plain text. */ }
      if (code === 0 && request?.operation === "online_update") {
        const { update } = await import("./update.mjs");
        await update(request);
      } else process.stdout.write(output);
    } catch (error) {
      console.error(`AgentJobRelay update: ${error.message}`);
      process.exitCode = 1;
    }
  });
} catch (error) {
  console.error(`AgentJobRelay: ${error.message}`);
  process.exitCode = 1;
}
