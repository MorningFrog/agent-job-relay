#!/usr/bin/env node
import { spawn } from "node:child_process";
import { constants } from "node:os";
import { resolveExecutable, defaultStateDir } from "./index.mjs";

try {
  const args = process.argv.slice(2);
  const env = { ...process.env };
  if (["codex", "opencode"].includes(args[0])) {
    const host = args.shift();
    // Native --state-dir remains an explicit override after the host selector.
    env.AJR_STATE_DIR = defaultStateDir(host);
  }
  const child = spawn(resolveExecutable(), args, { stdio: "inherit", env });
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"]) process.on(signal, () => child.kill(signal));
  child.once("error", (error) => { console.error(`AgentJobRelay: ${error.message}`); process.exitCode = 1; });
  child.once("exit", (code, signal) => { process.exitCode = code ?? 128 + (constants.signals[signal] ?? 1); });
} catch (error) {
  console.error(`AgentJobRelay: ${error.message}`);
  process.exitCode = 1;
}
